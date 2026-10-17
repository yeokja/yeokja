---
date: "2026-03-01"
tags: ["ai", "python", "llms", "compilers", "mlir"]
url: "https://www.stephendiehl.com/posts/mlir_python_frontend/"
---

<img src="https://www.stephendiehl.com/images/mlir_frontend.jpeg" width="500" alt="Python Frontend" />

# A Python Frontend for MLIR

Last time we took MLIR through a GPU backend and looked at what it takes to launch the resulting code. We now need something at the other end of the compiler: a way to describe tensor computations in Python and construct the IR which those backends consume.

We'll build this part around MLIR's Python bindings. They expose the same objects we've been writing in textual form: types, values, operations, regions and blocks. A Python program can construct a `linalg.matmul` operation, pass its result to another operation, and run compiler passes over the resulting module. The printed MLIR is a way to inspect that module.

The companion project is [`tiny-mlir`](https://github.com/sdiehl/tiny-mlir). It contains the complete implementation used in these remaining chapters. Clone the repository and install its environment:

```
git clone https://github.com/sdiehl/tiny-mlir.git
cd tiny-mlir
uv sync
uv run pytest
```

The project pins Python 3.12 and a specific MLIR 22 development build, including its Python bindings and execution engine. These examples use that bundled toolchain. The command-line LLVM installation from earlier chapters can remain separate; mixing its libraries into the Python environment isn't necessary.

## A Function to Compile

Here's the interface we'll use:

``` python
import numpy as np
from tinymlir import jit

@jit
def scale_bias(x, bias):
    return x * 2.0 + bias

x = np.arange(8, dtype=np.float32)
bias = np.ones(8, dtype=np.float32)
print(scale_bias(x, bias))
```

The first call specializes the function for the argument shapes and dtypes, constructs MLIR, lowers it and creates an execution engine. Later calls with the same types reuse that specialization. Changing the contents of `x` doesn't require compilation. Changing its shape does.

The function doesn't need to know anything about memref descriptors or execution engines. It describes a tensor expression. The compiler owns the representation and the runtime boundary.

## Tracing Python

There are several reasonable ways to build a frontend. In Part 6 we parsed Python syntax and transformed it. Here we'll use **tracing**: call the Python function with symbolic tensor arguments in place of arrays. Their arithmetic methods construct expression nodes.

When Python evaluates `x * 2.0`, it calls our symbolic value's `__mul__` method. When it evaluates the addition, that method receives the multiplication node as its left operand. The result is a graph which describes the computation without reading any array elements.

This is a small embedded language. Ordinary Python function calls compose naturally during tracing. Loops over static shape information can also run during tracing, since their bounds are known. A branch depending on tensor contents cannot: the symbolic value's `__bool__` raises an error. We aren't compiling arbitrary Python or capturing mutations to Python objects.

Our type contains exactly the information needed for specialization:

``` python
@dataclass(frozen=True)
class TensorType:
    shape: tuple[int, ...]
    dtype: np.dtype
```

The implementation normalizes the shape and dtype, requires positive static dimensions, and accepts float32, float64 and int32. The integer type is used for token indices. Floating-point computations carry their precision explicitly.

Each expression stores an operation, a result type, its operands and any static attributes:

``` python
@dataclass(eq=False)
class Expr:
    op: Op
    type: TensorType
    args: tuple[Expr, ...] = ()
    attr: object = None
```

`Op` is an enum shared by the frontend and builder, so operation names are defined in one place. These nodes use identity rather than structural equality. If a computation reuses the same node, the lowering caches its MLIR value and reuses that value too. MLIR's common-subexpression elimination pass can subsequently identify independently constructed equivalent operations.

## Shapes Before Code Generation

Addition first checks that its operands can broadcast. For shapes `(3, 7)` and `(7,)`, the result has shape `(3, 7)`. For `(3, 7)` and `(3, 1)`, it has the same shape. For `(3, 7)` and `(5,)`, compilation fails.

The frontend also determines a common dtype and inserts explicit cast nodes when needed. Python numeric literals use the surrounding tensor's dtype. NumPy scalar values retain their own dtype. This distinction will matter when we reproduce the NumPy GPT-2 implementation, which introduces float64 values in some of its formulas.

After this stage, the backend doesn't have to guess whether an addition is scalar, elementwise or broadcast. Its operand shapes and result shape already describe the indexing it needs.

Shape specialization is a useful instance of separating static information from runtime data. It is related to the partial-evaluation ideas from Part 6: we fix the types and static parameters, leaving array contents as inputs to the compiled program. Our implementation is a dedicated tensor compiler, rather than a general specializer for the Python interpreter.

## Constructing a Module

The MLIR bindings give us a context, a module and insertion points:

``` python
from mlir import ir
from mlir.dialects import func

with ir.Context(), ir.Location.unknown():
    module = ir.Module.create()
    with ir.InsertionPoint(module.body):
        f32 = ir.F32Type.get()
        buffer = ir.MemRefType.get([8], f32)
        function = func.FuncOp("kernel", ([buffer, buffer], []))
        block = function.add_entry_block()
        with ir.InsertionPoint(block):
            func.ReturnOp([])
    module.operation.verify()
```

This is a complete, if uninteresting, function which takes two buffers and returns nothing. The context owns uniqued objects such as types. The insertion point determines where newly created operations go. The block owns its arguments and operations. These relationships let the binding check how objects fit together as we construct the program.

The real builder creates a memref argument for each input and one for the caller's output buffer. It converts inputs to tensor values with `bufferization.to_tensor`, builds the computation on tensors, and finishes with `bufferization.materialize_in_destination`. The latter says that the result must end up in the output buffer supplied by the caller.

This boundary keeps the external calling convention simple while allowing the body to remain a functional tensor computation. Bufferization will decide which internal tensors need storage and which buffers can be reused.

## Elementwise Regions

An elementwise expression becomes `linalg.generic`. The builder supplies its input values, an empty destination tensor, indexing maps and iterator types. It then creates the operation's region and fills in the scalar computation:

``` python
op = linalg.GenericOp(
    [output.type], inputs, [output], indexing_maps,
    ["parallel"] * rank,
)
block = op.regions[0].blocks.append(
    *(value.type.element_type for value in [*inputs, output])
)
with ir.InsertionPoint(block):
    product = arith.MulFOp(block.arguments[0], block.arguments[1])
    linalg.YieldOp([product.result])
```

This fragment is the multiplication case within the builder. `output`, `inputs`, maps and rank come from the typed expression. The region arguments are scalar elements, so its body multiplies two floats and yields one float. The surrounding `linalg.generic` describes how those scalar operations apply across the tensors.

Broadcasting belongs in the indexing maps. For a matrix plus a row vector, those maps are:

```
matrix: (i, j) -> (i, j)
vector: (i, j) -> (j)
output: (i, j) -> (i, j)
```

For a `(3, 1)` tensor broadcast across seven columns, its map becomes `(i, j) -> (i, 0)`. A scalar tensor has an empty index list. We construct these maps using `AffineDimExpr`, `AffineConstantExpr` and `AffineMap`; no textual affine-expression parser is involved.

The distinction between the scalar region and its iteration space is precisely the structure we want the optimizer to see. It can fuse elementwise operations by composing their maps and regions, before committing to a loop schedule.

## Composition

A helper function simply constructs more of the same graph:

``` python
from tinymlir import tanh


def activation(x):
    return 0.5 * x * (1 + tanh(x))


@jit
def activated_scale(x, bias):
    return activation(x * 2.0 + bias)
```

There is no special `activated_scale` kernel in the compiler. Nor does `activation` need its own backend implementation. The compiler only needs rules for the constituent tensor operations. If we change the formula, the graph and generated module change with it.

A nested call to a `@jit` function behaves the same way while tracing. When its arguments are symbolic, it contributes expressions to the surrounding graph. When its arguments are arrays, it selects and executes a compiled specialization.

## NumPy Calls

Operator overloading covers `x * 2.0 + bias`, but NumPy code also calls functions: `np.sqrt`, `np.tanh`, `np.mean`, `np.split`. The symbolic value implements two NumPy dispatch protocols so those calls trace too. `__array_ufunc__` handles the elementwise universal functions, so `np.tanh(x)` on a traced value builds the same node as our own `tanh(x)`. `__array_function__` handles a short table of array functions: `mean`, `var`, `sum`, `max`, `split`, `reshape` and `transpose`. NumPy calls these methods whenever one of the arguments is our type.

``` python
@jit
def normalize(x):
    centered = x - np.mean(x, axis=-1, keepdims=True)
    return centered / np.sqrt(np.var(x, axis=-1, keepdims=True) + 1e-5)
```

This is ordinary NumPy. Called on arrays it runs in NumPy; called through the JIT it traces into a graph. A function outside the table returns `NotImplemented`, and NumPy raises a `TypeError` rather than quietly evaluating something we can't compile. In Part 14 this lets us trace the reference GPT-2 implementation without rewriting it against our library.

A traced function may also use arrays from its enclosing scope, such as a mask or a table built in Python. A NumPy array captured this way becomes a constant node. Rather than embedding its contents in the IR, the JIT lifts each captured array to a trailing argument of the compiled function and passes it at every call. The specialization depends on the array's shape and dtype, not its values.

## Inspecting the Result

The compiled object retains three modules:

``` python
compiled = scale_bias.compile(x, bias)
print(compiled.module)
print(compiled.optimized)
print(compiled.lowered)
```

The first contains the tensor program emitted by the builder. The second shows the result of canonicalization, common-subexpression elimination and elementwise fusion. The third contains the LLVM dialect program used by the execution engine. They are separate module objects, so running a pass doesn't destroy the version we're trying to inspect.

The runtime allocates the output array, constructs ranked memref descriptors for the arguments, and invokes the generated C interface. The compiled object keeps its context and execution engine alive. There is no NumPy execution fallback if compilation fails.

This gives us the outline of a small compiler: trace a restricted Python computation, check its tensor types, construct structured MLIR, run transformations, and execute the lowered result. Next we'll add the reductions needed by softmax and layer normalization.

## External Resources

- [MLIR Python bindings](https://mlir.llvm.org/docs/Bindings/Python/)
- [MLIR Linalg dialect](https://mlir.llvm.org/docs/Dialects/Linalg/)
- [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/)
- [Partial Evaluation and Automatic Program Generation](https://raspi.itu.dk/people/sestoft/pebook/pebook.html)

[Previous: Part 8 - GPU Compilation with MLIR](mlir_gpu.md) · [Series contents](mlir_introduction.md) · [Next: Part 10 - Compiling Transformer Kernels](mlir_transformer_kernels.md)

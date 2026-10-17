---
date: "2026-05-30"
tags: ["ai", "python", "llms", "compilers", "mlir"]
url: "https://www.stephendiehl.com/posts/mlir_optimization/"
---

<img src="https://www.stephendiehl.com/images/mlir_fusion.jpeg" width="500" alt="Fusion" />

# Fusion, Bufferization and Execution

Our frontend now describes the transformer as tensor operations. We've deliberately left questions about storage and execution out of those definitions. This chapter follows the other half of the compiler: transforming the tensor program, assigning buffers, lowering to LLVM and calling the result from Python.

The implementation is in `tinymlir/jit.py`. It retains the original, optimized and lowered modules separately, so we can inspect what each stage changes.

## Elementwise Fusion

Consider a simple function:

``` python
from tinymlir import jit

@jit
def affine_activation(x, gain, bias):
    return (x * gain + bias) * 0.5
```

The builder initially emits three elementwise `linalg.generic` operations. Each has its own output tensor. At the tensor level, these values describe the results of computations; their existence doesn't yet require three separate heap allocations.

Our optimization pipeline is:

``` python
OPTIMIZATION_PIPELINE = (
    "builtin.module("
    "canonicalize,cse,linalg-fuse-elementwise-ops,canonicalize,cse)"
)
```

The fusion pass combines compatible producer and consumer regions, composing their indexing maps. For this example, the optimized module contains one `linalg.generic`. Its scalar body multiplies, adds and multiplies again before yielding a result.

The test suite checks both facts: the operation count changes from three to one, and executing the optimized and unoptimized modules produces the same array. This verifies an actual compiler transformation, rather than comparing two separately written kernel implementations.

The pass-pipeline string names transformations for MLIR's pass manager. The program being transformed is an object graph constructed through the Python bindings:

``` python
from mlir.passmanager import PassManager

PassManager.parse(OPTIMIZATION_PIPELINE).run(module.operation)
module.operation.verify()
```

The verifier checks the result after transformation. Errors are propagated to the caller; there is no alternative NumPy path which could conceal a compilation failure.

## The Limits of Fusion

Elementwise fusion is useful because it can eliminate intermediate stores, loads and allocations. It doesn't imply that every chain of operations should become one region. Reductions have different iteration semantics, matrix multiplication has a reduction dimension, and a producer may have multiple consumers.

For example, softmax's exponentials feed both a sum and the final division. A legal transformation must account for that use pattern. Depending on the schedule, storing a result or recomputing it can have different costs. The tensor graph gives the compiler enough information to make and check those decisions.

Floating-point arithmetic also limits transformations. Reassociating a reduction can change its result. Our frontend preserves the formula's dtype promotions, and the generated arithmetic doesn't request unrestricted fast-math behavior. We still compare against NumPy with numerical tolerances because its reductions and BLAS implementation may accumulate in a different order.

## Bufferization

After tensor transformations, we run one-shot bufferization. It analyzes tensor uses to decide where storage can be reused and where a separate allocation is required.

The external function already has a straightforward contract: memref inputs and a caller-provided output memref. Inside the function, `bufferization.to_tensor` makes inputs available to tensor operations. `bufferization.materialize_in_destination` requires the final result to be written to the output buffer.

We mark the input conversions `restrict`. This is a promise about aliasing at the boundary. If the caller passes the same array twice, or passes overlapping views, the runtime copies overlapping inputs before invoking the function. The new output allocation is also distinct from the inputs. The promise made to bufferization therefore matches what the runtime supplies.

Inputs are read-only from the tensor program's perspective. Intermediate results remain functional values until bufferization chooses their storage. We don't modify the model's weights while evaluating a block.

Storage ownership is a separate concern from buffer assignment. Our pipeline includes the buffer-deallocation pipeline so internally allocated buffers are released. The caller's NumPy arrays remain owned by Python. Returning a pointer to an unmanaged temporary would make a small successful example misleading; repeated inference needs the ownership boundary to be correct too.

## Lowering the Module

After fusion, the companion project lowers in two stages, with a target-specific step between them:

``` python
BUFFERIZATION_PIPELINE = (
    "builtin.module("
    "one-shot-bufferize{bufferize-function-boundaries},buffer-deallocation-pipeline)"
)
LOWERING_PIPELINE = (
    "builtin.module("
    "convert-linalg-to-loops,"
    "expand-strided-metadata,lower-affine,"
    "convert-scf-to-cf,convert-math-to-libm,convert-math-to-llvm,"
    "convert-arith-to-llvm,convert-func-to-llvm,convert-cf-to-llvm,"
    "finalize-memref-to-llvm,reconcile-unrealized-casts)"
)
```

Bufferization runs once, for every target. It resolves tensor storage and inserts the deallocations. What comes out is a module of `linalg` operations over memrefs in which the matrix multiplications are still `linalg.matmul` and `linalg.batch_matmul`. That is where the targets diverge:

``` python
PassManager.parse(BUFFERIZATION_PIPELINE).run(lowered.operation)
if target == GPU:
    gpu.lower(lowered)
else:
    blas.rewrite(lowered)
    PassManager.parse(LOWERING_PIPELINE).run(lowered.operation)
```

On the CPU, `blas.rewrite` walks the module for row-major matmuls and replaces each with a call to `cblas_sgemm` or `cblas_dgemm`. The library is whatever the process can load, Accelerate on macOS or OpenBLAS on Linux, and NumPy's own matrix multiplication calls a BLAS in the same way. The rewrite reads the aligned base pointer, offset, sizes and strides out of each memref with `memref.extract_strided_metadata`, passes the row strides as leading dimensions, and turns a batched product into an `scf.for` loop of GEMM calls. When no BLAS is present the rewrite does nothing and the matmuls lower to loops along with everything else. This is why the frontend was careful to keep the matmul recognizable through fusion.

The remaining stages follow the information being removed. Linalg lowering chooses explicit loops. Metadata expansion and affine lowering expose address calculations. Structured control flow becomes ordinary branches. Arithmetic, functions and memory references then become LLVM dialect operations. Math operations use the LLVM or host math-library paths available in this pinned toolchain. Finally, reconciliation removes conversion artifacts whose source and destination types have converged.

The CPU pipeline runs in the same process as the Python bindings and uses their matching execution engine. We aren't serializing a module to a subprocess and attempting to combine unrelated LLVM builds. The GPU path takes the same bufferized module in a different direction, and gets its own chapter.

## Calling Native Code

The function carries `llvm.emit_c_interface`, which requests a wrapper with a C-compatible calling convention. The runtime uses MLIR's ranked memref descriptor helper:

``` python
pointers = [
    ctypes.pointer(ctypes.pointer(get_ranked_memref_descriptor(array)))
    for array in [*inputs, output]
]
engine.invoke("kernel", *pointers)
```

The execution engine's packed invocation API takes pointers to argument storage. The C interface's memref arguments are themselves descriptor pointers, which accounts for the extra level of indirection here. The descriptor records the array's address, offset, sizes and strides.

Before invoking the function, the runtime checks the specialization's shapes and dtypes and makes inputs contiguous. It allocates a NumPy output array of the inferred result type. The arrays and descriptors remain alive throughout the call, and the compiled object owns the context, modules and engine for as long as its specialization is cached.

Tensor-valued gather indices are also checked before entering native code. Shape checking can't prove that runtime token IDs lie within an embedding table. The model validates vocabulary bounds, and the generic gather boundary performs its own index check.

## Specialization and Cost

The JIT cache is keyed by the structure of the arguments, the static keyword arguments and the target. Arguments are flattened as trees: tuples, named tuples, dataclasses and dicts are taken apart recursively until arrays are reached, and the key records the tree's shape together with the shape and dtype of every leaf. A compiled function can therefore take a whole parameter record while remaining a function of arrays. Transformer blocks with different weight values share a specialization when their argument shapes and dtypes agree. The weights are runtime inputs, rather than constants embedded in each compiled module.

A new sequence length changes the shapes and requires another specialization. The example recomputes the full prefix for each generated token, so its first run includes compilation for several lengths. Repeated calls at an existing shape reuse the engine.

This is a deliberately small in-process cache. It doesn't provide persistent object caching or a memory policy for an unlimited stream of shapes. Those would be useful extensions, but they aren't necessary to understand the compilation boundary.

When measuring execution, warm up the relevant specializations first. Otherwise a timing includes tracing, IR construction, pass execution and machine-code generation as well as inference. A comparison which mixes those costs tells us little about the generated kernels.

## Returning to the GPU

Part 8 established the GPU execution model and PTX path. A GPU backend for this tensor compiler makes different scheduling decisions before lowering away the structured operations: matrix multiplication is tiled, loop iterations are mapped to blocks and threads, and launches appear at the host-device boundary.

The retained `linalg.matmul` and reduction operations in the bufferized module give such a backend something useful to transform. Once everything has become scalar loops and pointer arithmetic, recovering that structure becomes harder.

The [`tiny-mlir`](https://github.com/sdiehl/tiny-mlir) project has this backend. Passing `--gpu` to the example or the verification script selects a target that replaces the matmuls with a shared-memory tiled kernel, maps the remaining parallel loops onto blocks and threads, outlines them as GPU kernels and runs the result over CUDA managed memory. The PTX it emits is checked with `ptxas` in continuous integration, and the full reference comparison has been run on a Tesla T4. None of the Python model's formulas change between the two targets. Part 14 walks through the backend.

Next we'll load the pretrained checkpoint and test the complete inference path, including the exact sequence of generated tokens.

## External Resources

- [MLIR Python bindings](https://mlir.llvm.org/docs/Bindings/Python/)
- [MLIR Bufferization](https://mlir.llvm.org/docs/Bufferization/)
- [MLIR passes](https://mlir.llvm.org/docs/Passes/)
- [MLIR GPU dialect](https://mlir.llvm.org/docs/Dialects/GPU/)

[Previous: Part 11 - Matrix Multiplication and Attention](mlir_attention.md) · [Series contents](mlir_introduction.md) · [Next: Part 13 - Running GPT-2](mlir_gpt2.md)

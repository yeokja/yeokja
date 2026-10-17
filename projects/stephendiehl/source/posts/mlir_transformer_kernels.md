---
date: "2026-03-28"
tags: ["ai", "python", "llms", "compilers", "mlir"]
url: "https://www.stephendiehl.com/posts/mlir_transformer_kernels/"
---

<img src="https://www.stephendiehl.com/images/mlir_kernels.jpeg" width="500" alt="Transformer Kernels" />

# Compiling Transformer Kernels

Our frontend can now turn Python tensor expressions into MLIR operations. We'll use it to implement GELU, softmax and layer normalization from the [NumPy GPT-2 model](mlir_transformers.md). These are a useful progression: GELU is elementwise, softmax introduces reductions, and normalization combines reductions with broadcasting.

The distinction between a library function and a compiler primitive will keep the implementation small. GELU is a library function. Addition, multiplication, powers and tanh are compiler primitives. The compiler doesn't need to recognize the name `gelu` to compile its formula.

The examples below run from the [`tiny-mlir`](https://github.com/sdiehl/tiny-mlir) companion project, using the environment from Part 9.

## GELU

GPT-2 uses the tanh approximation:

$$\operatorname{GELU}(x)=\frac{x}{2}\left(1+\tanh\left[\sqrt{\frac{2}{\pi}}(x+0.044715x^3)\right]\right)$$

Our implementation closely follows the reference:

``` python
import numpy as np
from tinymlir import tanh


def gelu(x):
    return 0.5 * x * (1 + tanh(np.sqrt(2 / np.pi) * (x + 0.044715 * x**3)))
```

`np.sqrt(2 / np.pi)` is evaluated while tracing, since its argument is a constant. Everything depending on `x` becomes part of the tensor graph. In the MLIR builder, arithmetic goes into scalar regions of `linalg.generic`; tanh becomes `math.tanh`, and exponentiation becomes `math.powf`.

``` python
from tinymlir import jit

compiled_gelu = jit(gelu)
x = np.linspace(-5, 5, 31, dtype=np.float32)
y = compiled_gelu(x)
expected = 0.5 * x * (
    1 + np.tanh(np.sqrt(2 / np.pi) * (x + 0.044715 * x**3))
)
np.testing.assert_allclose(y, expected, rtol=2e-6, atol=2e-6)
```

There is a subtle detail here: the NumPy scalar produced by `np.sqrt` has dtype float64. In the reference environment, multiplying it by the float32 expression promotes that part of the computation. The resulting GELU output is float64. Our frontend retains this promotion through explicit cast nodes.

It would be reasonable to design an all-float32 model, but it would be a change to the reference computation. For this series, the reference's behavior is what we're reproducing. Matching its formulas includes their dtypes and evaluation order.

## Reductions

An elementwise operation can compute each output independently. A sum has to combine several input elements:

$$y_i=\sum_{j=0}^{N-1}x_{ij}$$

The frontend represents this as a reduction node with an axis. Its type rule removes the reduced dimension. Reducing a `(3, 7)` tensor over axis 1 produces shape `(3,)`. With `keepdims=True`, we reshape that result to `(3, 1)`, making subsequent broadcasting straightforward.

Our builder uses `linalg.reduce`. Here is the central construction, with `operand`, `output_type`, `axis` and element type supplied by the expression:

``` python
zero = arith.ConstantOp(element_type, 0.0).result
initial = tensor.EmptyOp(output_shape, element_type).result
filled = linalg.fill(zero, outs=[initial])
reduction = linalg.ReduceOp(
    [output_type], [operand], [filled], [axis]
)
block = reduction.regions[0].blocks.append(element_type, element_type)
with ir.InsertionPoint(block):
    total = arith.AddFOp(*block.arguments)
    linalg.YieldOp([total.result])
```

The initial tensor must contain zeros. `tensor.empty` gives us a destination with a shape; it doesn't initialize an accumulator. This distinction matters for both reductions and matrix multiplication.

The body receives an input element and the current accumulated value. It adds them and yields the new accumulator. Maximum reduction uses the same structure, initialized to negative infinity, with `arith.maximumf` in the region.

The operation still describes a reduction, rather than specifying which processor or thread computes each addition. That leaves scheduling decisions to later transformations. Our initial native backend eventually lowers it to loops.

## Stable Softmax

Softmax turns a row of scores into probabilities:

$$p_j=\frac{e^{x_j}}{\sum_k e^{x_k}}$$

Directly exponentiating large scores can overflow. Subtracting the row maximum leaves the result unchanged in real arithmetic while keeping the largest exponential equal to one:

$$m=\max_k x_k,\qquad p_j=\frac{e^{x_j-m}}{\sum_k e^{x_k-m}}$$

The Python definition is a composition of operations we now have:

``` python
from tinymlir import exp


def softmax(x):
    numerator = exp(x - x.max(axis=-1, keepdims=True))
    return numerator / numerator.sum(axis=-1, keepdims=True)
```

For an input of shape `(H, T, T)`, the maximum and sum have shape `(H, T, 1)`. Their elementwise consumers broadcast that final unit dimension. The same function also works on an ordinary two-dimensional matrix; the frontend derives the appropriate types and maps for each specialization.

The compiler retains the shared `numerator` expression. It doesn't independently rebuild the exponential for each Python use. Whether a later transformation chooses to recompute or materialize an intermediate is a separate optimization question.

We can test cases which would break the direct exponential formula:

``` python
x = np.array([
    [10000, 9999, -10000],
    [-10000, -9999, -10001],
    [0, -np.inf, -np.inf],
], dtype=np.float32)
probabilities = jit(softmax)(x)
np.testing.assert_allclose(probabilities.sum(axis=-1), 1, atol=1e-6)
assert probabilities[2, 0] == 1
```

A row containing only negative infinity is outside this example's useful domain: subtracting its maximum gives undefined values. GPT-2's causal mask always leaves the diagonal available, so a nonempty query row has at least one permitted key. We'll make that property explicit when we build attention.

## Layer Normalization

Layer normalization computes statistics across the last dimension of each token's representation:

$$\mu_i=\frac{1}{C}\sum_jx_{ij},\qquad
\sigma_i^2=\frac{1}{C}\sum_j(x_{ij}-\mu_i)^2$$

It then applies learned scale and bias vectors:

$$y_{ij}=g_j\frac{x_{ij}-\mu_i}{\sqrt{\sigma_i^2+\epsilon}}+b_j$$

We use population variance and GPT-2's $\epsilon=10^{-5}$:

``` python
from tinymlir import sqrt


def layer_norm(x, gain, bias):
    centered = x - x.mean(axis=-1, keepdims=True)
    variance = (centered * centered).mean(axis=-1, keepdims=True)
    return gain * centered / sqrt(variance + 1e-5) + bias
```

`mean` is itself a sum divided by the statically known reduction length. There is no separate normalization loop in the backend. This function expands into reductions, elementwise operations and broadcast maps, using the same lowering rules as the earlier examples.

Using centered differences is also important numerically. The algebraically equivalent expression $E[x^2]-E[x]^2$ can suffer severe cancellation for values with a large common offset. We follow the reference's centered computation rather than choosing a more convenient-looking identity.

A constant row is a useful test. Its variance is zero, and after normalization the output should equal the learned bias. The test suite combines constant rows with random rows so that a special case doesn't conceal an error in the general path.

## What the Optimizer Sees

At this point, the compiler sees more than a sequence of calls to named kernels. The entire Python expression becomes a tensor graph inside a function. A subtraction followed by an exponential and division appears as structured operations whose regions and indexing maps can be composed.

We can inspect the difference before and after fusion:

```
uv run python emit.py softmax
uv run python emit.py softmax --stage optimized
```

Reductions place constraints on fusion. Computing a row sum requires the values contributing to that row. A pass must preserve the producer-consumer relationship and reduction semantics; it can't move every operation into an arbitrary elementwise region. Keeping those semantics explicit is why we're using structured IR in the first place.

## Numerical Checks

The small operation tests compare against NumPy with tight tolerances and check output dtypes. They cover multiple reduction axes, retained dimensions, broadcasted vectors, large softmax offsets, constant normalization rows and GELU's mixed precision.

```
uv run pytest
```

These tests establish the elementary pieces. The final model test will also compare intermediate residual outputs and logits against the original `tiny-gpt2` repository, followed by exact equality of the generated token IDs. An implementation can make a local rounding difference and still be numerically correct. Selecting a different token for the required example is a separate, observable failure.

Next we'll add matrix multiplication and express the attention computation using these same composable tensor operations.

## External Resources

- [GPT-2 model implementation](https://github.com/openai/gpt-2/blob/master/src/model.py)
- [MLIR Linalg dialect](https://mlir.llvm.org/docs/Dialects/Linalg/)
- [MLIR Math dialect](https://mlir.llvm.org/docs/Dialects/MathOps/)
- [NumPy GPT-2 reference](https://github.com/sdiehl/tiny-gpt2)

[Previous: Part 9 - A Python Frontend for MLIR](mlir_python_frontend.md) · [Series contents](mlir_introduction.md) · [Next: Part 11 - Matrix Multiplication and Attention](mlir_attention.md)

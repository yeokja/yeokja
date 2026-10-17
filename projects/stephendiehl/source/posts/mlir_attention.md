---
date: "2026-05-01"
tags: ["ai", "python", "llms", "compilers", "mlir"]
url: "https://www.stephendiehl.com/posts/mlir_attention/"
---

<img src="https://www.stephendiehl.com/images/mlir_attention.jpeg" width="500" alt="Attention" />

# Matrix Multiplication and Attention

We have GELU, softmax and layer normalization expressed as Python functions over our tensor language. Most of a transformer's arithmetic, however, comes from matrix multiplication. Adding it will let us assemble attention without introducing a special attention kernel into the compiler.

We'll use $T$ for sequence length, $C$ for embedding width, $H$ for head count and $D=C/H$ for the width of a head. The model processes one sequence at a time. Heads will become a batch dimension in the attention matrix products.

## Preserving the Matrix Operation

For $A$ with shape $(M,K)$ and $B$ with shape $(K,N)$:

$$Y_{ij}=\sum_{k=0}^{K-1}A_{ik}B_{kj}$$

Our frontend's `__matmul__` checks the reduction dimensions, determines the output shape and inserts dtype conversions if necessary. The result is a `matmul` expression. Its builder creates a named `linalg.matmul` operation:

``` python
initial = linalg.fill(
    zero, outs=[tensor.EmptyOp(output_shape, element_type).result]
)
result = linalg.matmul(lhs, rhs, outs=[initial])
```

`zero`, the operands and output type come from the typed expression. The zero initialization is essential because matrix multiplication accumulates into its destination.

At this point we haven't specified a loop order, tile size, vector width or thread mapping. The IR still says that this is a matrix multiplication. The backend can subsequently choose how to implement it. Our CPU pipeline hands it to the system BLAS when one is available and otherwise uses MLIR's loop lowering; the GPU target substitutes a tiled kernel. Both substitutions depend on the operation still being recognizable when they run.

A small rectangular example checks the shape rule and generated operation:

``` python
import numpy as np
from tinymlir import jit

@jit
def multiply(a, b):
    return a @ b

rng = np.random.default_rng(7)
a = rng.normal(size=(3, 5)).astype(np.float32)
b = rng.normal(size=(5, 7)).astype(np.float32)
np.testing.assert_allclose(multiply(a, b), a @ b, rtol=2e-6, atol=2e-6)
print(multiply.compile(a, b).module)
```

Rectangular inputs help reveal swapped dimensions and incorrect strides. A collection of square examples can hide those errors surprisingly well.

For rank-three operands with matching batch dimensions, the builder selects `linalg.batch_matmul`. The frontend currently requires those batch dimensions to agree exactly; it doesn't implement the full range of NumPy's batched broadcasting rules.

## Linear Layers

A linear layer is simply:

``` python
def linear(x, weight, bias):
    return x @ weight + bias
```

The matrix product produces shape `(T, N)`, and the vector bias broadcasts over its rows. GPT-2's projection weights have shape `(input_width, output_width)`, so they fit this expression directly.

This definition also illustrates why the frontend matters. Composing `gelu(linear(x, weight, bias))` exposes the whole computation to the compiler. There is no separately maintained `linear_gelu` implementation. The tensor operations provide the material on which MLIR's fusion pass works.

## Queries, Keys and Values

GPT-2 computes all three projections at once:

$$[Q;K;V]=XW_{qkv}+b_{qkv}$$

The result has shape `(T, 3*C)`. We take three slices, reshape each into `(T, H, D)`, and transpose to `(H, T, D)`:

``` python
packed = linear(x, qkv_weight, qkv_bias)
length, width = x.shape
depth = width // heads
q, k, v = [
    packed[:, i*width:(i+1)*width]
        .reshape((length, heads, depth))
        .transpose((1, 0, 2))
    for i in range(3)
]
```

These statements run during tracing. The loop has a static bound, and all its slices and shapes are known. The arrays themselves remain runtime inputs.

Each operation has a corresponding structured lowering. Slices become `tensor.extract_slice`. Reshapes become `tensor.reshape` with a constant shape operand. Transposes become `linalg.transpose` with a permutation attribute.

The choice of reshape operation is deliberate. MLIR also has `tensor.collapse_shape` and `tensor.expand_shape`, which carry explicit reassociation groups and are the natural fit for a reshape between two static shapes. But the elementwise fusion pass knows how to fold those operations into an adjacent `linalg.matmul`, rewriting the named operation into a `linalg.generic` whose indexing maps absorb the reshape. That transformation is legal and saves an operation. It also erases the fact that the operation was a matrix multiplication, which is exactly what the BLAS and GPU substitutions in the next chapters look for. A `tensor.reshape` with a shape operand isn't a reassociation the fusion pass recognizes, so the matmul survives to the point where a backend can claim it.

A reshape must preserve element count. A transpose must specify a permutation. A slice must fit the static shape and, in our small language, have unit stride and a nonempty result. Those checks happen before creating the MLIR operation.

The layout operations retain their meaning until bufferization and lowering. Whether a particular combination becomes a view or requires a copy depends on its layout and uses. We shouldn't assume every transpose is free merely because the Python expression is short.

## Scores and the Mask

For each head, attention scores are:

$$S_h=\frac{Q_hK_h^\top}{\sqrt{D}}+M$$

The batched product has shape `(H, T, T)`. The causal mask has shape `(T, T)` and broadcasts across heads. To match the NumPy reference, allowed entries are zero and future positions contain $-10^{10}$ in the projection's dtype.

Our mask is a small primitive built with `linalg.generic`. Its region obtains the row and column indices using `linalg.index`, compares them with `arith.cmpi`, and selects zero or the mask value. This is index-dependent tensor construction; it doesn't require a host NumPy array containing the mask.

The diagonal remains allowed, so every query position has at least one visible key. Applying the softmax from Part 10 normalizes the final dimension:

``` python
scores = (q @ k.transpose((0, 2, 1))) / np.sqrt(depth)
probabilities = softmax(scores + causal_mask(length, packed.dtype))
```

The NumPy square root returns a float64 scalar. Our type rules therefore promote the score computation as in the reference. This propagation is visible in the IR as explicit elementwise casts, rather than being hidden in a runtime conversion.

## Combining Values

The probabilities weight the value vectors:

$$O_h=P_hV_h$$

The result has shape `(H, T, D)`. Transposing it to `(T, H, D)` and reshaping to `(T, C)` combines the heads. A final linear projection returns to the residual stream's width:

``` python
context = (probabilities @ v).transpose((1, 0, 2)).reshape((length, width))
return linear(context, out_weight, out_bias)
```

Putting these pieces together gives the entire attention function in `tinymlir/ops.py`. Its backend vocabulary consists of matrix products, elementwise arithmetic, reductions, slices, reshapes and transposes. The compiler doesn't need a special rule which recognizes the model's attention function by name.

This version materializes the score and probability tensors. Their size is proportional to $HT^2$. The code is compact, but the algorithm still has quadratic attention storage. FlashAttention would require a different schedule that processes tiles and maintains the normalization statistics without materializing those full tensors. That isn't a consequence of ordinary elementwise fusion.

## Residual Connections

The transformer normalizes its input before attention, then adds the attention output back to the input:

``` python
@jit
def attend(x, gain, bias, qkv_weight, qkv_bias, out_weight, out_bias, *, heads):
    normalized = layer_norm(x, gain, bias)
    return x + attention(
        normalized, qkv_weight, qkv_bias, out_weight, out_bias, heads
    )
```

`heads` is a static keyword parameter. Changing it selects another specialization. The tensors are dynamic data whose shapes and dtypes determine their specialization.

The feedforward half has the same residual structure:

``` python
@jit
def feedforward(x, gain, bias, up_weight, up_bias, down_weight, down_bias):
    normalized = layer_norm(x, gain, bias)
    hidden = gelu(linear(normalized, up_weight, up_bias))
    return x + linear(hidden, down_weight, down_bias)
```

These are the functions which the final model executes. Each contains several composable tensor operations in a single module, giving the compiler useful producer-consumer relationships to transform.

## Checking Causality

Numerical comparison is necessary, but attention also has a useful structural property: changing future inputs must leave earlier outputs unchanged.

The test suite creates a short sequence, runs compiled attention, changes the last two input rows, and checks that the first three output rows remain equal. This catches a reversed mask, the wrong transpose, and some head-layout mistakes which shape checks alone can't detect.

It also tests nonsquare matrix products, batched products and layout changes on nontrivial slices. These are independent places for indexing errors to enter the model. Checking them separately makes a mismatch in the final logits much easier to locate.

We now have the arithmetic needed for a GPT-2 block. Next we'll examine the transformation and execution pipeline that turns this structured tensor program into callable native code.

## External Resources

- [GPT-2 model implementation](https://github.com/openai/gpt-2/blob/master/src/model.py)
- [MLIR Linalg dialect](https://mlir.llvm.org/docs/Dialects/Linalg/)
- [MLIR Tensor dialect](https://mlir.llvm.org/docs/Dialects/TensorOps/)
- [NumPy GPT-2 reference](https://github.com/sdiehl/tiny-gpt2)

[Previous: Part 10 - Compiling Transformer Kernels](mlir_transformer_kernels.md) · [Series contents](mlir_introduction.md) · [Next: Part 12 - Fusion, Bufferization and Execution](mlir_optimization.md)

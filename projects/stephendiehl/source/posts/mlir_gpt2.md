---
date: "2026-06-19"
tags: ["ai", "python", "llms", "compilers", "mlir"]
url: "https://www.stephendiehl.com/posts/mlir_gpt2/"
---

<img src="https://www.stephendiehl.com/images/mlir_gpt2.jpeg" width="500" alt="GPT-2" />

# Running GPT-2

We now have a Python tensor language, an MLIR builder, a transformation pipeline and a native execution boundary. The remaining task is to connect those pieces to the pretrained model and check that the result agrees with our original NumPy implementation.

The example uses the small GPT-2 checkpoint: 12 transformer blocks, embedding width 768, 12 attention heads and a vocabulary of 50,257 tokens. The companion project's download script pins a specific checkpoint revision so a later change to a remote file doesn't quietly change the example.

From the [`tiny-mlir`](https://github.com/sdiehl/tiny-mlir) checkout:

```
uv sync
uv run python fetch_model.py model
uv run python example.py "Alan Turing theorized that computers would one day become" --tokens 10
```

The model download is approximately 550 MB. Compilation and execution use the MLIR Python bindings installed by `uv sync`; the example doesn't need a separate command-line LLVM installation.

## The Top-Level Example

The entry point accepts a prompt, generation length and optional checkpoint directory. After parsing those arguments, it loads the model and prints the generated text:

``` python
model, tokenizer = load_model(args.model)
print(generate(model, tokenizer, args.prompt, args.tokens))
```

For the reference example, pass the prompt directly to the command line:

```
uv run python example.py "Alan Turing theorized that computers would one day become" --tokens 10
```

The completion from both the compiled model and the NumPy reference is:

```
Alan Turing theorized that computers would one day become the most powerful machines on the planet.
```

There are two newline tokens after the period. They aren't visible in an ordinary rendered sentence, but they are part of the ten-token output. We'll check the token IDs as well as the decoded text.

## Loading Parameters

`load_model` reads `config.json`, `model.safetensors` and `tokenizer.json`. The model constructor checks every required parameter's shape and requires float32 checkpoint arrays. It also checks that the embedding width is divisible by the head count. The loader rejects a different activation or layer-normalization epsilon rather than quietly applying the wrong formulas.

For a block with embedding width $C$, the important projection shapes are:

| Parameter                     | Shape    |
|-------------------------------|----------|
| Query/key/value projection    | $(C,3C)$ |
| Attention output projection   | $(C,C)$  |
| Feedforward expansion         | $(C,4C)$ |
| Feedforward output projection | $(4C,C)$ |

Each projection also has a bias vector matching its output width. The two layer normalizations have gain and bias vectors of length $C$.

The arrays remain runtime inputs to compiled functions. We don't compile a separate copy of the same block for each set of weight values. Compatible shapes and dtypes share a specialization.

## Embeddings

The initial residual stream adds a token embedding and a learned position embedding:

$$X_i=W_{\mathrm{token}}[t_i]+W_{\mathrm{position}}[i]$$

This is another small compiled function:

``` python
@jit
def embed(ids, words, positions):
    return gather(words, ids) + positions[:ids.shape[0]]
```

The gather operation's column indexing is affine, while its row index comes from the token-ID tensor. Its `linalg.generic` region loads the selected table entry with `tensor.extract`. The model checks token IDs against the vocabulary before entering native code; the JIT's gather boundary checks indices too.

Position embeddings are a static slice for the current sequence length. The token sequence must be nonempty and fit the configured context window.

## The Transformer Stack

Each block runs the two compiled functions from Part 11:

```
x = x + attention(layer_norm(x))
x = x + feedforward(layer_norm(x))
```

The actual calls pass each block's normalization parameters and projection arrays. `attend` and `feedforward` are ordinary Python definitions whose tensor expressions are traced and lowered together. The model loop itself stays in Python, making the sequence of blocks and their parameters easy to inspect.

The first attention projection starts from float32 embeddings. The reference's arithmetic promotes later values to float64, and our typed graph follows that behavior. This means the first block can require a different specialization from subsequent blocks even when their dimensions agree.

After the final block, we normalize and project to the vocabulary using the same token-embedding matrix:

``` python
@jit
def project(x, gain, bias, words):
    return layer_norm(x[-1:], gain, bias) @ words.T
```

Only the final position's logits are needed for next-token selection. This function therefore returns shape `(1, vocab_size)`. The reference computes logits for every position; the verification compares its last row with our result.

## Decoding

The example uses greedy decoding. At each step, it chooses the index of the largest logit, appends that token, and evaluates the longer prefix:

``` python
for _ in range(max_tokens):
    ids.append(int(np.argmax(model(ids)[0])))
```

Tokenization, orchestration and the final `argmax` remain on the Python side. The model's embedding lookup, normalization, attention, projections, activation and residual arithmetic execute through compiled MLIR.

Like the NumPy reference's `generate_tokens`, this loop produces the requested number of tokens without an early stop on the end-of-text token. The model validates that the prompt and requested generation fit its context limit.

We haven't implemented a key/value cache. Each step recomputes the prefix, and each new length selects new static tensor shapes. This is a direct implementation of the reference algorithm, useful for following the compiler. A decoding cache would change the model interface and the attention shapes, and should be validated as a separate change.

## The Whole Model as One Kernel

The per-sublayer path above is the one this series has been building. The project has a second mode, `--fused`, which compiles the forward pass of the NumPy reference itself. The `gpt2` function from `tiny-gpt2` is traced as written, through the array protocols from Part 9, with its `np.mean` normalizations, `np.split` projections, `np.triu` mask and nested parameter tuples. The result is one module containing all twelve blocks, so fusion and the matmul substitution see the whole model at once.

A kernel per sequence length would mean a compilation per generated token. The fused mode instead pads the token sequence to the next power of two. The causal mask keeps the real positions from attending to the padding, so the row of logits for the last real token is unchanged, and a ten-token generation compiles a couple of modules rather than ten. Part 14 looks at this mode in more detail. It matters here because it changes what the verification below can see.

## Checking the Actual Reference

A matching sentence is a useful result, but it doesn't tell us where numerical mistakes might have cancelled or gone unnoticed. The verification script runs the original `tiny-gpt2` repository in its own Python environment and captures the model at every decoding step.

```
uv run python verify.py
```

The script downloads a pinned revision of the public NumPy reference, installs its locked environment with uv, and fetches the matching tokenizer files. It requires Git and network access, but no sibling checkout or manual reference setup. `--reference /path/to/tiny-gpt2` optionally selects an existing checkout. Both implementations use the same pinned checkpoint. The script adapts its arrays to the reference's parameter dataclasses, then calls the reference's existing numerical operations unchanged. It also checks its captured trace against the reference's complete `gpt2` function.

For each of the ten steps, the comparison covers:

- Token and position embeddings.
- Both residual outputs in every transformer block.
- The final position's vocabulary logits.
- The selected token ID.

In the fused mode the forward pass is one compiled function with no intermediate values to capture, so only the final logits and the chosen token are compared. The per-block checks belong to the per-sublayer path. Continuous integration runs both modes, and the GPU target runs the same script with `--gpu`.

It requires the same dtypes and compares arrays with `rtol=3e-4` and `atol=3e-4`. Token IDs and the decoded completion require exact equality. The test run against the original repository's NumPy 2.4.4 environment had a largest absolute difference of approximately $8.65\times10^{-6}$ across all captured arrays.

The generated IDs were exactly:

```
[262, 749, 3665, 8217, 319, 262, 5440, 13, 198, 198]
```

And the completion, with whitespace made explicit, was:

``` python
' the most powerful machines on the planet.\n\n'
```

This is numerical agreement for the intermediate floating-point computation and exact agreement for the required observable output. We aren't claiming bit-for-bit equality of every float or that every possible prompt has been tested.

## What We've Built

The complete package fits in a handful of modules. `expr.py` contains the typed tensor graph, `builder.py` constructs MLIR operations, `jit.py` handles specialization and compiler passes, `ops.py` contains the transformer formulas, and `model.py` connects them to the checkpoint and decoding loop. Three more modules build on that path without changing it: `blas.py` hands matmuls to the system BLAS, `fused.py` traces the reference as a single kernel, and `gpu.py` targets NVIDIA hardware.

The important property is that the compiler understands the constituents of the computation. Matrix multiplication remains a matrix operation, reductions have explicit axes and regions, and broadcasts have indexing maps. GELU and attention are compositions of these primitives. MLIR's passes can transform that structure before storage and execution details are fixed.

Everything in this chapter ran on the CPU. The executable endpoint is a small native compiler which reproduces the NumPy model's required output, and the reference checks give us a concrete way to judge further changes to the compiler. The next chapter makes one such change: the same tensor programs, the same formulas and the same checks, on a GPU.

That completes the path from Python tensor expressions to a running pretrained transformer. Congratulations on building a working MLIR compiler and running GPT-2 with it!

## External Resources

- [NumPy GPT-2 reference](https://github.com/sdiehl/tiny-gpt2)
- [GPT-2 source](https://github.com/openai/gpt-2/blob/master/src/model.py)
- [Pinned GPT-2 checkpoint](https://huggingface.co/openai-community/gpt2/tree/607a30d783dfa663caf39e06633721c8d4cfcd7e)
- [MLIR Python bindings](https://mlir.llvm.org/docs/Bindings/Python/)

[Previous: Part 12 - Fusion, Bufferization and Execution](mlir_optimization.md) · [Series contents](mlir_introduction.md) · [Next: Part 14 - GPT-2 on the GPU](mlir_gpu_target.md)

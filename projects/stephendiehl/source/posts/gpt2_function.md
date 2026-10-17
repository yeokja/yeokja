---
date: "2023-04-29"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/gpt2_function/"
---

# GPT-2 in One Function

GPT-2 is not a complicated model, in fact we can code golf it down into a single Python function using nothing more than numpy. This is everything inlined in a single function, the layernorm, attention, mlp, token and position embeddings, and the final logits in 70 lines of code if you don't count whitespace.

``` python
import numpy as np

def gpt2(inputs: list[int], params: ModelParams, n_head: int) -> np.ndarray:
    x = params.wte[inputs] + params.wpe[range(len(inputs))]

    seq_len = x.shape[0]
    embedding_dim = x.shape[-1]
    head_size = embedding_dim // n_head

    for block_params in params.blocks:
        ln1_mean = np.mean(x, axis=-1, keepdims=True)
        ln1_variance = np.var(x, axis=-1, keepdims=True)
        ln1_normalized = (x - ln1_mean) / np.sqrt(ln1_variance + 1e-5)
        ln1_output = block_params.ln_1.g * ln1_normalized + block_params.ln_1.b

        qkv_proj = ln1_output @ block_params.attn.c_attn.w + block_params.attn.c_attn.b
        q_proj, k_proj, v_proj = np.split(qkv_proj, 3, axis=-1)

        q_heads = q_proj.reshape(seq_len, n_head, head_size)
        k_heads = k_proj.reshape(seq_len, n_head, head_size)
        v_heads = v_proj.reshape(seq_len, n_head, head_size)

        q_heads_t = q_heads.transpose(1, 0, 2)
        k_heads_t = k_heads.transpose(1, 0, 2)
        v_heads_t = v_heads.transpose(1, 0, 2)

        attention_scores = (q_heads_t @ k_heads_t.transpose(0, 2, 1)) / np.sqrt(
            head_size
        )

        causal_mask = np.triu(np.ones((seq_len, seq_len), dtype=x.dtype) * -np.inf, k=1)
        attention_scores = attention_scores + causal_mask

        exp_scores = np.exp(
            attention_scores - np.max(attention_scores, axis=-1, keepdims=True)
        )
        attention_weights = exp_scores / np.sum(exp_scores, axis=-1, keepdims=True)

        weighted_values = attention_weights @ v_heads_t

        merged_heads = weighted_values.transpose(1, 0, 2).reshape(
            seq_len, embedding_dim
        )

        mha_output = (
            merged_heads @ block_params.attn.c_proj.w + block_params.attn.c_proj.b
        )

        x = x + mha_output

        ln2_mean = np.mean(x, axis=-1, keepdims=True)
        ln2_variance = np.var(x, axis=-1, keepdims=True)
        ln2_normalized = (x - ln2_mean) / np.sqrt(ln2_variance + 1e-5)
        ln2_output = block_params.ln_2.g * ln2_normalized + block_params.ln_2.b

        fc_output = ln2_output @ block_params.mlp.c_fc.w + block_params.mlp.c_fc.b

        gelu_output = (
            0.5
            * fc_output
            * (1 + np.tanh(np.sqrt(2 / np.pi) * (fc_output + 0.044715 * fc_output**3)))
        )

        ffn_output = gelu_output @ block_params.mlp.c_proj.w + block_params.mlp.c_proj.b

        x = x + ffn_output

    lnf_mean = np.mean(x, axis=-1, keepdims=True)
    lnf_variance = np.var(x, axis=-1, keepdims=True)
    lnf_normalized = (x - lnf_mean) / np.sqrt(lnf_variance + 1e-5)
    x_normalized_final = params.ln_f.g * lnf_normalized + params.ln_f.b

    logits = x_normalized_final @ params.wte.T

    return logits
```

Of course, here I'm cheating a little bit because we have a helper module to load the weights into a dataclass `ModelParams`, but the logic for the forward pass is all contained within this single function. Here are the files for the full implementation:

- [gpt2_tensors.py](https://github.com/sdiehl/tiny-gpt2/blob/main/tinygpt2/gpt2_tensors.py)
- [gpt2_minimal.py](https://github.com/sdiehl/tiny-gpt2/blob/main/tinygpt2/gpt2_minimal.py)
- [gpt2_run.py](https://github.com/sdiehl/tiny-gpt2/blob/main/tinygpt2/gpt2_run.py)

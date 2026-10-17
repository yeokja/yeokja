---
date: "2025-04-29"
tags: ["ai", "deep learning", "transformers", "python"]
url: "https://www.stephendiehl.com/posts/post_transformers/"
---

# Attention Wasn't All We Needed

There's a lot of modern techniques that have been developed since the original *Attention Is All You Need* paper. Let's look at some of the most important ones that have been developed over the years and try to implement the basic ideas as succinctly as possible. We'll use the Pytorch framework for most of the examples. Note that most of these examples are highly simplified sketches of the core ideas, if you want the full implementation please read the original paper or the production code in frameworks like PyTorch or Jax.

1.  [Group Query Attention](#group-query-attention)
2.  [Multi-head Latent Attention](#multi-head-latent-attention)
3.  [Flash Attention](#flash-attention)
4.  [Ring Attention](#ring-attention)
5.  [Pre-normalization](#pre-normalization)
6.  [RMSNorm](#rmsnorm)
7.  [SwiGLU](#swiglu)
8.  [Rotary Positional Embedding](#rotary-positional-embedding)
9.  [Mixture of Experts](#mixture-of-experts)
10. [Learning Rate Warmup](#learning-rate-warmup)
11. [Cosine Schedule](#cosine-schedule)
12. [AdamW Optimizer](#adamw-optimizer)
13. [Multi-token Prediction](#multi-token-prediction)
14. [Speculative Decoding](#speculative-decoding)

## Group Query Attention

Ok starting off in no particular order, **Grouped Query Attention** is a technique to reduce the memory usage of the KV cache during inference. Group Query Attention is an architectural optimization for the standard multi-head attention mechanism. The core idea behind GQA is based on the observation that the computational bottleneck and memory footprint in MHA are heavily influenced by the size of the K and V projections and their corresponding caches. GQA proposes to reduce this cost by sharing a single set of K and V projections across multiple Q heads. Instead of having $N_h$ distinct heads for Q, K, and V (as in MHA), GQA uses $N_h$ query heads but only $N_{kv}$ key/value heads, where $N_{kv} < N_h$ and $N_h$ is typically a multiple of $N_{kv}$. These $N_h$ query heads are divided into $N_{kv}$ groups, with each group of $N_h / N_{kv}$ query heads attending to the *same* key and value head. This structure significantly reduces the parameter count for `K` and `V` projection matrices and, more importantly, shrinks the size of the K/V cache needed during autoregressive decoding.

Let the input sequence representation be $X \in \mathbb{R}^{L \times d_{\text{model}}}$, where $L$ is the sequence length and $d_{\text{model}}$ is the embedding dimension. GQA first projects $X$ into queries, keys, and values using different linear transformations: $Q = XW_Q$, $K = XW_K$, and $V = XW_V$. Here, $W_Q \in \mathbb{R}^{d_{\text{model}} \times (N_h d_k)}$, $W_K \in \mathbb{R}^{d_{\text{model}} \times (N_{kv} d_k)}$, and $W_V \in \mathbb{R}^{d_{\text{model}} \times (N_{kv} d_k)}$, where $d_k$ is the dimension of each head (`head_dim`). These are reshaped into $N_h$ query heads $Q_i \in \mathbb{R}^{L \times d_k}$ ($i=1...N_h$) and $N_{kv}$ key/value heads $K_j, V_j \in \mathbb{R}^{L \times d_k}$ ($j=1...N_{kv}$). The key step in GQA is sharing: for the $i$-th query head, the corresponding key and value heads are $K_{\lceil i / g \rceil}$ and $V_{\lceil i / g \rceil}$, where $g = N_h / N_{kv}$ is the group size (number of queries per KV head). The attention output for the $i$-th head is computed as:

$$\text{Attention}(Q_i, K_{\lceil i / g \rceil}, V_{\lceil i / g \rceil}) = \text{softmax}\left(\frac{Q_i K_{\lceil i / g \rceil}^T}{\sqrt{d_k}}\right)V_{\lceil i / g \rceil}$$

In implementation, we do this by computing the $N_{kv}$ key/value heads and then repeating or interleaving them $g$ times to match the $N_h$ query heads before the batched matrix multiplication for attention scores, as shown in the [`repeat_interleave`](https://pytorch.org/docs/stable/generated/torch.repeat_interleave.html) step in the example code. Finally, the outputs of all $N_h$ heads are concatenated and passed through an output projection $W_O$.

GQA is primarily used as a technique to accelerate inference speed and reduce memory requirements without significantly compromising model performance. During autoregressive generation, the previously computed keys and values for the context sequence are cached and reused for subsequent token predictions. The size of this K/V cache is directly proportional to the number of K/V heads ($N_{kv}$ in GQA, $N_h$ in MHA). By reducing $N_{kv}$, GQA drastically cuts down the memory bandwidth needed to load the K/V cache at each decoding step, which is the main performance bottleneck.

While it might slightly reduce the model's representational capacity compared to MHA (as K/V projections are shared), empirical results show that GQA achieves a favorable trade-off, maintaining most of the quality of MHA while offering substantial speedups and memory savings, making it popular for deploying large models efficiently. Multi-query attention (MQA), where $N_{kv}=1$, is an extreme form of GQA.

``` python
class GroupQueryAttention(nn.Module):
    def __init__(self, dim, num_heads, num_kv_heads=None, head_dim=64, dropout=0.0):
        super().__init__()
        self.dim = dim
        self.num_heads = num_heads
        self.num_kv_heads = num_kv_heads if num_kv_heads else num_heads
        self.head_dim = head_dim

        # Ensure num_heads is divisible by num_kv_heads
        assert self.num_heads % self.num_kv_heads == 0, "num_heads must be divisible by num_kv_heads"

        # Number of queries per key-value head
        self.num_queries_per_kv = self.num_heads // self.num_kv_heads

        # Projections
        self.q_proj = nn.Linear(dim, num_heads * head_dim)
        self.k_proj = nn.Linear(dim, self.num_kv_heads * head_dim)
        self.v_proj = nn.Linear(dim, self.num_kv_heads * head_dim)
        self.o_proj = nn.Linear(num_heads * head_dim, dim)
        self.dropout = nn.Dropout(dropout)

    def forward(self, x, mask=None):
        batch_size, seq_len, _ = x.shape

        # Project to queries, keys, values
        q = self.q_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        k = self.k_proj(x).reshape(batch_size, seq_len, self.num_kv_heads, self.head_dim)
        v = self.v_proj(x).reshape(batch_size, seq_len, self.num_kv_heads, self.head_dim)

        # Transpose for attention computation
        q = q.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        k = k.transpose(1, 2)  # [batch_size, num_kv_heads, seq_len, head_dim]
        v = v.transpose(1, 2)  # [batch_size, num_kv_heads, seq_len, head_dim]

        # Repeat k,v for each query head in the group
        k = k.repeat_interleave(self.num_queries_per_kv, dim=1)
        v = v.repeat_interleave(self.num_queries_per_kv, dim=1)

        # Scaled dot-product attention
        scale = 1.0 / math.sqrt(self.head_dim)
        attn = torch.matmul(q, k.transpose(2, 3)) * scale

        # Apply mask if provided
        if mask is not None:
            attn = attn.masked_fill(mask == 0, -1e9)

        # Softmax and dropout
        attn = torch.softmax(attn, dim=-1)
        attn = self.dropout(attn)

        # Apply attention to values
        out = torch.matmul(attn, v)  # [batch_size, num_heads, seq_len, head_dim]
        out = out.transpose(1, 2).reshape(batch_size, seq_len, self.num_heads * self.head_dim)

        # Output projection
        out = self.o_proj(out)

        return out
```

## Multi-head Latent Attention

**Multi-head Latent Attention** introduces a set of learnable "latent" vectors that act as an intermediary bottleneck between the input sequence elements. The core idea is to alleviate the quadratic computational cost $O(L^2)$, where $L$ is the sequence length, inherent in standard self-attention mechanisms. Instead of allowing every input element to attend directly to every other element, inputs first attend to a fixed number of latent units ($N_{\text{latents}}$), and these latents then attend back to the inputs (or variations thereof). This effectively decouples the direct interaction within the long input sequence, replacing it with two cross-attention steps involving the much smaller set of latents. This approach assumes that the essential information from the input sequence can be effectively summarized or compressed into these latent representations, thus maintaining representational power while significantly reducing computation, especially when $N_{\text{latents}} \ll L$.

The mechanism involves two main stages of attention computation, typically within a multi-head framework. Let the input sequence be $X \in \mathbb{R}^{L \times d}$ and the learnable latent array be $L \in \mathbb{R}^{N_{\text{latents}} \times d}$. Both $X$ and $L$ are projected into Q, K, and V using shared or separate projection matrices. Let's denote the input projections as $Q_X, K_X, V_X$ and latent projections as $Q_L, K_L, V_L$, split across multiple heads. The first cross-attention step computes how latents attend to the input: the latent queries $Q_L$ attend to the input keys $K_X$ and aggregate information from input values $V_X$. The attention output for the latents is

$$H_L = \text{Attention}(Q_L, K_X, V_X) = \text{softmax}\left(\frac{Q_L K_X^T}{\sqrt{d_k}}\right) V_X$$

Where $d_k$ is the head dimension. In the second cross-attention step, the input queries $Q_X$ attend to the keys derived from the latents (e.g., $K_L$) and aggregate information from the values associated with the latents (which could be $V_L$ or, as implemented in the example code, the updated latent representation $H_L$). The final output $O$ is then

$$O = \text{Attention}(Q_X, K_L, H_L) = \text{softmax}\left(\frac{Q_X K_L^T}{\sqrt{d_k}}\right) H_L$$

These operations are performed independently for each head, and the results are concatenated and passed through a final linear projection.

Multi-head Latent Attention is primarily employed in architectures designed to handle very long sequences or high-dimensional inputs where standard self-attention is computationally infeasible. By using a fixed number of latents ($N_{\text{latents}}$), the computational complexity is reduced from $O(L^2)$ to $O(L \cdot N_{\text{latents}})$, making it scalable to much larger inputs. The learnable latent vectors $L$ (initialized randomly and updated via backpropagation, as seen in `self.latents = nn.Parameter(...)` in the code) adapt during training to function as a compressed representation or memory bank relevant to the task. While this introduces an information bottleneck, potentially limiting fine-grained local interactions compared to full self-attention, it excels at capturing global context efficiently and has proven effective in various modalities, enabling Transformer-like architectures to be applied to previously challenging domains due to sequence length constraints.

``` python
class MultiHeadLatentAttention(nn.Module):
    def __init__(self, dim, num_heads, num_latents=64, head_dim=64, dropout=0.0):
        super().__init__()
        self.dim = dim
        self.num_heads = num_heads
        self.num_latents = num_latents
        self.head_dim = head_dim

        # Projections
        self.q_proj = nn.Linear(dim, num_heads * head_dim)
        self.k_proj = nn.Linear(dim, num_heads * head_dim)
        self.v_proj = nn.Linear(dim, num_heads * head_dim)
        self.o_proj = nn.Linear(num_heads * head_dim, dim)

        # Latent vectors (learned)
        self.latents = nn.Parameter(torch.randn(1, num_latents, dim))

        self.dropout = nn.Dropout(dropout)

    def forward(self, x, mask=None):
        batch_size, seq_len, _ = x.shape

        # Get latents for this batch
        latents = self.latents.expand(batch_size, -1, -1)

        # Project inputs to queries, keys, values
        q_x = self.q_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        k_x = self.k_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        v_x = self.v_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)

        # Project latents to queries, keys, values
        q_latents = self.q_proj(latents).reshape(batch_size, self.num_latents, self.num_heads, self.head_dim)
        k_latents = self.k_proj(latents).reshape(batch_size, self.num_latents, self.num_heads, self.head_dim)
        v_latents = self.v_proj(latents).reshape(batch_size, self.num_latents, self.num_heads, self.head_dim)

        # Transpose for attention computation
        q_x = q_x.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        k_x = k_x.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        v_x = v_x.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]

        q_latents = q_latents.transpose(1, 2)  # [batch_size, num_heads, num_latents, head_dim]
        k_latents = k_latents.transpose(1, 2)  # [batch_size, num_heads, num_latents, head_dim]
        v_latents = v_latents.transpose(1, 2)  # [batch_size, num_heads, num_latents, head_dim]

        # Scale factor for attention
        scale = 1.0 / math.sqrt(self.head_dim)

        # Compute latent-to-input attention
        attn_latent_to_input = torch.matmul(q_latents, k_x.transpose(2, 3)) * scale

        # Apply mask if provided
        if mask is not None:
            # Expand mask for the latent queries
            latent_mask = mask.unsqueeze(1).expand(-1, self.num_heads, -1, -1)
            attn_latent_to_input = attn_latent_to_input.masked_fill(latent_mask == 0, -1e9)

        # Softmax and dropout
        attn_latent_to_input = torch.softmax(attn_latent_to_input, dim=-1)
        attn_latent_to_input = self.dropout(attn_latent_to_input)

        # Apply attention weights to input values
        latent_output = torch.matmul(attn_latent_to_input, v_x)  # [batch_size, num_heads, num_latents, head_dim]

        # Compute input-to-latent attention
        attn_input_to_latent = torch.matmul(q_x, k_latents.transpose(2, 3)) * scale

        # Softmax and dropout
        attn_input_to_latent = torch.softmax(attn_input_to_latent, dim=-1)
        attn_input_to_latent = self.dropout(attn_input_to_latent)

        # Updated latent values are used as values for input-to-latent attention
        output = torch.matmul(attn_input_to_latent, latent_output)  # [batch_size, num_heads, seq_len, head_dim]

        # Reshape and apply output projection
        output = output.transpose(1, 2).reshape(batch_size, seq_len, self.num_heads * self.head_dim)
        output = self.o_proj(output)

        return output
```

## Flash Attention

**Flash Attention** (particularly the latest implementation [FlashAttention-3](https://pytorch.org/blog/flashattention-3/)) addresses the significant memory bottleneck inherent in standard self-attention mechanisms within Transformers, particularly for long sequences. The conventional approach computes the full attention score matrix $S = QK^T$, where $Q, K \in \mathbb{R}^{N \times d}$ are the query and key matrices for a sequence of length $N$. This requires storing the $N \times N$ matrix $S$, leading to $O(N^2)$ memory complexity with respect to sequence length. This becomes prohibitive for large $N$. Flash Attention overcomes this by avoiding the materialization and storage of the full $S$ matrix in the GPU's slow high bandwidth memory. Instead, it leverages tiling and recomputation techniques, processing the attention computation in smaller blocks that fit into the much faster on-chip SRAM.

The core mechanism involves breaking the Q, K, and V matrices into blocks. Flash Attention iteratively loads blocks of K and V into SRAM, and for each block of Q, it computes the attention scores against the current K block also residing in SRAM. Crucially, it employs an online softmax algorithm. Instead of computing the full softmax denominator across all keys at once, it maintains running statistics (the maximum score seen so far for numerical stability, and the cumulative sum of exponentiated scores for normalization) as it iterates through the K/V blocks. This allows it to compute the correctly scaled attention output block-by-block without ever needing the complete $N \times N$ matrix. By keeping intermediate results primarily within the fast SRAM and minimizing data transfer to and from high bandwidth memory, Flash Attention significantly reduces the memory footprint related to sequence length from $O(N^2)$ down to $O(N)$ (dominated by storing Q, K, V themselves) and achieves substantial speedups due to improved memory access patterns.

In practice the FlashAttention-3 implementation is a family of highly-optimized CUDA kernels that are designed to be efficient for different hardware configurations. But a minimal toy implementation in PyTorch is shown below:

``` python
class FlashAttention(nn.Module):
    def __init__(self, dim, num_heads, head_dim=64, dropout=0.0, block_size=1024):
        super().__init__()
        self.dim = dim
        self.num_heads = num_heads
        self.head_dim = head_dim
        self.block_size = block_size

        # Projections
        self.q_proj = nn.Linear(dim, num_heads * head_dim)
        self.k_proj = nn.Linear(dim, num_heads * head_dim)
        self.v_proj = nn.Linear(dim, num_heads * head_dim)
        self.o_proj = nn.Linear(num_heads * head_dim, dim)

        self.dropout = nn.Dropout(dropout)

    def _flash_attention_forward(self, q, k, v, mask=None):
        # This is a simplified approximation of Flash Attention
        # In practice, FlashAttention uses custom CUDA kernels for tiled attention

        batch_size, num_heads, seq_len, head_dim = q.shape
        scale = 1.0 / math.sqrt(head_dim)

        # Initialize output and attention statistics
        output = torch.zeros_like(q)
        normalizer = torch.zeros((batch_size, num_heads, seq_len, 1), device=q.device)

        # Process blocks of keys and values
        for block_start in range(0, seq_len, self.block_size):
            block_end = min(block_start + self.block_size, seq_len)

            # Extract key and value blocks
            k_block = k[:, :, block_start:block_end]
            v_block = v[:, :, block_start:block_end]

            # Compute attention scores for this block
            attn_scores = torch.matmul(q, k_block.transpose(2, 3)) * scale

            # Apply mask if provided
            if mask is not None:
                block_mask = mask[:, :, :, block_start:block_end]
                attn_scores = attn_scores.masked_fill(block_mask == 0, -1e9)

            # Apply softmax and dropout
            attn_probs = torch.softmax(attn_scores, dim=-1)
            attn_probs = self.dropout(attn_probs)

            # Update output with the attention results for this block
            output += torch.matmul(attn_probs, v_block)
            normalizer += attn_probs.sum(dim=-1, keepdim=True)

        # Normalize the output
        output = output / (normalizer + 1e-6)

        return output

    def forward(self, x, mask=None):
        batch_size, seq_len, _ = x.shape

        # Project queries, keys, values
        q = self.q_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        k = self.k_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        v = self.v_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)

        # Transpose for attention computation
        q = q.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        k = k.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        v = v.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]

        # Compute flash attention
        output = self._flash_attention_forward(q, k, v, mask)

        # Reshape and apply output projection
        output = output.transpose(1, 2).reshape(batch_size, seq_len, self.num_heads * self.head_dim)
        output = self.o_proj(output)

        return output
```

In reality the repository [Dao-AILab/flash-attention](https://github.com/Dao-AILab/flash-attention) has a number of different implementations for different hardware configurations. The `flash_attn_qkvpacked_func` function is a minimal example of how to use the FlashAttention-3 implementation. It takes a packed QKV tensor as input and returns the attention output.

``` python
import torch
from flash_attn import flash_attn_qkvpacked_func

# Minimal configuration
BATCH_SIZE, SEQ_LEN, NUM_HEADS, HEAD_DIM = 2, 64, 4, 32
CAUSAL = False
DTYPE = torch.float16
DEVICE = "cuda"

# Create dummy packed QKV tensor
# Shape: (batch_size, seq_len, 3, num_heads, head_dim)
qkv = torch.randn(
    BATCH_SIZE,
    SEQ_LEN,
    3,
    NUM_HEADS,
    HEAD_DIM,
    dtype=DTYPE,
    device=DEVICE,
)

print(f"Input qkv shape: {qkv.shape}")

# Call FlashAttention packed QKV function
output = flash_attn_qkvpacked_func(
    qkv,
    dropout_p=0.0, # Set dropout probability (0.0 for no dropout)
    causal=CAUSAL,
    softmax_scale=None # Use default scaling (1 / sqrt(head_dim))
)

# Output shape: (batch_size, seq_len, num_heads, head_dim)
print(f"Output shape: {output.shape}")
print("FlashAttention call successful.")
```

## Ring Attention

[**Ring Attention**](https://arxiv.org/abs/2310.01889) uses blockwise computation of self-attention on multiple GPUs and enables training and inference of sequences that would be too long for a single devices. It addresses the significant memory bottleneck inherent in standard self-attention mechanisms, particularly when processing very long sequences where the quadratic memory complexity $O(N^2)$ of the full attention score matrix becomes prohibitive.

The core idea is to distribute the computation across multiple processing units, like GPUs, arranged conceptually in a ring topology. This approach avoids the need for any single device to hold the entire `K` and `V` tensors. Instead, these tensors are sharded or chunked along the sequence length dimension, drastically reducing the peak memory requirement per device and enabling attention calculations over sequences that would otherwise exceed the memory capacity of individual accelerators.

In a practical distributed implementation, each device initially holds the `Q`, `K`, and `V` shards corresponding to its segment of the input sequence. The attention calculation unfolds in synchronized steps across this ring. During each step, a device calculates partial attention scores using its local `Q` shard and the `K` shard it currently possesses. The crucial element is the subsequent communication: the `K` and `V` shards are passed to the next device in the ring. This rotation repeats until every `Q` shard has interacted with every `K/V` shard. Throughout this process, each device accumulates partial outputs (weighted `V` vectors) and normalization factors (softmax denominators). Finalizing the attention output typically involves a collective operation across all devices to combine these partial results correctly for each segment of the sequence.

The Python example below offers a simulated Ring Attention logic on a single device, illustrating the underlying principles without necessitating actual multi-GPU hardware. The `_simulate_ring_attention` function mimics the distributed process by iterating through hypothetical shards. In each iteration, it selects slices of the `K` and `V` tensors (`k_shard`, `v_shard`) to represent the data one device would handle at a given step. It then computes attention scores between the full `Q` tensor (a simplification from a truly distributed setup) and the current `k_shard`. The simulation effectively captures the essence of the ring approach by accumulating the weighted values and the softmax normalizers across these iterations, mirroring how partial results would be combined in a distributed setting before a final normalization step yields the output. While demonstrating the computational flow, this simulation naturally doesn't provide the parallelism or memory savings of a true multi-device Ring Attention implementation.

``` python
class RingAttention(nn.Module):
    def __init__(self, dim, num_heads, head_dim=64, dropout=0.0, num_shards=4):
        super().__init__()
        self.dim = dim
        self.num_heads = num_heads
        self.head_dim = head_dim
        self.num_shards = num_shards

        # Projections
        self.q_proj = nn.Linear(dim, num_heads * head_dim)
        self.k_proj = nn.Linear(dim, num_heads * head_dim)
        self.v_proj = nn.Linear(dim, num_heads * head_dim)
        self.o_proj = nn.Linear(num_heads * head_dim, dim)

        self.dropout = nn.Dropout(dropout)

    def _simulate_ring_attention(self, q, k, v, mask=None):
        # This simulates ring attention without actual multi-GPU support
        batch_size, num_heads, seq_len, head_dim = q.shape
        scale = 1.0 / math.sqrt(head_dim)

        # Compute shard sizes
        shard_size = (seq_len + self.num_shards - 1) // self.num_shards

        # Initialize outputs
        output = torch.zeros_like(q)
        normalizer = torch.zeros((batch_size, num_heads, seq_len, 1), device=q.device)

        # Simulate sharded processing
        for shard_idx in range(self.num_shards):
            start_idx = shard_idx * shard_size
            end_idx = min(start_idx + shard_size, seq_len)

            # Process this shard's keys and values
            if start_idx < seq_len:
                k_shard = k[:, :, start_idx:end_idx]
                v_shard = v[:, :, start_idx:end_idx]

                # Compute attention scores
                attn_scores = torch.matmul(q, k_shard.transpose(2, 3)) * scale

                # Apply mask if provided
                if mask is not None:
                    shard_mask = mask[:, :, :, start_idx:end_idx]
                    attn_scores = attn_scores.masked_fill(shard_mask == 0, -1e9)

                # Apply softmax and dropout (accumulated over shards)
                attn_probs = torch.softmax(attn_scores, dim=-1)
                attn_probs = self.dropout(attn_probs)

                # Update output and normalizer
                output += torch.matmul(attn_probs, v_shard)
                normalizer += attn_probs.sum(dim=-1, keepdim=True)

        # Normalize the output
        output = output / (normalizer + 1e-6)

        return output

    def forward(self, x, mask=None):
        batch_size, seq_len, _ = x.shape

        # Project queries, keys, values
        q = self.q_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        k = self.k_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)
        v = self.v_proj(x).reshape(batch_size, seq_len, self.num_heads, self.head_dim)

        # Transpose for attention computation
        q = q.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        k = k.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]
        v = v.transpose(1, 2)  # [batch_size, num_heads, seq_len, head_dim]

        # Compute ring attention
        output = self._simulate_ring_attention(q, k, v, mask)

        # Reshape and apply output projection
        output = output.transpose(1, 2).reshape(batch_size, seq_len, self.num_heads * self.head_dim)
        output = self.o_proj(output)

        return output
```

## Pre-normalization

**Pre-normalization** (often referred to as Pre-LN) was a shift in the architectural design of residual blocks. Instead of applying the normalization layer *after* the main operation (like self-attention or a feed-forward network) as done in traditional post-normalization schemes, pre-normalization applies it *before*. This seemingly small change has significant implications for training dynamics. By normalizing the input *before* it enters the computationally intensive sub-layer, pre-normalization helps to stabilize the activations and gradients flowing through the network. This stabilization effect is particularly pronounced in very deep networks, mitigating issues like vanishing or exploding gradients and often allowing for higher learning rates and faster, more reliable convergence.

The typical implementation within a residual block follows the structure $x + f(\text{norm}(x))$, as demonstrated in the `PreNorm` class. Here, $x$ is the input to the block, $\text{norm}(\cdot)$ represents a normalization function like Layer Normalization (LN) or Root Mean Square Normalization (RMSNorm), and $f(\cdot)$ denotes the main transformation function (e.g., multi-head attention or a position-wise feed-forward network). The input $x$ first passes through the normalization layer (e.g., `self.norm(x)`). The normalized output is then processed by the function `fn`. Crucially, the output of this function is then added back to the *original*, unnormalized input $x$ via the residual connection. This structure ensures a clean gradient path through the identity connection (`+ x`), further enhancing training stability compared to post-normalization where the normalization layer resides on the residual path itself.

``` python
class PreNorm(nn.Module):
    def __init__(self, dim, fn, norm_type='layer'):
        super().__init__()
        self.fn = fn

        if norm_type == 'layer':
            self.norm = nn.LayerNorm(dim)
        elif norm_type == 'rms':
            self.norm = RMSNorm(dim)
        else:
            raise ValueError(f"Unknown normalization type: {norm_type}")

    def forward(self, x, *args, **kwargs):
        # Apply normalization first, then the function
        return self.fn(self.norm(x), *args, **kwargs) + x
```

## RMSNorm

**RMSNorm** (or Root Mean Square Normalization) is a simplification of the widely used LayerNorm, designed to reduce computational overhead while retaining comparable performance and often improving training stability. Unlike LayerNorm, which centers the activations by subtracting the mean and then scales by the standard deviation, RMSNorm omits the mean centering step entirely. The motivation behind this simplification stems from the empirical observation that the re-centering operation in LayerNorm accounts for a noticeable portion of its computational cost and that removing it often does not significantly harm, and can sometimes even benefit, model performance. It operates solely on the basis of re-scaling the inputs according to their root mean square magnitude.

The core mechanism of RMSNorm involves normalizing the activations within a layer by dividing them by their root mean square value, computed across the features (or a specified dimension). For an input vector $x = (x_1, \dots, x_n)$, the RMS value is calculated as $\text{RMS}(x) = \sqrt{\frac{1}{n} \sum_{i=1}^n x_i^2}$. The normalized output $\bar{x}_i$ is then $\bar{x}_i = \frac{x_i}{\text{RMS}(x) + \epsilon}$, where $\epsilon$ is a small constant for numerical stability. Similar to LayerNorm, RMSNorm typically includes a learnable scaling parameter $g$ (and sometimes a bias $b$, although the original formulation often omits it to stick closer to the simplification principle), resulting in the final output $y_i = g_i \bar{x}_i$. By foregoing the mean calculation, RMSNorm offers a reduction in computation (estimated to be 7-64% faster than LayerNorm on GPUs depending on the setup) and memory usage, making it an attractive alternative, especially for large models where efficiency is paramount.

``` python
class RMSNorm(nn.Module):
    def __init__(self, dim, eps=1e-8, elementwise_affine=True):
        super().__init__()
        self.eps = eps
        self.elementwise_affine = elementwise_affine

        if elementwise_affine:
            self.weight = nn.Parameter(torch.ones(dim))
        else:
            self.register_parameter('weight', None)

    def forward(self, x):
        # Calculate root mean square along the last dimension
        rms = torch.sqrt(torch.mean(x ** 2, dim=-1, keepdim=True) + self.eps)

        # Normalize by RMS
        x_normalized = x / rms

        # Apply scaling if using learnable parameters
        if self.elementwise_affine:
            x_normalized = x_normalized * self.weight

        return x_normalized
```

## SwiGLU

SwiGLU is an activation function derived from the Gated Linear Unit (GLU) family, specifically tailored for enhancing the performance of neural networks. The core concept behind GLU variants is to introduce a gating mechanism that adaptively controls the flow of information through the network. Standard feed-forward layers typically apply a single non-linearity to a linear transformation of the input. In contrast, GLU-based activations split the output of a linear layer into two parts; one part acts as a "gate" after passing through a non-linearity, modulating the other part via element-wise multiplication. SwiGLU distinguishes itself by employing the Sigmoid-weighted Linear Unit (SiLU), also known as Swish ($\text{SiLU}(x) = x \cdot \sigma(x)$, where $\sigma$ is the sigmoid function), as the specific non-linearity applied to the gating part. This choice has been empirically shown to yield significant performance improvements in various Transformer-based models compared to other activations like ReLU or standard GLU variants using sigmoid or other functions for the gate.

The operational mechanism of SwiGLU within a feed-forward block typically involves projecting the input $x$ using two separate linear transformations, yielding $Wx + b$ and $Vx + c$. The SwiGLU activation is then computed as $\text{SwiGLU}(x, W, V, b, c) = \text{SiLU}(Wx + b) \odot (Vx + c)$, where $\odot$ denotes element-wise multiplication. Effectively, the input is processed through two parallel linear paths. One path undergoes the SiLU activation to form the gate values, which then scale the output of the second linear path. This gating allows the network to dynamically control which features are passed forward based on the input context, leading to increased expressive power and better gradient flow compared to simpler activation functions. Its success in models like PaLM and LLaMA highlights its effectiveness in capturing complex patterns within language data.

``` python
class SwiGLU(nn.Module):
    def __init__(self, dim_in, dim_hidden=None, dim_out=None, bias=True):
        super().__init__()
        dim_hidden = dim_hidden or 4 * dim_in
        dim_out = dim_out or dim_in

        # Linear transformations for gating
        self.w1 = nn.Linear(dim_in, dim_hidden, bias=bias)
        self.w2 = nn.Linear(dim_in, dim_hidden, bias=bias)

        # Output projection
        self.w3 = nn.Linear(dim_hidden, dim_out, bias=bias)

    def forward(self, x):
        # SwiGLU applies SiLU activation to one branch and gates it with the other
        hidden1 = self.w1(x)
        hidden2 = self.w2(x)

        # SiLU (Swish) activation: x * sigmoid(x)
        hidden1_act = hidden1 * torch.sigmoid(hidden1)

        # Element-wise product for gating
        hidden = hidden1_act * hidden2

        # Output projection
        return self.w3(hidden)
```

## Rotary Positional Embedding

[**Rotary Positional Embedding**](https://arxiv.org/abs/2104.09864) (RoPE) introduces an elegant method for incorporating positional information directly into the self-attention mechanism of Transformer models, specifically designed to capture relative positional dependencies effectively. Traditional approaches often rely on adding absolute positional encodings to the input embeddings or using complex relative positional bias terms within the attention score calculation. RoPE takes a different route by viewing positional encoding as a rotation operation applied to the query and key vectors *before* their dot product is computed. The key insight is that by rotating the Q vector corresponding to position $m$ and the K vector corresponding to position $n$ by angles proportional to $m$ and $n$ respectively, the resulting dot product inherently depends only on the *relative* position $m - n$ and the content of the vectors themselves, gracefully encoding relative distance without altering the vectors' magnitudes.

The core mathematical idea leverages the properties of complex number multiplication or, equivalently, 2D rotation matrices. Imagine representing pairs of embedding dimensions as complex numbers. RoPE applies a rotation to the query vector $q_m$ at position $m$ and the key vector $k_n$ at position $n$ using position-dependent rotation matrices $R_m$ and $R_n$, respectively. These matrices effectively rotate the vectors in 2D subspaces spanned by pairs of dimensions. The angle of rotation for each 2D subspace is determined by the absolute position ($m$ or $n$) multiplied by a frequency term $\theta_i$, which decreases for higher dimensions, analogous to sinusoidal embeddings. The crucial property is that the dot product between the rotated vectors, $(R_m q_m)^T (R_n k_n)$, simplifies such that it only depends on the original vectors $q_m, k_n$ and a rotation matrix $R_{m-n}$ corresponding to the relative distance, effectively embedding relative positional information directly into the attention score calculation.

In practice, this rotation is implemented efficiently without explicit matrix multiplication. As shown in the `apply_rotary_pos_emb` function, the embedding dimensions are typically split into pairs. For each pair $(x_i, x_{i+1})$, the rotation corresponding to position $m$ and frequency $\theta_j$ (derived from `inv_freq` in the `RotaryEmbedding` class) is applied using trigonometric functions: the new pair $(x'_i, x'_{i+1})$ becomes $(x_i \cos(m\theta_j) - x_{i+1} \sin(m\theta_j), x_{i+1} \cos(m\theta_j) + x_i \sin(m\theta_j))$. The `RotaryEmbedding` class pre-computes the necessary cosine and sine values (`cos`, `sin`) based on the sequence length and the inverse frequency bands (`inv_freq`), which are derived from a base value (`base`) and the embedding dimension (`dim`). These pre-computed values represent $\cos(m\theta_j)$ and $\sin(m\theta_j)$ for all positions $m$ and relevant frequencies $\theta_j$.

Applying RoPE involves generating these `cos` and `sin` embeddings for the given sequence length and then using them to transform the Q and K vectors pair-wise across their head dimension *after* the initial linear projections but *before* the attention score calculation. This method offers several advantages: it naturally encodes relative positions, avoids adding positional information directly to word embeddings (potentially preserving more semantic information), and has shown strong empirical performance, including good generalization to sequence lengths longer than those seen during training. By integrating position information via rotation, RoPE provides a computationally efficient and effective mechanism for context-aware sequence modeling.

``` python
class RotaryEmbedding(nn.Module):
    def __init__(self, dim, base=10000, interleaved=False):
        super().__init__()
        self.dim = dim
        self.base = base
        self.interleaved = interleaved

        # Generate inverse frequency bands
        inv_freq = 1.0 / (base ** (torch.arange(0, dim, 2).float() / dim))
        self.register_buffer('inv_freq', inv_freq)

    def forward(self, seq_len, device=None):
        # Get device from buffer if not specified
        if device is None:
            device = self.inv_freq.device

        # Generate position indices
        positions = torch.arange(seq_len, device=device).float()

        # Compute sinusoidal patterns
        freqs = torch.outer(positions, self.inv_freq)

        # Get sine and cosine embeddings
        emb = torch.cat((freqs, freqs), dim=-1)
        cos = torch.cos(emb)[:, :self.dim]
        sin = torch.sin(emb)[:, :self.dim]

        return cos, sin

def apply_rotary_pos_emb(q, k, cos, sin, interleaved=False):
    # Apply rotary embeddings to queries and keys
    batch_size, num_heads, seq_len, head_dim = q.shape
    cos = cos.reshape(1, 1, seq_len, cos.shape[-1])  # [1, 1, seq_len, dim/2]
    sin = sin.reshape(1, 1, seq_len, sin.shape[-1])  # [1, 1, seq_len, dim/2]

    # Split queries and keys for rotation
    half_dim = head_dim // 2
    q1, q2 = q[..., :half_dim], q[..., half_dim:]
    k1, k2 = k[..., :half_dim], k[..., half_dim:]

    # Apply rotation using half-dim rotary embeddings
    q_rotated = torch.cat([
        q1 * cos - q2 * sin,
        q2 * cos + q1 * sin
    ], dim=-1)

    k_rotated = torch.cat([
        k1 * cos - k2 * sin,
        k2 * cos + k1 * sin
    ], dim=-1)

    return q_rotated, k_rotated
```

## Mixture of Experts

Mixture of Experts (shortened as MoE) is a model architecture designed to significantly increase the parameter count, and thus the potential capacity, of a neural network without incurring a proportionally massive increase in computational cost during inference or training. The core idea is to replace computationally intensive components, like the feed-forward network block in a Transformer, with multiple, smaller "expert" networks. Crucially, not all experts process every input token. Instead, a lightweight "router" or "gating" network dynamically selects a small subset of experts (typically just one or two, known as top-k routing) deemed most suitable for processing each specific input token based on its features. This conditional computation allows MoE models to possess potentially trillions of parameters while only activating a small fraction of them for any given input, maintaining manageable FLOPs compared to a similarly sized dense model.

The router network is the core idea, it acts as a learned decision-maker. It takes the representation of an input token and produces scores or logits indicating the suitability of each available expert for that token. In the example code's `_compute_routing_weights` function, these logits are often processed using a top-k function to identify the `k` experts with the highest scores. The scores for these selected experts are then typically normalized, often using a softmax function, to produce routing weights. These weights determine the contribution of each selected expert to the final output for that token. During training, noise can be added to the router logits (as seen with `noise_std`) to encourage exploration and prevent the router from collapsing to always favor only a few experts early on.

Once the router selects the top-k experts and calculates their respective weights for a given input token, that token is dispatched only to those chosen experts. Each selected expert network (often a standard FFN, as shown in the `experts` ModuleList) processes the token independently. The outputs produced by these active experts are then combined to form the final output for that token. This combination is typically a weighted sum, where the output of each selected expert is multiplied by its corresponding routing weight calculated by the router. For instance, if experts `i` and `j` are selected with weights `w_i` and `w_j`, the final output for token `x` would be `w_i * expert_i(x) + w_j * expert_j(x)`. This ensures that the final representation incorporates specialized knowledge from the most relevant experts.

A challenge in training MoE models is ensuring that all experts are utilized effectively; otherwise, the router might learn to consistently overload a few "popular" experts while others remain underdeveloped. To counteract this, an auxiliary load balancing loss is typically incorporated into the training objective, as demonstrated by the `_compute_balance_loss` method. This loss encourages the router to distribute the computational load (i.e., the input tokens) more evenly across all available experts, often by penalizing imbalances in either the number of tokens assigned to each expert or the sum of routing weights directed towards each expert. By successfully implementing sparse activation via routing and incorporating load balancing, MoE enables the construction of extremely large yet computationally efficient models.

``` python
class MixtureOfExperts(nn.Module):
    def __init__(self, input_dim, hidden_dim, output_dim, num_experts=4, top_k=2, noise_std=1.0):
        super().__init__()
        self.input_dim = input_dim
        self.hidden_dim = hidden_dim
        self.output_dim = output_dim
        self.num_experts = num_experts
        self.top_k = min(top_k, num_experts)
        self.noise_std = noise_std

        # Create experts
        self.experts = nn.ModuleList([
            nn.Sequential(
                nn.Linear(input_dim, hidden_dim),
                nn.ReLU(),
                nn.Linear(hidden_dim, output_dim)
            ) for _ in range(num_experts)
        ])

        # Router network
        self.router = nn.Linear(input_dim, num_experts)

        # Save expert counts and loads for balancing loss
        self.register_buffer('expert_counts', torch.zeros(num_experts))

    def _compute_routing_weights(self, x):
        # Calculate routing weights
        routing_logits = self.router(x)  # [batch_size, seq_len, num_experts]

        # Add noise during training to encourage exploration
        if self.training and self.noise_std > 0:
            noise = torch.randn_like(routing_logits) * self.noise_std
            routing_logits = routing_logits + noise

        # Get top-k experts for each token
        routing_weights, selected_experts = torch.topk(routing_logits, self.top_k, dim=-1)

        # Normalize the routing weights with softmax
        routing_weights = F.softmax(routing_weights, dim=-1)

        return routing_weights, selected_experts

    def _compute_balance_loss(self, selected_experts, routing_weights):
        # Compute auxiliary load balancing loss
        batch_size, seq_len, _ = selected_experts.shape

        # Compute probability of each expert being selected across batch
        expert_mask = torch.zeros(batch_size, seq_len, self.num_experts, device=selected_experts.device)

        # For each position in selected_experts, increment the corresponding expert index
        for k in range(self.top_k):
            expert_mask.scatter_(-1, selected_experts[..., k:k+1], routing_weights[..., k:k+1])

        # Compute mean routing probability per expert
        expert_routing_probs = expert_mask.mean(dim=[0, 1])

        # Compute load balancing loss (all experts should receive equal probability)
        target_probs = torch.ones_like(expert_routing_probs) / self.num_experts
        balance_loss = F.mse_loss(expert_routing_probs, target_probs) * self.num_experts

        return balance_loss

    def forward(self, x):
        batch_size, seq_len, _ = x.shape

        # Compute routing weights and selected experts
        routing_weights, selected_experts = self._compute_routing_weights(x)

        # Prepare output
        output = torch.zeros(batch_size, seq_len, self.output_dim, device=x.device)

        # Dispatch to selected experts
        for k in range(self.top_k):
            # Extract the current expert indices and weights
            expert_indices = selected_experts[..., k]  # [batch_size, seq_len]
            expert_weights = routing_weights[..., k].unsqueeze(-1)  # [batch_size, seq_len, 1]

            # Dispatch to each expert
            for expert_idx in range(self.num_experts):
                # Find tokens assigned to this expert
                mask = (expert_indices == expert_idx)

                if mask.any():
                    # Gather input tokens assigned to this expert
                    expert_inputs = x[mask]

                    # Process with the expert
                    expert_outputs = self.experts[expert_idx](expert_inputs)

                    # Scatter outputs back with appropriate weights
                    output[mask] += expert_outputs * expert_weights[mask]

                    # Update expert counts (for monitoring)
                    self.expert_counts[expert_idx] += mask.sum().item()

        # Compute auxiliary load balancing loss
        balance_loss = self._compute_balance_loss(selected_experts, routing_weights)

        # Return output and auxiliary loss
        return output, balance_loss
```

## Learning Rate Warmup

Learning rate warmup is a widely adopted heuristic employed during the initial phase of training neural networks to enhance stability and prevent divergence. At the beginning of training, model parameters are typically initialized randomly, often far from an optimal configuration. If a relatively large learning rate is used immediately, the initial gradients, which can also be large and erratic, may cause drastic parameter updates, potentially pushing the model into a poor region of the loss landscape or even causing numerical instability (e.g., loss explosion). Learning rate warmup mitigates this risk by starting the training process with a very small learning rate, which is then gradually increased over a predefined number of initial training steps (the "warmup steps") until it reaches its target base value, from which point a standard learning rate schedule (like decay) might commence.

The mechanism involves progressively scaling the base learning rate during the warmup phase. A common strategy, illustrated by the `LinearWarmupScheduler` class, is linear warmup. In this approach, the learning rate $\eta_t$ at step $t$ is calculated as $\eta_t = \eta_{\text{base}} \times \frac{t}{T_{\text{warmup}}}$ for $t < T_{\text{warmup}}$, where $\eta_{\text{base}}$ is the target base learning rate and $T_{\text{warmup}}$ is the total number of warmup steps. As seen in the `get_lr` method, the scaling factor `scale` increases linearly from near zero to 1 over the `warmup_steps`. Once the step count `last_epoch` reaches `warmup_steps`, the learning rate becomes equal to the `base_lrs`, and the warmup phase concludes. This gentle ramp-up allows the model to adapt gradually during the critical early stages when activations and gradients might otherwise be volatile, leading to smoother convergence and often enabling the use of higher base learning rates later in training.

``` python
class LinearWarmupScheduler(_LRScheduler):
    def __init__(self, optimizer, warmup_steps, last_epoch=-1):
        self.warmup_steps = warmup_steps
        super().__init__(optimizer, last_epoch)

    def get_lr(self):
        if self.last_epoch < self.warmup_steps:
            # During warmup: linearly increase from 0 to base LR
            scale = float(self.last_epoch + 1) / float(max(1, self.warmup_steps))
            return [base_lr * scale for base_lr in self.base_lrs]
        else:
            # After warmup: use base learning rate
            return self.base_lrs
```

## Cosine Schedule

**Cosine scheduling** (sometimes called **cosine annealing**) is a learning rate schedule technique. Its core principle is to gradually decrease the learning rate over the course of training, following the shape of a cosine curve. Unlike step decay schedules, which reduce the learning rate abruptly at specific epochs, cosine annealing provides a smooth, continuous reduction. Typically, the learning rate starts at an initial high value and decreases following the first half-cycle of a cosine function, reaching a predefined minimum value (often close to zero) by the final training step. This smooth decay has been shown empirically to help the optimization process by allowing larger steps early in training for broad exploration of the loss landscape, and progressively smaller steps later on for fine-tuning and convergence towards a good minimum.

Early in training, a higher learning rate encourages faster exploration and helps escape poor local minima. As training progresses and the model parameters approach a more optimal region, reducing the learning rate becomes crucial to avoid overshooting the minimum and to allow for more precise convergence. The cosine function provides a schedule that starts with a relatively slow decay rate, allowing the optimizer to maintain momentum initially. The decay rate then accelerates towards the middle of the schedule before slowing down again as it approaches the minimum learning rate. This final slow-down phase near the end of training is particularly important, as it allows the optimizer to carefully settle into a potentially flat minimum, which empirical evidence suggests often correlates with better generalization performance compared to sharper minima.

Furthermore, cosine scheduling is often combined with a "warmup" phase, as seen in the code example. During this initial phase (e.g., `warmup_steps`), the learning rate is typically increased linearly from a very small value (or zero) up to the main initial learning rate. This warmup period helps stabilize training in the very beginning, especially for large models or datasets where large initial learning rates applied to randomly initialized weights could lead to instability or divergence. After the warmup, the cosine decay phase begins, smoothly decreasing the learning rate from its peak value down to the target minimum (`base_lr * min_lr_ratio`) over the remaining `total_steps - warmup_steps`. This combination of a gentle start (warmup) followed by a smooth, theoretically motivated decay (cosine annealing) provides a robust and effective learning rate strategy that often requires less hyperparameter tuning than step-based schedules and frequently leads to improved model accuracy.

There is a [CosineScheduler](https://pytorch.org/docs/stable/generated/torch.optim.lr_scheduler.CosineAnnealingWarmRestarts.html) in PyTorch that implements this technique. Let's look at a simplified version of it:

``` python
class CosineAnnealingWarmupScheduler(_LRScheduler):
    def __init__(self, optimizer, warmup_steps, total_steps, min_lr_ratio=1e-4, last_epoch=-1):
        self.warmup_steps = warmup_steps
        self.total_steps = total_steps
        self.min_lr_ratio = min_lr_ratio
        super().__init__(optimizer, last_epoch)

    def get_lr(self):
        if self.last_epoch < self.warmup_steps:
            # During warmup: linearly increase from 0 to base LR
            scale = float(self.last_epoch + 1) / float(max(1, self.warmup_steps))
            return [base_lr * scale for base_lr in self.base_lrs]
        else:
            # After warmup: cosine decay from base LR to min_lr
            progress = float(self.last_epoch - self.warmup_steps) / float(
                max(1, self.total_steps - self.warmup_steps)
            )
            # Cosine decay formula: min_lr + 0.5 * (base_lr - min_lr) * (1 + cos(pi * progress))
            scale = self.min_lr_ratio + 0.5 * (1.0 - self.min_lr_ratio) * (
                1.0 + math.cos(math.pi * progress)
            )
            return [base_lr * scale for base_lr in self.base_lrs]
```

## AdamW Optimizer

**AdamW** (Adam with Decoupled Weight Decay) addresses a subtle issue in the standard implementation of weight decay (L2 regularization) within adaptive optimizers like Adam. This improved optimizer gives us a method where weight decay does not accumulate in the momentum nor variance. In traditional Adam, L2 regularization is often implemented by adding the decay term ($\lambda \cdot \text{weight}$) directly to the gradient before computing the moving averages ($m_t$ and $v_t$). However, this couples the weight decay effect with the adaptive learning rate derived from the gradient moments. Consequently, parameters with historically large gradients (and thus larger $v_t$ values) experience smaller effective weight decay, contrary to the goal of applying uniform regularization pressure. AdamW decouples these processes: it performs the standard Adam updates based purely on the gradients and separately applies the weight decay step directly to the weights, effectively restoring the original behavior of L2 regularization where weights decay proportionally to their magnitude, independent of their gradient history.

The core update mechanism of AdamW largely follows the standard Adam procedure but modifies how weight decay is applied. At each step $t$ for a parameter $\theta$, it first calculates the biased first moment estimate $m_t = \beta_1 \cdot m_{t-1} + (1 - \beta_1) \cdot g_t$ and the biased second raw moment estimate $v_t = \beta_2 \cdot v_{t-1} + (1 - \beta_2) \cdot g_t^2$, where $g_t$ is the gradient at step $t$, and $\beta_1$, $\beta_2$ are exponential decay rates. These are then bias-corrected: $\hat{m}_t = m_t / (1 - \beta_1^t)$ and $\hat{v}_t = v_t / (1 - \beta_2^t)$. The crucial difference lies in the update rule. Instead of incorporating weight decay into $g_t$, AdamW first applies weight decay directly to the parameters: $\theta_{t-1}' = \theta_{t-1} \cdot (1 - \text{lr} \cdot \lambda)$, where $\text{lr}$ is the learning rate and $\lambda$ is the weight_decay factor (as seen in the line `p.data.mul_(1 - group['lr'] * group['weight_decay'])`). Then, the standard Adam update using the bias-corrected moments is applied to these decayed weights: $\theta_t = \theta_{t-1}' - \text{lr} \cdot \hat{m}_t / (\sqrt{\hat{v}_t} + \epsilon)$. This corresponds to the code line `p.data.addcdiv_(exp_avg, denom, value=-step_size)`, operating on the already decayed `p.data`.

In deep learning practice, AdamW is employed similarly to Adam. It is instantiated by providing the model's parameters and key hyperparameters like the learning rate ($\text{lr}$), beta values ($\beta_1$, $\beta_2$), epsilon ($\epsilon$), and the weight decay factor ($\lambda$). The `step()` method, typically called within the training loop after computing gradients (`loss.backward()`), executes the update logic described above for each parameter. Its primary advantage is improved generalization, particularly observed in training large models like Transformers where regularization is critical. By ensuring that the weight decay strength is independent of the adaptive learning rate scaling, AdamW often allows for better hyperparameter tuning (especially $\text{lr}$ and $\lambda$) and can lead to models that perform better on unseen data compared to standard Adam with L2 regularization.

There is a highly optimized implementation of [AdamW](https://pytorch.org/docs/stable/generated/torch.optim.AdamW.html) in PyTorch. Let's look at a simplified version of it, the example below demonstrates a minimal PyTorch implementation, initializing state variables (like `exp_avg`, `exp_avg_sq`) and performing the decoupled weight decay and moment-based updates within the parameter loop.

``` python
class AdamW(Optimizer):
    def __init__(self, params, lr=1e-3, betas=(0.9, 0.999), eps=1e-8,
                 weight_decay=1e-2, amsgrad=False):
        defaults = dict(lr=lr, betas=betas, eps=eps,
                        weight_decay=weight_decay, amsgrad=amsgrad)
        super(AdamW, self).__init__(params, defaults)

    def step(self, closure=None):
        loss = None
        if closure is not None:
            loss = closure()

        for group in self.param_groups:
            for p in group['params']:
                if p.grad is None:
                    continue

                # Get gradient
                grad = p.grad.data

                if grad.is_sparse:
                    raise RuntimeError('AdamW does not support sparse gradients')

                amsgrad = group['amsgrad']
                state = self.state[p]

                # State initialization
                if len(state) == 0:
                    state['step'] = 0
                    # Exponential moving average of gradient values
                    state['exp_avg'] = torch.zeros_like(p.data)
                    # Exponential moving average of squared gradient values
                    state['exp_avg_sq'] = torch.zeros_like(p.data)
                    if amsgrad:
                        # Maintains max of all exp. moving avg. of sq. grad. values
                        state['max_exp_avg_sq'] = torch.zeros_like(p.data)

                exp_avg, exp_avg_sq = state['exp_avg'], state['exp_avg_sq']
                if amsgrad:
                    max_exp_avg_sq = state['max_exp_avg_sq']

                beta1, beta2 = group['betas']

                state['step'] += 1

                # Decay the first and second moment running average coefficient
                exp_avg.mul_(beta1).add_(grad, alpha=1 - beta1)
                exp_avg_sq.mul_(beta2).addcmul_(grad, grad, value=1 - beta2)

                if amsgrad:
                    # Maintains the maximum of all 2nd moment running avg. till now
                    torch.maximum(max_exp_avg_sq, exp_avg_sq, out=max_exp_avg_sq)
                    # Use the max. for normalizing running avg. of gradient
                    denom = max_exp_avg_sq.sqrt().add_(group['eps'])
                else:
                    denom = exp_avg_sq.sqrt().add_(group['eps'])

                bias_correction1 = 1 - beta1 ** state['step']
                bias_correction2 = 1 - beta2 ** state['step']
                step_size = group['lr'] * math.sqrt(bias_correction2) / bias_correction1

                # Apply weight decay BEFORE the optimization step
                if group['weight_decay'] != 0:
                    p.data.mul_(1 - group['lr'] * group['weight_decay'])

                # Update parameters
                p.data.addcdiv_(exp_avg, denom, value=-step_size)

        return loss
```

## Multi-token Prediction

Multi-token prediction is a technique developed to accelerate the inference speed of autoregressive language models. Normally, autoregressive generation predicts tokens one by one: the model takes a sequence, predicts the single most likely next token, appends it to the sequence, and repeats the process. This sequential nature, requiring one full forward pass of the model for each generated token, becomes a significant bottleneck for latency-sensitive applications. Multi-token prediction attempts to overcome this by modifying the model's prediction head to output probabilities for multiple future tokens simultaneously based on the current hidden state, thereby reducing the number of forward passes needed to generate a sequence of a given length.

The implementation typically involves adapting the final layer(s) of the language model. Instead of a single output layer (or "language model head") mapping the final hidden state to logits over the vocabulary for the next token, a multi-token predictor might employ several strategies. One approach, as shown in the example class using separate heads, is to have multiple distinct prediction heads, each trained to predict a token at a different future offset (e.g., one head for token $t+1$, another for $t+2$, up to $t+N$). These heads usually take the same final hidden state (e.g., corresponding to token $t$) as input but learn different projections specialized for their respective time steps. Another approach involves using a single shared prediction head, which might require more complex mechanisms, potentially involving learned transformations of the hidden state or incorporating positional information, to generate distinct probability distributions for each of the $N$ future tokens from essentially the same starting representation.

Training a multi-token predictor involves teaching the model to correctly anticipate the sequence of $N$ subsequent tokens given the preceding context. During the training phase, as illustrated in the `compute_loss` method, the model receives input sequences and its predictions for the next $N$ tokens are compared against the actual $N$ target tokens in the training data. A loss function, usually cross-entropy, is calculated for each predicted position ($t+1$ to $t+N$) and then aggregated (e.g., averaged) to form the final loss signal used for backpropagation.

While this can show speed improvements, multi-token prediction has several drawbacks: the accuracy of predicting tokens further into the future tends to decrease, and the chosen sequence of $N$ tokens might diverge from the optimal path that would have been taken by single-token generation. Therefore, it often represents a trade-off between generation speed and potential quality degradation that is very use-case dependent.

``` python
class MultiTokenPredictor(nn.Module):
    def __init__(self, hidden_dim, vocab_size, num_predicted_tokens=2, shared_prediction_head=False):
        super().__init__()
        self.hidden_dim = hidden_dim
        self.vocab_size = vocab_size
        self.num_predicted_tokens = num_predicted_tokens
        self.shared_prediction_head = shared_prediction_head

        if shared_prediction_head:
            # Share the same prediction head for all positions
            self.lm_head = nn.Linear(hidden_dim, vocab_size)
        else:
            # Use separate prediction heads for each position
            self.lm_heads = nn.ModuleList([
                nn.Linear(hidden_dim, vocab_size)
                for _ in range(num_predicted_tokens)
            ])

    def forward(self, hidden_states):
        batch_size, seq_len, _ = hidden_states.shape

        # Get the hidden states for the last token
        last_hidden = hidden_states[:, -1]

        if self.shared_prediction_head:
            # Use the shared head for all predictions
            logits = self.lm_head(last_hidden)
            next_token_logits = logits.unsqueeze(1)

            # For positions beyond the first, we need to make a projection
            multi_token_logits = []
            for i in range(self.num_predicted_tokens):
                if i == 0:
                    multi_token_logits.append(logits)
                else:
                    # A simple projection for demonstration; in practice, this would be more complex
                    projected_hidden = last_hidden + i * 0.1  # Just a dummy transformation
                    multi_token_logits.append(self.lm_head(projected_hidden))

            multi_token_logits = torch.stack(multi_token_logits, dim=1)
        else:
            # Use separate heads for each position
            multi_token_logits = torch.stack([
                head(last_hidden) for head in self.lm_heads
            ], dim=1)

            # The first position is used for next-token prediction
            next_token_logits = multi_token_logits[:, 0:1]

        return next_token_logits, multi_token_logits

    def compute_loss(self, hidden_states, labels, ignore_index=-100):
        # Get predictions
        _, multi_token_logits = self.forward(hidden_states)

        # Prepare targets: shift labels to align with predictions
        # We need the next num_predicted_tokens tokens after the input
        shifted_labels = labels[:, :-self.num_predicted_tokens]
        targets = []

        for i in range(self.num_predicted_tokens):
            targets.append(labels[:, 1+i:1+i-self.num_predicted_tokens+1])

        # Stack targets
        stacked_targets = torch.stack(targets, dim=1)

        # Compute loss across all predicted positions
        loss = 0
        for i in range(self.num_predicted_tokens):
            loss += F.cross_entropy(
                multi_token_logits[:, i].view(-1, self.vocab_size),
                stacked_targets[:, i].reshape(-1),
                ignore_index=ignore_index
            )

        return loss / self.num_predicted_tokens
```

## Speculative Decoding

Speculative decoding is an clever technique designed to accelerate the inference process of large autoregressive language models, significantly reducing the wall-clock time required to generate text. Standard generation is bottlenecked by its sequential nature, where the computationally expensive large model (the "target" model) must perform a full forward pass to predict just one token at a time. Speculative decoding introduces a secondary, much smaller and faster "draft" model. This draft model rapidly generates a short sequence of candidate future tokens (a "draft"). The core idea is then to use the large target model to verify this entire draft sequence in a single, parallel forward pass, potentially accepting multiple tokens at once and thus amortizing the cost of the target model's computation over several generated tokens.

The mechanism hinges on comparing the predictions of the draft model with those of the target model. After the draft model proposes a sequence of $k$ tokens, $d_1, d_2, \dots, d_k$, the target model is run once on the original input sequence concatenated with the draft. This single pass yields the target model's probability distributions for the *next* token at *each* position within the draft sequence. For each draft token $d_i$, its validity is checked against the target model's prediction at that same position. If the target model strongly agrees with the draft token (e.g., it would have also predicted $d_i$ with high probability, or based on a specific acceptance rule comparing $P_{\text{target}}(d_i)$ and $P_{\text{draft}}(d_i)$), the token is accepted. This verification proceeds sequentially through the draft.

The process continues until either all $k$ draft tokens are accepted, or a draft token $d_j$ is rejected. If rejection occurs at position $j$, all preceding accepted draft tokens ($d_1, \dots, d_{j-1}$) are kept. The remaining part of the draft ($d_j, \dots, d_k$) is discarded. Crucially, the target model's probability distribution calculated at position $j$ can then be used to sample a *corrected* token to append after the accepted sequence, ensuring the overall generation statistically follows the distribution of the more powerful target model. This corrected token, along with the previously accepted ones, forms the input for the next round of draft generation. By accepting multiple tokens per target model inference step on average, speculative decoding can achieve substantial speedups (e.g., 2-3x) with minimal impact on the quality of the generated text.

``` python
class SimpleSpeculativeDecoding:
    def __init__(self, target_model, draft_model, tokenizer, max_draft_tokens=5):
        self.target_model = target_model
        self.draft_model = draft_model
        self.tokenizer = tokenizer
        self.max_draft_tokens = max_draft_tokens

    def generate(self, prompt, max_length=100):
        # Start with the prompt's token IDs
        input_ids = self.tokenizer.encode(prompt, return_tensors="pt")

        # Keep generating until we reach max_length or an end token
        while input_ids.shape[1] < max_length:
            # Step 1: Generate multiple draft tokens
            draft_input_ids = input_ids.clone()
            draft_tokens = []

            with torch.no_grad():
                for _ in range(self.max_draft_tokens):
                    # Generate next token with draft model
                    outputs = self.draft_model(draft_input_ids)
                    next_token_logits = outputs.logits[:, -1, :]
                    next_token = torch.argmax(next_token_logits, dim=-1)

                    # Save the draft token
                    draft_tokens.append(next_token.item())

                    # Add to draft input
                    draft_input_ids = torch.cat([draft_input_ids, next_token.unsqueeze(0)], dim=1)

                    # Check for end token
                    if next_token.item() == self.tokenizer.eos_token_id:
                        break

            # Step 2: Verify with target model
            with torch.no_grad():
                # Get target model probabilities for all tokens including drafts
                verification_ids = torch.cat([input_ids, torch.tensor([draft_tokens]).to(input_ids.device)], dim=1)
                target_outputs = self.target_model(verification_ids)
                target_logits = target_outputs.logits[:, input_ids.shape[1]-1:]

                # Convert to probabilities
                target_probs = F.softmax(target_logits, dim=-1)

                # Accept tokens until mismatch or all drafts accepted
                accepted_tokens = []
                for i, token_id in enumerate(draft_tokens):
                    # Check if target model agrees with draft
                    target_prob = target_probs[0, i, token_id].item()
                    draft_prob = 1.0  # Draft chose this with highest probability

                    # Accept based on probability ratio (simplified)
                    if random.random() < min(1.0, target_prob / draft_prob):
                        accepted_tokens.append(token_id)
                    else:
                        # Rejection: get new token from target model
                        new_token = torch.argmax(target_logits[0, i]).item()
                        accepted_tokens.append(new_token)
                        break

            # Add accepted tokens to input_ids
            input_ids = torch.cat([input_ids, torch.tensor([accepted_tokens]).to(input_ids.device)], dim=1)

            # Check for end token
            if input_ids[0, -1].item() == self.tokenizer.eos_token_id:
                break

        # Decode the generated tokens
        return self.tokenizer.decode(input_ids[0])
```

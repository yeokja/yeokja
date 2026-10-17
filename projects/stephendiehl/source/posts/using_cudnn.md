---
date: "2025-05-15"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/using_cudnn/"
---

# Using CUDA Deep Neural Network (cuDNN) in Python

Let's go through how to implement scaled dot product attention using the cuDNN Python API. This is the most computationally expensive part of inference in a transformer-style model, while also being partially parallelizable so it's usually offloaded to the GPU. Under the hood this uses the FlashAttention-2 algorithm but provides an API that is higher level than just the [raw kernel implementation](https://github.com/Dao-AILab/flash-attention).

First, we import necessary libraries: `cudnn` for the cuDNN API, `torch` for tensor operations and comparison. Note that you will need a GPU with compute capability SM80 architecture (Ampere) or above in order to use the optimized kernels.

``` python
import cudnn
import torch
import math

torch.manual_seed(0xDEADBEEF)
handle = cudnn.create_handle()

assert torch.cuda.is_available()
```

Next, we define the problem dimensions for our attention mechanism, based on a GPT-2 like configuration: batch size (`b`), number of heads (`h`), maximum sequence length (`s`), and embedding dimension per head (`d`). The attention scaling factor `attn_scale` is also calculated.

``` python
b = 4
h = 12
s = 1024
d = 64

attn_scale = 1.0 / math.sqrt(d)
```

We then create the query (Q), key (K), value (V), and output (O) tensors on the GPU using PyTorch. These tensors are initialized with half-precision (`.half()`). We define their logical dimensions as `(b, h, s, d)` (BHSD). Importantly, we specify a physical memory layout using `strides` that corresponds to BSHD (Batch, Sequence, Head, Dims_per_head) to demonstrate explicit layout control. The `as_strided` method applies this layout without data copies.

``` python
dims = (b, h, s, d)
strides = (s * h * d, d, h * d, 1) # BSHD physical layout

q_gpu = torch.randn(b * s * h * d).half().cuda().as_strided(dims, strides)
k_gpu = torch.randn(b * s * h * d).half().cuda().as_strided(dims, strides)
v_gpu = torch.randn(b * s * h * d).half().cuda().as_strided(dims, strides)
o_gpu = torch.empty(b * s * h * d).half().cuda().as_strided(dims, strides)
```

Now, we construct the cuDNN computation graph. The cuDNN graph system provides a high-level interface for describing tensor operations as a dataflow graph that can be efficiently executed on the GPU. Users build up their computation by adding operations to the graph, which typically represents a subset of their full neural network that they want to optimize and offload to specialized GPU kernels. Once the graph is finalized, cuDNN provides multiple execution engines with different tradeoffs—some prioritizing ease of use, others minimizing runtime overhead or maximizing performance.

The graph is initialized specifying half-precision for I/O and float precision for intermediate and compute operations. We create graph tensor descriptors (`q`, `k`, `v`) that mirror our PyTorch GPU tensors. The SDPA operation is added to the graph, configured for inference (`is_inference=True`), using the calculated `attn_scale`, and with causal masking enabled (`use_causal_mask=True`). The second return value (stats tensor) is ignored as it's for training. Finally, the output graph tensor `o` is marked as an output and its dimensions and strides are set.

``` python
graph = cudnn.pygraph(
    io_data_type=cudnn.data_type.HALF,
    intermediate_data_type=cudnn.data_type.FLOAT,
    compute_data_type=cudnn.data_type.FLOAT,
)

q = graph.tensor_like(q_gpu)
k = graph.tensor_like(k_gpu)
v = graph.tensor_like(v_gpu)

o, _ = graph.sdpa(
    name="sdpa",
    q=q,
    k=k,
    v=v,
    is_inference=True,
    attn_scale=attn_scale,
    use_causal_mask=True,
)

o.set_output(True).set_dim(dims).set_stride(strides)
```

With the graph defined, we build it. This involves validating the graph structure, building an internal operation graph, creating execution plans using cuDNN's heuristics (Mode A and Fallback), checking if the chosen plans are supported, and finally building these plans, which may involve JIT compilation of kernels. Heuristics Mode A is designed to be fast and be able to handle most operation graph patterns. Heuristics Mode B is designed to be more accurate but slower.

``` python
graph.validate()
graph.build_operation_graph()
graph.create_execution_plans([cudnn.heur_mode.A, cudnn.heur_mode.FALLBACK])
graph.check_support()
graph.build_plans()
```

To execute the graph, we create a `variant_pack` dictionary. This maps the symbolic graph tensors (defined earlier) to the actual PyTorch GPU tensors holding the data. We also query the required workspace size for the execution plan and allocate it on the GPU. The graph is then executed, and `torch.cuda.synchronize()` ensures the computation completes before proceeding.

``` python
variant_pack = {
    q: q_gpu,
    k: k_gpu,
    v: v_gpu,
    o: o_gpu,
}

workspace = torch.empty(graph.get_workspace_size(), device="cuda", dtype=torch.uint8)
graph.execute(variant_pack, workspace)
torch.cuda.synchronize()
```

Finally, we verify the output of our cuDNN SDPA implementation against PyTorch's native `scaled_dot_product_attention`. We prepare reference tensors `q_ref`, `k_ref`, `v_ref` from our original GPU tensors, converting them to float. Since our cuDNN tensors used a BSHD physical layout, and PyTorch's native function expects 4D inputs in BHSD, we permute the reference tensors from BSHD to BHSD. We then compute the reference output `o_ref`.

In simple terms, permute(0, 2, 1, 3) rearranges the dimensions of a 4D tensor. If the original tensor's dimensions represent (Batch, Sequence, Head, Dim_per_head) (BSHD), this operation swaps the second and third dimensions (Sequence and Head) to produce a tensor with dimensions (Batch, Head, Sequence, Dim_per_head) (BHSD). This is done because the torch.nn.functional.scaled_dot_product_attention function expects its 4D input tensors to be in the BHSD format, so the permutation ensures the tensor's layout matches the function's requirements.

``` python
q_ref = q_gpu.detach().clone().float()
k_ref = k_gpu.detach().clone().float()
v_ref = v_gpu.detach().clone().float()

# Permute from BSHD to BHSD for PyTorch native SDPA
q_ref = q_ref.permute(0, 2, 1, 3).contiguous()
k_ref = k_ref.permute(0, 2, 1, 3).contiguous()
v_ref = v_ref.permute(0, 2, 1, 3).contiguous()

o_ref = torch.nn.functional.scaled_dot_product_attention(
    q_ref, k_ref, v_ref, is_causal=True, scale=attn_scale
)

o_gpu_for_comparison = o_gpu.float()
# Permute cuDNN output from BSHD to BHSD for comparison
o_gpu_for_comparison = o_gpu_for_comparison.permute(0, 2, 1, 3).contiguous()

torch.testing.assert_close(o_ref, o_gpu_for_comparison, atol=5e-3, rtol=3e-3)
print("PyTorch and cuDNN SDPA implementations match!")
```

In the modern AI tooling stack, tools like the cuDNN Frontend API sit at below high-level deep learning frameworks (like PyTorch or Jax) but above raw CUDA programming. Exposing this level of control over highly optimized kernels like SDPA directly allows for rapid prototyping and performance tuning of model components without needing to delve into C++ for every optimization.

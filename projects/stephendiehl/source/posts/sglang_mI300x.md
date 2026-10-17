---
date: "2025-03-04"
tags: ["ai"]
url: "https://www.stephendiehl.com/posts/sglang_mI300x/"
---

# SGLang on AMD MI300X

The best kept secret these days is that new AMD GPUs are *incredibly* cost efficient for doing inference. If you want the cliff-notes on getting an extremely fast inference server running on a fleet of AMD MI300X, here you go:

If you're serving high-throughput inference, your options for inference servers are essentially:

- [TensorRT-LLM](https://github.com/NVIDIA/TensorRT-LLM): NVIDIA's high-performance deep learning inference library that optimizes models for deployment on GPUs, focusing on low-latency and high-throughput.
- [vLLM](https://github.com/vllm-project/vllm): A versatile framework for serving large language models, particularly well-suited for Kubernetes environments and high-throughput inference scenarios.
- [**SGLang**](https://github.com/sgl-project/sglang): A framework designed for efficient inference with advanced features like zero overhead batch scheduling and optimized attention kernels.

In my experience, SGLang is the easiest to get running. Although if you're using kserve to manage your Kubernetes inference service then you'll probably end up using vLLM because of network effects and lock-in.

SGLang perfmance is extremely good though, with [zero overhead batch scheduling](https://lmsys.org/blog/2024-12-04-sglang-v0-4/) with [FlashInfer](https://flashinfer.ai/2024/02/02/introduce-flashinfer.html) attention kernels and [RadixAttention](https://arxiv.org/abs/2312.07104) for automatic KV cache reuse there's a lot of optimizations that can yield performance gains. SGLang also has an [extremely fast constrained decoding algorithm](https://lmsys.org/blog/2024-02-05-compressed-fsm/) if you're handlings lots of JSON output.

## Setup

To get started, pull the latest SGLang image:

```
docker pull rocm/sglang-staging:latest
```

Run the container.

```
docker run -d \
  -it \
  --ipc=host \
  --network=host \
  --privileged \
  --device=/dev/kfd \
  --device=/dev/dri \
  --device=/dev/mem \
  --group-add render \
  --security-opt seccomp=unconfined \
  -v /home:/workspace \
  rocm/sglang-staging:latest
```

Then get the container id and run the following command to get a shell into the container.

```
docker exec -it $CONTAINER_ID bash
```

Then login to Hugging Face.

```
huggingface-cli login
```

Then run the following command to start the server, we'll use the DeepSeek-R1 model and quantize it to FP8.

```
HSA_NO_SCRATCH_RECLAIM=1 python -m sglang.launch_server \
  --model deepseek-ai/DeepSeek-R1 \
  --tp 8 \
  --quant fp8 \
  --trust-remote-code \
  --port 8000
```

Then run the following command to start the client.

``` python
import openai

port = 8000
client = openai.Client(base_url=f"http://127.0.0.1:{port}/v1", api_key="None")

response = client.chat.completions.create(
    model="deepseek-ai/DeepSeek-R1",
    messages=[
        {"role": "user", "content": "Who wrote the Simarillion?"},
    ],
    temperature=0,
    max_tokens=64,
)

print_highlight(f"Response: {response}")
```

There are also some other tricks you can do to get better performance. For example, you can disable NUMA balancing.

```
sudo sh -c 'echo 0 > /proc/sys/kernel/numa_balancing'
```

For more tuning guides, see the AMD documentation:

- [AMD MI300X Tuning Guides](https://rocm.docs.amd.com/en/latest/how-to/tuning-guides/mi300x/index.html)
- [LLM inference performance validation on AMD Instinct MI300X](https://rocm.docs.amd.com/en/latest/how-to/rocm-for-ai/inference/vllm-benchmark.html)
- [AMD Instinct MI300X System Optimization](https://rocm.docs.amd.com/en/latest/how-to/system-optimization/mi300x.html)
- [AMD Instinct MI300X Workload Optimization](https://rocm.docs.amd.com/en/latest/how-to/rocm-for-ai/inference-optimization/workload.html)

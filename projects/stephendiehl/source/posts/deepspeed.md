---
date: "2023-06-05"
tags: ["ai"]
url: "https://www.stephendiehl.com/posts/deepspeed/"
---

# Going Deep on DeepSpeed

Getting started with training modern AI models on multi-GPU hardware can feel quite daunting. So let's break down the key concepts and how to get started with the Microsoft DeepSpeed library. Here's a short DeepSpeed tutorial, this is a deep (har har) topic so we'll only scratch the surface.

DeepSpeed's core technology is the Zero Redundancy Optimizer called ZeRO which enables training large models by partitioning model states across GPUs. ZeRO works in several stages:

- **ZeRO-1**: Partitions optimizer states across GPUs
- **ZeRO-2**: Partitions gradients and optimizer states
- **ZeRO-3**: Partitions model parameters, gradients, and optimizer states

**ZeRO-1: Optimizer State Partitioning**

ZeRO-1 focuses on partitioning optimizer states, such as momentum and variance in Adam, across multiple GPUs. In this configuration, each GPU maintains only its portion of the optimizer states, which reduces memory usage by approximately 4x when using the Adam optimizer. This stage introduces minimal communication overhead and is particularly effective in scenarios where optimizer states are the primary memory bottleneck.

**ZeRO-2: Gradient Partitioning**

Building upon ZeRO-1's optimizations, ZeRO-2 extends the partitioning to include gradients across GPUs. Each GPU stores gradients only for its designated parameters, resulting in memory reduction of roughly 8x compared to standard data parallel training. While this stage introduces moderate communication overhead, it provides an excellent balance between memory savings and performance.

**ZeRO-3: Parameter Partitioning**

ZeRO-3 represents the most aggressive optimization stage, incorporating all the benefits of ZeRO-1 and ZeRO-2 while also partitioning the model parameters across GPUs. Under this configuration, each GPU maintains only its assigned parameters, leading to memory reduction of 16x or more. Though it introduces higher communication overhead, ZeRO-3 is essential for training very large models that wouldn't otherwise fit in GPU memory, enabling the training of models with hundreds of billions of parameters.

**Installation**

To install DeepSpeed we install from PyPI.

``` bash
pip install deepspeed
```

For optimal performance, it's recommended to install from source to match your hardware:

``` bash
git clone https://github.com/microsoft/DeepSpeed.git
cd DeepSpeed
pip install -e .
```

Before training, estimate your GPU VRAM memory requirements using DeepSpeed's built-in tool:

``` python
from transformers import AutoModel
from deepspeed.runtime.zero.stage3 import estimate_zero3_model_states_mem_needs_all_live

model = AutoModel.from_pretrained("meta-llama/Llama-3.2-1B")
estimate_zero3_model_states_mem_needs_all_live(model, num_gpus_per_node=1, num_nodes=1)
```

This will output estimated memory needs for different configurations. The output will look like this:

```
Estimated memory needed for params, optim states and gradients for a:
HW: Setup with 1 node, 8 GPUs per node.
SW: Model with 2851M total params.
  per CPU  |  per GPU |   Options
  127.48GB |   5.31GB | offload_optimizer=cpu
  127.48GB |  15.93GB | offload_optimizer=none
```

**Configuration Files**

DeepSpeed is configured through a JSON file typically named `ds_config.json`. Here's a basic ZeRO-3 configuration:

``` json
{
  "fp16": {
    "enabled": "auto",
    "loss_scale": 0,
    "loss_scale_window": 1000,
    "initial_scale_power": 16,
    "hysteresis": 2,
    "min_loss_scale": 1
  },
  "optimizer": {
    "type": "AdamW",
    "params": {
      "lr": "auto",
      "betas": "auto",
      "eps": "auto",
      "weight_decay": "auto"
    }
  },
  "scheduler": {
    "type": "WarmupLR",
    "params": {
      "warmup_min_lr": "auto",
      "warmup_max_lr": "auto",
      "warmup_num_steps": "auto"
    }
  },
  "zero_optimization": {
    "stage": 3,
    "offload_optimizer": {
      "device": "cpu",
      "pin_memory": true
    },
    "offload_param": {
      "device": "cpu",
      "pin_memory": true
    },
    "overlap_comm": true,
    "contiguous_gradients": true,
    "stage3_gather_16bit_weights_on_model_save": true
  },
  "gradient_accumulation_steps": "auto",
  "gradient_clipping": "auto",
  "train_batch_size": "auto",
  "train_micro_batch_size_per_gpu": "auto"
}
```

The most important part of the configuration is the `zero_optimization` section which controls how ZeRO is applied. In this example, we're using ZeRO-3 with optimizer and parameter states offloaded to CPU memory. The offloading is specified with the `offload_optimizer` and `offload_param` sections. This describes how the model training will be distributed across multiple GPUs and gather the results back.

Now to use DeepSpeed with the HuggingFace Trainer in a file `train.py`, simply add the `--deepspeed` flag and config file poining at your DeepSpeed configuration JSON file.

``` python
from transformers import Trainer, TrainingArguments

training_args = TrainingArguments(
    output_dir="output",
    deepspeed="ds_config.json",  # DeepSpeed config file
    fp16=True,
    # Put other training arguments here
)

trainer = Trainer(
    model=model,
    args=training_args,
    train_dataset=dataset,
    # Put other trainer arguments here
)

trainer.train()
```

**Launching Training**

If you have a single server with multiple GPUs attached you can launch training with the following command:

``` bash
deepspeed --num_gpus=8 train.py \
    --num_nodes=1 \
    --num_gpus=8 \
    --deepspeed ds_config.json \
    --other_training_args
```

If you have a multi-node setup you can launch training with the following command:

``` bash
deepspeed --hostfile=hostfile \
    --num_nodes=2 \
    --num_gpus=8 \
    train.py \
    --deepspeed ds_config.json \
    --other_training_args
```

Where the local file `hostfile` contains that list of hostnames and the number of GPUs per host.

```
worker-1 slots=8
worker-2 slots=8
```

When getting started with DeepSpeed, it's best to begin with simpler configurations. Start by using ZeRO-2 optimization and only move to the more aggressive ZeRO-3 if you find you need the additional memory savings. For managing GPU memory constraints, CPU offloading can be helpful but should only be enabled when GPU memory alone is insufficient, as it introduces additional overhead. Gradient checkpointing provides another way to reduce memory usage by trading it for additional computation time.

The batch size is another important consideration that requires careful tuning. Begin with smaller batch sizes and gradually increase them while closely monitoring memory usage to find the optimal balance for your hardware. Finally, take advantage of mixed precision training using fp16 or bf16 when possible, as this can provide significant performance improvements without sacrificing model quality in most cases.

**Troubleshooting**

Running models in multi-GPU configurations can be extremely trickly because of both hardware and software failures. The most common failure modes (that you absolutely will encounter) are:

1.  NaN loss values.
2.  Out of memory errors.
3.  CPU offloading failures.
4.  Random hardware failures.

NaN loss values are a common problem when using fp16 precision with models that were pre-trained using bf16. In these cases, switching to either bf16 or fp32 precision usually resolves the issue.

If you run into OOM errors, you can try enabling CPU offloading or increasing the ZeRO optimization stage to reduce GPU memory usage. However, be mindful that if your process gets killed entirely, it may indicate insufficient CPU memory when using offloading features.

Performance issues like slow training speeds can often be traced back to unnecessary CPU offloading. If your GPU memory is sufficient to handle the model and training data, disabling CPU offloading will typically result in faster training times.

When optimizing GPU memory usage, start by implementing gradient checkpointing before enabling various ZeRO stages. Mixed precision training using fp16 or bf16 can significantly reduce memory requirements while maintaining model quality. Regular monitoring of GPU memory `nvidia-smi` during training is essential. CPU or NVMe offloading should only be considered when GPU memory alone proves insufficient for your training needs.

When implementing CPU offloading, begin by offloading optimizer states, as this has a relatively minor impact on performance. Parameter offloading should only be employed when absolutely necessary, as it introduces significant overhead. Ensure your system has sufficient CPU RAM, typically 2-4 times the model size, to handle offloading effectively. For models that exceed CPU RAM capacity, consider using NVMe offload capabilities as a last resort.

Effective batch size tuning begins with smaller sizes that are gradually increased as training stabilizes. Gradient accumulation can be employed to achieve effectively larger batch sizes without increasing memory requirements proportionally. Careful monitoring of loss scaling is essential, and adjustments should be made based on convergence patterns. Some training scenarios may benefit from dynamic batch sizing that adapts to available memory conditions.

Pipeline parallelism offers another dimension of model distribution by splitting model layers across multiple GPUs. This approach effectively reduces the memory required per GPU and enables training of very deep models that wouldn't fit on a single device. Pipeline parallelism can be combined with various ZeRO stages to achieve optimal training configurations for specific hardware setups.

Activation checkpointing provides a clever trade-off between computation and memory usage by recomputing activations during the backward pass instead of storing them. This technique can significantly reduce the memory footprint of training

At this point in time, setting up multi-GPU training is not trivial and requires a lot of careful thinking. It's a lot more art than science, and involves a lot of trial and error.

---
date: "2024-11-20"
tags: ["ai", "ssm"]
url: "https://www.stephendiehl.com/posts/mamba_x_transformers/"
---

# Hymba : Mamba × Transformers

Up until now we've had transformers, which are the core deep learning architecture based on the attention mechanism. But there are other experimental architectures that have been gaining a lot of attention lately. One of the more promising ones is the state-space model **Mamba**, an architecture developed by researchers at Carnegie Mellon University and Princeton University that aims to address some key limitations of transformer models, particularly in processing long sequences and large context windows.

At its core, Mamba is built on three fundamental ideas. The first is its use of selective state spaces - rather than relying on attention mechanisms like transformers do, Mamba employs SSMs that can selectively process information based on the current input. This selective processing allows the model to effectively focus on relevant information while discarding what isn't needed. The theory is best described in this [blog post](https://srush.github.io/annotated-s4/).

Mamba offers several key advantages over traditional transformer architectures. The most significant benefits is better scaling through linear-time processing, making it substantially more efficient when handling long sequences compared to transformers' quadratic scaling behavior. The architecture is also notably memory efficient, carefully avoiding the materialization of expanded states in memory-intensive layers that can bog down other models.

The second idea is Mamba's simplified architecture. Unlike transformers which separate attention and MLP blocks, Mamba unifies these into a single SSM block. This architectural simplification leads to reduced computational complexity and improved inference speed. However, despite the conceptual promise, SSMs have been shown to have limited recall capabilities and underperforming on in-context learning tasks.

In short, traditional attention heads are great at precisely remembering and recalling specific information, like a photographic memory, though they're computationally expensive and memory-hungry. state space models, on the other hand, are more efficient but work somewhat like human memory - good at capturing the general gist of things but not great at precise recall.

| Feature | Transformer | Mamba |
|----|----|----|
| Core Mechanism | Attention-based | SSM-based |
| Architectural Complexity | Higher (separate attention & MLP blocks) | Lower (unified SSM block) |
| Training Complexity | $O(n^2)$ | $O(n)$ |
| Inference Speed | $O(n)$ | $O(1)$ |

**Hymba**

Nvidia [recently explored](https://www.arxiv.org/abs/2411.13676) combining different computational paradigms to leverage their complementary strengths. A notable example is the Hymba architecture, which introduces a hybrid-head parallel design that integrates transformer attention mechanisms with state space models within the same layer. The end result is a model that outperforms larger models while using significantly less memory. On the self-reported benchmarks, Hymba-1.5B beats Llama-3.2-3B while using 11.67x less memory cache and running 3.49x faster. Which is incredibly impressive.

The key innovation in hybrid architectures like Hymba is the parallel fusion of attention and SSM heads, allowing simultaneous processing of inputs through different computational paths. In contrast to sequential approaches that stack different layer types, parallel fusion enables each layer to simultaneously leverage both the high-resolution recall capabilities of attention and the efficient context summarization of SSMs. This is implemented through a unified formulation where the input is projected to both attention components (query, key, value) and SSM components (input features and gates) in parallel. The outputs from attention heads and SSM heads are normalized and combined using learnable scaling factors $\beta_1$ and $\beta_2$. Specifically, if we denote the attention output as $Y_{attn}$ and the SSM output as $Y_{ssm}$, the final layer output is computed as:

$$Y = W_{out_proj}(\beta_1 \cdot \text{norm}(M_{attn} \cdot X) + \beta_2 \cdot \text{norm}(M_{ssm} \cdot X))$$

where $\beta_1$ and $\beta_2$ are control the relative contribution of each path.

**Memory and Efficiency Optimizations**

To address the memory demands of attention mechanisms, hybrid architectures employ several optimizations. These include combining global and local attention patterns, where only select layers (typically first, middle, and last) use full attention while others use sliding window attention. Additionally, key-value cache sharing between layers reduces memory requirements without significantly impacting model performance.

The SSM components help maintain global context awareness even with localized attention, allowing more aggressive use of sliding window attention compared to pure transformer architectures. Empirical studies show that hybrid models can maintain comparable task performance while achieving significant reductions in memory usage and increased throughput.

Additionally the architecture includes learnable meta tokens that are prepended to input sequences. These tokens serve a twofold purpose: they act as learned cache initialization, help redistribute attention patterns more effectively, and store compressed world knowledge. The meta tokens participate in both attention and SSM computations, essentially providing a form of learned memory initialization that helps guide the model's focus toward relevant information.

The Hybrid architecture empirically demonstrates several advantages over both pure transformer and pure SSM architectures. In the paper, it achieves better effective receptive field measurements while maintaining reasonable cache sizes. The relative importance of attention and SSM heads varies by layer and task, suggesting that the architecture learns to leverage different computational modes as needed. This flexibility contributes to strong performance across both general language tasks and memory-intensive applications while maintaining competitive efficiency metrics.

These hybrid architectures are super exciting - they show us that the future of language models probably isn't about picking between different approaches, but rather mixing and matching the best parts of each. By cleverly combining different architectures like this, we could theoretically get the best of both worlds - fast, efficient models that are only worst case quadratic and still incredibly capable. The early results are really promising.

**Using It**

Before running the code, ensure you have the correct environment setup with CUDA 12.4 and [FlexAttention](https://pytorch.org/blog/flexattention/).

``` bash
# Install PyTorch with CUDA 12.4
pip install pytorch==2.5.0
pip install torchvision==0.20.0
pip install torchaudio==2.5.0
pip install pytorch-cuda=12.4

# Install required packages
pip install --upgrade transformers
pip install tiktoken sentencepiece protobuf ninja einops triton packaging

# Install Mamba
git clone https://github.com/state-spaces/mamba.git
cd mamba
pip install -e .
cd ..

# Install causal-conv1d
git clone https://github.com/Dao-AILab/causal-conv1d.git
cd causal-conv1d
export CUDA_HOME=/usr/local/cuda-12.4
export TORCH_CUDA_ARCH_LIST="7.0;7.5;8.0;8.6;8.9;9.0"
python setup.py install
cd ..

# Install attention-gym
git clone https://github.com/pytorch-labs/attention-gym.git
cd attention-gym
pip install .
cd ..

# Install Flash Attention
pip install flash_attn
```

Here's a minimal example of using Hymba through Hugging Face:

``` python
import torch
from transformers import
    AutoModelForCausalLM,
    AutoTokenizer,
    StopStringCriteria,
    StoppingCriteriaList

# Initialize model and tokenizer for chat
repo_name = "nvidia/Hymba-1.5B-Instruct"
tokenizer = AutoTokenizer.from_pretrained(repo_name, trust_remote_code=True)
model = AutoModelForCausalLM.from_pretrained(repo_name, trust_remote_code=True)
model = model.cuda().to(torch.bfloat16)

def chat_with_model(messages, model, tokenizer, max_new_tokens=256):
    # Format the conversation using the model's chat template
    tokenized_chat = tokenizer.apply_chat_template(
        messages,
        tokenize=True,
        add_generation_prompt=True,
        return_tensors="pt"
    ).to('cuda')

    # Set up stopping criteria to properly end responses
    stopping_criteria = StoppingCriteriaList([
        StopStringCriteria(tokenizer=tokenizer, stop_strings="</s>")
    ])

    # Generate response
    outputs = model.generate(
        tokenized_chat,
        max_new_tokens=max_new_tokens,
        do_sample=False,
        temperature=0.7,
        use_cache=True,
        stopping_criteria=stopping_criteria
    )

    # Decode and return only the new tokens
    input_length = tokenized_chat.shape[1]
    response = tokenizer.decode(outputs[0][input_length:], skip_special_tokens=True)
    return response

# Interactive chat loop
messages = [
    {"role": "system", "content": "You are a helpful assistant."}
]

def main():
    print("Chat with Hymba (type 'exit' to quit):")
    while True:
        print("User:")
        prompt = input()
        if prompt.lower() == "exit":
            break

        messages.append({"role": "user", "content": prompt})
        response = chat_with_model(messages, model, tokenizer)
        messages.append({"role": "assistant", "content": response})

        print(f"Model: {response}")
```

---
date: "2024-09-03"
tags: ["ai", "python", "llms"]
url: "https://www.stephendiehl.com/posts/orpo/"
---

# Fine-tuning with ORPO and Unsloth

ORPO was introduced in the paper ["ORPO: Monolithic Preference Optimization without Reference Model"](https://huggingface.co/docs/trl/main/en/orpo_trainer). The key innovation is that it eliminates the need for a separate preference alignment phase by incorporating a minor penalty for disfavored generations during the training process.

Traditional approaches like DPO require two steps:

1.  Supervised fine-tuning on the base model
2.  Preference optimization using paired data

This two-step process has a significant drawback: during SFT, the model increases the probability of generating both preferred and undesired responses. This happens because SFT treats all training examples as equally valid, without distinguishing between better and worse responses. As a result, a separate direct preference alignment step is needed to widen the gap between preferred and rejected outputs. With ORPO, we can do it all in one step, which is great and more efficient.

ORPO requires a preference dataset with three key components:

- A prompt/instruction
- A chosen (preferred) response
- A rejected (non-preferred) response

A preference dataset will look like this:

``` python
{
    "prompt": "What is the meaning of life?",
    "chosen": "The meaning of life is to grow, learn, and find purpose through our experiences and connections with others.",
    "rejected": "42 lol"
}
```

We'll use the new Llama 3.2 8B model with Unsloth for efficient training. First, let's set up our environment:

``` bash
# Install required dependencies
pip install flash-attn --no-build-isolation
pip install -v -U git+https://github.com/facebookresearch/xformers.git@main#egg=xformers
pip install "unsloth[colab-new] @ git+https://github.com/unslothai/unsloth.git"
```

Now let's set up our training script:

``` python
import torch
from datasets import load_dataset
from transformers import TextStreamer
from trl import ORPOTrainer, ORPOConfig

from unsloth import FastLanguageModel, is_bfloat16_supported
from unsloth.chat_templates import get_chat_template

# Model configuration
max_seq_length = 2048
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="unsloth/Meta-Llama-3-8B-bnb-4bit",
    max_seq_length=max_seq_length,
    load_in_4bit=True,
    dtype=None,
)

# Setup Unsloth's optimized LoRA
model = FastLanguageModel.get_peft_model(
    model,
    r=16,
    lora_alpha=16,
    lora_dropout=0,
    target_modules=[
        "q_proj", "k_proj", "v_proj",
        "o_proj", "gate_proj", "up_proj", "down_proj"
    ],
    use_rslora=True,  # Faster training
    use_gradient_checkpointing="unsloth",
)

# Setup ChatML format
tokenizer = get_chat_template(
    tokenizer,
    chat_template="chatml"
)
```

For the dataset, we'll use the `ultrafeedback_binarized` dataset which contains binarized preferences for training a model to avoid unethical and toxic responses. Using trl we don't need to preprocess the dataset since it already understands the preference format.

``` python
dataset = load_dataset("trl-lib/ultrafeedback_binarized", split="train")
```

Now let's configure the ORPO trainer with Unsloth-optimized settings:

``` python
orpo_args = ORPOConfig(
    learning_rate=8e-6,
    beta=0.1,
    lr_scheduler_type="cosine_with_restarts",  # Better with Unsloth
    max_length=2048,
    max_prompt_length=1024,
    per_device_train_batch_size=4,  # Increased due to Unsloth optimizations
    per_device_eval_batch_size=4,
    gradient_accumulation_steps=2,
    optim="adamw_8bit",
    num_train_epochs=3,
    evaluation_strategy="steps",
    eval_steps=0.2,
    logging_steps=1,
    warmup_steps=10,
    output_dir="./results/",
    fp16=not is_bfloat16_supported(),
    bf16=is_bfloat16_supported(),
)

trainer = ORPOTrainer(
    model=model,
    args=orpo_args,
    train_dataset=dataset["train"],
    eval_dataset=dataset["test"],
    tokenizer=tokenizer,
)

trainer.train()
```

After training, we can save the model in various formats:

``` python
# Save LoRA weights
model.save_pretrained("lora_model")
tokenizer.save_pretrained("lora_model")

# Save merged model in 16-bit
print("Saving merged model...")
model.save_pretrained_merged("fused_model", tokenizer, save_method="merged_16bit")

# Save GGUF formats for inference
print("Saving GGUF formats...")
model.save_pretrained_gguf("ggufs", tokenizer, quantization_method="f16")
model.save_pretrained_gguf("ggufs", tokenizer, quantization_method="q4_k_m")
```

During training, ORPO logs several important metrics:

- `rewards/chosen`: Log probabilities for chosen responses
- `rewards/rejected`: Log probabilities for rejected responses
- `rewards/accuracies`: How often chosen rewards exceed rejected rewards
- `rewards/margins`: Mean difference between chosen and rejected rewards
- `log_odds_ratio`: Mean log odds ratio of chosen vs rejected responses
- `nll_loss`: Negative log likelihood loss from SFT component

You can monitor these metrics using Weights & Biases.

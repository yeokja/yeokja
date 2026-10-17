---
date: "2024-07-28"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/unsloth/"
---

# A Rapid Tutorial on Unsloth

Unsloth is a library for fast and efficient fine-tuning of large language models. It is built on top of the Hugging Face Transformers library and can be used to fine-tune models on a variety of tasks. In this case we'll train a Llama 3.1 model to do text to sql generation.

Unsloth can be used to do 2x faster training and 60% less memory than standard fine-tuning on single GPU setups. It uses a technique called Quantized Low Rank Adaptation (QLoRA) to quantize the model and train a small number of additional trainable parameters instead of the entire model. This allows us to train the model on a single NVIDIA H100 GPU.

We'll need to use raw pip to install flash-attn and xformers from source.

``` bash
# Install flash-attn
poetry run pip install flash-attn --no-build-isolation

# Install xformers
poetry run pip install -v -U git+https://github.com/facebookresearch/xformers.git@main#egg=xformers

# Install unsloth
poetry run pip install "unsloth[colab-new] @ git+https://github.com/unslothai/unsloth.git"
```

The rest can be installed via poetry from PyPI.

``` toml
[tool.poetry.dependencies]
trl = "^0.9.6"
nvitop = "^1.3.2"
xformers = "0.0.27"
peft = "^0.12.0"
accelerate = "^0.33.0"
bitsandbytes = "^0.43.3"
```

Now we can initialize out `train.py` script. Load the following Hugging Face libraries:

``` python
import torch

from datasets import load_dataset
from transformers import EarlyStoppingCallback, TextStreamer, TrainingArguments
from trl import SFTTrainer

from unsloth import FastLanguageModel, is_bfloat16_supported
from unsloth.chat_templates import get_chat_template
```

We'll download the 8 billion parameter Llama 3.1 model that has been quantized to 4 bits. This means that the model has been compressed to use 4 bits instead of 16 bits for its weights. This makes the model much faster to load and use, but it is still a fairly large model.

``` python
max_seq_length = 2048
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name="unsloth/Meta-Llama-3.1-8B-bnb-4bit",
    max_seq_length=max_seq_length,
    load_in_4bit=True,
    dtype=None,
)

model = FastLanguageModel.get_peft_model(
    model,
    r=16,
    lora_alpha=16,
    lora_dropout=0,
    target_modules=[
        "q_proj",
        "k_proj",
        "v_proj",
        "up_proj",
        "down_proj",
        "o_proj",
        "gate_proj",
    ],
    use_rslora=True,
    use_gradient_checkpointing="unsloth",
)

tokenizer = get_chat_template(
    tokenizer,
    chat_template="chatml",
)
```

We'll use a synthetic data set of text to sql prompts.

``` python
def convert(question):
    return {"user": question["sql_prompt"], "assistant": question["sql"]}


def apply_template(examples):
    messages = examples["conversations"]
    text = [
        tokenizer.apply_chat_template(
            convert(message), tokenize=False, add_generation_prompt=False
        )
        for message in messages
    ]
    return {"text": text}


dataset = load_dataset("gretelai/synthetic_text_to_sql", split="train")
dataset = dataset.map(apply_template, batched=True)
```

We'll use the 8bit AdamW optimizer with the cosine annealing warm restarts scheduler. In addition we'll add early stopping to prevent overfitting.

``` python
callbacks = [
    EarlyStoppingCallback(early_stopping_patience=3),
]

training_args = TrainingArguments(
    learning_rate=3e-4,
    lr_scheduler_type="linear",
    per_device_train_batch_size=8,
    gradient_accumulation_steps=2,
    num_train_epochs=6,
    fp16=not is_bfloat16_supported(),
    bf16=is_bfloat16_supported(),
    logging_steps=1,
    optim = "adamw_8bit",
    lr_scheduler_type = "cosine_with_restarts",
    weight_decay=0.05,
    warmup_steps=10,
    output_dir="output",
    seed=0,
)

trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=dataset,
    dataset_text_field="text",
    max_seq_length=max_seq_length,
    dataset_num_proc=2,
    packing=True,
    args=training_args,
    load_best_model_at_end=True,
    metric_for_best_model="eval_loss",
    callbacks=callbacks,
)
trainer.train()
print("Done.")
```

After running this you should see output like this. This will run for six epochs and should take about 2-3 hours to complete on a Nvidia H100 GPU.

```
# ==((====))==  Unsloth - 2x faster free finetuning | Num GPUs = 1
#    \\   /|    Num examples = 2,254 | Num Epochs = 3
# O^O/ \_/ \    Batch size per device = 2 | Gradient Accumulation steps = 4
# \        /    Total batch size = 8 | Total steps = 843
#  "-____-"     Number of trainable parameters = 29,884,416
```

Now we can use the model to generate text.

``` python
model = FastLanguageModel.for_inference(model)

messages = [
    {
        "user": "What is the average salary of employees in the IT department?",
        "assistant": "",
    },
]
inputs = tokenizer.apply_chat_template(
    messages,
    tokenize=True,
    add_generation_prompt=True,
    return_tensors="pt",
).to("cuda")

text_streamer = TextStreamer(tokenizer)
_ = model.generate(
    input_ids=inputs, streamer=text_streamer, max_new_tokens=128, use_cache=True
)
```

The model can also be exported to a variety of formats.

``` python
model.save_pretrained_merged("model", tokenizer, save_method="merged_16bit")

trainer_stats = trainer.train()
print("Done.")


model.save_pretrained("lora_model")
tokenizer.save_pretrained("lora_model")

print("Merging model")
model.save_pretrained_merged("fused_model", tokenizer, save_method="merged_16bit")

print("Saving gguf")
model.save_pretrained_gguf("ggufs", tokenizer, quantization_method="f16")
model.save_pretrained_gguf("ggufs", tokenizer, quantization_method="q4_k_m")
```

You should see output like this:

```
==((====))==  Unsloth: Conversion from QLoRA to GGUF information
   \\   /|    [0] Installing llama.cpp will take 3 minutes.
O^O/ \_/ \    [1] Converting HF to GGUF 16bits will take 3 minutes.
\        /    [2] Converting GGUF 16bits to ['f16'] will take 10 minutes each.
 "-____-"     In total, you will have to wait at least 16 minutes.

Unsloth: [0] Installing llama.cpp. This will take 3 minutes...
Unsloth: [1] Converting model at lora-fused into f16 GGUF format.
The output location will be ./lora-fused/unsloth.F16.gguf

...

Writing: 100%|██████████| 7.64G/7.64G [01:18<00:00, 97.1Mbyte/s]
INFO:hf-to-gguf:Model successfully exported to lora-fused/unsloth.F16.gguf
Unsloth: Conversion completed! Output location: ./lora-fused/unsloth.F16.gguf
```

And that's it! You now have a fine-tuned text to sql generator. This can easily be adapted to other tasks with minimal changes to the data processing script.

From here you can also train the model using a variety of reinforcement learning techniques to further improve the model's performance.

- [Kahneman-Tversky Optimization (KTO)](https://huggingface.co/docs/trl/main/en/kto_trainer) can be used aligning language models with binary feedback data.
- [Direct Preference Optimization (DPO)](https://huggingface.co/docs/trl/main/en/dpo_trainer) can be used aligning language models with pairwise feedback data.

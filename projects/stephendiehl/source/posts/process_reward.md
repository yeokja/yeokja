---
date: "2024-12-01"
tags: ["ai", "python", "llms"]
url: "https://www.stephendiehl.com/posts/process_reward/"
---

# Process Reward Models

Reasoning models are all the rage with the kids these days, and in particular the training data behind them is a hot topic. One particular case of this is the [process reward models](https://arxiv.org/abs/2405.15489) which are a type of reward model used in complex reasoning and decision-making tasks where evaluating intermediate steps is crucial to achieving the desired outcome. In short we want to have a model that can help guide the reasoning of another model to tell it when it's on the right track or off track.

The source code for this post is available <https://github.com/sdiehl/prm>.

New approaches to scaling inference have recently led to breakthrough performance improvements in the world of LLMs. Through the use of basic sampling and search strategies, particularly when combined with custom verifiers, scoring functions, and process reward models, we're now seeing small models outperform vastly larger models. Which is quite an exciting development!

So let's back up a second and talk about reward models generally. A reward model is a specialized model that acts as an evaluator or critic for other language model outputs. It takes in text (like a response from a language model) and outputs a score indicating how "good" that text is according to correlations in it's training data about what "good" responses look like.

For example, a reward model might score responses based on the following criteria:

- *Helpfulness*: Overall helpfulness of the response to the prompt.
- *Correctness*: Inclusion of all pertinent facts without errors.
- *Coherence*: Consistency and clarity of expression.
- *Complexity*: Intellectual depth required to write response (i.e. whether the response can be written by anyone with basic language competency or requires deep domain expertise).
- *Verbosity*: Amount of detail included in the response, relative to what is asked for in the prompt.

The [Nemotron-4-340B-Reward](https://huggingface.co/nvidia/Nemotron-4-340B-Reward) uses this metric to evaluate the quality of a user/assistant interaction.

``` python
from openai import OpenAI

client = OpenAI(
  base_url = "https://integrate.api.nvidia.com/v1",
  api_key = "..."
)

completion = client.chat.completions.create(
    model="nvidia/nemotron-4-340b-reward",
    messages=[
      {"role": "user", "content": "What is the capital of France?"},
      {"role": "assistant", "content": "Paris is the capital of France."}
    ],
)
print(completion)
```

The output of the reward model is an enumeration of the grade of the user/assistant interaction.

``` python
helpfulness: 2.1875
correctness: 2.109375
coherence: 3.46875
complexity: 0.330078125
verbosity: 0.38671875
```

Reward models are typically trained on human preferences - showing the model pairs of responses and teaching it which one humans preferred. This allows the reward model to learn what makes a "good" response according to human judgment.

A variant of this is the [Process Reward Model](https://arxiv.org/abs/2405.15489), which evaluates the quality of each step in a reasoning process. Process Reward Models are a type of reward model used in complex reasoning and decision-making tasks where evaluating intermediate steps is crucial to achieving the desired outcome. Unlike Outcome Reward Models (ORMs) that only consider the final result, PRMs provide feedback at each stage, capturing the value of intermediate actions and offering a more granular perspective on the problem-solving process.

## Processes

By a *process* we mean a sequence of steps that a actor (or language model) takes to solve a problem. We'll assume these are set of natural language steps encoded as tokens which "guide" the actor towards a solution.

``` python
[
    "Calculate the area of a circle with radius 5.",
    "We know that the formula for the area of a circle is $\pi r^2$.",
    "The radius of the circle is 5, so we can substitute that in.",
    "So the area of the circle is $\pi(5)^2$ = $\boxed{25\pi}$.",
]
```

If we have a problem specification

- $x$: problem specification
- $z_{1:T} \in \mathcal{S}^T$: sequence of steps taken to solve the problem
- $y \in \mathcal{Y}$: final answer

A chain of thought is then a sequence such as $(x, z_{1:T}, y)$ like:

$$S^4 = \{x, z_1, z_2, z_3, z_4, y\}$$

Might look like:

$$\begin{align*}
x &= \text{Find the area of a circle with radius 5} \\
z_1 &= \text{Let's use the formula for circle area} \\
z_2 &= \text{The formula is} A = \pi r^2 \\
z_3 &= \text{Substituting } r=5 \\
z_4 &= \text{Computing } A = \pi(5)^2 = 25\pi \\
y &= 25\pi
\end{align*}$$

The probability of a chain of thought $(x, z_{1:T}, y)$ is given by

$$p(y|x) = \mathbb{E}_z p(y | x, z)$$

For some problems we will have a verifier function that can be used to evaluate the correctness of a final answer.

$$\text{Ver}_x : \mathcal{Y} \to \{0, 1\}$$

We may also have a verifier function that can be used to evaluate the correctness of a sequence of steps.

$$\text{Ver}_x : \mathcal{S}^T \to \{0, 1\}$$

These functions are possible for certain domains like mathematical reasoning, it may be possible to formally verify proofs or calculations but this still remains an active area of research and is at minimum a very hard engineering problem and intractable in the general case (a la Rice's theorem). However we can still train probabilistic models to evaluate the quality of a chain of thought and in practice this can guide the inference of the model towards better behavior even in the case where the verifier function is intractable.

Some other terminology:

- A **non-sequitor** is a step that does not contribute to the solution of the problem.
- A process is said to be **complete** if it has no non-sequitors.
- A process is said to be **correct** if it reaches the correct final answer.
- A process is said to be **valid** if and only if it is complete and correct.

This is somewhat subtle, because a process can be correct and incomplete or incorrect and complete. For example, a process could reach the right answer but include unnecessary or irrelevant steps (correct but incomplete). Conversely, a process could follow a perfectly logical sequence of steps but arrive at the wrong conclusion due to a calculation error or faulty assumption (incorrect but complete). The ideal process is both complete and correct - containing only relevant steps that contribute to reaching the right answer.

We will also use the term *trajectory* to refer to families of processes that start with the same initial state and may end with different final states. Formally, we denote a trajectory as $\mathcal{T}(x)$ which represents the set of all possible process sequences $S^T$ that begin with initial state $x$:

$$\mathcal{T}(x) = {S^T \in \mathcal{S}^T : S^T_1 = x}$$

The goal of a *process reward model* is to learn a mapping from a trajectory to a reward value (i.e. $r : \mathcal{S}^T \to \mathbb{R}$) in order to guide sampling of valid processes.

This is contrast to a *outcome reward model* which maps from a final answer to a reward value (i.e. $r : \mathcal{Y} \to \mathbb{R}$) which is common in traditional post-training in which the models learn to generate final answers that are correct.

## Generating Processes

The simplest and most accurate way way to generate processes is to use a human experts to solve the problem and write the process. This however is not scalable so we need to develop other methods.

A major breakthrough in the field came when researchers discovered that models could learn to reason better by generating and refining their own chains of thought through reinforcement learning, rather than just imitating human-written examples. This was a pivotal realization that unlocked the ability to scale reasoning capabilities far beyond what was possible with human demonstrations alone. The discovery that models could effectively bootstrap their own reasoning process through RL represented a fundamental shift in how we now approach training language models to solve complex problems.

The simplest way to start bootstrapping these processes is to use the model itself to generate them using the *guess and check* method. Assuming we have a problem specification $x$ and a set of known answers $\mathcal{Y}$ we can generate a set of trajectories $\mathcal{T}(x)$ by:

1.  Generate multiple solution trajectories (typically around 15 per problem) by having the model attempt different reasoning paths
2.  For each intermediate step in a trajectory, sample multiple possible completions (around 16 per step)
3.  Evaluate each completion - if any completion reaches the correct final answer, label that step as positive (`+`)
4.  If all completions for a step fail to reach the correct answer, label that step as negative (`-`)

We let the langauge model explore different paths and then select the paths that lead to the correct answers. There is no guarantee that this will find the minimal process (or even a valid process) but it will find some correct processes and this is good starting point.

## Building a Process Reward Model

We'll create a training pipeline that builds models to evaluate the quality of each step in a chain of reasoning based on a set of training examples about mathematical reasoning. We're going to use the base `Mistral-7B-0.3` model and train it to act as a PRM as it has generally good performance and is small enough to train on a modest GPU.

The training process involves fine-tuning the model to predict whether each step in a reasoning chain is helpful (+) or unhelpful (-) towards reaching the correct solution. This binary classification approach allows the model to learn what constitutes effective reasoning steps across different types of problems.

You'll need to install the `prm` library and the `transformers`, `datasets`, and `torch` libraries. The PRM library provides specialized training utilities for process reward models, while the Hugging Face libraries handle the underlying model architecture and training infrastructure.

``` bash
pip install transformers datasets torch
pip install git+https://github.com/sdiehl/prm.git
```

First, let's set up our training script with the necessary imports and configurations. We'll need to configure several system-level settings to handle the large model efficiently, including proper multiprocessing setup and tokenizer parallelism. The model initialization includes setting the pad token to match the EOS token (a common practice for causal language models) and disabling the KV cache to save memory during training.

``` python
from transformers import AutoModelForCausalLM, AutoTokenizer, TrainingArguments
from datasets import Dataset
import torch
from prm.trainer import ProcessRewardTrainer
import os

# Configure system settings
torch.multiprocessing.set_sharing_strategy("file_system")
os.environ["TOKENIZERS_PARALLELISM"] = "false"

# Initialize model and tokenizer
tokenizer = AutoTokenizer.from_pretrained("mistralai/Mistral-7B-v0.3")
model = AutoModelForCausalLM.from_pretrained("mistralai/Mistral-7B-v0.3")

# Configure model settings
tokenizer.pad_token = tokenizer.eos_token
model.config.use_cache = False
model.config.pad_token_id = tokenizer.pad_token_id

# Setup the step string format template
STEP_TEMPLATE = "Step {i}: {step}"
```

To train our Process Reward Model, we need high-quality training data that demonstrates both correct reasoning chains and common pitfalls. Let's create a simple dataset with mathematical reasoning steps. Each example consists of an input problem, a sequence of reasoning steps, and corresponding labels indicating whether each step contributes positively to the solution. This structure allows the model to learn the characteristics of effective problem-solving processes.

``` python
raw_dataset = [
    {
        "input": "Solve the equation: 2x + 6 = 14",
        "value": [
            "First, subtract 6 from both sides:\n2x + 6 - 6 = 14 - 6\n2x = 8",
            "Then, divide both sides by 2:\n2x ÷ 2 = 8 ÷ 2\nx = 4",
            "Check the solution:\n2(4) + 6 = 14\n8 + 6 = 14\n14 = 14 ✓"
        ],
        "label": ["+", "+", "+"]
    }
]
```

Now we'll create a function to convert the examples into training examples. This function processes each example by accumulating steps and formatting them into a structured text format that the model can understand. By iterating over each step, we build a conversation-like history that the model uses to learn the sequence of reasoning.

``` python
def create_training_examples(examples):
    """Convert step-by-step reasoning into training examples"""
    training_examples = []

    for example in examples:
        prompt = example["input"]
        steps = example["value"]
        labels = example["label"]
        accumulated_steps = []

        for i, current_step in enumerate(steps):
            accumulated_steps.append(current_step)

            # Build conversation history
            full_text = prompt
            for j, step in enumerate(accumulated_steps, 1):
                full_text += f"\n{STEP_TEMPLATE.format(i=j, step=step)}"

            training_examples.append({
                "text": full_text,
                "label": labels[i],
                "step_idx": i,
                "is_final_step": (i == len(steps) - 1)
            })

    return training_examples
```

Next, we need to preprocess the training examples into a format suitable for the model. This involves tokenizing the text and converting labels into a format that the model can use to learn. The preprocessing function handles truncation and padding to ensure that all inputs are of a consistent length, which is crucial for efficient batch processing during training.

``` python
def preprocess_function(examples):
    """Convert examples to model inputs"""
    encoded = tokenizer(
        examples["text"],
        truncation=True,
        padding="max_length",
        max_length=2048,
        return_tensors="pt"
    )

    reward_tokens = torch.tensor([
        tokenizer.convert_tokens_to_ids(label)
        for label in examples["label"]
    ])

    return {
        "input_ids": encoded["input_ids"],
        "attention_mask": encoded["attention_mask"],
        "labels": encoded["input_ids"].clone(),
        "step_idx": torch.tensor(examples["step_idx"]),
        "is_final_step": torch.tensor(examples["is_final_step"]),
        "reward_tokens": reward_tokens
    }
```

With the preprocessing function defined, we can now create and process the dataset into a format that the model can train on. This step involves converting the raw dataset into a structured format using the Hugging Face `Dataset` class, which provides efficient data handling and processing capabilities.

``` python
# Create dataset
paired_dataset = Dataset.from_list(create_training_examples(raw_dataset))

# Process dataset
processed_dataset = paired_dataset.map(
    preprocess_function,
    batched=True,
    remove_columns=paired_dataset.column_names,
    desc="Processing dataset"
)
```

Now we'll configure the training arguments. These settings control various aspects of the training process, such as the number of epochs, batch size, learning rate, and logging. Proper configuration is essential to ensure that the model trains efficiently and effectively, especially when dealing with large models like Mistral.

``` python
training_args = TrainingArguments(
    output_dir="prm_output",
    num_train_epochs=3,
    per_device_train_batch_size=1,
    learning_rate=2e-6,
    logging_dir="logs",
    logging_steps=10,
    save_strategy="epoch",
    remove_unused_columns=False,
    gradient_accumulation_steps=4,  # Increased for Mistral's size
    fp16=True,  # Enable mixed precision training
    local_rank=-1
)
```

With the training arguments set, we can initialize the trainer and run the training process. The `ProcessRewardTrainer` class handles the training loop, including data loading, model updates, and logging. It also includes error handling to manage any issues that arise during training, such as memory constraints or unexpected errors.

``` python
# Initialize trainer
trainer = ProcessRewardTrainer(
    model=model,
    args=training_args,
    train_dataset=processed_dataset,
    tokenizer=tokenizer,
    data_collator=None
)

# Train with error handling
try:
    trainer.train()
    trainer.save_model("prm_output/final_model")
    tokenizer.save_pretrained("prm_output/final_model")
except Exception as e:
    print(f"Training failed with error: {str(e)}")
    import gc
    gc.collect()
    torch.cuda.empty_cache()
    raise e
```

Now let's take a look at the internals of the PRM training process. We have to handle the reward tokens and compute the loss a bit differently than a normal causal language model. The PRM training process uses a special token system to handle rewards, which involves converting "+" and "-" tokens to their corresponding vocabulary IDs. This conversion is crucial for the model to understand and predict the reward values associated with each step.

``` python
class RewardDataCollator:
    def __init__(self, tokenizer):
        # Convert "+" and "-" tokens to their vocabulary IDs
        self.pos_token_id = tokenizer.convert_tokens_to_ids("+")
        self.neg_token_id = tokenizer.convert_tokens_to_ids("-")
        self.reward_token_ids = [self.pos_token_id, self.neg_token_id]
```

Our system uses `+` and `-` as special tokens to represent positive and negative rewards respectively. These are converted to their corresponding token IDs in the model's vocabulary. The data collator is responsible for preparing batches of data during training, ensuring that the reward tokens are correctly aligned with the input sequences.

``` python
def __call__(self, features):
    batch = {}
    # Convert boolean labels to reward token IDs
    batch["reward_tokens"] = torch.tensor([
        self.pos_token_id if label == 1.0 else self.neg_token_id
        for label in [f["labels"] for f in features]
    ])

    # Track step position information
    batch["step_idx"] = torch.tensor([f["step_idx"] for f in features])
    batch["is_final_step"] = torch.tensor([f["is_final_step"] for f in features])
```

The loss computation process is handled in the `compute_loss` method of `ProcessRewardTrainer`. This method extracts metadata from the inputs, such as reward tokens and step indices, and calculates the loss based on the model's predictions. The loss function is designed to focus on the reward tokens, ensuring that the model learns to predict the correct reward values for each step.

``` python
def compute_loss(self, model, inputs, return_outputs=False, **kwargs):
    # Extract metadata
    reward_tokens = inputs.pop("reward_tokens")
    step_idx = inputs.pop("step_idx")
    is_final_step = inputs.pop("is_final_step")
```

Now we'll find the reward positions. The model uses placeholder tokens to mark where rewards should be predicted. The code finds these positions in the input sequence, allowing the model to focus on the relevant parts of the sequence when calculating the loss.

``` python
# Get model predictions
outputs = model(**inputs)
logits = outputs.logits

# Locate placeholder tokens
placeholder_positions = torch.where(
    inputs["input_ids"] == self.placeholder_token_id
)
batch_indices = placeholder_positions[0]  # Which items in batch
seq_indices = placeholder_positions[1]    # Position within sequence
```

The model uses placeholder tokens to mark where rewards should be predicted. The code finds these positions in the input sequence.

``` python
# Extract logits at placeholder positions
placeholder_logits = logits[batch_indices, seq_indices]

# Focus only on reward token logits (+/-)
reward_logits = placeholder_logits[:, self.reward_token_ids]
```

Now we'll get the target labels and convert them to binary format. This step involves mapping the reward tokens to binary labels, which are used to calculate the cross-entropy loss. The binary format simplifies the loss calculation, allowing the model to focus on predicting the correct reward values.

``` python
# Get target labels
labels = reward_tokens[batch_indices]

# Convert to binary format (0 for positive, 1 for negative)
binary_labels = (labels == self.reward_token_ids[0]).long()

# Calculate cross entropy loss
loss = self.loss_fn(reward_logits, binary_labels)
```

Finally, we'll optionally calculate accuracy for watching the loss in Weights & Biases. This step provides additional metrics for monitoring the training process, allowing us to track the model's performance and make adjustments as needed.

``` python
if return_outputs:
    with torch.no_grad():
        predictions = reward_logits.argmax(-1)
        acc = (predictions == binary_labels).float().mean()
    return loss, {
        "logits": logits,
        "loss": loss,
        "acc": acc,
        "step_idx": step_idx,
        "is_final_step": is_final_step,
    }
```

## Inference

Now we can setup and `infer.py` script to load the model and use it to score chains of thought. The inference process involves loading the trained model and tokenizer, preparing the input data, and evaluating each step in a reasoning chain. This allows us to assess the quality of the reasoning process and identify areas for improvement.

``` python
from transformers import AutoModelForCausalLM, AutoTokenizer
import torch

# Load your trained model and tokenizer
model = AutoModelForCausalLM.from_pretrained("output/final_model")
tokenizer = AutoTokenizer.from_pretrained("output/final_model")
model.eval()
```

The core of our inference system is the step probability function. This function evaluates the likelihood of a step being positive or negative, providing a quantitative measure of the step's contribution to the overall reasoning process. By calculating probabilities for each step, we can identify which steps are most effective and which may need revision.

``` python
def get_step_probability(input_text, accumulated_steps):
    # Combine input and accumulated steps
    full_text = input_text
    for i, step in enumerate(accumulated_steps, 1):
        full_text += f"\nStep {i}: {step}"

    # Prepare inputs
    inputs = tokenizer(
        full_text,
        return_tensors="pt",
        truncation=True,
        max_length=2048
    )

    with torch.no_grad():
        # Set up reward token masking
        reward_token_ids = torch.tensor([
            tokenizer.convert_tokens_to_ids("+"),
            tokenizer.convert_tokens_to_ids("-")
        ])

        # Create logits mask to only allow reward tokens
        logits_mask = torch.full((tokenizer.vocab_size,), float("-inf"))
        logits_mask[reward_token_ids] = 0

        # Get model outputs
        outputs = model(**inputs, temperature=0.7, do_sample=True, seed=42)
        logits = outputs.logits[:, -1, :] + logits_mask

        # Calculate probabilities
        probs = torch.softmax(logits, dim=-1)
        reward_probs = probs[0, reward_token_ids]

        return {
            "+": reward_probs[0].item(),
            "-": reward_probs[1].item()
        }
```

``` python
# Define your problem and steps
example = {
    "input": "Find all values of x in the interval [0, 2π] that satisfy sin(x) = 1/2.",
    "steps": [
        "To solve sin(x) = 1/2, we need to find the reference angle first.\nα = arcsin(1/2) = π/6",
        "Since sin(x) is positive in quadrants I and II, and we're looking in [0, 2π], we'll have two solutions:",
        "First solution: x = α = π/6 ≈ 0.524 radians",
        "Second solution: x = π - α = π - π/6 = 5π/6 ≈ 2.618 radians",
        "We can verify these solutions:\nsin(π/6) = sin(5π/6) = 1/2",
        "Therefore, the solutions in [0, 2π] are x = π/6, 5π/6.",
    ]
}

# Evaluate steps progressively
print(f"Question: {example['input']}\n")
accumulated_steps = []

for i, step in enumerate(example["steps"], 1):
    accumulated_steps.append(step)
    print(f"After Step {i}:")
    print("Accumulated steps:")
    for j, acc_step in enumerate(accumulated_steps, 1):
        print(f"Step {j}: {acc_step}")

    probs = get_step_probability(example["input"], accumulated_steps)
    print(f"\nProbabilities: + = {probs['+']:.2%}, - = {probs['-']:.2%}\n")
    print("-" * 80 + "\n")
```

The model evaluates each step in the context of all previous steps, providing probabilities for positive and negative rewards. A higher probability for "+" indicates that the model considers the step to be correct and helpful in the reasoning process. The evaluation is cumulative, meaning it considers how each new step builds upon previous ones.

The masking mechanism ensures that the model only predicts reward tokens ("+" or "-") by setting all other token probabilities to negative infinity. This forces the model to make a binary decision about the quality of each step. You can now use the PRM as either a reward model using trl's `RewardTrainer` or as a step-wise evaluator in your own custom training loops, or even at inference time to guide sampling.

---
date: "2025-06-23"
tags: ["ai"]
url: "https://www.stephendiehl.com/posts/fine_tuning_tools/"
---

# Fine-tuning With Tool Calling

With the release of `trl` 0.19.0, there is now [native support](https://huggingface.co/docs/trl/en/sft_trainer#tool-calling-with-sft) for fine-tuning models with custom tools, a capability that has been seamlessly integrated into the `Dataset` and `SFTTrainer` classes. This streamlines the training process of teaching a model how and when to call external functions. We can now structure our training data to include the full conversational flow of tool usage. From the initial user prompt, to the model's decision to call a tool, the resulting output from that tool, and the model's final, synthesized response.

An existing part of this workflow is the `get_json_schema` utility function, which elegantly bridges the gap between your Python code and the structured format that the model requires. This function automatically generates a compliant JSON schema directly from a Python function, but it relies on two key coding practices. First, the function must include modern Python type annotations for all its arguments and its return value. Second, the function's docstring must be written in the [Google docstring style](https://google.github.io/styleguide/pyguide.html#38-comments-and-docstrings). With `get_json_schema` we can parse the function to extract not only the names and data types of the parameters but also the descriptive text that explains what the function and each of its arguments do. So let's look at an example:

``` python
from transformers.utils import get_json_schema

def multiply(a: float, b: float):
    """
    A function that multiplies two numbers

    Args:
        a: The first number to multiply
        b: The second number to multiply
    """
    return a * b

schema = get_json_schema(multiply)
print(schema)
```

This gives the dataset [JSON schema](https://huggingface.co/docs/trl/en/dataset_formats#tool-calling) for the function.

``` json
{
  "type": "function",
  "function": {
    "name": "multiply",
    "description": "A function that multiplies two numbers",
    "parameters": {
      "type": "object",
      "properties": {
        "a": {
          "type": "number",
          "description": "The first number to multiply"
        },
        "b": {
          "type": "number",
          "description": "The second number to multiply"
        }
      },
      "required": ["a", "b"]
    }
  }
}
```

Now we can use this schema to create a dataset with tool calling. For instance we can wrap the `mpmath` library to create a dataset with tool calling related to polynomial roots and Bessel functions. We just need to add a `tools` column in an dataset with the tool schemas.

``` python
import mpmath
from datasets import Dataset
from transformers.utils import get_json_schema
from trl import SFTTrainer


def find_polynomial_roots(coefficients: list[float]) -> str:
    """
    Finds the roots of a polynomial given its coefficients.

    Args:
        coefficients: A list of coefficients [c_n, ..., c_1, c_0] for c_n*x^n + ... + c_0.

    Returns:
        A string representation of the list of roots.
    """
    roots = mpmath.polyroots(coefficients)
    return str(roots)


def calculate_bessel_j(order: float, value: float) -> float:
    """
    Calculates the Bessel function of the first kind, J_v(z).

    Args:
        order: The order 'v' of the Bessel function.
        value: The value 'z' at which to evaluate the function.

    Returns:
        The result of the Bessel function J_v(z).
    """
    return mpmath.besselj(order, value)


polyroots = get_json_schema(find_polynomial_roots)
bessel = get_json_schema(calculate_bessel_j)

dataset = Dataset.from_dict(
    {
        "messages": [
            [
                {
                    "role": "user",
                    "content": "Find the roots of the polynomial x^2 - 4.",
                },
                {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "type": "function",
                            "function": {
                                "name": "find_polynomial_roots",
                                "arguments": {"coefficients": [1, 0, -4]},
                            },
                        }
                    ],
                },
                {
                    "role": "tool",
                    "name": "find_polynomial_roots",
                    "content": "[-2.0, 2.0]",
                },
                {
                    "role": "assistant",
                    "content": "The roots of x^2 - 4 are -2.0 and 2.0.",
                },
            ],
            [
                {
                    "role": "user",
                    "content": "What is the value of the Bessel function J0(2.5)?",
                },
                {
                    "role": "assistant",
                    "tool_calls": [
                        {
                            "type": "function",
                            "function": {
                                "name": "calculate_bessel_j",
                                "arguments": {"order": 0, "value": 2.5},
                            },
                        }
                    ],
                },
                {
                    "role": "tool",
                    "name": "calculate_bessel_j",
                    "content": "0.0483837764022209",
                },
                {
                    "role": "assistant",
                    "content": "The value of J0(2.5) is approximately 0.04838.",
                },
            ],
        ],
        "tools": [
            [polyroots, bessel],
            [polyroots, bessel],
        ],
    }
)

trainer = SFTTrainer(
    model="Qwen3-0.6B",
    train_dataset=dataset,
)

trainer.train()
```

In my limited testing the `Qwen3-0.6B` and `Llama-3.1-70B-Instruct` are currently the best open weight models for fine-tuning for custom tool use. Your mileage may vary.

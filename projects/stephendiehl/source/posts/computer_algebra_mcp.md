---
date: "2025-05-18"
tags: ["ai", "math", "python"]
url: "https://www.stephendiehl.com/posts/computer_algebra_mcp/"
---

# Adventures in Symbolic Algebra with Model Context Protocol

I spent last weekend playing with this new [MCP protocol](https://modelcontextprotocol.io/introduction) all the kids are talking about, using it to make language models talk to symbolic computer algebra systems. The idea was simple: LLMs are great at understanding natural language math problems but terrible at actually solving them, while computer algebra systems excel at symbolic manipulation but have arcane interfaces. By connecting them through MCP, we can get the best of both worlds. The code is still early and rough around the edges, but I've put the source up on Github if anyone wants to experiment with it.

- [Github Source Code](https://github.com/sdiehl/sympy-mcp) (`git clone https://github.com/sdiehl/sympy-mcp.git`)

MCP, if you're not familiar, is Anthropic's answer to the question: "How do we get an LLM to actually DO things instead of just TALK about doing them?" It's essentially the [cgi-bin](https://en.wikipedia.org/wiki/Common_Gateway_Interface) of AI (I'm probably dating myself with that reference). It's a protocol that allows language models to call external tools through a server that exposes the functions as a REST API. The core idea of MCP is to standardize the way language models call external tools, instead of writing custom connectors for each AI model and each tool, you implement the protocol once on each side. It's the USB-C of AI tooling, if USB-C were still in its awkward adolescent phase.

Notably, the MCP server runs locally on your machine, letting the language model invoke arbitrary code and commands by using one of the LLM desktop clients to call out to the local server. This is probably a bad idea in general and as dangerous and reckless as you might think, so there's a definite security concern here ... be warned. But let's not let a potential rootkit get in the way of a fun weekend experiment.

My particular itch stemmed from watching Claude (and its cousins like o4-mini-high and DeepSeek-R1) really struggle with tensor calculus. If you've ever asked an LLM to perform complex symbolic manipulation, you know the drill: confident answers, beautiful LaTeX formatting, and results that would make your math professor weep bitter tears. These models, despite their impressive linguistic capabilities, are absolute disasters when it comes to keeping track of indices in tensor expressions or manipulating complicated algebraic forms. The expressions involved in even moderate general relativity problems are HUGE, with hundreds of terms and complex [index gymnastics](https://mathworld.wolfram.com/IndexGymnastics.html).

But we already have specialized tools that excel at this! Computer algebra systems like Mathematica, Sympy, xAct, Cadabra were built specifically for this purpose. So the obvious solution presented itself: let's expose these tools to the LLM through MCP and let each system do what it does best. The LLM handles the natural language understanding and planning, while the symbolic algebra system performs the actual mathematical manipulations with perfect precision.

Working with the MCP ecosystem is like visiting a frontier town in the Wild West. The documentation exists in the form of scattered campfire stories, the implementations have a distinctly "I wrote this at a 3 AM hackathon" vibe, and everything is strangely Node-heavy. This Node fixation likely stems from most MCP tools being designed to call REST services for cloud applications. Then there's the peculiar ecosystem of suspiciously self-referential products from companies that just happen to sell AI coding assistants. There's definitely a faint whiff of opportunism in the air.

Debugging an MCP server is a crazy exercise. You're essentially working with a stochastic black box that communicates through a complex web of JSON schemas attached to docstring annotations. When something goes wrong, good luck figuring out if it's your server, the client, the LLM's interpretation, or just the model having a laugh at your expense. The non-deterministic nature of the whole setup means that something can work perfectly five times in a row and then spontaneously fail on the sixth attempt because the random number seed on the inference server is different, the prompt vibes are off, or for reasons completely shrouded in the mists of the digital occult. Welcome to the fun future of software.

However, once you've made the ritual animal sacrifice to appease the non-determinism gods (I find that sacrificing a rubber duck while chanting *"O Great Transformer of the Deep, may thy hallucinations be few and thy completions true"* works best), you'll find the basic implementation is refreshingly straightforward, similar to FastAPI if FastAPI were designing its endpoints for a language model. Here's a simple example that highlights why this approach matters: Ask any LLM to factor a large integer, and watch it confidently fabricate entirely wrong answers. By design, transformers can't perform the arbitrary computation required for integer factorization. They've merely memorized some factorizations from the internet (and even those, poorly).

``` python
from mcp.server.fastmcp import FastMCP
import subprocess

mcp = FastMCP("Demo", instructions="You factor integers.")

def factor_number(number):
    result = subprocess.run(['factor', str(number)], capture_output=True, text=True)
    return result.stdout.strip()

@mcp.tool()
def factor(a: int) -> int:
    """Factor an integer"""
    return factor_number(a)
```

Throw that in `server.py` and then with [uv](https://docs.astral.sh/uv/), you can install the server and start it up with a single command.

``` bash
uv run --with mcp[cli] mcp install server.py
```

And bingo you've got your first MCP server. If you open up the [Claude Desktop app](https://claude.ai/download) you should see the server `Demo` in the tool dropdown and now you can ask Claude what you've always wanted "Give me the prime factors of 170141183460469231731687303715884105727 or else a kitten gets factored" and it will actually factor it for you (using the GNU `factor` command) instead of vibin' up some bullshit. Progress!

When it works, it's pretty magical. The successful integration of these components (properly formatted JSON, accurate tool selection by the language model, and reliable server processing) demonstrates the potential of this architectural approach. While the technology is still in its early stages of development, the possibilities for integration with formal verification systems are particularly compelling. The combination of LLMs with [theorem provers like Lean](https://www.youtube.com/watch?v=zZr54G7ec7A) could enable natural language interfaces to formal mathematics, allowing automated reasoning systems to benefit from the language models' ability to understand informal mathematical concepts while maintaining rigorous logical foundations. Similarly, making computer algebra systems accessible through natural language could dramatically lower the barrier to entry for complex mathematical computation. However, significant research challenges remain in ensuring reliable and semantically correct interactions between these different systems.

Let me show you how this looks in practice with a classic example from physics: solving the damped harmonic oscillator with a forcing term. This is the bread and butter problem for undergraduate physics students. Here's how the workflow unfolds with our `sympy-mcp` server:

**User**: Solve the damped harmonic oscillator with forcing term: the mass-spring-damper system described by the differential equation where m is mass, c is the damping coefficient, k is the spring constant, and F(t) is an external force.

$$m\frac{d^2x}{dt^2} + c\frac{dx}{dt} + kx = F(t)$$

**Assistant**: (Internal tool call) `intro_many` to introduce variables with appropriate assumptions:

``` json
{
  "variables": [
    {"var_name": "t", "pos_assumptions": ["real"], "neg_assumptions": []},
    {"var_name": "m", "pos_assumptions": ["real", "positive"], "neg_assumptions": []},
    {"var_name": "c", "pos_assumptions": ["real", "positive"], "neg_assumptions": []},
    {"var_name": "k", "pos_assumptions": ["real", "positive"], "neg_assumptions": []}
  ]
}
```

**Assistant**: (Internal tool call) `introduce_function` to define x(t) as a function:

``` json
{
  "func_name": "x"
}
```

**Assistant**: (Internal tool call) `introduce_expression` to create the differential equation:

``` json
{
  "expr_str": "Eq(m*Derivative(x(t), t, t) + c*Derivative(x(t), t) + k*x(t), F(t))"
}
```

**Assistant**: (Internal tool call) `introduce_function` to define the forcing function F(t):

``` json
{
  "func_name": "F"
}
```

**Assistant**: (Internal tool call) `dsolve_ode` to solve the ODE:

``` json
{
  "expr_key": "expr_12",
  "func_name": "x"
}
```

And voilà! Instead of hallucinating a solution, the LLM delegates to SymPy which returns the correct solution:

$$x{\left(t \right)} = C_{1} e^{\frac{t \left(- c + \sqrt{c^{2} - 4 k m}\right)}{2 m}} + C_{2} e^{- \frac{t \left(c + \sqrt{c^{2} - 4 k m}\right)}{2 m}} + \frac{e^{\frac{t \left(- c + \sqrt{c^{2} - 4 k m}\right)}{2 m}} \int F{\left(t \right)} e^{\frac{c t}{2 m}} e^{- \frac{t \sqrt{c^{2} - 4 k m}}{2 m}}, dt}{\sqrt{c^{2} - 4 k m}} - \frac{e^{- \frac{t \left(c + \sqrt{c^{2} - 4 k m}\right)}{2 m}} \int F{\left(t \right)} e^{\frac{c t}{2 m}} e^{\frac{t \sqrt{c^{2} - 4 k m}}{2 m}}, dt}{\sqrt{c^{2} - 4 k m}}$$

No hallucinated terms, no mysterious constants appearing out of nowhere, just the correct solution. The LLM handles the natural language interaction and orchestration, while the computer algebra system does what it does best ... exact symbolic manipulation.

Anyways all the code is up on [Github here](https://github.com/sdiehl/sympy-mcp) so maybe someone else will find it useful. If you have Cursor or Claude installed add the following to `~/.cursor/mcp.json` or `~/Library/Application Support/Claude/claude_desktop_config.json` to install the MCP server.

``` json
{
  "mcpServers": {
    "sympy-mcp": {
      "command": "uv",
      "args": [
        "run",
        "--with",
        "https://github.com/sdiehl/sympy-mcp/releases/download/0.1/sympy_mcp-0.1.0-py3-none-any.whl",
        "python",
        "server.py"
      ]
    }
  }
}
```

Or maybe slightly better, run it from a Docker image.

``` json
{
  "mcpServers": {
    "sympy-mcp": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "-p",
        "8081:8081",
        "--rm",
        "ghcr.io/sdiehl/sympy-mcp:latest"
      ]
    }
  }
}
```

And maybe read the source code for what you're installing here, because I feel like a lot of people are going to install these MCP servers without knowing what they're doing and thus install a lot of malware and exploits. There's basically no security going on here. [And that could be a big problem](https://elenacross7.medium.com/%EF%B8%8F-the-s-in-mcp-stands-for-security-91407b33ed6b). At this rate of security negligence, Skynet won't arise from AI research, but from someone accidentally giving a model root access to national defense systems because they curl'd some random MCP server from the internet.

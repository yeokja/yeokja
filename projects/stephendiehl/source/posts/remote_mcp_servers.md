---
date: "2025-05-26"
tags: ["ai", "python"]
url: "https://www.stephendiehl.com/posts/remote_mcp_servers/"
---

# Remote MCP Servers

After my last adventure with [symbolic algebra and MCP](computer_algebra_mcp.md), I found myself that nagging voice in the back of my head whispering "What if Claude decides today is the day to go postal and `rm -rf /`?" The solution, as with most modern software problems, was obvious: make it someone else's problem.

Enter [Vercel](https://vercel.com/docs/functions), purveyor of the finest VC-subsidized free compute known to humanity. But hey, if they want to let me run arbitrary Python code on their infrastructure for the low, low price of absolutely nothing, who am I to question their financial decisions? So the question is, can we host our MCP servers as edge functions on their Hobby Tier without paying anything? And the answer is yes, yes we can.

Instead of giving the language model root access to my laptop (because what could possibly go wrong there?), I'm now giving it the ability to execute arbitrary code on Vercel's servers on a sandboxed serverless environment that resets after every request. It's like the difference between letting a toddler play with matches in your living room versus letting them play with matches in someone else's swimming pool. Second, and perhaps more importantly, every time Claude calls one of my MCP tools, I'm burning through some venture capitalist's money. So two birds, one stone.

I've built a little adapter that takes any FastMCP server and wraps it in a Vercel Function compatible runtime. The whole thing deploys with a single `git push` to the repo and starts in about 4 seconds. Trying to get the default FastMCP server to work was a bit of a pain because it has quite a bit of stateful session handling logic, but with a bit of internal mangling of it's reflection API we can introspect its tool definitions and schemas and build a simple HTTP API around them that exposes them as edge functions that run on the Vercel Python edge runtime fairly simply and efficiently. After all is said and done here's what the code looks like:

``` python
import datetime
from fastmcp import FastMCP
from .mcp_adapter import build_app

mcp: FastMCP = FastMCP("Vercel MCP Server", stateless_http=True, json_response=True)

@mcp.tool()
def echo(message: str) -> str:
    """Echo the provided message back to the user"""
    return f"Tool echo: {message}"


@mcp.tool()
def get_time() -> str:
    """Get the current server time"""
    current_time = datetime.datetime.now().isoformat()
    return f"Current Vercel server time: {current_time}"


@mcp.tool()
def add_numbers(a: int, b: int) -> int:
    """Add two numbers together"""
    return a + b

app = build_app(mcp)
```

Push that to GitHub, connect it to Vercel, and boom: you've instantly got a serverless MCP server running. You can limit the runtime of the tool calls by setting the `maxDuration` in the `vercel.json` file, so can limit runtime to 2 seconds or something.

The server includes a self-hosted installer and [bridge script](https://github.com/sdiehl/mcp-on-vercel/blob/main/bridge.py) (that proxies remote MCP servers to local MCP servers, via RPC to stdio) that can be deployed with a single shell command. Instead of manually editing configuration files, users can run `install` with `uv` and the installer automatically configures Claude Desktop or Cursor to use the remote server. The installer downloads the bridge script directly from the server itself, eliminating the need for local files or manual configuration steps. About as close to one-click remote server install as you can get.

```
uv run https://your-domain.vercel.app/install.py
```

The dumb example everyone gives is always chaining together a bunch of REST API calls to external services. With this running on a remote function you can do the same thing, like if you wanted to use `requests` to call out to the National Weather Service API to get the current weather forecast for a location. Or you could install PyTorch in `requirements.txt` and run a neural network in there if you really wanted to. The free tier gives you 100 GB-Hours and 100,000 invocations per month to work with. If you configure a function to use 1GB of memory and it runs for 5 second, that's 5 GB-s per invocation. So you could make around 72,000 five-second calls before hitting the memory limit or invocation limit, whichever comes first.

``` python
import requests

NWS_API_BASE = "https://api.weather.gov"

@mcp.tool()
def get_weather(latitude: float, longitude: float) -> str:
    """
    Get the current weather forecast for a location using the National Weather Service API
    """
    point_data = requests.get(
        f"https://api.weather.gov/points/{latitude},{longitude}",
        headers={"Accept": "application/json"},
    ).json()

    forecast_url = point_data["properties"]["forecast"]

    forecast_data = requests.get(
        forecast_url,
        headers={"Accept": "application/json"},
    ).json()

    current_period = forecast_data["properties"]["periods"][0]

    return f"Forecast for {current_period['name']}: {current_period['detailedForecast']}"
```

Once you have it setup either use a LLM client (Claude Desktop, Cursor, Cline, 5ire, VS Code, etc) or use the [Cloudflare MCP Playground](https://playground.ai.cloudflare.com/). If you're using Claude Max, Team or Enterprise you don't need to use the bridge script at all and can just use the URL of the remote MCP server directly in the `Custom Integrations` in the [integrations](https://www.anthropic.com/news/integrations) section of your Claude app settings. Otherwise, under the hood the installer uses the Bridge shim to proxy the remote MCP server to a local MCP server.

``` json
{
  "mcpServers": {
    "example": {
      "command": "/opt/homebrew/bin/uv",
      "args": [
        "run",
        "--with",
        "mcp>=1.0.0",
        "https://your-domain.vercel.app/bridge.py",
        "https://your-domain.vercel.app"
      ],
      "env": {
        "MCP_API_KEY": "your-secret-key-here"
      }
    }
  }
}
```

Beyond dedicated MCP clients, you can also integrate remote MCP servers directly into your applications using client libraries. The MCP tool in the OpenAI Responses API, for instance, allows developers to give the model access to tools hosted on Remote MCP servers. These are MCP servers maintained by developers and organizations across the internet that expose these tools to MCP clients, like the Responses API.

Calling a remote MCP server with the Responses API is straightforward. For example, here's how you can use your own MCP server using the weather tool above. For more details, see the [OpenAI Platform documentation](https://platform.openai.com/docs/guides/tools-remote-mcp).

``` python
from openai import OpenAI

client = OpenAI()

resp = client.responses.create(
    model="gpt-4.1",
    tools=[
        {
            "type": "mcp",
            "server_label": "example",
            "server_url": "https://your-domain.vercel.app",
            "require_approval": "never",
        },
    ],
    input="What is the current weather in Berlin?"
)

print(resp.output_text)
```

Similarly, you can use remote MCP servers with Anthropic's Claude API.

``` python
from anthropic import Anthropic

anthropic = Anthropic()

response = anthropic.beta.messages.create(
    model="claude-3-7-sonnet-20250219",
    max_tokens=1000,
    messages=[
        {
            "role": "user",
            "content": 'What is the current weather in Berlin?',
        },
    ],
    mcp_servers=[
        {
            "type": "url",
            "url": "https://your-domain.vercel.app",
            "name": "example",
            "tool_configuration": {
                "enabled": True,
            },
        }
    ],
    extra_headers={
        "anthropic-beta": "mcp-client-2025-04-04",
    },
)
print(response.content)
```

Or using Gemini's MCP client.

``` python
import os
import asyncio
from datetime import datetime
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from google import genai

client = genai.Client(api_key="YOUR_GEMINI_API_KEY")

# Create server parameters for stdio connection
server_params = StdioServerParameters(
    command="uv",
    args=[
        "run",
        "--with",
        "mcp>=1.0.0",
        "https://your-domain.vercel.app/bridge.py",
        "https://your-domain.vercel.app",
    ],
)

async def run():
    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            prompt = f"What is the weather in London?"
            await session.initialize()
            response = await client.aio.models.generate_content(
                model="gemini-2.0-flash",
                contents=prompt,
                config=genai.types.GenerateContentConfig(
                    temperature=0,
                    tools=[session],
                ),
            )
            print(response.text)

asyncio.run(run())
```

The security model of this whole endeavor remains, shall we say, "optimistic." MCP servers are essentially giving language models the ability to execute arbitrary code based on natural language instructions. This approach requires trusting a [single `install.py` file](https://github.com/sdiehl/mcp-on-vercel/blob/main/api/install.py) served from the remote endpoint, which admittedly isn't ideal. However, it's arguably not much worse than the common practice of piping installation scripts from `install.sh` (like uv) directly into shell execution. Additionally, there's nothing preventing a malicious remote MCP server from sending back prompt injections that could instruct your agent to ignore previous instructions and exfiltrate sensitive local data to a North Korean server.

And if you're handing API tokens over to Claude, you're essentially giving it the keys to the kingdom - despite any careful instructions about what the token is meant for, it's not hard to produce a sufficiently convincing prompt that could persuade the model to use that token for anything it's authorized to do. The attack vector is as simple as sweet-talking an overeager LLM into doing whatever the attacker wants with the tools you've given it.

But hey, if you're already experimenting with MCP servers that execute arbitrary code based on natural language prompts interpreted by what is essentially the internet's collective fever dream, you've likely already made peace with a certain level of operational risk which could best be described as "YOLO, hold my beer". So if you're running running a nuclear reactor, maybe don't touch this stuff, mkay?

For anything approaching production use of MCP (god help us all), you'd want proper OAuth flows, request signing, rate limiting. Which you could totally add to this project, but I'm not going to do that here because this is a proof of concept. The documentation for that is [here](https://modelcontextprotocol.io/specification/draft/basic/authorization).

It's also worth nothing that I'm not exactly endorsing Vercel's business model. They lure you in with their generous free tier, let you build your entire infrastructure around their platform, and then gently suggest that maybe you'd like to pay \$20 per month for features that would cost you 2 cents on EC2. The first hit is always free, but you end up paying for it down the line. There's also no reason you couldn't also do the same thing on AWS Lambda or Google Cloud Functions, but the setup isn't as easy (or free).

There's something beautifully absurd, almost tragically bureaucratic, about the whole grotesque Rube Goldberg-esque setup we have here. We've created a system where natural language gets translated into JSON RPC requests, which get routed through edge networks to serverless functions across the globe, which spin up Linux hypervisors to isolate the runtime, which execute Python code that dispatches enormous attention matrix multiplications on 5nm etched silicon GPUs performing billions of floating point operations per second, which return results that get packaged into JSON responses that get translated back into natural language just to add two integers, which is otherwise a single CPU instruction. It's like using a symphony orchestra, three circus troupes, six carrier pigeons, and a nuclear reactor to butter a single piece of toast.

But it works, and in our current technological moment, "it works" is often the highest praise we can give to any system. It's pretty cool that you can go from a simple Python script to a fully-fledged MCP server deployed automatically from a GitHub repository, with zero ongoing maintenance or costs and immeditetly plug that into the frontier models in a single click. I'm not sure what the future holds for this kind of thing. I'm sure there are a lot of interesting things that can be done with it, but I'm also sure that there are a lot of interesting things that can be done with it that are probably not good.

Of course, no discussion of MCP would be complete without acknowledging the chorus of hustle bros who have descended upon LinkedIn like locusts, proclaiming that MCP is the future of everything and will revolutionize how we think about the fundamental nature of existence. At least ten different visionary thought leaders have nearly broken down in tears proselytizing about it in the last week. Some are even going so far as to call it the new [Web3](https://www.anildash.com//2025/05/20/mcp-web20-20/), because that went great last time.

Welp, whatever. The code is up on [GitHub](https://github.com/sdiehl/mcp-on-vercel) if you want to experiment with your own serverless MCP misadventures.

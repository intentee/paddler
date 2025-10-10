# Paddler

Digital products and their users need privacy, reliability, cost control, and an option to be independent from closed-source model providers.

Paddler is an open-source LLM load balancer and serving platform. It allows you to run inference, deploy, and scale LLMs on your own infrastructure, providing a great developer experience along the way.

## Key features

<img align="right" src="https://github.com/user-attachments/assets/19e74262-1918-4b1d-9b4c-bcb4f0ab79f5">

* Inference through a built-in [llama.cpp](https://github.com/ggml-org/llama.cpp) engine
* LLM-specific load balancing
* Works through agents that can be added dynamically, allowing integration with autoscaling tools
* Request buffering, enabling scaling from zero hosts
* Dynamic model swapping
* Built-in web admin panel for management, monitoring, and testing
* Observability metrics

## Who is Paddler for?

* Product teams that need LLM inference and embeddings in their features
* DevOps/LLMOps teams that need to run and deploy LLMs at scale
* Organizations handling sensitive data with high compliance and privacy requirements (medical, financial, etc.)
* Organizations wanting to achieve predictable LLM costs instead of being exposed to per-token pricing
* Product leaders who need reliable model performance to maintain a consistent user experience of their AI-based features

## Installation and Quickstart

Paddler is self-contained in a single binary file, so all you need to do to start using it is obtain the `paddler` binary and make it available in your system.

You can obtain the binary by:

* Option 1: Downloading the latest release from our [GitHub releases](https://github.com/intentee/paddler/releases)
* Option 2: Or building Paddler from source

### Using Paddler

Once you have made the binary available in your system, you can start using Paddler. The entire Paddler functionality is available through the `paddler` command (running `paddler --help` will list all available commands).

There are only two deployable components, the `balancer` (which distributes the incoming requests), and the `agent` (which generates tokens and embeddings through slots).

To start the balancer, run:

```sh
paddler balancer --inference-addr 127.0.0.1:8061 --management-addr 127.0.0.1:8060 --web-admin-panel-addr 127.0.0.1:8062
```
The `--web-admin-panel-addr` flag is optional, but it will allow you to view your setup in a web browser.

And to start an agent with, for example, 4 slots, run:

```sh
paddler agent --management-addr 127.0.0.1:8060 --slots 4
```

Read more about the [installation](https://paddler.intentee.com/docs/introduction/installation/) and [setting up a basic cluster](http://127.0.0.1:8050/docs/starting-out/set-up-a-basic-llm-cluster/). 

## Documentation

Visit our [documentation page](https://paddler.intentee.com/docs/introduction/what-is-paddler/) to install Paddler and get started with it. 

[API documentation](https://paddler.intentee.com/api/introduction/using-paddler-api/) is also available.

[Video overview](https://www.youtube.com/watch?v=aT6QCL8lk08)

## Community and contributions

For questions or community conversations, use GitHub discussions or join our [Discord server](https://discord.gg/92x3Z8a4gj). All contributions are welcome.

## How does it work?

Paddler is built for an easy setup. It comes as a self-contained binary with only two deployable components, the `balancer` and the `agents`. 

The `balancer` exposes the following:

- Inference service (used by applications that connect to it to obtain tokens or embeddings)
- Management service, which manages the Paddler's setup internally
- Web admin panel that lets you view and test your Paddler setup

`Agents` are usually deployed on separate instances. They further distribute the incoming requests to `slots`, which are responsible for generating tokens and embeddings.

Paddler uses a built-in llama.cpp engine for inference, but has its own implementation of llama.cpp slots, which keep their own context and KV cache.

### Web admin panel

Paddler comes with a built-in web admin panel. 

You can use it to monitor your Paddler fleet:
<img width="1587" height="732" alt="paddler-web-admin-panel" src="https://github.com/user-attachments/assets/de26312e-e83e-4def-8326-0aa5d559396c" />

Add and update your model and customize the chat template and inference parameters:
<img width="1422" height="1584" alt="paddler-model" src="https://github.com/user-attachments/assets/dd9d7eb0-a990-4b1c-b523-7286956baeb2" />

And use a GUI to test the inference:
<img width="1413" height="984" alt="paddler-prompt" src="https://github.com/user-attachments/assets/30b35b5a-c3de-4acc-a602-c7ffaa21d0a6" />


## Starting out
* [Setup a basic LLM cluster](https://paddler.intentee.com/docs/starting-out/set-up-a-basic-llm-cluster/)
* [Use Paddler's web admin panel](https://paddler.intentee.com/docs/starting-out/using-web-admin-panel/)
* [Generate tokens and embeddings](https://paddler.intentee.com/docs/starting-out/generating-tokens-and-embeddings/)
* [Use function calling](https://paddler.intentee.com/docs/starting-out/using-function-calling/)
* [Create a multi agent fleet](https://paddler.intentee.com/docs/starting-out/multi-agent-fleet/)
* [Go beyond a single device](https://paddler.intentee.com/docs/starting-out/going-beyond-a-single-device/)

## Why the Name

We initially wanted to use [Raft](https://raft.github.io/) consensus algorithm (thus Paddler, because it paddles on a Raft), but eventually dropped that idea. The name stayed, though.

Later, people started sending us the "that's a paddlin'" clip from The Simpsons, and we just embraced it.

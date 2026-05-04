# Paddler

<img alt="Paddler 3.0 supports multimodal!" src="https://github.com/user-attachments/assets/cc3b4a1d-4ab3-4a57-8cf0-14e682cf1492" width="100%" />

Digital products and their users need privacy, reliability, cost control, and an option to be independent from closed-source model providers.

Paddler is an open-source LLM load balancer and serving platform. It allows you to run inference, deploy, and scale LLMs on your own infrastructure, providing a great developer experience along the way.

## Key features

<img align="right" alt="Paddler logo" src="https://github.com/user-attachments/assets/19e74262-1918-4b1d-9b4c-bcb4f0ab79f5">

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

## Community

- Discord https://discord.gg/92x3Z8a4gj
- Reddit (just started a subreddit, we will see how it goes :)) https://www.reddit.com/r/paddler/

## Installation and Quickstart

Paddler is self-contained in a single binary file, so all you need to do to start using it is obtain the `paddler` binary and make it available in your system.

You can obtain the binary by:

* Option 1: Downloading the latest release from our [GitHub releases](https://github.com/intentee/paddler/releases)
* Option 2: Or building Paddler from source (MSRV is *1.88.0*)

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

Read more about the [installation](https://paddler.intentee.com/docs/introduction/installation/) and [setting up a basic cluster](https://paddler.intentee.com/docs/starting-out/set-up-a-basic-llm-cluster/). 

## Documentation and resources

- Visit our [documentation page](https://paddler.intentee.com/docs/introduction/what-is-paddler/) to install Paddler and get started with it. 
- [API documentation](https://paddler.intentee.com/api/introduction/using-paddler-api/) is also available.
- [Video overview](https://www.youtube.com/watch?v=aT6QCL8lk08)
- [FOSEDM 2026 talk](https://fosdem.org/2026/schedule/event/PD8WGF-from_infrastructure_to_production_a_year_of_self-hosted_llms/) - From Infrastructure to Production: A Year of Self-Hosted LLMs.

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
<img width="100%" alt="Dashboard section of the Web Admin Panel" src="https://github.com/user-attachments/assets/1f17e7ee-861a-4831-8018-3f75f6827d2b" />

Add and update your model and customize the chat template and inference parameters:
<img width="100%" alt="Model section of the Web Admin Panel" src="https://github.com/user-attachments/assets/53247c83-3ead-4a8e-8f0e-c2db15ae41ba" />

And use a GUI to test the inference:
<img width="100%" alt="Prompt section of the Web Admin Panel" src="https://github.com/user-attachments/assets/c02fcfb8-53e0-45d4-97f4-5ff85d600bd7" />

### Desktop application (beta)

Paddler comes in two versions: a command-line interface for infrastructure use, and a desktop application for more casual use cases, like using multiple laptops and PCs in a local AI cluster or setting up an office-wide company second brain, without using a console. 

You can also mix both; for example, you can set up a Paddler balancer on your server rack, and ask a colleague in the office with an RTX 5090 to plug in ad hoc as an agent if they do not need their entire compute.

The world is your oyster with this one. :) 

See the [desktop app docs](https://paddler.intentee.com/docs/desktop-app/introduction/) to get started.

<img width="100%" alt="Home screen of Paddler's destkop application" src="https://github.com/user-attachments/assets/1e70dc24-e831-4c9f-85a7-6638fe56dee4" />

<img width="100%" alt="Start a cluster screen in Paddler's desktop application" src="https://github.com/user-attachments/assets/d65d1deb-fc61-481e-bcfd-b968120c07b1" />


## Starting out
* [Setup a basic LLM cluster](https://paddler.intentee.com/docs/starting-out/set-up-a-basic-llm-cluster/)
* [Use Paddler's web admin panel](https://paddler.intentee.com/docs/starting-out/using-web-admin-panel/)
* [Generate tokens and embeddings](https://paddler.intentee.com/docs/starting-out/generating-tokens-and-embeddings/)
* [Use function calling](https://paddler.intentee.com/docs/starting-out/using-function-calling/)
* [Use grammars](https://paddler.intentee.com/docs/starting-out/using-grammars/)
* [Use multimodal models](https://paddler.intentee.com/docs/starting-out/using-multimodal-models/)
* [Create a multi agent fleet](https://paddler.intentee.com/docs/starting-out/multi-agent-fleet/)
* [Go beyond a single device](https://paddler.intentee.com/docs/starting-out/going-beyond-a-single-device/)

## Do you accept AI-Generated code?

All code in the project is human-reviewed, and most is handcrafted. We have been experimenting with using AI to generate some code, and so far, we had success with:
- coding and maintaining the HTTP client that connects to the core library
- [creating an integration test harness for Paddler, where we were able to consolidate all the existing tests to use the new, improved harness almost automatically](https://github.com/intentee/paddler/pull/220)

If you successfully generate something, you can submit it. We will still need to review it, so make sure you understand what you are doing.

You can try, though. :) We have even added [CLAUDE.md](CLAUDE.md) with some code style and other basic instructions.

## Why the Name

We initially wanted to use [Raft](https://raft.github.io/) consensus algorithm (thus Paddler, because it paddles on a Raft), but eventually dropped that idea. The name stayed, though.

Later, people started sending us the "that's a paddlin'" clip from The Simpsons, and we just embraced it.

## Thanks

[<img width="100%" alt="Thank you llama.cpp" src="https://github.com/user-attachments/assets/ba12e3a3-f78f-40a6-aa1b-36b3924ac0ec" />](https://github.com/ggml-org/llama.cpp)

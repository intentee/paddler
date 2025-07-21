# Paddler

Paddler is an open-source, production-ready, stateful load balancer and reverse proxy designed to optimize servers running [llama.cpp](https://github.com/ggerganov/llama.cpp).

## Why Paddler

<img align="right" src="https://github.com/user-attachments/assets/19e74262-1918-4b1d-9b4c-bcb4f0ab79f5">

Typical load balancing strategies like round robin and least connections are ineffective for llama.cpp servers, which utilize continuous batching algorithms and allow to configure slots to handle multiple requests concurrently. 

Paddler is designed to support llama.cpp-specific features like slots. It works by maintaining a stateful load balancer aware of each server's available slots, ensuring efficient request distribution.

> [!NOTE]
> In simple terms, the `slots` in llama.cpp refer to predefined memory slices within the server that handle individual requests. When a request comes in, it is assigned to an available slot for processing. They are predictable and highly configurable.
>
> You can learn more about them in [llama.cpp server](https://github.com/ggerganov/llama.cpp/tree/master/examples/server) documentation.

## Key features
* Uses agents to monitor the slots of individual llama.cpp instances.
* Supports the dynamic addition or removal of llama.cpp servers, enabling integration with autoscaling tools.
* Buffers requests, allowing to scale from zero hosts.
* Integrates with StatsD protocol but also comes with a built-in dashboard.
* AWS integration.

![paddler-animation](https://github.com/user-attachments/assets/2a0f3837-7b0a-4249-b385-46ebc7c38065)
*Paddler's aware of each server's available slots, ensuring efficient request ("R") distribution*

## How it Works

llama.cpp instances need to be registered in Paddler. Paddler’s agents should be installed alongside llama.cpp instances so that they can report their slots status to the load balancer.

The sequence repeats for each agent:

```mermaid
sequenceDiagram
    participant loadbalancer as Paddler Load Balancer
    participant agent as Paddler Agent
    participant llamacpp as llama.cpp

    agent->>llamacpp: Hey, are you alive?
    llamacpp-->>agent: Yes, this is my slots status
    agent-->>loadbalancer: llama.cpp is still working
    loadbalancer->>llamacpp: I have a request for you to handle
```

## Usage

### Quick Start with Docker Compose

For a quick demonstration of Paddler, see the Docker Compose example in the `example/` directory.

### Installation

Download the latest release for Linux, Mac, or Windows from the 
[releases page](https://github.com/distantmagic/paddler/releases).

On Linux, if you want Paddler to be accessible system-wide, rename the downloaded executable to `/usr/bin/paddler` (or `/usr/local/bin/paddler`).

### Running llama.cpp

Slots endpoint is required to be enabled in llama.cpp. To do so, run llama.cpp with the `--slots` flag. 

### Running Agents

The next step is to run Paddler’s agents. Agents register your llama.cpp instances in Paddler and monitor the slots of llama.cpp instances. 
They should be installed on the same host as your server that runs [llama.cpp](https://github.com/ggerganov/llama.cpp).

An agent needs a few pieces of information:
1. `external-llamacpp-addr` tells how the load balancer can connect to the llama.cpp instance
2. `local-llamacpp-addr` tells how the agent can connect to the llama.cpp instance
3. `management-addr` tell where the agent should report the slots status

Run the following to start a Paddler’s agent (replace the hosts and ports with your own server addresses when deploying):

```shell
./paddler agent \
    --external-llamacpp-addr 127.0.0.1:8088 \
    --local-llamacpp-addr 127.0.0.1:8088 \
    --management-addr 127.0.0.1:8085
```

#### Naming the Agents

With the `--name` flag, you can assign each agent a custom name. This name will be displayed in the management dashboard and not used for any other purpose. 

#### API Key

If your llama.cpp instance requires an API key, you can provide it with the `--local-llamacpp-api-key` flag.

### Running Load Balancer

Load balancer collects data from agents and exposes reverse proxy to the outside world.

It requires two sets of flags:
1. `management-addr` tells where the load balancer should listen for updates from agents
2. `reverseproxy-addr` tells how load balancer can be reached from the outside hosts

To start the load balancer, run:
```shell
./paddler balancer \
    --management-addr 127.0.0.1:8085 \
    --reverseproxy-addr 196.168.2.10:8080
```

`management-host` and `management-port` in agents should be the same as in the load balancer.

#### Enabling Dashboard

You can enable dashboard to see the status of the agents with 
`--management-dashboard-enable` flag. If enabled, it is available at the 
management server address under `/dashboard` path.

#### Enabling Slots Endpoint

> [!NOTE]
> Available since v1.0.0

By default, Paddler blocks access to `/slots` endpoint, even if it is enabled in `llama.cpp`, because it exposes a lot of sensistive information about the server, and should only be used internally. If you want to expose it anyway, you can use the `--slots-endpoint-enable` flag.

#### Rewriting the `Host` Header
.
> [!NOTE]
> Available since v0.8.0

In some cases (see: [#20](https://github.com/distantmagic/paddler/issues/20)), you might want to rewrite the `Host` header.

In such cases, you can use the `--rewrite-host-header` flag. If used, Paddler will use the `external` host provided by agents instead of the balancer host when forwarding the requests.

## Feature Highlights

### Aggregated Health Status

Paddler balancer endpoint aggregates the slots of all `llama.cpp` instances and reports the total number of available and processing slots.

Aggregated health status is available at the `/api/v1/agents` endpoint of the management server.

![Aggregated Health Status](https://github.com/distantmagic/paddler/assets/1286785/01f2fb39-ccc5-4bfa-896f-919b66318b2c)

### Buffered Requests (Scaling from Zero Hosts)

> [!NOTE]
> Available since v0.3.0

Load balancer's buffered requests allow your infrastructure to scale from zero hosts by providing an additional metric (unhandled requests). 

It also gives your infrastructure some additional time to add additional hosts. For example, if your autoscaler is setting up an additional server, putting an incoming request on hold for 60 seconds might give it a chance to be handled even though there might be no available llama.cpp instances at the moment of issuing it.

Scaling from zero hosts is especially suitable for low-traffic projects because it allows you to cut costs on your infrastructure—you won't be paying your cloud provider anything if you are not using your service at the moment.

![Paddler Buffered Requests](https://github.com/distantmagic/paddler/assets/1286785/a1754d46-d728-4858-a991-11e8b52bd20d)

https://github.com/distantmagic/paddler/assets/1286785/34b93e4c-0746-4eed-8be3-cd698e15cbf9

### State Dashboard

Although Paddler integrates with the [StatsD protocol](https://github.com/statsd/statsd), you can preview the cluster's state using a built-in dashboard.

#### Web Dashboard

Paddler needs to be compiled with the `web_dashboard` feature flag enabled (enabled by default in GitHub releases).

To start the dashboard, run `paddler balancer` with the `--management-dashboard-enable` flag.

![Paddler Web Dashboard](https://github.com/user-attachments/assets/b12413ca-481b-4d49-9908-5dc38346305a)

#### TUI Dashobard

> [!NOTE]
> Available since v1.2.0

You can connect to any running Paddler instance with `paddler dashboard --management-addr [HOST]:[PORT]`.

![Paddler TUI Dashboard](https://github.com/user-attachments/assets/55dc8cfe-4b82-4619-871f-0bb79cb44d01)

Thank you [@Propfend](https://github.com/Propfend) for [contributing](https://github.com/distantmagic/paddler/pull/31) the TUI Dashboard!

### StatsD Metrics

> [!NOTE]
> Available since v0.3.0

> [!TIP]
> If you keep your stack self-hosted you can use [Prometheus](https://prometheus.io/) with StatsD exporter to handle the incoming metrics.

> [!TIP]
> This feature works with [AWS CloudWatch Agent](https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch-Agent-custom-metrics-statsd.html) as well.

Paddler supports the following StatsD metrics:
- `requests_buffered` number of buffered requests since the last report (resets after each report)
- `slots_idle` total idle slots
- `slots_processing` total slots processing requests

All of them use `gauge` internally.

StatsD metrics need to be enabled with the following flags:

```shell
./paddler balancer \
    # .. put all the other flags here ...
    --statsd-addr=127.0.0.1:8125
```

If you do not provide the `--statsd-addr` flag, the StatsD metrics will not be collected.

## Tutorials

- [Installing llama.cpp on AWS EC2 CUDA Instance](https://llmops-handbook.distantmagic.com/deployments/llama.cpp/aws-ec2-cuda/index.html)
- [Installing llama.cpp with AWS EC2 Image Builder](https://llmops-handbook.distantmagic.com/deployments/llama.cpp/aws-image-builder/index.html)

## Changelog

### v2.0.0 (in progress)

#### Breaking Changes

- Change the agent object structure (which in turn changes the response from the API)
- Rename the `agents` endpoint from `/agents` to `/api/v1/agents`
- Adjusted to use milliseconds instead of seconds in services field:
  
    - `--monitoring-interval` in `agents`
    - `--buffered-request_timeout` and `--statsd-reporting-interval` in `balancer`

#### Features

- Add `--management-cors-allowed-host` repeatable flag to be able to specify the allowed CORS hosts for the management API
- Add `/api/v1/agents/stream` endpoint that streams the updates from the agents in real-time

### v1.2.0

#### Features

- Add TUI dashboard (`paddler dashboard --management-addr [HOST]:[PORT]`) to be able to easily observe balancer instances from the terminal level

### v1.1.0

- More meaningful error messages when the agent can't connect to the llama.cpp slot endpoint, or when slot endpoint is not enabled in llama.cpp
- Set default logging level to `info` for agents and balancer to increase the amount of information in the logs (it wasn't clean if the agent was running or not)
- Enable LTO optimization for the release builds (see [#28](https://github.com/distantmagic/paddler/issues/28))

### v1.0.0

The first stable release! Paddler is now rewritten in Rust and uses the [Pingora](https://github.com/cloudflare/pingora) framework for the networking stack. A few minor API changes and reporting improvements are introduced (documented in the README). API and configuration are now stable, and won't be changed until version `2.0.0`.

This is a stability/quality release. The next plan is to introduce a supervisor who does not just monitor llama.cpp instances, but to also manage them.

Requires llama.cpp version [b4027](https://github.com/ggerganov/llama.cpp/releases/tag/b4027) or above.

### v0.10.0

This update is a minor release to make Paddler compatible with `/slots` endpoint changes introduced in llama.cpp b4027.

Requires llama.cpp version [b4027](https://github.com/ggerganov/llama.cpp/releases/tag/b4027) or above.

### v0.9.0

Latest supported llama.cpp release: [b4026](https://github.com/ggerganov/llama.cpp/releases/tag/b4026)

#### Features

- Add `--local-llamacpp-api-key` flag to balancer to support llama.cpp API keys (see: [#23](https://github.com/distantmagic/paddler/issues/23))

### v0.8.0

#### Features

- Add `--rewrite-host-header` flag to balancer to rewrite the `Host` header in forwarded requests (see: [#20](https://github.com/distantmagic/paddler/issues/20))

### v0.7.1

#### Fixes

- Incorrect preemptive counting of remaining slots in some scenarios

### v0.7.0

Requires at least [b3606](https://github.com/ggerganov/llama.cpp/releases/tag/b3606) llama.cpp release.

#### Breaking Changes

- Adjusted to handle breaking changes in llama.cpp `/health` endpoint: https://github.com/ggerganov/llama.cpp/pull/9056
  
    Instead of using the `/health` endpoint to monitor slot statuses, starting from this version, Paddler uses the `/slots` endpoint to monitor llama.cpp instances.
    Paddler's `/health` endpoint remains unchanged.

### v0.6.0

Latest supported llama.cpp release: [b3604](https://github.com/ggerganov/llama.cpp/releases/tag/b3604)

#### Features

- [Name agents with `--name` flag](https://github.com/distantmagic/paddler/issues/15)

### v0.5.0

#### Fixes

- Management server crashed in some scenarios due to concurrency issues

### v0.4.0

Thank you, [@ScottMcNaught](https://github.com/ScottMcNaught), for the help with debugging the issues! :)

#### Fixes

- OpenAI compatible endpoint is now properly balanced (`/v1/chat/completions`)
- Balancer's reverse proxy `panic`ked in some scenarios when the underlying `llama.cpp` instance was abruptly closed during the generation of completion tokens
- Added mutex in the targets collection for better internal slots data integrity

### v0.3.0

#### Features

* Requests can queue when all llama.cpp instances are busy
* AWS Metadata support for agent local IP address
* StatsD metrics support

### v0.1.0

#### Features

* [Aggregated Health Status Responses](https://github.com/distantmagic/paddler/releases/tag/v0.1.0)

## Why the Name

I initially wanted to use [Raft](https://raft.github.io/) consensus algorithm (thus Paddler, because it paddles on a Raft), but eventually, I dropped that idea. The name stayed, though.

Later, people started sending me a "that's a paddlin'" clip from The Simpsons, and I just embraced it.

## Community

Discord: https://discord.gg/kysUzFqSCK

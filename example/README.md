# Paddler Docker Compose Example

This example demonstrates how to set up a complete Paddler cluster using Docker Compose.

## What's Included

This example sets up:

* **Paddler load balancer** - Distributes requests across the agents with web admin panel enabled
* **Two Paddler agents** - Each responsible for inference tasks

## How to Run

1. **Navigate to the example directory:**

   ```bash
   cd example/
   ```

2. **Start the cluster:**

   ```bash
   docker compose up
   ```

3. **Access the services:**
   * **Management dashboard:** <http://127.0.0.1:8062/dashboard>

4. **Stop the cluster:**

   ```bash
   docker compose down
   ```

   To remove all data including cached models:

   ```bash
   docker compose down -v
   ```

## Web admin panel and networking

In production and other infrastructure deployments, Paddler runs normally in Docker with no special networking: agents connect to the balancer over the cluster network, and your applications consume the inference and OpenAI-compatible APIs directly.

The **web admin panel** is the exception. It is a browser UI served by the balancer — once the page loads, your browser calls the management server (`:8060`) and the inference server (`:8061`) directly, at the addresses the balancer was started with. For your browser to reach them, this example uses Docker **host networking** (`network_mode: host`), which binds all three servers on your host's loopback so the browser can reach them at `127.0.0.1`. **This works on Linux.** On Docker Desktop (macOS/Windows) containers run inside a Linux VM, so host networking does not expose these ports to your host browser and the panel will not be reachable.

To use the web admin panel locally you therefore need either:

* **Docker with host networking** — this example, on Linux, or
* the **`paddler` binary run directly** on your machine — see the [quickstart](../README.md) (`paddler balancer --inference-addr 127.0.0.1:8061 --management-addr 127.0.0.1:8060 --web-admin-panel-addr 127.0.0.1:8062`).

## GPU Support

To use GPU acceleration with CUDA, follow these steps:

### 1. Install NVIDIA Container Toolkit

Install the NVIDIA Container Toolkit on your host system by following the official installation guide:

<https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html>

### 2. Verify GPU Usage

After starting the containers, you can verify GPU usage with:

```bash
nvidia-smi
```

You should see the Docker containers listed in the GPU processes.

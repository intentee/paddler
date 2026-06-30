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

## Configuration

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

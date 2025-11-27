# Start from official CUDA 13 image
FROM docker.io/nvidia/cuda:13.0.1-cudnn-devel-ubuntu24.04

# Install base utils
RUN apt-get update && apt-get install -y \
    cmake \
    curl \
    libclang-dev

# Install nodejs
RUN apt-get update && apt-get install -y \
    nodejs \
    npm

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --no-modify-path && \
    ln -s /usr/local/cargo/bin/* /usr/local/bin/

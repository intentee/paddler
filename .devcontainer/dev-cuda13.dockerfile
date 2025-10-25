FROM node:trixie-slim

# Install common utils
RUN apt-get update && apt-get install -y \
    wget \
    curl \
    git \
    cmake \
    build-essential \
    libssl-dev \
    pkg-config \
    libclang-dev

# Install CUDA 13
RUN wget https://developer.download.nvidia.com/compute/cuda/repos/debian12/x86_64/cuda-keyring_1.1-1_all.deb \
    && dpkg -i cuda-keyring_1.1-1_all.deb \
    && apt-get update \
    && apt-get -y install \
        cuda-toolkit-13-0 \
        nvidia-open

ENV PATH=/usr/local/cuda/bin:$PATH

# Install Rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --no-modify-path && \
    ln -s /usr/local/cargo/bin/* /usr/local/bin/

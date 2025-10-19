FROM node:latest

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && apt-get install -y \
    curl \
    git \
    cmake \
    build-essential \
    libssl-dev \
    pkg-config \
    clang \
    libclang-dev


RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --no-modify-path && \
    ln -s /usr/local/cargo/bin/* /usr/local/bin/

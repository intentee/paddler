FROM node:latest

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && apt-get install -y \
    curl \
    git \
    cmake \
    build-essential \
    g++ \
    clang \
    libssl-dev \
    pkg-config

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --no-modify-path && \
    ln -s /usr/local/cargo/bin/* /usr/local/bin/

WORKDIR /app

RUN git clone https://github.com/intentee/paddler.git .

RUN make

RUN mv target/release/paddler /usr/local/bin/paddler

ENTRYPOINT ["paddler"]

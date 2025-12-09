FROM rust:slim-bookworm AS builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/tgin

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY ./src ./src

RUN touch src/main.rs

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash tginuser
USER tginuser
WORKDIR /home/tginuser

COPY --from=builder /usr/src/tgin/target/release/tgin /usr/local/bin/tgin


CMD ["tgin", "-f", "/etc/tgin/config.ron"]
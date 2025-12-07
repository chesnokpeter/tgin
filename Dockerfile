FROM rust:1.81-slim-bookworm as builder

WORKDIR /usr/src/tgin

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

COPY . .

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

EXPOSE 3000

CMD ["tgin", "-c", "/etc/tgin/config.ron"]
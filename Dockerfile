FROM lukemathwalker/cargo-chef:latest-rust-1.74.1 as chef
WORKDIR /app

FROM chef as planner
COPY . .

RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin evm-block-extractor-server

FROM debian:bookworm-slim AS runtime

WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && update-ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/evm-block-extractor-server /app/evm-block-extractor-server

EXPOSE 8080

ENTRYPOINT ["./evm-block-extractor-server"]

CMD ["-d", "testnet"]

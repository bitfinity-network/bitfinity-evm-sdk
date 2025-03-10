#
# This dockerfile has to be called from the root folder of the project
# > docker build -f src/evm-block-extractor/Dockerfile -t evm-block-extractor .
#
FROM rust:slim-bookworm AS builder
WORKDIR /app

ADD . .

RUN cargo build --release --bin evm-block-extractor

FROM ubuntu:22.04 AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates \
    && update-ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/evm-block-extractor /app/evm-block-extractor

EXPOSE 8080

# Set stop signal to ctrl+c
STOPSIGNAL SIGINT

ENTRYPOINT ["./evm-block-extractor"]


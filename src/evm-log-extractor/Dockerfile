#
# This dockerfile has to be called from the root folder of the project
# > docker build -f src/evm-log-extractor/Dockerfile -t evm-log-extractor .
#
FROM rust:slim-bookworm as builder
WORKDIR /app

ADD . .

RUN cargo build --release --bin evm-log-extractor

FROM ubuntu:22.04 AS runtime

WORKDIR /app

# RUN apt-get update -y \
#     && apt-get install -y --no-install-recommends ca-certificates \
#     && update-ca-certificates \
#     # Clean up
#     && apt-get autoremove -y \
#     && apt-get clean -y \
#     && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/evm-log-extractor /app/evm-log-extractor

ENV LOGGER_FILTER=info
ENV EVMC_PRINCIPAL=
ENV IDENTITY=/data/config/identity.pem
ENV EVMC_NETWORK_URL=https://icp0.io
ENV LOGS_DIR=/data/logs

# Set stop signal to ctrl+c
STOPSIGNAL SIGINT

CMD exec ./evm-log-extractor --evmc-principal=${EVMC_PRINCIPAL} --identity=${IDENTITY} --logs-directory=${LOGS_DIR} --evmc-network-url=${EVMC_NETWORK_URL} --logger-filter=${LOGGER_FILTER}


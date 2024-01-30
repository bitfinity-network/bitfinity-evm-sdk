FROM lukemathwalker/cargo-chef:latest-rust-1.75 as chef
WORKDIR /app

FROM chef as planner
COPY ../../ .

RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin evm-block-extractor-server

FROM debian:bookworm-slim AS runtime

# GCP BigQuery Environment Variables
ARG DATASET
ARG PROJECT_ID
ARG GCP_BLOCK_EXTRACTOR_SA_KEY

# Set environment variables
ENV DATASET=$DATASET \
    PROJECT_ID=$PROJECT_ID \
    GCP_BLOCK_EXTRACTOR_SA_KEY=$GCP_BLOCK_EXTRACTOR_SA_KEY

WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates \
    && update-ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/evm-block-extractor-server /app/evm-block-extractor-server


# Copy the entrypoint script into the container
COPY src/evm-block-extractor/scripts/entrypoint.sh /app/entrypoint.sh

# Make the entrypoint script executable
RUN chmod +x /app/entrypoint.sh

# Set the entrypoint script as the default way to start the container
ENTRYPOINT ["/app/entrypoint.sh"]

EXPOSE 8080



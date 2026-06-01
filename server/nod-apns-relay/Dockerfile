ARG RUST_VERSION=1.94
FROM rust:${RUST_VERSION}-slim AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /app /secrets \
    && chown -R 10001:10001 /app /secrets

WORKDIR /app
COPY --from=build /app/target/release/nod-apns-relay /usr/local/bin/nod-apns-relay

ENV NOD_APNS_RELAY_BIND=0.0.0.0:8768 \
    RUST_LOG=nod_apns_relay=info,tower_http=info

EXPOSE 8768
USER 10001:10001
CMD ["nod-apns-relay"]

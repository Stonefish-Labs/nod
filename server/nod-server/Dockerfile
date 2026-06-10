# Built from the repo ROOT context (the server is a member of the root Cargo
# workspace, so the build needs the workspace manifest, lockfile, nod-proto, and
# the sibling member manifests). See docker-compose.yml (context: ../..).
ARG RUST_VERSION=1.94
FROM rust:${RUST_VERSION}-slim AS build
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Whole workspace (trimmed by .dockerignore). Only nod-server and its deps
# (including nod-proto) are compiled; the client crates are resolved, not built.
COPY . .
RUN cargo build --release --locked -p nod-server

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /app /data \
    && chown -R 10001:10001 /app /data

WORKDIR /app
COPY --from=build /app/target/release/nod-server /usr/local/bin/nod-server

ENV NOD_BIND=0.0.0.0:8767 \
    NOD_DATABASE_URL=sqlite:///data/nod.sqlite \
    NOD_DATA_DIR=/data \
    RUST_LOG=nod_server=info,tower_http=info

EXPOSE 8767
VOLUME ["/data"]
USER 10001:10001
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD curl -fsS http://127.0.0.1:8767/health || exit 1
CMD ["nod-server"]

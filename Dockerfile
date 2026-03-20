FROM oven/bun:latest AS frontend

WORKDIR /app

COPY frontend/package.json frontend/bun.lock /app/

RUN bun install

COPY --exclude=dist --exclude=node_modules frontend /app

RUN --mount=type=secret,id=SENTRY_ORG,env=SENTRY_ORG \
    --mount=type=secret,id=SENTRY_PROJECT,env=SENTRY_PROJECT \
    --mount=type=secret,id=SENTRY_AUTH_TOKEN,env=SENTRY_AUTH_TOKEN \
    --mount=type=secret,id=SENTRY_URL,env=SENTRY_URL \
    bun run build

FROM rust:1-trixie AS build

WORKDIR /app

# Cache dependency compilation by building a stub first.
COPY Cargo.toml Cargo.lock /app/
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/cbzViewer target/release/deps/cbzViewer*

COPY src /app/src
RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update && apt-get -y install ca-certificates && \
    sed -i.bak -r 's@http://deb\.debian\.org/?@https://ftp.riken.jp/Linux/debian/@g' /etc/apt/sources.list.d/debian.sources && \
    apt-get update
RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    ca-certificates \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY config.docker.json /app/config.json
COPY --from=build /app/target/release/cbzViewer /app/
COPY --from=frontend /app/dist /app/dist

VOLUME /books
EXPOSE 8080

ENTRYPOINT ["/app/cbzViewer"]

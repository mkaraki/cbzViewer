FROM rust:1-trixie AS lepton_jpeg_build

RUN apt-get update && apt-get -y install ca-certificates && \
    sed -i.bak -r 's@http://deb\.debian\.org/?@https://ftp.riken.jp/Linux/debian/@g' /etc/apt/sources.list.d/debian.sources && \
    apt-get update
RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y git

RUN git clone --depth 1 --branch v0.5.3 https://github.com/microsoft/lepton_jpeg_rust.git /lepton_jpeg_rust
WORKDIR /lepton_jpeg_rust

RUN cargo build --release --workspace --locked

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

FROM golang:1.25-trixie AS build

RUN apt-get update && apt-get -y install ca-certificates && \
    sed -i.bak -r 's@http://deb\.debian\.org/?@https://ftp.riken.jp/Linux/debian/@g' /etc/apt/sources.list.d/debian.sources && \
    apt-get update
RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    libvips-dev

WORKDIR /app
COPY --from=lepton_jpeg_build /lepton_jpeg_rust/target/release/liblepton_jpeg_dll.so /app/liblepton_jpeg.so

COPY go.mod go.sum /app/
RUN go mod download

COPY *.go /app/
COPY lepton_jpeg /app/lepton_jpeg

RUN go build -ldflags '-linkmode external -extldflags=-L=.'

FROM debian:trixie-slim

RUN apt-get update && apt-get -y install ca-certificates && \
    sed -i.bak -r 's@http://deb\.debian\.org/?@https://ftp.riken.jp/Linux/debian/@g' /etc/apt/sources.list.d/debian.sources && \
    apt-get update
RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    libvips42t64 \
    ca-certificates \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN sed -i '/disable ghostscript format types/,+6d' /etc/ImageMagick-7/policy.xml
    
COPY --from=lepton_jpeg_build /lepton_jpeg_rust/target/release/liblepton_jpeg_dll.so /usr/lib/liblepton_jpeg.so
RUN ldconfig

WORKDIR /app
COPY config.docker.json /app/config.json
COPY --from=build /app/cbzViewer /app/
COPY --from=frontend /app/dist /app/dist

VOLUME /books
EXPOSE 8080

ENTRYPOINT ["/app/cbzViewer"]


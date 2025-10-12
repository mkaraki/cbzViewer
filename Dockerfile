FROM rust:1-trixie AS lepton_jpeg_build

RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y git

RUN git clone --depth 1 https://github.com/microsoft/lepton_jpeg_rust.git /lepton_jpeg_rust
WORKDIR /lepton_jpeg_rust

RUN cargo build --release --workspace --locked

FROM golang:1.25-trixie AS build

RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    libvips-dev

WORKDIR /app
COPY --from=lepton_jpeg_build /lepton_jpeg_rust/target/release/liblepton_jpeg.so /app/
COPY . /app/

RUN go build -ldflags '-linkmode external -extldflags=-L=.'

FROM debian:trixie-slim

RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    libvips42t64 \
    ca-certificates \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN sed -i '/disable ghostscript format types/,+6d' /etc/ImageMagick-7/policy.xml
    
COPY --from=lepton_jpeg_build /lepton_jpeg_rust/target/release/liblepton_jpeg.so /usr/lib/
RUN ldconfig

WORKDIR /app
COPY templates /app/templates
COPY config.docker.json /app/config.json
COPY --from=build /app/cbzViewer /app/

VOLUME /books
EXPOSE 8080

ENTRYPOINT ["/app/cbzViewer"]

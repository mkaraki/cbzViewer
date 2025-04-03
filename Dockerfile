FROM rust:1-bookworm AS lepton_jpeg_build

RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y git

WORKDIR /
RUN git clone --depth 1 https://github.com/microsoft/lepton_jpeg_rust.git
WORKDIR lepton_jpeg_rust

RUN cargo build --release

FROM golang:1.22-bookworm AS build

RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    libmagickwand-dev

WORKDIR /app
COPY --from=lepton_jpeg_build /lepton_jpeg_rust/target/release/liblepton_jpeg.so /app/
COPY . /app/

RUN go build -ldflags '-linkmode external -extldflags=-L=.'

FROM debian:bookworm

RUN apt-get update -o Acquire::CompressionTypes::Order::=gz && \
    apt-get install -y \
    libmagickwand-6.q16-6 \
    && apt-get clean && \
    rm -rf /var/lib/apt/lists/*

RUN sed -i '/disable ghostscript format types/,+6d' /etc/ImageMagick-6/policy.xml

COPY --from=lepton_jpeg_build /lepton_jpeg_rust/target/release/liblepton_jpeg.so /usr/lib/
RUN ldconfig

WORKDIR /app
COPY templates /app/templates
COPY config.docker.json /app/config.json
COPY --from=build /app/cbzViewer /app/

VOLUME /books
EXPOSE 8080

ENTRYPOINT ["/app/cbzViewer"]

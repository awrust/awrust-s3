FROM rust:1.93-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /build
COPY . .

RUN cargo build --release -p awrust-s3-server

FROM alpine:3

RUN addgroup -S awrust && adduser -S awrust -G awrust
RUN mkdir -p /data && chown awrust:awrust /data

COPY --from=builder /build/target/release/awrust-s3-server /usr/local/bin/awrust-s3-server

USER awrust

ENV AWRUST_S3_LISTEN_ADDR=[::]:4566
ENV AWRUST_S3_STORE=memory
ENV AWRUST_S3_DATA_DIR=/data
ENV AWRUST_LOG=info

EXPOSE 4566

ENTRYPOINT ["awrust-s3-server"]

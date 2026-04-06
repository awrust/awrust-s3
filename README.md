# awrust-s3

[![CI](https://github.com/awrust/awrust-s3/actions/workflows/ci.yml/badge.svg)](https://github.com/awrust/awrust-s3/actions/workflows/ci.yml)
[![Release](https://github.com/awrust/awrust-s3/actions/workflows/release.yml/badge.svg)](https://github.com/awrust/awrust-s3/actions/workflows/release.yml)

Minimal, deterministic S3-compatible emulator written in Rust. Designed for local development, integration tests, and CI pipelines.

Status: **v0.2 complete.**

## Quick start

```bash
docker run --rm -p 4566:4566 ghcr.io/awrust/awrust-s3:0.x
```

Or from source:

```bash
cargo run -p awrust-s3-server
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `AWRUST_S3_LISTEN_ADDR` | `0.0.0.0:4566` | Listen address |
| `AWRUST_S3_STORE` | `memory` | Storage backend (`memory` or `fs`) |
| `AWRUST_S3_DATA_DIR` | `/data` | Data directory when `store=fs` |
| `AWRUST_LOG` | `info` | Log level |

## Supported operations

- Bucket CRUD (create, head, delete, list all)
- Object CRUD (put, get, head, delete)
- ListObjectsV2 with prefix filtering and pagination
- Multipart upload (initiate, upload part, complete, abort)
- Range requests (byte ranges on GET)
- Content-Type, ETag, Last-Modified, x-amz-meta-* headers
- x-amz-request-id on every response

## Usage with AWS CLI

```bash
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 mb s3://my-bucket
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 cp file.txt s3://my-bucket/file.txt
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 cp large.bin s3://my-bucket/large.bin  # multipart
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 ls s3://my-bucket/
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 ls
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 rm s3://my-bucket/file.txt
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 rb s3://my-bucket
```

## Testing

### Rust unit tests

```bash
cargo test --workspace
```

### Linting

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

### BDD integration tests (Behave + boto3)

```bash
pip install -r tests/integration/requirements.txt
cd tests/integration
behave                          # memory backend via cargo run
STORE=fs behave                 # filesystem backend via cargo run
IMAGE=awrust-s3:test behave     # memory backend via Docker
IMAGE=awrust-s3:test STORE=fs behave  # filesystem backend via Docker
```

## Architecture Decisions

See [docs/adr](docs/adr) for architectural decisions and rationale.

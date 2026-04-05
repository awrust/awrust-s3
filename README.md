# awrust-s3

Minimal, deterministic S3-compatible emulator written in Rust. Designed for local development, integration tests, and CI pipelines.

Status: v0 in progress.

## Running

```bash
cargo run -p awrust-s3-server
```

The server listens on `0.0.0.0:4566` by default. Override with `AWRUST_S3_LISTEN_ADDR`:

```bash
AWRUST_S3_LISTEN_ADDR=127.0.0.1:9000 cargo run -p awrust-s3-server
```

### Usage with AWS CLI

```bash
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 mb s3://my-bucket
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 cp file.txt s3://my-bucket/file.txt
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 ls s3://my-bucket/
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
behave
```

The integration suite automatically builds and starts the server on a random port, runs all scenarios, and tears down cleanly.

## Architecture Decisions

See [docs/adr](docs/adr) for architectural decisions and rationale.

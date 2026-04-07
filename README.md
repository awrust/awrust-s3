# awrust-s3

[![CI](https://github.com/awrust/awrust-s3/actions/workflows/ci.yml/badge.svg)](https://github.com/awrust/awrust-s3/actions/workflows/ci.yml)
[![Release](https://github.com/awrust/awrust-s3/actions/workflows/release.yml/badge.svg)](https://github.com/awrust/awrust-s3/actions/workflows/release.yml)

Minimal, deterministic S3-compatible emulator written in Rust. Designed for local development, integration tests, and CI pipelines.

Status: **v0.3 complete.**

## Quick start

```bash
docker run --rm -p 4566:4566 ghcr.io/awrust/awrust-s3:0.x
```

Or from source:

```bash
cargo run -p awrust-s3-server
```

Then:

```bash
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 mb s3://my-bucket
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 cp file.txt s3://my-bucket/
aws --endpoint-url=http://localhost:4566 --no-sign-request s3 ls s3://my-bucket/
```

## Documentation

| Document | Description |
|----------|-------------|
| [Usage Guide](docs/USAGE.md) | Configuration, supported operations, examples |
| [Architecture](docs/ARCHITECTURE.md) | Design decisions, internals, storage model |
| [ADRs](docs/adr) | Architecture Decision Records |

## Development

```bash
cargo fmt --all --check
cargo clippy -- -D warnings
cargo test --workspace
```

BDD integration tests:

```bash
cd tests/integration && behave
```

See [AGENTS.md](AGENTS.md) for the full development workflow.

## License

MIT

# awrust-s3 — Project Instructions

## Build & Test

```bash
cargo fmt --all --check
cargo clippy -- -D warnings
cargo test --workspace
```

### BDD integration tests

```bash
cd tests/integration
behave                                    # memory backend
STORE=fs behave                           # filesystem backend
IMAGE=awrust-s3:test behave              # Docker + memory
IMAGE=awrust-s3:test STORE=fs behave     # Docker + filesystem
```

## Development workflow

### TDD is mandatory
1. Write failing tests first (BDD scenarios + Rust unit tests)
2. Run tests — confirm they fail (RED)
3. Implement the production code
4. Run tests — confirm they pass (GREEN)
5. Run `cargo fmt --all && cargo clippy -- -D warnings` before committing

### New dependencies require an ADR
Any new crate dependency must have an Architecture Decision Record in `docs/adr/` before merging. Follow the existing format (ADR-0001 through ADR-0007).

### Pull requests
When opening a PR, follow the template in `.github/pull_request_template.md`. Commit messages use the format:

```
S3-XXX Concise summary explaining the "what" (#PR)

Context on "why" and "how". Each line limited to 80 characters.
```

## Code principles

- No comments; code is truth
- No backwards compatibility hacks; only move forward
- No storing what can be computed
- No duplication; extract when patterns emerge
- Expose what must be exposed, hide what must be hidden

## Project structure

```
crates/
  awrust-s3-domain/    # Store trait, MemoryStore, FsStore (no HTTP)
  awrust-s3-server/    # Axum server, handlers, XML, error mapping
tests/
  integration/         # Behave BDD tests with boto3
docker/
  Dockerfile           # Multi-stage alpine build
docs/
  adr/                 # Architecture Decision Records
```

## S3 compatibility

- Path-style (`/<bucket>/<key>`) and virtual-host (`bucket.host/<key>`) addressing
- No auth validation (accept all requests)
- ETags: MD5 for single-part, composite MD5 for multipart
- XML responses via quick-xml with serde
- All responses include `x-amz-request-id` header

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AWRUST_S3_LISTEN_ADDR` | `0.0.0.0:4566` | Listen address |
| `AWRUST_S3_STORE` | `memory` | `memory` or `fs` |
| `AWRUST_S3_DATA_DIR` | `/data` | Data dir for fs backend |
| `AWRUST_S3_BASE_DOMAIN` | `localhost` | Base domain for virtual-host addressing |
| `AWRUST_LOG` | `info` | Log level |

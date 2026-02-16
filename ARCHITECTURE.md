# awrust-s3 — Architecture (v0)

**Status:** Draft (v0)
**Project:** awrust-s3
**License:** MIT
**Primary distribution:** Docker image (Linux amd64/arm64)
**Scope:** Minimal, deterministic S3-compatible emulator for local development and CI.

---

## 1. Goals

### 1.1 Product goals

* Provide a **fast**, **deterministic**, **Docker-first** S3 emulator suitable for:

  * local development
  * integration tests
  * CI pipelines
* Be **simple to run**:

  * single container / single binary
  * minimal configuration
* Be **explicitly scoped**: only the operations documented here are supported in v0.

### 1.2 Engineering goals

* Keep the implementation **testable** and **modular** so it can later be bundled into an `awrust` “stack” container.
* Prefer correctness of **supported behavior** over breadth.
* Keep maintenance cost low: avoid plugin systems, custom DSLs, and complex persistence layers in v0.

---

## 2. Non-goals

* Full AWS S3 parity (features, edge cases, security model, performance profile).
* Production object storage.
* IAM policy simulation.
* Perfect SigV4 verification.
* Multi-region semantics.
* Event notifications (S3 → SNS/SQS/EventBridge) in v0.

---

## 3. User experience

### 3.1 How users run it

Docker-first:

```bash
docker run --rm -p 4566:4566 ghcr.io/awrust/awrust-s3:0.x
```

### 3.2 Endpoint style

**v0 supports path-style addressing only**:

```
http://localhost:4566/<bucket>/<key>
```

Rationale:

* Minimizes DNS/host-header complexity in Docker and CI.
* Keeps routing simple and predictable.

Virtual-host style may be added later.

### 3.3 Configuration (environment variables)

* `AWRUST_S3_LISTEN_ADDR` (default: `0.0.0.0:4566`)
* `AWRUST_S3_STORE` (default: `memory`) — `memory` | `fs`
* `AWRUST_S3_DATA_DIR` (default: `/data`) — used when `store=fs`
* `AWRUST_LOG` (default: `info`) — logging level

---

## 4. Supported API surface (v0)

### 4.1 Buckets

* `PUT /<bucket>` — Create bucket
* `HEAD /<bucket>` — Check bucket exists
* `DELETE /<bucket>` — Delete bucket (only if empty)
* `GET /` — List buckets (optional; minimal ListAllMyBucketsResult)

### 4.2 Objects

* `PUT /<bucket>/<key>` — Put object (single-part)
* `GET /<bucket>/<key>` — Get object
* `HEAD /<bucket>/<key>` — Object metadata
* `DELETE /<bucket>/<key>` — Delete object
* `GET /<bucket>?list-type=2&prefix=...&continuation-token=...` — List objects v2 (minimal)

### 4.3 Headers and metadata (minimal)

Supported:

* `Content-Type`
* `Content-Length`
* `ETag` (MD5 of payload for single-part uploads)
* `Last-Modified`
* `x-amz-meta-*`

Not supported:

* Server-side encryption
* ACLs
* Tagging
* Object versioning
* Multipart upload (v0)

### 4.4 Errors

* `NoSuchBucket`
* `NoSuchKey`
* `BucketNotEmpty`
* Idempotent create bucket behavior (default)

---

## 5. High-level architecture

### 5.1 Components

* **HTTP server** (Tokio runtime)
* **Router** (method + path + query matching)
* **Service layer** (S3 semantics)
* **Storage backend** (pluggable)
* **XML serializer** (minimal S3-compatible responses)

### 5.2 Data flow

1. Request arrives.
2. Router parses path-style bucket/key and query params.
3. Handler constructs a command.
4. Service executes against Store.
5. Response assembled with headers + XML body.
6. Logging and tracing recorded.

---

## 6. Key design decisions

### 6.1 Pluggable storage

Define a Store trait:

```rust
trait Store {
    create_bucket(name)
    delete_bucket(name)
    bucket_exists(name)
    put_object(bucket, key, bytes, metadata)
    get_object(bucket, key)
    head_object(bucket, key)
    delete_object(bucket, key)
    list_objects_v2(bucket, prefix, continuation, max_keys)
    list_buckets()
}
```

Implementations:

* `MemoryStore`
* `FsStore`

### 6.2 Determinism

* Lexicographic ordering for bucket and object listings.
* Injectable clock for timestamps in tests.
* Stable request ID per response.

### 6.3 Authentication

v0 accepts requests regardless of auth headers.
No SigV4 validation is performed.

### 6.4 Single-part only

Multipart upload deferred to later versions.

---

## 7. Storage design

### 7.1 In-memory

* `HashMap<BucketName, BucketState>`
* `HashMap<(BucketName, Key), ObjectRecord>`

ObjectRecord:

* bytes
* metadata
* etag
* last_modified

### 7.2 Filesystem

```
/data/
  buckets/
    <bucket>/
      objects/
        <encoded-key>
      meta/
        <encoded-key>.json
```

Keys must be safely encoded for filesystem storage.

---

## 8. Routing

### 8.1 Path-style routing

* `/health` → health check
* `/`:

  * `GET` list buckets
* `/<bucket>`:

  * `PUT` create
  * `HEAD` exists
  * `DELETE` delete
  * `GET` list objects v2
* `/<bucket>/<key>`:

  * `PUT` object
  * `GET` object
  * `HEAD` object
  * `DELETE` object

### 8.2 Health endpoint

```
GET /health → 200 OK
{"status":"ok"}
```

---

## 9. Observability

### 9.1 Logging

Use `tracing`:

* method
* path
* status
* latency
* request id
* store backend

### 9.2 Metrics

Hooks reserved for future Prometheus support.

---

## 10. Testing strategy

### 10.1 Unit tests

* Store implementations
* Error mapping
* XML response snapshots
* Deterministic ordering

### 10.2 Integration tests

Run SDK-level tests:

* create bucket
* put object
* get object
* head object
* list objects
* delete object
* delete bucket

Must work with:

* AWS CLI (`--endpoint-url`)
* At least one official AWS SDK

---

## 11. Repository structure

```
awrust-s3/
  README.md
  ARCHITECTURE.md
  COMPAT.md
  LICENSE
  Cargo.toml
  crates/
    awrust-s3-core/
    awrust-s3-server/
  docker/
    Dockerfile
  tests/
    integration/
```

---

## 12. Distribution

* GHCR:

  * `ghcr.io/awrust/awrust-s3:<version>`
* Multi-arch:

  * linux/amd64
  * linux/arm64
* Semantic versioning (0.x until stable)

---

## 13. Open questions

* Include ListAllMyBuckets in v0?
* Idempotent bucket creation vs strict AWS error?
* Filesystem store in v0 or v0.2?
* Basic range requests in early versions?

---

## 14. Compatibility checklist

Must support:

```bash
aws --endpoint-url=http://localhost:4566 s3 mb s3://test-bucket
aws --endpoint-url=http://localhost:4566 s3 cp ./file.txt s3://test-bucket/file.txt
aws --endpoint-url=http://localhost:4566 s3 ls s3://test-bucket/
```
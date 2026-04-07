# awrust-s3 — Architecture

**Status:** v0.3 complete
**License:** MIT
**Distribution:** Docker image (linux/amd64, linux/arm64) + cargo binary

---

## 1. Goals

### 1.1 Product goals

* **Fast**, **deterministic**, **Docker-first** S3 emulator for local development, integration tests, and CI pipelines.
* **Simple to run**: single container or binary, minimal configuration.
* **Explicitly scoped**: only documented operations are supported.

### 1.2 Engineering goals

* **Testable** and **modular** — can later be bundled into an `awrust` stack container.
* Prefer correctness of supported behavior over breadth.
* Low maintenance cost: no plugin systems, custom DSLs, or complex persistence layers.

---

## 2. Non-goals

* Full AWS S3 parity.
* Production object storage.
* IAM policy simulation or SigV4 verification.
* Multi-region semantics.
* Event notifications (SNS/SQS/EventBridge).

---

## 3. High-level architecture

### 3.1 Components

```
Request → CORS layer → Virtual-host rewrite → Router → Handler → Store → Response
```

* **CORS layer** — permissive `Access-Control-*` headers on all responses, handles `OPTIONS` preflight.
* **Virtual-host rewrite** — extracts bucket from `Host` header when subdomain of `AWRUST_S3_BASE_DOMAIN`, rewrites to path-style.
* **Router** — Axum router matching method + path + query params.
* **Handlers** — parse request, call Store, assemble XML/headers.
* **Store** — pluggable trait with `MemoryStore` and `FsStore` implementations.
* **XML serializer** — quick-xml with serde for S3-compatible responses.

### 3.2 Crate structure

```
crates/
  awrust-s3-domain/    # Store trait, types, MemoryStore, FsStore (no HTTP)
  awrust-s3-server/    # Axum server, router, handlers, XML, error mapping
```

The domain crate has zero HTTP dependencies. The server crate depends on the domain crate.

---

## 4. Routing

### 4.1 Path-style

* `/health` — health check
* `GET /` — list buckets
* `/<bucket>`:
  * `PUT` create bucket
  * `HEAD` head bucket
  * `DELETE` delete bucket
  * `GET` list objects / list uploads / list versions / bucket location (query-param dispatch)
  * `POST ?delete` batch delete
* `/<bucket>/<key>`:
  * `PUT` put object / copy object / upload part (query-param + header dispatch)
  * `GET` get object / get tagging (query-param dispatch)
  * `HEAD` head object
  * `DELETE` delete object / abort upload / delete tagging (query-param dispatch)
  * `POST ?uploads` initiate multipart / `POST ?uploadId=X` complete multipart
  * `PUT ?tagging` / `GET ?tagging` / `DELETE ?tagging` object tagging

### 4.2 Virtual-host style

When `Host` is `<bucket>.<base-domain>`, the bucket is extracted from the host header and the path is treated as the key. This is transparent to handlers — the rewrite middleware normalizes to path-style before routing.

---

## 5. Key design decisions

### 5.1 Pluggable storage

The `Store` trait in `awrust-s3-domain` defines all storage operations. Implementations:

* **MemoryStore** — `DashMap`-based concurrent store. Ephemeral. Default.
* **FsStore** — filesystem-backed with encoded key paths. Persistent across restarts.

### 5.2 Determinism

* Lexicographic ordering for all listings (buckets, objects, uploads).
* Injectable clock for timestamps in tests.
* Stable request ID (UUID v4) per response.

### 5.3 Authentication

Accepts all requests regardless of auth headers. No SigV4 validation.

### 5.4 ETag computation

* Single-part uploads: MD5 hex digest.
* Multipart uploads: composite `MD5(concat(part_md5s))-N` matching AWS behavior.

### 5.5 CORS

Permissive CORS on all responses. No origin filtering. This matches the use case of local development and testing where browser-based S3 access is common.

---

## 6. Storage design

### 6.1 In-memory

* `DashMap<BucketName, BucketState>` — bucket metadata + creation time.
* `DashMap<(BucketName, Key), ObjectRecord>` — object data + metadata + etag + timestamp.
* `DashMap<UploadId, MultipartState>` — in-flight multipart uploads.
* `DashMap<(BucketName, Key), TagSet>` — object tags.

### 6.2 Filesystem

```
<data_dir>/
  buckets/
    <bucket>/
      .created          # creation timestamp
      objects/
        <encoded-key>   # object bytes
      meta/
        <encoded-key>.json   # ObjectMeta as JSON
      tags/
        <encoded-key>.json   # TagSet as JSON
      uploads/
        <upload-id>/
          part-<N>      # part bytes
```

Keys are URL-encoded for filesystem safety.

---

## 7. Testing strategy

### 7.1 Unit tests

Rust tests via `cargo test --workspace`:

* Store trait contract tests (both implementations)
* ETag computation
* XML response snapshots
* Deterministic ordering

### 7.2 Integration tests

BDD tests via Behave + boto3 in `tests/integration/`:

* 14 feature files covering all supported operations
* Runs against both memory and fs backends
* Runs against both cargo binary and Docker image
* Server lifecycle managed by `environment.py` (random port, auto-teardown)

### 7.3 CI

GitHub Actions:

* **build** — fmt, clippy, unit tests, release build
* **integration** — behave against memory + fs backends
* **docker** — build image, behave against container (memory + fs)

---

## 8. Distribution

* **GHCR**: `ghcr.io/awrust/awrust-s3:<version>`
* **Multi-arch**: linux/amd64, linux/arm64
* **Semantic versioning**: 0.x until stable
* **Docker image**: multi-stage alpine build, minimal runtime image

---

## 9. Resolved questions

| Question | Decision |
|----------|----------|
| Include ListAllMyBuckets? | Yes — `GET /` returns `ListAllMyBucketsResult` |
| Idempotent bucket creation? | Yes — `create_bucket` is a no-op if exists |
| Filesystem store in v0? | Yes — selectable via `AWRUST_S3_STORE=fs` |
| Range requests? | Yes — `Range` header with 206 responses |
| Virtual-host addressing? | Yes — via `AWRUST_S3_BASE_DOMAIN` host rewrite |
| CORS? | Yes — permissive on all responses |

# awrust-s3 — Usage Guide

## Configuration

All configuration is via environment variables.

| Variable | Default | Description |
|----------|---------|-------------|
| `AWRUST_S3_LISTEN_ADDR` | `0.0.0.0:4566` | Listen address |
| `AWRUST_S3_STORE` | `memory` | Storage backend: `memory` or `fs` |
| `AWRUST_S3_DATA_DIR` | `/data` | Data directory for `fs` backend |
| `AWRUST_S3_BASE_DOMAIN` | `localhost` | Base domain for virtual-host addressing |
| `AWRUST_LOG` | `info` | Log level (tracing filter syntax) |

## Running

### Docker

```bash
docker run --rm -p 4566:4566 ghcr.io/awrust/awrust-s3:0.x
```

With filesystem persistence:

```bash
docker run --rm -p 4566:4566 \
  -e AWRUST_S3_STORE=fs \
  -v /tmp/s3data:/data \
  ghcr.io/awrust/awrust-s3:0.x
```

### From source

```bash
cargo run -p awrust-s3-server
```

```bash
AWRUST_S3_STORE=fs AWRUST_S3_DATA_DIR=/tmp/s3data cargo run -p awrust-s3-server
```

---

## Addressing styles

### Path-style

```
http://localhost:4566/<bucket>/<key>
```

### Virtual-host style

```
http://<bucket>.localhost/<key>
```

Controlled by `AWRUST_S3_BASE_DOMAIN`. The server extracts the bucket name from the `Host` header when the host is a subdomain of the base domain.

---

## Supported operations

### Health check

```
GET /health → 200 {"status":"ok"}
```

### Buckets

| Operation | Method | Path | Notes |
|-----------|--------|------|-------|
| Create bucket | `PUT` | `/<bucket>` | Idempotent |
| Head bucket | `HEAD` | `/<bucket>` | |
| Delete bucket | `DELETE` | `/<bucket>` | Must be empty |
| List buckets | `GET` | `/` | |
| Get bucket location | `GET` | `/<bucket>?location` | Returns `us-east-1` |

### Objects

| Operation | Method | Path | Notes |
|-----------|--------|------|-------|
| Put object | `PUT` | `/<bucket>/<key>` | |
| Get object | `GET` | `/<bucket>/<key>` | Supports `Range` header |
| Head object | `HEAD` | `/<bucket>/<key>` | |
| Delete object | `DELETE` | `/<bucket>/<key>` | |
| Copy object | `PUT` | `/<bucket>/<key>` | With `x-amz-copy-source` header |
| Batch delete | `POST` | `/<bucket>?delete` | XML body with key list |
| List objects (v2) | `GET` | `/<bucket>?list-type=2` | Prefix, delimiter, pagination |
| List object versions | `GET` | `/<bucket>?versions` | Stub: all versions are null |

### Multipart upload

| Operation | Method | Path |
|-----------|--------|------|
| Initiate | `POST` | `/<bucket>/<key>?uploads` |
| Upload part | `PUT` | `/<bucket>/<key>?uploadId=X&partNumber=N` |
| Complete | `POST` | `/<bucket>/<key>?uploadId=X` |
| Abort | `DELETE` | `/<bucket>/<key>?uploadId=X` |
| List uploads | `GET` | `/<bucket>?uploads` |

### Object tagging

| Operation | Method | Path |
|-----------|--------|------|
| Put tags | `PUT` | `/<bucket>/<key>?tagging` |
| Get tags | `GET` | `/<bucket>/<key>?tagging` |
| Delete tags | `DELETE` | `/<bucket>/<key>?tagging` |

### CORS

All responses include permissive CORS headers. `OPTIONS` preflight requests are handled automatically.

---

## Headers

### Request headers

| Header | Purpose |
|--------|---------|
| `Content-Type` | MIME type for stored object |
| `x-amz-meta-*` | Custom metadata (preserved on get/head) |
| `x-amz-copy-source` | `/<bucket>/<key>` for copy operations |
| `Range` | Byte range: `bytes=0-99`, `bytes=100-`, `bytes=-50` |

### Response headers

| Header | Purpose |
|--------|---------|
| `ETag` | MD5 for single-part, composite `MD5-N` for multipart |
| `Content-Type` | Original MIME type |
| `Content-Length` | Body size in bytes |
| `Last-Modified` | ISO 8601 timestamp |
| `x-amz-meta-*` | Custom metadata |
| `x-amz-request-id` | UUID per request |
| `Accept-Ranges` | `bytes` (on GET/HEAD object) |
| `Content-Range` | `bytes start-end/total` (on 206 responses) |

---

## List objects query parameters

| Parameter | Description |
|-----------|-------------|
| `prefix` | Filter keys by prefix |
| `delimiter` | Group keys into common prefixes (typically `/`) |
| `max-keys` | Max results per page (default: 1000) |
| `continuation-token` | Pagination token from previous response |

---

## Error responses

All errors return XML:

```xml
<Error>
  <Code>NoSuchBucket</Code>
  <Message>The specified bucket does not exist</Message>
</Error>
```

| Code | HTTP Status | Meaning |
|------|-------------|---------|
| `NoSuchBucket` | 404 | Bucket does not exist |
| `NoSuchKey` | 404 | Object does not exist |
| `BucketNotEmpty` | 409 | Cannot delete non-empty bucket |
| `NoSuchUpload` | 404 | Invalid multipart upload ID |
| `InvalidPart` | 400 | Invalid part in multipart completion |

Range requests return `416 Range Not Satisfiable` for out-of-bounds ranges.

---

## Authentication

awrust-s3 accepts all requests regardless of authentication headers. No SigV4 validation is performed. Use `--no-sign-request` with the AWS CLI or disable signing in SDK clients.

---

## AWS CLI examples

```bash
ENDPOINT="--endpoint-url=http://localhost:4566 --no-sign-request"

# Buckets
aws $ENDPOINT s3 mb s3://my-bucket
aws $ENDPOINT s3 ls
aws $ENDPOINT s3 rb s3://my-bucket

# Objects
aws $ENDPOINT s3 cp file.txt s3://my-bucket/file.txt
aws $ENDPOINT s3 cp s3://my-bucket/file.txt downloaded.txt
aws $ENDPOINT s3 ls s3://my-bucket/
aws $ENDPOINT s3 rm s3://my-bucket/file.txt

# Multipart (automatic for files > 8MB)
aws $ENDPOINT s3 cp large.bin s3://my-bucket/large.bin

# Copy
aws $ENDPOINT s3 cp s3://my-bucket/file.txt s3://my-bucket/copy.txt

# Recursive
aws $ENDPOINT s3 cp ./dir/ s3://my-bucket/prefix/ --recursive
aws $ENDPOINT s3 rm s3://my-bucket/prefix/ --recursive
```

## boto3 examples

```python
import boto3

s3 = boto3.client(
    "s3",
    endpoint_url="http://localhost:4566",
    aws_access_key_id="test",
    aws_secret_access_key="test",
    region_name="us-east-1",
)

s3.create_bucket(Bucket="my-bucket")

s3.put_object(Bucket="my-bucket", Key="hello.txt", Body=b"hello world")

obj = s3.get_object(Bucket="my-bucket", Key="hello.txt")
print(obj["Body"].read())

s3.put_object_tagging(
    Bucket="my-bucket",
    Key="hello.txt",
    Tagging={"TagSet": [{"Key": "env", "Value": "dev"}]},
)

response = s3.list_objects_v2(Bucket="my-bucket", Prefix="", Delimiter="/")
for obj in response.get("Contents", []):
    print(obj["Key"], obj["Size"])
```

## Testing

### Rust unit tests

```bash
cargo test --workspace
```

### BDD integration tests (Behave + boto3)

```bash
pip install -r tests/integration/requirements.txt
cd tests/integration

behave                                    # memory backend
STORE=fs behave                           # filesystem backend
IMAGE=awrust-s3:test behave              # Docker + memory
IMAGE=awrust-s3:test STORE=fs behave     # Docker + filesystem
```

import urllib.request

from behave import when, then


def _encode_aws_chunked(data: bytes) -> bytes:
    chunk_size = 16384
    buf = bytearray()
    offset = 0
    while offset < len(data):
        end = min(offset + chunk_size, len(data))
        chunk = data[offset:end]
        buf.extend(f"{len(chunk):x}\r\n".encode())
        buf.extend(chunk)
        buf.extend(b"\r\n")
        offset = end
    buf.extend(b"0\r\n\r\n")
    return bytes(buf)


@when("I put an aws-chunked object \"{path}\" with content '{content}'")
def step_put_aws_chunked(context, path, content):
    bucket, key = path.split("/", 1)
    raw = content.encode()
    chunked_body = _encode_aws_chunked(raw)

    url = f"{context.base_url}/{bucket}/{key}"
    req = urllib.request.Request(url, data=chunked_body, method="PUT")
    req.add_header("content-encoding", "aws-chunked")
    req.add_header("x-amz-content-sha256", "STREAMING-AWS4-HMAC-SHA256-PAYLOAD")
    req.add_header("x-amz-decoded-content-length", str(len(raw)))
    urllib.request.urlopen(req)


@when(
    'I put a large aws-chunked object "{path}" of {size:d} bytes'
)
def step_put_large_aws_chunked(context, path, size):
    bucket, key = path.split("/", 1)
    raw = bytes(range(256)) * (size // 256) + bytes(range(size % 256))
    chunked_body = _encode_aws_chunked(raw)

    url = f"{context.base_url}/{bucket}/{key}"
    req = urllib.request.Request(url, data=chunked_body, method="PUT")
    req.add_header("content-encoding", "aws-chunked")
    req.add_header("x-amz-content-sha256", "STREAMING-AWS4-HMAC-SHA256-PAYLOAD")
    req.add_header("x-amz-decoded-content-length", str(len(raw)))
    urllib.request.urlopen(req)


@then("the aws-chunked object \"{path}\" should contain '{content}'")
def step_assert_aws_chunked_content(context, path, content):
    bucket, key = path.split("/", 1)
    resp = context.s3.get_object(Bucket=bucket, Key=key)
    body = resp["Body"].read().decode()
    assert body == content, f"expected {content!r}, got {body!r}"


@then('object "{path}" should have content length {expected:d}')
def step_assert_content_length(context, path, expected):
    bucket, key = path.split("/", 1)
    resp = context.s3.get_object(Bucket=bucket, Key=key)
    body = resp["Body"].read()
    assert len(body) == expected, f"expected {expected} bytes, got {len(body)}"

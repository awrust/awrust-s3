import hashlib
import os
import subprocess
import tempfile

from behave import given, when, then
from botocore.exceptions import ClientError


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@when('I upload "{path}" as multipart with {n:d} parts of {size_mb:d}MB each')
def step_multipart_upload(context, path, n, size_mb):
    bucket, key = _split(path)
    part_size = size_mb * 1024 * 1024

    resp = context.s3.create_multipart_upload(Bucket=bucket, Key=key)
    upload_id = resp["UploadId"]

    parts = []
    context._multipart_content = b""
    for i in range(1, n + 1):
        data = os.urandom(part_size)
        context._multipart_content += data
        part_resp = context.s3.upload_part(
            Bucket=bucket, Key=key, UploadId=upload_id,
            PartNumber=i, Body=data,
        )
        parts.append({"ETag": part_resp["ETag"], "PartNumber": i})

    context.s3.complete_multipart_upload(
        Bucket=bucket, Key=key, UploadId=upload_id,
        MultipartUpload={"Parts": parts},
    )


@then('object "{path}" should have size {expected_size:d}')
def step_assert_object_size(context, path, expected_size):
    bucket, key = _split(path)
    resp = context.s3.head_object(Bucket=bucket, Key=key)
    actual = resp["ContentLength"]
    assert actual == expected_size, f"expected size {expected_size}, got {actual}"


@then('I can download "{path}" and it matches the uploaded content')
def step_assert_download_matches(context, path):
    bucket, key = _split(path)
    resp = context.s3.get_object(Bucket=bucket, Key=key)
    body = resp["Body"].read()
    assert body == context._multipart_content, (
        f"downloaded content does not match (got {len(body)} bytes, "
        f"expected {len(context._multipart_content)})"
    )


@when('I initiate a multipart upload for "{path}"')
def step_initiate_multipart(context, path):
    bucket, key = _split(path)
    resp = context.s3.create_multipart_upload(Bucket=bucket, Key=key)
    context._upload_id = resp["UploadId"]
    context._upload_bucket = bucket
    context._upload_key = key


@when("I abort the multipart upload")
def step_abort_multipart(context):
    context.s3.abort_multipart_upload(
        Bucket=context._upload_bucket,
        Key=context._upload_key,
        UploadId=context._upload_id,
    )


@when('I copy a {size_mb:d}MB file to "{path}" using aws s3 cp')
def step_aws_cli_cp(context, size_mb, path):
    bucket, key = _split(path)
    size = size_mb * 1024 * 1024

    with tempfile.NamedTemporaryFile(delete=False) as f:
        f.write(os.urandom(size))
        tmp_path = f.name

    try:
        subprocess.run(
            [
                "aws", "--endpoint-url", context.base_url,
                "--no-sign-request",
                "s3", "cp", tmp_path, f"s3://{bucket}/{key}",
            ],
            check=True,
            capture_output=True,
            timeout=30,
        )
    finally:
        os.unlink(tmp_path)

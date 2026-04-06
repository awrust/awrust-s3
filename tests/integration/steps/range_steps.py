import os
import urllib.request

from behave import given, when, then


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@given('object "{path}" has {n:d} bytes of known content')
def step_create_known_content(context, path, n):
    bucket, key = _split(path)
    context._known_content = bytes(range(256)) * (n // 256 + 1)
    context._known_content = context._known_content[:n]
    context.s3.put_object(Bucket=bucket, Key=key, Body=context._known_content)


@when('I get bytes {start:d} to {end:d} of "{path}"')
def step_get_byte_range(context, start, end, path):
    bucket, key = _split(path)
    range_header = f"bytes={start}-{end}"
    resp = context.s3.get_object(Bucket=bucket, Key=key, Range=range_header)
    context._range_response = resp
    context._range_body = resp["Body"].read()
    context._range_status = resp["ResponseMetadata"]["HTTPStatusCode"]


@when('I get bytes from {start:d} to end of "{path}"')
def step_get_byte_range_open_end(context, start, path):
    bucket, key = _split(path)
    range_header = f"bytes={start}-"
    resp = context.s3.get_object(Bucket=bucket, Key=key, Range=range_header)
    context._range_response = resp
    context._range_body = resp["Body"].read()
    context._range_status = resp["ResponseMetadata"]["HTTPStatusCode"]


@when('I get the last {n:d} bytes of "{path}"')
def step_get_last_n_bytes(context, n, path):
    bucket, key = _split(path)
    range_header = f"bytes=-{n}"
    resp = context.s3.get_object(Bucket=bucket, Key=key, Range=range_header)
    context._range_response = resp
    context._range_body = resp["Body"].read()
    context._range_status = resp["ResponseMetadata"]["HTTPStatusCode"]


@then("the response status should be {status:d}")
def step_assert_status(context, status):
    assert context._range_status == status, (
        f"expected status {status}, got {context._range_status}"
    )


@then("the response body should be {n:d} bytes")
def step_assert_body_length(context, n):
    actual = len(context._range_body)
    assert actual == n, f"expected {n} bytes, got {actual}"


@then("the response body should match bytes {start:d} to {end:d} of the original")
def step_assert_body_matches_range(context, start, end):
    expected = context._known_content[start:end + 1]
    assert context._range_body == expected, (
        f"body mismatch: expected {len(expected)} bytes, got {len(context._range_body)}"
    )


@when('I get object "{path}"')
def step_get_object_full(context, path):
    bucket, key = _split(path)
    resp = context.s3.get_object(Bucket=bucket, Key=key)
    context._get_response = resp


@then('the response should have header "{header}" with value "{value}"')
def step_assert_response_header(context, header, value):
    headers = context._get_response["ResponseMetadata"]["HTTPHeaders"]
    actual = headers.get(header.lower())
    assert actual == value, f"expected header {header}={value}, got {actual}"

import re

from behave import given, when, then
from botocore.exceptions import ClientError


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@given('object "{path}" contains "{content}"')
def step_object_contains(context, path, content):
    bucket, key = _split(path)
    context.s3.put_object(Bucket=bucket, Key=key, Body=content.encode())


@when('I put object "{path}" with content "{content}"')
def step_put_object(context, path, content):
    bucket, key = _split(path)
    context.s3.put_object(Bucket=bucket, Key=key, Body=content.encode())


@when('I delete object "{path}"')
def step_delete_object(context, path):
    bucket, key = _split(path)
    context.s3.delete_object(Bucket=bucket, Key=key)


@when('I try to get object "{path}"')
def step_try_get_object(context, path):
    bucket, key = _split(path)
    try:
        context.s3.get_object(Bucket=bucket, Key=key)
        context.last_error = None
    except ClientError as e:
        context.last_error = e


@when('I list objects in "{bucket}" with prefix "{prefix}"')
def step_list_objects(context, bucket, prefix):
    resp = context.s3.list_objects_v2(Bucket=bucket, Prefix=prefix)
    context.listed_keys = [obj["Key"] for obj in resp.get("Contents", [])]


@then('object "{path}" should contain "{content}"')
def step_assert_object_content(context, path, content):
    bucket, key = _split(path)
    resp = context.s3.get_object(Bucket=bucket, Key=key)
    body = resp["Body"].read().decode()
    assert body == content, f"expected {content!r}, got {body!r}"


@then('object "{path}" should not exist')
def step_assert_object_not_exists(context, path):
    bucket, key = _split(path)
    try:
        context.s3.get_object(Bucket=bucket, Key=key)
        assert False, f"object {path} still exists"
    except ClientError:
        pass


@then('the listed keys should be "{expected}"')
def step_assert_listed_keys(context, expected):
    expected_keys = expected.split(",")
    assert context.listed_keys == expected_keys, (
        f"expected {expected_keys}, got {context.listed_keys}"
    )


@then("the operation should fail")
def step_assert_operation_failed(context):
    assert context.last_error is not None, "expected an error but operation succeeded"


@when('I head object "{path}"')
def step_head_object(context, path):
    bucket, key = _split(path)
    context.head_response = context.s3.head_object(Bucket=bucket, Key=key)


@then('the head response content length should be "{length}"')
def step_assert_head_content_length(context, length):
    actual = str(context.head_response["ContentLength"])
    assert actual == length, f"expected content-length {length}, got {actual}"


@then("the head response should have an etag")
def step_assert_head_etag(context):
    etag = context.head_response.get("ETag")
    assert etag is not None and len(etag) > 0, "expected ETag header"


@when('I upload "{path}" with body "{content}" and content type "{ct}"')
def step_upload_with_ct(context, path, content, ct):
    bucket, key = _split(path)
    context.s3.put_object(Bucket=bucket, Key=key, Body=content.encode(), ContentType=ct)


@then('object "{path}" should have content type "{expected_ct}"')
def step_assert_content_type(context, path, expected_ct):
    bucket, key = _split(path)
    resp = context.s3.head_object(Bucket=bucket, Key=key)
    actual = resp["ContentType"]
    assert actual == expected_ct, f"expected content-type {expected_ct}, got {actual}"


@when('I upload "{path}" with body "{content}" and metadata "{meta_str}"')
def step_upload_with_metadata(context, path, content, meta_str):
    bucket, key = _split(path)
    metadata = dict(pair.split("=") for pair in meta_str.split(","))
    context.s3.put_object(
        Bucket=bucket, Key=key, Body=content.encode(), Metadata=metadata
    )


@then('object "{path}" should have metadata "{meta_key}" with value "{meta_value}"')
def step_assert_metadata(context, path, meta_key, meta_value):
    bucket, key = _split(path)
    resp = context.s3.head_object(Bucket=bucket, Key=key)
    metadata = resp.get("Metadata", {})
    actual = metadata.get(meta_key)
    assert actual == meta_value, (
        f"expected metadata {meta_key}={meta_value}, got {actual} (all: {metadata})"
    )


RFC_7231_PATTERN = re.compile(
    r"^(Mon|Tue|Wed|Thu|Fri|Sat|Sun), \d{2} "
    r"(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) "
    r"\d{4} \d{2}:\d{2}:\d{2} GMT$"
)


@then('the last modified header for "{path}" should be RFC 7231 format')
def step_assert_last_modified_rfc7231(context, path):
    bucket, key = _split(path)
    resp = context.s3.head_object(Bucket=bucket, Key=key)
    last_mod = resp["ResponseMetadata"]["HTTPHeaders"]["last-modified"]
    assert RFC_7231_PATTERN.match(last_mod), (
        f"expected RFC 7231 format, got {last_mod!r}"
    )

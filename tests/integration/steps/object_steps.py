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

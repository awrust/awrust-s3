from behave import when, then
from botocore.exceptions import ClientError


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@when('I set tags on "{path}" with "{tags_str}"')
def step_put_tagging(context, path, tags_str):
    bucket, key = _split(path)
    tag_set = []
    for pair in tags_str.split(","):
        k, v = pair.split("=")
        tag_set.append({"Key": k, "Value": v})
    context.s3.put_object_tagging(
        Bucket=bucket, Key=key, Tagging={"TagSet": tag_set}
    )


@when('I delete tags on "{path}"')
def step_delete_tagging(context, path):
    bucket, key = _split(path)
    context.s3.delete_object_tagging(Bucket=bucket, Key=key)


@when('I try to get tags on "{path}"')
def step_try_get_tagging(context, path):
    bucket, key = _split(path)
    try:
        context.s3.get_object_tagging(Bucket=bucket, Key=key)
        context.last_tagging_error = None
    except ClientError as e:
        context.last_tagging_error = e


@then('object "{path}" should have tag "{tag_key}" with value "{tag_value}"')
def step_assert_tag(context, path, tag_key, tag_value):
    bucket, key = _split(path)
    resp = context.s3.get_object_tagging(Bucket=bucket, Key=key)
    tags = {t["Key"]: t["Value"] for t in resp.get("TagSet", [])}
    actual = tags.get(tag_key)
    assert actual == tag_value, (
        f"expected tag {tag_key}={tag_value}, got {actual} (all: {tags})"
    )


@then('object "{path}" should have no tags')
def step_assert_no_tags(context, path):
    bucket, key = _split(path)
    resp = context.s3.get_object_tagging(Bucket=bucket, Key=key)
    tag_set = resp.get("TagSet", [])
    assert len(tag_set) == 0, f"expected no tags, got {tag_set}"


@then("the tagging operation should fail")
def step_assert_tagging_failed(context):
    assert context.last_tagging_error is not None, (
        "expected an error but tagging operation succeeded"
    )

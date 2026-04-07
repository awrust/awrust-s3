import urllib.request

from behave import when, then


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@when('I upload "{path}" via presigned URL with content "{content}"')
def step_put_presigned(context, path, content):
    bucket, key = _split(path)
    url = context.s3.generate_presigned_url(
        "put_object",
        Params={"Bucket": bucket, "Key": key},
        ExpiresIn=3600,
    )
    req = urllib.request.Request(url, data=content.encode(), method="PUT")
    with urllib.request.urlopen(req) as resp:
        context.presigned_status = resp.status


@when('I download "{path}" via presigned URL')
def step_get_presigned(context, path):
    bucket, key = _split(path)
    url = context.s3.generate_presigned_url(
        "get_object",
        Params={"Bucket": bucket, "Key": key},
        ExpiresIn=3600,
    )
    with urllib.request.urlopen(url) as resp:
        context.presigned_body = resp.read().decode()


@then('the presigned response should contain "{expected}"')
def step_assert_presigned_content(context, expected):
    assert context.presigned_body == expected, (
        f"expected {expected!r}, got {context.presigned_body!r}"
    )

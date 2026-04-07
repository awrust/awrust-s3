from behave import given, when, then
from botocore.exceptions import ClientError


@given('bucket "{name}" exists')
def step_bucket_exists(context, name):
    context.s3.create_bucket(Bucket=name)


@when('I create bucket "{name}"')
def step_create_bucket(context, name):
    context.s3.create_bucket(Bucket=name)


@when('I delete bucket "{name}"')
def step_delete_bucket(context, name):
    context.s3.delete_bucket(Bucket=name)


@when('I try to delete bucket "{name}"')
def step_try_delete_bucket(context, name):
    try:
        context.s3.delete_bucket(Bucket=name)
        context.last_error = None
    except ClientError as e:
        context.last_error = e


@then('bucket "{name}" should exist')
def step_assert_bucket_exists(context, name):
    context.s3.head_bucket(Bucket=name)


@then('bucket "{name}" should not exist')
def step_assert_bucket_not_exists(context, name):
    try:
        context.s3.head_bucket(Bucket=name)
        assert False, f"bucket {name} still exists"
    except ClientError as e:
        assert e.response["Error"]["Code"] in ("404", "NoSuchBucket")


@when("I list all buckets")
def step_list_all_buckets(context):
    resp = context.s3.list_buckets()
    context.bucket_list_raw = resp.get("Buckets", [])
    context.bucket_list = [b["Name"] for b in context.bucket_list_raw]


@then('the bucket list should contain "{name}"')
def step_assert_bucket_in_list(context, name):
    assert name in context.bucket_list, (
        f"expected {name} in {context.bucket_list}"
    )


@then('bucket "{name}" in the list should have a creation date')
def step_assert_bucket_has_creation_date(context, name):
    for b in context.bucket_list_raw:
        if b["Name"] == name:
            assert "CreationDate" in b, f"bucket {name} missing CreationDate"
            return
    assert False, f"bucket {name} not found in list"


@when('I get the location of bucket "{name}"')
def step_get_bucket_location(context, name):
    resp = context.s3.get_bucket_location(Bucket=name)
    context.location = resp.get("LocationConstraint")


@when('I try to get the location of bucket "{name}"')
def step_try_get_bucket_location(context, name):
    try:
        context.s3.get_bucket_location(Bucket=name)
        context.last_error = None
    except ClientError as e:
        context.last_error = e


@then('the location should be "{region}"')
def step_assert_location(context, region):
    assert context.location == region, (
        f"expected {region}, got {context.location}"
    )

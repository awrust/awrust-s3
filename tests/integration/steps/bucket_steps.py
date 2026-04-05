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

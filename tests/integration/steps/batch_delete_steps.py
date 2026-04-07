from behave import when, then


@when('I batch delete keys "{keys}" from "{bucket}"')
def step_batch_delete(context, keys, bucket):
    objects = [{"Key": k} for k in keys.split(",")]
    context.batch_delete_response = context.s3.delete_objects(
        Bucket=bucket, Delete={"Objects": objects}
    )


@when('I batch delete keys "{keys}" from "{bucket}" in quiet mode')
def step_batch_delete_quiet(context, keys, bucket):
    objects = [{"Key": k} for k in keys.split(",")]
    context.batch_delete_response = context.s3.delete_objects(
        Bucket=bucket, Delete={"Objects": objects, "Quiet": True}
    )


@then("the batch delete response should have {count:d} deleted keys")
def step_assert_deleted_count(context, count):
    deleted = context.batch_delete_response.get("Deleted", [])
    assert len(deleted) == count, f"expected {count} deleted, got {len(deleted)}: {deleted}"


@then("the batch delete response should have {count:d} errors")
def step_assert_error_count(context, count):
    errors = context.batch_delete_response.get("Errors", [])
    assert len(errors) == count, f"expected {count} errors, got {len(errors)}: {errors}"

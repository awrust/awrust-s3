from behave import given, when, then


@given('{n:d} objects exist in "{bucket}" with prefix "{prefix}"')
def step_create_n_objects(context, n, bucket, prefix):
    for i in range(n):
        key = f"{prefix}{i:04d}"
        context.s3.put_object(Bucket=bucket, Key=key, Body=f"data-{i}".encode())


@when('I list objects in "{bucket}" with prefix "{prefix}" and max keys {max_keys:d}')
def step_list_with_max_keys(context, bucket, prefix, max_keys):
    context._page_bucket = bucket
    context._page_prefix = prefix
    context._page_max_keys = max_keys
    resp = context.s3.list_objects_v2(
        Bucket=bucket, Prefix=prefix, MaxKeys=max_keys,
    )
    context._page_response = resp
    context._page_keys = [o["Key"] for o in resp.get("Contents", [])]


@when("I list the next page")
def step_list_next_page(context):
    token = context._page_response.get("NextContinuationToken")
    assert token is not None, "no NextContinuationToken in response"
    resp = context.s3.list_objects_v2(
        Bucket=context._page_bucket,
        Prefix=context._page_prefix,
        MaxKeys=context._page_max_keys,
        ContinuationToken=token,
    )
    context._page_response = resp
    context._page_keys = [o["Key"] for o in resp.get("Contents", [])]


@then("{n:d} objects should be returned")
def step_assert_n_objects(context, n):
    actual = len(context._page_keys)
    assert actual == n, f"expected {n} objects, got {actual}: {context._page_keys}"


@then("the response should be truncated")
def step_assert_truncated(context):
    assert context._page_response["IsTruncated"] is True, "expected truncated response"


@then("the response should not be truncated")
def step_assert_not_truncated(context):
    assert context._page_response["IsTruncated"] is False, "expected non-truncated response"

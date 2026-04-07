from behave import when, then


@when('I list objects in "{bucket}" using delimiter "{delimiter}"')
def step_list_with_delimiter(context, bucket, delimiter):
    context._delim_response = context.s3.list_objects_v2(
        Bucket=bucket, Delimiter=delimiter,
    )


@when('I list objects in "{bucket}" using prefix "{prefix}" and delimiter "{delimiter}"')
def step_list_with_prefix_and_delimiter(context, bucket, prefix, delimiter):
    context._delim_response = context.s3.list_objects_v2(
        Bucket=bucket, Prefix=prefix, Delimiter=delimiter,
    )


@when('I list objects in "{bucket}" using prefix "{prefix}" without delimiter')
def step_list_without_delimiter(context, bucket, prefix):
    context._delim_response = context.s3.list_objects_v2(
        Bucket=bucket, Prefix=prefix,
    )


@then('the content keys should be "{expected_keys}"')
def step_assert_content_keys(context, expected_keys):
    expected = sorted(expected_keys.split(","))
    actual = sorted(o["Key"] for o in context._delim_response.get("Contents", []))
    assert actual == expected, f"expected {expected}, got {actual}"


@then('the common prefix list should be "{expected_prefixes}"')
def step_assert_common_prefix_list(context, expected_prefixes):
    expected = sorted(expected_prefixes.split(","))
    actual = sorted(p["Prefix"] for p in context._delim_response.get("CommonPrefixes", []))
    assert actual == expected, f"expected {expected}, got {actual}"


@then("the common prefix list should be empty")
def step_assert_no_common_prefixes(context):
    prefixes = context._delim_response.get("CommonPrefixes", [])
    assert len(prefixes) == 0, f"expected no common prefixes, got {prefixes}"

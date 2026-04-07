from behave import given, when, then


@given('version list prefix is "{prefix}"')
def step_set_version_prefix(context, prefix):
    context.version_prefix = prefix


@when('I list object versions in "{bucket}"')
def step_list_versions(context, bucket):
    kwargs = {"Bucket": bucket}
    prefix = getattr(context, "version_prefix", None)
    if prefix:
        kwargs["Prefix"] = prefix
    resp = context.s3.list_object_versions(**kwargs)
    context.versions = resp.get("Versions", [])
    context.version_prefix = None


@then('the version list should contain keys "{expected}"')
def step_assert_version_keys(context, expected):
    expected_keys = expected.split(",")
    actual_keys = [v["Key"] for v in context.versions]
    assert actual_keys == expected_keys, (
        f"expected {expected_keys}, got {actual_keys}"
    )


@then("the version list should be empty")
def step_assert_versions_empty(context):
    assert len(context.versions) == 0, (
        f"expected empty, got {context.versions}"
    )


@then('every version id should be "{expected}"')
def step_assert_version_ids(context, expected):
    for v in context.versions:
        assert v["VersionId"] == expected, (
            f"expected VersionId={expected}, got {v['VersionId']}"
        )


@then("every version should be latest")
def step_assert_all_latest(context):
    for v in context.versions:
        assert v["IsLatest"] is True, (
            f"expected IsLatest=True for {v['Key']}"
        )

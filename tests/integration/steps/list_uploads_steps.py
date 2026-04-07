from behave import then


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@then('listing multipart uploads in "{bucket}" returns {n:d} uploads')
def step_list_uploads(context, bucket, n):
    resp = context.s3.list_multipart_uploads(Bucket=bucket)
    uploads = resp.get("Uploads", [])
    context._listed_uploads = uploads
    assert len(uploads) == n, f"expected {n} uploads, got {len(uploads)}"


@then('the upload keys include "{key1}" and "{key2}"')
def step_upload_keys_include(context, key1, key2):
    keys = sorted(u["Key"] for u in context._listed_uploads)
    assert key1 in keys, f"{key1} not in {keys}"
    assert key2 in keys, f"{key2} not in {keys}"
    for u in context._listed_uploads:
        assert "UploadId" in u, "missing UploadId"
        assert "Initiated" in u, "missing Initiated"


@then('listing multipart uploads in "{bucket}" with prefix "{prefix}" returns {n:d} upload')
def step_list_uploads_prefix(context, bucket, prefix, n):
    resp = context.s3.list_multipart_uploads(Bucket=bucket, Prefix=prefix)
    uploads = resp.get("Uploads", [])
    context._listed_uploads = uploads
    assert len(uploads) == n, f"expected {n} uploads, got {len(uploads)}"


@then('the upload key is "{key}"')
def step_upload_key_is(context, key):
    assert len(context._listed_uploads) >= 1
    assert context._listed_uploads[0]["Key"] == key

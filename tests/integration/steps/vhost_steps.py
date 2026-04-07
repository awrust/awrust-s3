import requests
from behave import when, then


def _vhost_url(context, bucket, path=""):
    return f"http://127.0.0.1:{context.port}/{path}"


def _vhost_headers(context, bucket):
    return {"Host": f"{bucket}.localhost:{context.port}"}


@when('I put object "{key}" with body "{body}" via virtual-host to "{bucket}"')
def step_vhost_put(context, key, body, bucket):
    resp = requests.put(
        _vhost_url(context, bucket, key),
        data=body.encode(),
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 200, f"PUT failed: {resp.status_code} {resp.text}"


@then('I can get object "{key}" via virtual-host from "{bucket}" with body "{expected}"')
def step_vhost_get(context, key, bucket, expected):
    resp = requests.get(
        _vhost_url(context, bucket, key),
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 200, f"GET failed: {resp.status_code} {resp.text}"
    assert resp.text == expected, f"expected {expected!r}, got {resp.text!r}"


@when('I create bucket "{bucket}" via virtual-host')
def step_vhost_create_bucket(context, bucket):
    resp = requests.put(
        f"http://127.0.0.1:{context.port}/",
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 200, f"PUT bucket failed: {resp.status_code} {resp.text}"


@then('I can head bucket "{bucket}" via virtual-host')
def step_vhost_head_bucket(context, bucket):
    resp = requests.head(
        f"http://127.0.0.1:{context.port}/",
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 200, f"HEAD bucket failed: {resp.status_code}"


@when('I list objects via virtual-host from "{bucket}"')
def step_vhost_list_objects(context, bucket):
    resp = requests.get(
        f"http://127.0.0.1:{context.port}/",
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 200, f"LIST failed: {resp.status_code} {resp.text}"
    context.vhost_list_body = resp.text


@then('the virtual-host listing should contain "{key}"')
def step_vhost_list_contains(context, key):
    assert f"<Key>{key}</Key>" in context.vhost_list_body, (
        f"key {key} not found in listing: {context.vhost_list_body}"
    )


@when('I vhost-delete object "{key}" from "{bucket}"')
def step_vhost_delete(context, key, bucket):
    resp = requests.delete(
        _vhost_url(context, bucket, key),
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 204, f"DELETE failed: {resp.status_code}"


@then('getting object "{key}" via virtual-host from "{bucket}" should fail')
def step_vhost_get_fails(context, key, bucket):
    resp = requests.get(
        _vhost_url(context, bucket, key),
        headers=_vhost_headers(context, bucket),
    )
    assert resp.status_code == 404, f"expected 404, got {resp.status_code}"


@when('I get object "{path}" via path-style')
def step_path_style_get(context, path):
    resp = requests.get(f"{context.base_url}/{path}")
    context.path_style_response = resp


@then('the path-style response body should be "{expected}"')
def step_path_style_body(context, expected):
    assert context.path_style_response.text == expected, (
        f"expected {expected!r}, got {context.path_style_response.text!r}"
    )

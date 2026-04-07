import urllib.request

from behave import when, then


@when('I send an OPTIONS preflight for "{method}" on "{path}"')
def step_options_preflight(context, method, path):
    url = f"{context.base_url}{path}"
    req = urllib.request.Request(url, method="OPTIONS")
    req.add_header("Origin", "http://localhost:3000")
    req.add_header("Access-Control-Request-Method", method)
    req.add_header("Access-Control-Request-Headers", "content-type")
    context.cors_response = urllib.request.urlopen(req)


@when('I send a GET request to "{path}"')
def step_get_request(context, path):
    url = f"{context.base_url}{path}"
    req = urllib.request.Request(url, method="GET")
    req.add_header("Origin", "http://localhost:3000")
    context.cors_response = urllib.request.urlopen(req)


@then("the CORS response status should be {code:d}")
def step_check_status(context, code):
    assert context.cors_response.status == code, (
        f"expected {code}, got {context.cors_response.status}"
    )


@then('the CORS response should include header "{name}" with value "{value}"')
def step_check_header_value(context, name, value):
    actual = context.cors_response.getheader(name)
    assert actual is not None, f"header '{name}' not found"
    assert actual == value, f"expected '{value}', got '{actual}'"


@then('the CORS response should include header "{name}"')
def step_check_header_present(context, name):
    actual = context.cors_response.getheader(name)
    assert actual is not None, f"header '{name}' not found"


@then("the CORS response body should be empty")
def step_check_empty_body(context):
    body = context.cors_response.read()
    assert len(body) == 0, f"expected empty body, got {len(body)} bytes"

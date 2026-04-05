import os
import socket
import subprocess
import tempfile
import time
import urllib.request

import boto3


def _free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _wait_for_health(url, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            urllib.request.urlopen(f"{url}/health", timeout=1)
            return
        except Exception:
            time.sleep(0.1)
    raise RuntimeError(f"server did not become ready at {url}")


def before_all(context):
    port = _free_port()
    addr = f"127.0.0.1:{port}"
    context.base_url = f"http://{addr}"

    root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    store = os.environ.get("STORE", "memory")

    subprocess.run(
        ["cargo", "build", "-p", "awrust-s3-server"],
        cwd=root,
        check=True,
        capture_output=True,
    )

    env = os.environ.copy()
    env["AWRUST_S3_LISTEN_ADDR"] = addr
    env["AWRUST_S3_STORE"] = store
    env["RUST_LOG"] = "warn"

    if store == "fs":
        context._tmpdir = tempfile.TemporaryDirectory()
        env["AWRUST_S3_DATA_DIR"] = context._tmpdir.name

    context.server = subprocess.Popen(
        ["cargo", "run", "-p", "awrust-s3-server"],
        cwd=root,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    _wait_for_health(context.base_url)

    context.s3 = boto3.client(
        "s3",
        endpoint_url=context.base_url,
        aws_access_key_id="test",
        aws_secret_access_key="test",
        region_name="us-east-1",
    )


def after_all(context):
    if hasattr(context, "server"):
        context.server.terminate()
        context.server.wait(timeout=5)
    if hasattr(context, "_tmpdir"):
        context._tmpdir.cleanup()

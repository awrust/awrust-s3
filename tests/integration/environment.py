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


def _wait_for_health(url, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            urllib.request.urlopen(f"{url}/health", timeout=1)
            return
        except Exception:
            time.sleep(0.2)
    raise RuntimeError(f"server did not become ready at {url}")


def _start_cargo(context, port, store):
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))

    subprocess.run(
        ["cargo", "build", "-p", "awrust-s3-server"],
        cwd=root,
        check=True,
        capture_output=True,
    )

    env = os.environ.copy()
    env["AWRUST_S3_LISTEN_ADDR"] = f"127.0.0.1:{port}"
    env["AWRUST_S3_STORE"] = store
    env["AWRUST_LOG"] = "warn"

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


def _start_docker(context, port, store, image):
    docker_env = {
        "AWRUST_S3_LISTEN_ADDR": f"0.0.0.0:4566",
        "AWRUST_S3_STORE": store,
        "AWRUST_LOG": "warn",
    }

    cmd = [
        "docker", "run", "--rm", "-d",
        "-p", f"{port}:4566",
        "--name", f"awrust-bdd-{port}",
    ]

    for key, val in docker_env.items():
        cmd.extend(["-e", f"{key}={val}"])

    cmd.append(image)

    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    context._container = f"awrust-bdd-{port}"


def before_all(context):
    port = _free_port()
    context.base_url = f"http://127.0.0.1:{port}"

    store = os.environ.get("STORE", "memory")
    image = os.environ.get("IMAGE")

    if image:
        _start_docker(context, port, store, image)
    else:
        _start_cargo(context, port, store)

    _wait_for_health(context.base_url)

    context.s3 = boto3.client(
        "s3",
        endpoint_url=context.base_url,
        aws_access_key_id="test",
        aws_secret_access_key="test",
        region_name="us-east-1",
    )


def after_all(context):
    if hasattr(context, "_container"):
        subprocess.run(
            ["docker", "rm", "-f", context._container],
            capture_output=True,
            timeout=10,
        )
    if hasattr(context, "server"):
        context.server.terminate()
        context.server.wait(timeout=5)
    if hasattr(context, "_tmpdir"):
        context._tmpdir.cleanup()

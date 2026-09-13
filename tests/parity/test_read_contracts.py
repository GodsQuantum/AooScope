import json
import shutil
import socket
import subprocess
import tempfile
import time
import unittest
import urllib.request
from pathlib import Path

from tests.parity.normalize import normalize
from webui import create_app

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests" / "fixtures" / "appdata-v1"
ROUTES = ["/api/health", "/api/status", "/api/settings", "/api/pages"]


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def rust_get(base, path):
    with urllib.request.urlopen(base + path, timeout=3) as response:
        return response.status, response.headers.get_content_type(), json.load(response)


class ReadContractParityTests(unittest.TestCase):
    def test_reference_and_rust_read_contracts_match(self):
        with tempfile.TemporaryDirectory(prefix="aooscope-parity-") as temp:
            fixture = Path(temp) / "cfg"
            shutil.copytree(FIXTURE, fixture)
            flask = create_app(fixture)
            flask.config["TESTING"] = True
            client = flask.test_client()

            port = free_port()
            command = [
                "cargo", "run", "-q", "-p", "xtask", "--",
                "parity-server", str(fixture), f"127.0.0.1:{port}",
            ]
            with subprocess.Popen(
                command, cwd=ROOT, stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE, text=True,
            ) as process:
                base = f"http://127.0.0.1:{port}"
                try:
                    self._wait_for_server(base, process)
                    self._compare_routes(client, base)
                finally:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=5)

    def _wait_for_server(self, base, process):
        deadline = time.time() + 120
        while time.time() < deadline:
            if process.poll() is not None:
                stderr = process.stderr.read() if process.stderr else ""
                self.fail(f"Rust parity server exited early: {stderr}")
            try:
                rust_get(base, "/api/health")
                return
            except Exception:
                time.sleep(0.1)
        self.fail("Rust parity server did not become ready")

    def _compare_routes(self, client, base):
        for path in ROUTES:
            py = client.get(path)
            rs_status, rs_type, rs_json = rust_get(base, path)
            with self.subTest(path=path):
                self.assertEqual(py.status_code, rs_status)
                self.assertEqual(py.mimetype, rs_type)
                self.assertEqual(
                    normalize(path, py.get_json()),
                    normalize(path, rs_json),
                )


if __name__ == "__main__":
    unittest.main()

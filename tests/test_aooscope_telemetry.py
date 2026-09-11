import json
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
import tempfile
import unittest
from pathlib import Path

from aooscope.telemetry import (
    atomic_write_state,
    collect_sysfs,
    flatten_state,
    normalize_guests,
    normalize_pve_status,
    normalize_smart,
    PVEClient,
    collect_pve_fast,
    collect_pve_slow,
    collect_pve_smart,
    load_pve_client,
    RuntimeCollector,
)


class AooScopeTelemetryTests(unittest.TestCase):
    def make_file(self, root, rel, value):
        p = Path(root, rel)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(str(value), encoding="utf-8")

    def test_collects_real_style_hwmon_and_gpu_metrics(self):
        with tempfile.TemporaryDirectory() as td:
            self.make_file(td, "class/hwmon/hwmon0/name", "k10temp\n")
            self.make_file(td, "class/hwmon/hwmon0/temp1_input", "68125\n")
            self.make_file(td, "class/hwmon/hwmon1/name", "amdgpu\n")
            self.make_file(td, "class/hwmon/hwmon1/temp1_input", "48000\n")
            self.make_file(td, "class/hwmon/hwmon1/device/gpu_busy_percent", "37\n")
            self.make_file(td, "class/hwmon/hwmon1/device/mem_info_vram_used", "536870912\n")
            self.make_file(td, "class/hwmon/hwmon1/device/mem_info_vram_total", "2147483648\n")
            state = collect_sysfs(td)
            self.assertEqual(state["hardware"]["cpu_temp_c"], 68.1)
            self.assertEqual(state["hardware"]["gpu_temp_c"], 48.0)
            self.assertEqual(state["hardware"]["gpu_busy_pct"], 37)
            self.assertEqual(state["hardware"]["gpu_vram_pct"], 25.0)

    def test_load_pve_client_reads_pveum_json_file(self):
        with tempfile.TemporaryDirectory() as td:
            token=Path(td, "token.json")
            token.write_text(json.dumps({"full-tokenid":"display@pve!lcd","value":"secret"}))
            client=load_pve_client("http://127.0.0.1:8006/api2/json", token)
            self.assertEqual(client.token_id, "display@pve!lcd")
            self.assertEqual(client.token_secret, "secret")

    def test_collect_pve_smart_isolates_one_bad_disk(self):
        class Fake:
            def get(self, path, params=None):
                if params["disk"] == "/dev/sdb": raise OSError("offline")
                return {"health":"PASSED","type":"ata","attributes":[{"id":"194","raw":"38"}]}
        rows=collect_pve_smart(Fake(), "pve-node", [
            {"name":"sda","devpath":"/dev/sda"}, {"name":"sdb","devpath":"/dev/sdb"}])
        self.assertEqual(rows[0]["temperature_c"], 38)
        self.assertEqual(rows[1]["health"], "UNKNOWN")

    def test_normalizes_pve_node_status_and_guests(self):
        status = normalize_pve_status({
            "cpu": 0.125,
            "memory": {"used": 50, "total": 100},
            "uptime": 7200,
            "loadavg": ["1.25", "1.10", "0.90"],
            "wait": 0.015,
        })
        self.assertEqual(status["cpu_pct"], 12.5)
        self.assertEqual(status["memory_pct"], 50.0)
        self.assertEqual(status["uptime_seconds"], 7200)
        self.assertEqual(status["load1"], 1.25)
        self.assertEqual(status["iowait_pct"], 1.5)
        guests = normalize_guests(
            [{"status": "running"}, {"status": "stopped"}, {"status": "running"}],
            [{"status": "running"}, {"status": "stopped"}],
        )
        self.assertEqual(guests, {"lxc_running": 2, "lxc_total": 3, "vm_running": 1, "vm_total": 2, "guests_running": 3, "guests_total": 5})

    def test_normalizes_smart_ata_and_nvme(self):
        ata = normalize_smart("sda", {"health": "PASSED", "type": "ata", "attributes": [
            {"id": "  5", "name": "Reallocated_Sector_Ct", "raw": "2"},
            {"id": "194", "name": "Temperature_Celsius", "raw": "37 (0 13 0 0 0)"},
            {"id": "197", "name": "Current_Pending_Sector", "raw": "1"},
            {"id": "199", "name": "UDMA_CRC_Error_Count", "raw": "3"},
        ]})
        self.assertEqual(ata["temperature_c"], 37)
        self.assertEqual(ata["reallocated"], 2)
        self.assertEqual(ata["pending"], 1)
        self.assertEqual(ata["crc_errors"], 3)
        nvme = normalize_smart("nvme0n1", {"health": "PASSED", "type": "text", "wearout": 100, "text": "Temperature: 42 Celsius\nPower On Hours: 4,172\nMedia and Data Integrity Errors: 0\n"})
        self.assertEqual(nvme["temperature_c"], 42)
        self.assertEqual(nvme["power_on_hours"], 4172)
        self.assertEqual(nvme["media_errors"], 0)



    def test_collect_pve_fast_and_slow_normalizes_api_payloads(self):
        class FakeClient:
            def get(self, path, params=None):
                data = {
                    "/nodes/pve-node/status": {"cpu": 0.25, "memory": {"used": 60, "total": 100}, "uptime": 99, "loadavg": ["2.0"], "wait": 0.01},
                    "/nodes/pve-node/lxc": [{"vmid": 130, "status": "running"}, {"vmid": 200, "status": "stopped"}],
                    "/nodes/pve-node/qemu": [{"vmid": 100, "status": "running"}],
                    "/nodes/pve-node/storage": [{"storage": "local-zfs", "type": "zfspool", "used": 25, "total": 100, "active": 1}],
                    "/nodes/pve-node/disks/zfs": [{"name": "rpool", "health": "ONLINE", "alloc": 20, "size": 100, "frag": 2}],
                    "/nodes/pve-node/disks/list": [{"devpath": "/dev/sda", "model": "Disk A", "size": 1000, "health": "PASSED", "type": "hdd", "used": "ext4"}],
                }
                return data[path]
        fast = collect_pve_fast(FakeClient(), "pve-node")
        self.assertEqual(fast["cpu_pct"], 25.0)
        self.assertEqual(fast["memory_pct"], 60.0)
        self.assertEqual(fast["guests_running"], 2)
        slow = collect_pve_slow(FakeClient(), "pve-node")
        self.assertEqual(slow["storage"][0]["used_pct"], 25.0)
        self.assertEqual(slow["zfs"][0]["used_pct"], 20.0)
        self.assertEqual(slow["disks"][0]["name"], "sda")

    def test_pve_client_sends_token_header_and_reads_data(self):
        seen = {}
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                seen["path"] = self.path
                seen["auth"] = self.headers.get("Authorization")
                payload = json.dumps({"data": {"ok": 1}}).encode()
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            def log_message(self, *args):
                pass
        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            base = f"http://127.0.0.1:{server.server_port}/api2/json"
            client = PVEClient(base, "user@pve!lcd", "sekret", timeout=2)
            self.assertEqual(client.get("/nodes/pve-node/status"), {"ok": 1})
            self.assertEqual(seen["path"], "/api2/json/nodes/pve-node/status")
            self.assertEqual(seen["auth"], "PVEAPIToken=user@pve!lcd=sekret")
        finally:
            server.shutdown()
            server.server_close()

    def test_flatten_state_indexes_structured_lists(self):
        flat = flatten_state({"pve": {"disks": [{"name": "sda", "health": "PASSED"}]}})
        self.assertEqual(flat["aooscope_pve_disks_0_name"], "sda")
        self.assertEqual(flat["aooscope_pve_disks_0_health"], "PASSED")

    def test_flatten_state_produces_asterctl_labels(self):
        flat = flatten_state({"hardware": {"cpu_temp_c": 61.5, "gpu_busy_pct": 12}, "pve": {"guests_running": 9}})
        self.assertEqual(flat["aooscope_hardware_cpu_temp_c"], "61.5")
        self.assertEqual(flat["aooscope_hardware_gpu_busy_pct"], "12")
        self.assertEqual(flat["aooscope_pve_guests_running"], "9")

    def test_atomic_write_outputs_valid_json_and_sensor_file(self):
        with tempfile.TemporaryDirectory() as td:
            sensor = Path(td, "aooscope.txt")
            state_file = Path(td, "state.json")
            state = {"hardware": {"cpu_temp_c": 61.5}}
            atomic_write_state(state, sensor, state_file)
            self.assertEqual(json.loads(state_file.read_text()), state)
            self.assertIn("aooscope_hardware_cpu_temp_c: 61.5", sensor.read_text())
            self.assertFalse(Path(str(state_file) + ".tmp").exists())

    def test_settings_pve_can_fallback_to_mounted_token_file(self):
        import os
        from unittest.mock import patch
        from aooscope.telemetry import build_runtime_collector_from_settings
        settings={"providers":{"proxmox":{"enabled":True,"url":"https://pve.lan:8006","node":"pve","verify_tls":True}}}
        with tempfile.TemporaryDirectory() as td:
            token=Path(td,"token.json")
            token.write_text(json.dumps({"full-tokenid":"display@pve!lcd","value":"secret"}))
            with patch.dict(os.environ,{"PVE_TOKEN_FILE":str(token)},clear=False):
                collector=build_runtime_collector_from_settings(settings,{})
            self.assertIsNotNone(collector.pve_client)
            self.assertEqual(collector.pve_client.token_id,"display@pve!lcd")


if __name__ == "__main__":
    unittest.main()


class AooScopeTelemetrySettingsTests(unittest.TestCase):
    def test_build_runtime_collector_from_settings_uses_admin_config(self):
        from aooscope.telemetry import build_runtime_collector_from_settings
        settings = {"providers": {
            "proxmox": {"enabled": True, "url": "pve:8006", "node": "node-a", "verify_tls": False},
            "jellyfin": {"enabled": True, "url": "media:8096", "verify_tls": True},
        }}
        secrets = {
            "proxmox": {"api_token": "display@pve!lcd=secret"},
            "jellyfin": {"api_key": "jelly-secret"},
        }
        collector = build_runtime_collector_from_settings(settings, secrets)
        self.assertEqual(collector.pve_node, "node-a")
        self.assertEqual(collector.pve_client.token_id, "display@pve!lcd")
        self.assertEqual(collector.pve_client.token_secret, "secret")
        self.assertIn("jellyfin", collector.media_clients)

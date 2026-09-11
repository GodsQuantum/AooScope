import json
import tempfile
import unittest
from pathlib import Path

from cloud9_telemetry import collect_sysfs, flatten_state, atomic_write_state


class Cloud9TelemetryTests(unittest.TestCase):
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

    def test_flatten_state_produces_asterctl_labels(self):
        flat = flatten_state({"hardware": {"cpu_temp_c": 61.5, "gpu_busy_pct": 12}, "pve": {"guests_running": 9}})
        self.assertEqual(flat["cloud9_hardware_cpu_temp_c"], "61.5")
        self.assertEqual(flat["cloud9_hardware_gpu_busy_pct"], "12")
        self.assertEqual(flat["cloud9_pve_guests_running"], "9")

    def test_atomic_write_outputs_valid_json_and_sensor_file(self):
        with tempfile.TemporaryDirectory() as td:
            sensor = Path(td, "cloud9.txt")
            state_file = Path(td, "state.json")
            state = {"hardware": {"cpu_temp_c": 61.5}}
            atomic_write_state(state, sensor, state_file)
            self.assertEqual(json.loads(state_file.read_text()), state)
            self.assertIn("cloud9_hardware_cpu_temp_c: 61.5", sensor.read_text())
            self.assertFalse(Path(str(state_file) + ".tmp").exists())


if __name__ == "__main__":
    unittest.main()

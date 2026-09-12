import unittest

from aooscope.sensor_catalog import build_sensor_catalog, sensor_exists


class SensorCatalogTests(unittest.TestCase):
    def test_flattens_live_state_to_stable_keys(self):
        catalog = build_sensor_catalog({
            "pve": {"cpu_pct": 37.5, "guests_running": 8},
            "hardware": {"gpu_busy_pct": 8, "cpu_temp_c": 54.2},
        })
        by_key = {item["key"]: item for item in catalog}
        self.assertEqual(by_key["aooscope_pve_cpu_pct"]["value"], 37.5)
        self.assertEqual(by_key["aooscope_hardware_gpu_busy_pct"]["value"], 8)
        self.assertEqual(by_key["aooscope_pve_cpu_pct"]["unit"], "%")
        self.assertEqual(by_key["aooscope_hardware_cpu_temp_c"]["unit"], "°C")
        self.assertTrue(sensor_exists("aooscope_pve_cpu_pct", catalog))

    def test_offline_provider_schema_is_still_discoverable(self):
        catalog = build_sensor_catalog({}, {
            "jellyfin": {
                "sessions_active": {"type": "number", "label": "Active sessions", "unit": ""},
                "transcodes_active": {"type": "number", "label": "Transcodes", "unit": ""},
            }
        })
        by_key = {item["key"]: item for item in catalog}
        self.assertIn("aooscope_jellyfin_sessions_active", by_key)
        self.assertIsNone(by_key["aooscope_jellyfin_sessions_active"]["value"])
        self.assertEqual(by_key["aooscope_jellyfin_sessions_active"]["group"], "jellyfin")

import json
import tempfile
import unittest
from datetime import datetime
from pathlib import Path

from aooscope.settings import (
    DEFAULT_SETTINGS,
    effective_brightness,
    load_settings,
    save_settings,
    public_settings,
)


class AooScopeSettingsTests(unittest.TestCase):
    def test_defaults_include_core_providers_and_brightness(self):
        self.assertEqual(DEFAULT_SETTINGS["display"]["brightness"], 100)
        for name in ("proxmox", "beszel", "jellyfin", "silo", "radarr", "qbittorrent"):
            self.assertIn(name, DEFAULT_SETTINGS["providers"])
    def test_schedule_crossing_midnight(self):
        settings = load_settings(None)
        settings["display"].update({
            "brightness": 100,
            "schedule_enabled": True,
            "schedule": [{"start": "22:00", "end": "08:00", "brightness": 70}],
        })
        self.assertEqual(effective_brightness(settings, datetime(2026, 9, 11, 23, 0)), 70)
        self.assertEqual(effective_brightness(settings, datetime(2026, 9, 12, 7, 30)), 70)
        self.assertEqual(effective_brightness(settings, datetime(2026, 9, 12, 12, 0)), 100)

    def test_save_splits_secrets_from_public_settings(self):
        with tempfile.TemporaryDirectory() as td:
            public = Path(td) / "settings.json"
            private = Path(td) / "private" / "providers.json"
            payload = {
                "display": {"brightness": 88},
                "providers": {
                    "jellyfin": {"enabled": True, "url": "http://media:8096", "api_key": "secret-key"}
                },
            }
            saved = save_settings(payload, public, private)
            self.assertNotIn("api_key", json.loads(public.read_text())["providers"]["jellyfin"])
            self.assertEqual(json.loads(private.read_text())["jellyfin"]["api_key"], "secret-key")
            self.assertTrue(saved["providers"]["jellyfin"]["enabled"])

    def test_public_settings_masks_secrets(self):
        settings = load_settings(None)
        settings["providers"]["radarr"].update({"enabled": True, "url": "http://radarr:7878"})
        public = public_settings(settings, {"radarr": {"api_key": "abc"}})
        self.assertTrue(public["providers"]["radarr"]["secret_set"])
        self.assertNotIn("api_key", public["providers"]["radarr"])

    def test_schedule_uses_configured_timezone(self):
        from datetime import datetime, timezone
        settings = {
            "display": {
                "brightness": 100, "schedule_enabled": True,
                "timezone": "Europe/Paris",
                "schedule": [{"start":"22:00","end":"08:00","brightness":70}],
            }
        }
        # 21:30 UTC = 23:30 in Paris during summer time.
        now = datetime(2026, 9, 11, 21, 30, tzinfo=timezone.utc)
        self.assertEqual(effective_brightness(settings, now), 70)


if __name__ == "__main__":
    unittest.main()

import tempfile
import unittest
from pathlib import Path

from webui import create_app


class AooScopeWebTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.cfg = Path(self.tmp.name)
        self.app = create_app(self.cfg, provider_tester=lambda *a, **k: {"ok": True, "message": "connected"})
        self.client = self.app.test_client()

    def tearDown(self):
        self.tmp.cleanup()

    def test_index_is_aooscope_admin(self):
        response = self.client.get("/")
        self.assertEqual(response.status_code, 200)
        text = response.get_data(as_text=True)
        self.assertIn("AooScope", text)
        self.assertIn("Brightness", text)
        self.assertIn("Providers", text)
    def test_settings_api_masks_secret_and_persists_display(self):
        response = self.client.put("/api/settings", json={
            "display": {"brightness": 88, "schedule_enabled": True},
            "providers": {"jellyfin": {"enabled": True, "url": "http://media:8096", "api_key": "secret-value"}},
        })
        self.assertEqual(response.status_code, 200)
        data = response.get_json()
        self.assertEqual(data["display"]["brightness"], 88)
        self.assertTrue(data["providers"]["jellyfin"]["secret_set"])
        self.assertNotIn("api_key", data["providers"]["jellyfin"])
        private = (self.cfg / "private" / "providers.json").read_text()
        self.assertIn("secret-value", private)

    def test_provider_test_never_returns_secret(self):
        self.client.put("/api/settings", json={
            "providers": {"radarr": {"enabled": True, "url": "http://radarr:7878", "api_key": "radarr-secret"}},
        })
        response = self.client.post("/api/providers/radarr/test")
        self.assertEqual(response.status_code, 200)
        data = response.get_json()
        self.assertTrue(data["ok"])
        self.assertNotIn("radarr-secret", response.get_data(as_text=True))

    def test_status_reports_software_brightness(self):
        self.client.put("/api/settings", json={"display": {"brightness": 77}})
        data = self.client.get("/api/status").get_json()
        self.assertEqual(data["brightness"], 77)
        self.assertFalse(data["native_brightness"])


if __name__ == "__main__":
    unittest.main()

class AooScopeExternalSecretTests(unittest.TestCase):
    def test_provider_test_uses_external_pve_secret_file(self):
        import json
        import os
        from unittest.mock import patch

        seen = {}
        def tester(name, provider, secrets):
            seen["name"] = name
            seen["secrets"] = dict(secrets)
            return {"ok": True, "message": "connected"}

        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            token = root / "pve-token.json"
            token.write_text(json.dumps({"full-tokenid": "display@pve!lcd", "value": "sekret"}))
            with patch.dict(os.environ, {"PVE_TOKEN_FILE": str(token)}, clear=False):
                app = create_app(root / "cfg", provider_tester=tester)
                client = app.test_client()
                client.put("/api/settings", json={"providers": {"proxmox": {"enabled": True, "url": "https://pve.lan:8006"}}})
                response = client.post("/api/providers/proxmox/test")
        self.assertEqual(response.status_code, 200)
        self.assertEqual(seen["name"], "proxmox")
        self.assertEqual(seen["secrets"].get("api_token"), "display@pve!lcd=sekret")
        self.assertNotIn("sekret", response.get_data(as_text=True))

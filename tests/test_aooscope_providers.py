import io
import json
import unittest
from unittest.mock import patch

from aooscope.providers import normalize_url, test_provider


class FakeResponse:
    def __init__(self, payload=b'{}', status=200, headers=None):
        self.payload = payload
        self.status = status
        self.headers = headers or {}
    def read(self, *args): return self.payload
    def __enter__(self): return self
    def __exit__(self, *args): return False


class AooScopeProviderTests(unittest.TestCase):
    def test_normalize_url_accepts_ip_port(self):
        self.assertEqual(normalize_url('192.0.2.10:8096'), 'http://192.0.2.10:8096')
        self.assertEqual(normalize_url('https://example.test/'), 'https://example.test')

    @patch('aooscope.providers.urllib.request.urlopen')
    def test_jellyfin_test_uses_key_without_returning_it(self, opener):
        opener.return_value = FakeResponse(b'[]')
        out = test_provider('jellyfin', {'url':'media:8096'}, {'api_key':'secret-key'})
        self.assertTrue(out['ok'])
        self.assertNotIn('secret-key', json.dumps(out))
        req = opener.call_args.args[0]
        self.assertEqual(req.headers['X-emby-token'], 'secret-key')

class ProxmoxCATests(unittest.TestCase):
    def test_proxmox_test_uses_mounted_ca_file(self):
        import os
        from unittest.mock import patch
        import aooscope.providers as providers

        seen = {}
        def fake_json(url, headers=None, data=None, timeout=4.0,
                      verify_tls=True, ca_file=None):
            seen["ca_file"] = ca_file
            return {"data": {"version": "9.2"}}

        with patch.dict(os.environ, {"PVE_CA_FILE": "/app/cfg/private/pve-ca.pem"}, clear=False):
            with patch.object(providers, "_json_request", side_effect=fake_json):
                result = providers.test_provider(
                    "proxmox",
                    {"url": "https://pve.lan:8006", "verify_tls": True},
                    {"api_token": "display@pve!lcd=secret"},
                )
        self.assertTrue(result["ok"])
        self.assertEqual(seen["ca_file"], "/app/cfg/private/pve-ca.pem")

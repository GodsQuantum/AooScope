import json
import tempfile
import unittest
from pathlib import Path

from aooscope.runtime_config import file_signature, load_runtime_settings


class RuntimeReloadTests(unittest.TestCase):
    def test_file_signature_changes_after_settings_write(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td, "settings.json")
            path.write_text('{"display":{"brightness":100}}')
            before = file_signature(path)
            path.write_text('{"display":{"brightness":70}}')
            after = file_signature(path)
            self.assertNotEqual(before, after)

    def test_load_runtime_settings_reads_public_and_private(self):
        with tempfile.TemporaryDirectory() as td:
            public = Path(td, "settings.json")
            private = Path(td, "private.json")
            public.write_text(json.dumps({"providers":{"jellyfin":{"enabled":True,"url":"http://media:8096"}}}))
            private.write_text(json.dumps({"jellyfin":{"api_key":"secret"}}))
            settings, secrets = load_runtime_settings(public, private)
            self.assertTrue(settings["providers"]["jellyfin"]["enabled"])
            self.assertEqual(secrets["jellyfin"]["api_key"], "secret")

import tempfile
import unittest
from pathlib import Path

from aooscope.supervisor import (
    active_media_event, event_signature, media_sensor_lines,
    desired_monitor_config,
)


class AooScopeSupervisorTests(unittest.TestCase):
    def test_active_media_event_filters_idle_and_offline(self):
        self.assertIsNone(active_media_event({"media":{"display":{"mode":"idle"}}}))
        self.assertIsNone(active_media_event({"media":{"display":{"mode":"offline"}}}))
        e = active_media_event({"media":{"display":{"mode":"incoming","title":"Dune"}}})
        self.assertEqual(e["title"], "Dune")

    def test_signature_ignores_progress_updates(self):
        a={"mode":"playing","source":"silo","title":"Film","poster_url":"x","progress_pct":1}
        b=dict(a, progress_pct=92)
        self.assertEqual(event_signature(a), event_signature(b))

    def test_media_sensor_lines_are_glanceable(self):
        e={"mode":"incoming","title":"A very very very long movie title that should be short","eta_minutes":9,"progress_pct":73.4}
        text=media_sensor_lines(e)
        self.assertIn("aooscope_media_display_headline: READY IN 9 MIN", text)
        self.assertIn("aooscope_media_display_title_short:", text)
        title=[x for x in text.splitlines() if "title_short" in x][0].split(": ",1)[1]
        self.assertLessEqual(len(title), 28)

    def test_desired_config_injects_media_before_splash(self):
        e={"mode":"playing","title":"Film"}
        cfg=desired_monitor_config(e, splash_image="branding/logo.jpg", media_image="aooscope/media-event.jpg")
        self.assertEqual([p["id"] for p in cfg["diy"]], ["media","splash","home","storage","compute"])

    def test_desired_config_idle_keeps_normal_rotation(self):
        cfg=desired_monitor_config(None, splash_image="branding/logo.jpg")
        self.assertEqual([p["id"] for p in cfg["diy"]], ["splash","home","storage","compute"])


if __name__ == "__main__":
    unittest.main()

class AooScopeDisplaySettingsTests(unittest.TestCase):
    def test_desired_config_applies_brightness_to_text(self):
        cfg = desired_monitor_config(None, brightness=50)
        text = next(s for p in cfg["diy"] for s in p["sensor"] if s["mode"] == 1)
        self.assertNotEqual(text["fontColor"], "#ffffff")

    def test_profile_signature_changes_with_brightness_not_progress(self):
        from aooscope.supervisor import profile_signature
        event = {"mode":"playing","source":"jellyfin","title":"Film","poster_url":"x","progress_pct":1}
        a = profile_signature(event, 100, "AOOSCOPE", 8, "branding/logo.jpg")
        b = profile_signature(dict(event, progress_pct=90), 100, "AOOSCOPE", 8, "branding/logo.jpg")
        c = profile_signature(event, 70, "AOOSCOPE", 8, "branding/logo.jpg")
        self.assertEqual(a, b)
        self.assertNotEqual(a, c)

class AooScopeLiveSettingsTests(unittest.TestCase):
    def test_supervisor_reads_display_settings_without_restart(self):
        import json
        import os
        from unittest.mock import patch
        from aooscope.supervisor import DisplaySupervisor
        with tempfile.TemporaryDirectory() as td:
            cfg = Path(td)
            settings = cfg / "settings.json"
            settings.write_text(json.dumps({"display":{
                "brand":"TESTBOX","brightness":73,"switch_seconds":11,
                "schedule_enabled":False}}))
            with patch.dict(os.environ, {
                "AOOSCOPE_CONFIG_DIR_IN_CONTAINER":td,
                "AOOSCOPE_SETTINGS_PATH":str(settings),
            }, clear=False):
                sup = DisplaySupervisor()
                profile = sup.current_display_profile()
            self.assertEqual(profile["brand"], "TESTBOX")
            self.assertEqual(profile["brightness"], 73)
            self.assertEqual(profile["switch_seconds"], 11)

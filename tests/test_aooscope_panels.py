import tempfile
import unittest
from pathlib import Path
from PIL import Image

from aooscope.panels import build_monitor_config, generate_backgrounds


class AooScopePanelTests(unittest.TestCase):
    def test_normal_rotation_is_three_glanceable_pages(self):
        cfg = build_monitor_config(media_active=False)
        self.assertEqual(len(cfg['diy']), 3)
        self.assertEqual([p['id'] for p in cfg['diy']], ['home', 'storage', 'compute'])
        self.assertEqual(cfg['setup']['refresh'], 1)

    def test_media_event_preempts_rotation(self):
        cfg = build_monitor_config(media_active=True, media_image='aooscope/media-event.jpg')
        self.assertEqual(cfg['diy'][0]['id'], 'media')
        self.assertEqual(cfg['diy'][0]['img'], 'aooscope/media-event.jpg')
        labels = {s['label'] for s in cfg['diy'][0]['sensor']}
        self.assertIn('aooscope_media_display_headline', labels)
        self.assertIn('aooscope_media_display_title_short', labels)

    def test_optional_splash_panel_is_first(self):
        cfg = build_monitor_config(splash_image='branding/splash.jpg')
        self.assertEqual(cfg['diy'][0]['id'], 'splash')
        self.assertEqual(cfg['diy'][0]['img'], 'branding/splash.jpg')
        self.assertEqual(cfg['diy'][0]['sensor'], [])

    def test_compute_uses_shared_gtt_not_reserved_vram(self):
        cfg = build_monitor_config()
        compute = next(p for p in cfg['diy'] if p['id'] == 'compute')
        labels = {s['label'] for s in compute['sensor']}
        self.assertIn('aooscope_hardware_gpu_gtt_pct', labels)
        self.assertNotIn('aooscope_hardware_gpu_vram_pct', labels)

    def test_storage_exposes_six_bays(self):
        cfg = build_monitor_config()
        storage = next(p for p in cfg['diy'] if p['id'] == 'storage')
        labels = {s['label'] for s in storage['sensor']}
        for index in range(6):
            self.assertIn(f'aooscope_pve_disks_{index}_name', labels)

    def test_every_sensor_has_aoostar_compat_value_field(self):
        cfg = build_monitor_config(media_active=True)
        for panel in cfg['diy']:
            for sensor in panel.get('sensor', []):
                self.assertIn('value', sensor)

    def test_generated_backgrounds_match_lcd_resolution(self):
        with tempfile.TemporaryDirectory() as td:
            paths = generate_backgrounds(td, brand='AOOSCOPE')
            self.assertEqual(set(paths), {'home', 'storage', 'compute', 'media'})
            for path in paths.values():
                with Image.open(path) as image:
                    self.assertEqual(image.size, (960, 376))


if __name__ == '__main__':
    unittest.main()

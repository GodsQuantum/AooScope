import tempfile
import unittest
from pathlib import Path
from PIL import Image

from aooscope.panels import (
    apply_config_brightness,
    build_monitor_config,
    generate_backgrounds,
    scale_hex_color,
)


class AooScopeBrightnessTests(unittest.TestCase):
    def test_text_colors_are_scaled_with_brightness(self):
        self.assertEqual(scale_hex_color("#ffffff", 70), "#b2b2b2")
        cfg = apply_config_brightness(build_monitor_config(), 50)
        text = next(s for p in cfg["diy"] for s in p["sensor"] if s["mode"] == 1)
        self.assertNotEqual(text["fontColor"], "#ffffff")

    def test_generated_assets_dim_at_lower_brightness(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            generate_backgrounds(a, brand="TEST", brightness=100)
            generate_backgrounds(b, brand="TEST", brightness=50)
            p100 = Image.open(Path(a) / "aooscope" / "home.jpg").convert("RGB")
            p50 = Image.open(Path(b) / "aooscope" / "home.jpg").convert("RGB")
            self.assertLess(sum(p50.getpixel((30, 30))), sum(p100.getpixel((30, 30))))

    def test_gauge_graphic_has_clear_label_window(self):
        with tempfile.TemporaryDirectory() as td:
            generate_backgrounds(td, brand="TEST", brightness=100)
            gauge = Image.open(Path(td) / "aooscope" / "gauge_cpu.png").convert("RGBA")
            # Top-center area is reserved for the label/icon, so the dynamic arc cannot cover text.
            samples = [gauge.getpixel((x, y))[3] for x in range(70, 141, 10) for y in range(4, 70, 10)]
            self.assertTrue(all(alpha == 0 for alpha in samples))


if __name__ == "__main__":
    unittest.main()

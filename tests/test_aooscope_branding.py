import tempfile
import unittest
from pathlib import Path
from PIL import Image

from aooscope.panels import generate_splash


class AooScopeBrandingTests(unittest.TestCase):
    def test_default_splash_is_lcd_sized_and_generic(self):
        with tempfile.TemporaryDirectory() as td:
            path = generate_splash(td, brand="AOOSCOPE", brightness=100)
            self.assertTrue(Path(path).is_file())
            with Image.open(path) as image:
                self.assertEqual(image.size, (960, 376))


if __name__ == "__main__":
    unittest.main()

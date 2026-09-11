import tempfile
import unittest
from pathlib import Path

from PIL import Image
from aooscope.render import (
    WIDTH, HEIGHT, media_headline,
    render_home, render_storage, render_compute, render_media,
)


class AooScopeRenderTests(unittest.TestCase):
    def test_media_headlines_are_short_and_glanceable(self):
        self.assertEqual(media_headline({"mode":"incoming","eta_minutes":9}), "READY IN 9 MIN")
        self.assertEqual(media_headline({"mode":"playing","progress_pct":42}), "PLAYING 42%")
        self.assertEqual(media_headline({"mode":"landed"}), "JUST LANDED")
        self.assertEqual(media_headline({"mode":"idle"}), "MEDIA READY")

    def test_all_primary_pages_are_exact_lcd_size(self):
        state={
            "hardware":{"cpu_temp_c":64,"gpu_temp_c":53,"gpu_busy_pct":37,"ram_temp_c":49},
            "pve":{"cpu_pct":22,"memory_pct":47,"guests_running":9,"guests_total":15,
                   "disks":[{"name":f"sd{x}","health":"PASSED"} for x in "abcdef"]},
        }
        for image in (render_home(state), render_storage(state), render_compute(state)):
            self.assertEqual(image.size, (WIDTH, HEIGHT))
            self.assertEqual(image.mode, "RGB")

    def test_media_page_uses_real_poster_when_available(self):
        with tempfile.TemporaryDirectory() as td:
            poster=Path(td,"poster.jpg")
            Image.new("RGB",(500,750),(80,20,20)).save(poster)
            event={"mode":"incoming","title":"Dune: Part Three","eta_minutes":12,
                   "progress_pct":73,"poster_path":str(poster)}
            image=render_media(event)
            self.assertEqual(image.size,(WIDTH,HEIGHT))
            # poster occupies left side; it must not remain the background color
            self.assertNotEqual(image.getpixel((80,180)), image.getpixel((900,20)))

    def test_storage_page_accepts_fewer_than_six_disks(self):
        image=render_storage({"pve":{"disks":[{"name":"sda","health":"PASSED"}]}})
        self.assertEqual(image.size,(WIDTH,HEIGHT))


if __name__ == "__main__":
    unittest.main()

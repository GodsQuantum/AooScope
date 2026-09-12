import io
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from aooscope.media_library import AssetInUse, InvalidMedia, MediaLibrary


def png_bytes(size=(32, 24)):
    out = io.BytesIO()
    Image.new("RGB", size, (10, 20, 30)).save(out, "PNG")
    return out.getvalue()


class MediaLibraryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.library = MediaLibrary(self.root)

    def tearDown(self):
        self.tmp.cleanup()

    def test_ingest_uses_generated_name_and_decoded_metadata(self):
        asset = self.library.ingest(io.BytesIO(png_bytes()), "../../logo.png")
        self.assertTrue(asset["id"])
        self.assertNotIn("..", asset["stored_name"])
        self.assertEqual(asset["format"], "PNG")
        self.assertEqual(asset["width"], 32)
        self.assertEqual(asset["height"], 24)
        self.assertTrue(self.library.resolve(asset["id"]).is_file())
    def test_delete_blocks_asset_in_use(self):
        asset = self.library.ingest(io.BytesIO(png_bytes()), "logo.png")
        pages = {
            "pages": {
                "p": {"layers": [{"id": "img", "type": "image", "asset_id": asset["id"]}]}
            }
        }
        with self.assertRaises(AssetInUse):
            self.library.delete(asset["id"], pages)
        self.library.delete(asset["id"], {"pages": {}})
        self.assertEqual(self.library.list(), [])

    def test_rejects_fake_image_and_too_many_pixels(self):
        with self.assertRaises(InvalidMedia):
            self.library.ingest(io.BytesIO(b"not an image"), "fake.png")
        strict = MediaLibrary(self.root / "strict", max_image_pixels=100)
        with self.assertRaises(InvalidMedia):
            strict.ingest(io.BytesIO(png_bytes((20, 20))), "large.png")

    def test_svg_rejects_script_and_external_href(self):
        unsafe = b'<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>'
        with self.assertRaises(InvalidMedia):
            self.library.ingest(io.BytesIO(unsafe), "x.svg")
        external = b'<svg xmlns="http://www.w3.org/2000/svg"><image href="https://evil.invalid/x.png"/></svg>'
        with self.assertRaises(InvalidMedia):
            self.library.ingest(io.BytesIO(external), "x.svg")
    def test_replace_keeps_asset_id_and_increments_revision(self):
        asset = self.library.ingest(io.BytesIO(png_bytes()), "logo.png")
        replaced = self.library.replace(asset["id"], io.BytesIO(png_bytes((40, 30))), "logo2.png")
        self.assertEqual(replaced["id"], asset["id"])
        self.assertEqual(replaced["revision"], asset["revision"] + 1)
        self.assertEqual((replaced["width"], replaced["height"]), (40, 30))

    def test_video_source_is_bounded_and_detected_by_content(self):
        fake_mp4 = b"\x00\x00\x00\x18ftypisom" + b"x" * 32
        small = MediaLibrary(self.root / "video", max_upload_bytes=128)
        asset = small.ingest(io.BytesIO(fake_mp4), "clip.bin")
        self.assertEqual(asset["kind"], "video")
        self.assertEqual(asset["format"], "MP4")
        with self.assertRaises(InvalidMedia):
            small.ingest(io.BytesIO(fake_mp4 + b"x" * 200), "too-big.mp4")

    def test_safe_svg_can_be_rasterized_for_page_preview(self):
        svg=b'<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60"><rect width="120" height="60" fill="#35d9ff"/></svg>'
        asset=self.library.ingest(io.BytesIO(svg),'logo.svg')
        preview=self.library.preview_path(asset['id'])
        self.assertTrue(preview.is_file())
        with Image.open(preview) as image:
            self.assertGreater(image.width,0)
            self.assertGreater(image.height,0)


if __name__ == "__main__":
    unittest.main()

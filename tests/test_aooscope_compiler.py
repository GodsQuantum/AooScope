import io
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from aooscope.media_library import MediaLibrary
from aooscope.page_compiler import compile_document, compile_page
from aooscope.factory_templates import factory_document


def make_png(size=(80, 40)):
    out = io.BytesIO()
    Image.new("RGB", size, (180, 40, 70)).save(out, "PNG")
    return out.getvalue()


class PageCompilerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.media = MediaLibrary(self.root / "cfg")
        self.out = self.root / "compiled"

    def tearDown(self):
        self.tmp.cleanup()

    def test_gauge_and_value_compile_with_text_after_graphic(self):
        page = {"id":"p","name":"CPU","enabled":True,"duration":8,
                "background":{"color":"#071019"},"layers":[
            {"id":"g","type":"gauge","binding":"aooscope_pve_cpu_pct","x":40,"y":60,"width":260,"height":240,"z":1,"min":0,"max":100,"color":"#35d9ff","opacity":1,"rotation":0},
            {"id":"v","type":"value","binding":"aooscope_pve_cpu_pct","x":90,"y":140,"width":160,"height":80,"z":2,"unit":"%","font_size":48,"color":"#ffffff","opacity":1,"rotation":0},
        ]}
        result = compile_page(page, {"pve":{"cpu_pct":42}}, self.media, self.out)
        self.assertEqual(result.background.size, (960, 376))
        self.assertEqual(result.monitor_panel["sensor"][0]["mode"], 2)
        self.assertEqual(result.monitor_panel["sensor"][-1]["label"], "aooscope_pve_cpu_pct")
        self.assertEqual(result.monitor_panel["sensor"][-1]["mode"], 1)
    def test_horizontal_and_vertical_bars_emit_direction(self):
        page = {"id":"bars","name":"Bars","enabled":True,"duration":8,
                "background":{"color":"#071019"},"layers":[
            {"id":"h","type":"bar","binding":"aooscope_pve_cpu_pct","x":20,"y":40,"width":300,"height":30,"z":1,"orientation":"horizontal","min":0,"max":100,"color":"#35d9ff","opacity":1,"rotation":0},
            {"id":"v","type":"bar","binding":"aooscope_pve_memory_pct","x":350,"y":40,"width":30,"height":200,"z":2,"orientation":"vertical","min":0,"max":100,"color":"#58e5a4","opacity":1,"rotation":0},
        ]}
        result = compile_page(page, {"pve":{"cpu_pct":20,"memory_pct":30}}, self.media, self.out)
        directions = [sensor["direction"] for sensor in result.monitor_panel["sensor"]]
        self.assertEqual(directions, [1, 4])
        for sensor in result.monitor_panel["sensor"]:
            self.assertTrue((self.out / sensor["pic"]).is_file())

    def test_image_layer_reuses_asset_with_contain(self):
        asset = self.media.ingest(io.BytesIO(make_png()), "logo.png")
        page = {"id":"img","name":"Image","enabled":True,"duration":8,
                "background":{"color":"#000000"},"layers":[
            {"id":"i","type":"image","asset_id":asset["id"],"fit":"contain","x":100,"y":100,"width":300,"height":160,"z":1,"opacity":1,"rotation":0}
        ]}
        result = compile_page(page, {}, self.media, self.out)
        self.assertEqual(result.background.getpixel((250, 180)), (180, 40, 70))

    def test_missing_sensor_uses_fallback_for_dynamic_value(self):
        page = {"id":"fallback","name":"Fallback","enabled":True,"duration":8,
                "background":{"color":"#071019"},"layers":[
            {"id":"v","type":"value","binding":"aooscope_missing_value","fallback":"--","x":20,"y":20,"width":180,"height":60,"z":1,"font_size":30,"color":"#ffffff","opacity":1,"rotation":0}
        ]}
        result = compile_page(page, {}, self.media, self.out)
        self.assertEqual(result.monitor_panel["sensor"][0]["value"], "--")
    def test_document_compiles_enabled_carousel_and_brightness(self):
        doc = factory_document()
        result = compile_document(doc, {"pve":{"cpu_pct":10,"memory_pct":20},"hardware":{"cpu_temp_c":50,"gpu_busy_pct":0,"gpu_gtt_pct":5}}, self.media, self.out, brightness=50)
        enabled = [doc["pages"][pid] for pid in doc["carousel"] if doc["pages"][pid]["enabled"]]
        self.assertEqual(len(result.monitor_config["diy"]), len(enabled))
        self.assertEqual(result.monitor_config["setup"]["switchTime"], "8")
        self.assertTrue(all(Path(path).is_file() for path in result.files))


if __name__ == "__main__":
    unittest.main()

import tempfile
import unittest
from pathlib import Path

from webui import create_app


class DesignerFrontendTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.app = create_app(Path(self.tmp.name), provider_tester=lambda *a, **k: {'ok': True})
        self.client = self.app.test_client()

    def tearDown(self):
        self.tmp.cleanup()

    def test_admin_serves_tabbed_designer_shell(self):
        html = self.client.get('/').get_data(as_text=True)
        for marker in ('id="pageList"', 'id="designerCanvas"', 'data-tab="pages"', 'data-tab="media"', 'data-tab="display"', 'data-tab="providers"'):
            self.assertIn(marker, html)
        self.assertIn('/static/admin.js', html)
        self.assertIn('/static/admin.css', html)

    def test_admin_js_exposes_page_management_functions(self):
        js = self.client.get('/static/admin.js').get_data(as_text=True)
        for fn in (
            'loadPages', 'selectPage', 'createPage', 'duplicatePage',
            'deletePage', 'restorePage', 'saveCarousel'
        ):
            self.assertTrue(
                f'function {fn}' in js or f'async function {fn}' in js,
                fn,
            )
        self.assertIn('dragstart', js)
        self.assertIn('/api/carousel', js)

    def test_admin_js_exposes_wysiwyg_functions(self):
        js = self.client.get('/static/admin.js').get_data(as_text=True)
        for fn in ('renderCanvas','addLayer','moveLayer','resizeLayer','setLayerType','setLayerZ','saveCurrentPage'):
            self.assertIn(f'function {fn}', js)
        self.assertIn("from './designer-model.js'", js)
        self.assertIn('/api/sensors', js)

    def test_existing_display_and_provider_controls_remain(self):
        html = self.client.get('/').get_data(as_text=True)
        for marker in ('id="brightness"', 'id="timezone"', 'id="providers"', 'id="saveSettings"'):
            self.assertIn(marker, html)


if __name__ == '__main__':
    unittest.main()

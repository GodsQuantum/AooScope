import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from PIL import Image
from webui import create_app

class DesignerMigrationTests(unittest.TestCase):
    def test_existing_custom_splash_becomes_media_asset_and_settings_survive(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td); (root/'branding').mkdir()
            Image.new('RGB',(960,376),(5,10,20)).save(root/'branding'/'custom.jpg')
            original={'display':{'brand':'PRIVATE','brightness':83,'timezone':'Europe/Paris'},'providers':{}}
            (root/'settings.json').write_text(json.dumps(original))
            with patch.dict(os.environ,{'AOOSCOPE_SPLASH_IMAGE':'branding/custom.jpg'},clear=False):
                client=create_app(root).test_client()
                listing=client.get('/api/pages').get_json()
            settings=json.loads((root/'settings.json').read_text())
            self.assertEqual(settings,original)
            splash_id=listing['carousel'][0]
            splash=client.get(f'/api/pages/{splash_id}').get_json()
            self.assertEqual(splash['template_id'],'factory.splash.v1')
            self.assertEqual(splash['layers'][0]['type'],'image')
            asset_id=splash['layers'][0]['asset_id']
            assets=client.get('/api/media').get_json()['assets']
            self.assertIn(asset_id,{a['id'] for a in assets})

if __name__=='__main__': unittest.main()

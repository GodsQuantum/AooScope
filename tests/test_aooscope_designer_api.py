import io
import json
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from webui import create_app


def png_bytes():
    buf = io.BytesIO()
    Image.new('RGB', (32, 16), (20, 80, 140)).save(buf, 'PNG')
    return buf.getvalue()


class DesignerApiTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.cfg = Path(self.tmp.name)
        self.app = create_app(self.cfg, provider_tester=lambda *a, **k: {'ok': True, 'message': 'connected'})
        self.client = self.app.test_client()

    def tearDown(self):
        self.tmp.cleanup()

    def test_page_crud_duplicate_restore_and_revision_conflict(self):
        listing = self.client.get('/api/pages')
        self.assertEqual(listing.status_code, 200)
        created = self.client.post('/api/pages', json={'template_id': 'factory.home.v1'})
        self.assertEqual(created.status_code, 201)
        page = created.get_json()
        pid = page['id']
        current_rev = page['revision']
        updated = self.client.put(f'/api/pages/{pid}', json={'revision': current_rev, 'name': 'CPU Wall'})
        self.assertEqual(updated.status_code, 200)
        self.assertEqual(updated.get_json()['name'], 'CPU Wall')
        stale = self.client.put(f'/api/pages/{pid}', json={'revision': current_rev, 'name': 'stale'})
        self.assertEqual(stale.status_code, 409)
        duplicate = self.client.post(f'/api/pages/{pid}/duplicate')
        self.assertEqual(duplicate.status_code, 201)
        restored = self.client.post(f'/api/pages/{pid}/restore')
        self.assertEqual(restored.status_code, 200)
        self.assertEqual(restored.get_json()['template_id'], 'factory.home.v1')

    def test_carousel_sensor_media_preview_and_apply_routes(self):
        pages = self.client.get('/api/pages').get_json()
        ids = [p['id'] for p in pages['pages']]
        order = list(reversed(ids))
        response = self.client.put('/api/carousel', json={
            'revision': pages['revision'],
            'items': [{'id': pid, 'enabled': True, 'duration': 7} for pid in order],
        })
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.get_json()['carousel'], order)
        sensors = self.client.get('/api/sensors')
        self.assertEqual(sensors.status_code, 200)
        upload = self.client.post('/api/media', data={'file': (io.BytesIO(png_bytes()), 'logo.png')}, content_type='multipart/form-data')
        self.assertEqual(upload.status_code, 201)
        asset = upload.get_json()
        page = self.client.get(f"/api/pages/{order[0]}").get_json()
        page['layers'].append({'id':'img','type':'image','asset_id':asset['id'],'x':10,'y':10,'width':120,'height':80,'z':5,'opacity':1.0,'fit':'contain'})
        preview = self.client.post('/api/preview', json={'page': page})
        self.assertEqual(preview.status_code, 200)
        self.assertEqual(preview.mimetype, 'image/png')
        applied = self.client.post('/api/apply')
        self.assertEqual(applied.status_code, 200)
        self.assertTrue(applied.get_json()['ok'])
        self.assertTrue(applied.get_json()['revision_id'])

    def test_media_delete_is_reference_protected_and_api_never_leaks_secrets(self):
        self.client.put('/api/settings', json={'providers': {'jellyfin': {
            'enabled': True, 'url': 'http://media.lan:8096', 'api_key': 'top-secret-key'}}})
        upload = self.client.post('/api/media', data={'file': (io.BytesIO(png_bytes()), 'shared.png')}, content_type='multipart/form-data')
        asset = upload.get_json()
        doc = self.client.get('/api/pages').get_json()
        page = self.client.get(f"/api/pages/{doc['carousel'][0]}").get_json()
        page['layers'].append({'id':'asset','type':'image','asset_id':asset['id'],'x':0,'y':0,'width':32,'height':16,'z':2,'opacity':1.0})
        self.client.put(f"/api/pages/{page['id']}", json=page)
        blocked = self.client.delete(f"/api/media/{asset['id']}")
        self.assertEqual(blocked.status_code, 409)
        for url in ('/api/pages', '/api/media', '/api/sensors', '/api/status'):
            response = self.client.get(url)
            self.assertNotIn('top-secret-key', response.get_data(as_text=True))


if __name__ == '__main__':
    unittest.main()

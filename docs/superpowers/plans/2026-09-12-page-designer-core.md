# AooScope Page Designer Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fixed carousel with an editable 960×376 page system, reusable media library, visual editor, preview, safe apply and automatic rollback.

**Architecture:** Persist normalized pages in `/app/cfg/pages.json` and media metadata in `/app/cfg/media.json`. Compile normalized layers into generated LCD assets plus `monitor.json`; the supervisor remains the sole owner of `asterctl` and promotes only validated revisions.

**Tech Stack:** Python 3.13, Flask, Pillow, Waitress, vanilla JavaScript/CSS, `asterctl`/`aoostar-rs`, unittest, Node `--check` in CI.

**Spec:** `docs/superpowers/specs/2026-09-12-page-designer-media-animation-design.md`

## Global Constraints

- Logical canvas is exactly 960×376 pixels.
- Existing `settings.json`, provider secrets, Cloud 9 splash and brightness schedule must survive migration unchanged.
- All page writes are atomic and revision-checked.
- Text belonging to a compound widget must render above its graphics.
- Active rendering must never depend on arbitrary HTML/JS supplied by users.
- Public Docker deployment remains pull-only and generic, with no Cloud 9/personal data in Git.
- No PVE or CT130 reboot is allowed during hardware acceptance.

---
## File structure

- `aooscope/page_store.py` — schema defaults, validation, atomic persistence, revisions, carousel mutations and migration backups.
- `aooscope/factory_templates.py` — immutable Splash/Home/Storage/Compute/Media normalized templates.
- `aooscope/media_library.py` — upload validation, metadata, references, normalization and delete protection.
- `aooscope/sensor_catalog.py` — flatten current `state.json` and registered provider fields into stable bindings.
- `aooscope/page_compiler.py` — normalized page/layer model → backgrounds, dynamic assets and `monitor.json`.
- `aooscope/revisions.py` — staging, validation, promotion, current/previous compiled revision and rollback.
- `aooscope/supervisor.py` — consume promoted revision instead of rebuilding fixed panels directly.
- `webui.py` — API only; serve static Admin assets and orchestrate page/media/apply endpoints.
- `web/index.html`, `web/admin.css`, `web/admin.js` — visual editor, page list, Media Library and existing Display/Providers UI.
- `tests/test_aooscope_pages.py`, `test_aooscope_media_library.py`, `test_aooscope_compiler.py`, `test_aooscope_revisions.py`, `test_aooscope_designer_api.py` — backend behavior.
- `tests/test_aooscope_designer_frontend.py` — static DOM/JS contracts and browser-model logic.

### Task 1: Normalized Page Store and Factory Migration

**Files:** Create `aooscope/page_store.py`, `aooscope/factory_templates.py`, `tests/test_aooscope_pages.py`; modify `aooscope/panels.py` only to expose compatibility template helpers.

**Interfaces:** Produce `PageStore(root: Path)`, `PageStore.load() -> dict`, `PageStore.save(doc, expected_revision=None) -> dict`, `PageStore.restore(page_id) -> dict`, and `factory_document(splash_asset_id=None) -> dict`.

- [ ] **Step 1: Write the failing migration/revision tests**

```python
store = PageStore(tmp_path)
doc = store.load()
assert [doc['pages'][pid]['name'] for pid in doc['carousel']][:4] == ['Splash','Home','Storage','Compute']
with self.assertRaises(RevisionConflict):
    store.save(doc, expected_revision=doc['revision'] - 1)
```
- [ ] **Step 2: Run the focused test and verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_pages -v`
Expected: import failure for `aooscope.page_store` or missing `PageStore`.

- [ ] **Step 3: Implement schema, validation, atomic writes and migration**

```python
SCHEMA_VERSION = 1
class RevisionConflict(Exception): pass
class PageValidationError(Exception): pass

class PageStore:
    def __init__(self, root):
        self.root = Path(root); self.path = self.root / 'pages.json'
    def load(self):
        if not self.path.exists():
            doc = factory_document(self._existing_splash_asset())
            self._atomic(doc); return doc
        return validate_document(json.loads(self.path.read_text()))
```

Validation must reject duplicate page/layer IDs, out-of-bounds geometry unless `clip:true`, invalid z values, unknown layer types, invalid duration and malformed colors. Before schema upgrades, write `pages.json.bak-YYYYmmdd-HHMMSS`.

- [ ] **Step 4: Verify GREEN plus existing page/panel tests**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_pages tests.test_aooscope_panels -v`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add aooscope/page_store.py aooscope/factory_templates.py aooscope/panels.py tests/test_aooscope_pages.py && git commit -m 'feat: add normalized page store and factory migration'`
### Task 2: Sensor Catalog and Binding Validation

**Files:** Create `aooscope/sensor_catalog.py`, `tests/test_aooscope_sensor_catalog.py`; modify `aooscope/telemetry.py` only to expose provider field descriptors when needed.

**Interfaces:** Produce `build_sensor_catalog(state: dict, provider_schemas: dict | None=None) -> list[dict]`, `sensor_exists(key, catalog) -> bool`, and catalog entries `{key,label,group,type,value,unit,min,max}`.

- [ ] **Step 1: Write failing catalog tests**

```python
catalog = build_sensor_catalog({'pve': {'cpu_pct': 37.5}, 'hardware': {'gpu_busy_pct': 8}})
keys = {item['key'] for item in catalog}
assert 'aooscope_pve_cpu_pct' in keys
assert 'aooscope_hardware_gpu_busy_pct' in keys
assert next(x for x in catalog if x['key']=='aooscope_pve_cpu_pct')['value'] == 37.5
```

Also assert registered-but-currently-offline provider metrics remain discoverable with `value=None` so pages can be authored while a provider is stopped.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_sensor_catalog -v`
Expected: missing module/function.

- [ ] **Step 3: Implement stable flattening and provider schemas**

```python
def stable_key(parts):
    return 'aooscope_' + '_'.join(str(p).lower().replace('-', '_') for p in parts)

def build_sensor_catalog(state, provider_schemas=None):
    # recurse dict/list state, merge schema-only keys, infer number/string/bool
    ...
```

Map known units/ranges for CPU/RAM/GPU percent, temperatures, bytes/rates, guest counts and media progress. Do not expose secrets or raw provider credentials.

- [ ] **Step 4: Verify GREEN**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_sensor_catalog tests.test_aooscope_telemetry -v`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add aooscope/sensor_catalog.py aooscope/telemetry.py tests/test_aooscope_sensor_catalog.py && git commit -m 'feat: expose designer sensor catalog'`
### Task 3: Media Library with Safe Uploads and References

**Files:** Create `aooscope/media_library.py`, `tests/test_aooscope_media_library.py`; modify `Dockerfile` only if an already-approved sanitizer/decoder dependency is required.

**Interfaces:** Produce `MediaLibrary(root: Path)`, `.list()`, `.ingest(stream, filename) -> dict`, `.replace(asset_id, stream, filename) -> dict`, `.delete(asset_id, pages_doc)`, `.resolve(asset_id) -> Path`.

- [ ] **Step 1: Write failing lifecycle/security tests**

```python
asset = library.ingest(io.BytesIO(png_bytes), '../../logo.png')
assert asset['id'] and '..' not in asset['stored_name']
assert library.resolve(asset['id']).is_file()
with self.assertRaises(AssetInUse):
    library.delete(asset['id'], {'pages': {'p': {'layers':[{'type':'image','asset_id':asset['id']}]}}})
```

Tests must reject a renamed text file, decoded images above configured pixel limits, unsupported SVG content, oversized video source and path traversal.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_media_library -v`
Expected: missing module/classes.

- [ ] **Step 3: Implement metadata and validated storage**

```python
STATIC_FORMATS = {'PNG','JPEG','WEBP','GIF'}
MAX_UPLOAD_BYTES = 64 * 1024 * 1024
MAX_IMAGE_PIXELS = 32_000_000

def generated_name(asset_id, suffix):
    return f'{asset_id}{suffix.lower()}'
```

Use Pillow decoding for raster truth, generated UUID filenames, atomic `media.json`, SHA-256, dimensions, revision and source type. Sanitize SVG to a conservative whitelist; store GIF/video as source assets for the animation plan, not direct LCD playback.

- [ ] **Step 4: Verify GREEN**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_media_library -v`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add aooscope/media_library.py tests/test_aooscope_media_library.py Dockerfile && git commit -m 'feat: add safe reusable media library'`
### Task 4: Normalized Page Compiler

**Files:** Create `aooscope/page_compiler.py`, `tests/test_aooscope_compiler.py`; modify `aooscope/panels.py` only for reusable gauge asset generation helpers.

**Interfaces:** Produce `compile_document(doc, state, media_library, output_dir, brightness=100) -> CompileResult`, `compile_page(page, ...)`, and `CompileResult.monitor_config`, `.files`, `.warnings`.

- [ ] **Step 1: Write failing compiler/z-order tests**

```python
page = {'id':'p','name':'CPU','enabled':True,'duration':8,'background':{'color':'#071019'},'layers':[
 {'id':'g','type':'gauge','binding':'aooscope_pve_cpu_pct','x':40,'y':60,'width':260,'height':240,'z':1,'min':0,'max':100},
 {'id':'v','type':'value','binding':'aooscope_pve_cpu_pct','x':90,'y':140,'width':160,'height':80,'z':2,'unit':'%'}]}
result = compile_page(page, {'pve':{'cpu_pct':42}}, media_library, tmp_path)
assert result.background.size == (960,376)
assert result.monitor_panel['sensor'][-1]['label'] == 'aooscope_pve_cpu_pct'
```

Also assert bars support horizontal/vertical orientation, images honor contain/cover, missing sensors use configured fallback, and compound gauge/value text z-order remains above visual layers.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_compiler -v`
Expected: missing compiler.

- [ ] **Step 3: Implement layer compilers**

```python
LAYER_COMPILERS = {
    'text': compile_text, 'value': compile_value, 'bar': compile_bar,
    'gauge': compile_gauge, 'ring': compile_ring, 'badge': compile_badge,
    'image': compile_image, 'sparkline': compile_sparkline,
}
```

Static portions render with Pillow in z-order. Dynamic `asterctl` sensor entries are emitted only where partial updates are valuable; value/text entries are emitted after associated graphics. Brightness is applied after composition so schedules remain independent of page definitions.

- [ ] **Step 4: Verify GREEN and golden dimensions**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_compiler tests.test_aooscope_brightness tests.test_aooscope_panels -v`
Expected: PASS and every generated raster is exactly 960×376 or the declared dynamic asset size.

- [ ] **Step 5: Commit**

`git add aooscope/page_compiler.py aooscope/panels.py tests/test_aooscope_compiler.py && git commit -m 'feat: compile editable pages for aoostar display'`
### Task 5: Compiled Revision Staging, Apply and Rollback

**Files:** Create `aooscope/revisions.py`, `tests/test_aooscope_revisions.py`; modify `aooscope/supervisor.py`.

**Interfaces:** Produce `RevisionManager(root)`, `.stage(doc, state, compiler) -> str`, `.promote(revision_id)`, `.rollback()`, `.current()`, plus supervisor method `load_promoted_revision()`.

- [ ] **Step 1: Write failing promotion/rollback tests**

```python
rid = manager.stage(doc, state, compiler)
manager.promote(rid)
assert manager.current()['revision_id'] == rid
manager.promote('broken')
manager.mark_failed('broken', 'asterctl exited')
assert manager.rollback()['revision_id'] == rid
```

Assert promotion uses an atomic pointer/file replacement and keeps exactly the configured number of previous compiled revisions.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_revisions -v`
Expected: missing `RevisionManager`.

- [ ] **Step 3: Implement staged compile directories and supervisor handshake**

```text
/app/cfg/compiled/<revision-id>/monitor.json
/app/cfg/compiled/<revision-id>/assets/...
/app/cfg/compiled/current.json
/app/cfg/compiled/previous.json
```

Supervisor loads only the promoted revision, starts `asterctl`, waits for process survival/readiness, and re-promotes previous on failure. Existing media event priority remains compatible by compiling the Media factory page as an event override rather than mutating user page definitions.

- [ ] **Step 4: Verify GREEN and supervisor regression**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_revisions tests.test_aooscope_supervisor tests.test_aooscope_runtime_reload -v`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add aooscope/revisions.py aooscope/supervisor.py tests/test_aooscope_revisions.py && git commit -m 'feat: add safe page revision apply and rollback'`
### Task 6: Designer and Media REST API

**Files:** Modify `webui.py`; create `tests/test_aooscope_designer_api.py`.

**Interfaces:** Add exactly the spec routes: `/api/pages`, `/api/pages/<id>`, duplicate, restore, `/api/carousel`, `/api/sensors`, `/api/media`, `/api/preview`, `/api/apply`.

- [ ] **Step 1: Write failing API tests**

```python
created = client.post('/api/pages', json={'template_id':'factory.home.v1'}).get_json()
page_id = created['id']
rev = created['revision']
assert client.put(f'/api/pages/{page_id}', json={'revision':rev,'name':'CPU Wall'}).status_code == 200
assert client.put(f'/api/pages/{page_id}', json={'revision':rev,'name':'stale'}).status_code == 409
```

Upload tests use multipart form data; secret-leak tests scan every JSON response for provider secret values. Preview returns an image response without altering current compiled revision. Apply returns `{ok, revision_id, warnings}`.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_designer_api -v`
Expected: 404s for designer routes.

- [ ] **Step 3: Implement routes with structured errors**

```python
@app.errorhandler(PageValidationError)
def page_error(exc):
    return jsonify({'ok':False,'error':'validation','issues':exc.issues}), 422

@app.errorhandler(RevisionConflict)
def revision_error(exc):
    return jsonify({'ok':False,'error':'revision_conflict','current_revision':exc.current}), 409
```

All endpoints instantiate stores beneath the configured root, never accept filesystem paths, never return secrets, and use `send_file` only on resolved library/compiler-owned files.

- [ ] **Step 4: Verify GREEN plus old Admin routes**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_designer_api tests.test_aooscope_web -v`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add webui.py tests/test_aooscope_designer_api.py && git commit -m 'feat: expose page designer and media APIs'`
### Task 7: Split Admin Assets and Build Page/Carousel UI

**Files:** Create `web/index.html`, `web/admin.css`, `web/admin.js`, `tests/test_aooscope_designer_frontend.py`; modify `webui.py`, `Dockerfile`, `tests/test_deployment.sh`.

**Interfaces:** Browser functions `loadPages()`, `selectPage(id)`, `createPage(templateId)`, `duplicatePage(id)`, `deletePage(id)`, `restorePage(id)`, `saveCarousel()`; existing Display/Provider behavior remains available.

- [ ] **Step 1: Write failing static/frontend contract tests**

```python
html = client.get('/').get_data(as_text=True)
assert 'id="pageList"' in html and 'id="designerCanvas"' in html
js = client.get('/static/admin.js').get_data(as_text=True)
for fn in ['loadPages','selectPage','createPage','duplicatePage','deletePage','restorePage','saveCarousel']:
    assert f'function {fn}' in js or f'async function {fn}' in js
```

Add Node syntax check for `web/admin.js` to `tests/test_deployment.sh` and CI.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_designer_frontend -v && bash tests/test_deployment.sh`
Expected: missing static assets/contracts.

- [ ] **Step 3: Move existing Admin into static files and add Pages shell**

Use tabs `Pages | Media | Display | Providers`. Page cards show drag handle, thumbnail, name, enabled toggle, duration, Duplicate/Delete/Restore. Reordering uses HTML5 drag events and persists one atomic `/api/carousel` request.

```js
async function saveCarousel(){
  const items=[...document.querySelectorAll('[data-page-id]')].map((el,i)=>({id:el.dataset.pageId,order:i,enabled:el.querySelector('.page-enabled').checked,duration:+el.querySelector('.page-duration').value}));
  return api('/api/carousel',{method:'PUT',body:JSON.stringify({items})});
}
```

- [ ] **Step 4: Verify GREEN and old Display/Provider UI contracts**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_designer_frontend tests.test_aooscope_web -v && bash tests/test_deployment.sh`
Expected: PASS and `node --check web/admin.js` succeeds.

- [ ] **Step 5: Commit**

`git add web webui.py Dockerfile tests/test_aooscope_designer_frontend.py tests/test_deployment.sh && git commit -m 'feat: add carousel page management UI'`
### Task 8: 960×376 WYSIWYG Layer Editor

**Files:** Modify `web/index.html`, `web/admin.css`, `web/admin.js`, `tests/test_aooscope_designer_frontend.py`.

**Interfaces:** Browser functions `renderCanvas(page)`, `addLayer(kind,binding?)`, `moveLayer(id,x,y)`, `resizeLayer(id,w,h)`, `setLayerType(id,type)`, `setLayerZ(id,z)`, `saveCurrentPage()`.

- [ ] **Step 1: Write failing editor-model tests/contracts**

```js
const p=normalizePage({layers:[]});
const layer=makeLayer('value','aooscope_pve_cpu_pct',120,80);
assert(layer.x===120 && layer.y===80 && layer.type==='value');
assert(clampGeometry({...layer,x:950,width:100}).x===860);
```

Expose pure editor-model helpers from `web/designer-model.js` so Node can execute behavior tests without a browser. Tests cover drag coordinates under CSS scaling, resize bounds, grid snap, z-order, keyboard nudge and widget-type conversion preserving `binding`.

- [ ] **Step 2: Verify RED**

Run: `node tests/designer_model_test.mjs`
Expected: missing `web/designer-model.js` or functions.

- [ ] **Step 3: Implement canvas interaction and inspector**

Canvas uses an inner element with logical dimensions `960px × 376px` and CSS `transform:scale(...)`; pointer positions are converted back with `logicalX=(clientX-left)/scale`. The left palette lists Sensors, Widgets and Media; dropping a sensor creates a `value` layer. Inspector edits geometry, type, typography, colors, unit, min/max, thresholds, fallback, opacity and z.

```js
function clientToLogical(ev,rect,scale){return {x:Math.round((ev.clientX-rect.left)/scale),y:Math.round((ev.clientY-rect.top)/scale)}}
function nudge(layer,dx,dy){return clampGeometry({...layer,x:layer.x+dx,y:layer.y+dy})}
```

- [ ] **Step 4: Verify GREEN**

Run: `node tests/designer_model_test.mjs && ./.venv/bin/python -m unittest tests.test_aooscope_designer_frontend -v`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add web tests/designer_model_test.mjs tests/test_aooscope_designer_frontend.py && git commit -m 'feat: add drag and drop LCD page editor'`
### Task 9: Media Library UI, Placement and Preview

**Files:** Modify `web/index.html`, `web/admin.css`, `web/admin.js`; modify `tests/test_aooscope_designer_frontend.py`, `tests/test_aooscope_designer_api.py`.

**Interfaces:** Browser functions `loadMedia()`, `uploadMedia(files)`, `placeAsset(assetId)`, `replaceAsset(assetId,file)`, `deleteAsset(assetId)`, `previewCurrentPage()`.

- [ ] **Step 1: Write failing upload/reuse/preview tests**

```python
upload = client.post('/api/media', data={'file':(io.BytesIO(png_bytes),'logo.png')}, content_type='multipart/form-data')
asset = upload.get_json()
assert upload.status_code == 201
preview = client.post('/api/preview', json={'page': page_with_asset(asset['id'])})
assert preview.status_code == 200 and preview.mimetype == 'image/png'
```

Frontend contracts assert drag of one asset can create image layers on two distinct pages without duplicating the binary.

- [ ] **Step 2: Verify RED**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_designer_api tests.test_aooscope_designer_frontend -v`
Expected: missing media-tab behavior/preview paths.

- [ ] **Step 3: Implement Media tab and image inspector**

Media cards show thumbnail, name, dimensions/type/revision, Replace, Delete and drag handle. Drop on canvas creates an `image` layer with `fit:'contain'`; inspector offers contain/cover, focal X/Y, opacity and crop clipping. GIF/video cards are marked `Animation source` and remain preview-only until the animation plan is complete.

- [ ] **Step 4: Verify GREEN**

Run: `./.venv/bin/python -m unittest tests.test_aooscope_media_library tests.test_aooscope_designer_api tests.test_aooscope_designer_frontend -v && node tests/designer_model_test.mjs`
Expected: PASS.

- [ ] **Step 5: Commit**

`git add web tests/test_aooscope_designer_api.py tests/test_aooscope_designer_frontend.py && git commit -m 'feat: add media library to page designer'`

### Task 10: Apply UX, Migration Acceptance and Public Packaging

**Files:** Modify `web/index.html`, `web/admin.js`, `README.md`, `README.fr.md`, `docs/architecture.md`, `docs/deployment.md`, `Dockerfile`, `tests/test_deployment.sh`, `tests/test_repo_hygiene.py`.

**Interfaces:** UI action `applyCarousel()` calls `/api/apply`, shows revision/warnings, and presents rollback failure details without losing draft pages.
- [ ] **Step 1: Write failing end-to-end migration/apply tests**

```python
store = PageStore(cfg)
doc = store.load()
assert any(p.get('template_id') == 'factory.home.v1' for p in doc['pages'].values())
assert json.loads((cfg/'settings.json').read_text()) == original_settings
result = client.post('/api/apply').get_json()
assert result['ok'] and result['revision_id']
```

Repository hygiene test must reject personal hostnames/domains/IPs, Cloud 9 branding assets and provider credentials from tracked files while allowing generic RFC/example values.

- [ ] **Step 2: Verify RED where integration is not wired**

Run: `./.venv/bin/python -m unittest discover -s tests -p 'test_*.py' -v && bash tests/test_deployment.sh`
Expected before final wiring: at least the apply/migration acceptance assertion fails for the new path.

- [ ] **Step 3: Complete packaging and user-facing apply state**

Docker image copies `web/`; `/app/cfg/pages.json`, `/app/cfg/media.json`, `/app/cfg/media/` and `/app/cfg/compiled/` remain persistent. Admin shows `Draft saved`, `Preview`, `Apply to LCD`, applying spinner, promoted revision and rollback error state. Docs explain that page edits are drafts until Apply.

- [ ] **Step 4: Run full software gate**

Run:
`./.venv/bin/python -m unittest discover -s tests -p 'test_*.py' -v`
`node --check web/admin.js`
`node tests/designer_model_test.mjs`
`bash tests/test_deployment.sh`
Expected: all PASS, privacy scan 0 hits.

- [ ] **Step 5: Hardware acceptance on Cloud 9 without reboot**

Build/publish the tested image, pull it in CT130, preserve current appdata, then verify through the Admin: create page → drag CPU metric → select gauge → upload/reuse image → preview → reorder carousel → Apply. Confirm `asterctl --simulate` succeeds first, container stays healthy/restarts=0, physical LCD shows the custom page, brightness schedule still works and Restore Factory returns the original page.

- [ ] **Step 6: Commit**

`git add web README.md README.fr.md docs Dockerfile tests && git commit -m 'feat: complete visual LCD page designer'`

## Plan self-review

- Spec coverage: page CRUD/order, factory restore, sensor drag/drop, all core widget types, media reuse, preview, apply/rollback, brightness, migration, security and hardware acceptance are mapped to Tasks 1–10.
- Animation/GIF/video-on-device compilation is intentionally implemented by the companion animation plan, while core Media Library stores those source assets safely.
- Type consistency: page revision is integer optimistic concurrency; compiled revision ID is an opaque string; media references use immutable asset IDs.
- Placeholder scan: implementation steps define concrete APIs, commands, expected failures/passes and file responsibilities; no deferred core behavior remains.

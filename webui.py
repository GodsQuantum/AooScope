#!/usr/bin/env python3
import copy
import io
import json
import os
import tempfile
import uuid
from pathlib import Path

from flask import Flask, jsonify, request, Response, send_file

from aooscope.factory_templates import FACTORY_TEMPLATES, factory_page
from aooscope.media_library import AssetInUse, AssetNotFound, InvalidMedia, MediaLibrary
from aooscope.page_compiler import compile_document, compile_page
from aooscope.page_store import PageStore, PageValidationError, RevisionConflict, validate_document
from aooscope.revisions import RevisionManager, RevisionError
from aooscope.sensor_catalog import build_sensor_catalog

from aooscope.settings import (
    PROVIDER_DEFAULTS,
    effective_brightness,
    load_secrets,
    load_settings,
    public_settings,
    save_settings,
)

APP_VERSION = "0.2.0"


def _json_safe_result(result, secrets):
    text = json.dumps(result or {}, ensure_ascii=False)
    for bucket in (secrets or {}).values():
        for value in (bucket or {}).values():
            if value:
                text = text.replace(str(value), "***")
    return json.loads(text)


def _external_provider_secrets():
    out = {}
    token_file = os.getenv("PVE_TOKEN_FILE")
    if token_file:
        try:
            data = json.loads(Path(token_file).read_text(encoding="utf-8"))
            if data.get("full-tokenid") and data.get("value"):
                out["proxmox"] = {"api_token": f"{data['full-tokenid']}={data['value']}"}
        except (OSError, json.JSONDecodeError):
            pass
    for name, env_name in (("jellyfin","JELLYFIN_API_KEY_FILE"),("silo","SILO_API_KEY_FILE"),("radarr","RADARR_API_KEY_FILE")):
        path = os.getenv(env_name)
        if not path:
            continue
        try:
            value = Path(path).read_text(encoding="utf-8").strip()
        except OSError:
            value = ""
        if value:
            out[name] = {"api_key": value}
    return out


def _merge_external_secrets(saved):
    merged = {k: dict(v or {}) for k, v in (saved or {}).items()}
    for name, values in _external_provider_secrets().items():
        bucket = merged.setdefault(name, {})
        for key, value in values.items():
            bucket.setdefault(key, value)
    return merged


def create_app(config_dir=None, provider_tester=None):
    root = Path(config_dir or os.getenv("AOOSCOPE_CONFIG_DIR_IN_CONTAINER", "/app/cfg"))
    settings_path = root / "settings.json"
    secrets_path = root / "private" / "providers.json"
    state_path = root / "state.json"
    device = os.getenv("AOOSCOPE_DEVICE", "/dev/ttyACM0")
    web_root = Path(__file__).with_name("web")
    app = Flask(__name__, static_folder=str(web_root), static_url_path="/static")
    media_library = MediaLibrary(root)
    splash_asset_id = None
    if not (root / "pages.json").exists():
        splash_value = os.getenv("AOOSCOPE_SPLASH_IMAGE") or ""
        splash_path = Path(splash_value) if splash_value else None
        if splash_path is not None and not splash_path.is_absolute():
            splash_path = root / splash_path
        if splash_path is not None and splash_path.is_file():
            with splash_path.open("rb") as handle:
                splash_asset_id = media_library.ingest(handle, splash_path.name)["id"]
    page_store = PageStore(root, splash_asset_id=splash_asset_id)

    def read_state():
        try:
            return json.loads(state_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return {}


    @app.after_request
    def disable_browser_cache(response):
        response.headers["Cache-Control"] = "no-store, no-cache, must-revalidate, max-age=0"
        response.headers["Pragma"] = "no-cache"
        response.headers["Expires"] = "0"
        return response

    if provider_tester is None:
        from aooscope.providers import test_provider as provider_tester

    @app.get("/")
    def index():
        return send_file(web_root / "index.html", mimetype="text/html; charset=utf-8")

    @app.get("/api/settings")
    def get_settings():
        settings = load_settings(settings_path)
        secrets = _merge_external_secrets(load_secrets(secrets_path))
        return jsonify(public_settings(settings, secrets))

    @app.put("/api/settings")
    def put_settings():
        payload = request.get_json(silent=True) or {}
        settings = save_settings(payload, settings_path, secrets_path)
        return jsonify(public_settings(settings, _merge_external_secrets(load_secrets(secrets_path))))

    @app.post("/api/providers/<name>/test")
    def test_provider_route(name):
        if name not in PROVIDER_DEFAULTS:
            return jsonify({"ok": False, "message": "Unknown provider"}), 404
        settings = load_settings(settings_path)
        secrets = _merge_external_secrets(load_secrets(secrets_path))
        provider = settings["providers"].get(name, {})
        try:
            result = provider_tester(name, provider, secrets.get(name, {}))
        except Exception as exc:
            result = {"ok": False, "message": f"{type(exc).__name__}: {exc}"}
        return jsonify(_json_safe_result(result, secrets))

    @app.get("/api/status")
    def get_status():
        settings = load_settings(settings_path)
        state = {}
        try:
            state = json.loads(state_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            pass
        return jsonify({
            "version": APP_VERSION,
            "brightness": effective_brightness(settings),
            "native_brightness": False,
            "device_present": Path(device).exists(),
            "updated_unix": ((state.get("meta") or {}).get("updated_unix")),
        })

    @app.errorhandler(PageValidationError)
    def page_validation_error(exc):
        return jsonify({"ok": False, "error": "validation", "issues": exc.issues}), 422

    @app.errorhandler(RevisionConflict)
    def page_revision_error(exc):
        return jsonify({"ok": False, "error": "revision_conflict", "current_revision": exc.current}), 409

    @app.errorhandler(InvalidMedia)
    def invalid_media_error(exc):
        return jsonify({"ok": False, "error": "invalid_media", "message": str(exc)}), 422

    @app.errorhandler(AssetInUse)
    def asset_in_use_error(exc):
        return jsonify({"ok": False, "error": "asset_in_use", "asset_id": str(exc)}), 409

    @app.errorhandler(AssetNotFound)
    def asset_not_found_error(exc):
        return jsonify({"ok": False, "error": "asset_not_found", "asset_id": str(exc)}), 404

    def page_summary(page):
        return {k: copy.deepcopy(page.get(k)) for k in (
            "id", "name", "enabled", "duration", "template_id", "revision"
        )}

    @app.get("/api/pages")
    def get_pages():
        doc = page_store.load()
        return jsonify({
            "schema_version": doc["schema_version"],
            "revision": doc["revision"],
            "carousel": list(doc["carousel"]),
            "pages": [page_summary(doc["pages"][pid]) for pid in doc["carousel"] if pid in doc["pages"]],
        })

    @app.post("/api/pages")
    def create_page():
        payload = request.get_json(silent=True) or {}
        doc = page_store.load()
        template_id = payload.get("template_id")
        if template_id:
            if template_id not in FACTORY_TEMPLATES:
                return jsonify({"ok": False, "error": "unknown_template"}), 404
            page = factory_page(template_id)
        else:
            pid = str(uuid.uuid4())
            page = {
                "id": pid, "name": str(payload.get("name") or "New page")[:80],
                "enabled": True, "duration": 8, "background": {"color": "#071019"},
                "layers": [], "template_id": None, "revision": 1,
            }
        doc["pages"][page["id"]] = page
        doc["carousel"].append(page["id"])
        saved = page_store.save(doc, expected_revision=doc["revision"])
        return jsonify(saved["pages"][page["id"]]), 201

    @app.get("/api/pages/<page_id>")
    def get_page(page_id):
        doc = page_store.load()
        page = doc["pages"].get(page_id)
        if not page:
            return jsonify({"ok": False, "error": "page_not_found"}), 404
        return jsonify(page)

    @app.put("/api/pages/<page_id>")
    def put_page(page_id):
        payload = request.get_json(silent=True) or {}
        doc = page_store.load()
        current = doc["pages"].get(page_id)
        if not current:
            return jsonify({"ok": False, "error": "page_not_found"}), 404
        expected = payload.get("revision")
        if expected != current.get("revision"):
            raise RevisionConflict(current.get("revision"))
        page = copy.deepcopy(current)
        for key, value in payload.items():
            if key not in {"id", "revision"}:
                page[key] = copy.deepcopy(value)
        page["id"] = page_id
        doc["pages"][page_id] = page
        saved = page_store.save(doc, expected_revision=doc["revision"])
        return jsonify(saved["pages"][page_id])

    @app.delete("/api/pages/<page_id>")
    def delete_page(page_id):
        doc = page_store.load()
        if page_id not in doc["pages"]:
            return jsonify({"ok": False, "error": "page_not_found"}), 404
        if len(doc["pages"]) <= 1:
            return jsonify({"ok": False, "error": "last_page"}), 409
        doc["pages"].pop(page_id)
        doc["carousel"] = [pid for pid in doc["carousel"] if pid != page_id]
        saved = page_store.save(doc, expected_revision=doc["revision"])
        return jsonify({"ok": True, "revision": saved["revision"]})

    @app.post("/api/pages/<page_id>/duplicate")
    def duplicate_page(page_id):
        doc = page_store.load()
        source = doc["pages"].get(page_id)
        if not source:
            return jsonify({"ok": False, "error": "page_not_found"}), 404
        page = copy.deepcopy(source)
        page["id"] = str(uuid.uuid4())
        page["name"] = (str(source.get("name") or "Page") + " Copy")[:80]
        page["template_id"] = source.get("template_id")
        page["revision"] = 1
        doc["pages"][page["id"]] = page
        idx = doc["carousel"].index(page_id) + 1 if page_id in doc["carousel"] else len(doc["carousel"])
        doc["carousel"].insert(idx, page["id"])
        saved = page_store.save(doc, expected_revision=doc["revision"])
        return jsonify(saved["pages"][page["id"]]), 201

    @app.post("/api/pages/<page_id>/restore")
    def restore_page(page_id):
        try:
            saved = page_store.restore(page_id)
        except KeyError:
            return jsonify({"ok": False, "error": "page_not_found"}), 404
        return jsonify(saved["pages"][page_id])

    @app.put("/api/carousel")
    def put_carousel():
        payload = request.get_json(silent=True) or {}
        doc = page_store.load()
        if payload.get("revision") != doc["revision"]:
            raise RevisionConflict(doc["revision"])
        items = payload.get("items") or []
        order = [str(item.get("id")) for item in items]
        if set(order) != set(doc["pages"]):
            raise PageValidationError(["carousel items must contain every page exactly once"])
        for item in items:
            page = doc["pages"][str(item["id"])]
            page["enabled"] = bool(item.get("enabled", page.get("enabled", True)))
            page["duration"] = int(item.get("duration", page.get("duration", 8)))
        doc["carousel"] = order
        saved = page_store.save(doc, expected_revision=doc["revision"])
        return jsonify({"ok": True, "revision": saved["revision"], "carousel": saved["carousel"]})

    @app.get("/api/sensors")
    def get_sensors():
        return jsonify({"sensors": build_sensor_catalog(read_state())})

    @app.get("/api/media")
    def get_media():
        return jsonify({"assets": media_library.list()})

    @app.post("/api/media")
    def post_media():
        upload = request.files.get("file")
        if upload is None:
            return jsonify({"ok": False, "error": "missing_file"}), 400
        asset = media_library.ingest(upload.stream, upload.filename or "asset")
        return jsonify(asset), 201

    @app.put("/api/media/<asset_id>")
    def put_media(asset_id):
        upload = request.files.get("file")
        if upload is None:
            return jsonify({"ok": False, "error": "missing_file"}), 400
        return jsonify(media_library.replace(asset_id, upload.stream, upload.filename or "asset"))

    @app.get("/api/media/<asset_id>/file")
    def media_file(asset_id):
        return send_file(media_library.resolve(asset_id), conditional=True)

    @app.delete("/api/media/<asset_id>")
    def delete_media(asset_id):
        media_library.delete(asset_id, page_store.load())
        return jsonify({"ok": True})

    @app.post("/api/preview")
    def preview_page():
        payload = request.get_json(silent=True) or {}
        page = copy.deepcopy(payload.get("page") or {})
        preview_doc = {
            "schema_version": 1,
            "revision": 1,
            "carousel": [page.get("id")],
            "pages": {str(page.get("id")): page},
        }
        validate_document(preview_doc)
        settings = load_settings(settings_path)
        with tempfile.TemporaryDirectory(prefix="aooscope-preview-") as td:
            result = compile_page(
                page, read_state(), media_library, Path(td),
                brightness=effective_brightness(settings),
            )
            buf = io.BytesIO()
            result.background.save(buf, "PNG")
            buf.seek(0)
            return send_file(buf, mimetype="image/png", download_name="preview.png")

    @app.post("/api/apply")
    def apply_pages():
        doc = page_store.load()
        settings = load_settings(settings_path)
        brightness = effective_brightness(settings)
        compiler = lambda d, state, output: compile_document(
            d, state, media_library, output, brightness=brightness
        )
        revisions = RevisionManager(root)
        rid = revisions.stage(doc, read_state(), compiler)
        revisions.promote(rid)
        manifest = json.loads((root / "compiled" / rid / "manifest.json").read_text(encoding="utf-8"))
        return jsonify({"ok": True, "revision_id": rid, "warnings": manifest.get("warnings", [])})

    @app.get("/api/health")
    def health():
        return jsonify({"ok": True, "version": APP_VERSION})

    return app


app = create_app()

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8765, debug=False)

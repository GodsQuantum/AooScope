#!/usr/bin/env python3
import copy
import json
import os
import re
import tempfile
from datetime import datetime
from pathlib import Path

from aooscope.factory_templates import (
    CANVAS_HEIGHT,
    CANVAS_WIDTH,
    FACTORY_TEMPLATES,
    factory_document,
    factory_page,
)

SCHEMA_VERSION = 1
ALLOWED_LAYER_TYPES = {
    "text", "value", "bar", "gauge", "ring", "badge", "image",
    "sparkline", "animation",
}
COLOR_RE = re.compile(r"^#[0-9a-fA-F]{6}$")


class RevisionConflict(Exception):
    def __init__(self, current):
        super().__init__(f"revision conflict; current={current}")
        self.current = current


class PageValidationError(Exception):
    def __init__(self, issues):
        self.issues = list(issues)
        super().__init__("; ".join(self.issues))

def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _validate_color(value, label, issues):
    if value is not None and (not isinstance(value, str) or not COLOR_RE.match(value)):
        issues.append(f"{label}: invalid color")


def _validate_layer(page_id, layer, seen, issues):
    lid = layer.get("id")
    if not isinstance(lid, str) or not lid:
        issues.append(f"{page_id}: layer missing id")
        return
    if lid in seen:
        issues.append(f"{page_id}: duplicate layer id {lid}")
    seen.add(lid)
    if layer.get("type") not in ALLOWED_LAYER_TYPES:
        issues.append(f"{page_id}/{lid}: unknown layer type")
    for key in ("x", "y", "width", "height", "z"):
        if not _is_number(layer.get(key)):
            issues.append(f"{page_id}/{lid}: invalid {key}")
    if any(not _is_number(layer.get(k)) for k in ("x", "y", "width", "height")):
        return
    x, y, width, height = (layer[k] for k in ("x", "y", "width", "height"))
    if width <= 0 or height <= 0:
        issues.append(f"{page_id}/{lid}: non-positive geometry")
    if not layer.get("clip") and (x < 0 or y < 0 or x + width > CANVAS_WIDTH or y + height > CANVAS_HEIGHT):
        issues.append(f"{page_id}/{lid}: geometry outside canvas")
    if _is_number(layer.get("z")) and not (-10000 <= layer["z"] <= 10000):
        issues.append(f"{page_id}/{lid}: invalid z")
    opacity = layer.get("opacity", 1.0)
    if not _is_number(opacity) or not 0 <= opacity <= 1:
        issues.append(f"{page_id}/{lid}: invalid opacity")
    for key, value in layer.items():
        if key == "color" or key.endswith("_color"):
            _validate_color(value, f"{page_id}/{lid}/{key}", issues)

def validate_document(doc):
    issues = []
    if not isinstance(doc, dict):
        raise PageValidationError(["document must be an object"])
    if doc.get("schema_version") != SCHEMA_VERSION:
        issues.append("unsupported schema_version")
    if not isinstance(doc.get("revision"), int) or doc.get("revision", 0) < 1:
        issues.append("invalid document revision")
    pages = doc.get("pages")
    carousel = doc.get("carousel")
    if not isinstance(pages, dict):
        issues.append("pages must be an object")
        pages = {}
    if not isinstance(carousel, list):
        issues.append("carousel must be an array")
        carousel = []
    if len(carousel) != len(set(carousel)):
        issues.append("carousel contains duplicate page ids")
    internal_ids = []
    for pid, page in pages.items():
        if not isinstance(page, dict):
            issues.append(f"{pid}: page must be an object")
            continue
        internal_ids.append(page.get("id"))
        if page.get("id") != pid:
            issues.append(f"{pid}: page id mismatch")
        if not isinstance(page.get("name"), str) or not page.get("name", "").strip():
            issues.append(f"{pid}: invalid name")
        duration = page.get("duration")
        if not _is_number(duration) or not 2 <= duration <= 120:
            issues.append(f"{pid}: invalid duration")
        background = page.get("background") or {}
        _validate_color(background.get("color", "#071019"), f"{pid}/background/color", issues)
        layers = page.get("layers")
        if not isinstance(layers, list):
            issues.append(f"{pid}: layers must be an array")
            continue
        seen = set()
        for layer in layers:
            if not isinstance(layer, dict):
                issues.append(f"{pid}: layer must be an object")
                continue
            _validate_layer(pid, layer, seen, issues)
    if len(internal_ids) != len(set(internal_ids)):
        issues.append("duplicate page ids")
    missing = [pid for pid in carousel if pid not in pages]
    if missing:
        issues.append("carousel references missing pages: " + ",".join(missing))
    if issues:
        raise PageValidationError(issues)
    return doc

def _atomic_json(path, payload, mode=0o644):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=path.name + ".", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2, ensure_ascii=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    finally:
        try:
            os.unlink(tmp)
        except OSError:
            pass


class PageStore:
    def __init__(self, root):
        self.root = Path(root)
        self.path = self.root / "pages.json"

    def _existing_splash_asset(self):
        return None

    def _backup(self):
        if not self.path.is_file():
            return None
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        backup = self.root / f"pages.json.bak-{stamp}"
        backup.write_bytes(self.path.read_bytes())
        return backup
    def load(self):
        if not self.path.exists():
            doc = factory_document(self._existing_splash_asset())
            validate_document(doc)
            _atomic_json(self.path, doc)
            return copy.deepcopy(doc)
        try:
            raw = json.loads(self.path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise PageValidationError([f"cannot read pages.json: {exc}"]) from exc
        if raw.get("schema_version") != SCHEMA_VERSION:
            self._backup()
            raw = factory_document(self._existing_splash_asset())
            _atomic_json(self.path, raw)
        validate_document(raw)
        return copy.deepcopy(raw)

    def save(self, doc, expected_revision=None):
        current = self.load()
        current_revision = current["revision"]
        if expected_revision is not None and expected_revision != current_revision:
            raise RevisionConflict(current_revision)
        candidate = copy.deepcopy(doc)
        candidate["schema_version"] = SCHEMA_VERSION
        candidate["revision"] = current_revision + 1
        for page in candidate.get("pages", {}).values():
            page["revision"] = int(page.get("revision", 0)) + 1
        validate_document(candidate)
        _atomic_json(self.path, candidate)
        return copy.deepcopy(candidate)

    def restore(self, page_id):
        current = self.load()
        if page_id not in current["pages"]:
            raise KeyError(page_id)
        existing = current["pages"][page_id]
        template_id = existing.get("template_id")
        if template_id not in FACTORY_TEMPLATES:
            raise PageValidationError([f"{page_id}: no factory template"])
        replacement = factory_page(template_id, page_id=page_id)
        replacement["revision"] = int(existing.get("revision", 0)) + 1
        updated = copy.deepcopy(current)
        updated["pages"][page_id] = replacement
        return self.save(updated, expected_revision=current["revision"])

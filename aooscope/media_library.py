#!/usr/bin/env python3
import hashlib
import io
import json
import os
import tempfile
import uuid
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image, UnidentifiedImageError

STATIC_FORMATS = {"PNG", "JPEG", "WEBP", "GIF"}
MAX_UPLOAD_BYTES = 64 * 1024 * 1024
MAX_IMAGE_PIXELS = 32_000_000

FORMAT_SUFFIX = {
    "PNG": ".png", "JPEG": ".jpg", "WEBP": ".webp", "GIF": ".gif",
    "SVG": ".svg", "MP4": ".mp4", "WEBM": ".webm",
}


class InvalidMedia(Exception):
    pass


class AssetInUse(Exception):
    pass


class AssetNotFound(KeyError):
    pass
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


def _read_bounded(stream, limit):
    data = stream.read(limit + 1)
    if len(data) > limit:
        raise InvalidMedia(f"upload exceeds {limit} bytes")
    if not data:
        raise InvalidMedia("empty upload")
    return data


def _local_name(tag):
    return str(tag).split("}")[-1].lower()


def _sanitize_svg(data):
    try:
        root = ET.fromstring(data)
    except ET.ParseError as exc:
        raise InvalidMedia("invalid SVG") from exc
    if _local_name(root.tag) != "svg":
        raise InvalidMedia("not an SVG")
    forbidden = {"script", "foreignobject", "iframe", "object", "embed"}
    for elem in root.iter():
        if _local_name(elem.tag) in forbidden:
            raise InvalidMedia("unsafe SVG element")
        for key, value in elem.attrib.items():
            attr = _local_name(key)
            low = str(value).strip().lower()
            if attr.startswith("on"):
                raise InvalidMedia("unsafe SVG event attribute")
            if attr in {"href", "src"} and (":" in low or low.startswith("//")):
                raise InvalidMedia("external SVG resource")
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)

def _detect_media(data, max_image_pixels):
    stripped = data.lstrip()
    if stripped.startswith(b"<svg") or stripped.startswith(b"<?xml"):
        clean = _sanitize_svg(data)
        return {
            "kind": "image", "format": "SVG", "data": clean,
            "width": None, "height": None,
        }
    if len(data) >= 12 and data[4:8] == b"ftyp":
        return {"kind": "video", "format": "MP4", "data": data, "width": None, "height": None}
    if data.startswith(b"\x1a\x45\xdf\xa3"):
        return {"kind": "video", "format": "WEBM", "data": data, "width": None, "height": None}
    try:
        with Image.open(io.BytesIO(data)) as image:
            fmt = str(image.format or "").upper()
            width, height = image.size
            if fmt not in STATIC_FORMATS:
                raise InvalidMedia(f"unsupported raster format {fmt or 'unknown'}")
            if width <= 0 or height <= 0 or width * height > max_image_pixels:
                raise InvalidMedia("image dimensions exceed configured pixel limit")
            image.verify()
    except InvalidMedia:
        raise
    except (UnidentifiedImageError, OSError, ValueError) as exc:
        raise InvalidMedia("unsupported or corrupt media") from exc
    return {"kind": "image", "format": fmt, "data": data, "width": width, "height": height}


def _asset_is_referenced(value, asset_id):
    if isinstance(value, dict):
        if value.get("asset_id") == asset_id:
            return True
        return any(_asset_is_referenced(child, asset_id) for child in value.values())
    if isinstance(value, list):
        return any(_asset_is_referenced(child, asset_id) for child in value)
    return False

class MediaLibrary:
    def __init__(self, root, max_upload_bytes=MAX_UPLOAD_BYTES, max_image_pixels=MAX_IMAGE_PIXELS):
        self.root = Path(root)
        self.media_root = self.root / "media"
        self.meta_path = self.root / "media.json"
        self.max_upload_bytes = int(max_upload_bytes)
        self.max_image_pixels = int(max_image_pixels)

    def _load_doc(self):
        if not self.meta_path.is_file():
            return {"schema_version": 1, "assets": {}}
        try:
            data = json.loads(self.meta_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise InvalidMedia(f"invalid media metadata: {exc}") from exc
        assets = data.get("assets") if isinstance(data, dict) else None
        if not isinstance(assets, dict):
            raise InvalidMedia("invalid media metadata")
        return {"schema_version": 1, "assets": assets}

    def _save_doc(self, doc):
        _atomic_json(self.meta_path, doc)

    def list(self):
        assets = self._load_doc()["assets"].values()
        return sorted((dict(asset) for asset in assets), key=lambda item: (item.get("name", ""), item["id"]))

    def _stored_name(self, asset_id, revision, fmt):
        return f"{asset_id}-r{revision}{FORMAT_SUFFIX[fmt]}"
    def _ingest_as(self, asset_id, revision, stream, filename):
        data = _read_bounded(stream, self.max_upload_bytes)
        detected = _detect_media(data, self.max_image_pixels)
        stored_name = self._stored_name(asset_id, revision, detected["format"])
        self.media_root.mkdir(parents=True, exist_ok=True)
        target = self.media_root / stored_name
        tmp = target.with_name(target.name + ".tmp")
        tmp.write_bytes(detected["data"])
        os.replace(tmp, target)
        return {
            "id": asset_id,
            "name": Path(filename or "asset").name[:120] or "asset",
            "stored_name": stored_name,
            "kind": detected["kind"],
            "format": detected["format"],
            "width": detected["width"],
            "height": detected["height"],
            "size": len(detected["data"]),
            "sha256": hashlib.sha256(detected["data"]).hexdigest(),
            "revision": revision,
        }

    def ingest(self, stream, filename):
        doc = self._load_doc()
        asset_id = str(uuid.uuid4())
        asset = self._ingest_as(asset_id, 1, stream, filename)
        doc["assets"][asset_id] = asset
        self._save_doc(doc)
        return dict(asset)

    def replace(self, asset_id, stream, filename):
        doc = self._load_doc()
        current = doc["assets"].get(asset_id)
        if not current:
            raise AssetNotFound(asset_id)
        asset = self._ingest_as(asset_id, int(current.get("revision", 1)) + 1, stream, filename)
        old_path = self.media_root / current["stored_name"]
        doc["assets"][asset_id] = asset
        self._save_doc(doc)
        if old_path != self.media_root / asset["stored_name"]:
            try:
                old_path.unlink()
            except OSError:
                pass
        return dict(asset)
    def resolve(self, asset_id):
        doc = self._load_doc()
        asset = doc["assets"].get(asset_id)
        if not asset:
            raise AssetNotFound(asset_id)
        candidate = (self.media_root / asset["stored_name"]).resolve()
        root = self.media_root.resolve()
        try:
            candidate.relative_to(root)
        except ValueError as exc:
            raise InvalidMedia("asset path escapes media root") from exc
        if not candidate.is_file():
            raise AssetNotFound(asset_id)
        return candidate

    def delete(self, asset_id, pages_doc):
        doc = self._load_doc()
        asset = doc["assets"].get(asset_id)
        if not asset:
            raise AssetNotFound(asset_id)
        if _asset_is_referenced(pages_doc or {}, asset_id):
            raise AssetInUse(asset_id)
        path = self.media_root / asset["stored_name"]
        doc["assets"].pop(asset_id, None)
        self._save_doc(doc)
        try:
            path.unlink()
        except OSError:
            pass
        return True

#!/usr/bin/env python3
import json
import os
import shutil
import tempfile
import time
import uuid
from pathlib import Path


class RevisionError(Exception):
    pass


def _atomic_json(path, payload):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix=path.name + ".", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2, ensure_ascii=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, path)
    finally:
        try:
            os.unlink(tmp)
        except OSError:
            pass


class RevisionManager:
    def __init__(self, root, keep=4):
        self.root = Path(root)
        self.compiled = self.root / "compiled"
        self.compiled.mkdir(parents=True, exist_ok=True)
        self.keep = max(2, int(keep))
        self.current_path = self.compiled / "current.json"
        self.previous_path = self.compiled / "previous.json"

    def _pointer(self, path):
        try:
            data = json.loads(Path(path).read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return None
        rid = data.get("revision_id")
        if not rid or not (self.compiled / rid).is_dir():
            return None
        return data

    def current(self):
        return self._pointer(self.current_path)

    def previous(self):
        return self._pointer(self.previous_path)

    def stage(self, doc, state, compiler):
        rid = f"r{int(time.time())}-{uuid.uuid4().hex[:8]}"
        target = self.compiled / rid
        target.mkdir(parents=True, exist_ok=False)
        try:
            result = compiler(doc, state, target)
            _atomic_json(target / "monitor.json", result.monitor_config)
            _atomic_json(target / "manifest.json", {
                "revision_id": rid,
                "page_revision": int((doc or {}).get("revision", 0)),
                "created_unix": time.time(),
                "warnings": list(result.warnings or []),
            })
        except Exception:
            shutil.rmtree(target, ignore_errors=True)
            raise
        return rid

    def promote(self, revision_id):
        revision_id = str(revision_id)
        target = self.compiled / revision_id
        if not target.is_dir() or not (target / "monitor.json").is_file():
            raise RevisionError(f"unknown compiled revision: {revision_id}")
        old = self.current()
        if old and old.get("revision_id") != revision_id:
            _atomic_json(self.previous_path, old)
        pointer = {
            "revision_id": revision_id,
            "promoted_unix": time.time(),
        }
        _atomic_json(self.current_path, pointer)
        self._prune()
        return pointer

    def mark_failed(self, revision_id, reason):
        revision_id = str(revision_id)
        failed_dir = self.compiled / revision_id
        if failed_dir.is_dir():
            _atomic_json(failed_dir / "failed.json", {
                "reason": str(reason),
                "failed_unix": time.time(),
            })
        previous = self.previous()
        if not previous:
            raise RevisionError("no previous revision available for rollback")
        _atomic_json(self.current_path, previous)
        return previous

    def rollback(self):
        previous = self.previous()
        if not previous:
            raise RevisionError("no previous revision available for rollback")
        current = self.current()
        _atomic_json(self.current_path, previous)
        if current:
            _atomic_json(self.previous_path, current)
        return previous

    def _prune(self):
        protected = {
            item.get("revision_id")
            for item in (self.current(), self.previous())
            if item
        }
        directories = [p for p in self.compiled.iterdir() if p.is_dir()]
        directories.sort(key=lambda p: p.stat().st_mtime, reverse=True)
        keep_names = set(protected)
        for path in directories:
            if len(keep_names) < self.keep:
                keep_names.add(path.name)
        for path in directories:
            if path.name not in keep_names:
                shutil.rmtree(path, ignore_errors=True)

#!/usr/bin/env python3
from pathlib import Path

from aooscope.settings import load_secrets, load_settings


def file_signature(path):
    path = Path(path)
    try:
        stat = path.stat()
        return (stat.st_mtime_ns, stat.st_size)
    except OSError:
        return None


def load_runtime_settings(public_path, private_path):
    return load_settings(public_path), load_secrets(private_path)


def combined_signature(*paths):
    return tuple(file_signature(path) for path in paths)

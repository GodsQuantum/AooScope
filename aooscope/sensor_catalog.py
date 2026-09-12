#!/usr/bin/env python3
import re

SECRET_PARTS = {"password", "passwd", "token", "api_key", "apikey", "secret", "credential"}

KNOWN_META = {
    "cpu_pct": ("CPU", "%", 0, 100),
    "memory_pct": ("Memory", "%", 0, 100),
    "gpu_busy_pct": ("GPU", "%", 0, 100),
    "gpu_gtt_pct": ("Shared GPU memory", "%", 0, 100),
    "progress_pct": ("Progress", "%", 0, 100),
    "temperature_c": ("Temperature", "°C", 0, 120),
    "cpu_temp_c": ("CPU temperature", "°C", 0, 120),
    "gpu_temp_c": ("GPU temperature", "°C", 0, 120),
    "eta_minutes": ("ETA", "min", 0, None),
    "speed_bytes_s": ("Speed", "B/s", 0, None),
    "guests_running": ("Guests running", "", 0, None),
    "guests_total": ("Guests total", "", 0, None),
}


def _slug(value):
    value = re.sub(r"[^a-zA-Z0-9]+", "_", str(value).strip().lower())
    return value.strip("_")


def stable_key(parts):
    return "aooscope_" + "_".join(_slug(part) for part in parts if str(part) != "")


def _value_type(value):
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return "number"
    return "string"

def _meta_for(path, value):
    leaf = _slug(path[-1]) if path else "value"
    label, unit, minimum, maximum = KNOWN_META.get(
        leaf,
        (leaf.replace("_", " ").title(), "", None, None),
    )
    if leaf.endswith("_pct") and not unit:
        unit, minimum, maximum = "%", 0, 100
    elif leaf.endswith("_temp_c") and not unit:
        unit, minimum, maximum = "°C", 0, 120
    return {
        "label": label,
        "unit": unit,
        "min": minimum,
        "max": maximum,
        "type": _value_type(value),
    }


def _walk(value, path, out):
    if isinstance(value, dict):
        for key, child in value.items():
            if _slug(key) in SECRET_PARTS:
                continue
            _walk(child, [*path, key], out)
        return
    if isinstance(value, list):
        for index, child in enumerate(value):
            _walk(child, [*path, index], out)
        return
    if value is None or isinstance(value, (str, int, float, bool)):
        meta = _meta_for(path, value)
        out[stable_key(path)] = {
            "key": stable_key(path),
            "label": meta["label"],
            "group": _slug(path[0]) if path else "general",
            "type": meta["type"],
            "value": value,
            "unit": meta["unit"],
            "min": meta["min"],
            "max": meta["max"],
        }

def build_sensor_catalog(state, provider_schemas=None):
    entries = {}
    _walk(state or {}, [], entries)
    for group, fields in (provider_schemas or {}).items():
        if not isinstance(fields, dict):
            continue
        for field, descriptor in fields.items():
            if _slug(field) in SECRET_PARTS:
                continue
            key = stable_key([group, field])
            if key in entries:
                continue
            descriptor = descriptor if isinstance(descriptor, dict) else {}
            entries[key] = {
                "key": key,
                "label": descriptor.get("label") or _slug(field).replace("_", " ").title(),
                "group": _slug(group),
                "type": descriptor.get("type") or "number",
                "value": None,
                "unit": descriptor.get("unit") or "",
                "min": descriptor.get("min"),
                "max": descriptor.get("max"),
            }
    return sorted(entries.values(), key=lambda item: (item["group"], item["label"], item["key"]))


def sensor_exists(key, catalog):
    return any(item.get("key") == key for item in (catalog or []))

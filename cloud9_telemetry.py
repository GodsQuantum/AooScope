#!/usr/bin/env python3
import json
import math
import os
import time
from pathlib import Path


def _read(path, default=None):
    try:
        return Path(path).read_text(encoding="utf-8").strip()
    except (OSError, UnicodeError):
        return default


def _millideg(path):
    raw = _read(path)
    try:
        return round(float(raw) / 1000.0, 1)
    except (TypeError, ValueError):
        return None


def _int(path):
    raw = _read(path)
    try:
        return int(raw)
    except (TypeError, ValueError):
        return None


def _pct(used, total):
    if used is None or total in (None, 0):
        return None
    return round(used * 100.0 / total, 1)


def collect_sysfs(sys_root="/sys"):
    root = Path(sys_root)
    hw = {"ram_temps_c": [], "block_devices": []}
    for d in sorted((root / "class/hwmon").glob("hwmon*")):
        name = _read(d / "name", "")
        temp = _millideg(d / "temp1_input")
        if name == "k10temp" and temp is not None:
            hw["cpu_temp_c"] = temp
        elif name == "amdgpu":
            if temp is not None:
                hw["gpu_temp_c"] = temp
            dev = d / "device"
            hw["gpu_busy_pct"] = _int(dev / "gpu_busy_percent")
            vram_used = _int(dev / "mem_info_vram_used")
            vram_total = _int(dev / "mem_info_vram_total")
            gtt_used = _int(dev / "mem_info_gtt_used")
            gtt_total = _int(dev / "mem_info_gtt_total")
            hw["gpu_vram_used_bytes"] = vram_used
            hw["gpu_vram_total_bytes"] = vram_total
            hw["gpu_vram_pct"] = _pct(vram_used, vram_total)
            hw["gpu_gtt_used_bytes"] = gtt_used
            hw["gpu_gtt_total_bytes"] = gtt_total
            hw["gpu_gtt_pct"] = _pct(gtt_used, gtt_total)
        elif name == "nvme" and temp is not None:
            hw["nvme_temp_c"] = temp
        elif name == "spd5118" and temp is not None:
            hw["ram_temps_c"].append(temp)

    if hw["ram_temps_c"]:
        hw["ram_temp_c"] = max(hw["ram_temps_c"])

    for d in sorted((root / "class/block").glob("*")):
        n = d.name
        if (n.startswith("sd") and len(n) == 3) or (n.startswith("nvme") and "p" not in n):
            hw["block_devices"].append(n)

    return {"hardware": hw}


def flatten_state(state):
    out = {}

    def walk(prefix, value):
        if isinstance(value, dict):
            for key, child in value.items():
                walk(f"{prefix}_{key}" if prefix else str(key), child)
        elif isinstance(value, list):
            out[prefix] = ",".join(str(v) for v in value)
        elif value is not None:
            if isinstance(value, float) and math.isfinite(value):
                out[prefix] = f"{value:g}"
            else:
                out[prefix] = str(value)

    walk("cloud9", state)
    return out


def _atomic_text(path, text):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = Path(str(path) + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    os.replace(tmp, path)


def atomic_write_state(state, sensor_path, state_path):
    flat = flatten_state(state)
    sensor_text = "".join(f"{k}: {flat[k]}\n" for k in sorted(flat))
    _atomic_text(sensor_path, sensor_text)
    _atomic_text(state_path, json.dumps(state, ensure_ascii=False, separators=(",", ":")) + "\n")


def collect_once(sys_root="/sys"):
    state = collect_sysfs(sys_root)
    state["meta"] = {"updated_unix": int(time.time())}
    return state


def main():
    interval = max(1.0, float(os.getenv("CLOUD9_REFRESH_SECONDS", "5")))
    sensor_path = os.getenv("CLOUD9_SENSOR_PATH", "/app/cfg/sensors/cloud9.txt")
    state_path = os.getenv("CLOUD9_STATE_PATH", "/app/cfg/state.json")
    while True:
        try:
            atomic_write_state(collect_once(), sensor_path, state_path)
        except Exception as exc:
            print(f"cloud9-telemetry error: {exc}", flush=True)
        time.sleep(interval)


if __name__ == "__main__":
    main()

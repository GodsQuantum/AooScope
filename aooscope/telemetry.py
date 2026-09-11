#!/usr/bin/env python3
import json
import math
import os
import time
import ssl
import urllib.parse
import urllib.request
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
            if all(isinstance(v, (dict, list)) for v in value):
                for index, child in enumerate(value):
                    walk(f"{prefix}_{index}", child)
            else:
                out[prefix] = ",".join(str(v) for v in value)
        elif value is not None:
            if isinstance(value, float) and math.isfinite(value):
                out[prefix] = f"{value:g}"
            else:
                out[prefix] = str(value)

    walk("aooscope", state)
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




class PVEClient:
    def __init__(self, base_url, token_id, token_secret, ca_file=None, timeout=3.0):
        self.base_url = base_url.rstrip("/")
        self.token_id = token_id
        self.token_secret = token_secret
        self.timeout = float(timeout)
        self.ssl_context = None
        if self.base_url.startswith("https://"):
            self.ssl_context = ssl.create_default_context(cafile=ca_file) if ca_file else ssl.create_default_context()

    def get(self, path, params=None):
        url = self.base_url + "/" + path.lstrip("/")
        if params:
            url += "?" + urllib.parse.urlencode(params)
        req = urllib.request.Request(
            url,
            headers={
                "Authorization": f"PVEAPIToken={self.token_id}={self.token_secret}",
                "Accept": "application/json",
                "User-Agent": "aooscope/0.1",
            },
        )
        kwargs = {"timeout": self.timeout}
        if self.ssl_context is not None:
            kwargs["context"] = self.ssl_context
        with urllib.request.urlopen(req, **kwargs) as response:
            payload = json.load(response)
        return payload.get("data")

def normalize_pve_status(data):
    memory = data.get("memory") or {}
    load = data.get("loadavg") or []
    return {
        "cpu_pct": round(float(data.get("cpu", 0)) * 100, 1),
        "memory_used_bytes": int(memory.get("used", 0)),
        "memory_total_bytes": int(memory.get("total", 0)),
        "memory_pct": _pct(memory.get("used"), memory.get("total")),
        "uptime_seconds": int(data.get("uptime", 0)),
        "load1": float(load[0]) if load else 0.0,
        "iowait_pct": round(float(data.get("wait", 0)) * 100, 1),
    }


def normalize_guests(lxc, qemu):
    lr = sum(1 for g in lxc if g.get("status") == "running")
    vr = sum(1 for g in qemu if g.get("status") == "running")
    return {"lxc_running": lr, "lxc_total": len(lxc), "vm_running": vr, "vm_total": len(qemu), "guests_running": lr + vr, "guests_total": len(lxc) + len(qemu)}



def collect_pve_fast(client, node):
    out = normalize_pve_status(client.get(f"/nodes/{node}/status") or {})
    lxc = client.get(f"/nodes/{node}/lxc") or []
    qemu = client.get(f"/nodes/{node}/qemu") or []
    out.update(normalize_guests(lxc, qemu))
    out["online"] = True
    return out


def collect_pve_slow(client, node):
    storage = []
    for item in client.get(f"/nodes/{node}/storage") or []:
        storage.append({
            "name": item.get("storage"),
            "type": item.get("type"),
            "active": item.get("active", 0),
            "used_bytes": item.get("used", 0),
            "total_bytes": item.get("total", 0),
            "used_pct": _pct(item.get("used"), item.get("total")),
        })
    zfs = []
    for item in client.get(f"/nodes/{node}/disks/zfs") or []:
        zfs.append({
            "name": item.get("name"),
            "health": item.get("health"),
            "used_bytes": item.get("alloc", 0),
            "total_bytes": item.get("size", 0),
            "used_pct": _pct(item.get("alloc"), item.get("size")),
            "fragmentation_pct": item.get("frag"),
        })
    disks = []
    for item in client.get(f"/nodes/{node}/disks/list") or []:
        devpath = item.get("devpath") or ""
        disks.append({
            "name": Path(devpath).name,
            "devpath": devpath,
            "model": item.get("model"),
            "size_bytes": item.get("size", 0),
            "health": item.get("health"),
            "type": item.get("type"),
            "used": item.get("used"),
        })
    return {"storage": storage, "zfs": zfs, "disks": disks}

def _raw_int(value):
    import re
    m = re.search(r"-?\d[\d,]*", str(value or ""))
    return int(m.group(0).replace(",", "")) if m else None


def normalize_smart(name, data):
    out = {"name": name, "health": data.get("health", "UNKNOWN"), "type": data.get("type", "unknown")}
    attrs = {str(a.get("id", "")).strip(): a for a in (data.get("attributes") or [])}
    if attrs:
        for key, ident in (("reallocated", "5"), ("pending", "197"), ("crc_errors", "199")):
            if ident in attrs:
                out[key] = _raw_int(attrs[ident].get("raw"))
        temp = attrs.get("194") or attrs.get("190")
        if temp:
            out["temperature_c"] = _raw_int(temp.get("raw"))
    text = data.get("text") or ""
    if text:
        import re
        patterns = {
            "temperature_c": r"(?m)^Temperature:\s*([\d,]+)",
            "power_on_hours": r"(?m)^Power On Hours:\s*([\d,]+)",
            "media_errors": r"(?m)^Media and Data Integrity Errors:\s*([\d,]+)",
        }
        for key, pattern in patterns.items():
            m = re.search(pattern, text)
            if m:
                out[key] = int(m.group(1).replace(",", ""))
    if "wearout" in data:
        out["wearout"] = data["wearout"]
    return out

def load_pve_client(base_url, token_file, ca_file=None, timeout=3.0):
    data = json.loads(Path(token_file).read_text(encoding="utf-8"))
    return PVEClient(
        base_url,
        data["full-tokenid"],
        data["value"],
        ca_file=ca_file,
        timeout=timeout,
    )


def collect_pve_smart(client, node, disks):
    rows = []
    for disk in disks or []:
        name = disk.get("name") or Path(disk.get("devpath") or "unknown").name
        devpath = disk.get("devpath")
        if not devpath:
            continue
        try:
            data = client.get(f"/nodes/{node}/disks/smart", {"disk": devpath, "healthonly": 0}) or {}
            rows.append(normalize_smart(name, data))
        except Exception:
            rows.append({"name": name, "health": "UNKNOWN", "type": disk.get("type", "unknown")})
    return rows


class RuntimeCollector:
    def __init__(self, pve_client=None, pve_node="pve", media_clients=None,
                 slow_seconds=300.0, smart_seconds=1800.0):
        self.pve_client = pve_client
        self.pve_node = pve_node
        self.media_clients = media_clients or {}
        self.slow_seconds = float(slow_seconds)
        self.smart_seconds = float(smart_seconds)
        self._slow_cache = {}
        self._smart_cache = []
        self._last_slow = -1e30
        self._last_smart = -1e30


    def collect(self, sys_root="/sys", now_monotonic=None):
        now = time.monotonic() if now_monotonic is None else float(now_monotonic)
        state = collect_sysfs(sys_root)
        if self.pve_client is not None:
            pve = {}
            try:
                pve.update(collect_pve_fast(self.pve_client, self.pve_node))
            except Exception:
                pve["online"] = False
            if now - self._last_slow >= self.slow_seconds:
                try:
                    self._slow_cache = collect_pve_slow(self.pve_client, self.pve_node)
                    self._last_slow = now
                except Exception:
                    pass
            pve.update(self._slow_cache)
            disks = self._slow_cache.get("disks", [])
            if disks and now - self._last_smart >= self.smart_seconds:
                self._smart_cache = collect_pve_smart(self.pve_client, self.pve_node, disks)
                self._last_smart = now
            if self._smart_cache:
                pve["smart"] = self._smart_cache
            state["pve"] = pve

        if self.media_clients:
            try:
                from aooscope.media import collect_media_state
                state["media"] = collect_media_state(
                    jelly_client=self.media_clients.get("jellyfin"),
                    silo_client=self.media_clients.get("silo"),
                    radarr_client=self.media_clients.get("radarr"),
                    qbit_client=self.media_clients.get("qbittorrent"),
                )
            except Exception:
                state["media"] = {"display": {"mode": "offline"}}
        state["meta"] = {"updated_unix": int(time.time())}
        return state


def _build_runtime_collector():
    pve_client = None
    token_file = os.getenv("PVE_TOKEN_FILE")
    if token_file and Path(token_file).is_file():
        try:
            pve_client = load_pve_client(
                os.getenv("PVE_API_URL", ""),
                token_file,
                ca_file=os.getenv("PVE_CA_FILE") or None,
                timeout=float(os.getenv("PVE_TIMEOUT_SECONDS", "2.5")),
            )
        except Exception as exc:
            print(f"pve client disabled: {type(exc).__name__}", flush=True)
    try:
        from aooscope.media import build_media_clients
        media_clients = build_media_clients()
    except Exception as exc:
        print(f"media clients disabled: {type(exc).__name__}", flush=True)
        media_clients = {}
    return RuntimeCollector(
        pve_client=pve_client,
        pve_node=os.getenv("PVE_NODE", "pve"),
        media_clients=media_clients,
        slow_seconds=float(os.getenv("PVE_SLOW_SECONDS", "300")),
        smart_seconds=float(os.getenv("PVE_SMART_SECONDS", "1800")),
    )


def main():
    interval = max(1.0, float(os.getenv("AOOSCOPE_REFRESH_SECONDS", "5")))
    sensor_path = os.getenv("AOOSCOPE_SENSOR_PATH", "/app/cfg/sensors/aooscope.txt")
    state_path = os.getenv("AOOSCOPE_STATE_PATH", "/app/cfg/state.json")
    collector = _build_runtime_collector()
    while True:
        try:
            atomic_write_state(collector.collect(), sensor_path, state_path)
        except Exception as exc:
            print(f"aooscope-telemetry error: {type(exc).__name__}: {exc}", flush=True)
        time.sleep(interval)


if __name__ == "__main__":
    main()

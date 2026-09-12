#!/usr/bin/env python3
import copy
import uuid

CANVAS_WIDTH = 960
CANVAS_HEIGHT = 376


def _geometry(x, y, width, height, z):
    return {
        "x": x, "y": y, "width": width, "height": height,
        "rotation": 0, "opacity": 1.0, "z": z,
    }


def _layer(kind, layer_id, x, y, width, height, z, **extra):
    layer = {"id": layer_id, "type": kind, **_geometry(x, y, width, height, z)}
    layer.update(extra)
    return layer


def _page(template_id, name, layers, enabled=True, duration=8, background="#071019"):
    return {
        "id": "", "name": name, "enabled": enabled, "duration": duration,
        "background": {"color": background}, "layers": layers,
        "template_id": template_id, "revision": 1,
    }
FACTORY_TEMPLATES = {
    "factory.splash.v1": _page("factory.splash.v1", "Splash", [
        _layer("text", "splash-brand", 80, 105, 800, 110, 2,
               text="AOOSCOPE", font_size=72, color="#f4f8fc", align="center"),
        _layer("text", "splash-subtitle", 160, 235, 640, 44, 3,
               text="SMART LCD DASHBOARD", font_size=22, color="#35d9ff", align="center"),
    ]),
    "factory.home.v1": _page("factory.home.v1", "Home", [
        _layer("gauge", "home-cpu-gauge", 40, 96, 240, 220, 1,
               binding="aooscope_pve_cpu_pct", min=0, max=100, color="#35d9ff"),
        _layer("value", "home-cpu-value", 80, 175, 160, 84, 5,
               binding="aooscope_pve_cpu_pct", unit="%", font_size=64, color="#f4f8fc"),
        _layer("gauge", "home-ram-gauge", 360, 96, 240, 220, 1,
               binding="aooscope_pve_memory_pct", min=0, max=100, color="#58e5a4"),
        _layer("value", "home-ram-value", 400, 175, 160, 84, 5,
               binding="aooscope_pve_memory_pct", unit="%", font_size=64, color="#f4f8fc"),
        _layer("gauge", "home-temp-gauge", 680, 96, 240, 220, 1,
               binding="aooscope_hardware_cpu_temp_c", min=30, max=95, color="#ffc35d"),
        _layer("value", "home-temp-value", 720, 175, 160, 84, 5,
               binding="aooscope_hardware_cpu_temp_c", unit="°C", font_size=58, color="#f4f8fc"),
    ]),
    "factory.storage.v1": _page("factory.storage.v1", "Storage", [
        _layer("text", "storage-title", 28, 24, 420, 44, 5,
               text="STORAGE HEALTH", font_size=30, color="#f4f8fc", align="left"),
        *[
            _layer("value", f"storage-temp-{i}", 40 + (i % 3) * 310,
                   105 + (i // 3) * 132, 120, 70, 4,
                   binding=f"aooscope_pve_smart_{i}_temperature_c", unit="°C",
                   font_size=34, color="#f4f8fc", fallback="--")
            for i in range(6)
        ],
        *[
            _layer("badge", f"storage-health-{i}", 165 + (i % 3) * 310,
                   118 + (i // 3) * 132, 112, 42, 3,
                   binding=f"aooscope_pve_smart_{i}_health", color="#58e5a4",
                   fallback="EMPTY")
            for i in range(6)
        ],
    ]),
    "factory.compute.v1": _page("factory.compute.v1", "Compute", [
        _layer("gauge", "compute-gpu-gauge", 40, 96, 240, 220, 1,
               binding="aooscope_hardware_gpu_busy_pct", min=0, max=100, color="#35d9ff"),
        _layer("value", "compute-gpu-value", 80, 175, 160, 84, 5,
               binding="aooscope_hardware_gpu_busy_pct", unit="%", font_size=64, color="#f4f8fc"),
        _layer("gauge", "compute-cpu-gauge", 360, 96, 240, 220, 1,
               binding="aooscope_pve_cpu_pct", min=0, max=100, color="#58e5a4"),
        _layer("value", "compute-cpu-value", 400, 175, 160, 84, 5,
               binding="aooscope_pve_cpu_pct", unit="%", font_size=64, color="#f4f8fc"),
        _layer("gauge", "compute-gtt-gauge", 680, 96, 240, 220, 1,
               binding="aooscope_hardware_gpu_gtt_pct", min=0, max=100, color="#c078ff"),
        _layer("value", "compute-gtt-value", 720, 175, 160, 84, 5,
               binding="aooscope_hardware_gpu_gtt_pct", unit="%", font_size=64, color="#f4f8fc"),
    ]),
    "factory.media.v1": _page("factory.media.v1", "Media", [
        _layer("text", "media-headline", 320, 58, 590, 48, 6,
               binding="aooscope_media_display_headline", font_size=28,
               color="#35d9ff", fallback="MEDIA READY"),
        _layer("text", "media-title", 320, 125, 590, 84, 6,
               binding="aooscope_media_display_title_short", font_size=40,
               color="#f4f8fc", fallback="Ready for media"),
        _layer("bar", "media-progress", 320, 232, 570, 24, 2,
               binding="aooscope_media_display_progress_pct", orientation="horizontal",
               min=0, max=100, color="#35d9ff", fallback=0),
        _layer("value", "media-progress-value", 820, 266, 90, 38, 6,
               binding="aooscope_media_display_progress_pct", unit="%",
               font_size=22, color="#93a8bc", fallback="--"),
    ], enabled=False),
}

FACTORY_ORDER = [
    "factory.splash.v1", "factory.home.v1", "factory.storage.v1",
    "factory.compute.v1", "factory.media.v1",
]


def factory_page(template_id, page_id=None, splash_asset_id=None):
    if template_id not in FACTORY_TEMPLATES:
        raise KeyError(template_id)
    page = copy.deepcopy(FACTORY_TEMPLATES[template_id])
    page["id"] = page_id or str(uuid.uuid4())
    if template_id == "factory.splash.v1" and splash_asset_id:
        page["layers"] = [
            _layer("image", "splash-image", 0, 0, CANVAS_WIDTH, CANVAS_HEIGHT, 1,
                   asset_id=splash_asset_id, fit="contain")
        ]
    return page

def factory_document(splash_asset_id=None):
    pages = {}
    carousel = []
    for template_id in FACTORY_ORDER:
        page = factory_page(template_id, splash_asset_id=splash_asset_id)
        pages[page["id"]] = page
        carousel.append(page["id"])
    return {
        "schema_version": 1,
        "revision": 1,
        "carousel": carousel,
        "pages": pages,
    }

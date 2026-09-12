#!/usr/bin/env python3
import math
from dataclasses import dataclass, field
from functools import reduce
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageOps

from aooscope.panels import scale_hex_color
from aooscope.render import WIDTH, HEIGHT, _font
from aooscope.sensor_catalog import build_sensor_catalog


@dataclass
class PageCompileResult:
    background: Image.Image
    monitor_panel: dict
    files: list = field(default_factory=list)
    warnings: list = field(default_factory=list)


@dataclass
class CompileResult:
    monitor_config: dict
    files: list
    warnings: list


def _rgb(value, default=(7, 16, 25)):
    if isinstance(value, str) and len(value) == 7 and value.startswith("#"):
        try:
            return tuple(int(value[i:i + 2], 16) for i in (1, 3, 5))
        except ValueError:
            pass
    return default


def _values(state):
    return {item["key"]: item.get("value") for item in build_sensor_catalog(state or {})}

def _clamp_pct(value):
    try:
        return max(0, min(100, int(value)))
    except (TypeError, ValueError):
        return 100


def _dim_image(image, brightness):
    pct = _clamp_pct(brightness)
    if pct == 100:
        return image.copy()
    if image.mode == "RGBA":
        rgb = ImageEnhance.Brightness(image.convert("RGB")).enhance(pct / 100.0)
        out = rgb.convert("RGBA")
        out.putalpha(image.getchannel("A"))
        return out
    return ImageEnhance.Brightness(image).enhance(pct / 100.0)


def _value_for(layer, values):
    binding = layer.get("binding")
    value = values.get(binding) if binding else None
    if value is None:
        return layer.get("fallback", "--")
    return value


def _format_value(value, layer):
    if isinstance(value, float):
        decimals = int(layer.get("decimals", 0))
        return f"{value:.{decimals}f}"
    return str(value)


def _sensor_base(layer, value, brightness):
    binding = layer.get("binding") or layer.get("id")
    align = layer.get("align", "center")
    x = int(layer.get("x", 0))
    width = int(layer.get("width", 0))
    if align == "left":
        anchor_x, text_align = x, "left"
    elif align == "right":
        anchor_x, text_align = x + width, "right"
    else:
        anchor_x, text_align = x + width // 2, "center"
    color = scale_hex_color(layer.get("color", "#ffffff"), brightness)
    return {
        "label": binding, "name": binding, "mode": 1, "type": 1,
        "x": anchor_x, "y": int(layer.get("y", 0) + layer.get("height", 0) / 2),
        "width": 0, "height": 0, "fontFamily": "HarmonyOS_Sans_SC_Bold",
        "fontSize": int(layer.get("font_size", 28)), "fontColor": color,
        "fontWeight": "bold", "textAlign": text_align, "textDirection": 0,
        "integerDigits": -1, "decimalDigits": int(layer.get("decimals", 0)),
        "unit": str(layer.get("unit", "")), "value": _format_value(value, layer),
        "pic": "", "direction": 1, "minValue": layer.get("min", 0),
        "maxValue": layer.get("max", 100), "minAngle": 0, "maxAngle": 180,
        "xz_x": 0, "xz_y": 0,
    }

def _save_dynamic_asset(output_dir, layer, brightness):
    output_dir = Path(output_dir)
    assets = output_dir / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    kind = layer["type"]
    width = max(8, int(layer.get("width", 40)))
    height = max(8, int(layer.get("height", 40)))
    color = _rgb(scale_hex_color(layer.get("color", "#35d9ff"), brightness), (53, 217, 255))
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    if kind in {"gauge", "ring"}:
        stroke = max(5, min(width, height) // 9)
        box = (stroke // 2 + 1, stroke // 2 + 1, width - stroke // 2 - 2, height - stroke // 2 - 2)
        if kind == "gauge":
            draw.arc(box, 150, 390, fill=color + (255,), width=stroke)
        else:
            draw.ellipse(box, outline=color + (255,), width=stroke)
    elif kind == "bar":
        draw.rounded_rectangle((0, 0, width - 1, height - 1), radius=max(1, min(width, height) // 3), fill=color + (255,))
    name = f"layer-{layer['id']}.png"
    path = assets / name
    image.save(path)
    return path, f"assets/{name}"


def _visual_sensor(layer, relative_pic, brightness):
    value = layer.get("fallback", 0)
    sensor = _sensor_base(layer, value, brightness)
    kind = layer["type"]
    sensor["pic"] = relative_pic
    sensor["minValue"] = layer.get("min", 0)
    sensor["maxValue"] = layer.get("max", 100)
    if kind in {"gauge", "ring"}:
        sensor["mode"] = 2
        sensor["x"] = int(layer["x"] + layer["width"] / 2)
        sensor["y"] = int(layer["y"] + layer["height"] / 2)
        sensor["minAngle"] = -115 if kind == "gauge" else 0
        sensor["maxAngle"] = 115 if kind == "gauge" else 360
    else:
        sensor["mode"] = 3
        sensor["x"] = int(layer["x"])
        sensor["y"] = int(layer["y"])
        sensor["direction"] = 4 if layer.get("orientation") == "vertical" else 1
    return sensor

def _paste_image(canvas, layer, media_library, warnings):
    try:
        source_path = media_library.resolve(layer["asset_id"])
        with Image.open(source_path) as source:
            image = source.convert("RGBA")
    except Exception as exc:
        warnings.append(f"{layer['id']}: media unavailable: {exc}")
        return
    size = (int(layer["width"]), int(layer["height"]))
    fit = layer.get("fit", "contain")
    if fit == "cover":
        rendered = ImageOps.fit(image, size, method=Image.Resampling.LANCZOS)
    else:
        rendered = ImageOps.contain(image, size, method=Image.Resampling.LANCZOS)
        frame = Image.new("RGBA", size, (0, 0, 0, 0))
        frame.alpha_composite(rendered, ((size[0] - rendered.width) // 2, (size[1] - rendered.height) // 2))
        rendered = frame
    opacity = max(0.0, min(1.0, float(layer.get("opacity", 1))))
    if opacity < 1:
        alpha = rendered.getchannel("A").point(lambda value: int(value * opacity))
        rendered.putalpha(alpha)
    angle = float(layer.get("rotation", 0) or 0)
    if angle:
        rendered = rendered.rotate(-angle, resample=Image.Resampling.BICUBIC, expand=False)
    canvas.paste(rendered, (int(layer["x"]), int(layer["y"])), rendered)


def _draw_text(canvas, layer, text, brightness):
    draw = ImageDraw.Draw(canvas)
    color = _rgb(scale_hex_color(layer.get("color", "#ffffff"), brightness), (255, 255, 255))
    size = int(layer.get("font_size", 28))
    align = layer.get("align", "left")
    x = int(layer["x"])
    if align == "center":
        x += int(layer["width"] / 2)
        anchor = "ma"
    elif align == "right":
        x += int(layer["width"])
        anchor = "ra"
    else:
        anchor = "la"
    y = int(layer["y"])
    draw.text((x, y), str(text), font=_font(size, True), fill=color, anchor=anchor)

def _draw_badge(canvas, layer):
    draw = ImageDraw.Draw(canvas)
    box = (int(layer["x"]), int(layer["y"]), int(layer["x"] + layer["width"]), int(layer["y"] + layer["height"]))
    fill = _rgb(layer.get("background_color", "#132433"), (19, 36, 51))
    draw.rounded_rectangle(box, radius=max(4, min(int(layer["height"]) // 3, 18)), fill=fill)


def _draw_sparkline(canvas, layer, warnings):
    series = layer.get("series") or []
    numbers = [float(v) for v in series if isinstance(v, (int, float))]
    if len(numbers) < 2:
        warnings.append(f"{layer['id']}: sparkline has no history")
        return
    low, high = min(numbers), max(numbers)
    span = high - low or 1.0
    x0, y0 = float(layer["x"]), float(layer["y"])
    width, height = float(layer["width"]), float(layer["height"])
    points = []
    for index, value in enumerate(numbers):
        x = x0 + width * index / max(1, len(numbers) - 1)
        y = y0 + height - height * (value - low) / span
        points.append((x, y))
    ImageDraw.Draw(canvas).line(points, fill=_rgb(layer.get("color", "#35d9ff")), width=max(1, int(layer.get("stroke", 3))))


def compile_page(page, state, media_library, output_dir, brightness=100):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    values = _values(state)
    background = Image.new("RGB", (WIDTH, HEIGHT), _rgb((page.get("background") or {}).get("color")))
    sensors, files, warnings = [], [], []
    dynamic_z = []
    static_z = []
    for layer in sorted(page.get("layers") or [], key=lambda item: (item.get("z", 0), item.get("id", ""))):
        kind = layer.get("type")
        binding = layer.get("binding")
        if kind == "image":
            _paste_image(background, layer, media_library, warnings)
            static_z.append(layer.get("z", 0))
        elif kind == "animation":
            if layer.get("asset_id"):
                _paste_image(background, layer, media_library, warnings)
            warnings.append(f"{layer['id']}: animation rendered as static source frame")
            static_z.append(layer.get("z", 0))
        elif kind == "sparkline":
            _draw_sparkline(background, layer, warnings)
            static_z.append(layer.get("z", 0))
        elif kind == "badge":
            _draw_badge(background, layer)
            if binding:
                sensor = _sensor_base(layer, _value_for(layer, values), brightness)
                sensors.append((layer.get("z", 0), sensor))
                dynamic_z.append(layer.get("z", 0))
            else:
                _draw_text(background, layer, layer.get("text", ""), 100)
                static_z.append(layer.get("z", 0))
        elif kind in {"text", "value"}:
            if binding:
                sensor = _sensor_base(layer, _value_for(layer, values), brightness)
                sensors.append((layer.get("z", 0), sensor))
                dynamic_z.append(layer.get("z", 0))
            else:
                text = layer.get("text", layer.get("value", ""))
                _draw_text(background, layer, text, 100)
                static_z.append(layer.get("z", 0))
        elif kind in {"gauge", "ring", "bar"}:
            asset_path, relative_pic = _save_dynamic_asset(output_dir, layer, brightness)
            sensor = _visual_sensor(layer, relative_pic, brightness)
            sensor["value"] = _format_value(_value_for(layer, values), layer)
            sensors.append((layer.get("z", 0), sensor))
            files.append(asset_path)
            dynamic_z.append(layer.get("z", 0))
        else:
            warnings.append(f"{layer.get('id','?')}: unsupported layer type {kind}")

    if dynamic_z and static_z and max(static_z) > min(dynamic_z):
        warnings.append("static layers above dynamic layers are flattened below asterctl overlays")
    background = _dim_image(background, brightness).convert("RGB")
    safe_id = "".join(ch if ch.isalnum() or ch in "-_" else "_" for ch in str(page.get("id", "page")))
    assets = output_dir / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    background_path = assets / f"page-{safe_id}.jpg"
    background.save(background_path, "JPEG", quality=96, optimize=True)
    files.insert(0, background_path)
    ordered_sensors = [sensor for _, sensor in sorted(sensors, key=lambda item: item[0])]
    panel = {
        "id": str(page.get("id")),
        "type": 1,
        "img": f"assets/{background_path.name}",
        "sensor": ordered_sensors,
    }
    return PageCompileResult(background=background, monitor_panel=panel, files=files, warnings=warnings)

def _switch_schedule(pages):
    durations = [max(2, int(round(float(page.get("duration", 8))))) for page in pages]
    if not durations:
        return 8, []
    base = reduce(math.gcd, durations)
    base = max(2, min(120, base))
    order = []
    for index, duration in enumerate(durations, start=1):
        repeat = max(1, int(round(duration / base)))
        order.extend([index] * repeat)
    return base, order


def compile_document(doc, state, media_library, output_dir, brightness=100):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    pages = []
    for page_id in doc.get("carousel", []):
        page = (doc.get("pages") or {}).get(page_id)
        if page and page.get("enabled", True):
            pages.append(page)
    panels, files, warnings = [], [], []
    for page in pages:
        result = compile_page(page, state, media_library, output_dir, brightness=brightness)
        panels.append(result.monitor_panel)
        files.extend(result.files)
        warnings.extend(result.warnings)
    switch_seconds, order = _switch_schedule(pages)
    config = {
        "setup": {"switchTime": str(switch_seconds), "refresh": 1},
        "mianban": order,
        "diy": panels,
    }
    return CompileResult(monitor_config=config, files=files, warnings=warnings)

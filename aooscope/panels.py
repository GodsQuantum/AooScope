#!/usr/bin/env python3
from pathlib import Path
from PIL import Image, ImageDraw

from aooscope.render import WIDTH, HEIGHT, BG, CARD, CARD_2, FG, MUTED, CYAN, GREEN, AMBER, _font


def _sensor(label, x, y, size, unit='', align='center', color='#f4f8fc', decimals=0):
    return {
        'label': label, 'name': label, 'mode': 1, 'type': 1,
        'x': x, 'y': y, 'width': 0, 'height': 0,
        'fontFamily': 'HarmonyOS_Sans_SC_Bold', 'fontSize': size,
        'fontColor': color, 'fontWeight': 'normal',
        'textAlign': align, 'textDirection': 0,
        'integerDigits': -1, 'decimalDigits': decimals, 'unit': unit,
        'value': '0', 'pic': '', 'direction': 1,
        'minValue': 0, 'maxValue': 100, 'minAngle': 0, 'maxAngle': 180,
        'xz_x': 0, 'xz_y': 0,
    }


def _panel(panel_id, image, sensors):
    return {'id': panel_id, 'type': 1, 'img': image, 'sensor': sensors}


def _home_panel():
    return _panel('home', 'aooscope/home.jpg', [
        _sensor('aooscope_pve_cpu_pct', 140, 222, 58, '%'),
        _sensor('aooscope_pve_memory_pct', 370, 222, 58, '%'),
        _sensor('aooscope_hardware_cpu_temp_c', 600, 222, 58, '°C'),
        _sensor('aooscope_pve_guests_running', 828, 222, 58),
    ])


def _storage_panel():
    sensors = []
    centers = [102, 253, 404, 555, 706, 857]
    for index, x in enumerate(centers):
        sensors.extend([
            _sensor(f'aooscope_pve_disks_{index}_name', x, 252, 16),
            _sensor(f'aooscope_pve_smart_{index}_health', x, 288, 12, color='#50e19c'),
            _sensor(f'aooscope_pve_smart_{index}_temperature_c', x, 318, 15, '°C'),
        ])
    return _panel('storage', 'aooscope/storage.jpg', sensors)


def _compute_panel():
    return _panel('compute', 'aooscope/compute.jpg', [
        _sensor('aooscope_hardware_gpu_busy_pct', 168, 222, 66, '%'),
        _sensor('aooscope_pve_cpu_pct', 456, 222, 66, '%'),
        _sensor('aooscope_hardware_cpu_temp_c', 874, 183, 26, '°C', align='right'),
        _sensor('aooscope_hardware_gpu_temp_c', 874, 236, 26, '°C', align='right'),
        _sensor('aooscope_hardware_gpu_gtt_pct', 874, 289, 26, '%', align='right'),
    ])


def _media_panel(image):
    return _panel('media', image, [
        _sensor('aooscope_media_display_headline', 330, 80, 28, align='left', color='#26d6ff'),
        _sensor('aooscope_media_display_title_short', 330, 154, 38, align='left'),
        _sensor('aooscope_media_display_progress_pct', 330, 244, 24, '%', align='left'),
        _sensor('aooscope_media_display_eta_minutes', 900, 244, 22, ' min', align='right'),
    ])


def build_monitor_config(media_active=False, media_image='aooscope/media.jpg', switch_seconds=7, splash_image=None):
    normal = [_home_panel(), _storage_panel(), _compute_panel()]
    panels = [_media_panel(media_image)] + normal if media_active else normal
    if splash_image:
        panels = [_panel('splash', splash_image, [])] + panels
    return {
        'setup': {'switchTime': str(float(switch_seconds)).rstrip('0').rstrip('.'), 'refresh': 1},
        'mianban': list(range(1, len(panels) + 1)),
        'diy': panels,
    }


def _new_page():
    return Image.new('RGB', (WIDTH, HEIGHT), BG)


def _round(draw, box, fill=CARD, radius=22, outline=(31, 43, 56), width=1):
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def _text(draw, xy, text, size, fill=FG, anchor='la'):
    draw.text(xy, str(text), font=_font(size, True), fill=fill, anchor=anchor)


def _header(draw, brand, section):
    _text(draw, (34, 34), brand, 29)
    draw.rounded_rectangle((34, 72, 112, 78), radius=3, fill=CYAN)
    _text(draw, (132, 75), section, 15, MUTED, anchor='lm')


def _background_home(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _header(d, brand, 'SYSTEM OVERVIEW')
    labels = ['CPU', 'RAM', 'TEMP', 'GUESTS']
    boxes = [(34,112,246,332),(266,112,478,332),(498,112,710,332),(730,112,926,332)]
    for label, box in zip(labels, boxes):
        _round(d, box)
        _text(d, (box[0]+18, box[1]+24), label, 16, MUTED)
        d.rounded_rectangle((box[0]+18, box[3]-42, box[2]-18, box[3]-25), radius=8, fill=(27,38,50))
    return img


def _background_storage(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _header(d, brand, 'STORAGE')
    x0, gap, bay_w = 34, 14, 137
    for index in range(6):
        x = x0 + index * (bay_w + gap)
        _round(d, (x,108,x+bay_w,334))
        _text(d, (x+16,132), f'BAY {index+1}', 14, MUTED)
        d.rounded_rectangle((x+16,166,x+121,216), radius=9, fill=(9,13,19), outline=CYAN, width=1)
        _text(d, (x+68,191), '●', 21, CYAN, anchor='mm')
    return img


def _background_compute(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _header(d, brand, 'COMPUTE')
    boxes = [(34,108,302,330),(322,108,590,330),(610,108,926,330)]
    for box in boxes:
        _round(d, box)
    _text(d, (58,140), 'GPU', 17, MUTED)
    _text(d, (346,140), 'CPU', 17, MUTED)
    _text(d, (634,140), 'THERMALS', 17, MUTED)
    _text(d, (650,185), 'CPU', 15, MUTED)
    _text(d, (650,238), 'GPU', 15, MUTED)
    _text(d, (650,291), 'SHARED', 15, MUTED)
    d.rounded_rectangle((58,284,278,302), radius=9, fill=(27,38,50))
    d.rounded_rectangle((346,284,566,302), radius=9, fill=(27,38,50))
    return img


def _background_media(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _round(d, (28,24,274,350), fill=CARD_2)
    _text(d, (151,165), '◈', 78, CYAN, anchor='mm')
    _text(d, (151,250), brand, 20, MUTED, anchor='mm')
    _round(d, (298,24,932,350))
    d.rounded_rectangle((330,214,894,234), radius=10, fill=(27,38,50))
    return img


def generate_backgrounds(output_dir, brand='AOOSCOPE'):
    root = Path(output_dir) / 'aooscope'
    root.mkdir(parents=True, exist_ok=True)
    makers = {
        'home': _background_home,
        'storage': _background_storage,
        'compute': _background_compute,
        'media': _background_media,
    }
    paths = {}
    for name, maker in makers.items():
        path = root / f'{name}.jpg'
        maker(brand).save(path, 'JPEG', quality=93, optimize=True)
        paths[name] = path
    return paths

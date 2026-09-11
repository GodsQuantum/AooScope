#!/usr/bin/env python3
from pathlib import Path
import copy
from PIL import Image, ImageDraw, ImageEnhance

from aooscope.render import WIDTH, HEIGHT, FG, MUTED, CYAN, GREEN, AMBER, RED, _font

BG = (10, 18, 28)
TRACK = (45, 59, 76)
CARD = (18, 31, 46)
CARD_HI = (24, 42, 60)


def _sensor(label, x, y, size, unit='', align='center', color='#ffffff', decimals=0):
    return {
        'label': label, 'name': label, 'mode': 1, 'type': 1,
        'x': x, 'y': y, 'width': 0, 'height': 0,
        'fontFamily': 'HarmonyOS_Sans_SC_Bold', 'fontSize': size,
        'fontColor': color, 'fontWeight': 'bold',
        'textAlign': align, 'textDirection': 0,
        'integerDigits': -1, 'decimalDigits': decimals, 'unit': unit,
        'value': '0', 'pic': '', 'direction': 1,
        'minValue': 0, 'maxValue': 100, 'minAngle': 0, 'maxAngle': 180,
        'xz_x': 0, 'xz_y': 0,
    }


def _visual(label, mode, x, y, pic, min_value=0, max_value=100,
            min_angle=-115, max_angle=115, direction=1):
    s = _sensor(label, x, y, 1)
    s.update({
        'mode': mode, 'pic': pic, 'direction': direction,
        'minValue': min_value, 'maxValue': max_value,
        'minAngle': min_angle, 'maxAngle': max_angle,
    })
    return s


def _panel(panel_id, image, sensors):
    return {'id': panel_id, 'type': 1, 'img': image, 'sensor': sensors}


def _home_panel():
    gauges = [
        _visual('aooscope_pve_cpu_pct', 2, 155, 209, 'aooscope/gauge_cpu.png'),
        _visual('aooscope_pve_memory_pct', 2, 480, 209, 'aooscope/gauge_ram.png'),
        _visual('aooscope_hardware_cpu_temp_c', 2, 805, 209, 'aooscope/gauge_temp.png', 30, 95),
    ]
    values = [
        _sensor('aooscope_pve_cpu_pct', 155, 224, 72, '%'),
        _sensor('aooscope_pve_memory_pct', 480, 224, 72, '%'),
        _sensor('aooscope_hardware_cpu_temp_c', 805, 224, 72, '°'),
        _sensor('aooscope_pve_guests_running', 922, 50, 30, align='right'),
    ]
    return _panel('home', 'aooscope/home.jpg', gauges + values)


def _storage_panel():
    sensors = []
    xs = [28, 337, 646]
    ys = [104, 236]
    for index in range(6):
        col, row = index % 3, index // 3
        x, y = xs[col], ys[row]
        sensors.extend([
            _visual(f'aooscope_pve_smart_{index}_temperature_c', 3,
                    x + 18, y + 24, 'aooscope/temp_bar.png', 20, 60,
                    direction=4),
            _sensor(f'aooscope_pve_disks_{index}_name', x + 66, y + 46, 20,
                    align='left'),
            _sensor(f'aooscope_pve_smart_{index}_temperature_c', x + 70, y + 88,
                    38, '°C', align='left'),
            _sensor(f'aooscope_pve_smart_{index}_health', x + 280, y + 88, 15,
                    align='right', color='#76ffb0'),
        ])
    return _panel('storage', 'aooscope/storage.jpg', sensors)


def _compute_panel():
    gauges = [
        _visual('aooscope_hardware_gpu_busy_pct', 2, 155, 209, 'aooscope/gauge_gpu.png'),
        _visual('aooscope_pve_cpu_pct', 2, 480, 209, 'aooscope/gauge_cpu.png'),
        _visual('aooscope_hardware_gpu_gtt_pct', 2, 805, 209, 'aooscope/gauge_shared.png'),
    ]
    values = [
        _sensor('aooscope_hardware_gpu_busy_pct', 155, 224, 72, '%'),
        _sensor('aooscope_pve_cpu_pct', 480, 224, 72, '%'),
        _sensor('aooscope_hardware_gpu_gtt_pct', 805, 224, 72, '%'),
        _sensor('aooscope_hardware_gpu_temp_c', 248, 326, 22, '°C', align='right'),
        _sensor('aooscope_hardware_cpu_temp_c', 573, 326, 22, '°C', align='right'),
    ]
    return _panel('compute', 'aooscope/compute.jpg', gauges + values)


def _media_panel(image):
    return _panel('media', image, [
        _sensor('aooscope_media_display_headline', 330, 80, 30, align='left', color='#36e1ff'),
        _sensor('aooscope_media_display_title_short', 330, 154, 42, align='left'),
        _sensor('aooscope_media_display_progress_pct', 330, 244, 30, '%', align='left'),
        _sensor('aooscope_media_display_eta_minutes', 900, 244, 26, ' min', align='right'),
    ])


def build_monitor_config(media_active=False, media_image='aooscope/media.jpg',
                         switch_seconds=8, splash_image=None, brightness=100):
    normal = [_home_panel(), _storage_panel(), _compute_panel()]
    panels = [_media_panel(media_image)] + normal if media_active else normal
    if splash_image:
        panels = [_panel('splash', splash_image, [])] + panels
    cfg = {
        'setup': {'switchTime': str(float(switch_seconds)).rstrip('0').rstrip('.'), 'refresh': 1},
        'mianban': list(range(1, len(panels) + 1)),
        'diy': panels,
    }
    return apply_config_brightness(cfg, brightness)



def _clamp_brightness(value):
    try:
        return max(0, min(100, int(value)))
    except (TypeError, ValueError):
        return 100


def _brightness_image(image, brightness):
    pct = _clamp_brightness(brightness)
    if pct == 100:
        return image.copy()
    if image.mode == 'RGBA':
        rgb = ImageEnhance.Brightness(image.convert('RGB')).enhance(pct / 100.0)
        out = rgb.convert('RGBA')
        out.putalpha(image.getchannel('A'))
        return out
    return ImageEnhance.Brightness(image).enhance(pct / 100.0)


def scale_hex_color(value, brightness):
    if not isinstance(value, str) or not value.startswith('#') or len(value) != 7:
        return value
    pct = _clamp_brightness(brightness) / 100.0
    try:
        channels = [int(value[i:i+2], 16) for i in (1, 3, 5)]
    except ValueError:
        return value
    channels = [max(0, min(255, int(c * pct))) for c in channels]
    return '#' + ''.join(f'{c:02x}' for c in channels)


def apply_config_brightness(config, brightness):
    pct = _clamp_brightness(brightness)
    cfg = copy.deepcopy(config)
    if pct == 100:
        return cfg
    for panel in cfg.get('diy', []):
        for sensor in panel.get('sensor', []):
            if sensor.get('mode') == 1 and isinstance(sensor.get('fontColor'), str):
                sensor['fontColor'] = scale_hex_color(sensor['fontColor'], pct)
    return cfg

def _new_page():
    return Image.new('RGB', (WIDTH, HEIGHT), BG)


def _text(draw, xy, text, size, fill=FG, anchor='la'):
    draw.text(xy, str(text), font=_font(size, True), fill=fill, anchor=anchor)


def _header(draw, brand, section):
    _text(draw, (28, 28), brand, 31, fill=(255,255,255))
    draw.rounded_rectangle((28, 70, 148, 80), radius=5, fill=CYAN)
    _text(draw, (166, 76), section, 17, fill=(205,220,236), anchor='lm')


def _draw_metric_icon(draw, kind, cx, cy, color):
    if kind == 'cpu':
        draw.rounded_rectangle((cx-21, cy-21, cx+21, cy+21), radius=7, outline=color, width=5)
        for o in (-12, 0, 12):
            draw.line((cx-31, cy+o, cx-23, cy+o), fill=color, width=4)
            draw.line((cx+23, cy+o, cx+31, cy+o), fill=color, width=4)
    elif kind == 'ram':
        draw.rounded_rectangle((cx-31, cy-15, cx+31, cy+15), radius=6, outline=color, width=5)
        for o in (-17, 0, 17):
            draw.rectangle((cx+o-6, cy-7, cx+o+6, cy+7), fill=color)
    elif kind == 'temp':
        draw.ellipse((cx-13, cy+10, cx+13, cy+36), fill=color)
        draw.rounded_rectangle((cx-7, cy-30, cx+7, cy+22), radius=7, outline=color, width=5)


def _gauge_track(draw, x, y, accent, label, icon):
    box=(x, y, x+210, y+210)
    # Leave a physical gap around the label/icon. aoostar-rs composites dynamic
    # graphics above text, so the safest generic z-order is no overlap at all.
    draw.arc(box, 155, 252, fill=TRACK, width=30)
    draw.arc(box, 288, 385, fill=TRACK, width=30)
    _draw_metric_icon(draw, icon, x+105, y+57, accent)
    _text(draw, (x+105, y+29), label, 19, fill=(230,240,250), anchor='mm')


def _background_home(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _header(d, brand, 'AT A GLANCE')
    _text(d, (848, 30), 'GUESTS', 14, fill=(185,205,224))
    for x, color, label, icon in [
        (50, CYAN, 'CPU', 'cpu'), (375, GREEN, 'RAM', 'ram'), (700, AMBER, 'TEMP', 'temp')]:
        _gauge_track(d, x, 104, color, label, icon)
    _text(d, (155, 334), 'LOAD', 15, fill=(180,200,220), anchor='mm')
    _text(d, (480, 334), 'MEMORY', 15, fill=(180,200,220), anchor='mm')
    _text(d, (805, 334), 'THERMAL', 15, fill=(180,200,220), anchor='mm')
    return img


def _background_storage(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _header(d, brand, 'STORAGE HEALTH')
    xs = [28, 337, 646]; ys = [104, 236]
    for index in range(6):
        col, row = index % 3, index // 3
        x, y = xs[col], ys[row]
        d.rounded_rectangle((x, y, x+286, y+116), radius=18, fill=CARD,
                            outline=(62,82,104), width=2)
        d.rounded_rectangle((x+14, y+18, x+44, y+98), radius=12,
                            fill=(35,48,62), outline=(80,100,122), width=2)
        _text(d, (x+63, y+18), f'BAY {index+1}', 14, fill=(174,197,220))
        _text(d, (x+268, y+18), 'SMART', 12, fill=(174,197,220), anchor='ra')
    return img


def _background_compute(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    _header(d, brand, 'COMPUTE')
    for x, color, label, icon in [
        (50, CYAN, 'GPU', 'cpu'), (375, GREEN, 'CPU', 'cpu'),
        (700, (192,120,255), 'SHARED', 'ram')]:
        _gauge_track(d, x, 104, color, label, icon)
    _text(d, (155, 334), 'GPU TEMP', 15, fill=(180,200,220), anchor='mm')
    _text(d, (480, 334), 'CPU TEMP', 15, fill=(180,200,220), anchor='mm')
    _text(d, (805, 334), 'UMA / GTT', 15, fill=(180,200,220), anchor='mm')
    return img


def _background_media(brand):
    img = _new_page(); d = ImageDraw.Draw(img)
    d.rounded_rectangle((22,18,286,358), radius=24, fill=(24,40,58),
                        outline=(78,102,128), width=2)
    _text(d, (154, 168), '▶', 88, fill=CYAN, anchor='mm')
    _text(d, (154, 270), brand, 24, fill=(220,235,248), anchor='mm')
    d.rounded_rectangle((306,18,938,358), radius=24, fill=CARD_HI,
                        outline=(78,102,128), width=2)
    d.rounded_rectangle((330,214,902,240), radius=13, fill=(46,62,80))
    return img


def _mix(a, b, t):
    return tuple(round(a[i] + (b[i]-a[i]) * t) for i in range(3))


def _palette(stops, t):
    t=max(0.0,min(1.0,t)); n=len(stops)-1
    pos=t*n; idx=min(n-1, int(pos)); local=pos-idx
    return _mix(stops[idx], stops[idx+1], local)


def _make_arc(path, stops):
    img=Image.new('RGBA',(210,210),(0,0,0,0)); d=ImageDraw.Draw(img)
    start,end=155,385
    for angle in range(start,end,2):
        # Keep the same top-center gap as the static track so labels and icons
        # always stay visually in the foreground.
        if 252 <= angle <= 288:
            continue
        t=(angle-start)/(end-start)
        color=_palette(stops,t)+(255,)
        d.arc((8,8,202,202), angle, angle+3, fill=color, width=32)
    # Hard alpha window: visual layers are composited after text by aoostar-rs.
    # Clearing the label/icon zone guarantees text remains in the foreground.
    d.rectangle((58, 0, 152, 78), fill=(0, 0, 0, 0))
    img.save(path)


def _make_temp_bar(path):
    img=Image.new('RGBA',(30,80),(0,0,0,0)); d=ImageDraw.Draw(img)
    stops=[(55,150,255),(40,225,255),(86,240,150),(255,198,70),(255,72,86)]
    for y in range(80):
        t=1-(y/79)
        color=_palette(stops,t)+(255,)
        d.rounded_rectangle((2,y,27,y+2),radius=3,fill=color)
    img.save(path)


def generate_backgrounds(output_dir, brand='AOOSCOPE', brightness=100):
    root = Path(output_dir) / 'aooscope'
    root.mkdir(parents=True, exist_ok=True)
    makers = {'home': _background_home, 'storage': _background_storage,
              'compute': _background_compute, 'media': _background_media}
    paths = {}
    for name, maker in makers.items():
        path = root / f'{name}.jpg'
        image = _brightness_image(maker(brand), brightness)
        image.save(path, 'JPEG', quality=96, optimize=True)
        paths[name] = path
    _make_arc(root/'gauge_cpu.png', [CYAN, GREEN, AMBER, RED])
    _make_arc(root/'gauge_ram.png', [GREEN, GREEN, AMBER, RED])
    _make_arc(root/'gauge_temp.png', [(55,150,255), CYAN, AMBER, RED])
    _make_arc(root/'gauge_gpu.png', [CYAN, (120,120,255), AMBER, RED])
    _make_arc(root/'gauge_shared.png', [(160,100,255), (230,100,255), AMBER, RED])
    _make_temp_bar(root/'temp_bar.png')
    if int(brightness) != 100:
        for asset in root.glob('*.png'):
            with Image.open(asset) as src:
                _brightness_image(src.convert('RGBA'), brightness).save(asset)
    return paths


def generate_splash(output_dir, brand='AOOSCOPE', brightness=100):
    root = Path(output_dir) / 'branding'
    root.mkdir(parents=True, exist_ok=True)
    image = Image.new('RGB', (WIDTH, HEIGHT), (6, 12, 20))
    draw = ImageDraw.Draw(image)
    center = (480, 150)
    draw.ellipse((238, 78, 722, 230), outline=(24, 215, 255), width=9)
    draw.ellipse((286, 92, 674, 220), outline=(30, 132, 255), width=6)
    # compact three-bay server mark
    for x, h in ((360, 106), (421, 130), (482, 94)):
        y = 171 - h // 2
        draw.rounded_rectangle((x, y, x + 46, y + h), radius=8,
                               fill=(14, 27, 41), outline=(238, 248, 255), width=5)
        for yy in range(y + 18, y + h - 10, 23):
            draw.rectangle((x + 11, yy, x + 35, yy + 7), fill=(25, 216, 255))
    draw.ellipse((540, 92, 602, 154), outline=(25, 216, 255), width=8)
    _text(draw, center, '◉', 92, fill=(245, 250, 255), anchor='mm')
    _text(draw, (480, 292), brand, 62, fill=(245, 250, 255), anchor='mm')
    draw.rounded_rectangle((390, 325, 570, 333), radius=4, fill=(25, 216, 255))
    image = _brightness_image(image, brightness)
    path = root / 'aooscope-splash.jpg'
    image.save(path, 'JPEG', quality=96, optimize=True)
    return path

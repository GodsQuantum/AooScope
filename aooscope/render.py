#!/usr/bin/env python3
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageOps

WIDTH, HEIGHT = 960, 376
BG = (6, 9, 14)
CARD = (14, 20, 29)
CARD_2 = (19, 27, 38)
FG = (244, 248, 252)
MUTED = (139, 153, 170)
CYAN = (38, 214, 255)
GREEN = (80, 225, 156)
AMBER = (255, 188, 72)
RED = (255, 88, 104)

FONT_DIRS = (Path('/app/fonts'), Path('fonts'))


def _font(size, bold=True):
    names = ['HarmonyOS_Sans_SC_Bold.ttf', 'DejaVuSans.ttf'] if bold else ['DejaVuSans.ttf', 'HarmonyOS_Sans_SC_Bold.ttf']
    for base in FONT_DIRS:
        for name in names:
            p = base / name
            if p.exists():
                return ImageFont.truetype(str(p), size)
    return ImageFont.load_default()


def _canvas():
    return Image.new('RGB', (WIDTH, HEIGHT), BG)


def _round(draw, box, fill=CARD, radius=22, outline=None, width=1):
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def _text(draw, xy, text, size, fill=FG, anchor='la', bold=True):
    draw.text(xy, str(text), font=_font(size, bold), fill=fill, anchor=anchor)


def _bar(draw, box, pct, accent=CYAN):
    x0, y0, x1, y1 = box
    pct = max(0.0, min(100.0, float(pct or 0)))
    _round(draw, box, fill=(29, 38, 50), radius=(y1-y0)//2)
    if pct > 0:
        fill_x = x0 + max(y1-y0, int((x1-x0) * pct / 100.0))
        _round(draw, (x0, y0, min(fill_x, x1), y1), fill=accent, radius=(y1-y0)//2)


def _status_color(ok=True, warn=False):
    if not ok:
        return RED
    return AMBER if warn else GREEN


def media_headline(event):
    mode = (event or {}).get('mode', 'idle')
    if mode == 'incoming':
        eta = event.get('eta_minutes')
        return f'READY IN {eta} MIN' if eta else 'INCOMING'
    if mode == 'playing':
        progress = event.get('progress_pct')
        return f'PLAYING {int(round(progress))}%' if progress is not None else 'PLAYING'
    if mode == 'landed':
        return 'JUST LANDED'
    return 'MEDIA READY'


def _header(draw, title, subtitle=None, accent=CYAN):
    _text(draw, (34, 34), title, 30, FG, bold=True)
    draw.rounded_rectangle((34, 72, 112, 78), radius=3, fill=accent)
    if subtitle:
        _text(draw, (132, 75), subtitle, 16, MUTED, anchor='lm', bold=False)


def render_home(state, brand='AOOSCOPE'):
    img = _canvas()
    d = ImageDraw.Draw(img)
    hw = (state or {}).get('hardware', {})
    pve = (state or {}).get('pve', {})
    _header(d, brand, 'SYSTEM OVERVIEW')
    cpu = pve.get('cpu_pct', 0)
    ram = pve.get('memory_pct', 0)
    temp = hw.get('cpu_temp_c')
    guests = pve.get('guests_running')
    total = pve.get('guests_total')
    cards = [(34, 112, 246, 332), (266, 112, 478, 332), (498, 112, 710, 332), (730, 112, 926, 332)]
    labels = [('CPU', f'{cpu:.0f}%'), ('RAM', f'{ram:.0f}%'), ('TEMP', f'{temp:.0f}°' if temp is not None else '--'), ('GUESTS', f'{guests or 0}/{total or 0}')]
    accents = [CYAN, GREEN, AMBER if (temp or 0) >= 75 else CYAN, GREEN]
    for box, (label, value), accent in zip(cards, labels, accents):
        _round(d, box, fill=CARD, outline=(27, 38, 51), width=1)
        _text(d, (box[0]+18, box[1]+24), label, 17, MUTED)
        _text(d, ((box[0]+box[2])//2, box[1]+115), value, 56, FG, anchor='mm')
        _bar(d, (box[0]+18, box[3]-38, box[2]-18, box[3]-24), cpu if label == 'CPU' else ram if label == 'RAM' else 100, accent)
    return img


def render_storage(state):
    img = _canvas()
    d = ImageDraw.Draw(img)
    pve = (state or {}).get('pve', {})
    disks = list(pve.get('disks') or [])[:6]
    _header(d, 'STORAGE', '6-BAY STATUS')
    x0, gap, bay_w, bay_h = 34, 14, 137, 222
    for i in range(6):
        x = x0 + i * (bay_w + gap)
        box = (x, 108, x+bay_w, 330)
        disk = disks[i] if i < len(disks) else None
        health = str((disk or {}).get('health') or '').upper()
        ok = health in ('PASSED', 'ONLINE', 'OK')
        color = _status_color(ok=ok if disk else True)
        _round(d, box, fill=CARD, outline=(31, 43, 56), width=1)
        _text(d, (x+16, 132), f'BAY {i+1}', 15, MUTED)
        d.rounded_rectangle((x+16, 168, x+121, 218), radius=9, fill=(9,13,19), outline=color, width=2)
        if disk:
            _text(d, (x+68, 193), '●', 25, color, anchor='mm')
            name = (disk.get('name') or f'DISK {i+1}').upper()
            _text(d, (x+68, 252), name[:10], 19, FG, anchor='mm')
            _text(d, (x+68, 286), health[:8] or 'ONLINE', 14, color, anchor='mm')
        else:
            _text(d, (x+68, 193), '—', 25, MUTED, anchor='mm')
            _text(d, (x+68, 268), 'EMPTY', 15, MUTED, anchor='mm')
    return img


def render_compute(state):
    img = _canvas()
    d = ImageDraw.Draw(img)
    hw = (state or {}).get('hardware', {})
    pve = (state or {}).get('pve', {})
    _header(d, 'COMPUTE', 'RYZEN + RADEON')
    gpu = hw.get('gpu_busy_pct') or 0
    cpu = pve.get('cpu_pct') or 0
    gpu_t = hw.get('gpu_temp_c')
    cpu_t = hw.get('cpu_temp_c')
    vram = hw.get('gpu_gtt_pct')
    cards = [(34,108,302,330),(322,108,590,330),(610,108,926,330)]
    for box in cards:
        _round(d, box, fill=CARD, outline=(31,43,56), width=1)
    _text(d, (58,140), 'GPU', 18, MUTED)
    _text(d, (168,210), f'{gpu:.0f}%', 72, FG, anchor='mm')
    _bar(d, (58,284,278,302), gpu, CYAN)
    _text(d, (346,140), 'CPU', 18, MUTED)
    _text(d, (456,210), f'{cpu:.0f}%', 72, FG, anchor='mm')
    _bar(d, (346,284,566,302), cpu, GREEN)
    _text(d, (634,140), 'THERMALS', 18, MUTED)
    _text(d, (650,195), 'CPU', 16, MUTED)
    _text(d, (900,195), f'{cpu_t:.0f}°C' if cpu_t is not None else '--', 30, FG, anchor='ra')
    _text(d, (650,240), 'GPU', 16, MUTED)
    _text(d, (900,240), f'{gpu_t:.0f}°C' if gpu_t is not None else '--', 30, FG, anchor='ra')
    _text(d, (650,286), 'SHARED', 16, MUTED)
    _text(d, (900,286), f'{vram:.0f}%' if vram is not None else '--', 30, FG, anchor='ra')
    return img


def _poster_image(path, size=(246, 326)):
    if not path or not Path(path).exists():
        return None
    with Image.open(path) as src:
        return ImageOps.fit(src.convert('RGB'), size, method=Image.Resampling.LANCZOS)


def render_media(event, brand='MEDIA'):
    event = event or {'mode': 'idle'}
    img = _canvas()
    d = ImageDraw.Draw(img)
    poster_box = (28, 24, 274, 350)
    poster = _poster_image(event.get('poster_path'))
    if poster:
        img.paste(poster, poster_box[:2])
    else:
        _round(d, poster_box, fill=CARD_2, outline=(38,52,68), width=1)
        _text(d, (151,165), '◈', 78, CYAN, anchor='mm')
        _text(d, (151,250), brand, 22, MUTED, anchor='mm')
    _round(d, (298,24,932,350), fill=CARD, outline=(31,43,56), width=1)
    mode = event.get('mode', 'idle')
    accent = CYAN if mode == 'playing' else AMBER if mode == 'incoming' else GREEN if mode == 'landed' else MUTED
    _text(d, (330,62), media_headline(event), 29, accent)
    title = str(event.get('title') or 'Ready for media')
    if len(title) > 28:
        title = title[:27] + '…'
    _text(d, (330,132), title, 42, FG)
    progress = event.get('progress_pct')
    if progress is not None:
        _bar(d, (330,214,894,234), progress, accent)
        _text(d, (894,258), f'{progress:.0f}%', 18, MUTED, anchor='ra')
    meta = []
    if event.get('play_method'):
        meta.append(str(event['play_method']).upper())
    if event.get('speed_bytes_s'):
        meta.append(f"{event['speed_bytes_s']/1_000_000:.0f} MB/s")
    if event.get('year'):
        meta.append(str(event['year']))
    _text(d, (330,310), '  •  '.join(meta[:3]) or 'READY', 18, MUTED)
    return img

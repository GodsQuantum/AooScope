#!/usr/bin/env python3
import json
import os
import re
import tempfile
import time
from pathlib import Path

SAFE_RE = re.compile(r'[^A-Za-z0-9_-]')


def animation_sensor_key(layer_id):
    safe = SAFE_RE.sub('_', str(layer_id))
    return f'aooscope_animation_{safe}_phase'


def phase_value(elapsed, speed_seconds):
    speed=max(0.5,float(speed_seconds or 4.0))
    return round(((float(elapsed)%speed)/speed)*100.0,1)


def _read_json(path):
    try:return json.loads(Path(path).read_text(encoding='utf-8'))
    except (OSError,json.JSONDecodeError):return None


def active_animation_layers(root):
    root=Path(root); current=_read_json(root/'compiled'/'current.json') or {}
    rid=current.get('revision_id'); source=root/'compiled'/str(rid)/'source-pages.json' if rid else None
    doc=_read_json(source) if source else None
    if not doc:return []
    out=[]
    for pid in doc.get('carousel',[]):
        page=(doc.get('pages') or {}).get(pid) or {}
        if not page.get('enabled',True):continue
        for layer in page.get('layers') or []:
            if layer.get('type')=='animation' and layer.get('animation','orbit')=='orbit':out.append(layer)
    return out


def active_animation_keys(root):
    return [animation_sensor_key(layer.get('id')) for layer in active_animation_layers(root)]


def _atomic_text(path,text):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    fd,tmp=tempfile.mkstemp(prefix=path.name+'.',dir=str(path.parent))
    try:
        with os.fdopen(fd,'w',encoding='utf-8') as f:f.write(text);f.flush();os.fsync(f.fileno())
        os.replace(tmp,path)
    finally:
        try:os.unlink(tmp)
        except OSError:pass


def loop(root=None,fps=5.0):
    root=Path(root or os.getenv('AOOSCOPE_CONFIG_DIR_IN_CONTAINER','/app/cfg'))
    output=root/'sensors'/'animations.txt';fps=max(1.0,min(8.0,float(fps)));started=time.monotonic()
    while True:
        elapsed=time.monotonic()-started; lines=[]
        for layer in active_animation_layers(root):
            lines.append(f"{animation_sensor_key(layer.get('id'))}: {phase_value(elapsed,layer.get('speed_seconds',4))}")
        _atomic_text(output,'\n'.join(lines)+('\n' if lines else ''))
        time.sleep(1.0/fps)


def main():loop(fps=float(os.getenv('AOOSCOPE_ANIMATION_FPS','5')))
if __name__=='__main__':main()

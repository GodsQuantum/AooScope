#!/usr/bin/env python3
import json
import os
from pathlib import Path

from flask import Flask, jsonify, request, Response

from aooscope.settings import (
    PROVIDER_DEFAULTS,
    effective_brightness,
    load_secrets,
    load_settings,
    public_settings,
    save_settings,
)

APP_VERSION = "0.1.0"


def _json_safe_result(result, secrets):
    text = json.dumps(result or {}, ensure_ascii=False)
    for bucket in (secrets or {}).values():
        for value in (bucket or {}).values():
            if value:
                text = text.replace(str(value), "***")
    return json.loads(text)


def _external_provider_secrets():
    out = {}
    token_file = os.getenv("PVE_TOKEN_FILE")
    if token_file:
        try:
            data = json.loads(Path(token_file).read_text(encoding="utf-8"))
            if data.get("full-tokenid") and data.get("value"):
                out["proxmox"] = {"api_token": f"{data['full-tokenid']}={data['value']}"}
        except (OSError, json.JSONDecodeError):
            pass
    for name, env_name in (("jellyfin","JELLYFIN_API_KEY_FILE"),("silo","SILO_API_KEY_FILE"),("radarr","RADARR_API_KEY_FILE")):
        path = os.getenv(env_name)
        if not path:
            continue
        try:
            value = Path(path).read_text(encoding="utf-8").strip()
        except OSError:
            value = ""
        if value:
            out[name] = {"api_key": value}
    return out


def _merge_external_secrets(saved):
    merged = {k: dict(v or {}) for k, v in (saved or {}).items()}
    for name, values in _external_provider_secrets().items():
        bucket = merged.setdefault(name, {})
        for key, value in values.items():
            bucket.setdefault(key, value)
    return merged


def create_app(config_dir=None, provider_tester=None):
    root = Path(config_dir or os.getenv("AOOSCOPE_CONFIG_DIR_IN_CONTAINER", "/app/cfg"))
    settings_path = root / "settings.json"
    secrets_path = root / "private" / "providers.json"
    state_path = root / "state.json"
    device = os.getenv("AOOSCOPE_DEVICE", "/dev/ttyACM0")
    app = Flask(__name__)

    if provider_tester is None:
        from aooscope.providers import test_provider as provider_tester

    @app.get("/")
    def index():
        return Response(ADMIN_HTML, mimetype="text/html; charset=utf-8")

    @app.get("/api/settings")
    def get_settings():
        settings = load_settings(settings_path)
        secrets = _merge_external_secrets(load_secrets(secrets_path))
        return jsonify(public_settings(settings, secrets))

    @app.put("/api/settings")
    def put_settings():
        payload = request.get_json(silent=True) or {}
        settings = save_settings(payload, settings_path, secrets_path)
        return jsonify(public_settings(settings, _merge_external_secrets(load_secrets(secrets_path))))

    @app.post("/api/providers/<name>/test")
    def test_provider_route(name):
        if name not in PROVIDER_DEFAULTS:
            return jsonify({"ok": False, "message": "Unknown provider"}), 404
        settings = load_settings(settings_path)
        secrets = _merge_external_secrets(load_secrets(secrets_path))
        provider = settings["providers"].get(name, {})
        try:
            result = provider_tester(name, provider, secrets.get(name, {}))
        except Exception as exc:
            result = {"ok": False, "message": f"{type(exc).__name__}: {exc}"}
        return jsonify(_json_safe_result(result, secrets))

    @app.get("/api/status")
    def get_status():
        settings = load_settings(settings_path)
        state = {}
        try:
            state = json.loads(state_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            pass
        return jsonify({
            "version": APP_VERSION,
            "brightness": effective_brightness(settings),
            "native_brightness": False,
            "device_present": Path(device).exists(),
            "updated_unix": ((state.get("meta") or {}).get("updated_unix")),
        })

    @app.get("/api/health")
    def health():
        return jsonify({"ok": True, "version": APP_VERSION})

    return app


ADMIN_HTML = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>AooScope Admin</title>
<style>
:root{color-scheme:dark;--bg:#071019;--card:#101d2a;--line:#294158;--text:#f4f8fc;--muted:#93a8bc;--cyan:#35d9ff;--green:#58e5a4;--amber:#ffc35d;--red:#ff6375}
*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at top,#10263a 0,#071019 42%);font-family:Inter,system-ui,sans-serif;color:var(--text)}
main{max-width:1120px;margin:auto;padding:30px 20px 60px}.hero{display:flex;justify-content:space-between;align-items:center;gap:20px;margin-bottom:22px}
h1{font-size:34px;margin:0}.sub{color:var(--muted);margin-top:5px}.pill{border:1px solid var(--line);padding:8px 12px;border-radius:999px;color:var(--cyan)}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px}.card{background:rgba(16,29,42,.96);border:1px solid var(--line);border-radius:18px;padding:18px}
.card h2,.card h3{margin:0 0 14px}.row{display:grid;grid-template-columns:1fr 1fr;gap:12px}.field{margin:11px 0}label{display:block;color:var(--muted);font-size:13px;margin-bottom:6px}
input,select{width:100%;border:1px solid #35516d;background:#0b1723;color:var(--text);border-radius:10px;padding:10px 11px;font:inherit}input[type=range]{padding:0}
.toggle{display:flex;align-items:center;gap:9px}.toggle input{width:auto}.btn{border:1px solid #3a5b78;background:#12283b;color:var(--text);padding:9px 14px;border-radius:10px;cursor:pointer;font-weight:700}
.btn.primary{background:#0f83aa;border-color:#22c8ef}.btn:hover{filter:brightness(1.12)}.status{font-size:13px;color:var(--muted);min-height:18px}.ok{color:var(--green)}.bad{color:var(--red)}
.provider{position:relative}.provider .head{display:flex;justify-content:space-between;align-items:center;gap:10px}.secret-note{font-size:12px;color:var(--muted)}
.schedule-row{display:grid;grid-template-columns:1fr 1fr 1fr auto;gap:8px;margin:8px 0}.footerbar{display:flex;gap:12px;align-items:center;margin-top:18px;position:sticky;bottom:12px;background:#0b1620dd;border:1px solid var(--line);padding:12px;border-radius:14px;backdrop-filter:blur(12px)}
@media(max-width:640px){.row,.schedule-row{grid-template-columns:1fr}.hero{align-items:flex-start;flex-direction:column}}
</style>
</head>
<body><main>
<div class="hero"><div><h1>AooScope</h1><div class="sub">Smart LCD dashboard for AOOSTAR systems</div></div><div id="device" class="pill">Display …</div></div>
<div class="grid">
<section class="card"><h2>🖥️ Display</h2>
<div class="field"><label>Brand</label><input id="brand" maxlength="32"></div>
<div class="field"><label>Brightness <strong id="brightnessLabel">100%</strong></label><input id="brightness" type="range" min="0" max="100"></div>
<div class="secret-note">Software luminance. Native WTR MAX backlight control is not exposed by the known protocol.</div>
<div class="row"><div class="field"><label>Carousel interval (s)</label><input id="switchSeconds" type="number" min="2" max="120"></div><div class="field toggle"><input id="scheduleEnabled" type="checkbox"><label for="scheduleEnabled">Brightness schedule</label></div></div>
<div id="schedule"></div><button class="btn" id="addSchedule">+ Schedule</button>
</section>
<section class="card"><h2>📡 Providers</h2><div class="sub">Connect services by URL or IP:port. Secrets are stored separately and never returned by the API.</div></section>
</div>
<h2 style="margin-top:24px">Providers</h2><div id="providers" class="grid"></div>
<div class="footerbar"><button class="btn primary" id="save">Save settings</button><span id="saveStatus" class="status"></span></div>
</main>
<script>
'''const providerMeta={
  proxmox:{title:'Proxmox',secrets:['api_token'],extra:['node']},
  beszel:{title:'Beszel',secrets:['email','password']},
  jellyfin:{title:'Jellyfin',secrets:['api_key']},
  silo:{title:'Silo',secrets:['api_key']},
  radarr:{title:'Radarr',secrets:['api_key']},
  sonarr:{title:'Sonarr',secrets:['api_key']},
  qbittorrent:{title:'qBittorrent',secrets:['username','password']},
  immich:{title:'Immich',secrets:['api_key']},
  ollama:{title:'Ollama',secrets:[]}
};
let settings={};
const $=s=>document.querySelector(s);
function esc(v){return String(v??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function secretLabel(k){return k.replaceAll('_',' ').replace(/\b\w/g,c=>c.toUpperCase());}
function scheduleRow(rule={start:'22:00',end:'08:00',brightness:70}){
  const div=document.createElement('div');div.className='schedule-row';
  div.innerHTML=`<input type="time" class="sch-start" value="${esc(rule.start)}"><input type="time" class="sch-end" value="${esc(rule.end)}"><input type="number" class="sch-bright" min="0" max="100" value="${esc(rule.brightness)}"><button class="btn sch-del">×</button>`;
  div.querySelector('.sch-del').onclick=()=>div.remove();return div;
}
function renderSchedule(rules){const box=$('#schedule');box.innerHTML='';(rules||[]).forEach(r=>box.append(scheduleRow(r)));}
function renderProviders(){
  const root=$('#providers');root.innerHTML='';
  Object.entries(providerMeta).forEach(([name,meta])=>{
    const p=(settings.providers||{})[name]||{};const card=document.createElement('section');card.className='card provider';card.dataset.name=name;
    let extra='';(meta.extra||[]).forEach(k=>extra+=`<div class="field"><label>${secretLabel(k)}</label><input data-extra="${k}" value="${esc(p[k]||'')}"></div>`);
    let secrets='';meta.secrets.forEach(k=>secrets+=`<div class="field"><label>${secretLabel(k)}${p.secret_set?' · saved':''}</label><input data-secret="${k}" type="${k.includes('password')||k.includes('token')||k.includes('key')?'password':'text'}" placeholder="${p.secret_set?'leave blank to keep saved value':''}"></div>`);
    card.innerHTML=`<div class="head"><h3>${meta.title}</h3><label class="toggle"><input class="enabled" type="checkbox" ${p.enabled?'checked':''}> enabled</label></div><div class="field"><label>URL / IP:port</label><input class="url" value="${esc(p.url||'')}"></div>${extra}${secrets}<div class="row"><label class="toggle"><input class="verify" type="checkbox" ${p.verify_tls!==false?'checked':''}> Verify TLS</label><button class="btn test">Test connection</button></div><div class="status test-status"></div>`;
    card.querySelector('.test').onclick=()=>testProvider(name,card);root.append(card);
  });
}
function collectProvider(card){
  const name=card.dataset.name, out={enabled:card.querySelector('.enabled').checked,url:card.querySelector('.url').value.trim(),verify_tls:card.querySelector('.verify').checked};
  card.querySelectorAll('[data-extra]').forEach(el=>out[el.dataset.extra]=el.value.trim());
  card.querySelectorAll('[data-secret]').forEach(el=>{if(el.value.trim())out[el.dataset.secret]=el.value.trim();});
  return [name,out];
}
async function testProvider(name,card){
  const st=card.querySelector('.test-status');st.textContent='Testing…';st.className='status test-status';
  const [,provider]=collectProvider(card);await save(false,{providers:{[name]:provider}});
  const r=await fetch(`/api/providers/${name}/test`,{method:'POST'}),d=await r.json();st.textContent=d.message||JSON.stringify(d);st.className='status test-status '+(d.ok?'ok':'bad');
}
function collectDisplay(){
  return {brand:$('#brand').value.trim(),brightness:+$('#brightness').value,schedule_enabled:$('#scheduleEnabled').checked,switch_seconds:+$('#switchSeconds').value,timezone:$('#timezone').value.trim()||'UTC',schedule:[...document.querySelectorAll('.schedule-row')].map(r=>({start:r.querySelector('.sch-start').value,end:r.querySelector('.sch-end').value,brightness:+r.querySelector('.sch-bright').value}))};
}
async function save(show=true,partial=null){
  const payload=partial||{display:collectDisplay(),providers:Object.fromEntries([...document.querySelectorAll('.provider')].map(collectProvider))};
  const r=await fetch('/api/settings',{method:'PUT',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});settings=await r.json();
  if(show){$('#saveStatus').textContent='Saved';$('#saveStatus').className='status ok';setTimeout(()=>$('#saveStatus').textContent='',1800);}render();return settings;
}
function render(){
  const d=settings.display||{};$('#brand').value=d.brand||'AOOSCOPE';$('#brightness').value=d.brightness??100;$('#brightnessLabel').textContent=`${d.brightness??100}%`;$('#switchSeconds').value=d.switch_seconds??8;$('#timezone').value=d.timezone||'UTC';$('#scheduleEnabled').checked=!!d.schedule_enabled;renderSchedule(d.schedule||[]);renderProviders();
}
async function load(){settings=await (await fetch('/api/settings')).json();render();const s=await (await fetch('/api/status')).json();$('#device').textContent=`${s.device_present?'●':'○'} Display · ${s.brightness}%`;$('#device').className='pill '+(s.device_present?'ok':'bad');}
$('#brightness').oninput=e=>$('#brightnessLabel').textContent=`${e.target.value}%`;
$('#addSchedule').onclick=()=>$('#schedule').append(scheduleRow());$('#save').onclick=()=>save(true);load();
</script></body></html>"""


app = create_app()

if __name__ == "__main__":
    app.run(host="0.0.0.0", port=8765, debug=False)

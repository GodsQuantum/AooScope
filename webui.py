# -*- coding: utf-8 -*-
from flask import Flask, request, jsonify, Response, send_from_directory
import json, subprocess, os, base64
from PIL import Image
import io

app = Flask(__name__)
try:
    from flask_cors import CORS
    CORS(app)
except:
    pass

CONFIG = '/app/cfg/monitor.json'
IMG_DIR = '/app/cfg'

HTML = """<!DOCTYPE html>
<html>
<head>
<title>AOOSTAR Screen Editor v2</title>
<meta charset="utf-8">
<style>
* { box-sizing: border-box; }
body { font-family: Arial; background: #0d1117; color: #e6edf3; margin: 0; padding: 15px; }
h1 { color: #58a6ff; text-align: center; margin: 0 0 10px; }
.toolbar { display: flex; gap: 8px; justify-content: center; align-items: center; flex-wrap: wrap; margin-bottom: 10px; padding: 10px; background: #161b22; border-radius: 8px; }
.btn { background: #21262d; border: 1px solid #30363d; color: #e6edf3; padding: 6px 14px; border-radius: 6px; cursor: pointer; font-size: 13px; transition: all 0.2s; }
.btn:hover { background: #30363d; }
.btn.active { background: #1f6feb; border-color: #388bfd; }
.btn-green { background: #238636; border-color: #2ea043; }
.btn-green:hover { background: #2ea043; }
.btn-blue { background: #1f6feb; border-color: #388bfd; }
.btn-blue:hover { background: #388bfd; }
.btn-orange { background: #9e6a03; border-color: #d29922; }
.btn-orange:hover { background: #d29922; }
.btn-red { background: #da3633; border-color: #f85149; padding: 4px 10px; }
.btn-red:hover { background: #f85149; }
.btn-purple { background: #6e40c9; border-color: #a371f7; }
.btn-purple:hover { background: #a371f7; }
.status { text-align: center; padding: 8px; border-radius: 6px; margin: 5px 0; font-weight: bold; }
.ok { background: #1a4f2a; color: #56d364; }
.err { background: #4a1515; color: #f85149; }
.preview-wrap { position: relative; margin: 10px auto; width: 960px; }
.preview { background: #000; width: 960px; height: 376px; position: relative; border: 2px solid #30363d; border-radius: 8px; overflow: hidden; }
.preview-item { position: absolute; color: white; font-weight: bold; transform: translate(-50%, -50%); background: rgba(255,255,255,0.15); padding: 2px 6px; border-radius: 4px; cursor: grab; font-size: 11px; white-space: nowrap; border: 1px solid rgba(255,255,255,0.3); user-select: none; }
.preview-item.dragging { cursor: grabbing; background: rgba(88,166,255,0.6); outline: 2px dashed #58a6ff; z-index: 10; }
.grid-overlay { position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; opacity: 0.1; }
.snap-indicator { position: absolute; background: rgba(88,166,255,0.8); pointer-events: none; z-index: 20; }
.preview-item:hover { background: rgba(88,166,255,0.4); outline: 2px solid #58a6ff; }
.preview-item.selected { background: rgba(88,166,255,0.5); outline: 2px solid #58a6ff; }
.labels-wrap { background: #161b22; padding: 8px; border-radius: 6px; margin: 8px 0; max-height: 100px; overflow-y: auto; }
.label-tag { display: inline-block; background: #21262d; padding: 2px 7px; border-radius: 4px; margin: 2px; cursor: pointer; border: 1px solid #30363d; font-size: 11px; }
.label-tag:hover { background: #1f6feb; }
table { width: 100%; border-collapse: collapse; margin-top: 8px; font-size: 13px; }
th { background: #161b22; padding: 8px; text-align: left; border: 1px solid #30363d; color: #58a6ff; position: sticky; top: 0; }
td { padding: 5px; border: 1px solid #21262d; }
tr:hover td { background: #161b22; }
tr.selected-row td { background: #1a2a4a; }
input[type=text], input[type=number] { background: #21262d; border: 1px solid #30363d; color: #e6edf3; padding: 3px 6px; border-radius: 4px; width: 100%; }
input[type=color] { width: 40px; height: 28px; padding: 0; border: none; border-radius: 4px; cursor: pointer; background: none; }
.section-title { color: #8b949e; font-size: 12px; font-weight: bold; margin: 5px 0 3px; }
.modal { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.7); z-index: 100; justify-content: center; align-items: center; }
.modal.show { display: flex; }
.modal-box { background: #161b22; border: 1px solid #30363d; border-radius: 12px; padding: 20px; min-width: 400px; max-width: 600px; }
.modal-box h3 { color: #58a6ff; margin: 0 0 15px; }
.modal-box input { width: 100%; margin: 5px 0 10px; }
.fullscreen { position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: #000; z-index: 200; display: flex; align-items: center; justify-content: center; }
.fullscreen img { max-width: 100%; max-height: 100%; }
.fullscreen-close { position: fixed; top: 20px; right: 20px; z-index: 201; background: rgba(0,0,0,0.7); border: 1px solid #fff; color: #fff; padding: 8px 16px; border-radius: 6px; cursor: pointer; font-size: 16px; }
.badge { display: inline-block; background: #1f6feb; color: #fff; border-radius: 10px; padding: 1px 7px; font-size: 11px; margin-left: 4px; }
</style>
</head>
<body>
<h1>AOOSTAR Screen Editor <span class="badge">v2</span></h1>
<div id="status"></div>

<div class="toolbar">
  <strong>Panneau:</strong>
  <span id="panel-btns"></span>
  <button class="btn btn-blue" id="btn-add-panel">+ Panneau</button>
  <button class="btn btn-green" id="btn-save">Sauvegarder</button>
  <button class="btn btn-orange" id="btn-undo" title="Ctrl+Z">Annuler</button>
  <button class="btn btn-purple" id="btn-fullscreen">Plein ecran</button>
  <button class="btn" id="btn-export">Export JSON</button>
  <label class="btn" style="cursor:pointer">Import JSON<input type="file" id="import-file" accept=".json" style="display:none"></label>
</div>

<div class="toolbar">
  <strong>Image fond:</strong>
  <label class="btn btn-blue" style="cursor:pointer">
    Uploader image
    <input type="file" id="upload-file" accept="image/*" style="display:none">
  </label>
  <span id="current-img" style="color:#8b949e; font-size:13px;"></span>
  <label style="display:flex;align-items:center;gap:5px;font-size:13px">
    <input type="checkbox" id="snap-toggle" checked> Snap grille
  </label>
  <label style="display:flex;align-items:center;gap:5px;font-size:13px">
    Grille: <input type="number" id="grid-size" value="10" style="width:50px;background:#21262d;border:1px solid #30363d;color:#e6edf3;padding:2px 4px;border-radius:4px" min="5" max="50">px
  </label>
  <strong style="margin-left:15px">Elements:</strong>
  <button class="btn btn-green" id="btn-add-element">+ Element</button>
  <button class="btn" id="btn-duplicate">Dupliquer</button>
  <button class="btn btn-red" id="btn-delete">Supprimer</button>
</div>

<div class="toolbar" style="background:#0d1f12;">
  <strong style="color:#2ea043">Transition panneaux:</strong>
  <input type="number" id="switch-time" value="3" min="1" max="60" style="width:55px;background:#21262d;border:1px solid #30363d;color:#e6edf3;padding:4px 6px;border-radius:4px;font-size:14px">
  <span style="color:#8b949e">secondes</span>
  <button class="btn btn-green" id="btn-apply-time">Appliquer</button>
  <span style="width:30px"></span>
  <button class="btn" id="btn-live-preview" style="background:#0d3b2e;border-color:#2ea043;min-width:160px">&#9654; Live Preview OFF</button>
  <span style="color:#8b949e;font-size:12px">— affiche les vraies valeurs des capteurs sur l'apercu</span>
</div>

<div class="preview-wrap">
  <div class="preview" id="preview">
    <img id="bg-img" style="position:absolute;width:100%;height:100%;object-fit:cover;" src="">
  </div>
</div>

<div class="section-title">Labels disponibles (clic pour copier):</div>
<div class="labels-wrap" id="labels-list"></div>

<table>
  <thead>
    <tr>
      <th style="width:30px">#</th>
      <th>Nom</th>
      <th>Label</th>
      <th style="width:65px">X</th>
      <th style="width:65px">Y</th>
      <th style="width:60px">Taille</th>
      <th style="width:70px">Couleur</th>
      <th style="width:60px">Unite</th>
      <th>Valeur</th>
      <th style="width:80px">Actions</th>
    </tr>
  </thead>
  <tbody id="table-body"></tbody>
</table>

<div id="fullscreen-view" class="fullscreen" style="display:none" onclick="hideFullscreen()">
  <span class="fullscreen-close" id="btn-close-fullscreen">X Fermer</span>
  <img id="fullscreen-img" src="">
</div>

<script>
var cfg = null;
var currentPanel = 0;
var selectedIdx = -1;
var snapEnabled = true;
var gridSize = 10;
var isDragging = false;
var livePreviewActive = false;
var livePreviewInterval = null;
var dragEl = null;
var dragIdx = -1;
var dragOffX = 0;
var dragOffY = 0;
var undoHistory = [];
var undoIdx = -1;

function esc(s) {
  return String(s||'').replace(/&/g,'&amp;').replace(/"/g,'&quot;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

function colorToHex(c) {
  if (!c || c === -1 || c === '-1') return '#ffffff';
  if (typeof c === 'string' && c.startsWith('#')) return c;
  return '#ffffff';
}

function hexToColor(h) { return h; }

function toggleSnap(val) { snapEnabled = val; showStatus('Snap ' + (val ? 'actif' : 'desactive'), true); }

function applySwitchTime() {
  var t = parseInt(document.getElementById('switch-time').value) || 3;
  if (!cfg.setup) cfg.setup = {};
  cfg.setup.switchTime = String(t);
  showStatus('Transition: ' + t + 'sec - pensez a Sauvegarder!', true);
}

function toggleLivePreview() {
  var btn = document.getElementById('btn-live-preview');
  if (livePreviewActive) {
    clearInterval(livePreviewInterval);
    livePreviewActive = false;
    btn.textContent = 'Live Preview OFF';
    btn.style.background = '#0d3b2e';
    renderPanel();
    showStatus('Live Preview desactive', false);
  } else {
    livePreviewActive = true;
    btn.textContent = 'Live Preview ON';
    btn.style.background = '#238636';
    showStatus('Live Preview actif - valeurs reelles', true);
    updateLiveValues();
    livePreviewInterval = setInterval(updateLiveValues, 5000);
  }
}

async function updateLiveValues() {
  try {
    var r = await fetch('/api/live_values');
    var vals = await r.json();
    var sensors = cfg.diy[currentPanel].sensor;
    var preview = document.getElementById('preview');
    var items = preview.querySelectorAll('.preview-item');
    sensors.forEach(function(s, i) {
      var mapped = vals[s.label] || vals['mapped_' + s.label];
      if (mapped !== undefined && items[i]) {
        items[i].textContent = (s.name||'') + ': ' + mapped + (s.unit||'');
      }
    });
  } catch(e) {}
}

function snapVal(v, size) {
  if (!snapEnabled) return Math.round(v);
  return Math.round(v / size) * size;
}

function pushHistory() {
  undoHistory = undoHistory.slice(0, undoIdx + 1);
  undoHistory.push(JSON.stringify(cfg));
  undoIdx = undoHistory.length - 1;
  if (undoHistory.length > 30) { undoHistory.shift(); undoIdx--; }
}

function undo() {
  if (undoIdx <= 0) { showStatus('Rien a annuler', false); return; }
  undoIdx--;
  cfg = JSON.parse(history[undoIdx]);
  renderPanelBtns();
  renderPanel();
  showStatus('Annule', true);
}

document.addEventListener('keydown', function(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'z') { e.preventDefault(); undo(); }
});

document.addEventListener('DOMContentLoaded', function() {
  initButtons();
  load();
});

function initButtons() {
  document.getElementById('btn-add-panel').addEventListener('click', addPanel);
  document.getElementById('btn-save').addEventListener('click', save);
  document.getElementById('btn-undo').addEventListener('click', undo);
  document.getElementById('btn-fullscreen').addEventListener('click', showFullscreen);
  document.getElementById('btn-export').addEventListener('click', exportJSON);
  document.getElementById('btn-add-element').addEventListener('click', addElement);
  document.getElementById('btn-duplicate').addEventListener('click', duplicateSelected);
  document.getElementById('btn-delete').addEventListener('click', deleteSelected);
  document.getElementById('btn-close-fullscreen').addEventListener('click', hideFullscreen);
  document.getElementById('import-file').addEventListener('change', function() { importJSON(this); });
  document.getElementById('upload-file').addEventListener('change', function() { uploadImage(this); });
  document.getElementById('snap-toggle').addEventListener('change', function() { toggleSnap(this.checked); });
  document.getElementById('btn-apply-time').addEventListener('click', applySwitchTime);
  document.getElementById('btn-live-preview').addEventListener('click', toggleLivePreview);
}

async function load() {
  try {
    var r = await fetch('/api/config');
    cfg = await r.json();
    pushHistory();
    initButtons();
    if (cfg.setup && cfg.setup.switchTime) {
      document.getElementById('switch-time').value = cfg.setup.switchTime;
    }
    renderPanelBtns();
    renderPanel();
    loadLabels();
  } catch(e) { showStatus('Erreur chargement: ' + e, false); }
}

async function loadLabels() {
  try {
    var r = await fetch('/api/labels');
    var labels = await r.json();
    var el = document.getElementById('labels-list');
    el.innerHTML = labels.map(function(l) {
      return '<span class="label-tag" data-label="' + esc(l) + '" onclick="copyLabel(this.dataset.label)">' + esc(l) + '</span>';
    }).join('');
  } catch(e) {}
}

function copyLabel(l) {
  insertLabel(l);
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(l);
    }
  } catch(e) {}
}

function fallbackCopy(l) {
  var ta = document.createElement('textarea');
  ta.value = l;
  document.body.appendChild(ta);
  ta.select();
  document.execCommand('copy');
  document.body.removeChild(ta);
  insertLabel(l);
}

function insertLabel(l) {
  if (selectedIdx >= 0) {
    cfg.diy[currentPanel].sensor[selectedIdx].label = l;
    renderPanel();
    showStatus('Label applique: ' + l, true);
  } else {
    showStatus('Copie: ' + l, true);
  }
}

function renderPanelBtns() {
  var el = document.getElementById('panel-btns');
  el.innerHTML = cfg.diy.map(function(p, i) {
    return '<span style="display:inline-flex;gap:2px;align-items:center">' +
      '<button class="btn ' + (i===currentPanel?'active':'') + '" onclick="switchPanel(' + i + ')">P' + (i+1) + '</button>' +
      (cfg.diy.length > 1 ? '<button class="btn btn-red" style="padding:2px 6px;font-size:11px" onclick="deletePanel(' + i + ')">x</button>' : '') +
      '</span>';
  }).join('');
}

function deletePanel(i) {
  if (!confirm('Supprimer le panneau ' + (i+1) + ' ?')) return;
  pushHistory();
  cfg.diy.splice(i, 1);
  cfg.mianban = cfg.diy.map(function(_,idx) { return idx+1; });
  if (currentPanel >= cfg.diy.length) currentPanel = cfg.diy.length - 1;
  selectedIdx = -1;
  renderPanelBtns();
  renderPanel();
  showStatus('Panneau supprime', true);
}

function switchPanel(i) {
  currentPanel = i;
  selectedIdx = -1;
  renderPanelBtns();
  renderPanel();
}

function renderPanel() {
  var panel = cfg.diy[currentPanel];
  var sensors = panel.sensor;
  var img = panel.img;
  document.getElementById('bg-img').src = '/api/image/' + img + '?t=' + Date.now();
  document.getElementById('current-img').textContent = img;

  var preview = document.getElementById('preview');
  preview.querySelectorAll('.preview-item').forEach(function(e) { e.remove(); });
  
  sensors.forEach(function(s, i) {
    var el = document.createElement('div');
    el.className = 'preview-item' + (i===selectedIdx?' selected':'');
    el.style.left = (s.x / 960 * 100) + '%';
    el.style.top = (s.y / 376 * 100) + '%';
    el.style.fontSize = Math.max(8, (s.fontSize||24) * 0.45) + 'px';
    var col = colorToHex(s.fontColor);
    el.style.color = col;
    el.textContent = (s.name||'?') + ': ' + (s.value||'');
    el.setAttribute('data-idx', i);

    el.addEventListener('mousedown', function(e) {
      e.preventDefault();
      e.stopPropagation();
      isDragging = true;
      dragIdx = i;
      dragEl = el;
      el.classList.add('dragging');
      selectRow(i);
    });

    preview.appendChild(el);
  });

  var tbody = document.getElementById('table-body');
  if (sensors.length === 0) {
    tbody.innerHTML = '<tr><td colspan="10" style="text-align:center;color:#8b949e;padding:20px">Aucun element - cliquez sur + Element pour en ajouter</td></tr>';
  } else {
    tbody.innerHTML = sensors.map(function(s, i) {
      var col = colorToHex(s.fontColor);
      return '<tr id="row-' + i + '" class="' + (i===selectedIdx?'selected-row':'') + '" onclick="selectRow(' + i + ')">' +
        '<td style="text-align:center;color:#8b949e">' + (i+1) + '</td>' +
        '<td><input type="text" data-i="' + i + '" data-k="name" value="' + esc(s.name) + '" onchange="updateStr(this)"></td>' +
        '<td><input type="text" data-i="' + i + '" data-k="label" value="' + esc(s.label||"") + '" onchange="updateStr(this)"></td>' +
        '<td><input type="number" data-i="' + i + '" data-k="x" value="' + (s.x||0) + '" onchange="updateNum(this)"></td>' +
        '<td><input type="number" data-i="' + i + '" data-k="y" value="' + (s.y||0) + '" onchange="updateNum(this)"></td>' +
        '<td><input type="number" data-i="' + i + '" data-k="fontSize" value="' + (s.fontSize||24) + '" onchange="updateNum(this)"></td>' +
        '<td><input type="color" data-i="' + i + '" data-k="fontColor" value="' + col + '" onchange="updateColor(this)"></td>' +
        '<td><input type="text" data-i="' + i + '" data-k="unit" value="' + esc(s.unit||"") + '" onchange="updateStr(this)"></td>' +
        '<td><input type="text" data-i="' + i + '" data-k="value" value="' + esc(s.value||"") + '" onchange="updateStr(this)"></td>' +
        '<td style="text-align:center">' +
          '<button class="btn" style="padding:2px 6px;font-size:11px" onclick="dupRow(' + i + ')">Dup</button> ' +
          '<button class="btn btn-red" onclick="delRow(' + i + ')">X</button>' +
        '</td>' +
        '</tr>';
    }).join('');
  }
}

function selectRow(i) {
  selectedIdx = i;
  renderPanel();
  var row = document.getElementById('row-' + i);
  if (row) row.scrollIntoView({behavior:'smooth', block:'nearest'});
}

function updateStr(el) {
  pushHistory();
  cfg.diy[currentPanel].sensor[parseInt(el.dataset.i)][el.dataset.k] = el.value;
  renderPanel();
}

function updateNum(el) {
  pushHistory();
  cfg.diy[currentPanel].sensor[parseInt(el.dataset.i)][el.dataset.k] = +el.value;
  renderPanel();
}

function updateColor(el) {
  pushHistory();
  cfg.diy[currentPanel].sensor[parseInt(el.dataset.i)][el.dataset.k] = el.value;
  renderPanel();
}

function addElement() {
  if (!cfg || !cfg.diy || !cfg.diy[currentPanel]) {
    showStatus('Erreur: config non chargee', false);
    return;
  }
  pushHistory();
  if (!cfg.diy[currentPanel].sensor) cfg.diy[currentPanel].sensor = [];
  cfg.diy[currentPanel].sensor.push({
    mode:1, type:1, name:'Nouveau', label:'cpu_temperature',
    x:480, y:188, fontSize:24, fontColor:'#ffffff', fontWeight:'bold',
    unit:'', value:'0', width:0, height:0, textDirection:0, direction:1,
    textAlign:'center', integerDigits:-1, decimalDigits:0,
    minAngle:0, maxAngle:180, minValue:0, maxValue:100,
    pic:'', xz_x:0, xz_y:0
  });
  selectedIdx = cfg.diy[currentPanel].sensor.length - 1;
  renderPanel();
  showStatus('Element ajoute - modifiez le label dans le tableau', true);
  var tbody = document.getElementById('table-body');
  if (tbody) tbody.lastElementChild && tbody.lastElementChild.scrollIntoView({behavior:'smooth'});
}

function dupRow(i) {
  pushHistory();
  var clone = JSON.parse(JSON.stringify(cfg.diy[currentPanel].sensor[i]));
  clone.x += 20; clone.y += 20;
  cfg.diy[currentPanel].sensor.splice(i+1, 0, clone);
  selectedIdx = i+1;
  renderPanel();
}

function duplicateSelected() {
  if (selectedIdx < 0) { showStatus('Selectionnez un element', false); return; }
  dupRow(selectedIdx);
}

function delRow(i) {
  if (!confirm('Supprimer?')) return;
  pushHistory();
  cfg.diy[currentPanel].sensor.splice(i, 1);
  selectedIdx = -1;
  renderPanel();
}

function deleteSelected() {
  if (selectedIdx < 0) { showStatus('Selectionnez un element', false); return; }
  delRow(selectedIdx);
}

function addPanel() {
  pushHistory();
  cfg.diy.push({ type:5, img:'proxmox_panel.jpg', sensor:[], mianban: cfg.diy.length+1 });
  if (!cfg.mianban) cfg.mianban = [];
  cfg.mianban.push(cfg.diy.length);
  currentPanel = cfg.diy.length - 1;
  selectedIdx = -1;
  renderPanelBtns();
  renderPanel();
  showStatus('Panneau ' + cfg.diy.length + ' cree', true);
}

function uploadImage(input) {
  if (!input.files || !input.files[0]) return;
  var file = input.files[0];
  var formData = new FormData();
  formData.append('image', file);
  formData.append('panel', currentPanel);
  fetch('/api/upload_image', { method:'POST', body: formData })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) {
        cfg.diy[currentPanel].img = res.filename;
        renderPanel();
        showStatus('Image uploadee: ' + res.filename + ' (redim. 960x376)', true);
      } else {
        showStatus('Erreur: ' + res.message, false);
      }
    })
    .catch(function(e) { showStatus('Erreur upload: ' + e, false); });
}

function exportJSON() {
  var blob = new Blob([JSON.stringify(cfg, null, 2)], {type:'application/json'});
  var a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = 'monitor_backup_' + new Date().toISOString().slice(0,10) + '.json';
  a.click();
  showStatus('Export OK', true);
}

function importJSON(input) {
  if (!input.files || !input.files[0]) return;
  var reader = new FileReader();
  reader.onload = function(e) {
    try {
      var newCfg = JSON.parse(e.target.result);
      pushHistory();
      cfg = newCfg;
      currentPanel = 0;
      renderPanelBtns();
      renderPanel();
      showStatus('Import OK', true);
    } catch(err) {
      showStatus('JSON invalide: ' + err, false);
    }
  };
  reader.readAsText(input.files[0]);
}

function showFullscreen() {
  var el = document.getElementById('fullscreen-view');
  var img = document.getElementById('fullscreen-img');
  img.src = '/api/image/' + cfg.diy[currentPanel].img + '?t=' + Date.now();
  el.style.display = 'flex';
}

function hideFullscreen() {
  document.getElementById('fullscreen-view').style.display = 'none';
}

async function save() {
  try {
    var r = await fetch('/api/save', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(cfg)
    });
    var res = await r.json();
    showStatus(res.message, res.ok);
  } catch(e) { showStatus('Erreur: ' + e, false); }
}

function showStatus(msg, ok) {
  var el = document.getElementById('status');
  el.className = 'status ' + (ok?'ok':'err');
  el.textContent = msg;
  setTimeout(function() { el.className=''; el.textContent=''; }, 4000);
}

// Drag & Drop global events
document.addEventListener('mousemove', function(e) {
  if (!isDragging || dragIdx < 0) return;
  var preview = document.getElementById('preview');
  var rect = preview.getBoundingClientRect();
  var gs = parseInt(document.getElementById('grid-size').value) || 10;
  
  var rawX = (e.clientX - rect.left) / rect.width * 960;
  var rawY = (e.clientY - rect.top) / rect.height * 376;
  rawX = Math.max(0, Math.min(960, rawX));
  rawY = Math.max(0, Math.min(376, rawY));
  
  var newX = snapEnabled ? Math.round(rawX / gs) * gs : Math.round(rawX);
  var newY = snapEnabled ? Math.round(rawY / gs) * gs : Math.round(rawY);
  
  cfg.diy[currentPanel].sensor[dragIdx].x = newX;
  cfg.diy[currentPanel].sensor[dragIdx].y = newY;
  
  if (dragEl) {
    dragEl.style.left = (newX / 960 * 100) + '%';
    dragEl.style.top = (newY / 376 * 100) + '%';
  }
  
  var rowX = document.querySelector('#row-' + dragIdx + ' input[data-k="x"]');
  var rowY = document.querySelector('#row-' + dragIdx + ' input[data-k="y"]');
  if (rowX) rowX.value = newX;
  if (rowY) rowY.value = newY;
});

document.addEventListener('mouseup', function(e) {
  if (!isDragging) return;
  isDragging = false;
  if (dragEl) dragEl.classList.remove('dragging');
  if (dragIdx >= 0) pushHistory();
  dragEl = null;
  dragIdx = -1;
});
</script>
</body>
</html>"""

@app.route('/')
def index():
    return Response(HTML, mimetype='text/html; charset=utf-8')

@app.route('/api/config')
def get_config():
    return jsonify(json.load(open(CONFIG)))

@app.route('/api/save', methods=['POST'])
def save_config():
    try:
        cfg = request.json
        os.system('cp ' + CONFIG + ' ' + CONFIG + '.bak')
        with open(CONFIG, 'w', encoding='utf-8') as f:
            json.dump(cfg, f, indent=2, ensure_ascii=False)
        subprocess.run(['pkill', '-f', 'asterctl'], capture_output=True); import time; time.sleep(1); subprocess.Popen(['/usr/local/bin/asterctl', '--config-dir', '/app/cfg', '--config', 'monitor.json', '--sensor-path', '/app/cfg/sensors/values.txt', '--sensor-mapping', '/app/cfg/sensor-mapping.cfg'])
        return jsonify({'ok': True, 'message': 'Sauvegarde OK et asterctl recharge!'})
    except Exception as e:
        return jsonify({'ok': False, 'message': 'Erreur: ' + str(e)})

@app.route('/api/image/<filename>')
def get_image(filename):
    return send_from_directory(IMG_DIR, filename)

@app.route('/api/upload_image', methods=['POST'])
def upload_image():
    try:
        file = request.files['image']
        panel = int(request.form.get('panel', 0))
        img = Image.open(file.stream)
        img = img.convert('RGB')
        img = img.resize((960, 376), Image.LANCZOS)
        filename = 'panel_' + str(panel+1) + '_bg.jpg'
        filepath = os.path.join(IMG_DIR, filename)
        img.save(filepath, 'JPEG', quality=92)
        return jsonify({'ok': True, 'filename': filename})
    except Exception as e:
        return jsonify({'ok': False, 'message': str(e)})

@app.route('/api/live_values')
def get_live_values():
    try:
        vals = {}
        # Lire le fichier de valeurs brutes
        with open('/app/cfg/sensors/values.txt') as f:
            for line in f:
                if ':' in line:
                    parts = line.split(':', 1)
                    key = parts[0].strip()
                    val = parts[1].strip().split(' ')[0]
                    vals[key] = val
        # Lire le mapping
        mapping = {}
        try:
            with open('/app/cfg/sensor-mapping.cfg') as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and ':' in line:
                        k, v = line.split(':', 1)
                        mapping[k.strip()] = v.strip()
        except:
            pass
        # Appliquer le mapping
        mapped = {}
        for panel_label, sensor_key in mapping.items():
            if sensor_key in vals:
                mapped[panel_label] = vals[sensor_key]
        # Ajouter les valeurs directes aussi
        mapped.update(vals)
        return jsonify(mapped)
    except Exception as e:
        return jsonify({})

@app.route('/api/labels')
def get_labels():
    try:
        labels = []
        with open('/app/cfg/sensors/values.txt') as f:
            for line in f:
                if ':' in line:
                    labels.append(line.split(':')[0].strip())
        return jsonify(sorted(labels))
    except:
        return jsonify([])

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8765, debug=False)

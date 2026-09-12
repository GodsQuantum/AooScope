const $ = (s, root=document) => root.querySelector(s);
const $$ = (s, root=document) => [...root.querySelectorAll(s)];
const state = {settings:{}, pages:null, currentPage:null, sensors:[], media:[], selectedLayer:null};
let brightnessTimer = null;

async function api(url, options={}) {
  const opts = {...options};
  opts.headers = {...(opts.body instanceof FormData ? {} : {'Content-Type':'application/json'}), ...(opts.headers||{})};
  const response = await fetch(url, opts);
  const type = response.headers.get('content-type') || '';
  const data = type.includes('application/json') ? await response.json() : response;
  if (!response.ok) {
    const message = data?.message || data?.error || `${response.status} ${response.statusText}`;
    throw Object.assign(new Error(message), {response, data});
  }
  return data;
}
function esc(v){return String(v??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));}
function toast(message,bad=false){const el=$('#toast');el.textContent=message;el.className=`toast show ${bad?'bad':''}`;clearTimeout(el._t);el._t=setTimeout(()=>el.className='toast',2200);}

function setupTabs(){
  $$('.tab').forEach(button=>button.addEventListener('click',()=>{
    $$('.tab').forEach(x=>x.classList.toggle('active',x===button));
    $$('.tab-panel').forEach(x=>x.classList.toggle('active',x.dataset.panel===button.dataset.tab));
  }));
}
async function loadPages(){
  state.pages = await api('/api/pages');
  renderPageList();
  if (!state.currentPage && state.pages.carousel.length) await selectPage(state.pages.carousel[0]);
  return state.pages;
}
async function selectPage(id){
  state.currentPage = await api(`/api/pages/${id}`);
  $('#pageTitle').textContent = state.currentPage.name;
  renderPageList();
  renderCanvas(state.currentPage);
  return state.currentPage;
}
async function createPage(templateId=null){
  const page = await api('/api/pages',{method:'POST',body:JSON.stringify(templateId?{template_id:templateId}:{name:'New page'})});
  state.currentPage = page;
  await loadPages();
  await selectPage(page.id);
  toast('Page created');
  return page;
}
async function duplicatePage(id){
  const page = await api(`/api/pages/${id}/duplicate`,{method:'POST'});
  await loadPages(); await selectPage(page.id); toast('Page duplicated'); return page;
}
async function deletePage(id){
  if (!confirm('Delete this page?')) return;
  await api(`/api/pages/${id}`,{method:'DELETE'});
  state.currentPage=null; await loadPages(); toast('Page deleted');
}
async function restorePage(id){
  const page = await api(`/api/pages/${id}/restore`,{method:'POST'});
  state.currentPage=page; await loadPages(); await selectPage(id); toast('Factory template restored');
}
function renderPageList(){
  const root=$('#pageList'); root.innerHTML='';
  if (!state.pages) return;
  state.pages.pages.forEach(page=>{
    const card=document.createElement('div'); card.className='page-card'+(state.currentPage?.id===page.id?' selected':'');
    card.draggable=true; card.dataset.pageId=page.id;
    card.innerHTML=`<div class="page-card-head"><span class="page-name">☰ ${esc(page.name)}</span><span>${page.template_id?'↺':''}</span></div>
      <div class="page-meta"><label><input class="page-enabled" type="checkbox" ${page.enabled?'checked':''}> on</label><label>Duration <input class="page-duration" type="number" min="2" max="120" value="${page.duration}"></label><span>r${page.revision}</span></div>
      <div class="page-actions"><button class="btn mini duplicate">Duplicate</button><button class="btn mini restore">Restore</button><button class="btn mini danger delete">Delete</button></div>`;
    card.addEventListener('click',e=>{if(!e.target.closest('button,input'))selectPage(page.id)});
    $('.duplicate',card).onclick=e=>{e.stopPropagation();duplicatePage(page.id)};
    $('.restore',card).onclick=e=>{e.stopPropagation();restorePage(page.id)};
    $('.delete',card).onclick=e=>{e.stopPropagation();deletePage(page.id)};
    card.addEventListener('dragstart',()=>card.classList.add('dragging'));
    card.addEventListener('dragend',()=>card.classList.remove('dragging'));
    card.addEventListener('dragover',e=>{e.preventDefault();const moving=$('.page-card.dragging');if(moving&&moving!==card){const r=card.getBoundingClientRect();root.insertBefore(moving,e.clientY<r.top+r.height/2?card:card.nextSibling)}});
    root.append(card);
  });
}
async function saveCarousel(){
  const items=$$('.page-card').map(card=>({id:card.dataset.pageId,enabled:$('.page-enabled',card).checked,duration:+$('.page-duration',card).value}));
  const result=await api('/api/carousel',{method:'PUT',body:JSON.stringify({revision:state.pages.revision,items})});
  await loadPages(); toast('Carousel saved'); return result;
}
const providerMeta={
  proxmox:{title:'Proxmox',secrets:['api_token'],extra:['node']},beszel:{title:'Beszel',secrets:['email','password']},
  jellyfin:{title:'Jellyfin',secrets:['api_key']},silo:{title:'Silo',secrets:['api_key']},radarr:{title:'Radarr',secrets:['api_key']},
  sonarr:{title:'Sonarr',secrets:['api_key']},qbittorrent:{title:'qBittorrent',secrets:['username','password']},immich:{title:'Immich',secrets:['api_key']},ollama:{title:'Ollama',secrets:[]}
};
function secretLabel(k){return k.replaceAll('_',' ').replace(/\b\w/g,c=>c.toUpperCase())}
function renderProviders(){
  const root=$('#providers');root.innerHTML='';
  Object.entries(providerMeta).forEach(([name,meta])=>{
    const p=(state.settings.providers||{})[name]||{}, card=document.createElement('section');card.className='provider-card';card.dataset.name=name;
    let extra=(meta.extra||[]).map(k=>`<label>${secretLabel(k)}<input data-extra="${k}" value="${esc(p[k]||'')}"></label>`).join('');
    let secrets=meta.secrets.map(k=>`<label>${secretLabel(k)}${p.secret_set?' · saved':''}<input data-secret="${k}" type="${/(password|token|key)/.test(k)?'password':'text'}" placeholder="${p.secret_set?'leave blank to keep saved value':''}"></label>`).join('');
    card.innerHTML=`<div class="provider-head"><h3>${meta.title}</h3><label class="toggle-line"><input class="enabled" type="checkbox" ${p.enabled?'checked':''}> enabled</label></div><label>URL / IP:port<input class="url" value="${esc(p.url||'')}"></label>${extra}${secrets}<div class="footer-actions"><label class="toggle-line"><input class="verify" type="checkbox" ${p.verify_tls!==false?'checked':''}> Verify TLS</label><button class="btn test">Test</button></div><div class="status test-status"></div>`;
    $('.test',card).onclick=()=>testProvider(name,card);root.append(card);
  });
}
function collectProvider(card){
  const out={enabled:$('.enabled',card).checked,url:$('.url',card).value.trim(),verify_tls:$('.verify',card).checked};
  $$('[data-extra]',card).forEach(el=>out[el.dataset.extra]=el.value.trim());
  $$('[data-secret]',card).forEach(el=>{if(el.value.trim())out[el.dataset.secret]=el.value.trim()});
  return [card.dataset.name,out];
}
async function testProvider(name,card){
  const st=$('.test-status',card);st.textContent='Testing…';
  const [,provider]=collectProvider(card);
  await saveSettings(false,{providers:{[name]:provider}});
  try{const d=await api(`/api/providers/${name}/test`,{method:'POST'});st.textContent=d.message||'Connected';st.className='status ok'}catch(err){st.textContent=err.message;st.className='status bad'}
}
function scheduleRow(rule={start:'22:00',end:'08:00',brightness:70}){
  const row=document.createElement('div');row.className='schedule-row';
  row.innerHTML=`<input type="time" class="sch-start" value="${esc(rule.start)}"><input type="time" class="sch-end" value="${esc(rule.end)}"><input type="number" class="sch-bright" min="0" max="100" value="${esc(rule.brightness)}"><button class="btn sch-del">×</button>`;
  $('.sch-del',row).onclick=()=>row.remove();return row;
}
function renderSettings(){
  const d=state.settings.display||{};$('#brand').value=d.brand||'AOOSCOPE';$('#timezone').value=d.timezone||'UTC';$('#switchSeconds').value=d.switch_seconds??8;$('#scheduleEnabled').checked=!!d.schedule_enabled;$('#brightness').value=d.brightness??100;$('#brightnessLabel').textContent=`${d.brightness??100}%`;
  const box=$('#schedule');box.innerHTML='';(d.schedule||[]).forEach(r=>box.append(scheduleRow(r)));renderProviders();
}
function collectDisplay(){return {brand:$('#brand').value.trim(),timezone:$('#timezone').value.trim()||'UTC',switch_seconds:+$('#switchSeconds').value,schedule_enabled:$('#scheduleEnabled').checked,brightness:+$('#brightness').value,schedule:$$('.schedule-row').map(r=>({start:$('.sch-start',r).value,end:$('.sch-end',r).value,brightness:+$('.sch-bright',r).value}))}}
async function saveSettings(show=true,partial=null){
  const payload=partial||{display:collectDisplay(),providers:Object.fromEntries($$('.provider-card').map(collectProvider))};
  state.settings=await api('/api/settings',{method:'PUT',body:JSON.stringify(payload)});renderSettings();if(show)toast('Settings saved');return state.settings;
}
async function refreshStatus(){
  const s=await api('/api/status');$('#device').textContent=`${s.device_present?'●':'○'} Display · ${s.brightness}%`;$('#device').className='pill '+(s.device_present?'ok':'bad');const age=s.updated_unix?Math.max(0,Math.round(Date.now()/1000-s.updated_unix)):null;$('#displayLive').textContent=s.device_present?`LCD connected · telemetry ${age===null?'waiting':age+'s ago'} · ${s.brightness}%`:'LCD device unavailable';return s;
}
function applyBrightness(value){
  $('#brightnessLabel').textContent=`${value}%`;clearTimeout(brightnessTimer);
  brightnessTimer=setTimeout(async()=>{try{await saveSettings(false,{display:{brightness:+value}});const s=await refreshStatus();toast(`Brightness ${s.brightness}% applied to LCD`)}catch(err){toast(err.message,true)}},250);
}
function renderCanvas(page){
  const canvas=$('#designerCanvas');canvas.innerHTML='';canvas.style.background=(page?.background?.color||'#071019');
  if(!page){canvas.innerHTML='<div class="muted">Select a page</div>';return}
  const hint=document.createElement('div');hint.className='layer-label muted';hint.textContent=`${page.name} · ${page.layers?.length||0} layers`;canvas.append(hint);
}
async function loadBase(){
  state.settings=await api('/api/settings');renderSettings();await Promise.all([loadPages(),refreshStatus()]);
}
function bindUi(){
  setupTabs();$('#addPage').onclick=()=>createPage();$('#saveCarousel').onclick=()=>saveCarousel();$('#saveSettings').onclick=()=>saveSettings(true);$('#saveProviders').onclick=()=>saveSettings(true);$('#addSchedule').onclick=()=>$('#schedule').append(scheduleRow());$('#brightness').oninput=e=>applyBrightness(e.target.value);
}
window.addEventListener('DOMContentLoaded',()=>{bindUi();loadBase().catch(err=>toast(err.message,true));setInterval(()=>refreshStatus().catch(()=>{}),5000)});

export {loadPages,selectPage,createPage,duplicatePage,deletePage,restorePage,saveCarousel};

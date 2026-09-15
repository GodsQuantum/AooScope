<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { requestBlob, requestJson } from '$lib/api/client';
  import { connectMediaEvents, connectStatusEvents } from '$lib/api/live';
  import type { components } from '$lib/api/schema';
  import AdvancedInspector from '$lib/designer/AdvancedInspector.svelte';
  import Canvas from '$lib/designer/Canvas.svelte';
  import ContextToolbar from '$lib/designer/ContextToolbar.svelte';
  import MetricLibrary from '$lib/designer/MetricLibrary.svelte';
  import TemplatePicker from '$lib/designer/TemplatePicker.svelte';
  import WidgetPalette from '$lib/designer/WidgetPalette.svelte';
  import DisplayStatus from '$lib/components/DisplayStatus.svelte';
  import DisplayControls from '$lib/display/DisplayControls.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import MediaDashboard from '$lib/media/MediaDashboard.svelte';
  import PageStrip, { type PageSummary } from '$lib/pages/PageStrip.svelte';
  import ProvidersPanel from '$lib/providers/ProvidersPanel.svelte';
  import { clampRect, createWidgetId, isLatestRequest, type Layer, type Metric, type Page, type WidgetType } from '$lib/designer/model';

  type StatusDto = components['schemas']['StatusDto'];
  type MediaEvent = { mode?: 'playing' | 'incoming' | 'landed' | 'offline' | 'idle'; title?: string; poster_url?: string; progress_pct?: number; eta_minutes?: number; speed_bytes_s?: number; provider_chain?: string[] };
  type Asset = { id: string; name: string; kind?: string; format?: string; revision?: number; width?: number; height?: number };
  type Preset = { id: string; name: string; source_asset_id: string; settings: { fps: number; speed_seconds: number } };
  type ProviderDescriptor = { id: string; name: string; icon: string; categories: string[]; credential_fields: string[] };
  type ProviderSetting = { enabled: boolean; url: string; verify_tls: boolean; secret_set: boolean; node?: string | null; system?: string | null; [key: string]: unknown };
  type ProviderStatus = { id: string; configured: boolean; enabled: boolean; online: boolean; last_success: number | null; error: string | null };
  type SettingsDocument = { display: Record<string, unknown>; providers: Record<string, ProviderSetting>; [key: string]: unknown };
  type PagesResponse = { schema_version: number; revision: number; carousel: string[]; pages: PageSummary[] };

  const tabs = ['Pages', 'Media', 'Display', 'Providers'] as const;
  let active = $state<string>('Pages');
  let status = $state<StatusDto>({ version: '0.3.0-dev', brightness: 100, native_brightness: false, device_present: false, updated_unix: null });
  let pages = $state<PageSummary[]>([]); let documentRevision = $state(1); let page = $state<Page | undefined>();
  let metrics = $state<Metric[]>([]); let media = $state<MediaEvent>({}); let assets = $state<Asset[]>([]); let presets = $state<Preset[]>([]);
  let providerCatalog = $state<ProviderDescriptor[]>([]); let providerStatuses = $state<ProviderStatus[]>([]);
  let settings = $state<SettingsDocument>({ display: {}, providers: {} });
  let selected = $state<string>(); let error = $state(''); let notice = $state(''); let previewUrl = $state<string>(); let pageRequestGeneration = 0;
  let metricLibraryOpen = $state(false); let templatesOpen = $state(false); let advancedOpen = $state(false);
  let capabilities = $state({ width: 960, height: 376, native_brightness: false, power_control: true, power_on: false });
  const selectedLayer = $derived(page?.layers.find((layer) => layer.id === selected));

  async function loadPages(select?: string) {
    const listed = await requestJson<PagesResponse>('/api/pages'); pages = listed.pages; documentRevision = listed.revision;
    const target = select ?? page?.id ?? pages[0]?.id; if (target && pages.some((item) => item.id === target)) await loadPage(target);
  }
  async function refreshPagesPreservingCarousel() {
    const listed = await requestJson<PagesResponse>('/api/pages');
    const drafts = pages.map((item) => ({ ...item }));
    const persisted = new Map(listed.pages.map((item) => [item.id, item]));
    pages = [...drafts.filter((item) => persisted.has(item.id)).map((item) => ({ ...persisted.get(item.id)!, enabled: item.enabled, duration: item.duration })), ...listed.pages.filter((item) => !drafts.some((draft) => draft.id === item.id))];
    documentRevision = listed.revision;
  }
  async function loadLibrary() { const library = await requestJson<{ assets: Asset[]; presets: Preset[] }>('/api/media'); assets = library.assets; presets = library.presets; }
  onMount(() => {
    let alive = true;
    Promise.all([
      requestJson<PagesResponse>('/api/pages'), requestJson<{ metrics: Metric[] }>('/api/metrics'), requestJson<{ providers: ProviderDescriptor[] }>('/api/providers/catalog'),
      requestJson<{ assets: Asset[]; presets: Preset[] }>('/api/media'), requestJson<typeof capabilities>('/api/display/capabilities'), requestJson<SettingsDocument>('/api/settings'), requestJson<ProviderStatus[]>('/api/providers/status')
    ]).then(async ([listed, catalog, providers, library, display, publicSettings, statuses]) => {
      if (!alive) return; pages = listed.pages; documentRevision = listed.revision; metrics = catalog.metrics; providerCatalog = providers.providers; assets = library.assets; presets = library.presets; capabilities = display; settings = publicSettings; providerStatuses = statuses;
      if (pages[0]) await loadPage(pages[0].id, () => alive);
    }).catch((cause) => { if (alive) error = String(cause); });
    requestJson<StatusDto>('/api/status').then((value) => { if (alive) status = value; }).catch(() => {});
    const disconnect = connectStatusEvents((value) => { if (alive) status = value; });
    const disconnectMedia = connectMediaEvents((value) => { if (alive) media = value as MediaEvent; });
    return () => { alive = false; disconnect(); disconnectMedia(); };
  });
  function loadPage(id: string, alive = () => true) { const generation = ++pageRequestGeneration; return requestJson<Page>(`/api/pages/${id}`).then((value) => { if (alive() && isLatestRequest(generation, pageRequestGeneration)) { page = value; selected = undefined; advancedOpen = false; } return value; }).catch((cause) => { if (alive() && isLatestRequest(generation, pageRequestGeneration)) error = String(cause); throw cause; }); }
  function selectPage(id: string) { loadPage(id).catch(() => {}); }
  function addWidget(type: WidgetType, extra?: Record<string, unknown>) { if (!page) return; const id = createWidgetId(); page = { ...page, layers: [...page.layers, { id, type, ...extra, x: 24, y: 24, width: type === 'text' ? 240 : 160, height: type === 'text' ? 48 : 80, z: page.layers.length + 1, text: type === 'text' && !extra?.binding ? 'Text' : undefined }] }; selected = id; notice = 'Draft changed'; }
  function addMetric(metric: Metric, type: WidgetType) { addWidget(type, { binding: metric.id }); metricLibraryOpen = false; }
  function updateLayer(id: string, changes: Partial<Layer>) { if (!page) return; page = { ...page, layers: page.layers.map((layer) => layer.id === id ? { ...layer, ...changes, ...clampRect({ ...layer, ...changes }) } : layer) }; notice = 'Draft changed'; }
  function selectLayer(id: string) { selected = id; advancedOpen = false; }
  function duplicateLayer() { if (!page || !selectedLayer) return; const id = createWidgetId(); const position = clampRect({ ...selectedLayer, x: selectedLayer.x + 12, y: selectedLayer.y + 12 }); page = { ...page, layers: [...page.layers, { ...selectedLayer, ...position, id, z: page.layers.length + 1 }] }; selected = id; notice = 'Draft changed'; }
  function deleteLayer() { if (!page || !selected) return; page = { ...page, layers: page.layers.filter((layer) => layer.id !== selected) }; selected = undefined; advancedOpen = false; notice = 'Draft changed'; }
  async function persistPageDraft() { if (!page) return; const current = page; const selectionGeneration = pageRequestGeneration; const { enabled: _enabled, duration: _duration, ...draft } = current; const saved = await requestJson<Page>(`/api/pages/${current.id}`, 'PUT', draft); if (selectionGeneration === pageRequestGeneration && page?.id === current.id) page = { ...saved, enabled: current.enabled, duration: current.duration }; await refreshPagesPreservingCarousel(); }
  async function savePage() { try { await persistPageDraft(); notice = 'Page saved'; } catch (cause) { error = String(cause); } }
  async function persistCarouselDraft() { const saved = await requestJson<{ revision: number }>('/api/carousel', 'PUT', { revision: documentRevision, items: pages.map(({ id, enabled, duration }) => ({ id, enabled, duration })) }); documentRevision = saved.revision; }
  async function saveCarousel() { try { await persistCarouselDraft(); notice = 'Carousel saved'; } catch (cause) { error = String(cause); } }
  async function createPage(templateId?: string) { const selectionGeneration = pageRequestGeneration; try { const created = await requestJson<Page>('/api/pages', 'POST', templateId ? { template_id: templateId } : { name: 'New page' }); await refreshPagesPreservingCarousel(); templatesOpen = false; if (selectionGeneration === pageRequestGeneration) await loadPage(created.id); } catch (cause) { error = String(cause); } }
  async function duplicatePage(id: string) { const selectionGeneration = pageRequestGeneration; try { const created = await requestJson<Page>(`/api/pages/${id}/duplicate`, 'POST', {}); await refreshPagesPreservingCarousel(); if (selectionGeneration === pageRequestGeneration) await loadPage(created.id); } catch (cause) { error = String(cause); } }
  async function restorePage(id: string) { if (!confirm('Restore this factory page? Your page customizations will be replaced.')) return; const selectionGeneration = pageRequestGeneration; try { await requestJson<Page>(`/api/pages/${id}/restore`, 'POST', {}); await refreshPagesPreservingCarousel(); if (selectionGeneration === pageRequestGeneration) await loadPage(id); notice = 'Factory page restored'; } catch (cause) { error = String(cause); } }
  async function deletePage(id: string) { if (!confirm('Delete this page?')) return; const selectionGeneration = pageRequestGeneration; try { await requestJson(`/api/pages/${id}`, 'DELETE'); await refreshPagesPreservingCarousel(); if (selectionGeneration === pageRequestGeneration && page?.id === id) { const next = pages[0]; if (next) await loadPage(next.id); else page = undefined; } notice = 'Page deleted'; } catch (cause) { error = String(cause); } }
  async function renderPreview(target: Page) { try { const blob = await requestBlob('/api/preview', 'POST', { page: target }); closePreview(); previewUrl = URL.createObjectURL(blob); } catch (cause) { error = String(cause); } }
  async function preview() { if (page) await renderPreview(page); }
  async function applyPersisted() { await requestJson('/api/apply', 'POST', {}); notice = 'Applied to LCD'; }
  async function apply() { try { await persistPageDraft(); await persistCarouselDraft(); await applyPersisted(); } catch (cause) { error = String(cause); } }
  async function orbit(sourceAssetId: string, displayName?: string) { const selectionGeneration = pageRequestGeneration; try { const preset = await requestJson<Preset>('/api/media/presets/orbit', 'POST', { source_asset_id: sourceAssetId, ...(displayName ? { display_name: displayName } : {}) }); presets = [...presets.filter((item) => item.id !== preset.id), preset]; await refreshPagesPreservingCarousel(); const splash = pages.find((item) => item.name === 'Splash'); if (splash && selectionGeneration === pageRequestGeneration) await loadPage(splash.id); await persistCarouselDraft(); await applyPersisted(); } catch (cause) { error = String(cause); } }
  async function previewOrbit() { const splash = pages.find((item) => item.name === 'Splash'); if (splash) await loadPage(splash.id).then(renderPreview).catch((cause) => error = String(cause)); }
  async function upload(files: File[]) { try { for (const file of files) { const form = new FormData(); form.append('file', file); const response = await fetch('/api/media', { method: 'POST', body: form }); if (!response.ok) throw new Error(`Upload: HTTP ${response.status}`); } await loadLibrary(); notice = `${files.length} asset${files.length === 1 ? '' : 's'} uploaded`; } catch (cause) { error = String(cause); } }
  async function replaceAsset(id: string, file: File) { try { const form = new FormData(); form.append('file', file); const response = await fetch(`/api/media/${id}`, { method: 'PUT', body: form }); if (!response.ok) throw new Error(`Replace: HTTP ${response.status}`); await loadLibrary(); notice = 'Asset replaced'; } catch (cause) { error = String(cause); } }
  async function deleteAsset(id: string) { if (!confirm('Delete this media asset?')) return; try { await requestJson(`/api/media/${id}`, 'DELETE'); await loadLibrary(); notice = 'Asset deleted'; } catch (cause) { error = `${String(cause)}. Assets used by a page are protected.`; } }
  function closePreview() { if (previewUrl) URL.revokeObjectURL(previewUrl); previewUrl = undefined; }
  onDestroy(closePreview);
</script>

<svelte:head><title>AooScope · LCD control studio</title></svelte:head>
<main>
  <header><div class="brand"><i></i><div><span>LCD CONTROL STUDIO</span><h1>AooScope</h1><p>Systems in focus</p></div></div><DisplayStatus {status} /></header>
  <Tabs tabs={tabs} {active} onselect={(tab) => active = tab} />
  {#if error}<div class="alert" role="alert"><span>{error}</span><button aria-label="Dismiss error" onclick={() => error = ''}>×</button></div>{/if}
  {#if notice}<p class="notice" aria-live="polite">{notice}</p>{/if}
  {#if active === 'Pages'}
    <section class="pages-layout">
      <PageStrip {pages} selected={page?.id} onselect={selectPage} onchange={(next) => pages = next} onduplicate={duplicatePage} onrestore={restorePage} ondelete={deletePage} onsave={saveCarousel} oncreate={() => createPage()} />
      <section class="workspace card">
        <div class="workspace-head"><div><span class="eyebrow">Visual designer</span><h2>{page?.name ?? 'Select a page'}</h2><p>Logical canvas · 960 × 376</p></div><div class="toolbar"><button class:active={metricLibraryOpen} onclick={() => { metricLibraryOpen = !metricLibraryOpen; templatesOpen = false; }}>+ Metric</button><button class:active={templatesOpen} onclick={() => { templatesOpen = !templatesOpen; metricLibraryOpen = false; }}>Templates</button><button onclick={savePage}>Save page</button><button onclick={preview}>Preview</button><button class="primary" onclick={apply}>Apply to LCD</button></div></div>
        {#if metricLibraryOpen}<div class="surface"><button class="surface-close" aria-label="Close metric library" onclick={() => metricLibraryOpen = false}>×</button><MetricLibrary {metrics} onadd={addMetric} /></div>{/if}
        {#if templatesOpen}<div class="surface"><button class="surface-close" aria-label="Close templates" onclick={() => templatesOpen = false}>×</button><TemplatePicker oncreate={createPage} /></div>{/if}
        <div class="designer"><div class="canvas-shell"><Canvas layers={page?.layers ?? []} {metrics} background={page?.background?.color} {selected} onselect={selectLayer} onchange={updateLayer} /></div>
          {#if selectedLayer}<ContextToolbar layer={selectedLayer} {metrics} onchange={(changes) => selected && updateLayer(selected, changes)} onadvanced={() => advancedOpen = true} onduplicate={duplicateLayer} ondelete={deleteLayer} />{/if}
          {#if selectedLayer && advancedOpen}<AdvancedInspector layer={selectedLayer} {metrics} onchange={(changes) => selected && updateLayer(selected, changes)} initialOpen onclose={() => advancedOpen = false} />{/if}
          <details class="widget-drawer"><summary>More widgets</summary><WidgetPalette onadd={addWidget} metrics={[]} {assets} /></details>
        </div>
        {#if previewUrl}<section class="preview" aria-label="Preview"><div><h2>Rendered preview</h2><button onclick={closePreview}>Close</button></div><img src={previewUrl} alt="Rendered LCD page preview" /></section>{/if}
      </section>
    </section>
  {:else if active === 'Media'}
    <section class="panel"><MediaDashboard event={media} {assets} {presets} onorbit={orbit} onpreview={previewOrbit} onupload={upload} onreplace={replaceAsset} ondelete={deleteAsset} /></section>
  {:else if active === 'Display'}
    <section class="panel"><DisplayControls {capabilities} powerOn={capabilities.power_on} brightness={status.brightness} settings={settings.display} settingsDocument={settings} onpower={(on) => capabilities = { ...capabilities, power_on: on }} onbrightness={(value) => status = { ...status, brightness: value }} onsaved={(document) => settings = document as SettingsDocument} /></section>
  {:else}
    <section class="panel"><ProvidersPanel catalog={providerCatalog} settings={settings.providers} statuses={providerStatuses} settingsDocument={settings} onsaved={(document) => settings = document as SettingsDocument} /></section>
  {/if}
</main>

<style>
  :global(*){box-sizing:border-box}:global(html){max-width:100%;overflow-x:hidden;background:#061019}:global(body){margin:0;min-width:0;max-width:100%;overflow-x:hidden;background:radial-gradient(circle at 48% -15%,#12334a 0,#081622 38%,#050d15 78%);color:#edf7fc;font-family:Inter,ui-sans-serif,system-ui,sans-serif;min-height:100vh}:global(button),:global(input),:global(select){font:inherit}:global(button){border:1px solid #315267;border-radius:8px;padding:7px 10px;background:#102533;color:#dfedf4;cursor:pointer}:global(button:hover){filter:brightness(1.15)}:global(button:disabled){cursor:not-allowed;opacity:.4}:global(input),:global(select){min-width:0;padding:8px 9px;border:1px solid #304f63;border-radius:8px;background:#07141e;color:#edf7fc}:global(label){color:#91a8b8;font-size:.75rem}main{width:min(1500px,100%);margin:auto;padding:22px 24px 60px;overflow:clip}header{display:flex;justify-content:space-between;align-items:center;gap:20px}.brand{display:flex;align-items:center;gap:13px}.brand>i{width:43px;height:43px;border:2px solid #35d9ff;border-radius:50%;box-shadow:0 0 22px #35d9ff33,inset 0 0 18px #35d9ff22;position:relative}.brand>i:after{content:'';position:absolute;width:8px;height:8px;left:16px;top:-5px;border-radius:50%;background:#58e5a4;box-shadow:0 0 10px #58e5a4}.brand span,.eyebrow{color:#35d9ff;font-size:.65rem;font-weight:800;letter-spacing:.16em}.brand h1{margin:1px 0;font-size:2rem;line-height:1}.brand p{margin:0;color:#758e9e;font-size:.72rem;letter-spacing:.12em;text-transform:uppercase}.alert{display:flex;justify-content:space-between;gap:12px;margin-top:14px;padding:10px 12px;border:1px solid #713b46;border-radius:10px;background:#2a151b;color:#ff9ba6}.alert button{border:0;background:none;padding:0}.notice{margin:10px 0 -2px;color:#58e5a4;font-size:.75rem}.card,.panel{border:1px solid #263f52;border-radius:18px;background:linear-gradient(145deg,#0e1e2bfa,#0a1621fa);box-shadow:0 20px 50px #0004,inset 0 1px #ffffff09}.pages-layout{display:grid;gap:12px;margin-top:18px}.card{padding:14px}.workspace{min-width:0}.workspace-head{display:flex;justify-content:space-between;align-items:end;gap:16px}.workspace-head h2{margin:4px 0}.workspace-head p{margin:0;color:#839baa;font-size:.75rem}.toolbar{display:flex;justify-content:flex-end;gap:7px;flex-wrap:wrap}.toolbar button.active{border-color:#35d9ff;background:#123548}.primary{background:#087fa5;border-color:#35d9ff;color:white}.surface{position:relative;margin-top:12px;padding:13px;border:1px solid #29495b;border-radius:13px;background:#07131d}.surface-close{position:absolute;z-index:2;top:7px;right:7px;border:0;background:none;font-size:1.1rem}.designer{display:grid;gap:10px;margin-top:12px}.canvas-shell{min-width:0;padding:10px;border:1px solid #213b4c;border-radius:13px;background:#030a10}.widget-drawer{border:1px solid #213b4c;border-radius:11px;background:#07131d}.widget-drawer summary{padding:9px 11px;cursor:pointer;color:#a8beca;font-size:.75rem}.preview{display:grid;gap:8px;margin-top:14px;padding-top:14px;border-top:1px solid #263f52}.preview div{display:flex;justify-content:space-between}.preview h2{margin:0}.preview img{width:100%;height:auto;border-radius:10px}.panel{margin-top:18px;padding:18px}@media(max-width:760px){main{padding:15px 12px 45px}header{align-items:flex-start}.pages-layout{margin-top:12px}.workspace-head{align-items:flex-start;flex-direction:column}.toolbar{width:100%;justify-content:flex-start}.toolbar button{flex:1 1 auto}.canvas-shell{padding:6px}.panel{padding:13px}}@media(max-width:480px){header{flex-direction:column}.brand h1{font-size:1.7rem}.toolbar button{flex-basis:30%}.toolbar .primary{flex-basis:50%}}
</style>

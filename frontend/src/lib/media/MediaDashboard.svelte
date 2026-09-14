<script lang="ts">
  import MediaCard from './MediaCard.svelte';
  import MediaLibrary, { type Preset } from './MediaLibrary.svelte';
  type Event = { mode?: 'playing'|'incoming'|'landed'|'offline'|'idle'; title?: string; poster_url?: string; progress_pct?: number; eta_minutes?: number; speed_bytes_s?: number; provider_chain?: string[] };
  let { event = {}, assets = [], presets = [], onorbit, onpreview, onupload, onreplace, ondelete }: { event?: Event; assets?: { id: string; name: string; kind?: string; format?: string; revision?: number; width?: number; height?: number }[]; presets?: Preset[]; onorbit: (sourceAssetId: string, displayName?: string) => void; onpreview: () => void; onupload?: (files: File[]) => void; onreplace?: (id: string, file: File) => void; ondelete?: (id: string) => void } = $props();
  const mode = $derived(event.mode ?? 'offline');
  const heading = $derived(mode === 'playing' ? 'Lecture en cours' : mode === 'incoming' ? 'Téléchargement en cours' : mode === 'landed' ? 'Récemment arrivé' : mode === 'idle' ? 'Média inactif' : 'Sources média hors ligne');
  const detail = $derived([event.provider_chain?.join(' + '), event.eta_minutes !== undefined ? `ETA ${event.eta_minutes} min` : undefined, event.speed_bytes_s !== undefined ? `${Math.round(event.speed_bytes_s / 1_000_000)} MB/s` : undefined].filter(Boolean).join(' · ') || (mode === 'offline' ? 'Providers unavailable' : 'No active media event'));
</script>
<section class="dashboard"><div class="live"><div class="playing"><span>Live</span><h2>{heading}</h2><MediaCard title={event.title ?? 'Aucune activité'} {detail} poster={event.poster_url} progress={event.progress_pct} /></div></div><MediaLibrary {assets} {presets} {onorbit} {onpreview} {onupload} {onreplace} {ondelete} /></section>
<style>.dashboard{display:grid;gap:20px}.live{display:grid;gap:14px}.playing{display:grid;gap:8px;padding:15px;border:1px solid #263f52;border-radius:16px;background:linear-gradient(145deg,#0d1c29,#09151f);box-shadow:0 14px 34px #0003}.playing>span{color:#35d9ff;font-size:.65rem;font-weight:800;letter-spacing:.14em;text-transform:uppercase}.dashboard h2{margin:0}</style>

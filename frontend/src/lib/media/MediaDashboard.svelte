<script lang="ts">
  import MediaCard from './MediaCard.svelte';
  import MediaLibrary, { type Preset } from './MediaLibrary.svelte';
  type Event = { mode?: 'playing'|'incoming'|'landed'|'offline'|'idle'; title?: string; poster_url?: string; progress_pct?: number; eta_minutes?: number; speed_bytes_s?: number; provider_chain?: string[] };
  let { event = {}, assets = [], presets = [], onorbit, onpreview }: { event?: Event; assets?: { id: string; name: string }[]; presets?: Preset[]; onorbit: (sourceAssetId: string, displayName?: string) => void; onpreview: () => void } = $props();
  const demo: Event = { mode: 'playing', title: 'Sample media — Offline demo', progress_pct: 64, provider_chain: ['Jellyfin · demo'] };
  const current = $derived(event.mode && event.mode !== 'idle' ? event : demo);
  const incoming = $derived(current.mode === 'incoming' ? current : { title: 'The Expanse · S02E04', progress_pct: 82, eta_minutes: 12, speed_bytes_s: 18_400_000, provider_chain: ['Sonarr', 'qBittorrent'] });
</script>
<section class="dashboard"><div class="playing"><h2>Lecture en cours</h2><MediaCard title={current.title ?? 'Aucun média'} detail={`${current.provider_chain?.join(' + ') ?? 'Media agrégé'} · ${current.mode ?? 'offline'}`} poster={current.poster_url} progress={current.progress_pct} /></div><div class="incoming"><h2>À venir</h2><MediaCard title={incoming.title ?? 'À venir'} detail={`${incoming.provider_chain?.join(' + ')} · ETA ${incoming.eta_minutes ?? 12} min · ${Math.round((incoming.speed_bytes_s ?? 18_400_000) / 1_000_000)} MB/s`} progress={incoming.progress_pct} /></div><MediaLibrary {assets} {presets} {onorbit} {onpreview} /></section>
<style>.dashboard{display:grid;gap:18px}.playing,.incoming{display:grid;gap:8px}.dashboard h2{margin:0}</style>

<script lang="ts">
  import MediaCard from './MediaCard.svelte';
  export type Preset = { id: string; name: string; source_asset_id: string; settings: { fps: number; speed_seconds: number } };
  let { presets = [], onorbit, onpreview }: { presets?: Preset[]; onorbit: () => void; onpreview: () => void } = $props();
</script>
<section aria-label="Bibliothèque média"><h2>Sources et animations</h2><div class="cards">
  {#each presets as preset}<MediaCard title={preset.name} detail={`Source ${preset.source_asset_id} · ${preset.settings.fps} FPS · ${preset.settings.speed_seconds}s`} />{/each}
  <div class="actions"><button onclick={onorbit}>Créer / migrer Cloud 9 · Orbit</button><button onclick={onpreview} disabled={!presets.some((preset) => preset.name === 'Cloud 9 · Orbit')}>Aperçu Orbit</button></div>
</div></section>
<style>section{display:grid;gap:12px}.cards{display:grid;gap:10px}.actions{display:flex;gap:8px;flex-wrap:wrap}button{justify-self:start;background:#102630;color:#cfefff;border:1px solid #5dd9ff;border-radius:7px;padding:9px}button:disabled{opacity:.5}</style>

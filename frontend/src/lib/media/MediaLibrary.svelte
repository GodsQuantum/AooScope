<script lang="ts">
  import MediaCard from './MediaCard.svelte';
  export type Preset = { id: string; name: string; source_asset_id: string; settings: { fps: number; speed_seconds: number } };
  let { assets = [], presets = [], onorbit, onpreview }: { assets?: { id: string; name: string }[]; presets?: Preset[]; onorbit: (sourceAssetId: string, displayName?: string) => void; onpreview: () => void } = $props();
  let sourceAssetId = $state('');
  let displayName = $state('');
</script>
<section aria-label="Bibliothèque média"><h2>Sources et animations</h2><div class="cards">
  {#each presets as preset}<MediaCard title={preset.name} detail={`Source ${preset.source_asset_id} · ${preset.settings.fps} FPS · ${preset.settings.speed_seconds}s`} />{/each}
  <label>Source <select bind:value={sourceAssetId}><option value="">Sélectionner un asset</option>{#each assets as asset}<option value={asset.id}>{asset.name}</option>{/each}</select></label>
  <label>Nom (optionnel) <input bind:value={displayName} placeholder="Orbit" /></label>
  <div class="actions"><button onclick={() => onorbit(sourceAssetId, displayName || undefined)} disabled={!sourceAssetId}>Créer / mettre à jour Orbit</button><button onclick={onpreview} disabled={!presets.some((preset) => preset.id === 'orbit')}>Aperçu Orbit</button></div>
</div></section>
<style>section{display:grid;gap:12px}.cards{display:grid;gap:10px}.actions{display:flex;gap:8px;flex-wrap:wrap}button{justify-self:start;background:#102630;color:#cfefff;border:1px solid #5dd9ff;border-radius:7px;padding:9px}button:disabled{opacity:.5}</style>

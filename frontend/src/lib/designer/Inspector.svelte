<script lang="ts">
  import BindingPicker from './BindingPicker.svelte';
  import type { Layer, Metric } from './model';
  let { layer, metrics, onchange }: { layer: Layer | undefined; metrics: Metric[]; onchange: (changes: Partial<Layer>) => void } = $props();
</script>
<section><h3>Inspecteur</h3>{#if layer}<label>Nom <input value={layer.text ?? ''} oninput={(e) => onchange({ text: e.currentTarget.value })} /></label><div class="geometry">{#each ['x','y','width','height'] as key}<label>{key}<input type="number" value={layer[key] as number} oninput={(e) => onchange({ [key]: Number(e.currentTarget.value) })} /></label>{/each}</div><label>Couleur <input type="color" value={layer.color ?? '#5dd9ff'} oninput={(e) => onchange({ color: e.currentTarget.value })} /></label>{#if !['text','image','animation'].includes(layer.type)}<BindingPicker metrics={metrics} widgetType={layer.type} value={layer.binding} onchange={(binding) => onchange({ binding })} />{/if}{:else}<p>Sélectionnez un widget.</p>{/if}</section>
<style>section{display:grid;gap:9px}label{display:grid;gap:4px;color:#aac1cc;font-size:.78rem}input{width:100%;padding:7px;background:#071019;color:#eaf8ff;border:1px solid #315363;border-radius:6px}.geometry{display:grid;grid-template-columns:1fr 1fr;gap:7px}input[type=color]{padding:1px;height:32px}</style>

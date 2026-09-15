<script lang="ts">
  import type { Layer, Metric } from './model';
  let { layer, metrics, onchange, initialOpen = false, onclose }: { layer: Layer; metrics: Metric[]; onchange: (changes: Partial<Layer>) => void; initialOpen?: boolean; onclose?: () => void } = $props();
  let open = $state(false);
  $effect(() => { open = initialOpen; });
  const metric = $derived(metrics.find((item) => item.id === layer.binding));
  const geometry = [['x', 'X position'], ['y', 'Y position'], ['width', 'Width'], ['height', 'Height'], ['z', 'Z index']] as const;
  function toggle() { open = !open; if (!open) onclose?.(); }
</script>

<section class="advanced" aria-label="Advanced inspector"><button class="toggle" aria-label="Advanced settings" onclick={toggle}>{open ? 'Close advanced settings' : 'Advanced settings'}</button>
  {#if open}<div class="drawer"><div class="heading"><div><span>Technical details</span><h3>{metric?.label ?? layer.text ?? 'Layer'}</h3></div><button aria-label="Close advanced settings" onclick={toggle}>×</button></div>
    {#if layer.binding}<label>Technical binding<code>{layer.binding}</code><input aria-label="Technical binding" value={layer.binding} oninput={(event) => onchange({ binding: event.currentTarget.value })} /></label>{/if}
    <div class="geometry">{#each geometry as [key, title]}<label>{title}<input aria-label={title} type="number" value={Number(layer[key] ?? (key === 'z' ? 1 : 0))} oninput={(event) => onchange({ [key]: Number(event.currentTarget.value) })} /></label>{/each}</div>
    <div class="range"><label>Minimum<input aria-label="Minimum" type="number" value={Number(layer.min_value ?? 0)} oninput={(event) => onchange({ min_value: Number(event.currentTarget.value) })} /></label><label>Maximum<input aria-label="Maximum" type="number" value={Number(layer.max_value ?? 100)} oninput={(event) => onchange({ max_value: Number(event.currentTarget.value) })} /></label></div>
  </div>{/if}
</section>

<style>
  .advanced{display:grid;gap:8px}.toggle{justify-self:end}.drawer{display:grid;gap:12px;padding:13px;border:1px solid #315363;border-radius:12px;background:#07131d}.heading{display:flex;justify-content:space-between}.heading span{color:#35d9ff;font-size:.64rem;font-weight:800;letter-spacing:.13em;text-transform:uppercase}h3{margin:3px 0 0}.heading button{border:0;background:none;font-size:1.2rem}.drawer>label,.geometry label,.range label{display:grid;gap:4px}.drawer code{overflow:hidden;color:#d4b6ff;font-size:.72rem;text-overflow:ellipsis;white-space:nowrap}.geometry{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:7px}.range{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}@media(max-width:620px){.geometry{grid-template-columns:repeat(2,minmax(0,1fr))}}
</style>

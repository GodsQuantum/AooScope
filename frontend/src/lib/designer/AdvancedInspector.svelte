<script lang="ts">
  import type { Layer, Metric } from './model';
  let { layer, metrics, onchange, initialOpen = false, onclose }: { layer: Layer; metrics: Metric[]; onchange: (changes: Partial<Layer>) => void; initialOpen?: boolean; onclose?: () => void } = $props();
  let open = $state(false);
  $effect(() => { open = initialOpen; });
  const metric = $derived(metrics.find((item) => item.id === layer.binding));
  const geometry = [['x', 'X position'], ['y', 'Y position'], ['width', 'Width'], ['height', 'Height'], ['z', 'Z index']] as const;
  const isMedia = $derived(layer.type === 'image' || layer.type === 'animation');
  const isStar = $derived(layer.type === 'shooting_star');
  function toggle() { open = !open; if (!open) onclose?.(); }
  function geometryChange(key: 'x'|'y'|'width'|'height'|'z', raw: number) {
    const value = Math.round(raw);
    if (isMedia && layer.lock_aspect !== false && (key === 'width' || key === 'height')) {
      const ratio = Number(layer.aspect_ratio ?? layer.width / Math.max(layer.height, 1));
      if (Number.isFinite(ratio) && ratio > 0) {
        onchange(key === 'width' ? { width: value, height: Math.max(1, Math.round(value / ratio)) } : { height: value, width: Math.max(1, Math.round(value * ratio)) });
        return;
      }
    }
    onchange({ [key]: value });
  }
</script>

<section class="advanced" aria-label="Advanced inspector"><button class="toggle" aria-label="Advanced settings" onclick={toggle}>{open ? 'Close advanced settings' : 'Advanced settings'}</button>
  {#if open}<div class="drawer"><div class="heading"><div><span>Technical details</span><h3>{metric?.label ?? layer.text ?? (isStar ? 'Shooting star' : 'Layer')}</h3></div><button aria-label="Close advanced settings" onclick={toggle}>×</button></div>
    {#if layer.binding}<label>Technical binding<code>{layer.binding}</code><input aria-label="Technical binding" value={layer.binding} oninput={(event) => onchange({ binding: event.currentTarget.value })} /></label>{/if}
    <div class="geometry">{#each geometry as [key, title]}<label>{title}<input aria-label={title} type="number" step="1" value={Number(layer[key] ?? (key === 'z' ? 1 : 0))} oninput={(event) => geometryChange(key, Number(event.currentTarget.value))} /></label>{/each}</div>
    {#if isMedia}<div class="options media-options">
      <label>Image fit<select aria-label="Image fit" value={String(layer.fit ?? 'contain')} onchange={(event) => onchange({ fit: event.currentTarget.value })}><option value="contain">Contain</option><option value="cover">Cover</option><option value="stretch">Stretch</option></select></label>
      <label>Horizontal anchor<select aria-label="Horizontal anchor" value={String(layer.align ?? 'center')} onchange={(event) => onchange({ align: event.currentTarget.value })}><option value="left">Left</option><option value="center">Center</option><option value="right">Right</option></select></label>
      <label>Vertical anchor<select aria-label="Vertical anchor" value={String(layer.valign ?? 'center')} onchange={(event) => onchange({ valign: event.currentTarget.value })}><option value="top">Top</option><option value="center">Center</option><option value="bottom">Bottom</option></select></label>
      <label class="check"><input aria-label="Lock aspect ratio" type="checkbox" checked={layer.lock_aspect !== false} onchange={(event) => onchange({ lock_aspect: event.currentTarget.checked, aspect_ratio: Number(layer.aspect_ratio ?? layer.width / Math.max(layer.height, 1)) })} />Lock aspect ratio</label>
    </div>{/if}
    {#if isStar}<div class="options star-options">
      <label>Star speed<input aria-label="Star speed" type="number" min="0.5" step="0.5" value={Number(layer.speed_seconds ?? 3)} oninput={(event) => onchange({ speed_seconds: Number(event.currentTarget.value) })} /></label>
      <label>Trail length<input aria-label="Trail length" type="number" min="8" max="400" value={Number(layer.trail_length ?? 90)} oninput={(event) => onchange({ trail_length: Number(event.currentTarget.value) })} /></label>
      <label>Angle<input aria-label="Angle" type="number" min="-180" max="180" value={Number(layer.angle_deg ?? -18)} oninput={(event) => onchange({ angle_deg: Number(event.currentTarget.value) })} /></label>
      <label>Star size<input aria-label="Star size" type="number" min="1" max="20" value={Number(layer.size ?? 5)} oninput={(event) => onchange({ size: Number(event.currentTarget.value) })} /></label>
      <label>Compositing<select aria-label="Compositing" value={Number(layer.z ?? 1) < 0 ? 'behind' : 'front'} onchange={(event) => onchange({ z: event.currentTarget.value === 'behind' ? -10 : 50 })}><option value="front">Front</option><option value="behind">Behind</option></select></label>
      <label>Opacity<input aria-label="Star opacity" type="range" min="0" max="1" step="0.05" value={Number(layer.opacity ?? 1)} oninput={(event) => onchange({ opacity: Number(event.currentTarget.value) })} /></label>
    </div>{/if}
    {#if !isMedia && !isStar}<div class="range"><label>Minimum<input aria-label="Minimum" type="number" value={Number(layer.min_value ?? 0)} oninput={(event) => onchange({ min_value: Number(event.currentTarget.value) })} /></label><label>Maximum<input aria-label="Maximum" type="number" value={Number(layer.max_value ?? 100)} oninput={(event) => onchange({ max_value: Number(event.currentTarget.value) })} /></label></div>{/if}
  </div>{/if}
</section>

<style>
  .advanced{display:grid;gap:8px}.toggle{justify-self:end}.drawer{display:grid;gap:12px;padding:13px;border:1px solid #315363;border-radius:12px;background:#07131d}.heading{display:flex;justify-content:space-between}.heading span{color:#35d9ff;font-size:.64rem;font-weight:800;letter-spacing:.13em;text-transform:uppercase}h3{margin:3px 0 0}.heading button{border:0;background:none;font-size:1.2rem}.drawer>label,.geometry label,.range label,.options label{display:grid;gap:4px}.drawer code{overflow:hidden;color:#d4b6ff;font-size:.72rem;text-overflow:ellipsis;white-space:nowrap}.geometry{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:7px}.range{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}.options{display:grid;grid-template-columns:repeat(auto-fit,minmax(145px,1fr));gap:8px;padding-top:10px;border-top:1px solid #20394a}.check{display:flex!important;align-items:center;gap:7px}.check input{width:auto}@media(max-width:620px){.geometry{grid-template-columns:repeat(2,minmax(0,1fr))}}
</style>

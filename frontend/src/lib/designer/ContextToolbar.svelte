<script lang="ts">
  import type { Layer, Metric, WidgetType } from './model';
  let { layer, metrics, onchange, onadvanced, onduplicate, ondelete }: { layer: Layer; metrics: Metric[]; onchange: (changes: Partial<Layer>) => void; onadvanced: () => void; onduplicate?: () => void; ondelete?: () => void } = $props();
  const widgetNames: Record<WidgetType, string> = { text: 'Text', value: 'Value', gauge: 'Gauge', ring: 'Ring', bar: 'Bar', badge: 'Status', sparkline: 'Chart', image: 'Image', animation: 'Animation' };
  const bindableTypes: WidgetType[] = ['text', 'value', 'gauge', 'ring', 'bar', 'badge'];
  const metric = $derived(metrics.find((item) => item.id === layer.binding));
  const metricChoices = $derived(metrics.filter((item) => item.recommended_widgets.includes(layer.type)));
  const compatible = $derived((metric?.recommended_widgets ?? [layer.type]).filter((type): type is WidgetType => ['text','value','gauge','ring','bar','badge','sparkline','image','animation'].includes(type)));
  const label = $derived(layer.text ?? metric?.label ?? (layer.binding?.startsWith('aooscope_') ? 'Metric' : layer.binding) ?? widgetNames[layer.type]);
  const styles = $derived(layer.type === 'bar' ? [['Horizontal', 'horizontal'], ['Vertical', 'vertical']] : layer.type === 'gauge' || layer.type === 'ring' ? [['Slim', '8'], ['Default', '18'], ['Bold', '28']] : layer.type === 'text' || layer.type === 'value' ? [['Left', 'left'], ['Center', 'center'], ['Right', 'right']] : layer.type === 'badge' ? [['Compact', '4'], ['Rounded', '12'], ['Pill', '999']] : []);
  const styleValue = $derived(layer.type === 'bar' ? (layer.orientation === 'vertical' ? 'vertical' : 'horizontal') : layer.type === 'gauge' || layer.type === 'ring' ? Number(layer.thickness ?? 18) <= 10 ? '8' : Number(layer.thickness ?? 18) >= 24 ? '28' : '18' : layer.type === 'text' || layer.type === 'value' ? (['center', 'right'].includes(String(layer.align)) ? String(layer.align) : 'left') : layer.type === 'badge' ? Number(layer.radius ?? 12) >= Math.min(layer.width, layer.height) / 2 ? '999' : Number(layer.radius ?? 12) <= 6 ? '4' : '12' : '');
  function changeStyle(value: string) {
    if (layer.type === 'bar') onchange({ orientation: value });
    else if (layer.type === 'gauge' || layer.type === 'ring') onchange({ thickness: Number(value) });
    else if (layer.type === 'text' || layer.type === 'value') onchange({ align: value });
    else if (layer.type === 'badge') onchange({ radius: Number(value) });
  }
</script>

<section class="context" aria-label="Selected layer tools"><strong>{label}</strong>
  {#if bindableTypes.includes(layer.type)}<label>Source<select aria-label="Metric source" value={metric?.id ?? ''} onchange={(event) => onchange(layer.type === 'text' ? { binding: event.currentTarget.value, text: undefined } : { binding: event.currentTarget.value })}>{#if !metric}<option value="" disabled>Select metric</option>{/if}{#each metricChoices as item}<option value={item.id}>{item.label} · {item.provider_name}</option>{/each}</select></label>{/if}
  <div class="representations" aria-label="Representation">{#each compatible as type}<button class:active={type === layer.type} aria-label={widgetNames[type]} onclick={() => onchange({ type })}>{widgetNames[type]}</button>{/each}</div>
  <label>Color<input aria-label="Color" type="color" value={layer.color ?? '#5dd9ff'} oninput={(event) => onchange({ color: event.currentTarget.value })} /></label>
  {#if styles.length}<label>Style<select aria-label="Style" value={styleValue} onchange={(event) => changeStyle(event.currentTarget.value)}>{#each styles as [name, value]}<option {value}>{name}</option>{/each}</select></label>{/if}
  <div class="actions">{#if onduplicate}<button onclick={onduplicate}>Duplicate</button>{/if}{#if ondelete}<button class="danger" onclick={ondelete}>Delete</button>{/if}<button onclick={onadvanced}>Advanced</button></div>
</section>

<style>
  .context,.representations,.actions,label{display:flex;align-items:center}.context{flex-wrap:wrap;gap:8px;padding:9px 10px;border:1px solid #315363;border-radius:11px;background:#0b1a25}.context>strong{margin-right:auto;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.representations,.actions{gap:5px}.context button{padding:5px 8px;font-size:.7rem}.context button.active{border-color:#35d9ff;background:#123548}label{gap:5px;color:#91a8b8;font-size:.68rem}select{padding:5px 7px}input[type=color]{width:34px;height:29px;padding:2px}.danger{color:#ff8f9b;border-color:#673441!important}@media(max-width:620px){.context>strong{width:100%;margin:0}.context{align-items:stretch}.representations{flex-wrap:wrap}.actions{margin-left:auto}}
</style>

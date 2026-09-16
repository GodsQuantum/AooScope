<script lang="ts">
  import { onMount } from 'svelte';
  import { CANVAS_HEIGHT, CANVAS_WIDTH, clampRect, inversePointer, resizeRect, type Layer, type Metric } from './model';
  let { layers, metrics = [], background = '#071019', selected, onselect, onchange }: { layers: Layer[]; metrics?: Metric[]; background?: string; selected?: string; onselect: (id: string) => void; onchange: (id: string, changes: Partial<Layer>) => void } = $props();
  let host: HTMLDivElement; let canvas: HTMLDivElement; let scale = $state(1); let drag: { id: string; dx: number; dy: number; mode: 'move' | 'resize'; start?: Layer } | undefined;
  onMount(() => { const resize = () => { const rect = host.getBoundingClientRect(); scale = Math.min(Math.max(rect.width / CANVAS_WIDTH, 0.01), 1); }; const observer = new ResizeObserver(resize); observer.observe(host); resize(); return () => observer.disconnect(); });
  function move(event: PointerEvent) { if (!drag) return; const point = inversePointer(canvas.getBoundingClientRect(), event.clientX, event.clientY); const layer = layers.find((item) => item.id === drag?.id); if (!layer) return; onchange(layer.id, drag.mode === 'move' ? clampRect({ ...layer, x: point.x - drag.dx, y: point.y - drag.dy }) : resizeRect(drag.start ?? layer, { x: point.x - drag.dx, y: point.y - drag.dy })); }
  function start(event: PointerEvent, layer: Layer) { const point = inversePointer(canvas.getBoundingClientRect(), event.clientX, event.clientY); drag = { id: layer.id, dx: point.x - layer.x, dy: point.y - layer.y, mode: 'move' }; onselect(layer.id); (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId); }
  function startResize(event: PointerEvent, layer: Layer) { event.stopPropagation(); const point = inversePointer(canvas.getBoundingClientRect(), event.clientX, event.clientY); drag = { id: layer.id, dx: point.x, dy: point.y, mode: 'resize', start: layer }; onselect(layer.id); (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId); }
  const widgetNames: Record<Layer['type'], string> = { text: 'Text', value: 'Value', gauge: 'Gauge', ring: 'Ring', bar: 'Bar', badge: 'Status', sparkline: 'Chart', image: 'Image', animation: 'Animation' };
  const metricFor = (layer: Layer) => metrics.find((metric) => metric.id === layer.binding);
  const layerLabel = (layer: Layer, metric?: Metric) => layer.text ?? metric?.label ?? layer.binding ?? `${widgetNames[layer.type]} · ${layer.id}`;
  const compact = (value: number, decimals: number) => value.toFixed(decimals).replace(/\.0+$|(?<=\.[0-9]*?)0+$/g, '').replace(/\.$/, '');
  const bytes = (value: number) => value >= 1e12 ? `${compact(value / 1e12, 1)} TB` : value >= 1e9 ? `${compact(value / 1e9, value >= 1e11 ? 0 : 1)} GB` : value >= 1e6 ? `${compact(value / 1e6, value >= 1e8 ? 0 : 1)} MB` : value >= 1e3 ? `${Math.round(value / 1e3)} KB` : `${Math.round(value)} B`;
  function metricText(layer: Layer, metric?: Metric) {
    const raw = metric?.value ?? metric?.demo_value;
    const fallback = String(layer.fallback ?? (layer.type === 'value' ? '--' : ''));
    if (raw === undefined || raw === null) return fallback;
    const unit = String(layer.unit ?? metric?.unit ?? '');
    if (typeof raw === 'number') {
      if ((layer.format === 'bytes' || layer.format === 'bytes_per_second') && raw <= 0) return fallback;
      if (layer.format === 'bytes') return bytes(raw);
      if (layer.format === 'bytes_per_second') return `${bytes(raw)}/S`;
      return `${compact(raw, ['%', '°', '°C', ' C'].includes(unit) ? 0 : 1)}${unit}`;
    }
    return `${String(raw)}${unit}`;
  }
  function visibleContent(layer: Layer, metric?: Metric) {
    if (layer.type === 'text') return layer.text !== undefined ? String(layer.text) : metricText(layer, metric);
    if (layer.type === 'value') return metricText(layer, metric);
    if (layer.type === 'badge') return layer.text !== undefined ? String(layer.text) : metricText(layer, metric);
    return '';
  }
  const barFill = (layer: Layer, metric?: Metric) => { const raw = metric?.value ?? metric?.demo_value ?? layer.value; const value = typeof raw === 'number' ? raw : Number(raw); if (!Number.isFinite(value)) return '0%'; const min = Number(layer.min_value ?? 0); const max = Number(layer.max_value ?? 100); return `${Math.max(0, Math.min(100, (value - min) * 100 / Math.max(max - min, Number.EPSILON)))}%`; };
  const horizontalPlacement = (layer: Layer) => layer.align === 'right' ? 'end' : layer.align === 'center' ? 'center' : 'start';
  const verticalPlacement = (layer: Layer) => layer.valign === 'bottom' ? 'end' : layer.valign === 'center' ? 'center' : 'start';
  const textSize = (layer: Layer) => Number(layer.size ?? Number(layer.scale ?? Math.max(1, Math.min(8, Math.floor(layer.height / 8)))) * 8);
</script>
<div class="host" role="application" bind:this={host} onpointermove={move} onpointerup={() => drag = undefined} data-testid="canvas-host"><div class="stage" style={`width:${CANVAS_WIDTH * scale}px;height:${CANVAS_HEIGHT * scale}px`}><div class="canvas" bind:this={canvas} data-testid="logical-canvas" style={`width:${CANVAS_WIDTH}px;height:${CANVAS_HEIGHT}px;transform:scale(${scale});transform-origin:top left;background:${background}`}>
  {#each layers as layer (layer.id)}{@const metric = metricFor(layer)}<div class:selected={layer.id === selected} class:badge={layer.type === 'badge'} class:gauge={layer.type === 'gauge' || layer.type === 'ring'} class:bar={layer.type === 'bar'} class:vertical={layer.type === 'bar' && layer.orientation === 'vertical'} class:textual={layer.type === 'text' || layer.type === 'value' || layer.type === 'badge'} class="layer" role="button" tabindex="0" aria-label={`Select ${layerLabel(layer, metric)}`} data-layer={layer.id} style={`left:${layer.x}px;top:${layer.y}px;width:${layer.width}px;height:${layer.height}px;z-index:${layer.z ?? 1};color:${layer.color ?? '#eaf8ff'};background:${String(layer.background_color ?? '')};border-color:${String(layer.border_color ?? '')};border-radius:${Number(layer.radius ?? 0)}px;border-width:${layer.type === 'gauge' || layer.type === 'ring' ? Number(layer.thickness ?? 18) : 1}px;text-align:${String(layer.align ?? 'left')};justify-items:${horizontalPlacement(layer)};align-content:${verticalPlacement(layer)};font-size:${textSize(layer)}px;--fill:${barFill(layer, metric)}`} onpointerdown={(e) => start(e, layer)} onclick={() => onselect(layer.id)} onkeydown={(e) => e.key === 'Enter' && onselect(layer.id)}>{#if visibleContent(layer, metric)}<span class="content">{visibleContent(layer, metric)}</span>{/if}{#if layer.id === selected}<span class="resize-handle" role="button" tabindex="0" aria-label={`Redimensionner ${layer.id}`} onpointerdown={(e) => startResize(e, layer)}></span>{/if}</div>{/each}
</div></div></div>
<style>
  .host{display:grid;place-items:center;width:100%;max-height:376px;overflow:hidden;aspect-ratio:960/376;background:#03090e;border-radius:12px;box-shadow:0 18px 55px #0008}.stage{position:relative;flex:none}.canvas{position:absolute;inset:0;box-shadow:0 0 0 1px #315363,inset 0 0 80px #00131f}.layer{position:absolute;display:grid;border-color:transparent;border-style:dashed;color:inherit;overflow:hidden;font-weight:700;line-height:1.08}.content{overflow:hidden;max-width:100%;text-overflow:ellipsis;white-space:nowrap}.layer.badge{border-style:solid}.layer.gauge{border-style:solid;border-color:currentColor!important;border-top-color:#1c2b3b!important;border-radius:50%!important;background:#0b1722}.layer.bar{background:linear-gradient(90deg,currentColor 0 var(--fill,0%),#1c2b3b var(--fill,0%))!important;border-radius:999px!important}.layer.bar.vertical{background:linear-gradient(0deg,currentColor 0 var(--fill,0%),#1c2b3b var(--fill,0%))!important}.layer.selected{outline:2px solid #5dd9ff;outline-offset:2px}.resize-handle{position:absolute;right:0;bottom:0;width:12px;height:12px;background:#5dd9ff;border:2px solid #071019;cursor:nwse-resize}
</style>

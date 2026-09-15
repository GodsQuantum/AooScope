<script lang="ts">
  import type { Metric, WidgetType } from './model';
  let { metrics, onadd }: { metrics: Metric[]; onadd: (metric: Metric, widgetType: WidgetType) => void } = $props();
  let query = $state('');
  const matches = $derived(metrics.filter((metric) => `${metric.label} ${metric.provider_name} ${metric.category}`.toLowerCase().includes(query.toLowerCase())));
  const widgetName = (type: string) => ({ gauge: 'Gauge', ring: 'Ring', bar: 'Bar', value: 'Value', badge: 'Status', sparkline: 'Chart' }[type] ?? 'Value');
  const preferred = (metric: Metric) => (metric.recommended_widgets[0] as WidgetType | undefined) ?? 'value';
  const reading = (metric: Metric) => `${metric.value ?? metric.demo_value ?? '—'}${metric.unit ? ` ${metric.unit}` : ''}`;
</script>

<section class="library" aria-label="Metric library"><div class="heading"><div><span class="eyebrow">Data first</span><h3>Metric library</h3></div><label>Search <input aria-label="Search metrics" placeholder="CPU, storage, provider…" bind:value={query} /></label></div>
  <div class="metrics">{#each matches as metric}<article><div><strong>{metric.label}</strong><span>{metric.provider_name} · {metric.category}</span></div><div class="reading"><b>{reading(metric)}</b><small>Recommended · {widgetName(preferred(metric))}</small></div><button aria-label={`Add ${metric.label}`} onclick={() => onadd(metric, preferred(metric))}>Add</button></article>{:else}<p>No matching metrics</p>{/each}</div>
</section>

<style>
  .library{display:grid;gap:12px}.heading,.heading>div,article,.reading{display:flex}.heading{align-items:end;justify-content:space-between;gap:14px}.heading>div{align-items:baseline;gap:10px}.eyebrow{color:#35d9ff;font-size:.64rem;font-weight:800;letter-spacing:.14em;text-transform:uppercase}h3{margin:0}label{display:grid;gap:4px;min-width:min(260px,100%)}.metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(250px,100%),1fr));gap:8px}article{align-items:center;gap:10px;min-width:0;padding:10px;border:1px solid #244858;border-radius:11px;background:#0a1a25}article>div:first-child{display:grid;min-width:0;flex:1}article span,small{overflow:hidden;color:#89a4b4;font-size:.68rem;text-overflow:ellipsis;white-space:nowrap}.reading{display:grid;text-align:right}.reading b{color:#f4fbff}.reading small{color:#58dcb0}article button{align-self:stretch}@media(max-width:620px){.heading{align-items:stretch;flex-direction:column}.heading label{min-width:0}article{display:grid;grid-template-columns:minmax(0,1fr) auto}.reading{text-align:left}article button{grid-column:2;grid-row:1/3}}
</style>

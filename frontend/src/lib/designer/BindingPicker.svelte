<script lang="ts">
  import ProviderBadge from '$lib/components/ProviderBadge.svelte';
  import { compatibleMetrics, type Metric, type WidgetType } from './model';
  let { metrics, value, widgetType, onchange }: { metrics: Metric[]; value?: string; widgetType: WidgetType; onchange?: (id: string) => void } = $props();
  let query = $state('');
  const matches = $derived(compatibleMetrics(metrics, widgetType).filter((m) => `${m.label} ${m.provider_name} ${m.category} ${m.unit}`.toLowerCase().includes(query.toLowerCase())));
</script>
<div class="picker">
  <label>Source métrique <input aria-label="Rechercher une métrique" placeholder="Rechercher…" bind:value={query} /></label>
  <div class="options">
    {#each matches as metric}
      <button class:selected={metric.id === value} type="button" onclick={() => onchange?.(metric.id)}>
        <ProviderBadge provider={metric.provider_name} category={metric.category} />
        <strong>{metric.label}</strong><span>{metric.value ?? metric.demo_value ?? '—'} {metric.unit}</span>
      </button>
    {:else}<small>Aucune métrique compatible</small>{/each}
  </div>
</div>
<style>.picker{display:grid;gap:8px}label{display:grid;gap:4px;font-size:.8rem;color:#aac1cc}input{width:100%;padding:8px;background:#071019;color:#eaf8ff;border:1px solid #315363;border-radius:7px}.options{display:grid;gap:5px;max-height:180px;overflow:auto}button{display:grid;gap:3px;text-align:left;padding:8px;background:#102630;color:#eaf8ff;border:1px solid #244858;border-radius:7px}button.selected{border-color:#5dd9ff}button span{color:#b9d0d9;font-size:.8rem}</style>

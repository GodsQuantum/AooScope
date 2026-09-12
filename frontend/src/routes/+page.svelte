<script lang="ts">
  import { onMount } from 'svelte';
  import type { components } from '$lib/api/schema';
  import { getJson } from '$lib/api/client';
  import DisplayStatus from '$lib/components/DisplayStatus.svelte';
  import Tabs from '$lib/components/Tabs.svelte';

  type StatusDto = components['schemas']['StatusDto'];
  const tabs = ['Pages', 'Media', 'Display', 'Providers'] as const;
  let active = $state<string>('Pages');
  let status = $state<StatusDto>({
    version: '0.3.0-dev', brightness: 100, native_brightness: false,
    device_present: false, updated_unix: null
  });
  let error = $state('');
  const connectionLabel = $derived(status.device_present ? 'Display online' : 'Display offline');

  onMount(async () => {
    try { status = await getJson('/api/status'); }
    catch (err) { error = err instanceof Error ? err.message : String(err); }
  });
</script>

<svelte:head><title>AooScope</title></svelte:head>
<main>
  <header><div><h1>AooScope</h1><p>{connectionLabel}</p></div><DisplayStatus {status} /></header>
  <Tabs tabs={tabs} {active} onselect={(tab) => active = tab} />
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <section class="workspace"><h2>{active}</h2><p>Rust + Svelte compatibility foundation</p></section>
</main>

<style>
  :global(*){box-sizing:border-box} :global(body){margin:0;background:#071019;color:#eaf8ff;font-family:Inter,system-ui,sans-serif}
  main{max-width:1200px;margin:auto;padding:24px} header{display:flex;justify-content:space-between;gap:24px;align-items:center}
  h1{margin:0;font-size:2rem} p{color:#9db5c5}.workspace{margin-top:24px;border:1px solid #1e3948;border-radius:16px;padding:24px;background:#0b1720}
  :global(.tabs){display:flex;gap:8px;margin-top:24px;flex-wrap:wrap}:global(.tabs button){background:#102630;color:#cfefff;border:1px solid #244858;border-radius:10px;padding:10px 14px}:global(.tabs button.active){background:#183b49;border-color:#5dd9ff}
  :global(.status-card){display:grid;gap:3px;text-align:right}.error{color:#ff9a9a}
</style>

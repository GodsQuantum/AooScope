<script lang="ts">
  import { requestJson } from '$lib/api/client';
  type Descriptor = { id: string; name: string; icon: string; categories: string[]; credential_fields: string[] };
  type Provider = { enabled: boolean; url: string; verify_tls: boolean; secret_set: boolean; node?: string | null; system?: string | null; [key: string]: unknown };
  type ProviderStatus = { id: string; configured: boolean; enabled: boolean; online: boolean; last_success: number | null; error: string | null };
  let { catalog, settings, statuses, settingsDocument, onsaved }: { catalog: Descriptor[]; settings: Record<string, Provider>; statuses: ProviderStatus[]; settingsDocument?: Record<string, unknown>; onsaved?: (document: Record<string, unknown>) => void } = $props();
  let draft = $state<Record<string, Provider>>({});
  let results = $state<Record<string, ProviderStatus>>({});
  let message = $state('');
  $effect(() => { draft = Object.fromEntries(catalog.map((item) => [item.id, Object.assign({ enabled: false, url: '', verify_tls: true, secret_set: false }, settings[item.id])])); results = Object.fromEntries(statuses.map((status) => [status.id, status])); });
  const labels: Record<string, string> = { api_token: 'API token', token_id: 'Token ID', token_secret: 'Token secret', api_key: 'API key', username: 'Username', email: 'Email', password: 'Password' };
  const icon: Record<string, string> = { local: '⌁', proxmox: '▦', beszel: '◉', jellyfin: '▶', silo: '◆', radarr: '◐', sonarr: '◒', qbittorrent: '⇣', immich: '✦', ollama: '●' };
  const statusFor = (id: string) => results[id] ?? statuses.find((item) => item.id === id);
  function displayDate(value: number | null | undefined) { return value ? new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(value * 1000) : 'Never'; }
  async function save() {
    message = 'Saving…';
    try {
      const clean = Object.fromEntries(Object.entries(draft).map(([id, value]) => [id, Object.fromEntries(Object.entries(value).filter(([key, field]) => key !== 'secret_set' && field !== ''))]));
      const document = await requestJson<Record<string, unknown>>('/api/settings', 'PUT', { ...(settingsDocument ?? {}), providers: clean });
      message = 'Providers saved'; onsaved?.(document);
      for (const item of catalog) for (const field of item.credential_fields) if (draft[item.id]) draft[item.id][field] = '';
      return true;
    } catch (cause) { message = String(cause); return false; }
  }
  async function test(id: string) {
    message = `Testing ${id}…`;
    try { if (!await save()) return; results[id] = await requestJson<ProviderStatus>(`/api/providers/${id}/test`, 'POST', {}); message = results[id].online ? `${id} is online` : (results[id].error ?? `${id} is offline`); }
    catch (cause) { message = String(cause); }
  }
</script>

<section class="providers">
  <div class="section-head"><div><span class="eyebrow">Data sources</span><h2>Providers</h2><p>Connect services without exposing saved credentials.</p></div><button class="primary" onclick={save}>Save providers</button></div>
  <div class="grid">
    {#each catalog as item (item.id)}
      {@const config = draft[item.id]}
      {@const state = statusFor(item.id)}
      {#if config}<article class="provider-card">
        <div class="provider-head"><div class="identity"><i>{icon[item.id] ?? '⌁'}</i><div><h3>{item.name}</h3><small>{item.categories.join(' · ') || 'system'}</small></div></div><span class:online={state?.online} class:error={!!state?.error} class="state">{state?.online ? 'Online' : state?.error ? 'Error' : config.enabled ? 'Offline' : 'Disabled'}</span></div>
        {#if item.id !== 'local'}
          <label>URL<input aria-label={`${item.name} URL`} bind:value={config.url} placeholder="https://service.example" /></label>
          {#if item.id === 'proxmox'}<label>Node<input aria-label="Node" bind:value={config.node} placeholder="Optional node" /></label>{/if}
          {#if item.id === 'beszel'}<label>System<input aria-label="System" bind:value={config.system} placeholder="Optional system" /></label>{/if}
          {#each item.credential_fields as field}<label>{labels[field] ?? field}<input aria-label={labels[field] ?? field} type={field.includes('password') || field.includes('token') || field.includes('key') ? 'password' : 'text'} bind:value={config[field]} placeholder={config.secret_set ? 'Leave blank to keep saved value' : ''} /></label>{/each}
          <div class="secret"><span class:present={config.secret_set}>{config.secret_set ? 'Credential saved' : 'No credential saved'}</span><label class="check"><input type="checkbox" bind:checked={config.verify_tls} />Verify TLS</label></div>
          <div class="card-actions"><label class="check"><input type="checkbox" bind:checked={config.enabled} />Enabled</label><button aria-label={`Test ${item.name}`} onclick={() => test(item.id)}>Test connection</button></div>
        {:else}<p class="builtin">Built in · no credentials required</p>{/if}
        <footer><span>{state?.error ?? (state?.configured ? 'Latest collection healthy' : 'Not configured')}</span><small>Last success · {displayDate(state?.last_success)}</small></footer>
      </article>{/if}
    {/each}
  </div>
  <p class="message" aria-live="polite">{message}</p>
</section>

<style>
  .providers{display:grid;gap:18px}.section-head{display:flex;justify-content:space-between;align-items:end;gap:20px}.section-head h2{margin:3px 0}.section-head p{margin:0;color:#91a8b8}.eyebrow{color:#35d9ff;font-size:.7rem;font-weight:800;letter-spacing:.14em;text-transform:uppercase}.primary{background:#087fa5;border-color:#35d9ff;padding:10px 15px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,300px),1fr));gap:14px}.provider-card{display:grid;gap:11px;padding:16px;border:1px solid #263f52;border-radius:16px;background:linear-gradient(145deg,#0d1c29,#0a1621);box-shadow:0 14px 34px #0003,inset 0 1px #ffffff09}.provider-head,.identity,.secret,.card-actions{display:flex;align-items:center;justify-content:space-between;gap:10px}.identity{justify-content:flex-start}.identity i{display:grid;place-items:center;width:40px;height:40px;border:1px solid #27566d;border-radius:12px;background:#0b2938;color:#51defd;font-style:normal;font-size:1.25rem}.identity h3{margin:0}.identity small{color:#7892a3}.state{padding:5px 9px;border:1px solid #405162;border-radius:999px;color:#91a8b8;font-size:.7rem;font-weight:800}.state.online{color:#59e3a0;border-color:#25664c;background:#123329}.state.error{color:#ff8693;border-color:#713641;background:#30171c}label{display:grid;gap:5px;color:#91a8b8;font-size:.75rem}input{width:100%}.check{display:flex;align-items:center;gap:6px}.check input{width:auto}.secret{font-size:.72rem;color:#758d9e}.present{color:#55d99a}.card-actions button{padding:7px 10px}.builtin{color:#91a8b8}footer{display:grid;gap:3px;border-top:1px solid #213747;padding-top:10px;color:#9cb3c1;font-size:.72rem}footer small{color:#667f90}.message{min-height:1.2em;margin:0;color:#66e1fb}@media(max-width:600px){.section-head{align-items:stretch;flex-direction:column}.primary{width:100%}}
</style>

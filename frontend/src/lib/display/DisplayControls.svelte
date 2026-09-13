<script lang="ts">
  import { requestJson } from '$lib/api/client';
  type Capabilities = { width: number; height: number; native_brightness: boolean; power_control: boolean };
  let { capabilities, powerOn, brightness, onpower, onbrightness }: { capabilities: Capabilities; powerOn: boolean; brightness: number; onpower?: (on: boolean) => void; onbrightness?: (value: number) => void } = $props();
  let busy = $state(false); let error = $state('');
  async function setPower(on: boolean) {
    busy = true; error = '';
    try { await requestJson<{ on: boolean }>('/api/display/power', 'POST', { on }); onpower?.(on); }
    catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
  async function setLuminance(value: number) {
    busy = true; error = '';
    try { const status = await requestJson<{ brightness: number }>('/api/display/luminance', 'POST', { value }); onbrightness?.(status.brightness); }
    catch (cause) { error = String(cause); }
    finally { busy = false; }
  }
</script>
<section class="display-controls" aria-label="Contrôles de l’écran">
  <h2>Écran</h2>
  <label for="software-luminance">Luminosité de l’image (logicielle)</label>
  <input id="software-luminance" type="range" min="0" max="100" value={brightness} disabled={busy} oninput={(event) => setLuminance(Number(event.currentTarget.value))} />
  {#if capabilities.power_control}<button disabled={busy} onclick={() => setPower(!powerOn)}>{powerOn ? 'Éteindre l’écran' : 'Allumer l’écran'}</button>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>
<style>.display-controls{display:grid;gap:10px}.display-controls button{width:max-content;background:#102630;color:#eaf8ff;border:1px solid #315363;border-radius:7px;padding:8px 12px}.display-controls p{color:#b9d0d9}</style>

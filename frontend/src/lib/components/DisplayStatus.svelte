<script lang="ts">
  import type { components } from '../api/schema';
  type StatusDto = components['schemas']['StatusDto'];

  let { status }: { status: StatusDto } = $props();
</script>

<section aria-label="Display status" class="status-card">
  <i class:online={status.device_present}></i>
  <div><strong>{status.device_present ? 'Connected' : 'Disconnected'}</strong><small><b>{status.brightness}%</b> software luminance · {status.version}</small></div>
</section>

<style>.status-card{display:flex;align-items:center;gap:10px;padding:9px 13px;border:1px solid #294558;border-radius:999px;background:#0a1823;box-shadow:inset 0 1px #ffffff0a}.status-card i{width:9px;height:9px;border-radius:50%;background:#ff6375;box-shadow:0 0 10px #ff637577}.status-card i.online{background:#58e5a4;box-shadow:0 0 10px #58e5a477}.status-card div{display:grid}.status-card strong{font-size:.76rem}.status-card small{color:#7f99a9;font-size:.64rem}.status-card b{font-weight:inherit}@media(max-width:520px){.status-card{border-radius:12px}}</style>

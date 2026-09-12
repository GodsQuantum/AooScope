<script lang="ts">
  import type { components } from '../api/schema';
  type StatusDto = components['schemas']['StatusDto'];

  let { status }: { status: StatusDto } = $props();
</script>

<section aria-label="Display status" class="status-card">
  <strong>{status.brightness}%</strong>
  <span>{status.device_present ? 'Connected' : 'Disconnected'}</span>
  <small>{status.version}</small>
</section>

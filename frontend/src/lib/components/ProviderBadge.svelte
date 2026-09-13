<script lang="ts">
  let { provider, category = '' }: { provider: string; category?: string } = $props();
  const label = $derived(`${provider}${category ? ` / ${category}` : ''}`);
</script>
<span class="provider" title={label}><span aria-hidden="true">◈</span>{label}</span>
<style>.provider{display:inline-flex;gap:4px;align-items:center;color:#9eddf1;font-size:.75rem}.provider span{color:#64d5f4}</style>

<script lang="ts">
  let { oncreate }: { oncreate: (templateId: string) => void } = $props();
  const templates = [
    { id: 'factory.vertical-bars.v1', name: 'Vertical bars', description: 'Compact columns for glanceable metrics', preview: 'vertical' },
    { id: 'factory.semi-rings.v1', name: 'Semi rings', description: 'Large values framed by open gauges', preview: 'rings' },
    { id: 'factory.horizontal-bars.v1', name: 'Horizontal bars', description: 'Dense rows for compute and queues', preview: 'horizontal' },
    { id: 'factory.media.v1', name: 'Media card', description: 'Poster, title, progress and ETA', preview: 'media' },
    { id: 'factory.storage.v1', name: 'Storage cards', description: 'Capacity, temperature and health', preview: 'storage' }
  ];
</script>

<section class="picker" aria-label="Template picker"><div><span class="eyebrow">Starting points</span><h3>Templates</h3><p>Every template creates an ordinary editable page.</p></div><div class="templates">{#each templates as item}<button aria-label={`${item.name} — ${item.description}`} onclick={() => oncreate(item.id)}><span class={`preview ${item.preview}`} aria-hidden="true"><i></i><i></i><i></i></span><strong>{item.name}</strong><small>{item.description}</small></button>{/each}</div></section>

<style>
  .picker{display:grid;gap:12px}.eyebrow{color:#35d9ff;font-size:.64rem;font-weight:800;letter-spacing:.14em;text-transform:uppercase}h3{margin:3px 0}p{margin:0;color:#8da5b5;font-size:.75rem}.templates{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(170px,100%),1fr));gap:9px}.templates button{display:grid;grid-template-rows:72px auto auto;gap:5px;min-width:0;padding:9px;text-align:left}.templates small{color:#8da5b5;font-size:.68rem}.preview{position:relative;display:flex;align-items:end;justify-content:center;gap:8px;overflow:hidden;border:1px solid #29485a;border-radius:8px;background:#061019;padding:10px}.preview i{display:block;background:#35d9ff}.vertical i{width:14px;height:35px}.vertical i:nth-child(2){height:50px}.vertical i:nth-child(3){height:28px}.rings i{width:35px;height:20px;border:5px solid #58e5a4;border-bottom:0;border-radius:35px 35px 0 0;background:none}.horizontal{align-items:center;flex-direction:column;gap:6px}.horizontal i{width:80%;height:7px;border-radius:5px}.horizontal i:nth-child(2){width:55%}.media{justify-content:flex-start}.media i:first-child{width:34px;height:50px;background:#9a6cff}.media i:nth-child(2){width:70px;height:8px}.media i:nth-child(3){position:absolute;left:55px;bottom:20px;width:50px;height:5px;background:#58e5a4}.storage{display:grid;grid-template-columns:1fr 1fr;align-items:center}.storage i{width:100%;height:23px;border:1px solid #365467;border-radius:4px;background:linear-gradient(#35d9ff 0 0) 6px 13px/65% 4px no-repeat}.storage i:last-child{display:none}
</style>

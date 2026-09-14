<script lang="ts">
  let {
    tabs,
    active,
    onselect
  }: {
    tabs: readonly string[];
    active: string;
    onselect: (tab: string) => void;
  } = $props();
</script>

<nav aria-label="Admin sections" class="tabs">
  {#each tabs as tab}
    <button class:active={tab === active} onclick={() => onselect(tab)}><span>{tab === 'Pages' ? '▦' : tab === 'Media' ? '▧' : tab === 'Display' ? '◐' : '⌁'}</span>{tab}</button>
  {/each}
</nav>

<style>
  .tabs{display:flex;gap:4px;margin-top:22px;border-bottom:1px solid #223a4c;overflow:auto;scrollbar-width:none}.tabs button{display:flex;align-items:center;gap:7px;padding:12px 16px;border:0;border-bottom:2px solid transparent;border-radius:8px 8px 0 0;background:transparent;color:#839baa;white-space:nowrap}.tabs button:hover{color:#d8eaf3;background:#122534}.tabs button.active{color:#f4fbff;border-color:#35d9ff;background:linear-gradient(#112a3a,#0c1c28)}.tabs span{color:#4edbf8}
</style>

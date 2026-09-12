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
    <button class:active={tab === active} onclick={() => onselect(tab)}>{tab}</button>
  {/each}
</nav>

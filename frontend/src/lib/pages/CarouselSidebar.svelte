<script lang="ts">
  export type PageSummary = { id: string; name: string; enabled: boolean; duration: number; revision: number; template_id?: string | null };
  let { pages, selected, onselect, onchange, onduplicate, onrestore, ondelete, onsave, oncreate }: {
    pages: PageSummary[]; selected?: string; onselect: (id: string) => void; onchange: (pages: PageSummary[]) => void;
    onduplicate: (id: string) => void; onrestore: (id: string) => void; ondelete: (id: string) => void; onsave: () => void; oncreate?: () => void;
  } = $props();
  let dragged = $state<string>();
  function move(index: number, offset: number) {
    const next = [...pages];
    const target = index + offset;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    onchange(next);
  }
  function update(id: string, changes: Partial<PageSummary>) { onchange(pages.map((page) => page.id === id ? { ...page, ...changes } : page)); }
  function drop(target: string) {
    if (!dragged || dragged === target) return;
    const next = pages.filter((item) => item.id !== dragged);
    next.splice(next.findIndex((item) => item.id === target), 0, pages.find((item) => item.id === dragged)!);
    onchange(next); dragged = undefined;
  }
</script>

<aside class="sidebar card" aria-label="Carousel">
  <div class="heading"><div><span class="eyebrow">LCD sequence</span><h2>Carousel</h2></div>{#if oncreate}<button class="icon primary" aria-label="Create page" onclick={oncreate}>+</button>{/if}</div>
  <p class="hint">Drag to reorder. Changes remain drafts until Apply.</p>
  <div class="page-list">
    {#each pages as item, index (item.id)}
      <article class:active={item.id === selected} class="page-card" draggable="true" ondragstart={() => dragged = item.id} ondragover={(event) => event.preventDefault()} ondrop={() => drop(item.id)}>
        <button class="select" onclick={() => onselect(item.id)}><span>{item.name}</span><small>Revision {item.revision}</small></button>
        <div class="meta">
          <label class="toggle"><input type="checkbox" checked={item.enabled} onchange={(e) => update(item.id, { enabled: e.currentTarget.checked })} /><span>Enabled</span></label>
          <label>Duration {item.name}<span class="duration"><input aria-label={`Duration ${item.name}`} type="number" min="2" max="120" value={item.duration} onchange={(e) => update(item.id, { duration: Number(e.currentTarget.value) })} /> s</span></label>
        </div>
        <div class="actions">
          <button aria-label={`Move ${item.name} up`} disabled={index === 0} onclick={() => move(index, -1)}>↑</button>
          <button aria-label={`Move ${item.name} down`} disabled={index === pages.length - 1} onclick={() => move(index, 1)}>↓</button>
          <button aria-label={`Duplicate ${item.name}`} onclick={() => onduplicate(item.id)}>Duplicate</button>
          {#if item.template_id}<button aria-label={`Restore ${item.name}`} onclick={() => onrestore(item.id)}>Restore</button>{/if}
          <button class="danger" aria-label={`Delete ${item.name}`} onclick={() => ondelete(item.id)}>Delete</button>
        </div>
      </article>
    {/each}
  </div>
  <button class="save" onclick={onsave}>Save carousel</button>
</aside>

<style>
  .sidebar{position:sticky;top:16px;align-self:start}.heading{display:flex;justify-content:space-between;align-items:center}.eyebrow{color:#50d9f7;font-size:.68rem;font-weight:800;letter-spacing:.14em;text-transform:uppercase}h2{margin:3px 0 0}.hint{color:#8fa8b8;font-size:.78rem;line-height:1.45}.icon{width:36px;height:36px;border-radius:10px;font-size:1.3rem}.page-list{display:grid;gap:9px;margin:14px 0}.page-card{background:#08141e;border:1px solid #223c4e;border-radius:13px;padding:10px;box-shadow:inset 0 1px #ffffff08}.page-card.active{border-color:#35d9ff;box-shadow:0 0 0 1px #35d9ff55,0 0 22px #35d9ff12}.select{display:flex;width:100%;justify-content:space-between;gap:8px;text-align:left;background:none;border:0;padding:0;color:#f1f8fc;font-weight:800}.select small{color:#7791a2;font-size:.65rem;font-weight:600}.meta{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-top:9px;color:#8fa8b8;font-size:.68rem}.toggle{display:flex;align-items:center;gap:6px}.toggle input{width:auto}.duration{display:flex;align-items:center;gap:4px}.duration input{width:48px;padding:4px}.actions{display:flex;flex-wrap:wrap;gap:5px;margin-top:9px}.actions button{padding:4px 6px;font-size:.65rem}.danger{color:#ff8f9b;border-color:#673441!important}.save{width:100%;padding:10px}.primary,.save{background:#087fa5!important;border-color:#35d9ff!important;color:white!important}@media(max-width:1100px){.sidebar{position:static}}
</style>

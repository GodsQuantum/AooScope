<script lang="ts">
  export type PageSummary = { id: string; name: string; enabled: boolean; duration: number; revision: number; template_id?: string | null };
  let { pages, selected, onselect, onchange, onduplicate, onrestore, ondelete, onsave, oncreate }: {
    pages: PageSummary[]; selected?: string; onselect: (id: string) => void; onchange: (pages: PageSummary[]) => void;
    onduplicate: (id: string) => void; onrestore: (id: string) => void; ondelete: (id: string) => void; onsave: () => void; oncreate?: () => void;
  } = $props();
  let dragged = $state<string>();
  function move(index: number, offset: number) {
    const target = index + offset;
    if (target < 0 || target >= pages.length) return;
    const next = [...pages];
    [next[index], next[target]] = [next[target], next[index]];
    onchange(next);
  }
  function update(id: string, changes: Partial<PageSummary>) { onchange(pages.map((item) => item.id === id ? { ...item, ...changes } : item)); }
  function drop(target: string) {
    if (!dragged || dragged === target) return;
    const next = pages.filter((item) => item.id !== dragged);
    next.splice(next.findIndex((item) => item.id === target), 0, pages.find((item) => item.id === dragged)!);
    onchange(next); dragged = undefined;
  }
  function preview(item: PageSummary) {
    const identity = `${item.template_id ?? ''} ${item.name}`.toLowerCase();
    if (identity.includes('semi-ring')) return 'rings';
    if (identity.includes('vertical')) return 'vertical';
    if (identity.includes('horizontal')) return 'horizontal';
    if (identity.includes('storage') || identity.includes('disk')) return 'storage';
    if (identity.includes('media') || identity.includes('splash')) return 'media';
    if (identity.includes('home') || identity.includes('compute') || identity.includes('cpu')) return 'dashboard';
    return 'custom';
  }
  const initials = (name: string) => name.split(/\s+/).map((word) => word[0]).join('').slice(0, 2).toUpperCase();
</script>

<section class="strip card" aria-label="Carousel" data-testid="page-strip">
  <div class="strip-head"><div><span class="eyebrow">LCD sequence</span><strong>Pages</strong></div><div class="strip-actions">{#if oncreate}<button aria-label="Create page" onclick={oncreate}>+ Blank</button>{/if}<button class="save" onclick={onsave}>Save carousel</button></div></div>
  <div class="pages">
    {#each pages as item, index (item.id)}
      <article data-testid={`page-tile-${item.id}`} class:active={item.id === selected} draggable="true" ondragstart={() => dragged = item.id} ondragover={(event) => event.preventDefault()} ondrop={() => drop(item.id)}>
        <button class="select" aria-label={`${item.name} Revision ${item.revision}`} onclick={() => onselect(item.id)}><span class={`thumbnail ${preview(item)}`} data-preview={preview(item)} data-testid={`page-thumbnail-${item.id}`} aria-label={`${item.name} preview`}><i></i><i></i><i></i>{#if preview(item) === 'custom'}<b>{initials(item.name)}</b>{/if}</span><span class="identity"><strong>{item.name}</strong><small>Revision {item.revision}</small><span class:disabled={!item.enabled} class="status" aria-label={`${item.name} ${item.enabled ? 'enabled' : 'disabled'}`}>{item.enabled ? 'On' : 'Off'} · {item.duration}s</span></span></button>
        <details><summary aria-label={`Page actions ${item.name}`}>Actions</summary><div class="meta">
          <label><input type="checkbox" checked={item.enabled} onchange={(event) => update(item.id, { enabled: event.currentTarget.checked })} /> Enabled</label>
          <label>Duration {item.name}<span><input aria-label={`Duration ${item.name}`} type="number" min="2" max="120" value={item.duration} onchange={(event) => update(item.id, { duration: Number(event.currentTarget.value) })} /> s</span></label>
        </div><div class="menu">
          <button aria-label={`Move ${item.name} up`} disabled={index === 0} onclick={() => move(index, -1)}>↑ Up</button>
          <button aria-label={`Move ${item.name} down`} disabled={index === pages.length - 1} onclick={() => move(index, 1)}>↓ Down</button>
          <button aria-label={`Duplicate ${item.name}`} onclick={() => onduplicate(item.id)}>Duplicate</button>
          {#if item.template_id}<button aria-label={`Restore ${item.name}`} onclick={() => onrestore(item.id)}>Restore</button>{/if}
          <button class="danger" aria-label={`Delete ${item.name}`} onclick={() => ondelete(item.id)}>Delete</button>
        </div></details>
      </article>
    {/each}
  </div>
</section>

<style>
  .strip{display:grid;gap:8px}.strip-head,.strip-head>div,.strip-actions,.select,.meta label,.meta label span{display:flex;align-items:center}.strip-head{justify-content:space-between;gap:8px}.strip-head>div:first-child{gap:8px}.eyebrow{color:#35d9ff;font-size:.62rem;font-weight:800;letter-spacing:.13em;text-transform:uppercase}.strip-actions{gap:5px}.strip-actions button{padding:5px 7px;font-size:.7rem}.save{background:#087fa5;border-color:#35d9ff;color:white}.pages{display:grid;grid-template-columns:repeat(auto-fit,minmax(145px,1fr));gap:6px}article{min-width:0;padding:6px;border:1px solid #223c4e;border-radius:10px;background:#08141e;box-shadow:inset 0 1px #ffffff08}article.active{border-color:#35d9ff;box-shadow:0 0 0 1px #35d9ff44}.select{width:100%;gap:7px;padding:0;border:0;background:none;text-align:left}.thumbnail{position:relative;display:flex;align-items:end;justify-content:center;gap:3px;width:51px;height:22px;flex:none;padding:4px;border:1px solid #294b5d;border-radius:5px;background:#030a10}.thumbnail i{display:block;background:#35d9ff}.vertical i{width:6px;height:9px}.vertical i:nth-child(2){height:13px}.vertical i:nth-child(3){height:6px}.rings i{width:12px;height:7px;border:2px solid #58e5a4;border-bottom:0;border-radius:12px 12px 0 0;background:none}.horizontal{align-items:center;flex-direction:column;gap:2px}.horizontal i{width:35px;height:2px;border-radius:2px}.horizontal i:nth-child(2){width:24px}.media{justify-content:flex-start}.media i:first-child{width:9px;height:13px;background:#9a6cff}.media i:nth-child(2){align-self:center;width:22px;height:3px}.media i:nth-child(3){position:absolute;left:17px;bottom:4px;width:16px;height:2px;background:#58e5a4}.storage{display:grid;grid-template-columns:1fr 1fr;gap:2px}.storage i,.dashboard i{width:18px;height:10px;border:1px solid #365467;border-radius:2px;background:linear-gradient(#35d9ff 0 0) 3px 6px/65% 2px no-repeat}.storage i:last-child{display:none}.dashboard i{width:10px}.dashboard i:nth-child(2){height:14px;background:#58e5a4}.custom i{display:none}.custom b{color:#35d9ff;font-size:.62rem;letter-spacing:.08em}.identity{display:grid;min-width:0;line-height:1.15}.identity strong,.identity small,.status{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.identity strong{font-size:.75rem}.identity small{color:#7791a2;font-size:.58rem}.status{color:#58e5a4;font-size:.58rem}.status.disabled{color:#8094a1}.meta{display:grid;grid-template-columns:1fr 1.25fr;gap:7px;margin-top:6px}.meta label{justify-content:space-between;gap:5px;color:#91a8b8;font-size:.67rem}.meta input[type=checkbox]{width:auto}.meta input[type=number]{width:44px;padding:3px 4px}.meta label span{gap:3px}details{margin-top:3px;color:#8fa8b8;font-size:.64rem}summary{width:max-content;cursor:pointer}.menu{display:flex;flex-wrap:wrap;gap:4px;margin-top:5px}.menu button{padding:4px 6px;font-size:.62rem}.danger{color:#ff8f9b;border-color:#673441!important}@media(max-width:480px){.strip{padding:9px}.strip-head strong{display:none}.pages{grid-template-columns:repeat(2,minmax(0,1fr))}.strip-actions{margin-left:auto}}
</style>

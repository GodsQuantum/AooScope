<script lang="ts">
  let { title, detail, status, poster, progress }: { title: string; detail: string; status?: string; poster?: string; progress?: number } = $props();
  let posterFailed = $state(false);
  $effect(() => { void poster; posterFailed = false; });
</script>
<article class="card">
  {#if poster && !posterFailed}<img src={poster} alt={`Poster for ${title}`} onerror={() => posterFailed = true} />{:else}<div class="poster" aria-label="Poster unavailable">Poster unavailable</div>{/if}
  <div class="body">{#if status}<strong class="status" role="status">{status}</strong>{/if}<h3>{title}</h3><p>{detail}</p>{#if progress !== undefined}<progress max="100" value={progress}>{progress}%</progress>{/if}</div>
</article>
<style>.card{display:grid;grid-template-columns:minmax(120px,220px) 1fr;gap:20px;padding:14px;border:1px solid #244858;border-radius:12px;background:#0b1720}.card img,.poster{width:100%;aspect-ratio:2/3;max-height:280px;object-fit:cover;border-radius:8px;background:#16394b;display:grid;place-items:center;font-size:2rem}.body{align-self:center;min-width:0}.status{display:block;margin-bottom:12px;color:#62e3a3;font-size:clamp(1.5rem,4vw,3.2rem);line-height:1}.card h3{font-size:clamp(1.2rem,2.5vw,2rem)}.card h3,.card p{margin:0 0 10px}.card p{color:#9db5c5;font-size:.9rem}progress{width:100%;accent-color:#5dd9ff}@media(max-width:520px){.card{grid-template-columns:100px 1fr;gap:12px}.card img,.poster{max-height:150px}.status{font-size:1.35rem}}</style>

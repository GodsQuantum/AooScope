<script lang="ts">
  let { title, detail, poster, progress }: { title: string; detail: string; poster?: string; progress?: number } = $props();
</script>
<article class="card">
  {#if poster}<img src={poster} alt="" />{:else}<div class="poster" aria-hidden="true">✦</div>{/if}
  <div><h3>{title}</h3><p>{detail}</p>{#if progress !== undefined}<progress max="100" value={progress}>{progress}%</progress>{/if}</div>
</article>
<style>.card{display:grid;grid-template-columns:120px 1fr;gap:14px;padding:12px;border:1px solid #244858;border-radius:10px;background:#0b1720}.card img,.poster{width:120px;height:68px;object-fit:cover;border-radius:6px;background:#16394b;display:grid;place-items:center;font-size:2rem}.card h3,.card p{margin:0 0 6px}.card p{color:#9db5c5;font-size:.9rem}progress{width:100%;accent-color:#5dd9ff}</style>

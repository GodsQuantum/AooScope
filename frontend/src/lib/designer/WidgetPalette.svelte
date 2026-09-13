<script lang="ts">
  import type { WidgetType } from './model';
  let { onadd }: { onadd: (type: WidgetType, extra?: Record<string, unknown>) => void } = $props();
  const widgets: [WidgetType, string, string, Record<string, unknown>?][] = [['text','Texte','Aa'],['value','Valeur','42'],['gauge','Jauge demi-cercle','◔'],['ring','Anneau','◉'],['bar','Barre horizontale','▰',{ orientation: 'horizontal' }],['bar','Barre verticale','▥',{ orientation: 'vertical' }],['badge','Badge/état','OK'],['sparkline','Courbe','⌁'],['image','Image','▧'],['animation','Animation','✦']];
</script>
<section><h3>Widgets</h3><div class="palette">{#each widgets as [type, label, preview, extra]}<button type="button" aria-label={`Ajouter ${label}`} onclick={() => onadd(type, extra)}><b>{preview}</b><span>{label}</span></button>{/each}</div></section>
<style>section{min-width:0}.palette{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:7px}button{min-height:62px;display:grid;place-items:center;padding:7px;background:#102630;color:#eaf8ff;border:1px solid #244858;border-radius:8px}button:hover{border-color:#5dd9ff}b{font-size:1.35rem;color:#63d9f5}span{font-size:.7rem;text-align:center}</style>

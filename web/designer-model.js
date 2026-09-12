export const CANVAS_WIDTH=960;
export const CANVAS_HEIGHT=376;
const defaults={text:[220,60],value:[220,90],bar:[260,36],gauge:[220,220],ring:[150,150],badge:[180,48],image:[260,160],sparkline:[280,90],animation:[260,160]};

export function clamp(v,min,max){return Math.max(min,Math.min(max,v))}
export function normalizePage(page={}){
  return {...page,background:{color:'#071019',...(page.background||{})},layers:[...(page.layers||[])]};
}
export function makeLayer(type,binding=null,x=40,y=40){
  const [width,height]=defaults[type]||[200,80];
  return {id:cryptoId(),type,binding,x,y,width,height,rotation:0,opacity:1,z:10,clip:false,color:'#ffffff',unit:'',min:0,max:100,fallback:'--'};
}
function cryptoId(){return `layer-${Math.random().toString(36).slice(2,10)}`}
export function clampGeometry(layer){
  const width=clamp(Math.round(+layer.width||1),1,CANVAS_WIDTH);
  const height=clamp(Math.round(+layer.height||1),1,CANVAS_HEIGHT);
  const x=clamp(Math.round(+layer.x||0),0,CANVAS_WIDTH-width);
  const y=clamp(Math.round(+layer.y||0),0,CANVAS_HEIGHT-height);
  return {...layer,x,y,width,height};
}
export function snap(value,grid=1){grid=Math.max(1,+grid||1);return Math.round(value/grid)*grid}

export function moveLayer(layer,x,y,grid=1){return clampGeometry({...layer,x:snap(x,grid),y:snap(y,grid)})}
export function resizeLayer(layer,width,height,grid=1){const x=clamp(Math.round(+layer.x||0),0,CANVAS_WIDTH-1),y=clamp(Math.round(+layer.y||0),0,CANVAS_HEIGHT-1);const w=clamp(snap(width,grid),1,CANVAS_WIDTH-x),h=clamp(snap(height,grid),1,CANVAS_HEIGHT-y);return {...layer,x,y,width:w,height:h}}
export function nudge(layer,dx,dy,grid=1){return moveLayer(layer,layer.x+dx*grid,layer.y+dy*grid,1)}
export function setLayerType(layer,type){
  const [width,height]=defaults[type]||[layer.width,layer.height];
  return clampGeometry({...layer,type,width:layer.width||width,height:layer.height||height});
}
export function setLayerZ(layer,z){return {...layer,z:clamp(Math.round(+z||0),-10000,10000)}}
export function clientToLogical(ev,rect,scale){
  scale=Math.max(.01,+scale||1);
  return {x:Math.round((ev.clientX-rect.left)/scale),y:Math.round((ev.clientY-rect.top)/scale)};
}
export function replaceLayer(page,updated){
  return {...page,layers:(page.layers||[]).map(layer=>layer.id===updated.id?updated:layer)};
}
export function removeLayer(page,id){return {...page,layers:(page.layers||[]).filter(layer=>layer.id!==id)}}

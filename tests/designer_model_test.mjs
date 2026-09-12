import assert from 'node:assert/strict';
import {normalizePage,makeLayer,clampGeometry,clientToLogical,nudge,setLayerType,moveLayer,resizeLayer,setLayerZ} from '../web/designer-model.js';

const p=normalizePage({id:'p',name:'x',layers:[]});
assert.equal(p.background.color,'#071019');
assert.deepEqual(p.layers,[]);

const layer=makeLayer('value','aooscope_pve_cpu_pct',120,80);
assert.equal(layer.x,120); assert.equal(layer.y,80); assert.equal(layer.type,'value');
assert.equal(layer.binding,'aooscope_pve_cpu_pct');

const clamped=clampGeometry({...layer,x:950,width:100,y:-10,height:60});
assert.equal(clamped.x,860); assert.equal(clamped.y,0);

const logical=clientToLogical({clientX:250,clientY:150},{left:50,top:50},0.5);
assert.deepEqual(logical,{x:400,y:200});

const moved=moveLayer(layer,333,191,10);
assert.equal(moved.x,330); assert.equal(moved.y,190);
const resized=resizeLayer({...layer,x:900,y:340},200,100,1);
assert.equal(resized.width,60); assert.equal(resized.height,36);
const nudged=nudge(layer,1,-1,1);
assert.equal(nudged.x,121); assert.equal(nudged.y,79);
const changed=setLayerType(layer,'gauge');
assert.equal(changed.type,'gauge'); assert.equal(changed.binding,layer.binding);
const zed=setLayerZ(layer,99);
assert.equal(zed.z,99);

console.log('designer model: OK');

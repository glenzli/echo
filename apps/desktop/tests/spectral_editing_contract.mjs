import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const context=vm.createContext({});
vm.runInContext(fs.readFileSync(new URL('../qml/SpectralEditing.js',import.meta.url),'utf8'),context);
const value={startMillis:1000,endMillis:2000,lowHertz:950,highHertz:1050,attenuationCentibels:2400,timeFeatherMillis:24,frequencyFeatherHertz:20};
test('frequency axes round-trip at low, mid and high frequencies',()=>{
 for(const log of [true,false]) for(const h of [20,50,1000,12000,24000])
  assert.ok(Math.abs(context.hertzAt(context.frequencyRatio(h,20,24000,log),20,24000,log)-h)<1e-7);
});
test('bounded movement preserves time duration and bandwidth',()=>{
 const moved=context.move(value,100000,-10000,8000);
 assert.equal(moved.startMillis,7000); assert.equal(moved.endMillis,8000);
 assert.equal(moved.lowHertz,20); assert.equal(moved.highHertz,120);
});
test('edge resize cannot invert a selection',()=>{
 const resized=context.resize(value,'lefttop',3000,900,8000);
 assert.equal(resized.startMillis,1999); assert.equal(resized.highHertz,951);
});
test('harmonic copies retain time and width and stop at Nyquist',()=>{
 const copies=context.harmonics(value,4,8000);
 assert.equal(copies.length,3); assert.equal(copies[0].lowHertz,1950);
 assert.ok(copies.every(x=>x.startMillis===1000&&x.endMillis===2000&&x.highHertz-x.lowHertz===100));
 assert.equal(context.harmonics({...value,lowHertz:11500,highHertz:12500},4,8000).length,0);
});
test('short narrow selections have usable feather defaults, invalid numbers rejected',()=>{
 const s=context.selection(100,110,1000,1020,8000);
 assert.equal(s.timeFeatherMillis,2);assert.equal(s.frequencyFeatherHertz,5);
 assert.equal(context.region({...value,attenuationCentibels:NaN},8000),null);
});

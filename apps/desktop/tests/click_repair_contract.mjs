import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const ranges=vm.createContext({});
vm.runInContext(fs.readFileSync(new URL('../qml/SourceEditRanges.js',import.meta.url),'utf8').replace('.pragma library',''),ranges);
const edit=vm.createContext({Ranges:ranges});
vm.runInContext(fs.readFileSync(new URL('../qml/ClickRepairEditing.js',import.meta.url),'utf8').replace('.pragma library','').replace('.import "SourceEditRanges.js" as Ranges',''),edit);
const base=()=>({trimStartMillis:100,trimEndMillis:10000,effectMasks:[],effectChain:[0,1,2,3,4],deClickEnabled:false,deClickRepairPercent:75,deClickSensitivityPercent:85,gainCentibels:-200});
const findings=[{startMillis:1000,endMillis:1001},{startMillis:4000,endMillis:4002}];
const plain=v=>JSON.parse(JSON.stringify(v));
test('local repair preserves authored settings and inserts once with idempotent scope',()=>{
 const source=base(),result=edit.propose(source,findings);assert.equal(result.ok,true);assert.equal(result.changed,true);
 assert.deepEqual(plain(result.snapshot.effectChain),[6,0,1,2,3,4]);assert.equal(result.snapshot.deClickRepairPercent,75);assert.equal(result.snapshot.gainCentibels,-200);
 assert.deepEqual(plain(result.snapshot.effectMasks),[{startMillis:990,endMillis:1011,featherMillis:5,effectNodes:[6]},{startMillis:3990,endMillis:4012,featherMillis:5,effectNodes:[6]}]);
 assert.deepEqual(source,base());assert.equal(edit.propose(result.snapshot,findings).changed,false);
});
test('global and bypassed local inserts are never narrowed or awakened',()=>{
 const source=base();source.effectChain.unshift(6);source.deClickEnabled=true;
 assert.equal(edit.propose(source,findings).error,'global');source.deClickEnabled=false;source.effectMasks=[{startMillis:200,endMillis:400,featherMillis:5,effectNodes:[6]}];
 assert.equal(edit.propose(source,findings).error,'bypassed');source.deClickEnabled=true;
 assert.equal(edit.propose(source,findings).snapshot.effectMasks.length,3);
});
test('union, trim edges, mask budget, invalid input and zero repair fail atomically',()=>{
 const source=base();const r=edit.propose(source,[{startMillis:101,endMillis:102},{startMillis:104,endMillis:105}]);assert.equal(r.snapshot.effectMasks.length,1);assert.equal(r.snapshot.effectMasks[0].startMillis,100);assert.equal(r.snapshot.effectMasks[0].featherMillis,0);
 assert.equal(edit.propose(source,[]).error,'range');assert.equal(edit.propose(source,[{startMillis:NaN,endMillis:10}]).error,'range');
 source.deClickRepairPercent=0;assert.equal(edit.propose(source,findings).error,'amount');source.deClickRepairPercent=75;
 source.effectMasks=Array.from({length:63},()=>({startMillis:200,endMillis:300,featherMillis:5,effectNodes:[1]}));assert.equal(edit.propose(source,findings).error,'limit');assert.equal(source.effectMasks.length,63);
});

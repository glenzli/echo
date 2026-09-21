import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const edit=vm.createContext({});
vm.runInContext(fs.readFileSync(new URL('../qml/SourceEditRanges.js',import.meta.url),'utf8').replace('.pragma library',''),edit);
const plain=v=>JSON.parse(JSON.stringify(v));
const segment=(a,b,options={})=>({sourceStartMillis:a,sourceEndMillis:b,state:0,gainCentibels:-300,fadeInMillis:0,fadeOutMillis:0,fadeInCurve:1,fadeOutCurve:2,gapAfterMillis:0,...options});
test('batch ranges merge overlaps and retain source gain, external fades and inserted gaps',()=>{
    const original=[segment(1000,5000,{fadeInMillis:400,fadeOutMillis:400,gapAfterMillis:120})];
    const result=edit.apply(original,[{start:1100,end:1300},{start:1250,end:1400},{start:3300,end:3700}],1000,5000,'hide');
    assert.equal(result.ok,true);assert.equal(result.changed,true);
    assert.deepEqual(plain(result.segments).map(s=>[s.sourceStartMillis,s.sourceEndMillis,s.state]),[[1000,1100,0],[1100,1400,2],[1400,3300,0],[3300,3700,2],[3700,5000,0]]);
    assert.equal(result.segments[0].fadeInMillis,100);assert.equal(result.segments.at(-1).fadeOutMillis,400);
    assert.deepEqual(plain(result.segments).map(s=>s.gapAfterMillis),[0,0,0,0,120]);
    assert.ok(result.segments.every(s=>s.gainCentibels===-300));assert.equal(original.length,1);assert.equal(original[0].state,0);
});
test('keep selected hides the complement without resurrecting hidden or muted content',()=>{
    const original=[segment(0,1000,{state:1}),segment(1000,2000,{state:2}),segment(2000,4000)];
    const result=edit.apply(original,[{start:500,end:2500},{start:3000,end:3500}],0,4000,'keep');
    assert.equal(result.ok,true);
    assert.deepEqual(plain(result.segments).map(s=>[s.sourceStartMillis,s.sourceEndMillis,s.state]),[[0,500,2],[500,1000,1],[1000,2000,2],[2000,2500,0],[2500,3000,2],[3000,3500,0],[3500,4000,2]]);
});
test('invalid, empty and over-budget batches reject atomically; empty output with an authored gap remains valid',()=>{
    const original=[segment(0,10000)];
    for(const ranges of [[],[{start:NaN,end:4}],[{start:4,end:3}],[{start:10000,end:20000}]]) assert.equal(edit.apply(original,ranges,0,10000,'hide').error,'invalid');
    assert.equal(edit.apply(original,[{start:0,end:10000}],0,10000,'hide').error,'empty');
    const many=Array.from({length:64},(_,i)=>({start:10+i*100,end:20+i*100}));
    assert.equal(edit.apply(original,many,0,10000,'hide').error,'limit');
    assert.equal(edit.apply(original,many.slice(0,63),0,10000,'hide').ok,true);
    assert.equal(edit.apply([segment(0,10000,{gapAfterMillis:100})],[{start:0,end:10000}],0,10000,'hide').ok,true);
    assert.equal(original.length,1);assert.equal(original[0].state,0);
});
test('repeated hide and keep operations are no-ops and do not fragment an existing hidden span',()=>{
    const original=[segment(0,1000),segment(1000,9000,{state:2}),segment(9000,10000)];
    const result=edit.apply(original,[{start:1500,end:2000},{start:4000,end:6000}],0,10000,'hide');
    assert.equal(result.changed,false);assert.deepEqual(plain(result.segments),original);
    assert.equal(edit.apply(original,[{start:0,end:10000}],0,10000,'keep').changed,false);
});
test('large transcript selections stay bounded and each output frame follows the union policy',()=>{
    const original=[segment(0,10000)];
    const ranges=Array.from({length:20000},(_,i)=>({start:2000+i%5000,end:2001+i%5000}));
    const start=performance.now(); const result=edit.apply(original,ranges,0,10000,'hide');
    assert.ok(performance.now()-start<1000);assert.equal(result.segments.length,3);
    assert.deepEqual(plain(result.segments).map(s=>s.state),[0,2,0]);
    let seed=73;
    for(let round=0;round<100;++round){
        const next=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed%1000;};
        const spans=Array.from({length:12},()=>{const a=next(),b=next();return {start:Math.min(a,b),end:Math.max(a,b)+1};});
        for(const kind of ['hide','keep']){
            const result=edit.apply([segment(0,1000)],spans,0,1000,kind);
            if(!result.ok){assert.equal(result.error,'empty');continue;}
            for(let t=0;t<1000;t+=7){const inside=spans.some(r=>t>=r.start&&t<r.end),s=result.segments.find(s=>t>=s.sourceStartMillis&&t<s.sourceEndMillis);assert.equal(s.state,(kind==='hide'?inside:!inside)?2:0);}
        }
    }
});

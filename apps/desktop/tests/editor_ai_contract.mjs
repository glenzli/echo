import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const transcript = vm.createContext({}), search=vm.createContext({});
for (const [file, context] of [['SoundTranscriptEditing', transcript],['SoundSourceSearch',search]]) vm.runInContext(fs.readFileSync(new URL(`../qml/${file}.js`,import.meta.url),'utf8').replace('.pragma library',''),context);
const plain=v=>JSON.parse(JSON.stringify(v));
test('sentence selection retains source coordinates and clamps breathing margin to the current clip',()=>{
    assert.deepEqual(plain(transcript.bounds({start:21.2,end:23.5},80,21000,23000)),{start:21120,end:23000});
    assert.equal(transcript.bounds({start:1,end:2},80,3000,4000),null);
    assert.equal(transcript.bounds({start:NaN,end:2},80,0,4000),null);
    assert.equal(transcript.bounds({start:2,end:1},80,0,4000),null);
});
test('material search preserves literal priority, collection eligibility and late-result isolation',()=>{
    const eligible=[{id:'a',path:'Rain.wav'},{id:'b',path:'Forest.wav'},{id:'c',sourceTitle:'rain again'}];
    const hits=[{id:'b',score:0.9},{id:'foreign',score:1},{id:'a',score:0.5},{id:'b',score:0.4}];
    assert.deepEqual(plain(search.ranked(eligible,'rain',hits,'rain')).map(a=>a.id),['a','c','b']);
    assert.equal(search.ranked(eligible,'rain',hits,'rain')[2].semanticCandidate,true);
    assert.deepEqual(plain(search.ranked(eligible,'rain',hits,'old query')).map(a=>a.id),['a','c']);
    assert.equal(eligible[1].semanticCandidate,undefined);
});

test('aligned units retain repeated text and do not invent duration for a zero-length word',()=>{
    const units=plain(transcript.units([{words:[{text:'go',start:1,end:2},{text:'go',start:3,end:4},{text:'to',start:4,end:4}]}]));
    assert.deepEqual(units.map(u=>u.start),[1,3,4]);
    assert.deepEqual(plain(transcript.bounds(units[1],80,0,5000)),{start:2920,end:4080});
    assert.equal(transcript.bounds(units[2],80,0,5000),null);
});

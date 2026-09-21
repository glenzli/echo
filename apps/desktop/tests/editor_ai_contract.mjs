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

test('phrase search spans complete aligned units in Chinese and English, retaining repeated matches',()=>{
    const entries=transcript.entries([{text:'Soft'},{text:'rain'},{text:'then'},{text:'soft'},{text:'rain'}]);
    assert.deepEqual(plain(transcript.filtered(entries,'soft rain',true)).map(s=>s.key),['text:0','text:1','text:3','text:4']);
    assert.deepEqual(plain(transcript.filtered(transcript.entries([{text:'慢'},{text:'慢'},{text:'走'}]),'慢慢',true)).map(s=>s.key),['text:0','text:1']);
    assert.equal(transcript.filtered(entries,'[',true).length,0); // literal, not regex
    assert.equal(transcript.filtered(transcript.entries([{text:'Soft rain falls.'}]),'RAIN',false).length,1);
});
test('speech gaps union overlapping evidence and retain source offset and edge padding',()=>{
    const words=[{text:'one',start:20,end:21},{text:'nested',start:20.5,end:20.8},{text:'two',start:22,end:23},{text:'three',start:23.2,end:24}];
    const gaps=plain(transcript.gaps(words,600,120));
    assert.equal(gaps.length,1);assert.equal(gaps[0].start,21.12);assert.equal(gaps[0].end,21.88);
    assert.deepEqual(plain(transcript.bounds(gaps[0],0,21500,23000)),{start:21500,end:21880});
    assert.equal(transcript.gaps(words,600,500).length,0);
});
test('uncertain timing blocks gap suggestions; no fabricated leading/trailing silence',()=>{
    for(const words of [[],[{text:'a',start:1,end:2}],[{text:'a',start:1,end:2},{text:'?',start:2.5,end:2.5},{text:'b',start:3,end:4}],[{text:'a',start:3,end:4},{text:'b',start:1,end:2}],[{text:'a',start:1,end:NaN}]]) assert.equal(transcript.gaps(words,200,0).length,0);
});

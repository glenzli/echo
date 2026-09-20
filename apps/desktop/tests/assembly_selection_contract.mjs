import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const editing = vm.createContext({});
vm.runInContext(fs.readFileSync(new URL('../qml/SoundAssemblyEditing.js', import.meta.url), 'utf8').replace('.pragma library', ''), editing);
const selection = vm.createContext({Editing: editing});
vm.runInContext(fs.readFileSync(new URL('../qml/SoundAssemblySelection.js', import.meta.url), 'utf8').replace('.pragma library', '').replace('.import "SoundAssemblyEditing.js" as Editing', ''), selection);
const plain = value => JSON.parse(JSON.stringify(value));
let sequence = 0;
const newId = () => `copy-${++sequence}`;
const clip = (id, start, duration, extra={}) => ({id, assetId:'original',adjustmentRevisionId:7,sourceRole:'memory',sourceStartMillis:100,sourceEndMillis:100+duration,timelineStartMillis:start,fadeInMillis:0,fadeOutMillis:0,gainEnvelope:{enabled:true,points:[{sourceMillis:100,gainCentibels:-600},{sourceMillis:8100,gainCentibels:0}]},...extra});
const track = (...clips) => ({clips});
test('group movement preserves offsets, source revisions and envelope coordinates at both boundaries', () => {
    const tracks=[track(clip('a',1000,3000)),track(clip('b',2500,2000)),track(clip('untouched',0,500))];
    const original=JSON.stringify(tracks);
    const left=plain(selection.move(tracks,['a','b'],-9000));
    assert.equal(left[0].clips[0].timelineStartMillis,0);
    assert.equal(left[1].clips[0].timelineStartMillis,1500);
    const right=plain(selection.move(tracks,['a','b'],20000000));
    assert.equal(editing.end(right[1].clips[0]),14400000);
    assert.equal(right[1].clips[0].timelineStartMillis-right[0].clips[0].timelineStartMillis,1500);
    assert.deepEqual(right[0].clips[0].gainEnvelope,tracks[0].clips[0].gainEnvelope);
    assert.equal(right[0].clips[0].adjustmentRevisionId,7);
    assert.deepEqual(right[2],tracks[2]);assert.equal(JSON.stringify(tracks),original);
});
test('duplicate preserves relative placement across tracks and selects only the new identities', () => {
    const source=[track(clip('a',0,1000)),track(clip('b',3000,1000))];
    const result=plain(selection.duplicate(source,['a','b'],newId));
    assert.equal(result.tracks[0].clips[1].timelineStartMillis,4000);
    assert.equal(result.tracks[1].clips[1].timelineStartMillis,7000);
    assert.equal(result.ids.length,2);assert.notEqual(result.ids[0],'a');
    assert.equal(result.tracks[0].clips[1].adjustmentRevisionId,7);
    assert.equal(selection.duplicate([track(clip('a',14398000,2000))],['a'],newId).error,'duration');
});
test('batch split keeps source time and all-or-nothing capacity admission', () => {
    const tracks=[track(clip('a',0,4000),clip('later',6000,1000)),track(clip('b',1000,5000))];
    const result=plain(selection.split(tracks,['a','b','later'],2000,newId));
    assert.equal(result.tracks[0].clips.length,3);assert.equal(result.tracks[1].clips.length,2);
    for(const t of result.tracks) assert.deepEqual(t.clips[0].gainEnvelope,tracks[0].clips[0].gainEnvelope);
    assert.equal(result.tracks[1].clips[0].sourceEndMillis,1100);
    assert.equal(result.tracks[1].clips[1].sourceStartMillis,1100);
    const full=[track(...Array.from({length:256},(_,i)=>clip(String(i),0,4000)))];
    assert.equal(selection.split(full,['0'],2000,newId).error,'capacity');
    assert.equal(selection.duplicate(full,['0'],newId).error,'capacity');
});
test('ripple all tracks cuts a continuous background, closes only the union, and keeps source envelopes', () => {
    const tracks=[track(clip('one',1000,2000),clip('overlap',2000,2000),clip('two',6000,1000)),track(clip('bed',0,10000))];
    const original=JSON.stringify(tracks);
    const result=plain(selection.ripple(tracks,['one','overlap','two'],true,newId));
    assert.deepEqual(plain(selection.ranges(tracks,['one','overlap','two'])),[{start:1000,end:4000},{start:6000,end:7000}]);
    assert.equal(result.tracks[0].clips.length,0);
    assert.deepEqual(result.tracks[1].clips.map(c=>[c.timelineStartMillis,c.sourceStartMillis,c.sourceEndMillis]),[[0,100,1100],[1000,4100,6100],[3000,7100,10100]]);
    assert.equal(editing.end(result.tracks[1].clips.at(-1)),6000);
    for(const c of result.tracks[1].clips) assert.deepEqual(c.gainEnvelope,tracks[1].clips[0].gainEnvelope);
    assert.equal(new Set(result.tracks[1].clips.map(c=>c.id)).size,3);assert.equal(JSON.stringify(tracks),original);
});
test('ripple selected tracks leaves every other track byte-equivalent', () => {
    const tracks=[track(clip('a',1000,2000),clip('later',5000,1000)),track(clip('bed',0,8000))];
    const result=plain(selection.ripple(tracks,['a'],false,newId));
    assert.equal(result.tracks[0].clips[0].timelineStartMillis,3000);assert.deepEqual(result.tracks[1],tracks[1]);
});
test('ripple and delete cannot empty the project; cross-track movement rejects partial moves', () => {
    const tracks=[track(clip('a',1000,2000)),track(clip('b',1000,2000))];
    assert.equal(selection.remove(tracks,['a','b']).error,'empty');
    assert.equal(selection.ripple(tracks,['a'],true,newId).error,'empty');
    assert.equal(selection.moveTracks(tracks,['a','b'],-1).error,'track');
    const result=plain(selection.moveTracks(tracks,['b'],-1));
    assert.equal(result.tracks[0].clips.length,2);assert.equal(result.tracks[1].clips.length,0);
});
test('slip is source-bounded and preserves placement, duration and source-time automation', () => {
    const original=clip('a',2000,3000);
    const next=editing.slip(original,100000,8000);
    assert.equal(next.sourceEndMillis,8000);assert.equal(next.sourceStartMillis,5000);
    assert.equal(next.timelineStartMillis,2000);assert.deepEqual(next.gainEnvelope,original.gainEnvelope);
    assert.equal(editing.slip(next,-100000,8000).sourceStartMillis,0);
});
test('ripple at capacity rejects extra split pieces without touching the input', () => {
    const tracks=[track(clip('cut',1000,1000)),track(...Array.from({length:255},(_,i)=>clip(`bed-${i}`,0,4000)))];
    const before=JSON.stringify(tracks);assert.equal(selection.ripple(tracks,['cut'],true,newId).error,'capacity');assert.equal(JSON.stringify(tracks),before);
});

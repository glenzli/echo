// Exercise the geometry used by the actual QML clip gestures.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const editing = vm.createContext({});
vm.runInContext(fs.readFileSync(new URL('../qml/SoundAssemblyEditing.js', import.meta.url), 'utf8').replace('.pragma library', ''), editing);
const plain = value => JSON.parse(JSON.stringify(value));
const clip = (extra = {}) => ({id:'a',sourceStartMillis:1000,sourceEndMillis:6000,timelineStartMillis:500,fadeInMillis:1000,fadeOutMillis:2000,...extra});
test('trim never reveals negative timeline or out-of-source audio', () => {
    const left = editing.trim(clip(), 'left', -5000, 8000);
    assert.equal(left.timelineStartMillis, 0); assert.equal(left.sourceStartMillis, 500);
    const right = editing.trim(clip(), 'right', 99999, 8000);
    assert.equal(right.sourceEndMillis, 8000);
    const tiny = editing.trim(clip(), 'right', -99999, 8000);
    assert.equal(editing.duration(tiny), 10);
    assert.equal(tiny.fadeInMillis + tiny.fadeOutMillis, 10);
});
test('snapping considers both edges and respects a pixel-sized magnetic range', () => {
    assert.deepEqual(plain(editing.snap(1506, 500, [2000], 1000, 10, true)), {position:1500,guide:2000});
    assert.deepEqual(plain(editing.snap(1506, 500, [2000], 1000, 5, true)), {position:1506,guide:-1});
    assert.deepEqual(plain(editing.snap(1506, 500, [2000], 1000, 10, false)), {position:1506,guide:-1});
    assert.equal(editing.snap(2, 500, [496], 1000, 10, true).position, 0);
});
test('split preserves the outer envelopes and clears the inner seam', () => {
    const [a,b] = editing.split(clip(), 2000);
    assert.equal(a.sourceEndMillis, b.sourceStartMillis);
    assert.equal(editing.end(a), b.timelineStartMillis);
    assert.equal(a.fadeOutMillis, 0); assert.equal(b.fadeInMillis, 0);
    assert.equal(a.fadeInMillis, 1000); assert.equal(b.fadeOutMillis, 2000);
    assert.equal(editing.split(clip(), 501), null);
});
test('crossfade is bounded by actual overlap and rejects contained clips', () => {
    const a=clip({timelineStartMillis:0,fadeInMillis:0}), b=clip({id:'b',timelineStartMillis:4000,fadeOutMillis:0});
    assert.deepEqual(plain(editing.crossfade(b,a)), {first:'a',second:'b',duration:1000});
    assert.equal(editing.crossfade(a,clip({id:'b',timelineStartMillis:1000,sourceEndMillis:3000})),null);
    assert.equal(editing.crossfade(a,clip({id:'b',timelineStartMillis:5500})),null);
});
test('waveform projection follows trimmed, muted, hidden and gap segments', () => {
    const source={editSegments:[
        {sourceStartMillis:0,sourceEndMillis:2000,state:0,gapAfterMillis:200},
        {sourceStartMillis:2000,sourceEndMillis:3000,state:2,gapAfterMillis:0},
        {sourceStartMillis:3000,sourceEndMillis:4000,state:1,gapAfterMillis:0},
    ]};
    const spans=editing.sourceSpans(source,6000);
    assert.equal(spans.at(-1).end,3200);
    const view=plain(editing.visibleSpans(spans,clip({sourceStartMillis:1000,sourceEndMillis:3000})));
    assert.equal(view[0].sourceStart,1000); assert.equal(view[0].length,1000);
    assert.equal(view[1].silent,true); assert.equal(view[1].length,200);
    assert.equal(view[2].silent,true); assert.equal(view[2].sourceStart,3000);
});
test('grid remains legible at every supported zoom', () => {
    for (const zoom of [0.02,0.1,1,15,90,240,800]) assert.ok(editing.gridSeconds(zoom)*zoom>=70);
});

test('duplicated and undone clips resolve their pinned source, never a newer library revision', () => {
    const c={assetId:'a',id:'duplicate',adjustmentRevisionId:7};
    const old={clipId:'original',path:'/audio.wav',adjustmentRevisionId:7,trimStartMillis:100};
    const asset={id:'a',path:'/audio.wav',adjustmentRevision:9};
    assert.equal(editing.pinnedSource(c,[old],[asset]),old);
    assert.equal(editing.pinnedSource(c,[],[asset]),null);
    assert.deepEqual(plain(editing.pinnedSource({...c,adjustmentRevisionId:0},[old],[asset])),{});
});

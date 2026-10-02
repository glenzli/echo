// Run the production binding helper against document replacement states.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const source=fs.readFileSync(new URL('../qml/SoundAssemblyWorkspace.qml',import.meta.url),'utf8');
const body=source.match(/    function crossfadeCandidate\(\): var \{([\s\S]*?)^    \}/m)?.[1];
assert.ok(body);
const editing=vm.createContext({});
vm.runInContext(fs.readFileSync(new URL('../qml/SoundAssemblyEditing.js',import.meta.url),'utf8').replace('.pragma library',''),editing);
const clip=(id,start=0)=>({id,sourceStartMillis:0,sourceEndMillis:1000,timelineStartMillis:start,fadeInMillis:0,fadeOutMillis:0});
function candidate(state) {
    const context=vm.createContext({Editing:editing,...state});
    return vm.runInContext('(function(){'+body+'})()',context);
}
test('removing the last track safely clears crossfade availability before selection catches up',()=>{
    assert.equal(candidate({selectedClip:clip('removed'),selectedClipId:'removed',selectedTrackIndex:0,tracks:[]}),null);
});
test('a replaced track cannot crossfade with a stale selected clip from the old document',()=>{
    assert.equal(candidate({selectedClip:clip('removed'),selectedClipId:'removed',selectedTrackIndex:0,tracks:[{clips:[clip('other',500)]}]}),null);
});
test('current overlapping clips still offer the closest valid crossfade',()=>{
    const selected=clip('current');
    const result=candidate({selectedClip:selected,selectedClipId:'current',selectedTrackIndex:0,
        tracks:[{clips:[selected,clip('overlap',500),clip('outside',2000)]}]});
    assert.equal(result.first,'current');assert.equal(result.second,'overlap');assert.equal(result.duration,500);
});

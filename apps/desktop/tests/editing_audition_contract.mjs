// Execute the actual workspace transition, with a transport instead of an audio device.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const source = fs.readFileSync(new URL('../qml/SoundEditingWorkspace.qml', import.meta.url), 'utf8');
const body = source.match(/    function setOriginalAudition\(enabled: bool\): void \{([\s\S]*?)^    \}/m)?.[1];
assert.ok(body, 'workspace owns the real A/B transition');
const playBody=source.match(/    function togglePlayback\(\): void \{([\s\S]*?)^    \}/m)?.[1];
assert.ok(playBody);
function context({active=true, paused=false, own=true}={}) {
    const state = {auditionOriginal:false,hasAsset:true,asset:{path:'/synthetic.wav',pathStatus:'available'},loadedPath:'/synthetic.wav',
        loads:[],loadedSide:false,diagnosticMode:'',draft:{gainCentibels:-600},player:{active,paused,position:1750},
        defaultPlaybackStart:()=>500};
    state.ownsActivePlayback = () => state.player.active && own && state.auditionOriginal===state.loadedSide;
    state.playFrom = millis => { state.loads.push(millis);state.player.active=true;state.player.paused=false;state.loadedSide=state.auditionOriginal; };
    state.player.togglePause = () => {state.player.paused=!state.player.paused;};
    vm.createContext(state);
    vm.runInContext('function switchAudition(enabled) {'+body+'} function play() {'+playBody+'}',state);
    return state;
}
test('paused A/B keeps position, remains paused, and preserves the draft in both directions', () => {
    const s=context({paused:true}), before=JSON.stringify(s.draft);
    s.switchAudition(true); assert.equal(s.player.paused,true); assert.deepEqual(s.loads,[]);
    s.switchAudition(false); assert.equal(s.player.paused,true); assert.deepEqual(s.loads,[]);
    s.switchAudition(true); s.play(); assert.equal(s.player.paused,false); assert.deepEqual(s.loads,[1750]);
    assert.equal(JSON.stringify(s.draft),before);
});
test('playing A/B keeps playing at the same source position', () => {
    const s=context();s.switchAudition(true);assert.equal(s.player.paused,false);assert.deepEqual(s.loads,[1750]);
});
test('stopped or unrelated playback is never started by comparison controls', () => {
    for(const mode of [{active:false},{own:false}]) {const s=context(mode);s.switchAudition(true);assert.deepEqual(s.loads,[]);}
});
test('choosing the current comparison side does not rebuild playback', () => {
    const s=context({paused:true});s.switchAudition(false);assert.deepEqual(s.loads,[]);assert.equal(s.player.paused,true);
});

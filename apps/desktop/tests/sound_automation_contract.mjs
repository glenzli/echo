import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const automation = vm.createContext({}), editing = vm.createContext({});
for (const [file, context] of [['SoundAutomation', automation], ['SoundAssemblyEditing', editing]])
    vm.runInContext(fs.readFileSync(new URL(`../qml/${file}.js`, import.meta.url), 'utf8').replace('.pragma library', ''), context);
const plain = value => JSON.parse(JSON.stringify(value));
test('source-time curves survive moving, splitting and revealing trimmed audio', () => {
    const clip = {sourceStartMillis:1000, sourceEndMillis:5000,timelineStartMillis:3000,fadeInMillis:0,fadeOutMillis:0,
        gainEnvelope:{enabled:true,points:[{sourceMillis:0,gainCentibels:0},{sourceMillis:6000,gainCentibels:-1200}]}};
    const [a,b] = editing.split(clip,5000);
    assert.equal(automation.gainAt(a.gainEnvelope.points,a.sourceEndMillis), -600);
    assert.equal(automation.gainAt(b.gainEnvelope.points,b.sourceStartMillis), -600);
    const revealed = editing.trim(a,'left',-1000,6000);
    assert.equal(automation.gainAt(revealed.gainEnvelope.points,revealed.sourceStartMillis),0);
    const modified = automation.setPoint(b.gainEnvelope.points,3000,-2000);
    assert.equal(clip.gainEnvelope.points.length,2);
    assert.equal(modified.length,3);
});
test('activity maps original buckets through cuts, silent spans and clip offsets', () => {
    const spans = [{start:0,end:1000,sourceStart:2000,silent:false},{start:1000,end:1500,sourceStart:0,silent:true},
                   {start:1500,end:2500,sourceStart:4000,silent:false}];
    const clip = {sourceStartMillis:500,sourceEndMillis:2200,timelineStartMillis:10000};
    assert.deepEqual(plain(automation.activity(clip,spans,[0,0,1,1,0.1,0],6000,-30)),[[10000,10500],[11000,11700]]);
    assert.deepEqual(plain(automation.activity(clip,spans,[0,0,0,0,0,0],6000,-30)),[]);
});
test('ducking joins short pauses, respects target source coordinates and keeps silence neutral', () => {
    const regions = automation.duckRegions([[1000,2000],[2300,2700],[6000,7000]],200,300,500);
    assert.deepEqual(plain(regions),[[800,1000,3000,3500],[5800,6000,7300,7800]]);
    const clip = {sourceStartMillis:4000,sourceEndMillis:12000,timelineStartMillis:500};
    const envelope = automation.duckEnvelope(clip,regions,1200);
    assert.equal(automation.gainAt(envelope.points,4400),-600);
    assert.equal(automation.gainAt(envelope.points,5500),-1200);
    assert.equal(automation.gainAt(envelope.points,8000),0);
    assert.equal(automation.gainAt(envelope.points,12000),0);
    const cut = automation.duckEnvelope({...clip, timelineStartMillis:1500,sourceEndMillis:4300},regions,1200);
    assert.ok(cut.points.every(p => p.gainCentibels === -1200));
});
test('zero-time attack has unique coordinates, complexity remains bounded', () => {
    const clip={sourceStartMillis:0,sourceEndMillis:14400000,timelineStartMillis:0};
    const envelope=automation.duckEnvelope(clip,automation.duckRegions([[0,1000]],100,0,100),600);
    assert.equal(envelope.points[0].gainCentibels,-600);
    assert.ok(envelope.points.every((p,i) => !i || p.sourceMillis > envelope.points[i-1].sourceMillis));
    assert.throws(() => automation.duckEnvelope(clip,Array.from({length:600},(_,i)=>[i*10000,i*10000+10,i*10000+20,i*10000+30]),1200), /complexity/);
});

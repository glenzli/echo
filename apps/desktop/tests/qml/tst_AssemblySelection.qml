import QtQuick
import QtTest
import EchoDesktop
import "../../qml/SoundAssemblySelection.js" as Selection

TestCase {
    name: "AssemblySelectionRuntime"
    property int sequence: 0
    function newId() { return "new-" + (++sequence); }
    function clip(id, start, length) {
        return {id:id,assetId:"source",adjustmentRevisionId:7,sourceStartMillis:0,sourceEndMillis:length,timelineStartMillis:start,fadeInMillis:0,fadeOutMillis:0};
    }
    function test_batch_transforms_in_the_product_script_engine() {
        const tracks=[{clips:[clip("a",1000,2000)]},{clips:[clip("b",0,8000)]}];
        compare(Selection.bounds(tracks,["a","b"]).end,8000);
        compare(Selection.move(tracks,["a","b"],1000)[1].clips[0].timelineStartMillis,1000);
        const duplicate=Selection.duplicate(tracks,["a","b"],newId);
        compare(duplicate.ids.length,2);compare(duplicate.tracks[0].clips.length,2);
        const split=Selection.split(tracks,["a","b"],2000,newId);
        compare(split.ids.length,4);compare(split.tracks[1].clips[1].sourceStartMillis,2000);
        const ripple=Selection.ripple(tracks,["a"],true,newId);
        compare(ripple.tracks[0].clips.length,0);compare(ripple.tracks[1].clips.length,2);
        compare(ripple.tracks[1].clips[1].sourceStartMillis,3000);
        compare(ripple.tracks[1].clips[1].timelineStartMillis,1000);
        compare(Selection.remove(tracks,["a","b"]).error,"empty");
        compare(Selection.moveTracks(tracks,["a","b"],-1).error,"track");
    }
    function test_full_capacity_movement_stays_bounded() {
        const tracks=[], ids=[];
        for(let t=0;t<8;++t) {
            const clips=[];
            for(let c=0;c<32;++c) { const id=t+":"+c;clips.push(clip(id, c*1000,1000));ids.push(id); }
            tracks.push({clips:clips});
        }
        const start=Date.now();
        for(let i=0;i<100;++i) {
            const moved=Selection.move(tracks,ids,i*1000);
            compare(moved[7].clips[31].timelineStartMillis,31000+i*1000);
        }
        console.log("256-clip batch movement, 100 transforms:",Date.now()-start,"ms");
        compare(Selection.duplicate(tracks,ids,newId).error,"capacity");
    }
}

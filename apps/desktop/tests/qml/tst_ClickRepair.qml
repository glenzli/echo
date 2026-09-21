import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test
    name:"ClickRepair"; width:1100; height:460; visible:true; when:windowShown
    property var asset:({id:"source",path:"fixture.wav",pathStatus:"available",durationMillis:10000})
    property var located:[]
    property var lastResult:null
    QtObject {
        id: scanner
        property bool running:false
        property bool cancelling:false
        property string errorText:""
        property string resultKey:""
        property var result:({})
        signal stateChanged()
        function cancel(){running=false;resultKey="";result={};stateChanged();}
        function scan(path,start,end,sensitivity,maximum,key){running=true;stateChanged();}
    }
    SoundAdjustmentDraft { id:draft; asset:test.asset }
    SoundClickRepairPanel {
        id:panel; anchors.fill:parent; asset:test.asset; draft:draft; controller:scanner; rangeStart:0; rangeEnd:10000
        onRangeRequested:(start,end,play,original)=>test.located.push([start,end,play,original])
        onRepairRequested:candidates=>{test.lastResult=draft.applyClickRepairs(candidates);panel.finishEdit(test.lastResult);}
    }
    function init(){
        draft.resetFromAsset();panel.visible=true;panel.rangeStart=0;panel.rangeEnd=10000;panel.selectedIndices=[];panel.notice="";located=[];lastResult=null;
        scanner.running=false;scanner.cancelling=false;
        scanner.result={items:[{startMillis:1000,endMillis:1001,channelMask:1},{startMillis:4000,endMillis:4002,channelMask:2}],total:2,startMillis:0,endMillis:10000};scanner.resultKey=panel.identity;scanner.stateChanged();wait(30);
    }
    function test_review_apply_undo_and_duplicate(){
        const before=JSON.stringify(draft.snapshot());
        mouseClick(findChild(panel,"clickCheck-0"));compare(panel.selectedEntries.length,1);
        mouseClick(findChild(panel,"clickAudition-0"));compare(located[0],[750,1251,true,true]);
        panel.rangeStart=750;panel.rangeEnd=1251;compare(panel.selectedEntries.length,1);compare(panel.candidates.length,2);
        mouseClick(findChild(panel,"repairClicks"));verify(lastResult.ok);verify(lastResult.changed);compare(draft.effectMasks.length,1);verify(draft.deClickEnabled);
        const history=draft._history.length;panel.applySelected();verify(!lastResult.changed);compare(draft._history.length,history);
        draft.undo();compare(JSON.stringify(draft.snapshot()),before);verify(!draft.canUndo);draft.redo();compare(draft.effectMasks.length,1);
        mouseClick(findChild(panel,"auditionRepairedClick"));compare(located[1],[750,1251,true,false]);
    }
    function test_repairs_preserve_newest_effects_and_full_chain(){
        draft.addEffectNode(20);draft.addEffectNode(21);
        const before=JSON.stringify(draft.snapshot());panel.selectAll();panel.applySelected();verify(lastResult.ok);
        verify(draft.effectChain.indexOf(20)>=0);verify(draft.effectChain.indexOf(21)>=0);
        draft.undo();compare(JSON.stringify(draft.snapshot()),before);
        for(let node=0;node<22;++node)if(node!==4)draft.addEffectNode(node);
        compare(draft.effectChain.length,22);compare(draft.snapshot().effectChain.length,22);
    }
    function test_invalidated_results_and_stable_busy_button(){
        const button=findChild(panel,"scanClicks"),width=button.width;
        mouseClick(button);verify(scanner.running);compare(button.width,width);mouseClick(button);verify(!scanner.running);compare(button.width,width);
        compare(panel.candidates.length,0);verify(!findChild(panel,"repairClicks").enabled);
    }
    function test_parameter_and_visibility_invalidation(){
        panel.selectAll();draft.setDeClickParameter("sensitivity",85);compare(panel.candidates.length,0);compare(panel.selectedEntries.length,0);
        scanner.resultKey=panel.identity;scanner.result={items:[{startMillis:500,endMillis:501,channelMask:1}],total:1};scanner.stateChanged();panel.selectAll();panel.visible=false;compare(panel.selectedEntries.length,0);compare(scanner.resultKey,"");
    }
    function test_batch_admission_and_scope_preservation(){
        panel.selectAll();panel.applySelected();verify(lastResult.ok);compare(draft.effectMasks.length,2);
        draft.undo();draft.addEffectNode(6);draft.setDeClickEnabled(true);const before=JSON.stringify(draft.snapshot()),history=draft._history.length;
        panel.applySelected();compare(lastResult.error,"global");compare(JSON.stringify(draft.snapshot()),before);compare(draft._history.length,history);verify(panel.notice.length>0);
    }
}

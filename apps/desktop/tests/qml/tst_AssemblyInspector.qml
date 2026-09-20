import QtQuick
import QtQuick.Controls
import QtTest
import EchoDesktop

TestCase {
    id: tests
    name: "AssemblyInspector"
    width: 650; height: 900
    visible: true
    when: windowShown
    property var changes: []
    QtObject {
        id: workspace
        property var selectedClip: ({id:"clip",sourceStartMillis:0,sourceEndMillis:8000,timelineStartMillis:0,gainCentibels:0,panPercent:0,fadeInMillis:120,fadeOutMillis:300,fadeInCurve:"linear",fadeOutCurve:"smooth",sourceRole:"material",adjustmentRevisionId:3,muted:false})
        property var document: ({tracks:[{name:"Voice",clips:[]},{name:"Music",clips:[]}],master:{gainCentibels:0,limiterEnabled:true,limiterCeilingCentibels:-100}})
        property var tracks: document.tracks
        property var libraryAssets: []
        property int selectedTrackIndex: 0
        property int selectionCount: 1
        property bool hasDocument: true
        property bool independentMode: false
        property bool automationEditing: false
        property bool duckingVisible: false
        function sourceName(clip) { return clip ? "Long field recording beside the river.wav" : ""; }
        function sourceDurationFor(clip) { return 12000; }
        function totalClipCount() { return 2; }
        function fadeCurveIndex(value) { return ["linear","smooth","equal_power"].indexOf(value); }
        function fadeCurveValue(index) { return ["linear","smooth","equal_power"][index]; }
        function setClipTiming(key,value) { tests.changes.push({scope:"timing",key:key,value:value}); }
        function setClipValue(key,value) { tests.changes.push({scope:"clip",key:key,value:value}); }
        function setMasterValue(key,value) { tests.changes.push({scope:"master",key:key,value:value}); }
        function duplicateSelectedClip() { tests.changes.push({scope:"duplicate"}); }
        function splitSelectedClip() { tests.changes.push({scope:"split"}); }
        function openClipEditor() { tests.changes.push({scope:"source"}); }
    }
    SoundAssemblyInspector {
        id: inspector
        width: 280; height: tests.height
        workspace: workspace
        renderController: ({running:false,hasResult:false,integratedLufs:-18,truePeakDbtp:-1})
        waveforms: ({})
    }
    EchoTimeSpinBox { id: time; x: 330; width: 128; from: 0; to: 10000; value: 1200 }
    SignalSpy { id: numericEdits; target: time; signalName: "valueModified" }
    function init() { changes=[];numericEdits.clear(); }
    function test_seconds_input_and_increment_retain_millisecond_storage() {
        time.value=1200;
        mouseClick(time,time.width-12,time.height/2);
        compare(time.value,1210);compare(numericEdits.count,1);
        const input=findChild(time,"numericInput");input.forceActiveFocus();input.selectAll();
        for(const key of [Qt.Key_2,Qt.Key_Period,Qt.Key_3,Qt.Key_4,Qt.Key_5]) keyClick(key);keyClick(Qt.Key_Return);
        compare(time.value,2345);compare(numericEdits.count,2);
    }
    function test_parameter_columns_fit_and_keep_same_typography() {
        for(const key of ["timelineStartMillis","sourceStartMillis","sourceEndMillis","fadeInMillis","fadeOutMillis"]) {
            const field=findChild(inspector,"inspector-"+key);
            verify(field!==null);compare(field.width,128);compare(field.height,26);compare(field.font.pixelSize,Theme.fontSection);
            const point=field.mapToItem(inspector,field.width,0);verify(point.x<=inspector.width-8);
        }
        const scroll=findChild(inspector,"assemblyInspectorScroll");compare(scroll.contentWidth,scroll.availableWidth);
    }
    function test_timing_and_fade_edits_route_one_command_with_bounded_values() {
        const position=findChild(inspector,"inspector-timelineStartMillis");
        mouseClick(position,position.width-12,13);
        compare(changes.length,1);compare(changes[0].scope,"timing");compare(changes[0].value,10);
        const fade=findChild(inspector,"inspector-fadeInMillis");
        compare(fade.to,7700);
        mouseClick(fade,fade.width-12,13);
        compare(changes.length,2);compare(changes[1].key,"fadeInMillis");compare(changes[1].value,121);
    }
    function test_existing_commands_and_ducking_interface_remain_available() {
        mouseClick(findChild(inspector,"inspectorDuplicate"));
        compare(changes[0].scope,"duplicate");
        verify(inspector.ducking!==null);compare(inspector.ducking.references.length,1);
        workspace.duckingVisible=true;
        verify(waitForPolish(inspector.Window.window));
        verify(waitForRendering(inspector));
        verify(inspector.ducking.width<=256);
        workspace.duckingVisible=false;
    }
    function test_ceiling_shows_db_and_publishes_centibels() {
        const ceiling=findChild(inspector,"inspectorMasterCeiling");
        compare(ceiling.textFromValue(-125,Qt.locale("en_US")),"-1.25");
        compare(ceiling.valueFromText("-0.75",Qt.locale("en_US")),-75);
    }
}

import QtQuick
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "AssemblyMarkers"
    width: 740; height: 680
    visible: true
    when: windowShown
    property var actions: []
    QtObject {
        id: workspace
        property var markers: [{id:"point",name:"Door",startMillis:500}, {id:"range",name:"Conversation",startMillis:1000,endMillis:2500}]
        property string selectedMarkerId: "range"
        property int durationMillis: 3000
        property var selectionBounds: ({start:100,end:2000})
        property int trackHeaderWidth: 40
        property real pixelsPerSecond: 90
        property real scrollPosition: 0
        property bool markersVisible: false
        function formatTime(value) { return (value / 1000).toFixed(0); }
        function addMarker(range) { test.actions.push(["add",range]); }
        function patchMarker(id,key,value) { test.actions.push(["patch",id,key,value]); }
        function seekMarker(id) { test.actions.push(["seek",id]); }
        function previewMarker(id) { test.actions.push(["preview",id]); }
        function deleteMarker(id) { test.actions.push(["delete",id]); }
    }
    SoundAssemblyMarkers { id: panel; width: 280; height: 660; workspace: workspace }
    SoundAssemblyMarkerLane { id: lane; x:300; y:10; width:420; height:24; workspace:workspace }
    function init() { actions=[];workspace.selectedMarkerId="range";workspace.durationMillis=3000;workspace.selectionBounds={start:100,end:2000}; }
    function test_add_and_navigate() {
        mouseClick(findChild(panel,"addPointMarker")); mouseClick(findChild(panel,"addRangeMarker"));
        compare(actions[0],["add",false]);compare(actions[1],["add",true]);
        mouseClick(findChild(panel,"marker-point"));
        compare(workspace.selectedMarkerId,"point");compare(actions[2],["seek","point"]);
        verify(!findChild(panel,"previewMarkerRange").enabled);
    }
    function test_lane_tap_navigates_and_reveals_editor() {
        workspace.markersVisible = false;
        mouseClick(lane,88,12);
        compare(workspace.selectedMarkerId,"point"); verify(workspace.markersVisible);
        compare(actions[0],["seek","point"]);
        mouseClick(lane,150,12);
        compare(workspace.selectedMarkerId,"range");compare(actions[1],["seek","range"]);
    }
    function test_range_bounds_and_audition() {
        const start=findChild(panel,"markerStart"), end=findChild(panel,"markerEnd");
        compare(start.to,2499);compare(end.from,1001);
        mouseClick(start,start.width-12,start.height/2);
        compare(actions[0],["patch","range","startMillis",1010]);
        mouseClick(findChild(panel,"previewMarkerRange"));compare(actions[1],["preview","range"]);
        workspace.durationMillis=2000; verify(!findChild(panel,"previewMarkerRange").enabled);
        mouseClick(findChild(panel,"deleteMarker"));compare(actions[2],["delete","range"]);
    }
    function test_empty_name_is_restored_and_missing_selection_disables_range_creation() {
        const name=findChild(panel,"markerName");name.forceActiveFocus();name.selectAll();keyClick(Qt.Key_Backspace);keyClick(Qt.Key_Return);
        compare(name.text,"Conversation");compare(actions.length,0);
        workspace.selectionBounds=null;verify(!findChild(panel,"addRangeMarker").enabled);
    }
}

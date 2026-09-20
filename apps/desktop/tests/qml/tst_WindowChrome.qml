import QtQuick
import QtQuick.Window
import QtTest
import EchoDesktop

TestCase {
    id: testCase
    name: "WindowChrome"
    visible: true
    width: 1500; height: 180
    when: windowShown
    QtObject {
        id: host
        property int visibility: Window.Windowed
        property int moves: 0
        function startSystemMove() { ++moves; }
    }
    MainTitleBar {
        id: library
        width: testCase.width; height: implicitHeight
        hostWindow: host; editor: ({dirty:false,canUndo:false,canRedo:false}); assembly: ({dirty:false,canUndo:false,canRedo:false})
        workspaceIndex: 0; editorAvailable: false; jobsActive: false; activeJobCount: 0
    }
    IndependentTitleBar {
        id: independent
        y: 80
        width: testCase.width; height: implicitHeight
        hostWindow: host; projectName: "A long field recording project with music and narration.echo"
        dirty: true; processing: false; hasSource: true; multitrack: false
    }
    SignalSpy { id: openAction; target: independent; signalName: "openRequested" }
    SignalSpy { id: modeAction; target: independent; signalName: "multitrackRequested" }

    function centerY(item, target) { return item.mapToItem(target, item.width/2, item.height/2).y; }
    function test_brand_and_action_share_vertical_center() {
        const brand=findChild(library,"brandLabel"), button=findChild(library,"independentEditorButton");
        compare(library.height,44); compare(independent.height,44);
        verify(Math.abs(centerY(brand,library)-22)<0.6);
        compare(centerY(button,library),22);
        for (const name of ["openAudioButton","saveProjectButton","exportAudioButton"])
            compare(centerY(findChild(independent,name),independent),22);
    }
    function test_document_header_stays_centered_and_separate_data() {
        return [{tag:"minimum",w:1240},{tag:"wide",w:1800},{tag:"fullscreen",w:1500,full:true}];
    }
    function test_document_header_stays_centered_and_separate(data) {
        testCase.width=data.w;host.visibility=data.full ? Window.FullScreen : Window.Windowed;
        waitForRendering(independent);
        const modes=findChild(independent,"editorModes");
        const identity=findChild(independent,"projectIdentity"), actions=findChild(independent,"projectActions");
        const left=modes.mapToItem(independent,0,0).x;
        verify(Math.abs(left+modes.width/2-independent.width/2)<0.6);
        verify(identity.mapToItem(independent,identity.width,0).x+12<=left);
        verify(left+modes.width+12<=actions.mapToItem(independent,0,0).x);
        if (Qt.platform.os==="osx") compare(independent.leftPadding,data.full ? 16 : 96);
    }
    function test_buttons_route_actions_without_starting_window_drag() {
        openAction.clear();modeAction.clear();host.moves=0;
        mouseClick(findChild(independent,"openAudioButton"));
        compare(openAction.count,1);compare(host.moves,0);
        mouseClick(findChild(findChild(independent,"editorModes"),"segmentedChoice-1"));
        compare(modeAction.count,1);compare(host.moves,0);
        independent.processing=true;
        mouseClick(findChild(independent,"openAudioButton"));
        compare(openAction.count,1);
        independent.processing=false;
    }
    function test_blank_chrome_moves_the_window() {
        host.moves=0;
        mousePress(independent,independent.width-280,22,Qt.LeftButton);
        mouseMove(independent,independent.width-250,24,50,Qt.LeftButton);
        mouseRelease(independent,independent.width-250,24,Qt.LeftButton);
        tryCompare(host,"moves",1);
    }
}

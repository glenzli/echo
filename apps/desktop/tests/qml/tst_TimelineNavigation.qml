import QtQuick
import QtTest
import EchoDesktop
TestCase {
    id: test
    name: "TimelineNavigation"
    width: 1000; height: 400; visible: true; when: windowShown
    property int plays: 0
    SoundAdjustmentDraft {
        id: draft
        asset: ({id:"long",path:"long.wav",durationMillis:7200000,trimStartMillis:0,trimEndMillis:7200000,fadeInMillis:0,fadeOutMillis:0,gainCentibels:0,lowCutHertz:0})
    }
    SoundEditorTimeline {
        id: timeline; anchors.fill: parent; draft: draft; waveformLevels: []
        sourceDurationMillis: draft.sourceDurationMillis; trimStartMillis: draft.trimStartMillis; trimEndMillis: draft.trimEndMillis
        fadeInMillis: 0; fadeOutMillis: 0; fadeInCurve: 0; fadeOutCurve: 0; gainCentibels: 0
        playbackPositionMillis: 3000; isPlaying: false
        onPlayPauseRequested: ++test.plays
    }
    function init() { draft.resetFromAsset();timeline.clearTimeSelection();plays=0;timeline.forceActiveFocus(); }
    function test_trim_clamps_selection_and_clears_empty_loop() {
        timeline.selectionStartMillis=1000;timeline.selectionEndMillis=10000;timeline.hasTimeSelection=true;timeline.loopSelection=true;
        draft.setTrimRange(5000,8000);
        tryCompare(timeline,"selectionStartMillis",5000);compare(timeline.selectionEndMillis,8000);
        draft.setTrimRange(11000,15000);
        tryCompare(timeline,"hasTimeSelection",false);verify(!timeline.loopSelection);
    }
    function test_exact_range_button_on_long_source_only_changes_view_state() {
        const saved=JSON.stringify(draft.snapshot());
        mouseClick(findChild(timeline,"exactTimeRangeButton"));
        tryCompare(timeline.exactTimeDialog,"visible",true);
        findChild(timeline.exactTimeDialog,"exactTimeStart").text="1:59:01.123";
        findChild(timeline.exactTimeDialog,"exactTimeEnd").text="1:59:08.456";
        mouseClick(findChild(timeline.exactTimeDialog,"applyExactTime"));
        compare(timeline.selectionStartMillis,7141123);compare(timeline.selectionEndMillis,7148456);
        verify(timeline.viewStartMillis<=7141123 && timeline.viewEndMillis>=7148456);
        compare(JSON.stringify(draft.snapshot()),saved);verify(!draft.dirty);compare(plays,0);
    }
    function test_modified_keys_do_not_play_or_change_selection() {
        keyClick(Qt.Key_Space,Qt.ControlModifier);compare(plays,0);
        keyClick(Qt.Key_I,Qt.MetaModifier);verify(!timeline.hasTimeSelection);
        keyClick(Qt.Key_O,Qt.ControlModifier);verify(!timeline.hasTimeSelection);
        keyClick(Qt.Key_Space);compare(plays,1);
        keyClick(Qt.Key_I);verify(timeline.hasTimeSelection);compare(timeline.selectionStartMillis,3000);
    }
}

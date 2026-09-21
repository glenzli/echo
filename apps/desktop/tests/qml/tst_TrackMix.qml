import QtQuick
import QtQuick.Controls
import QtTest
import EchoDesktop

TestCase {
    id: test
    name: "TrackMix"
    width: 700; height: 300
    visible: true
    when: windowShown
    SoundAssemblyTrack {
        id: track
        width: 680
        track: ({id:"track",name:"Ambience",clips:[],gainCentibels:0,panPercent:0,muted:false,solo:false})
        trackIndex: 0
        pixelsPerSecond: 90
        playheadMillis: 0
        selectedClipId: ""
        canDeleteTrack: false
    }
    SignalSpy { id: edits; target: track; signalName: "trackValueRequested" }
    SignalSpy { id: previews; target: track; signalName: "trackPreviewRequested" }
    function init() {
        edits.clear(); previews.clear();
        track.track = {id:"track",name:"Ambience",clips:[],gainCentibels:0,panPercent:0,muted:false,solo:false};
    }
    function sliderInside(item) {
        for (const child of item.children) {
            if (child instanceof Slider) return child;
            const nested = sliderInside(child);
            if (nested) return nested;
        }
        return null;
    }
    function test_drag_auditions_without_history_then_commits_once_data() {
        return [{tag:"gain",name:"trackGain",key:"gainCentibels"}, {tag:"pan",name:"trackPan",key:"panPercent"}];
    }
    function test_drag_auditions_without_history_then_commits_once(data) {
        const field = findChild(track, data.name), slider = sliderInside(field);
        verify(slider !== null);
        const start = slider.leftPadding + slider.visualPosition * slider.availableWidth;
        mousePress(slider, start, slider.height / 2);
        mouseMove(slider, slider.width * 0.3, slider.height / 2, 30);
        mouseMove(slider, slider.width * 0.8, slider.height / 2, 30);
        compare(edits.count, 0); verify(previews.count > 0);
        compare(previews.signalArguments[0][1], data.key);
        mouseRelease(slider, slider.width * 0.8, slider.height / 2);
        compare(edits.count, 1); compare(edits.signalArguments[0][1], data.key);
        compare(track.track[data.key], 0);
    }
    function test_keyboard_commits_immediately() {
        const slider = sliderInside(findChild(track,"trackPan"));
        slider.forceActiveFocus(); keyClick(Qt.Key_Right);
        compare(edits.count, 1); compare(previews.count, 1);
        compare(edits.signalArguments[0][2], 1);
    }
}

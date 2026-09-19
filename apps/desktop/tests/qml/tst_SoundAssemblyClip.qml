import QtQuick
import QtTest
import EchoDesktop
import "../../qml/SoundAssemblyEditing.js" as Editing

TestCase {
    id: testCase
    name: "SoundAssemblyClipGestures"
    visible: true
    width: 1000; height: 200
    when: windowShown
    property var initialClip: ({id: "test", assetId: "source", sourceRole: "material", sourceStartMillis: 0, sourceEndMillis: 5000, timelineStartMillis: 1000,
                             fadeInMillis: 0, fadeOutMillis: 0, fadeInCurve: "linear", fadeOutCurve: "linear", muted: false})
    SoundAssemblyClip {
        id: clip
        clipData: testCase.initialClip
        title: "Rain · fixture"
        sourceSpans: [{start: 0, end: 8000, sourceStart: 0, sourceEnd: 8000, silent: false}]
        sourceDuration: 8000; originalDuration: 8000
        waveformLevels: [{mins: [-0.1,-0.4,-0.2,-0.6], maxs:[0.1,0.4,0.2,0.6]}]
        pixelsPerSecond: 100; selected: true; trackColor: "#498eba"
        snapPosition: (position, length, id, bypass) => Editing.snap(position, length, [2500], 1000, 80, !bypass)
    }
    EchoIcon { id: tintedIcon; x: 800; y: 100; width: 48; height: 48; size: 48; source: Qt.resolvedUrl("../../icons/folder.svg"); color: "#81b5e6" }
    EchoComboBox { id: translatedChoice; x: 800; y: 30; model: ["Linear", "Smooth", "Equal power"]; selectionIndex: 2 }
    SignalSpy { id: patches; target: clip; signalName: "patchRequested" }
    function initTestCase() { Theme.mode = Theme.AppearanceMode.Dark; }
    function init() { patches.clear(); }
    function dragAt(x, y, dx, modifiers) {
        mousePress(clip, x, y, Qt.LeftButton, modifiers || Qt.NoModifier);
        mouseMove(clip, x + dx / 2, y, 40, Qt.LeftButton, modifiers || Qt.NoModifier);
        mouseMove(clip, x + dx, y, 40, Qt.LeftButton, modifiers || Qt.NoModifier);
        mouseRelease(clip, x + dx, y, Qt.LeftButton, modifiers || Qt.NoModifier);
        wait(50);
    }
    function test_source_identity_does_not_follow_AI_caption() {
        const asset = {path: "/Rain.mp3", sourceTitle: "", soundCaption: "Toilet flush", calibratedFields: []};
        compare(SoundSemantics.sourceTitle(asset), "Rain.mp3");
        asset.soundCaption = "Roof after rain"; asset.calibratedFields = ["sound_caption"];
        compare(SoundSemantics.sourceTitle(asset), "Roof after rain");
    }
    function test_translated_model_preserves_semantic_selection() {
        translatedChoice.model = ["线性", "平滑", "等功率"];
        wait(30);
        compare(translatedChoice.currentIndex, 2);
        translatedChoice.model = ["Linear", "Smooth", "Equal power"];
        wait(30);
        compare(translatedChoice.currentIndex, 2);
    }
    function test_icon_uses_requested_color() {
        if (tintedIcon.GraphicsInfo.api === GraphicsInfo.Software) { skip("Run with --native to verify GPU icon coloring"); return; }
        wait(100);
        const pixels = grabImage(tintedIcon);
        const color = pixels.pixel(6, 24);
        verify(color.b > 0.8 && color.g > 0.5 && color.r > 0.4, "black vector did not acquire its light theme color");
    }
    function test_move_is_single_undo_patch() {
        dragAt(150, 50, 50, Qt.ShiftModifier);
        compare(patches.count, 1);
        verify(patches.signalArguments[0][0].timelineStartMillis > 1000);
        compare(clip.gesture, "");
    }
    function test_trim_right_is_bounded() {
        dragAt(clip.width - 4, 55, 60);
        compare(patches.count, 1);
        const patch = patches.signalArguments[0][0];
        verify(patch.sourceEndMillis > 5000 && patch.sourceEndMillis <= 8000);
        compare(patch.timelineStartMillis, 1000);
    }
    function test_fade_handle_publishes_without_moving_clip() {
        dragAt(7, 7, 75);
        compare(patches.count, 1);
        const patch = patches.signalArguments[0][0];
        verify(patch.fadeInMillis > 0);
        compare(patch.timelineStartMillis, 1000);
        compare(patch.sourceStartMillis, 0);
    }
}

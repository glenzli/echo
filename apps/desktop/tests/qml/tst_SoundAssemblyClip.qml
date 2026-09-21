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
    SignalSpy { id: selections; target: clip; signalName: "selectedRequested" }
    SignalSpy { id: previews; target: clip; signalName: "movePreviewRequested" }
    function initTestCase() { Theme.mode = Theme.AppearanceMode.Dark; }
    function init() {
        patches.clear(); selections.clear(); previews.clear(); clip.automationEditing = false;
        Theme.mode = Theme.AppearanceMode.Dark;
        clip.trackColor = "#498eba";
        clip.clipData = initialClip;
    }
    function luminance(color) {
        const linear = value => value <= 0.04045 ? value / 12.92 : Math.pow((value + 0.055) / 1.055, 2.4);
        return 0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b);
    }
    function test_waveform_contrast_data() {
        const rows = [];
        for (const mode of [Theme.AppearanceMode.Light, Theme.AppearanceMode.Dark])
            for (const color of Theme.assemblyTrackColors)
                rows.push({tag: mode + ":" + color, mode: mode, color: color});
        return rows;
    }
    function test_waveform_contrast(data) {
        Theme.mode = data.mode; clip.trackColor = data.color;
        wait(50);
        const pixels = grabImage(clip);
        const scale = pixels.width / clip.width;
        const wave = luminance(pixels.pixel(Math.round(250 * scale), Math.round(48 * scale)));
        const background = luminance(pixels.pixel(Math.round(250 * scale), Math.round(85 * scale)));
        const ratio = (Math.max(wave, background) + 0.05) / (Math.min(wave, background) + 0.05);
        verify(ratio >= 3, "rendered waveform contrast too low: " + ratio);
    }
    function test_fade_curve_only_appears_for_authored_fades() {
        const curve = findChild(clip, "assemblyFadeCurve");
        verify(curve !== null);
        compare(curve.visible, false);
        clip.clipData = Object.assign({}, initialClip, {fadeInMillis: 1000});
        compare(curve.visible, true);
        clip.clipData = initialClip;
        compare(curve.visible, false);
    }
    function test_modifier_click_preserves_selection_intent() {
        mouseClick(clip, 50, 8, Qt.LeftButton, Qt.ControlModifier);
        compare(selections.count, 1);
        verify(selections.signalArguments[0][0] & Qt.ControlModifier);
        compare(selections.signalArguments[0][1], false);
    }
    function test_alt_drag_slips_source_without_moving_timeline() {
        dragAt(150, 50, -50, Qt.AltModifier);
        compare(patches.count, 1);
        const patch = patches.signalArguments[0][0];
        compare(patch.timelineStartMillis, 1000);
        verify(patch.sourceStartMillis > 0);
        compare(patch.sourceEndMillis - patch.sourceStartMillis, 5000);
        compare(patches.signalArguments[0][1], "slip");
    }
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
    function test_envelope_drag_is_one_patch_and_does_not_move_clip() {
        clip.automationEditing = true;
        dragAt(150, 55, 80, Qt.ShiftModifier);
        compare(patches.count, 1);
        const patch = patches.signalArguments[0][0];
        compare(patch.timelineStartMillis, 1000);
        compare(patch.gainEnvelope.enabled, true);
        compare(patch.gainEnvelope.points.length, 1);
        verify(patch.gainEnvelope.points[0].sourceMillis >= 2000);
    }
    function test_move_is_single_undo_patch() {
        dragAt(150, 50, 50, Qt.ShiftModifier);
        compare(patches.count, 1);
        verify(patches.signalArguments[0][0].timelineStartMillis > 1000);
        compare(patches.signalArguments[0][1], "move");
        compare(selections.signalArguments[0][1], true);
        verify(previews.count > 1);
        compare(previews.signalArguments[previews.count - 1][0], 0);
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

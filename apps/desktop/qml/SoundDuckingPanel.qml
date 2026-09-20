//! Explicit source-activity ducking. Work is sliced by clip; edits invalidate candidates.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SoundAssemblyEditing.js" as Editing
import "SoundAutomation.js" as Automation

ColumnLayout {
    id: panel
    required property var document
    required property int targetTrackIndex
    required property var assets
    required property var waveforms
    property bool blocked: false
    property var snapshot: null
    property var candidates: []
    property var intervals: []
    property int cursor: 0
    property string message: ""
    readonly property bool running: stepper.running
    readonly property var references: (document.tracks || []).map((track, index) => ({name: track.name, index: index})).filter(track => track.index !== targetTrackIndex)
    signal applyRequested(var candidates)
    spacing: 8
    onDocumentChanged: cancel()
    onTargetTrackIndexChanged: cancel()
    onBlockedChanged: { if (blocked) cancel(); }
    function cancel(): void { stepper.stop(); snapshot = null; candidates = []; message = ""; }
    function generate(): void {
        cancel();
        const reference = references[referenceBox.currentIndex];
        if (!reference || targetTrackIndex < 0) return;
        snapshot = JSON.parse(JSON.stringify({reference: document.tracks[reference.index], target: document.tracks[targetTrackIndex],
            sources: document.clipSources || [], assets: assets, threshold: threshold.value, amount: amount.value * 100,
            attack: attack.value, hold: hold.value, release: release.value}));
        intervals = []; cursor = 0;
        if (snapshot.reference.muted) { message = qsTr("Unmute the reference track before detecting activity."); return; }
        stepper.start();
    }
    Label { text: qsTr("Automatic music ducking"); font.weight: Font.DemiBold }
    Label { Layout.fillWidth: true; wrapMode: Text.Wrap; color: Theme.textSecondary; text: qsTr("Detect peaks in the reference recording and replace this track's clip envelopes. Source effects are not part of detection.") }
    EchoComboBox { id: referenceBox; Layout.fillWidth: true; model: panel.references; textRole: "name"; enabled: !panel.running; onActivated: panel.cancel() }
    GridLayout {
        columns: 2
        Layout.fillWidth: true
        enabled: !panel.running
        Label { text: qsTr("Peak threshold (dBFS)"); Layout.fillWidth: true }
        SpinBox { id: threshold; from: -60; to: -6; value: -30; editable: true; onValueModified: panel.cancel() }
        Label { text: qsTr("Reduction (dB)") }
        SpinBox { id: amount; from: 1; to: 36; value: 12; editable: true; onValueModified: panel.cancel() }
        Label { text: qsTr("Anticipation (ms)") }
        SpinBox { id: attack; from: 10; to: 3000; value: 180; stepSize: 10; editable: true; onValueModified: panel.cancel() }
        Label { text: qsTr("Hold (ms)") }
        SpinBox { id: hold; from: 0; to: 5000; value: 250; stepSize: 10; editable: true; onValueModified: panel.cancel() }
        Label { text: qsTr("Recovery (ms)") }
        SpinBox { id: release; from: 10; to: 5000; value: 600; stepSize: 10; editable: true; onValueModified: panel.cancel() }
    }
    RowLayout {
        EchoButton { text: panel.running ? qsTr("Cancel") : qsTr("Generate envelopes"); enabled: panel.references.length > 0 && !panel.blocked; onClicked: panel.running ? panel.cancel() : panel.generate() }
        EchoButton {
            text: qsTr("Apply to track")
            enabled: panel.candidates.length > 0 && !panel.blocked && !panel.running
            onClicked: { const result = panel.candidates; panel.cancel(); panel.applyRequested(result); }
        }
    }
    ProgressBar { visible: panel.running; Layout.fillWidth: true; value: panel.snapshot ? panel.cursor / Math.max(1, panel.snapshot.reference.clips.length) : 0 }
    Label { Layout.fillWidth: true; visible: text.length > 0; text: panel.message; wrapMode: Text.Wrap; color: Theme.textSecondary }
    Timer {
        id: stepper
        interval: 1
        repeat: true
        onTriggered: {
            try {
                const job = panel.snapshot;
                if (panel.cursor < job.reference.clips.length) {
                    const clip = job.reference.clips[panel.cursor++];
                    if (clip.muted) return;
                    const asset = job.assets.find(a => a.id === clip.assetId);
                    const level = (panel.waveforms[clip.assetId] || [])[0];
                    const source = Editing.pinnedSource(clip, job.sources, job.assets);
                    if (!asset || !level || !level.mins || !level.mins.length || source === null) throw new Error("waveform");
                    const peaks = level.mins.map((value, i) => Math.max(Math.abs(value), Math.abs(level.maxs[i])));
                    const detected = Automation.activity(clip, Editing.sourceSpans(source, Number(asset.durationMillis)), peaks, Number(asset.durationMillis), job.threshold);
                    panel.intervals = panel.intervals.concat(detected);
                    if (panel.intervals.length > 16384) throw new Error("complexity");
                    return;
                }
                stop();
                if (!panel.intervals.length) { panel.message = qsTr("No activity was detected. Try a lower threshold."); return; }
                const regions = Automation.duckRegions(panel.intervals, job.attack, job.hold, job.release);
                panel.candidates = job.target.clips.map(clip => ({id: clip.id, envelope: Automation.duckEnvelope(clip, regions, job.amount)}));
                panel.message = qsTr("%1 activity regions; %2 clip envelopes ready. Apply creates one undo step.").arg(regions.length).arg(panel.candidates.length);
            } catch (error) {
                stop(); panel.candidates = [];
                panel.message = error.message === "complexity" ? qsTr("Too many activity changes. Increase hold time or use a shorter reference.") : qsTr("Reference waveforms are not ready. Wait for them to load and try again.");
            }
        }
    }
}

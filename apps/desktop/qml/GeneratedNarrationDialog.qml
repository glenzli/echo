//! Explicit authored speech: temporary candidate, audition, then material intake.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
Popup {
    id: dialog
    property string assemblyId: ""
    property string targetAssembly: ""
    property bool reviewed: false
    property string playbackError: ""
    readonly property bool privateProject: backend.independentEditing
    readonly property var receipt: generatedNarration.detailsJson ? JSON.parse(generatedNarration.detailsJson) : null
    readonly property bool busy: generatedNarration.running || generatedNarration.accepting
    signal materialAccepted(string assetId)
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(600,parent.width-32); height: Math.min(580,parent.height-32)
    padding: 24; modal: true; dim: true; focus: true
    closePolicy: generatedNarration.accepting ? Popup.NoAutoClose : Popup.CloseOnEscape
    background: Rectangle { radius: 12; color: Theme.panelRaised; border.color: Theme.border }
    function present() {
        if (busy) return;
        generatedNarration.discard(); reviewed=false; playbackError="";
        targetAssembly=assemblyId; collect.checked=!privateProject;
        open(); input.forceActiveFocus();
    }
    function modelName() {
        if (!receipt) return "";
        const job=receipt.runtime.job;
        const hub=/models--[^/]+--([^/]+)/.exec(job.physical_model || "");
        return hub ? hub[1] : (job.model_profile || job.physical_model || "").split("/").pop();
    }
    function stopPreview() { preview.stop(); preview.source=""; }
    onClosed: { stopPreview(); generatedNarration.discard(); }
    MediaPlayer {
        id: preview
        audioOutput: AudioOutput { volume: 0.75 }
        onPositionChanged: position => { if (position > 0) dialog.reviewed=true; }
        onErrorOccurred: dialog.playbackError=qsTr("This candidate could not be played. Generate it again.")
    }
    Connections {
        target: generatedNarration
        function onAccepted(assetId) {
            if (!dialog.visible) return;
            backend.refresh(); dialog.materialAccepted(assetId); dialog.close();
        }
    }
    contentItem: ColumnLayout {
        spacing: 14
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: qsTr("Add a narration"); color: Theme.textPrimary; font.pixelSize: 20; font.weight: Font.DemiBold }
            Text { text: qsTr("AI generated"); color: Theme.warningText; font.pixelSize: Theme.fontMeta; font.weight: Font.Medium }
        }
        Text { Layout.fillWidth: true; text: qsTr("Write a short narration to accompany your sound. Preview it before keeping it as a separate, clearly marked material."); color: Theme.textSecondary; font.pixelSize: Theme.fontBody; wrapMode: Text.WordWrap }
        ScrollView {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 110
            clip: true
            TextArea {
                id: input; objectName: "narrationText"
                readOnly: dialog.busy || !!dialog.receipt
                placeholderText: qsTr("What would you like to add?")
                color: Theme.textPrimary; placeholderTextColor: Theme.textMuted; selectionColor: Theme.accent
                font.pixelSize: Theme.fontBody; wrapMode: TextEdit.Wrap
                padding: 12; selectByMouse: true
                background: Rectangle { color: Theme.control; radius: 8; border.color: input.activeFocus ? Theme.accent : Theme.border }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: qsTr("%1 / 500 characters").arg(input.text.length); color: input.text.length>500 ? Theme.warningText : Theme.textMuted; font.pixelSize: Theme.fontMeta }
            Text { text: qsTr("Local model · Mandarin voice"); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
        }
        Rectangle {
            Layout.fillWidth: true; implicitHeight: reminder.implicitHeight+20; radius: 8; color: Theme.warningSurface
            Text { id: reminder; anchors.fill: parent; anchors.margins: 10; text: qsTr("This is an added narration, not recorded dialogue. Its generation label and model record stay with the accepted source."); color: Theme.warningText; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
        }
        RowLayout {
            visible: !!dialog.receipt; Layout.fillWidth: true
            EchoIconButton {
                objectName: "narrationPreview"
                source: preview.playbackState===MediaPlayer.PlayingState ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: qsTr("Preview narration"); enabled: !dialog.busy
                onClicked: {
                    if (preview.playbackState===MediaPlayer.PlayingState) { preview.pause(); return; }
                    if (player.playing) player.togglePause(); materialPlayer.stop();
                    if (preview.source.toString()!==generatedNarration.audioUrl.toString()) preview.source=generatedNarration.audioUrl;
                    dialog.playbackError=""; preview.play();
                }
            }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 3
                Text { text: dialog.receipt ? qsTr("%1 seconds · Preview candidate").arg((dialog.receipt.duration_millis/1000).toFixed(1)) : ""; color: Theme.textPrimary; font.pixelSize: Theme.fontBody }
                Text { Layout.fillWidth: true; text: dialog.modelName(); elide: Text.ElideMiddle; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            }
            EchoButton { text: qsTr("Revise text"); ghost: true; enabled: !dialog.busy; onClicked: { dialog.stopPreview(); generatedNarration.discard(); dialog.reviewed=false; dialog.playbackError=""; input.forceActiveFocus(); } }
        }
        CheckBox { id: collect; visible: !dialog.privateProject && !!dialog.targetAssembly; enabled: !dialog.busy; text: qsTr("Also collect in Materials") }
        Text { Layout.fillWidth: true; visible: !!generatedNarration.errorText || !!dialog.playbackError; text: generatedNarration.errorText || dialog.playbackError; color: Theme.warningText; font.pixelSize: Theme.fontBody; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            BusyIndicator { running: dialog.busy; visible: running; implicitWidth: 24; implicitHeight: 24 }
            Text { Layout.fillWidth: true; text: generatedNarration.accepting ? qsTr("Saving material…") : generatedNarration.running ? qsTr("Generating locally…") : dialog.receipt && !dialog.reviewed ? qsTr("Listen before keeping this candidate.") : ""; color: Theme.textMuted; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
            EchoButton { text: qsTr("Close"); ghost: true; enabled: !generatedNarration.accepting; onClicked: dialog.close() }
            EchoButton {
                objectName: "narrationGenerate"
                visible: !dialog.receipt; text: qsTr("Generate preview")
                enabled: !dialog.busy && input.text.trim().length>0 && input.text.length<=500
                onClicked: { dialog.reviewed=false; dialog.playbackError=""; generatedNarration.request(input.text,inferencePrefs.runtimeEndpoint); }
            }
            EchoButton {
                objectName: "narrationAccept"
                visible: !!dialog.receipt; text: qsTr("Keep as material")
                enabled: !dialog.busy && dialog.reviewed && !dialog.playbackError
                onClicked: { dialog.stopPreview(); generatedNarration.accept(dialog.targetAssembly,!dialog.privateProject && (!dialog.targetAssembly || collect.checked)); }
            }
        }
    }
}

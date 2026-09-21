//! Candidate review lifecycle shared by independent narration, sound effects and music sources.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
Popup {
    id: dialog
    required property var controller
    property bool soundMaterial: false
    property string controlPrefix: soundMaterial ? "soundMaterial" : "narration"
    property string assemblyId: ""
    property string targetAssembly: ""
    property bool reviewed: false
    property var heardCandidates: ({})
    readonly property string selectedCandidateId: dialog.controller.selectedCandidateId
    property string playbackError: ""
    readonly property var soundModels: [
        {choice:"stable_audio_3_small_sfx",title:qsTr("Sound effects · Small-SFX")},
        {choice:"stable_audio_3_small_music",title:qsTr("Music · Small-Music")}
    ]
    readonly property string modelChoice: soundModels[modelPicker.currentIndex].choice
    readonly property bool musicMode: soundMaterial && modelChoice==="stable_audio_3_small_music"
    // QML string iteration counts UTF-16 units; the backend counts Unicode scalars.
    readonly property int characterCount: input.text.trim().replace(/[\uD800-\uDBFF][\uDC00-\uDFFF]/g,"_").length
    readonly property bool privateProject: backend.independentEditing
    readonly property var receipt: dialog.controller.detailsJson ? JSON.parse(dialog.controller.detailsJson) : null
    readonly property bool busy: dialog.controller.running || dialog.controller.accepting
    signal materialAccepted(string assetId)
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(720,parent.width-32); height: Math.min(soundMaterial ? (dialog.controller.candidates.length ? 760 : 520) : 660,parent.height-32)
    padding: 24; modal: true; dim: true; focus: true
    closePolicy: dialog.controller.accepting ? Popup.NoAutoClose : Popup.CloseOnEscape
    background: Rectangle { radius: 12; color: Theme.panelRaised; border.color: Theme.border }
    function present(choice) {
        if (busy) return;
        dialog.controller.discard(); heardCandidates=({}); reviewed=false; playbackError="";
        if (soundMaterial) modelPicker.currentIndex=choice==="stable_audio_3_small_music" ? 1 : 0;
        targetAssembly=assemblyId; collect.checked=false;
        open(); input.forceActiveFocus();
    }
    function modelName() {
        if (!receipt) return "";
        const job=receipt.runtime.job;
        if (job.model_profile==="stable_audio_3_sm_sfx") return "Stable Audio 3 · Small-SFX";
        if (job.model_profile==="stable_audio_3_sm_music") return "Stable Audio 3 · Small-Music";
        if (job.model_profile==="stable_audio_open_small") return "Stable Audio Open Small";
        const hub=/models--[^/]+--([^/]+)/.exec(job.physical_model || "");
        return hub ? hub[1] : (job.model_profile || job.physical_model || "").split("/").pop();
    }
    function stopPreview() { preview.stop(); preview.source=""; }
    onSelectedCandidateIdChanged: {
        stopPreview(); reviewed=heardCandidates[selectedCandidateId]===true; playbackError="";
        const id=selectedCandidateId;
        Qt.callLater(() => {
            if (!id || id!==dialog.selectedCandidateId || !dialog.receipt) return;
            input.text=dialog.receipt.input_text || input.text;
            if (dialog.soundMaterial) {
                modelPicker.currentIndex=dialog.receipt.request.model_choice==="stable_audio_3_small_music" ? 1 : 0;
                duration.value=dialog.receipt.request.duration_seconds;
                ambience.checked=dialog.receipt.material_category==="ambience";
            }
        });
    }
    onClosed: { stopPreview(); dialog.controller.discard(); }
    MediaPlayer {
        id: preview
        audioOutput: AudioOutput { volume: 0.75 }
        onPositionChanged: position => {
            if (position>0 && dialog.selectedCandidateId && preview.source.toString()===dialog.controller.audioUrl.toString()) {
                dialog.heardCandidates[dialog.selectedCandidateId]=true;
                dialog.reviewed=true;
            }
        }
        onErrorOccurred: dialog.playbackError=qsTr("This candidate could not be played. Generate it again.")
    }
    Connections {
        target: dialog.controller
        function onAccepted(assetId) {
            if (!dialog.visible) return;
            backend.refresh(); dialog.materialAccepted(assetId); dialog.close();
        }
    }
    contentItem: ColumnLayout {
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: dialog.soundMaterial ? qsTr("Generate a sound") : qsTr("Add a narration"); color: Theme.textPrimary; font.pixelSize: 20; font.weight: Font.DemiBold }
            Text { text: qsTr("AI generated"); color: Theme.warningText; font.pixelSize: Theme.fontMeta; font.weight: Font.Medium }
        }
        Text { Layout.fillWidth: true; text: dialog.soundMaterial ? qsTr("Describe a sound effect or short music background. Listen to each candidate, then keep the one that fits.") : qsTr("Write a short narration to accompany your sound. Preview it before keeping it as a separate, clearly marked material."); color: Theme.textSecondary; font.pixelSize: Theme.fontBody; wrapMode: Text.WordWrap }
        EchoComboBox {
            id:modelPicker; objectName:"soundMaterialModel"; visible:dialog.soundMaterial; Layout.fillWidth:true
            textRole:"title"; model:dialog.soundModels; enabled:!dialog.busy
            Accessible.name:qsTr("Generation model")
            onCurrentIndexChanged: preset.currentIndex=0
        }
        RowLayout {
            visible: dialog.soundMaterial; Layout.fillWidth: true
            EchoComboBox {
                id: preset; objectName: "soundMaterialPreset"; Layout.fillWidth: true; enabled: !dialog.busy
                textRole: "title"
                model: dialog.musicMode ? [
                    {title:qsTr("Choose a starting point…"),prompt:"",ambience:false},
                    {title:qsTr("Soft piano background"),prompt:"Soft sparse piano notes, gentle reflective instrumental background, warm intimate room, no vocals or percussion.",ambience:false},
                    {title:qsTr("Ambient synth pad"),prompt:"A warm slowly evolving ambient synthesizer pad, calm spacious instrumental texture, no vocals or drums.",ambience:false},
                    {title:qsTr("Light acoustic guitar"),prompt:"Gentle fingerpicked acoustic guitar, simple warm instrumental background, relaxed tempo, no vocals or percussion.",ambience:false}
                ] : [
                    {title:qsTr("Choose a starting point…"),prompt:"",ambience:false},
                    {title:qsTr("Rain outside a window"),prompt:"Gentle rain outside a closed window, soft steady patter, distant outdoor ambience, no speech or music.",ambience:true},
                    {title:qsTr("Quiet room"),prompt:"A quiet room with soft air ventilation, subtle steady room tone, no speech or music.",ambience:true},
                    {title:qsTr("Footsteps on gravel"),prompt:"Slow footsteps on a gravel path, distinct close crunches, natural outdoor sound, no speech or music.",ambience:false},
                    {title:qsTr("A wooden door closing"),prompt:"A wooden door slowly closes with a soft creak and a single gentle latch click, no speech or music.",ambience:false},
                    {title:qsTr("Distant traffic"),prompt:"Distant road traffic heard from a quiet park, occasional cars passing, soft continuous background, no speech or music.",ambience:true}
                ]
                onActivated: if (currentIndex>0) { input.text=model[currentIndex].prompt; ambience.checked=model[currentIndex].ambience; }
            }
            Text { text:qsTr("Seconds"); color:Theme.textSecondary; font.pixelSize:Theme.fontBody }
            EchoValueSpinBox { id:duration; objectName:"soundMaterialDuration"; from:1; to:30; value:10; enabled:!dialog.busy; Accessible.name:qsTr("Duration in seconds") }
        }
        ScrollView {
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 100
            clip: true
            TextArea {
                id: input; objectName: dialog.soundMaterial ? "soundMaterialPrompt" : "narrationText"
                readOnly: dialog.busy
                placeholderText: dialog.soundMaterial ? qsTr("Describe the sound in English, including its setting and distance…") : qsTr("What would you like to add?")
                color: Theme.textPrimary; placeholderTextColor: Theme.textMuted; selectionColor: Theme.accent
                font.pixelSize: Theme.fontBody; wrapMode: TextEdit.Wrap
                padding: 12; selectByMouse: true
                background: Rectangle { color: Theme.control; radius: 8; border.color: input.activeFocus ? Theme.accent : Theme.border }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Text { Layout.fillWidth: true; text: qsTr("%1 / 500 characters").arg(dialog.characterCount); color: dialog.characterCount>500 ? Theme.warningText : Theme.textMuted; font.pixelSize: Theme.fontMeta }
            Text { text: dialog.soundMaterial ? qsTr("Local sound model · English prompts") : qsTr("Local model · Mandarin voice"); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
        }
        Text {
            Layout.fillWidth: true; visible: !!dialog.receipt && (input.text.trim()!==dialog.receipt.input_text || (dialog.soundMaterial && (dialog.modelChoice!==(dialog.receipt.request.model_choice || "stable_audio_3_small_sfx") || duration.value!==dialog.receipt.request.duration_seconds || (!dialog.musicMode && ambience.checked!==(dialog.receipt.material_category==="ambience")))))
            text: dialog.soundMaterial ? qsTr("Changes apply to the next candidate. Keeping uses the selected audio and its settings.") : qsTr("Text changes apply to the next candidate. Keeping uses the selected audio.")
            wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta
        }
        EchoCheckBox { id:ambience; objectName:"soundMaterialAmbience"; visible:dialog.soundMaterial && !dialog.musicMode; enabled:!dialog.busy; text:qsTr("Use as background ambience") }
        Rectangle {
            visible: !dialog.soundMaterial
            Layout.fillWidth: true; implicitHeight: reminder.implicitHeight+20; radius: 8; color: Theme.warningSurface
            Text { id: reminder; anchors.fill: parent; anchors.margins: 10; text: qsTr("This is an added narration, not recorded dialogue. Its generation label and model record stay with the accepted source."); color: Theme.warningText; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
        }
        RowLayout {
            visible: dialog.controller.candidates.length>0; Layout.fillWidth: true; spacing: 8
            Repeater {
                model: dialog.controller.candidates
                EchoButton {
                    required property var modelData
                    required property int index
                    objectName: dialog.controlPrefix+"Candidate"+index
                    Layout.fillWidth: true; ghost: true
                    selected: modelData.id===dialog.selectedCandidateId
                    text: qsTr("Candidate %1").arg(index+1)
                    enabled: !dialog.busy
                    onClicked: dialog.controller.selectCandidate(modelData.id)
                    ToolTip.visible: hovered; ToolTip.text: modelData.text; ToolTip.delay: 500
                }
            }
            Text { text: qsTr("%1 / 3").arg(dialog.controller.candidates.length); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
        }
        RowLayout {
            visible: !!dialog.receipt; Layout.fillWidth: true
            EchoIconButton {
                objectName: dialog.controlPrefix+"Preview"
                source: preview.playbackState===MediaPlayer.PlayingState ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                toolTipText: dialog.soundMaterial ? qsTr("Preview sound") : qsTr("Preview narration"); enabled: !dialog.busy
                onClicked: {
                    if (preview.playbackState===MediaPlayer.PlayingState) { preview.pause(); return; }
                    if (player.playing) player.togglePause(); materialPlayer.stop();
                    if (preview.source.toString()!==dialog.controller.audioUrl.toString()) preview.source=dialog.controller.audioUrl;
                    dialog.playbackError=""; preview.play();
                }
            }
            ColumnLayout {
                Layout.fillWidth: true; spacing: 3
                Text { text: dialog.receipt ? qsTr("%1 seconds · Preview candidate").arg((dialog.receipt.duration_millis/1000).toFixed(1)) : ""; color: Theme.textPrimary; font.pixelSize: Theme.fontBody }
                Text { Layout.fillWidth: true; text: dialog.modelName() + (dialog.soundMaterial && dialog.receipt ? " · " + qsTr("Seed %1").arg(String(dialog.receipt.request.seed)) : ""); elide: Text.ElideMiddle; color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
            }
            EchoButton { objectName: dialog.controlPrefix+"Remove"; text: qsTr("Remove candidate"); ghost: true; enabled: !dialog.busy; onClicked: { dialog.stopPreview(); dialog.controller.removeSelected(); input.forceActiveFocus(); } }
        }
        Text {
            Layout.fillWidth: true; wrapMode: Text.WordWrap
            text: dialog.privateProject || dialog.targetAssembly ? qsTr("Kept with this project. Add it to a track when you are ready.") : qsTr("Kept in Materials. You can add it to a project later.")
            color: Theme.textSecondary; font.pixelSize: Theme.fontMeta
        }
        EchoCheckBox { id: collect; objectName: dialog.controlPrefix+"CollectGlobally"; visible: !dialog.privateProject && !!dialog.targetAssembly; enabled: !dialog.busy; text: qsTr("Also collect in Materials") }
        Text { Layout.fillWidth: true; visible: !!dialog.controller.errorText || !!dialog.playbackError; text: dialog.controller.errorText || dialog.playbackError; color: Theme.warningText; font.pixelSize: Theme.fontBody; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            BusyIndicator { running: dialog.busy; visible: running; implicitWidth: 24; implicitHeight: 24 }
            Text { Layout.fillWidth: true; text: dialog.controller.accepting ? qsTr("Saving material…") : dialog.controller.running ? qsTr("Generating locally…") : dialog.receipt && !dialog.reviewed ? qsTr("Listen before keeping this candidate.") : ""; color: Theme.textMuted; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
            EchoButton { text: qsTr("Close"); ghost: true; enabled: !dialog.controller.accepting; onClicked: dialog.close() }
            EchoButton {
                objectName: dialog.controlPrefix+"Generate"
                text: dialog.controller.candidates.length ? qsTr("Generate another") : qsTr("Generate preview")
                enabled: !dialog.busy && dialog.characterCount>0 && dialog.characterCount<=500 && dialog.controller.candidates.length<3
                ghost: !!dialog.receipt
                onClicked: { dialog.stopPreview(); dialog.playbackError=""; if (dialog.soundMaterial) dialog.controller.requestSoundMaterial(input.text,duration.value,!dialog.musicMode && ambience.checked,inferencePrefs.runtimeEndpoint,dialog.modelChoice); else dialog.controller.request(input.text,inferencePrefs.runtimeEndpoint); }
            }
            EchoButton {
                objectName: dialog.controlPrefix+"Accept"
                visible: !!dialog.receipt; text: dialog.privateProject || dialog.targetAssembly ? qsTr("Keep in project") : qsTr("Keep in Materials")
                enabled: !dialog.busy && dialog.reviewed && !dialog.playbackError
                onClicked: { dialog.stopPreview(); dialog.controller.accept(dialog.targetAssembly,!dialog.privateProject && (!dialog.targetAssembly || collect.checked)); }
            }
        }
    }
}

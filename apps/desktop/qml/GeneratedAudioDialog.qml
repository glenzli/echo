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
    property var soundDrafts: ({})
    property string draftModelChoice: ""
    readonly property bool durationValid: !soundMaterial || duration.contentItem.acceptableInput
    readonly property bool hasCandidates: controller.candidates.length>0
    readonly property var soundModels: [
        {choice:"stable_audio_3_small_sfx",title:qsTr("Sound effects and ambience")},
        {choice:"stable_audio_3_small_music",title:qsTr("Short background music")}
    ]
    readonly property string modelChoice: soundModels[Math.max(0,modelPicker.currentIndex)].choice
    readonly property bool musicMode: soundMaterial && modelChoice==="stable_audio_3_small_music"
    // QML string iteration counts UTF-16 units; the backend counts Unicode scalars.
    readonly property int characterCount: input.text.trim().replace(/[\uD800-\uDBFF][\uDC00-\uDFFF]/g,"_").length
    readonly property bool privateProject: backend.independentEditing
    readonly property var receipt: dialog.controller.detailsJson ? JSON.parse(dialog.controller.detailsJson) : null
    readonly property bool busy: dialog.controller.running || dialog.controller.accepting
    signal materialAccepted(string assetId)
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(720,parent.width-32); height: Math.min(soundMaterial ? 640 : 560,parent.height-32)
    padding: 24; modal: true; dim: true; focus: true
    closePolicy: dialog.controller.accepting ? Popup.NoAutoClose : Popup.CloseOnEscape
    background: Rectangle { radius: 12; color: Theme.panelRaised; border.color: Theme.border }
    Component.onCompleted: draftModelChoice=modelChoice
    // Qt's editable SpinBox may still hold uncommitted text when a macOS button
    // is clicked. Capture the displayed value before taking a request snapshot.
    function commitDuration() {
        if (!durationValid) { duration.contentItem.forceActiveFocus(); return false; }
        duration.value=duration.valueFromText(duration.contentItem.text,duration.locale);
        duration.contentItem.text=duration.displayText;
        return true;
    }
    function switchDraft(choice) {
        if (!soundMaterial || choice===draftModelChoice) return;
        if (durationValid) commitDuration();
        soundDrafts[draftModelChoice]={text:input.text,seconds:duration.value,ambience:ambience.checked};
        draftModelChoice=choice;
        const draft=soundDrafts[choice] || {text:"",seconds:10,ambience:false};
        input.text=draft.text; duration.value=draft.seconds; duration.contentItem.text=duration.displayText;
        ambience.checked=draft.ambience; preset.currentIndex=0;
    }
    function timeLabel(millis) {
        const seconds=Math.max(0,Math.floor(millis/1000));
        return Math.floor(seconds/60)+":"+(seconds%60).toString().padStart(2,"0");
    }
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
            if (dialog.soundMaterial) {
                modelPicker.currentIndex=dialog.receipt.request.model_choice==="stable_audio_3_small_music" ? 1 : 0;
                duration.value=dialog.receipt.request.duration_seconds;
                duration.contentItem.text=duration.displayText;
                ambience.checked=dialog.receipt.material_category==="ambience";
            }
            input.text=dialog.receipt.input_text || "";
        });
    }
    onClosed: { stopPreview(); dialog.controller.discard(); }
    MediaPlayer {
        id: preview; objectName: dialog.controlPrefix+"Player"
        audioOutput: AudioOutput { volume: 0.75 }
        onErrorOccurred: dialog.playbackError=qsTr("This candidate could not be played. Generate it again.")
    }
    Timer {
        interval: 150; repeat: true; running: preview.playbackState===MediaPlayer.PlayingState && !dialog.reviewed
        onTriggered: {
            // Seeking or loading alone must never count as listening.
            if (preview.position<=0 || !dialog.selectedCandidateId || preview.source.toString()!==dialog.controller.audioUrl.toString()) return;
            const heard=Object.assign({},dialog.heardCandidates);
            heard[dialog.selectedCandidateId]=true;
            dialog.heardCandidates=heard; dialog.reviewed=true;
        }
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
            Text { Layout.fillWidth: true; text: dialog.soundMaterial ? (dialog.musicMode ? qsTr("Create background music") : qsTr("Create sound effects")) : qsTr("Add a narration"); color: Theme.textPrimary; font.pixelSize: 20; font.weight: Font.DemiBold }
            Text { text: qsTr("AI generated"); color: Theme.warningText; font.pixelSize: Theme.fontMeta; font.weight: Font.Medium }
        }
        Text { Layout.fillWidth: true; text: dialog.soundMaterial ? qsTr("Describe a sound effect or short music background. Listen to each candidate, then keep the one that fits.") : qsTr("Write a short narration to accompany your sound. Preview it before keeping it as a separate, clearly marked material."); color: Theme.textSecondary; font.pixelSize: Theme.fontBody; wrapMode: Text.WordWrap }
        ScrollView {
            id: body; objectName: dialog.controlPrefix+"Body"
            Layout.fillWidth: true; Layout.fillHeight: true; Layout.minimumHeight: 0; Layout.preferredHeight: 1; clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                width: body.availableWidth; spacing: 12
                EchoComboBox {
                    id:modelPicker; objectName:"soundMaterialModel"; visible:dialog.soundMaterial; Layout.fillWidth:true
                    textRole:"title"; model:dialog.soundModels; enabled:!dialog.busy
                    Accessible.name:qsTr("Sound type")
                    onCurrentIndexChanged: if (dialog.draftModelChoice) dialog.switchDraft(dialog.soundModels[Math.max(0,currentIndex)].choice)
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
                    EchoValueSpinBox { id:duration; objectName:"soundMaterialDuration"; from:1; to:30; value:10; enabled:!dialog.busy; Accessible.name:qsTr("Duration in seconds"); onValueChanged: if (contentItem) contentItem.text=displayText }
                }
                ScrollView {
                    Layout.fillWidth: true; Layout.preferredHeight: 120; Layout.minimumHeight: 120
                    clip: true
                    TextArea {
                        id: input; objectName: dialog.soundMaterial ? "soundMaterialPrompt" : "narrationText"
                        readOnly: dialog.busy
                        placeholderText: dialog.soundMaterial ? (dialog.musicMode ? qsTr("Describe the instruments, mood and pace in English…") : qsTr("Describe the sound in English, including its setting and distance…")) : qsTr("What would you like to add?")
                        onTextChanged: if (preset && preset.currentIndex>0 && text!==preset.model[preset.currentIndex].prompt) preset.currentIndex=0
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
                Rectangle {
                    Layout.fillWidth: true; implicitHeight: candidatesColumn.implicitHeight+24
                    color: Theme.controlQuiet; radius: 8; border.color: Theme.border
                    ColumnLayout {
                        id: candidatesColumn; anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top; anchors.margins: 12; spacing: 8
                        Text {
                            Layout.fillWidth: true; visible: !dialog.hasCandidates
                            text: qsTr("Your previews will appear here")
                            color: Theme.textPrimary; font.pixelSize: Theme.fontBody; font.weight: Font.Medium
                        }
                        Text {
                            Layout.fillWidth: true; visible: !dialog.hasCandidates; wrapMode: Text.WordWrap
                            text: qsTr("Generate up to three candidates, compare them, then keep one.")
                            color: Theme.textSecondary; font.pixelSize: Theme.fontMeta
                        }
                        RowLayout {
                            visible: dialog.hasCandidates; Layout.fillWidth: true; spacing: 8
                            Repeater {
                                model: dialog.controller.candidates
                                EchoButton {
                                    required property var modelData
                                    required property int index
                                    objectName: dialog.controlPrefix+"Candidate"+index
                                    Layout.fillWidth: true; ghost: true
                                    selected: modelData.id===dialog.selectedCandidateId
                                    text: dialog.heardCandidates[modelData.id] ? qsTr("Candidate %1 · Heard").arg(index+1) : qsTr("Candidate %1").arg(index+1)
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
                                toolTipText: preview.playbackState===MediaPlayer.PlayingState ? qsTr("Pause preview") : qsTr("Play preview"); enabled: !dialog.busy
                                onClicked: {
                                    if (preview.playbackState===MediaPlayer.PlayingState) { preview.pause(); return; }
                                    if (player.playing) player.togglePause(); materialPlayer.stop();
                                    if (preview.source.toString()!==dialog.controller.audioUrl.toString()) preview.source=dialog.controller.audioUrl;
                                    dialog.playbackError=""; preview.play();
                                }
                            }
                            EchoParameterSlider {
                                objectName: dialog.controlPrefix+"Progress"; Layout.fillWidth: true
                                from: 0; to: Math.max(1,preview.duration); value: preview.position
                                maximumTrackWidth: 600; valueWidth: 86; fillFromMinimum: true
                                valueText: dialog.timeLabel(preview.position)+" / "+dialog.timeLabel(dialog.receipt ? dialog.receipt.duration_millis : 0)
                                accessibleName: qsTr("Preview position"); enabled: !dialog.busy && preview.seekable
                                onEdited: value => preview.setPosition(value)
                            }
                            EchoButton { objectName: dialog.controlPrefix+"Remove"; text: qsTr("Remove candidate"); ghost: true; enabled: !dialog.busy; onClicked: { dialog.stopPreview(); dialog.controller.removeSelected(); input.forceActiveFocus(); } }
                        }
                        Text {
                            Layout.fillWidth: true; visible: !!dialog.receipt
                            text: dialog.modelName(); elide: Text.ElideMiddle; color: Theme.textMuted; font.pixelSize: Theme.fontMeta
                            ToolTip.visible: modelHover.hovered && dialog.soundMaterial; ToolTip.delay: 500
                            ToolTip.text: dialog.receipt && dialog.soundMaterial ? qsTr("Seed %1").arg(String(dialog.receipt.request.seed)) : ""
                            HoverHandler { id: modelHover }
                        }
                        Text {
                            Layout.fillWidth: true; visible: dialog.hasCandidates; wrapMode: Text.WordWrap
                            text: qsTr("Closing discards previews you have not kept.")
                            color: Theme.textMuted; font.pixelSize: Theme.fontMeta
                        }
                    }
                }
                Text {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap
                    text: dialog.privateProject || dialog.targetAssembly ? qsTr("Kept with this project. Add it to a track when you are ready.") : qsTr("Kept in Materials. You can add it to a project later.")
                    color: Theme.textSecondary; font.pixelSize: Theme.fontMeta
                }
                EchoCheckBox { id: collect; objectName: dialog.controlPrefix+"CollectGlobally"; visible: !dialog.privateProject && !!dialog.targetAssembly; enabled: !dialog.busy; text: qsTr("Also collect in Materials") }
            }
        }
        Text { Layout.fillWidth: true; visible: !!dialog.controller.errorText || !!dialog.playbackError; text: dialog.controller.errorText || dialog.playbackError; color: Theme.warningText; font.pixelSize: Theme.fontBody; wrapMode: Text.WordWrap }
        RowLayout {
            Layout.fillWidth: true
            BusyIndicator { running: dialog.busy; visible: running; implicitWidth: 24; implicitHeight: 24 }
            Text { Layout.fillWidth: true; text: dialog.controller.accepting ? qsTr("Saving material…") : dialog.controller.running ? qsTr("Generating locally…") : dialog.controller.candidates.length>=3 ? qsTr("Three previews ready. Remove one to generate another.") : dialog.receipt && !dialog.reviewed ? qsTr("Listen before keeping this candidate.") : ""; color: Theme.textMuted; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
        }
        RowLayout {
            Layout.fillWidth: true
            EchoButton {
                objectName: dialog.controlPrefix+"Close"
                text: dialog.hasCandidates || dialog.controller.running ? qsTr("Discard and close") : qsTr("Close")
                ghost: true; enabled: !dialog.controller.accepting; onClicked: dialog.close()
                ToolTip.visible: hovered && dialog.controller.running; ToolTip.delay: 500
                ToolTip.text: qsTr("The current local task may finish, but its result will be discarded.")
            }
            Item { Layout.fillWidth: true }
            EchoButton {
                objectName: dialog.controlPrefix+"Generate"
                text: dialog.controller.candidates.length ? qsTr("Generate another") : qsTr("Generate preview")
                enabled: !dialog.busy && dialog.durationValid && dialog.characterCount>0 && dialog.characterCount<=500 && dialog.controller.candidates.length<3
                ghost: !!dialog.receipt
                onClicked: { if (dialog.soundMaterial && !dialog.commitDuration()) return; dialog.stopPreview(); dialog.playbackError=""; if (dialog.soundMaterial) dialog.controller.requestSoundMaterial(input.text,duration.value,!dialog.musicMode && ambience.checked,inferencePrefs.runtimeEndpoint,dialog.modelChoice); else dialog.controller.request(input.text,inferencePrefs.runtimeEndpoint); }
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

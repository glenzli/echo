//! Arrangement parameter presentation. The workspace retains mutation and undo ownership.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "SoundAssemblyEditing.js" as Editing

Rectangle {
    id: inspector
    required property var workspace
    required property var renderController
    required property var waveforms
    readonly property var clipData: workspace.selectedClip
    readonly property var master: workspace.hasDocument ? workspace.document.master : ({})
    readonly property color trackColor: Theme.assemblyTrackColors[Math.max(0,workspace.selectedTrackIndex) % 8]
    property alias ducking: duckingPanel
    color: Theme.panelRaised
    implicitWidth: 280
    enabled: workspace.hasDocument && !renderController.running
    Rectangle { width: 1; height: parent.height; color: Theme.border }

    component Caption: Text {
        color: Theme.textSecondary
        font.pixelSize: Theme.fontSection
        verticalAlignment: Text.AlignVCenter
        Layout.fillWidth: true
        elide: Text.ElideRight
    }
    component Section: ColumnLayout {
        id: section
        required property string title
        default property alias fields: body.data
        spacing: 7
        Layout.fillWidth: true
        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.border }
        Text { Layout.topMargin: 3; text: section.title; color: Theme.textSecondary; font.pixelSize: Theme.fontSection; font.weight: Font.DemiBold }
        ColumnLayout { id: body; Layout.fillWidth: true; spacing: 6 }
    }
    component SmallButton: EchoButton {
        Layout.fillWidth: true
        Layout.minimumWidth: 0
        implicitHeight: 26
        verticalPadding: 4
        font.pixelSize: Theme.fontSection
        ghost: true
    }
    component Choice: EchoComboBox {
        Layout.preferredWidth: 128
        implicitHeight: 26
        font.pixelSize: Theme.fontSection
    }
    component MixSlider: EchoParameterSlider {
        Layout.fillWidth: true
        Layout.preferredHeight: 24
        labelWidth: 38
        valueWidth: 60
        showNeutralMarker: true
        neutralValue: 0
        property bool dragging: false
        signal committed(real result)
        onGestureStarted: dragging=true
        onGestureFinished: { dragging=false; committed(Math.round(value)); }
        onEdited: value => { if (!dragging) committed(Math.round(value)); }
    }

    ScrollView {
        id: scroll
        objectName: "assemblyInspectorScroll"
        anchors.fill: parent
        anchors.margins: 12
        clip: true
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout {
            width: scroll.availableWidth
            spacing: 12
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 6
                Text {
                    text: inspector.clipData ? qsTr("Clip inspector") : qsTr("Master output")
                    font.pixelSize: Theme.fontBody; font.weight: Font.DemiBold; color: Theme.textPrimary
                }
                RowLayout {
                    visible: inspector.clipData !== null
                    Layout.fillWidth: true
                    spacing: 7
                    Rectangle { width: 3; height: 26; radius: 1; color: inspector.trackColor }
                    Text { text: String(workspace.selectedTrackIndex+1).padStart(2,"0"); font.pixelSize: Theme.fontMeta; font.weight: Font.DemiBold; color: inspector.trackColor }
                    Text {
                        objectName: "inspectorSourceTitle"
                        Layout.fillWidth: true
                        text: workspace.sourceName(inspector.clipData)
                        font.pixelSize: Theme.fontSection; font.weight: Font.DemiBold; color: Theme.textPrimary
                        elide: Text.ElideMiddle
                    }
                }
                Text {
                    visible: inspector.clipData !== null
                    Layout.fillWidth: true
                    text: inspector.clipData ? (workspace.independentMode ? qsTr("Project source") : inspector.clipData.sourceRole === "material" ? qsTr("Material reference") : qsTr("Memory reference")) + " · " + (inspector.clipData.adjustmentRevisionId > 0 ? qsTr("Source version %1").arg(inspector.clipData.adjustmentRevisionId) : qsTr("Original source")) : ""
                    font.pixelSize: Theme.fontMeta; color: Theme.textMuted; wrapMode: Text.WordWrap
                }
                Text {
                    visible: workspace.selectionCount > 1
                    Layout.fillWidth: true
                    text: qsTr("%1 clips selected · Parameters below apply to the active clip.").arg(workspace.selectionCount || 1)
                    font.pixelSize: Theme.fontMeta; color: Theme.accent; wrapMode: Text.WordWrap
                }
            }
            ColumnLayout {
                visible: inspector.clipData !== null
                Layout.fillWidth: true
                spacing: 12
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 6
                    SmallButton { objectName: "editClipSourceButton"; text: qsTr("Edit this clip’s sound"); onClicked: workspace.openClipEditor() }
                    Choice {
                        visible: !workspace.independentMode
                        Layout.fillWidth: true
                        model: [qsTr("Memory reference"),qsTr("Material reference")]
                        selectionIndex: inspector.clipData && inspector.clipData.sourceRole === "material" ? 1 : 0
                        onActivated: workspace.setClipValue("sourceRole",currentIndex===1 ? "material" : "memory")
                    }
                    RowLayout {
                        Layout.fillWidth: true; spacing: 6
                        SmallButton { objectName: "inspectorSplit"; text: qsTr("Split"); onClicked: workspace.splitSelectedClip() }
                        SmallButton { objectName: "inspectorDuplicate"; text: qsTr("Duplicate"); onClicked: workspace.duplicateSelectedClip() }
                        EchoIconButton {
                            buttonSize: 26; iconSize: 16; source: "qrc:/EchoDesktop/icons/more-horizontal.svg"; toolTipText: qsTr("Clip actions")
                            onClicked: clipMenu.popup()
                            Menu {
                                id: clipMenu
                                MenuItem { text: qsTr("Track ↑"); enabled: workspace.selectedTrackIndex>0; onTriggered: workspace.moveSelectedClipToTrack(-1) }
                                MenuItem { text: qsTr("Track ↓"); enabled: workspace.selectedTrackIndex>=0 && workspace.selectedTrackIndex<workspace.tracks.length-1; onTriggered: workspace.moveSelectedClipToTrack(1) }
                                MenuSeparator {}
                                MenuItem { text: qsTr("Delete"); enabled: workspace.totalClipCount()>1; onTriggered: workspace.deleteSelectedClip() }
                            }
                        }
                    }
                }
                Section {
                    title: qsTr("Timing")
                    Repeater {
                        model: [{key:"timelineStartMillis",label:qsTr("Position (s)")},{key:"sourceStartMillis",label:qsTr("Source in (s)")},{key:"sourceEndMillis",label:qsTr("Source out (s)")}]
                        delegate: RowLayout {
                            required property var modelData
                            Layout.fillWidth: true; spacing: 8
                            Caption { text: modelData.label }
                            EchoTimeSpinBox {
                                objectName: "inspector-"+modelData.key
                                Layout.preferredWidth: 128
                                from: modelData.key==="sourceEndMillis" && inspector.clipData ? inspector.clipData.sourceStartMillis+10 : 0
                                to: !inspector.clipData ? 0 : modelData.key==="timelineStartMillis" ? 14400000-Editing.duration(inspector.clipData) : modelData.key==="sourceStartMillis" ? inspector.clipData.sourceEndMillis-10 : workspace.sourceDurationFor(inspector.clipData)
                                value: inspector.clipData ? inspector.clipData[modelData.key] : 0
                                onValueModified: workspace.setClipTiming(modelData.key,value)
                                Accessible.name: modelData.label
                            }
                        }
                    }
                }
                Section {
                    title: qsTr("Mix")
                    MixSlider { label: qsTr("Gain"); accessibleName: qsTr("Clip gain"); from: -2400; to: 1200; stepSize: 50; value: inspector.clipData ? inspector.clipData.gainCentibels : 0; valueText: (value/100).toFixed(1)+" dB"; onCommitted: value => workspace.setClipValue("gainCentibels",value) }
                    MixSlider { label: qsTr("Pan"); accessibleName: qsTr("Clip pan"); from: -100; to: 100; stepSize: 1; value: inspector.clipData ? inspector.clipData.panPercent : 0; valueText: value===0 ? qsTr("Center") : (value<0 ? "L " : "R ")+Math.abs(value); onCommitted: value => workspace.setClipValue("panPercent",value) }
                    RowLayout {
                        Layout.fillWidth: true
                        Caption { text: qsTr("Muted") }
                        EchoSwitch { accessibleName: qsTr("Clip muted"); checked: inspector.clipData !== null && inspector.clipData.muted; onToggled: workspace.setClipValue("muted",checked) }
                    }
                }
                Section {
                    title: qsTr("Fades")
                    Repeater {
                        model: [{time:"fadeInMillis",curve:"fadeInCurve",other:"fadeOutMillis",label:qsTr("Fade in (ms)"),curveLabel:qsTr("Fade-in curve")},{time:"fadeOutMillis",curve:"fadeOutCurve",other:"fadeInMillis",label:qsTr("Fade out (ms)"),curveLabel:qsTr("Fade-out curve")}]
                        delegate: ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true; spacing: 5
                            RowLayout {
                                Layout.fillWidth: true; spacing: 8
                                Caption { text: modelData.label }
                                EchoValueSpinBox {
                                    objectName: "inspector-"+modelData.time
                                    Layout.preferredWidth: 128
                                    from: 0; to: inspector.clipData ? Editing.duration(inspector.clipData)-inspector.clipData[modelData.other] : 0
                                    value: inspector.clipData ? inspector.clipData[modelData.time] : 0
                                    onValueModified: workspace.setClipValue(modelData.time,value)
                                    Accessible.name: modelData.label
                                }
                            }
                            RowLayout {
                                Layout.fillWidth: true; spacing: 8
                                Caption { text: qsTr("Curve"); color: Theme.textMuted }
                                Choice {
                                    objectName: modelData.curve==="fadeInCurve" ? "assemblyFadeInCurve" : "assemblyFadeOutCurve"
                                    model: [qsTr("Linear"),qsTr("Smooth"),qsTr("Equal power")]
                                    selectionIndex: inspector.clipData ? workspace.fadeCurveIndex(inspector.clipData[modelData.curve]) : 0
                                    onActivated: workspace.setClipValue(modelData.curve,workspace.fadeCurveValue(currentIndex))
                                    Accessible.name: modelData.curveLabel
                                }
                            }
                        }
                    }
                }
                Section {
                    title: qsTr("Volume envelope")
                    RowLayout {
                        Layout.fillWidth: true
                        Caption { text: qsTr("Enabled") }
                        EchoSwitch {
                            objectName: "inspectorEnvelopeEnabled"
                            accessibleName: qsTr("Envelope enabled")
                            checked: inspector.clipData !== null && !!(inspector.clipData.gainEnvelope || {}).enabled
                            onToggled: workspace.setClipValue("gainEnvelope",{enabled:checked,points:(inspector.clipData.gainEnvelope || {}).points || []})
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true; spacing: 6
                        SmallButton { text: qsTr("Edit points"); selected: workspace.automationEditing; onClicked: workspace.automationEditing=!workspace.automationEditing }
                        EchoIconButton {
                            source: "qrc:/EchoDesktop/icons/clear-selection.svg"; buttonSize: 26; iconSize: 16; toolTipText: qsTr("Clear envelope")
                            enabled: inspector.clipData !== null && ((inspector.clipData.gainEnvelope || {}).points || []).length>0
                            onClicked: workspace.setClipValue("gainEnvelope",{enabled:false,points:[]})
                        }
                    }
                    SmallButton { text: qsTr("Automatic music ducking"); selected: workspace.duckingVisible; onClicked: workspace.duckingVisible=!workspace.duckingVisible }
                    SoundDuckingPanel {
                        id: duckingPanel
                        visible: workspace.duckingVisible
                        Layout.fillWidth: true
                        document: workspace.document; targetTrackIndex: workspace.selectedTrackIndex
                        assets: workspace.libraryAssets; waveforms: inspector.waveforms; blocked: renderController.running
                        onApplyRequested: candidates => {
                            workspace.mutate(next => {
                                for (const clip of next.tracks[workspace.selectedTrackIndex].clips) {
                                    const candidate=candidates.find(c=>c.id===clip.id);
                                    if(candidate) clip.gainEnvelope=candidate.envelope;
                                }
                            });
                            workspace.automationEditing=true;
                        }
                    }
                }
            }
            Section {
                title: qsTr("Master output")
                MixSlider { label: qsTr("Gain"); accessibleName: qsTr("Master gain"); from: -2400; to: 1200; stepSize: 50; value: inspector.master.gainCentibels || 0; valueText: (value/100).toFixed(1)+" dB"; onCommitted: value => workspace.setMasterValue("gainCentibels",value) }
                RowLayout {
                    Layout.fillWidth: true
                    Caption { text: qsTr("Limiter") }
                    EchoSwitch { accessibleName: qsTr("Limiter"); checked: !!inspector.master.limiterEnabled; onToggled: workspace.setMasterValue("limiterEnabled",checked) }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 8
                    Caption { text: qsTr("Ceiling (dB)") }
                    EchoValueSpinBox {
                        id: ceilingControl
                        objectName: "inspectorMasterCeiling"
                        Layout.preferredWidth: 128
                        from: -600; to: 0
                        enabled: !!inspector.master.limiterEnabled
                        value: inspector.master.limiterCeilingCentibels === undefined ? -100 : inspector.master.limiterCeilingCentibels
                        textFromValue: (value,locale) => Number(value/100).toLocaleString(locale,"f",2)
                        valueFromText: (text,locale) => Math.round(Number.fromLocaleString(locale,text)*100)
                        validator: DoubleValidator { bottom: -6; top: 0; decimals: 2; locale: ceilingControl.locale.name; notation: DoubleValidator.StandardNotation }
                        onValueModified: workspace.setMasterValue("limiterCeilingCentibels",value)
                        Accessible.name: qsTr("Limiter ceiling")
                    }
                }
                Text { Layout.fillWidth: true; visible: renderController.hasResult; text: qsTr("Last mix: %1 LUFS · %2 dBTP").arg(renderController.integratedLufs.toFixed(1)).arg(renderController.truePeakDbtp.toFixed(1)); color: Theme.textMuted; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap }
            }
        }
    }
}

//! Unified adjustment console with a resizable signal-flow navigator and a
//! focused parameter surface. The draft owns order, bypass, history, and
//! persistence; this owner controls only selection and presentation.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import EchoDesktop

Item {
    id: rack

    required property var draft
    required property var meterSource
    required property var analyzer
    required property string sourcePath
    required property string analysisKey
    property bool hasTimeSelection: false
    property int selectionStartMillis: 0
    property int selectionEndMillis: 0

    property string selectedNodeId: "clip"
    readonly property int currentKind: effectKind(selectedNodeId)
    readonly property int preferredParameterWidth: selectedNodeId === "clip" ? Theme.editorSectionColumnWidth * 2 + Theme.editorPanelGap * 3 : currentKind === 0 ? Theme.editorControlTrackWidth * 3 - Theme.editorPanelGap * 2 : currentKind === 1 ? 800 : currentKind === 2 ? 740 : currentKind === 3 ? Theme.editorSectionColumnWidth * 2 + Theme.editorPanelGap * 4 : currentKind === 4 ? Theme.editorSectionColumnWidth * 2 + Theme.editorPanelGap * 2 : Theme.editorControlTrackWidth * 2 + Theme.editorPanelGap * 2
    readonly property int preferredParameterHeight: currentKind === 1 ? 320 : currentKind === 4 ? 340 : 310

    implicitHeight: 320

    function runAnalysis(): void {
        masterPanel.runAnalysis();
    }

    function nodeTitle(kind: int): string {
        if (kind === 0)
            return qsTr("Restore");
        if (kind === 1)
            return qsTr("Equalizer");
        if (kind === 2)
            return qsTr("Dynamics");
        if (kind === 3)
            return qsTr("Space");
        if (kind === 5)
            return qsTr("De-hum");
        if (kind === 6)
            return qsTr("De-click");
        if (kind === 7)
            return qsTr("Channel repair");
        return qsTr("Master");
    }

    function nodeSummary(kind: int): string {
        if (kind === 0)
            return qsTr("De-plosive · Noise reduction · De-esser");
        if (kind === 1)
            return qsTr("6-band parametric");
        if (kind === 2)
            return qsTr("Stereo compressor");
        if (kind === 3)
            return qsTr("Algorithmic room");
        if (kind === 5)
            return qsTr("Mains hum and harmonics");
        if (kind === 6)
            return qsTr("Short impulse repair");
        if (kind === 7)
            return qsTr("Polarity · Routing · Balance · Mono");
        return qsTr("Limiter · Loudness");
    }

    function nodeIcon(kind: int): string {
        if (kind === 0)
            return "qrc:/EchoDesktop/icons/high-pass.svg";
        if (kind === 1)
            return "qrc:/EchoDesktop/icons/equalizer.svg";
        if (kind === 2)
            return "qrc:/EchoDesktop/icons/dynamics.svg";
        if (kind === 3)
            return "qrc:/EchoDesktop/icons/waveform.svg";
        if (kind === 5)
            return "qrc:/EchoDesktop/icons/high-pass.svg";
        if (kind === 6)
            return "qrc:/EchoDesktop/icons/waveform.svg";
        if (kind === 7)
            return "qrc:/EchoDesktop/icons/tune.svg";
        return "qrc:/EchoDesktop/icons/gain.svg";
    }

    function effectId(kind: int): string {
        return ["restoration", "equalizer", "dynamics", "space", "master", "dehum", "declick", "channelRepair"][kind];
    }

    function effectKind(effectId: string): int {
        return ["restoration", "equalizer", "dynamics", "space", "master", "dehum", "declick", "channelRepair"].indexOf(effectId);
    }

    function buildChainModel(chain: var, restorationEnabled: bool, equalizerEnabled: bool, dynamicsEnabled: bool, spaceEnabled: bool, deHumEnabled: bool, deClickEnabled: bool, channelRepairEnabled: bool): var {
        const enabled = [restorationEnabled, equalizerEnabled, dynamicsEnabled, spaceEnabled, true, deHumEnabled, deClickEnabled, channelRepairEnabled];
        const result = [];
        for (let index = 0; index < chain.length; ++index) {
            const kind = Number(chain[index]);
            if (kind === 4)
                continue;
            result.push({
                effectId: effectId(kind),
                title: nodeTitle(kind),
                summary: nodeSummary(kind),
                iconSource: nodeIcon(kind),
                enabled: enabled[kind]
            });
        }
        return result;
    }

    function buildCatalogModel(chain: var): var {
        return [
            {
                effectId: "restoration",
                title: nodeTitle(0),
                summary: nodeSummary(0),
                categoryId: "restore",
                categoryTitle: qsTr("Restoration"),
                iconSource: nodeIcon(0),
                available: chain.indexOf(0) < 0
            },
            {
                effectId: "equalizer",
                title: nodeTitle(1),
                summary: nodeSummary(1),
                categoryId: "tone",
                categoryTitle: qsTr("Tone"),
                iconSource: nodeIcon(1),
                available: chain.indexOf(1) < 0
            },
            {
                effectId: "dynamics",
                title: nodeTitle(2),
                summary: nodeSummary(2),
                categoryId: "dynamics",
                categoryTitle: qsTr("Dynamics"),
                iconSource: nodeIcon(2),
                available: chain.indexOf(2) < 0
            },
            {
                effectId: "space",
                title: nodeTitle(3),
                summary: nodeSummary(3),
                categoryId: "space",
                categoryTitle: qsTr("Space"),
                iconSource: nodeIcon(3),
                available: chain.indexOf(3) < 0
            },
            {
                effectId: "dehum",
                title: nodeTitle(5),
                summary: nodeSummary(5),
                categoryId: "restore",
                categoryTitle: qsTr("Restoration"),
                iconSource: nodeIcon(5),
                available: chain.indexOf(5) < 0
            },
            {
                effectId: "declick",
                title: nodeTitle(6),
                summary: nodeSummary(6),
                categoryId: "restore",
                categoryTitle: qsTr("Restoration"),
                iconSource: nodeIcon(6),
                available: chain.indexOf(6) < 0
            },
            {
                effectId: "channelRepair",
                title: nodeTitle(7),
                summary: nodeSummary(7),
                categoryId: "restore",
                categoryTitle: qsTr("Restoration"),
                iconSource: nodeIcon(7),
                available: chain.indexOf(7) < 0
            }
        ];
    }

    function ensureCurrentNode(): void {
        if (selectedNodeId === "clip")
            return;
        const kind = effectKind(selectedNodeId);
        if (kind < 0 || draft.effectChain.indexOf(kind) < 0)
            selectedNodeId = "clip";
    }

    readonly property var chainModel: buildChainModel(draft.effectChain, draft.restorationEnabled, draft.equalizerEnabled, draft.compressorEnabled, draft.reverbEnabled, draft.deHumEnabled, draft.deClickEnabled, draft.channelRepairEnabled)
    readonly property var catalogModel: buildCatalogModel(draft.effectChain)

    onDraftChanged: ensureCurrentNode()

    Connections {
        target: rack.draft
        function onEffectChainChanged(): void {
            rack.ensureCurrentNode();
        }
    }

    Popup {
        id: catalogPopup

        parent: Overlay.overlay
        width: Math.min(380, parent.width - 24)
        height: Math.min(470, parent.height - 24)
        padding: 0
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        function presentFrom(item: var): void {
            const point = item.mapToItem(Overlay.overlay, 0, 0);
            x = Math.max(12, Math.min(parent.width - width - 12, point.x + item.width + 8));
            y = Math.max(12, Math.min(parent.height - height - 12, point.y));
            open();
        }

        background: null
        contentItem: EffectCatalog {
            effectModel: rack.catalogModel
            onEffectAddRequested: effectId => {
                const kind = rack.effectKind(effectId);
                rack.draft.addEffectNode(kind);
                rack.selectedNodeId = effectId;
                catalogPopup.close();
            }
        }
    }

    SplitView {
        anchors.fill: parent
        orientation: Qt.Horizontal

        handle: Rectangle {
            implicitWidth: 9
            color: SplitHandle.pressed ? Theme.surfaceSelected : SplitHandle.hovered ? Theme.surfaceSubtle : Theme.window

            Rectangle {
                anchors.centerIn: parent
                width: SplitHandle.hovered || SplitHandle.pressed ? 2 : 1
                height: SplitHandle.hovered || SplitHandle.pressed ? 64 : 44
                radius: 1
                color: SplitHandle.pressed ? Theme.accent : Theme.borderStrong

                Behavior on height {
                    NumberAnimation {
                        duration: 90
                    }
                }
            }

            HoverHandler {
                cursorShape: Qt.SplitHCursor
            }
        }

        SoundSignalChain {
            id: signalChain

            SplitView.preferredWidth: Theme.editorRailWidth
            SplitView.minimumWidth: 184
            SplitView.maximumWidth: 260
            SplitView.fillHeight: true
            effectModel: rack.chainModel
            selectedEffectId: rack.selectedNodeId
            masterEnabled: rack.draft.limiterEnabled
            onEffectSelected: effectId => rack.selectedNodeId = effectId
            onEffectBypassRequested: (effectId, bypassed) => rack.draft.setEffectNodeEnabled(rack.effectKind(effectId), !bypassed)
            onEffectDeleteRequested: effectId => rack.draft.removeEffectNode(rack.effectKind(effectId))
            onEffectMoveRequested: (effectId, direction) => rack.draft.moveEffectNode(rack.effectKind(effectId), direction)
            onAddEffectRequested: catalogPopup.presentFrom(signalChain)
        }

        Rectangle {
            id: parameterSurface

            SplitView.fillWidth: true
            SplitView.fillHeight: true
            SplitView.minimumWidth: 440
            color: Theme.window
            clip: true

            Item {
                id: parameterCanvas

                x: Theme.editorPanelGap
                y: 0
                width: Math.min(rack.preferredParameterWidth, Math.max(416, parameterSurface.width - Theme.editorPanelGap * 2))
                height: Math.min(rack.preferredParameterHeight, parameterSurface.height)

                BasicAdjustmentPanel {
                    anchors.fill: parent
                    visible: rack.selectedNodeId === "clip"
                    draft: rack.draft
                    hasTimeSelection: rack.hasTimeSelection
                    selectionStartMillis: rack.selectionStartMillis
                    selectionEndMillis: rack.selectionEndMillis
                }

                ToneEqualizerPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 1
                    draft: rack.draft
                    responseProvider: rack.meterSource
                }

                DynamicsPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 2
                    draft: rack.draft
                    meterSource: rack.meterSource
                }

                SpaceReverbPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 3
                    draft: rack.draft
                }

                MasterOutputPanel {
                    id: masterPanel
                    anchors.fill: parent
                    visible: rack.currentKind === 4
                    draft: rack.draft
                    meterSource: rack.meterSource
                    analyzer: rack.analyzer
                    sourcePath: rack.sourcePath
                    analysisKey: rack.analysisKey
                }

                RestorationPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 0
                    draft: rack.draft
                }

                DeHumPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 5
                    draft: rack.draft
                }

                DeClickPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 6
                    draft: rack.draft
                }

                ChannelRepairPanel {
                    anchors.fill: parent
                    visible: rack.currentKind === 7
                    draft: rack.draft
                }
            }
        }
    }
}

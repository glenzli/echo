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
        return qsTr("Master");
    }

    function nodeSummary(kind: int): string {
        if (kind === 0)
            return qsTr("Noise reduction · De-esser");
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
        return "qrc:/EchoDesktop/icons/gain.svg";
    }

    function effectId(kind: int): string {
        return ["restoration", "equalizer", "dynamics", "space", "master", "dehum", "declick"][kind];
    }

    function effectKind(effectId: string): int {
        return ["restoration", "equalizer", "dynamics", "space", "master", "dehum", "declick"].indexOf(effectId);
    }

    function buildChainModel(chain: var, restorationEnabled: bool, equalizerEnabled: bool, dynamicsEnabled: bool, spaceEnabled: bool, deHumEnabled: bool, deClickEnabled: bool): var {
        const enabled = [restorationEnabled, equalizerEnabled, dynamicsEnabled, spaceEnabled, true, deHumEnabled, deClickEnabled];
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

    readonly property var chainModel: buildChainModel(draft.effectChain, draft.restorationEnabled, draft.equalizerEnabled, draft.compressorEnabled, draft.reverbEnabled, draft.deHumEnabled, draft.deClickEnabled)
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

            SplitView.preferredWidth: 224
            SplitView.minimumWidth: 188
            SplitView.maximumWidth: 300
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
            SplitView.minimumWidth: 480
            radius: Theme.compactControlRadius
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
            clip: true

            BasicAdjustmentPanel {
                anchors.fill: parent
                visible: rack.selectedNodeId === "clip"
                radius: 0
                border.width: 0
                draft: rack.draft
                hasTimeSelection: rack.hasTimeSelection
                selectionStartMillis: rack.selectionStartMillis
                selectionEndMillis: rack.selectionEndMillis
            }

            ToneEqualizerPanel {
                anchors.fill: parent
                visible: rack.currentKind === 1
                radius: 0
                border.width: 0
                draft: rack.draft
                responseProvider: rack.meterSource
            }

            DynamicsPanel {
                anchors.fill: parent
                visible: rack.currentKind === 2
                radius: 0
                border.width: 0
                draft: rack.draft
                meterSource: rack.meterSource
            }

            SpaceReverbPanel {
                anchors.fill: parent
                visible: rack.currentKind === 3
                radius: 0
                border.width: 0
                draft: rack.draft
            }

            MasterOutputPanel {
                id: masterPanel
                anchors.fill: parent
                visible: rack.currentKind === 4
                radius: 0
                border.width: 0
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
                radius: 0
                border.width: 0
                draft: rack.draft
            }

            DeClickPanel {
                anchors.fill: parent
                visible: rack.currentKind === 6
                radius: 0
                border.width: 0
                draft: rack.draft
            }
        }
    }
}

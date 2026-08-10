//! Authored linear effect chain with direct node selection and a focused
//! parameter surface. The draft owns order, bypass, history, and persistence.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: rack

    required property var draft
    required property var meterSource
    required property var analyzer
    required property string sourcePath
    required property string analysisKey

    property int currentKind: 0

    radius: Theme.compactControlRadius
    color: Theme.panel
    border.width: 1
    border.color: Theme.borderStrong

    function runAnalysis() : void { masterPanel.runAnalysis() }

    function nodeTitle(kind: int) : string {
        if (kind === 0) return qsTr("Restore")
        if (kind === 1) return qsTr("Equalizer")
        if (kind === 2) return qsTr("Dynamics")
        if (kind === 3) return qsTr("Space")
        return qsTr("Master")
    }

    function nodeSummary(kind: int) : string {
        if (kind === 0) return qsTr("Noise reduction · De-esser")
        if (kind === 1) return qsTr("6-band parametric")
        if (kind === 2) return qsTr("Stereo compressor")
        if (kind === 3) return qsTr("Algorithmic room")
        return qsTr("Limiter · Loudness")
    }

    function nodeIcon(kind: int) : string {
        if (kind === 0) return "qrc:/EchoDesktop/icons/high-pass.svg"
        if (kind === 1) return "qrc:/EchoDesktop/icons/equalizer.svg"
        if (kind === 2) return "qrc:/EchoDesktop/icons/dynamics.svg"
        if (kind === 3) return "qrc:/EchoDesktop/icons/waveform.svg"
        return "qrc:/EchoDesktop/icons/gain.svg"
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.preferredWidth: 204
            Layout.fillHeight: true
            color: Theme.panelRaised

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 7
                spacing: 4

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 7
                    Layout.topMargin: 2
                    text: qsTr("EFFECT CHAIN")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    font.letterSpacing: 0.7
                    font.weight: Font.DemiBold
                }

                Repeater {
                    model: rack.draft.effectChain

                    delegate: EffectChainNode {
                        required property int index
                        required property int modelData

                        Layout.fillWidth: true
                        kind: modelData
                        orderIndex: index
                        title: rack.nodeTitle(kind)
                        summary: rack.nodeSummary(kind)
                        iconSource: rack.nodeIcon(kind)
                        nodeEnabled: rack.draft.effectNodeEnabled(kind)
                        selected: rack.currentKind === kind
                        terminal: kind === 4
                        canMoveUp: index > 0 && kind !== 4
                        canMoveDown: index < 3 && kind !== 4
                        onSelectedRequested: rack.currentKind = kind
                        onEnabledRequested: enabled =>
                            rack.draft.setEffectNodeEnabled(kind, enabled)
                        onMoveRequested: direction =>
                            rack.draft.moveEffectNode(kind, direction)
                    }
                }

                Item { Layout.fillHeight: true }

                Text {
                    Layout.fillWidth: true
                    Layout.leftMargin: 7
                    Layout.rightMargin: 5
                    Layout.bottomMargin: 3
                    text: qsTr("Signal flows from top to bottom. Master stays last.")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: Theme.border
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

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
        }
    }
}

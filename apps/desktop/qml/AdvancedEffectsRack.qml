//! One compact advanced-effects region. Tabs keep future effects from
//! consuming horizontal workspace while each semantic panel stays separate.

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

    property int currentIndex: 0

    radius: Theme.compactControlRadius
    color: Theme.panel
    border.width: 1
    border.color: Theme.borderStrong

    function runAnalysis() : void { masterPanel.runAnalysis() }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 31
            Layout.leftMargin: 5
            Layout.rightMargin: 5
            spacing: 2

            Repeater {
                model: [qsTr("Restore"), qsTr("EQ"), qsTr("Dynamics"), qsTr("Space"), qsTr("Master")]

                delegate: Rectangle {
                    required property int index
                    required property string modelData

                    Layout.fillWidth: true
                    Layout.preferredHeight: 24
                    radius: Theme.compactControlRadius
                    color: rack.currentIndex === index ? Theme.surfaceSelected : "transparent"

                    Text {
                        anchors.centerIn: parent
                        text: parent.modelData
                        color: rack.currentIndex === parent.index
                            ? Theme.textPrimary : Theme.textSecondary
                        font.pixelSize: Theme.fontMeta
                        font.weight: rack.currentIndex === parent.index
                            ? Font.DemiBold : Font.Normal
                    }

                    TapHandler { onTapped: rack.currentIndex = parent.index }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ToneEqualizerPanel {
                anchors.fill: parent
                visible: rack.currentIndex === 1
                draft: rack.draft
                responseProvider: rack.meterSource
            }

            DynamicsPanel {
                anchors.fill: parent
                visible: rack.currentIndex === 2
                draft: rack.draft
                meterSource: rack.meterSource
            }

            SpaceReverbPanel {
                anchors.fill: parent
                visible: rack.currentIndex === 3
                draft: rack.draft
            }

            MasterOutputPanel {
                id: masterPanel
                anchors.fill: parent
                visible: rack.currentIndex === 4
                draft: rack.draft
                meterSource: rack.meterSource
                analyzer: rack.analyzer
                sourcePath: rack.sourcePath
                analysisKey: rack.analysisKey
            }

            RestorationPanel {
                anchors.fill: parent
                visible: rack.currentIndex === 0
                draft: rack.draft
            }
        }
    }
}

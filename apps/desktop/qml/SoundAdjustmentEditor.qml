//! Borderless adjustment workbench. Global draft history and publication live
//! in the window toolbar; this owner composes the editing surfaces themselves.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import EchoDesktop

Item {
    id: inspector

    required property var draft
    required property var meterSource
    required property var analyzer
    required property string sourcePath
    required property string analysisKey
    property bool hasTimeSelection: false
    property int selectionStartMillis: 0
    property int selectionEndMillis: 0

    implicitHeight: 300

    function analyzeOutput() : void {
        effectsRack.runAnalysis()
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 8
        anchors.rightMargin: 8
        anchors.topMargin: 6
        anchors.bottomMargin: 6
        spacing: 10

        BasicAdjustmentPanel {
            Layout.preferredWidth: Math.min(310,
                Math.max(286, inspector.width * 0.19))
            Layout.minimumWidth: 280
            Layout.maximumWidth: 310
            Layout.fillHeight: true
            draft: inspector.draft
            hasTimeSelection: inspector.hasTimeSelection
            selectionStartMillis: inspector.selectionStartMillis
            selectionEndMillis: inspector.selectionEndMillis
        }

        AdvancedEffectsRack {
            id: effectsRack
            Layout.preferredWidth: Math.min(840,
                Math.max(620, inspector.width - 350))
            Layout.minimumWidth: 600
            Layout.maximumWidth: 840
            Layout.fillHeight: true
            draft: inspector.draft
            meterSource: inspector.meterSource
            analyzer: inspector.analyzer
            sourcePath: inspector.sourcePath
            analysisKey: inspector.analysisKey
        }

        Item { Layout.fillWidth: true }
    }
}

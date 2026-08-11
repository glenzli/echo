//! Borderless adjustment workbench. Global draft history and publication live
//! in the window toolbar; this owner composes the resizable signal chain and
//! the selected node's parameter surface.

pragma ComponentBehavior: Bound

import QtQuick
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

    implicitHeight: 320

    function analyzeOutput(): void {
        effectsRack.runAnalysis();
    }

    AdvancedEffectsRack {
        id: effectsRack
        anchors.fill: parent
        anchors.leftMargin: 8
        anchors.rightMargin: 8
        anchors.topMargin: 6
        anchors.bottomMargin: 6
        draft: inspector.draft
        meterSource: inspector.meterSource
        analyzer: inspector.analyzer
        sourcePath: inspector.sourcePath
        analysisKey: inspector.analysisKey
        hasTimeSelection: inspector.hasTimeSelection
        selectionStartMillis: inspector.selectionStartMillis
        selectionEndMillis: inspector.selectionEndMillis
    }
}

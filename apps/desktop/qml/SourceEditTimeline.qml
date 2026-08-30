//! Source-anchored edit overlay for one waveform viewport. It owns segment
//! discovery, contextual source-edit actions, gap markers, mask lanes, and
//! mask editor placement while the host timeline owns zoom and selection.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Item {
    id: sourceEdit

    required property var draft
    required property var timeline

    readonly property bool hasSelection: timeline.hasTimeSelection && timeline.selectionEndMillis > timeline.selectionStartMillis
    readonly property int selectedSegmentIndex: selectionExactSegmentIndex()

    function nodeTitle(kind: int): string {
        if (kind === 0)
            return qsTr("Restoration");
        if (kind === 1)
            return qsTr("Equalizer");
        if (kind === 2)
            return qsTr("Dynamics");
        if (kind === 3)
            return qsTr("Space");
        if (kind === 5)
            return qsTr("De-hum");
        if (kind === 7)
            return qsTr("Channel repair");
        if (kind === 8)
            return qsTr("Scene VFX");
        if (kind === 9)
            return qsTr("Delay VFX");
        if (kind === 10)
            return qsTr("Modulation VFX");
        if (kind === 12)
            return qsTr("Digital Degrade");
        if (kind === 17)
            return qsTr("Tape");
        if (kind === 19)
            return qsTr("Auto-Wah");
        if (kind === 20)
            return qsTr("Stereo");
        return qsTr("Effect");
    }

    function maskTitle(nodes: var): string {
        const titles = [];
        for (let index = 0; index < nodes.length; ++index)
            titles.push(nodeTitle(Number(nodes[index])));
        return titles.join(" + ");
    }

    function selectionTouchesState(state: int): bool {
        if (!hasSelection)
            return false;
        for (let index = 0; index < draft.editSegments.length; ++index) {
            const segment = draft.editSegments[index];
            if (Number(segment.state) === state && Number(segment.sourceEndMillis) > timeline.selectionStartMillis && Number(segment.sourceStartMillis) < timeline.selectionEndMillis)
                return true;
        }
        return false;
    }

    function selectionExactSegmentIndex(): int {
        if (!hasSelection)
            return -1;
        for (let index = 0; index < draft.editSegments.length; ++index) {
            const segment = draft.editSegments[index];
            if (Number(segment.sourceStartMillis) === timeline.selectionStartMillis
                    && Number(segment.sourceEndMillis) === timeline.selectionEndMillis)
                return index;
        }
        return -1;
    }

    function selectRange(startMillis: int, endMillis: int): void {
        timeline.selectionStartMillis = startMillis;
        timeline.selectionEndMillis = endMillis;
        timeline.hasTimeSelection = endMillis > startMillis;
        timeline.followPlayhead = false;
    }

    function perform(action: string): void {
        if (!hasSelection)
            return;
        draft.beginGesture();
        if (action === "split") {
            draft.splitSelection(timeline.selectionStartMillis, timeline.selectionEndMillis);
        } else if (action === "hide") {
            draft.setSelectionState(timeline.selectionStartMillis, timeline.selectionEndMillis, 2);
        } else if (action === "mute") {
            draft.setSelectionState(timeline.selectionStartMillis, timeline.selectionEndMillis, 1);
        } else if (action === "restore") {
            draft.setSelectionState(timeline.selectionStartMillis, timeline.selectionEndMillis, 0);
        }
        draft.endGesture();
    }

    Popup {
        id: gapEditor

        property int durationMillis: 1000
        property int editingSegmentIndex: -1

        parent: Overlay.overlay
        width: 320
        padding: 0
        modal: false
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        function present(trigger: var, segmentIndex: int): void {
            editingSegmentIndex = segmentIndex;
            durationMillis = segmentIndex >= 0 && segmentIndex < sourceEdit.draft.editSegments.length ? Number(sourceEdit.draft.editSegments[segmentIndex].gapAfterMillis) : Math.round(sourceEdit.timeline.clamp(sourceEdit.timeline.selectionEndMillis - sourceEdit.timeline.selectionStartMillis, 10, 3600000));
            const point = trigger.mapToItem(Overlay.overlay, 0, trigger.height);
            x = Math.max(16, Math.min(parent.width - width - 16, point.x + trigger.width / 2 - width / 2));
            y = Math.max(16, Math.min(parent.height - implicitHeight - 16, point.y + 8));
            open();
        }

        function insert(): void {
            sourceEdit.draft.beginGesture();
            if (editingSegmentIndex >= 0)
                sourceEdit.draft.setSegmentGap(editingSegmentIndex, durationMillis);
            else
                sourceEdit.draft.insertGap(sourceEdit.timeline.selectionEndMillis, durationMillis);
            sourceEdit.draft.endGesture();
            close();
        }

        function remove(): void {
            if (editingSegmentIndex < 0)
                return;
            sourceEdit.draft.beginGesture();
            const removed = sourceEdit.draft.setSegmentGap(editingSegmentIndex, 0);
            sourceEdit.draft.endGesture();
            if (removed)
                close();
        }

        background: Rectangle {
            radius: Theme.controlRadius + 2
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: ColumnLayout {
            spacing: 0

            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: 14
                spacing: 4

                Text {
                    text: gapEditor.editingSegmentIndex >= 0 ? qsTr("Edit silence") : qsTr("Insert silence")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: gapEditor.editingSegmentIndex >= 0 ? qsTr("The original stays unchanged. Adjust or remove this silence.") : qsTr("The original stays unchanged. Silence is added after the selection.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 14
                Layout.rightMargin: 14
                Layout.topMargin: 8
                Layout.bottomMargin: 8
                spacing: 10

                Text {
                    text: qsTr("Duration")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                Slider {
                    objectName: "gapDurationSlider"
                    Layout.fillWidth: true
                    from: Math.log(10) / Math.LN10
                    to: Math.log(3600000) / Math.LN10
                    value: Math.log(Math.max(10, gapEditor.durationMillis)) / Math.LN10
                    Accessible.name: qsTr("Inserted silence duration")
                    onMoved: gapEditor.durationMillis = Math.round(Math.pow(10, value) / 10) * 10
                }

                Text {
                    Layout.preferredWidth: 58
                    text: sourceEdit.timeline.formatTime(gapEditor.durationMillis, true)
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMeta
                    horizontalAlignment: Text.AlignRight
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: Theme.border
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.margins: 10
                spacing: 8

                Item {
                    Layout.fillWidth: true
                }

                EchoButton {
                    visible: gapEditor.editingSegmentIndex >= 0
                    objectName: "removeGapButton"
                    text: qsTr("Remove silence")
                    ghost: true
                    enabled: gapEditor.editingSegmentIndex >= 0 && sourceEdit.draft.canRemoveSegmentGap(gapEditor.editingSegmentIndex)
                    ToolTip.visible: hovered && !enabled
                    ToolTip.text: qsTr("Keep at least one playable section.")
                    onClicked: gapEditor.remove()
                }

                EchoButton {
                    objectName: "cancelGapButton"
                    text: qsTr("Cancel")
                    ghost: true
                    onClicked: gapEditor.close()
                }

                EchoButton {
                    objectName: "confirmGapButton"
                    text: gapEditor.editingSegmentIndex >= 0 ? qsTr("Update") : qsTr("Insert")
                    onClicked: gapEditor.insert()
                }
            }
        }
    }

    EffectMaskEditor {
        id: maskEditor
        draft: sourceEdit.draft
    }

    SourceRegionInspector {
        id: regionInspector
        draft: sourceEdit.draft
        timeline: sourceEdit.timeline
    }

    Repeater {
        model: sourceEdit.draft.effectMasks

        delegate: Rectangle {
            id: maskBar

            required property int index
            required property int startMillis
            required property int endMillis
            required property int featherMillis
            required property var effectNodes

            x: sourceEdit.timeline.timeToX(startMillis)
            y: 48 + index % 3 * 22
            width: Math.max(8, sourceEdit.timeline.timeToX(endMillis) - x)
            height: 18
            radius: 5
            color: Theme.accentSurface
            border.width: 1
            border.color: Theme.accent
            visible: x + width >= 0 && x <= sourceEdit.width
            clip: true
            z: 7

            Text {
                anchors.fill: parent
                anchors.leftMargin: 7
                anchors.rightMargin: 7
                text: sourceEdit.maskTitle(maskBar.effectNodes)
                color: Theme.accentSelectionText
                font.pixelSize: 9
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
            }

            HoverHandler {
                id: maskHover
            }
            ToolTip.visible: maskHover.hovered
            ToolTip.text: qsTr("Effect mask · %1 ms soft edge").arg(maskBar.featherMillis)
            ToolTip.delay: 450

            TapHandler {
                onTapped: {
                    sourceEdit.selectRange(maskBar.startMillis, maskBar.endMillis);
                    maskEditor.present(maskBar, maskBar.startMillis, maskBar.endMillis, maskBar.index);
                }
            }
        }
    }

    Repeater {
        model: sourceEdit.draft.editSegments

        delegate: Rectangle {
            id: segmentBand

            required property int index
            required property var modelData

            readonly property int sourceStartMillis: Number(modelData.sourceStartMillis)
            readonly property int sourceEndMillis: Number(modelData.sourceEndMillis)
            readonly property int segmentState: Number(modelData.state)
            readonly property int gainCentibels: Number(modelData.gainCentibels)
            readonly property int fadeInMillis: Number(modelData.fadeInMillis)
            readonly property int fadeOutMillis: Number(modelData.fadeOutMillis)
            readonly property int fadeInCurve: Number(modelData.fadeInCurve)
            readonly property int fadeOutCurve: Number(modelData.fadeOutCurve)
            readonly property int gapAfterMillis: Number(modelData.gapAfterMillis)

            x: sourceEdit.timeline.timeToX(sourceStartMillis)
            y: sourceEdit.height - 36
            width: Math.max(3, sourceEdit.timeline.timeToX(sourceEndMillis) - x)
            height: 28
            radius: 4
            color: segmentState === 1 ? Theme.warningSurface : segmentState === 2 ? Theme.surfaceSelected : Theme.accentSurfaceQuiet
            border.width: segmentState === 0 ? 1 : 1.5
            border.color: segmentState === 1 ? Theme.warningText : segmentState === 2 ? Theme.textDisabled : Theme.borderStrong
            opacity: segmentState === 2 ? 0.62 : 0.92
            visible: x + width >= 0 && x <= sourceEdit.width
            z: 7
            clip: false

            Canvas {
                id: regionEnvelope

                anchors.fill: parent
                anchors.margins: 2
                visible: segmentBand.segmentState === 0 && segmentBand.width > 24
                opacity: 0.8
                antialiasing: true
                renderStrategy: Canvas.Immediate

                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onPaint: {
                    const context = getContext("2d");
                    context.reset();
                    context.clearRect(0, 0, width, height);
                    if (width <= 0 || height <= 0)
                        return;
                    const duration = Math.max(1, segmentBand.sourceEndMillis - segmentBand.sourceStartMillis);
                    const gainY = 3 + (1200 - Math.max(-2400, Math.min(1200, segmentBand.gainCentibels))) / 3600 * Math.max(1, height - 6);
                    const silenceY = height - 2;
                    const fadeInX = width * Math.min(1, segmentBand.fadeInMillis / duration);
                    const fadeOutX = width * (1 - Math.min(1, segmentBand.fadeOutMillis / duration));
                    context.strokeStyle = Theme.accent;
                    context.lineWidth = 1.5;
                    context.beginPath();
                    context.moveTo(0, segmentBand.fadeInMillis > 0 ? silenceY : gainY);
                    if (segmentBand.fadeInMillis > 0) {
                        for (let step = 1; step <= 12; ++step) {
                            const progress = step / 12;
                            const amplitude = sourceEdit.timeline.curveValue(progress, segmentBand.fadeInCurve);
                            context.lineTo(fadeInX * progress, silenceY + (gainY - silenceY) * amplitude);
                        }
                    }
                    context.lineTo(fadeOutX, gainY);
                    if (segmentBand.fadeOutMillis > 0) {
                        for (let step = 1; step <= 12; ++step) {
                            const progress = step / 12;
                            const amplitude = sourceEdit.timeline.curveValue(1 - progress, segmentBand.fadeOutCurve);
                            context.lineTo(fadeOutX + (width - fadeOutX) * progress, silenceY + (gainY - silenceY) * amplitude);
                        }
                    } else {
                        context.lineTo(width, gainY);
                    }
                    context.stroke();
                }
            }

            Canvas {
                anchors.fill: parent
                visible: segmentBand.segmentState === 2
                opacity: 0.45
                onWidthChanged: requestPaint()
                onHeightChanged: requestPaint()
                onPaint: {
                    const context = getContext("2d");
                    context.reset();
                    context.strokeStyle = Theme.textDisabled;
                    context.lineWidth = 1;
                    for (let offset = -height; offset < width; offset += 8) {
                        context.beginPath();
                        context.moveTo(offset, height);
                        context.lineTo(offset + height, 0);
                        context.stroke();
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                visible: segmentBand.width > 42 && segmentBand.segmentState !== 0
                text: segmentBand.segmentState === 1 ? qsTr("Muted") : qsTr("Hidden")
                color: segmentBand.segmentState === 1 ? Theme.warningText : Theme.textSecondary
                font.pixelSize: 9
            }

            Text {
                anchors.right: parent.right
                anchors.rightMargin: 5
                anchors.verticalCenter: parent.verticalCenter
                visible: segmentBand.width > 70 && segmentBand.gainCentibels !== 0
                text: (segmentBand.gainCentibels > 0 ? "+" : "") + (segmentBand.gainCentibels / 100).toFixed(1) + " dB"
                color: Theme.textSecondary
                font.pixelSize: 9
            }

            Rectangle {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: 3
                height: parent.height + (segmentBand.segmentState === 2 ? 5 : 0)
                radius: 1
                color: segmentBand.segmentState === 2 ? Theme.textDisabled : Theme.borderStrong
            }

            TapHandler {
                onTapped: sourceEdit.selectRange(segmentBand.sourceStartMillis, segmentBand.sourceEndMillis)
            }

            Rectangle {
                id: gapMarker

                objectName: "insertedGapMarker" + segmentBand.index

                anchors.left: parent.right
                anchors.leftMargin: -2
                anchors.verticalCenter: parent.verticalCenter
                width: 5
                height: parent.height + 8
                radius: 2
                visible: segmentBand.gapAfterMillis > 0
                color: Theme.accent
                border.color: Theme.panelRaised
                z: 2

                HoverHandler {
                    id: gapHover
                }
                ToolTip.visible: gapHover.hovered
                ToolTip.text: qsTr("Inserted silence · %1").arg(sourceEdit.timeline.formatTime(segmentBand.gapAfterMillis, true))

                TapHandler {
                    margin: 8
                    onTapped: {
                        sourceEdit.selectRange(segmentBand.sourceStartMillis, segmentBand.sourceEndMillis);
                        gapEditor.present(gapMarker, segmentBand.index);
                    }
                }
            }
        }
    }

    Rectangle {
        id: actionBar

        anchors.horizontalCenter: parent.horizontalCenter
        y: 10
        width: actionRow.implicitWidth + 12
        height: 34
        radius: 9
        color: Theme.chrome
        border.width: 1
        border.color: Theme.borderStrong
        visible: sourceEdit.hasSelection
        z: 12

        RowLayout {
            id: actionRow
            anchors.centerIn: parent
            spacing: 1

            EchoButton {
                objectName: "splitSourceButton"
                text: qsTr("Split")
                ghost: true
                implicitHeight: 26
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Create region boundaries at the selection edges · S")
                onClicked: sourceEdit.perform("split")
            }

            EchoButton {
                id: regionButton
                objectName: "editSourceRegionButton"
                text: qsTr("Region")
                ghost: true
                implicitHeight: 26
                enabled: sourceEdit.selectedSegmentIndex >= 0
                ToolTip.visible: hovered
                ToolTip.text: enabled ? qsTr("Edit region gain and fades") : qsTr("Split the selection first to edit it as one region.")
                onClicked: regionInspector.present(regionButton, sourceEdit.selectedSegmentIndex)
            }

            EchoButton {
                objectName: "hideSourceButton"
                text: qsTr("Ripple remove")
                ghost: true
                implicitHeight: 26
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Close the selected time without changing the Original · Delete")
                onClicked: sourceEdit.perform("hide")
            }

            EchoButton {
                objectName: "muteSourceButton"
                text: qsTr("Mute")
                ghost: true
                implicitHeight: 26
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Keep the selected duration but silence its audio · M")
                onClicked: sourceEdit.perform("mute")
            }

            EchoButton {
                objectName: "restoreSourceButton"
                text: qsTr("Restore")
                ghost: true
                implicitHeight: 26
                enabled: sourceEdit.selectionTouchesState(1) || sourceEdit.selectionTouchesState(2)
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Restore muted or ripple-removed source time · R")
                onClicked: sourceEdit.perform("restore")
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 18
                color: Theme.border
            }

            EchoButton {
                id: gapButton
                objectName: "insertGapButton"
                text: qsTr("Insert gap")
                ghost: true
                implicitHeight: 26
                onClicked: gapEditor.present(gapButton, -1)
            }

            EchoButton {
                id: addMaskButton
                objectName: "addEffectMaskButton"
                text: qsTr("Effect mask")
                ghost: true
                implicitHeight: 26
                onClicked: maskEditor.present(addMaskButton, sourceEdit.timeline.selectionStartMillis, sourceEdit.timeline.selectionEndMillis, -1)
            }
        }
    }
}

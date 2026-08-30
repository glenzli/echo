//! Precision owner for one source-anchored edit region. It edits the authored
//! region gain and boundary fades as one undoable draft mutation; the audio
//! engine remains the sole executor for audition, analysis, and export.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: inspector

    required property var draft
    required property var timeline

    property int regionIndex: -1
    property int sourceStartMillis: 0
    property int sourceEndMillis: 0
    property int gainCentibels: 0
    property int fadeInMillis: 0
    property int fadeOutMillis: 0
    property int fadeInCurve: 0
    property int fadeOutCurve: 0

    readonly property int regionDurationMillis: Math.max(0, sourceEndMillis - sourceStartMillis)

    parent: Overlay.overlay
    width: Math.min(420, parent.width - 32)
    padding: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function formatGain(centibels: int): string {
        return (centibels >= 0 ? "+" : "") + (centibels / 100).toFixed(1) + " dB";
    }

    function present(trigger: var, index: int): void {
        if (index < 0 || index >= draft.editSegments.length)
            return;
        const region = draft.editSegments[index];
        regionIndex = index;
        sourceStartMillis = Number(region.sourceStartMillis);
        sourceEndMillis = Number(region.sourceEndMillis);
        gainCentibels = Number(region.gainCentibels);
        fadeInMillis = Number(region.fadeInMillis);
        fadeOutMillis = Number(region.fadeOutMillis);
        fadeInCurve = Number(region.fadeInCurve);
        fadeOutCurve = Number(region.fadeOutCurve);
        const point = trigger.mapToItem(Overlay.overlay, 0, trigger.height);
        x = Math.max(16, Math.min(parent.width - width - 16, point.x + trigger.width / 2 - width / 2));
        y = Math.max(16, Math.min(parent.height - implicitHeight - 16, point.y + 8));
        open();
    }

    function apply(): void {
        if (regionIndex < 0)
            return;
        draft.beginGesture();
        draft.setSegmentEnvelope(regionIndex, gainCentibels, fadeInMillis, fadeOutMillis, fadeInCurve, fadeOutCurve);
        draft.endGesture();
        close();
    }

    function reset(): void {
        gainCentibels = 0;
        fadeInMillis = 0;
        fadeOutMillis = 0;
        fadeInCurve = 0;
        fadeOutCurve = 0;
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
            Layout.margins: 16
            spacing: 4

            Text {
                text: qsTr("Region settings")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("%1 — %2 · %3").arg(inspector.timeline.formatTime(inspector.sourceStartMillis, true)).arg(inspector.timeline.formatTime(inspector.sourceEndMillis, true)).arg(inspector.timeline.formatTime(inspector.regionDurationMillis, true))
                color: Theme.textSecondary
                font.family: "Menlo"
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Gain and fades stay anchored to this original-time region.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            Layout.topMargin: 10
            Layout.bottomMargin: 10
            spacing: 8

            EchoParameterSlider {
                Layout.fillWidth: true
                label: qsTr("Gain")
                from: -2400
                to: 1200
                stepSize: 50
                value: inspector.gainCentibels
                valueText: inspector.formatGain(inspector.gainCentibels)
                accessibleName: qsTr("Region gain")
                neutralValue: 0
                showNeutralMarker: true
                onEdited: value => inspector.gainCentibels = Math.round(value)
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                label: qsTr("Fade in")
                from: 0
                to: Math.max(1, inspector.regionDurationMillis - inspector.fadeOutMillis)
                stepSize: 10
                value: inspector.fadeInMillis
                valueText: inspector.timeline.formatTime(inspector.fadeInMillis, true)
                accessibleName: qsTr("Region fade in")
                fillFromMinimum: true
                onEdited: value => inspector.fadeInMillis = Math.round(value)
            }

            EchoSegmentedControl {
                Layout.fillWidth: true
                model: [qsTr("Linear"), qsTr("Smooth"), qsTr("Equal power")]
                currentIndex: inspector.fadeInCurve
                onActivated: index => inspector.fadeInCurve = index
            }

            EchoParameterSlider {
                Layout.fillWidth: true
                label: qsTr("Fade out")
                from: 0
                to: Math.max(1, inspector.regionDurationMillis - inspector.fadeInMillis)
                stepSize: 10
                value: inspector.fadeOutMillis
                valueText: inspector.timeline.formatTime(inspector.fadeOutMillis, true)
                accessibleName: qsTr("Region fade out")
                fillFromMinimum: true
                onEdited: value => inspector.fadeOutMillis = Math.round(value)
            }

            EchoSegmentedControl {
                Layout.fillWidth: true
                model: [qsTr("Linear"), qsTr("Smooth"), qsTr("Equal power")]
                currentIndex: inspector.fadeOutCurve
                onActivated: index => inspector.fadeOutCurve = index
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.border }

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 12
            spacing: 8

            EchoButton {
                objectName: "resetRegionEnvelopeButton"
                text: qsTr("Reset region")
                ghost: true
                onClicked: inspector.reset()
            }

            Item { Layout.fillWidth: true }

            EchoButton {
                objectName: "cancelRegionEnvelopeButton"
                text: qsTr("Cancel")
                ghost: true
                onClicked: inspector.close()
            }

            EchoButton {
                objectName: "applyRegionEnvelopeButton"
                text: qsTr("Apply")
                onClicked: inspector.apply()
            }
        }
    }
}

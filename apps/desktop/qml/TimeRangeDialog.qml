//! Exact-time navigation only; it never edits or publishes the audio draft.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "Timecode.js" as Timecode

Popup {
    id: dialog
    property bool rangeMode: true
    property real minimumMillis: 0
    property real maximumMillis: 0
    property string contextKey: ""
    readonly property var startValue: Timecode.parse(startInput.text)
    readonly property var endValue: rangeMode ? Timecode.parse(endInput.text) : startValue
    readonly property bool validRange: startValue !== null && endValue !== null && startValue >= minimumMillis && endValue <= maximumMillis && (rangeMode ? endValue > startValue : startValue <= maximumMillis)
    signal requested(real startMillis, real endMillis)
    parent: Overlay.overlay
    x: Math.round((parent.width-width)/2); y: Math.round((parent.height-height)/2)
    width: Math.min(440,parent.width-32); padding: 20
    modal: true; dim: true; focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    background: Rectangle { radius: 12; color: Theme.panelRaised; border.color: Theme.border }
    function present(start: real, end: real): void {
        startInput.text = Timecode.format(Math.max(minimumMillis,Math.min(maximumMillis,start)));
        endInput.text = Timecode.format(Math.max(minimumMillis,Math.min(maximumMillis,end)));
        open();
        startInput.forceActiveFocus(); startInput.selectAll();
    }
    function apply(): void {
        if (!validRange) return;
        const start = startValue, end = endValue;
        close(); requested(start,end);
    }
    onContextKeyChanged: close()
    contentItem: ColumnLayout {
        spacing: 12
        Label { text: dialog.rangeMode ? qsTr("Select exact time range") : qsTr("Go to time"); font.pixelSize: 18; font.weight: Font.DemiBold; color: Theme.textPrimary }
        Label { Layout.fillWidth: true; wrapMode: Text.Wrap; text: qsTr("Enter seconds, m:ss, or h:mm:ss. Milliseconds are optional."); color: Theme.textSecondary }
        Label { text: dialog.rangeMode ? qsTr("Start") : qsTr("Position"); color: Theme.textSecondary }
        EchoTextField { id: startInput; objectName: "exactTimeStart"; Layout.fillWidth: true; font.family: "Menlo"; Accessible.name: dialog.rangeMode ? qsTr("Start") : qsTr("Position"); onAccepted: dialog.apply() }
        Label { visible: dialog.rangeMode; text: qsTr("End"); color: Theme.textSecondary }
        EchoTextField { id: endInput; objectName: "exactTimeEnd"; visible: dialog.rangeMode; Layout.fillWidth: true; font.family: "Menlo"; Accessible.name: qsTr("End"); onAccepted: dialog.apply() }
        Label {
            Layout.fillWidth: true; wrapMode: Text.Wrap
            text: qsTr("Available: %1 – %2").arg(Timecode.format(dialog.minimumMillis)).arg(Timecode.format(dialog.maximumMillis))
            color: Theme.textMuted; font.pixelSize: Theme.fontMeta
        }
        Label {
            visible: !dialog.validRange; Layout.fillWidth: true; wrapMode: Text.Wrap
            text: dialog.rangeMode ? qsTr("Enter a valid range with the end after the start, within the available audio.") : qsTr("Enter a valid time within the project.")
            color: Theme.warningText; font.pixelSize: Theme.fontMeta
        }
        RowLayout {
            Layout.alignment: Qt.AlignRight
            EchoButton { text: qsTr("Cancel"); ghost: true; onClicked: dialog.close() }
            EchoButton { objectName: "applyExactTime"; text: dialog.rangeMode ? qsTr("Select range") : qsTr("Go"); enabled: dialog.validRange; onClicked: dialog.apply() }
        }
    }
}

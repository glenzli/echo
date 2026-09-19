//! Theme-aligned selector shared by the source bin and project inspector.
import QtQuick
import QtQuick.Controls

ComboBox {
    id: control
    // Opt-in semantic selection. Rebuilding translated string models resets
    // ComboBox.currentIndex internally, even when the authored value is stable.
    property int selectionIndex: -1
    onSelectionIndexChanged: {
        if (selectionIndex >= 0)
            currentIndex = selectionIndex;
    }
    onModelChanged: Qt.callLater(() => {
        if (control.selectionIndex >= 0)
            control.currentIndex = control.selectionIndex;
    })
    implicitHeight: Theme.controlHeight
    font.pixelSize: Theme.fontBody
    leftPadding: 10
    rightPadding: 28
    contentItem: Text {
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.textPrimary : Theme.textDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    indicator: Text {
        x: control.width - width - 10
        y: (control.height - height) / 2
        text: "⌄"
        color: Theme.textMuted
        font.pixelSize: 16
    }
    background: Rectangle {
        radius: Theme.controlRadius
        color: control.down ? Theme.controlPressed : Theme.control
        border.color: control.activeFocus ? Theme.focusRing : Theme.borderStrong
    }
}

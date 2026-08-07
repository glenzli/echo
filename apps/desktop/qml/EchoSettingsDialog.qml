//! EchoSettingsDialog: application settings. Appearance mode (System / Light
//! / Dark) persists through UiPreferences; the effective palette follows the
//! platform scheme in System mode.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    /// Host-provided UI preferences (assigned after the shell composes).
    property UiPreferences uiPrefs: null

    title: qsTr("Settings")
    modal: true
    standardButtons: Dialog.Close
    width: 340
    padding: Theme.panelPadding

    contentItem: ColumnLayout {
        spacing: 12

        Text {
            text: qsTr("Appearance")
            color: Theme.textPrimary
            font.pixelSize: Theme.fontSection
            font.bold: true
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Repeater {
                model: [
                    { key: 0, label: qsTr("System") },
                    { key: 1, label: qsTr("Light") },
                    { key: 2, label: qsTr("Dark") }
                ]

                delegate: EchoButton {
                    Layout.fillWidth: true
                    text: modelData.label
                    ghost: true
                    backgroundColor: Theme.accentSurface
                    textColor: Theme.accentSelectionText
                    // Selected state is drawn by the caller through the
                    // non-ghost palette.
                    background: Rectangle {
                        radius: Theme.controlRadius
                        border.width: uiPrefs.mode === modelData.key ? 1 : 0
                        border.color: uiPrefs.mode === modelData.key
                            ? Theme.accent : Theme.buttonBorder
                        color: uiPrefs.mode === modelData.key
                            ? Theme.accentSurface : Theme.control
                        Behavior on color {
                            ColorAnimation { duration: 80 }
                        }
                    }
                    contentItem: Text {
                        text: modelData.label
                        color: uiPrefs.mode === modelData.key
                            ? Theme.accentSelectionText : Theme.textPrimary
                        font.pixelSize: Theme.fontBody
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                    onClicked: dialog.uiPrefs.mode = modelData.key
                }
            }
        }

        Text {
            text: qsTr("Echo follows the system appearance in System mode; "
                       + "you can pin Light or Dark at any time.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }
}

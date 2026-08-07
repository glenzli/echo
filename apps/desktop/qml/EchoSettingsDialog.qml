//! EchoSettingsDialog: application settings. Appearance mode (System / Light
//! / Dark) persists through UiPreferences; model access is configured through
//! ModelPreferences (Echo never downloads models — the user maintains the
//! shared HuggingFace cache).

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    /// Host-provided preferences (assigned after the shell composes).
    property UiPreferences uiPrefs: null
    property ModelPreferences modelPrefs: null

    title: qsTr("Settings")
    modal: true
    standardButtons: Dialog.Close
    width: 420
    padding: Theme.panelPadding

    contentItem: ColumnLayout {
        spacing: 14

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

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        Text {
            text: qsTr("Models")
            color: Theme.textPrimary
            font.pixelSize: Theme.fontSection
            font.bold: true
        }

        Text {
            text: qsTr("Echo reads models from the shared HuggingFace cache "
                       + "and never downloads them. Point the interpreter at "
                       + "a Python with mlx-audio installed.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: 8
            rowSpacing: 8

            Text {
                text: qsTr("Model root")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
            }

            TextField {
                Layout.fillWidth: true
                text: modelPrefs.modelRoot
                onEditingFinished: modelPrefs.modelRoot = text
                selectByMouse: true
            }

            Text {
                text: qsTr("Python")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
            }

            TextField {
                Layout.fillWidth: true
                text: modelPrefs.python
                placeholderText: qsTr("python3")
                onEditingFinished: modelPrefs.python = text
                selectByMouse: true
            }

            Text {
                text: qsTr("Worker")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontBody
            }

            TextField {
                Layout.fillWidth: true
                text: modelPrefs.workerScript
                onEditingFinished: modelPrefs.workerScript = text
                selectByMouse: true
            }
        }
    }
}

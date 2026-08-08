//! EchoSettingsDialog: application settings with a Shadow-style section list.
//! General (appearance + language) and Models; Library management lives in
//! its own workspace panel.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    /// Host-provided preferences (assigned after the shell composes).
    property UiPreferences uiPrefs: null
    property ModelPreferences modelPrefs: null

    readonly property var sections: [
        { key: "general", title: qsTr("General"), subtitle: qsTr("Appearance & language") },
        { key: "models", title: qsTr("Models"), subtitle: qsTr("Local inference access") }
    ]
    property int selectedIndex: 0

    title: qsTr("Settings")
    modal: true
    standardButtons: Dialog.Close
    width: 560
    height: 400
    padding: 0

    contentItem: RowLayout {
        spacing: 0

        // Section list (left rail).
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 180
            color: Theme.panel
            radius: 10

            ColumnLayout {
                anchors.fill: parent
                anchors.topMargin: 12
                anchors.bottomMargin: 12
                spacing: 2

                Repeater {
                    model: dialog.sections

                    delegate: Rectangle {
                        required property var modelData
                        required property int index

                        Layout.fillWidth: true
                        Layout.preferredHeight: 44
                        radius: Theme.compactControlRadius
                        color: dialog.selectedIndex === index
                            ? Theme.accentSurfaceQuiet : Theme.transparent

                        MouseArea {
                            anchors.fill: parent
                            onClicked: dialog.selectedIndex = index
                        }

                        ColumnLayout {
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.leftMargin: 12
                            spacing: 1

                            Text {
                                text: modelData.title
                                color: dialog.selectedIndex === index
                                    ? Theme.accentSelectionText : Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                                font.bold: dialog.selectedIndex === index
                            }

                            Text {
                                text: modelData.subtitle
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontMeta
                            }
                        }
                    }
                }
            }
        }

        // Selected pane.
        Rectangle {
            Layout.fillHeight: true
            Layout.fillWidth: true
            color: Theme.panelRaised
            radius: 10

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 14

                EchoSectionLabel {
                    Layout.fillWidth: true
                    text: dialog.sections[dialog.selectedIndex].title
                    hint: dialog.sections[dialog.selectedIndex].subtitle
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Theme.border
                }

                // ---- General pane ----
                ColumnLayout {
                    visible: dialog.selectedIndex === 0
                    Layout.fillWidth: true
                    spacing: 14

                    EchoSectionLabel {
                        Layout.fillWidth: true
                        text: qsTr("Appearance")
                        hint: qsTr("Echo follows the system appearance in System "
                                   + "mode; you can pin Light or Dark at any time.")
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
                                selected: uiPrefs.mode === modelData.key
                                onClicked: dialog.uiPrefs.mode = modelData.key
                            }
                        }
                    }

                    EchoSectionLabel {
                        Layout.fillWidth: true
                        text: qsTr("Language")
                        hint: qsTr("The default follows your system language.")
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Repeater {
                            model: [
                                { key: "system", label: qsTr("System") },
                                { key: "zh_CN", label: "中文" },
                                { key: "en", label: "English" }
                            ]

                            delegate: EchoButton {
                                Layout.fillWidth: true
                                text: modelData.label
                                ghost: true
                                selected: uiPrefs.languageMode === modelData.key
                                onClicked: dialog.uiPrefs.languageMode = modelData.key
                            }
                        }
                    }
                }

                // ---- Models pane ----
                ColumnLayout {
                    visible: dialog.selectedIndex === 1
                    Layout.fillWidth: true
                    spacing: 14

                    EchoSectionLabel {
                        Layout.fillWidth: true
                        text: qsTr("Model access")
                        hint: qsTr("ASR models come from the shared HuggingFace "
                                   + "cache; contextual understanding runs on "
                                   + "your local Ollama.")
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

                        EchoTextField {
                            Layout.fillWidth: true
                            text: modelPrefs.modelRoot
                            onEditingFinished: modelPrefs.modelRoot = text
                        }

                        Text {
                            text: qsTr("Python")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                        }

                        EchoTextField {
                            Layout.fillWidth: true
                            text: modelPrefs.python
                            placeholderText: qsTr("python3")
                            onEditingFinished: modelPrefs.python = text
                        }

                        Text {
                            text: qsTr("Worker")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                        }

                        EchoTextField {
                            Layout.fillWidth: true
                            text: modelPrefs.workerScript
                            onEditingFinished: modelPrefs.workerScript = text
                        }

                        Text {
                            text: qsTr("Ollama endpoint")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                        }

                        EchoTextField {
                            Layout.fillWidth: true
                            text: modelPrefs.ollamaEndpoint
                            onEditingFinished: modelPrefs.ollamaEndpoint = text
                        }

                        Text {
                            text: qsTr("Ollama model")
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                        }

                        EchoTextField {
                            Layout.fillWidth: true
                            text: modelPrefs.ollamaModel
                            onEditingFinished: modelPrefs.ollamaModel = text
                        }
                    }
                }
            }
        }
    }
}

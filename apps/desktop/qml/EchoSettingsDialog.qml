//! Centered application settings. Runtime configuration presents only the
//! consumer boundary; physical models and worker plumbing belong to Runtime.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Dialog {
    id: dialog

    property UiPreferences uiPrefs: null
    property InferencePreferences inferencePrefs: null
    property int selectedIndex: 0

    readonly property var sections: [
        { title: qsTr("General"), subtitle: qsTr("Appearance & language"),
          icon: "qrc:/EchoDesktop/icons/tune.svg" },
        { title: qsTr("Inference"), subtitle: qsTr("Infer Runtime consumer"),
          icon: "qrc:/EchoDesktop/icons/mic.svg" }
    ]

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(720, Math.max(620, parent.width - 96))
    height: Math.min(520, Math.max(460, parent.height - 96))
    title: qsTr("Settings")
    modal: true
    dim: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    Overlay.modal: Rectangle {
        color: Theme.effectiveDark ? "#99000000" : "#660f1720"
    }

    background: Rectangle {
        color: Theme.panelRaised
        radius: 14
        border.color: Theme.borderStrong
        border.width: 1
    }

    header: Rectangle {
        implicitHeight: 58
        color: Theme.chrome
        radius: 14

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 15
            color: parent.color
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 1
            color: Theme.border
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 20
            anchors.rightMargin: 20

            Rectangle {
                Layout.preferredWidth: 30
                Layout.preferredHeight: 30
                radius: 9
                color: Theme.accentSurface

                EchoIcon {
                    anchors.centerIn: parent
                    source: "qrc:/EchoDesktop/icons/tune.svg"
                    size: 17
                    color: Theme.accentSelectionText
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                Text {
                    text: qsTr("Echo Settings")
                    color: Theme.textPrimary
                    font.pixelSize: 15
                    font.bold: true
                }

                Text {
                    text: qsTr("Appearance, language, and AI services")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }
        }
    }

    footer: Rectangle {
        implicitHeight: 58
        color: Theme.chrome
        radius: 14

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 15
            color: parent.color
        }

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: 1
            color: Theme.border
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 16
            anchors.rightMargin: 16

            Item { Layout.fillWidth: true }

            EchoButton {
                text: qsTr("Done")
                onClicked: dialog.close()
            }
        }
    }

    contentItem: RowLayout {
        spacing: 0

        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 210
            color: Theme.panel

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 4

                Repeater {
                    model: dialog.sections

                    delegate: Rectangle {
                        required property var modelData
                        required property int index

                        Layout.fillWidth: true
                        Layout.preferredHeight: 58
                        radius: 8
                        color: dialog.selectedIndex === index
                            ? Theme.accentSurface : sectionHover.hovered
                                ? Theme.surfaceSubtle : Theme.transparent

                        HoverHandler { id: sectionHover }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: dialog.selectedIndex = index
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 10
                            spacing: 10

                            EchoIcon {
                                source: modelData.icon
                                size: 18
                                color: dialog.selectedIndex === index
                                    ? Theme.accentSelectionText : Theme.textSecondary
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2

                                Text {
                                    text: modelData.title
                                    color: dialog.selectedIndex === index
                                        ? Theme.accentSelectionText : Theme.textPrimary
                                    font.pixelSize: Theme.fontBody
                                    font.bold: dialog.selectedIndex === index
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: modelData.subtitle
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.fontMeta
                                    elide: Text.ElideRight
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: Theme.panelRaised

            StackLayout {
                anchors.fill: parent
                anchors.margins: 26
                currentIndex: dialog.selectedIndex

                Item {
                    ColumnLayout {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        spacing: 22

                        EchoSectionLabel {
                            Layout.fillWidth: true
                            text: qsTr("Appearance")
                            hint: qsTr("Follow the system, or keep Echo light or dark.")
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
                                    selected: dialog.uiPrefs !== null
                                        && dialog.uiPrefs.mode === modelData.key
                                    onClicked: dialog.uiPrefs.mode = modelData.key
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 1
                            color: Theme.border
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
                                    selected: dialog.uiPrefs !== null
                                        && dialog.uiPrefs.languageMode === modelData.key
                                    onClicked: dialog.uiPrefs.languageMode = modelData.key
                                }
                            }
                        }
                    }
                }

                Item {
                    ColumnLayout {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        spacing: 18

                        EchoSectionLabel {
                            Layout.fillWidth: true
                            text: qsTr("Infer Runtime")
                            hint: qsTr("Echo submits audio understanding intents; Runtime schedules execution and records run history.")
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 7

                            Text {
                                text: qsTr("Runtime endpoint")
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontBody
                            }

                            EchoTextField {
                                Layout.fillWidth: true
                                text: dialog.inferencePrefs !== null
                                    ? dialog.inferencePrefs.runtimeEndpoint : ""
                                placeholderText: qsTr("Automatic (Infra Discovery)")
                                onEditingFinished: {
                                    if (dialog.inferencePrefs !== null) {
                                        dialog.inferencePrefs.runtimeEndpoint = text
                                    }
                                }
                            }
                        }

                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: routeDetails.implicitHeight + 28
                            radius: 10
                            color: Theme.surfaceSubtle
                            border.color: Theme.border

                            ColumnLayout {
                                id: routeDetails
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                anchors.leftMargin: 14
                                anchors.rightMargin: 14
                                spacing: 10

                                Repeater {
                                    model: [
                                        { label: qsTr("Contract"), value: "0.1.0-candidate.4" },
                                        { label: qsTr("Intents"), value: "audio.transcribe · audio.align · audio.detect_events" },
                                        { label: qsTr("Placement"), value: qsTr("Local only") },
                                        { label: qsTr("Consumer credential"),
                                          value: dialog.inferencePrefs !== null
                                              && dialog.inferencePrefs.credentialAvailable
                                              ? qsTr("Available") : qsTr("Missing") }
                                    ]

                                    delegate: RowLayout {
                                        required property var modelData
                                        Layout.fillWidth: true

                                        Text {
                                            text: modelData.label
                                            color: Theme.textSecondary
                                            font.pixelSize: Theme.fontBody
                                        }

                                        Item { Layout.fillWidth: true }

                                        Text {
                                            Layout.maximumWidth: routeDetails.width * 0.65
                                            text: modelData.value
                                            color: Theme.textPrimary
                                            font.pixelSize: Theme.fontBody
                                            elide: Text.ElideMiddle
                                        }
                                    }
                                }
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            Rectangle {
                                Layout.preferredWidth: 8
                                Layout.preferredHeight: 8
                                radius: 4
                                color: dialog.inferencePrefs !== null
                                    && dialog.inferencePrefs.credentialAvailable
                                    ? Theme.accent : Theme.textDisabled
                            }

                            Text {
                                Layout.fillWidth: true
                                text: qsTr("The consumer credential is kept in Echo's private store and is never stored in settings. Endpoint changes take effect after restarting Echo.")
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontBody
                                wrapMode: Text.WordWrap
                            }
                        }
                    }
                }
            }
        }
    }
}

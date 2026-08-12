//! Transient form for saving selected processing modules as a named recipe.
//! The caller receives one save intent and owns persistence and errors.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: dialog

    // Optional entries accept { componentId, title, summary, included,
    // available }. With no model, the current bounded Echo processing set is
    // offered; Space is the only module left unselected by default.
    property var componentModel: []
    property int componentModelRevision: 0
    property string suggestedName: ""

    signal saveRequested(string name, var componentIds)

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: Math.round((parent.height - height) / 2)
    width: Math.min(520, parent.width - 40)
    height: Math.min(570, parent.height - 40)
    padding: 0
    modal: true
    dim: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function defaultComponents(): var {
        return [
            {
                componentId: "lowCut",
                title: qsTr("Low cut"),
                summary: qsTr("Remove low-frequency rumble"),
                included: true
            },
            {
                componentId: "restoration",
                title: qsTr("Restoration"),
                summary: qsTr("De-plosive, noise reduction, and de-essing"),
                included: true
            },
            {
                componentId: "deHum",
                title: qsTr("De-hum"),
                summary: qsTr("Mains hum and harmonics"),
                included: true
            },
            {
                componentId: "deClick",
                title: qsTr("De-click"),
                summary: qsTr("Short impulse repair"),
                included: true
            },
            {
                componentId: "channelRepair",
                title: qsTr("Channel repair"),
                summary: qsTr("Polarity, routing, balance and mono repair"),
                included: true
            },
            {
                componentId: "equalizer",
                title: qsTr("Equalizer"),
                summary: qsTr("Tone shaping"),
                included: true
            },
            {
                componentId: "dynamics",
                title: qsTr("Dynamics"),
                summary: qsTr("Compression and level control"),
                included: true
            },
            {
                componentId: "space",
                title: qsTr("Space"),
                summary: qsTr("Room and ambience"),
                included: false
            },
            {
                componentId: "sceneVfx",
                title: qsTr("Scene VFX"),
                summary: qsTr("Telephone, radio, intercom and scene filters"),
                included: false
            },
            {
                componentId: "delayVfx",
                title: qsTr("Delay VFX"),
                summary: qsTr("Slapback and echo"),
                included: false
            },
            {
                componentId: "modulationVfx",
                title: qsTr("Modulation VFX"),
                summary: qsTr("Chorus, flanger, phaser and tremolo"),
                included: false
            },
            {
                componentId: "transformVfx",
                title: qsTr("Transform VFX"),
                summary: qsTr("Stylized voice roles"),
                included: false
            },
            {
                componentId: "digitalDegradeVfx",
                title: qsTr("Digital Degrade"),
                summary: qsTr("Bit depth and sample-rate character"),
                included: false
            },
            {
                componentId: "driveVfx",
                title: qsTr("Drive"),
                summary: qsTr("Antialiased saturation character"),
                included: false
            },
            {
                componentId: "rotaryVfx",
                title: qsTr("Rotary"),
                summary: qsTr("Dual-rotor motion"),
                included: false
            },
            {
                componentId: "master",
                title: qsTr("Master output"),
                summary: qsTr("Limiter and output policy"),
                included: true
            }
        ];
    }

    function sourceCount(source: var): int {
        if (!source)
            return 0;
        if (source.count !== undefined)
            return source.count;
        return source.length !== undefined ? source.length : 0;
    }

    function sourceEntry(source: var, index: int): var {
        if (source.get !== undefined)
            return source.get(index);
        return source[index];
    }

    function rebuildChoices(): void {
        const source = dialog.sourceCount(dialog.componentModel) > 0 ? dialog.componentModel : dialog.defaultComponents();
        componentProjection.clear();
        for (let index = 0; index < dialog.sourceCount(source); ++index) {
            const entry = dialog.sourceEntry(source, index);
            const componentId = String(entry.componentId || entry.id || "");
            if (componentId.length === 0)
                continue;
            const available = entry.available === undefined ? true : Boolean(entry.available);
            const included = entry.included === undefined ? ["space", "sceneVfx", "delayVfx", "modulationVfx", "transformVfx", "digitalDegradeVfx", "driveVfx", "rotaryVfx"].indexOf(componentId) < 0 : Boolean(entry.included);
            componentProjection.append({
                componentId: componentId,
                title: String(entry.title || componentId),
                summary: String(entry.summary || ""),
                included: available && included,
                available: available
            });
        }
    }

    function selectedComponentIds(): var {
        const result = [];
        for (let index = 0; index < componentProjection.count; ++index) {
            const entry = componentProjection.get(index);
            if (entry.available && entry.included)
                result.push(entry.componentId);
        }
        return result;
    }

    function present(): void {
        recipeName.text = dialog.suggestedName;
        validationText.text = "";
        dialog.rebuildChoices();
        open();
        recipeName.forceActiveFocus();
        recipeName.selectAll();
    }

    onComponentModelChanged: rebuildChoices()
    onComponentModelRevisionChanged: rebuildChoices()

    background: Rectangle {
        radius: Theme.panelRadius
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    ListModel {
        id: componentProjection
    }

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 18
            spacing: 10

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Text {
                    text: qsTr("Save processing recipe")
                    color: Theme.textPrimary
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    Layout.fillWidth: true
                    text: qsTr("Reuse this processing on other sounds")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }
            }

            EchoButton {
                text: qsTr("Close")
                ghost: true
                onClicked: dialog.close()
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 18
            spacing: 10

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Name")
            }

            EchoTextField {
                id: recipeName

                Layout.fillWidth: true
                placeholderText: qsTr("Processing recipe name")
                maximumLength: 80
                onAccepted: confirmButton.clicked()
                onTextChanged: validationText.text = ""
            }

            EchoSectionLabel {
                Layout.fillWidth: true
                text: qsTr("Included processing")
                hint: qsTr("Choose the modules that should travel with this recipe.")
            }

            ListView {
                id: componentList

                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: 190
                model: componentProjection
                spacing: 3
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                activeFocusOnTab: true

                ScrollBar.vertical: ScrollBar {
                    policy: componentList.contentHeight > componentList.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
                }

                delegate: CheckDelegate {
                    id: componentRow

                    required property string componentId
                    required property string title
                    required property string summary
                    required property bool included
                    required property bool available
                    required property int index

                    width: componentList.width
                    height: 45
                    text: componentRow.title
                    checked: componentRow.included
                    enabled: componentRow.available
                    Accessible.description: componentRow.summary
                    onToggled: componentProjection.setProperty(componentRow.index, "included", checked)

                    contentItem: Column {
                        leftPadding: 4
                        spacing: 1

                        Text {
                            width: parent.width
                            text: componentRow.title
                            color: componentRow.available ? Theme.textPrimary : Theme.textDisabled
                            font.pixelSize: Theme.fontBody
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                        }

                        Text {
                            width: parent.width
                            text: componentRow.summary
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: noteText.implicitHeight + 18
                radius: Theme.controlRadius
                color: Theme.surfaceSubtle

                Text {
                    id: noteText

                    anchors.fill: parent
                    anchors.margins: 9
                    text: qsTr("Trim, fades, and clip gain stay with each sound and are never saved in a processing recipe.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                    wrapMode: Text.WordWrap
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Text {
                id: validationText

                Layout.fillWidth: true
                Layout.preferredHeight: 15
                color: Theme.warningText
                font.pixelSize: Theme.fontMeta
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Item {
                    Layout.fillWidth: true
                }

                EchoButton {
                    text: qsTr("Cancel")
                    ghost: true
                    onClicked: dialog.close()
                }

                EchoButton {
                    id: confirmButton

                    text: qsTr("Save recipe")
                    enabled: recipeName.text.trim().length > 0 && dialog.selectedComponentIds().length > 0
                    onClicked: {
                        const name = recipeName.text.trim();
                        const componentIds = dialog.selectedComponentIds();
                        if (name.length === 0) {
                            validationText.text = qsTr("Enter a processing recipe name.");
                            return;
                        }
                        if (componentIds.length === 0) {
                            validationText.text = qsTr("Choose at least one processing module.");
                            return;
                        }
                        dialog.saveRequested(name, componentIds);
                        dialog.close();
                    }
                }
            }
        }
    }
}

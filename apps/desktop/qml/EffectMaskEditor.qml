//! Focused editor for one source-anchored effect mask. It owns eligible
//! insert projection, multi-effect selection, feathering, and the atomic
//! draft mutation; the authored serial chain remains authoritative.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Popup {
    id: editor

    required property var draft

    property int rangeStartMillis: 0
    property int rangeEndMillis: 0
    property int editingIndex: -1
    property int featherMillis: 10

    readonly property bool hasSelection: selectedEffectNodes().length > 0

    parent: Overlay.overlay
    width: Math.min(390, parent.width - 32)
    padding: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    function effectTitle(kind: int): string {
        if (kind === 0)
            return qsTr("Restore");
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
        return qsTr("Effect");
    }

    function selectedEffectNodes(): var {
        const result = [];
        for (let index = 0; index < effectModel.count; ++index) {
            if (effectModel.get(index).selected)
                result.push(Number(effectModel.get(index).kind));
        }
        return result;
    }

    function present(trigger: var, startMillis: int, endMillis: int, maskIndex: int): void {
        rangeStartMillis = startMillis;
        rangeEndMillis = endMillis;
        editingIndex = maskIndex;
        const existing = maskIndex >= 0 && maskIndex < draft.effectMasks.length ? draft.effectMasks[maskIndex] : null;
        featherMillis = existing ? Number(existing.featherMillis) : 10;
        effectModel.clear();
        for (let index = 0; index < draft.effectChain.length; ++index) {
            const kind = Number(draft.effectChain[index]);
            if (kind === 4 || kind === 6 || kind === 11)
                continue;
            effectModel.append({
                kind: kind,
                title: effectTitle(kind),
                selected: existing ? existing.effectNodes.indexOf(kind) >= 0 : false
            });
        }
        const point = trigger.mapToItem(Overlay.overlay, 0, trigger.height);
        x = Math.max(16, Math.min(parent.width - width - 16, point.x + trigger.width / 2 - width / 2));
        y = Math.max(16, Math.min(parent.height - implicitHeight - 16, point.y + 8));
        open();
    }

    function applyMask(): void {
        const nodes = selectedEffectNodes();
        if (nodes.length === 0)
            return;
        draft.beginGesture();
        if (editingIndex >= 0) {
            draft.updateEffectMask(editingIndex, rangeStartMillis, rangeEndMillis, featherMillis, nodes);
        } else {
            draft.addEffectMask(rangeStartMillis, rangeEndMillis, featherMillis, nodes);
        }
        draft.endGesture();
        close();
    }

    function removeMask(): void {
        if (editingIndex < 0)
            return;
        draft.beginGesture();
        draft.removeEffectMask(editingIndex);
        draft.endGesture();
        close();
    }

    background: Rectangle {
        radius: Theme.controlRadius + 2
        color: Theme.panelRaised
        border.width: 1
        border.color: Theme.borderStrong
    }

    ListModel {
        id: effectModel
    }

    contentItem: ColumnLayout {
        spacing: 0

        ColumnLayout {
            Layout.fillWidth: true
            Layout.margins: 16
            spacing: 5

            Text {
                Layout.fillWidth: true
                text: editor.editingIndex >= 0 ? qsTr("Edit effect mask") : qsTr("Add effect mask")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                font.weight: Font.DemiBold
            }

            Text {
                Layout.fillWidth: true
                text: qsTr("Choose one or more effects. They keep their order in the signal chain.")
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

        ColumnLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 12
            Layout.rightMargin: 12
            Layout.topMargin: 8
            Layout.bottomMargin: 6
            spacing: 2

            Repeater {
                model: effectModel

                delegate: CheckBox {
                    required property int index
                    required property int kind
                    required property string title
                    required property bool selected

                    Layout.fillWidth: true
                    objectName: "effectMaskChoice" + index
                    text: title
                    checked: selected
                    onToggled: effectModel.setProperty(index, "selected", checked)
                }
            }

            Text {
                visible: effectModel.count === 0
                Layout.fillWidth: true
                Layout.preferredHeight: 34
                text: qsTr("Add an insert effect to the chain before adding a mask.")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                verticalAlignment: Text.AlignVCenter
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            Layout.topMargin: 8
            Layout.bottomMargin: 8
            spacing: 10

            Text {
                text: qsTr("Soft edge")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
            }

            Slider {
                Layout.fillWidth: true
                from: 0
                to: 100
                stepSize: 1
                value: editor.featherMillis
                Accessible.name: qsTr("Effect mask soft edge")
                onMoved: editor.featherMillis = Math.round(value)
            }

            Text {
                Layout.preferredWidth: 46
                text: editor.featherMillis + " ms"
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
            Layout.margins: 12
            spacing: 8

            EchoButton {
                visible: editor.editingIndex >= 0
                objectName: "removeEffectMaskButton"
                text: qsTr("Remove mask")
                ghost: true
                onClicked: editor.removeMask()
            }

            Item {
                Layout.fillWidth: true
            }

            EchoButton {
                objectName: "cancelEffectMaskButton"
                text: qsTr("Cancel")
                ghost: true
                onClicked: editor.close()
            }

            EchoButton {
                objectName: "confirmEffectMaskButton"
                text: editor.editingIndex >= 0 ? qsTr("Update") : qsTr("Add")
                enabled: editor.hasSelection
                onClicked: editor.applyMask()
            }
        }
    }
}

//! Controlled effect-chain editor. It renders a caller-owned projection and
//! emits intent signals so history, validation, and persistence stay in the
//! authoritative draft.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: chain

    // Each entry has effectId, title, summary, iconSource, enabled, and
    // terminal. Replace the array or bump modelRevision after in-place edits.
    property var effectModel: []
    property int modelRevision: 0
    property string selectedEffectId: ""
    property bool editable: true

    signal effectSelected(string effectId)
    signal effectBypassRequested(string effectId, bool bypassed)
    signal effectDeleteRequested(string effectId)
    signal effectMoveRequested(string effectId, int direction)
    signal addEffectRequested()

    implicitWidth: 220
    implicitHeight: 300
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong
    clip: true

    function modelCount(): int {
        if (!chain.effectModel)
            return 0
        if (chain.effectModel.count !== undefined)
            return chain.effectModel.count
        return chain.effectModel.length !== undefined
            ? chain.effectModel.length : 0
    }

    function modelEntry(index: int): var {
        if (chain.effectModel.get !== undefined)
            return chain.effectModel.get(index)
        return chain.effectModel[index]
    }

    function rebuildProjection(): void {
        chainProjection.clear()
        for (let index = 0; index < chain.modelCount(); ++index) {
            const entry = chain.modelEntry(index)
            if (!entry)
                continue
            chainProjection.append({
                effectId: String(entry.effectId || ""),
                title: String(entry.title || ""),
                summary: String(entry.summary || ""),
                iconSource: String(entry.iconSource || ""),
                nodeEnabled: entry.enabled === undefined
                    ? true : Boolean(entry.enabled),
                terminal: Boolean(entry.terminal)
            })
        }
    }

    readonly property int observedModelCount: modelCount()

    onEffectModelChanged: rebuildProjection()
    onModelRevisionChanged: rebuildProjection()
    onObservedModelCountChanged: rebuildProjection()
    Component.onCompleted: rebuildProjection()

    ListModel { id: chainProjection }

    component ChainAction: Button {
        id: action

        property string accessibleName: ""

        implicitWidth: 19
        implicitHeight: 22
        padding: 0
        focusPolicy: Qt.NoFocus
        Accessible.name: accessibleName

        ToolTip.visible: hovered && accessibleName.length > 0
        ToolTip.text: accessibleName
        ToolTip.delay: 500

        background: Rectangle {
            radius: Theme.compactControlRadius
            color: !action.enabled ? Theme.transparent
                : action.down ? Theme.buttonGhostPressed
                : action.hovered ? Theme.buttonGhostHover : Theme.transparent
        }

        contentItem: Text {
            text: action.text
            color: action.enabled ? Theme.textSecondary : Theme.textDisabled
            font.pixelSize: 12
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 30

            AdjustmentPanelHeader {
                anchors.fill: parent
                title: qsTr("Effect chain")
                iconSource: "qrc:/EchoDesktop/icons/equalizer.svg"
            }

            Button {
                id: addEffectButton

                anchors.right: parent.right
                anchors.rightMargin: 7
                anchors.verticalCenter: parent.verticalCenter
                z: 2
                width: 25
                height: 22
                padding: 0
                focusPolicy: Qt.NoFocus
                enabled: chain.editable
                Accessible.name: qsTr("Add effect")
                onClicked: chain.addEffectRequested()

                ToolTip.visible: hovered
                ToolTip.text: qsTr("Add effect")
                ToolTip.delay: 500

                background: Rectangle {
                    radius: Theme.compactControlRadius
                    color: addEffectButton.down ? Theme.buttonGhostPressed
                        : addEffectButton.hovered ? Theme.buttonGhostHover
                                                  : Theme.surfaceSubtle
                }

                contentItem: Text {
                    text: "+"
                    color: Theme.textSecondary
                    font.pixelSize: 15
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.border
        }

        ListView {
            id: chainList

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 7
            model: chainProjection
            spacing: 4
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true

            ScrollBar.vertical: ScrollBar {
                policy: chainList.contentHeight > chainList.height
                    ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: Rectangle {
                id: effectNode

                required property int index
                required property string effectId
                required property string title
                required property string summary
                required property string iconSource
                required property bool nodeEnabled
                required property bool terminal

                readonly property bool selected:
                    effectId === chain.selectedEffectId
                readonly property bool canMoveUp: chain.editable
                    && !terminal && index > 0
                readonly property bool canMoveDown: chain.editable
                    && !terminal && index + 1 < chainProjection.count
                    && !chainProjection.get(index + 1).terminal

                width: chainList.width - 5
                height: 58
                radius: Theme.compactControlRadius
                color: selected ? Theme.surfaceSelected
                    : nodeHover.hovered ? Theme.surfaceSubtle
                                        : Theme.transparent
                border.width: selected ? 1 : 0
                border.color: selected ? Theme.accent : Theme.transparent

                Rectangle {
                    x: 15
                    y: effectNode.height / 2
                    width: 1
                    height: effectNode.terminal ? 0 : effectNode.height + 5
                    color: Theme.borderStrong
                }

                Rectangle {
                    x: 11
                    y: Math.round((parent.height - height) / 2)
                    width: 9
                    height: 9
                    radius: 5
                    color: effectNode.nodeEnabled ? Theme.accent
                                                  : Theme.panelRaised
                    border.width: 1
                    border.color: effectNode.nodeEnabled ? Theme.accent
                                                         : Theme.textDisabled
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 27
                    anchors.rightMargin: 6
                    spacing: 6

                    EchoIcon {
                        source: effectNode.iconSource
                        size: 16
                        color: effectNode.nodeEnabled ? Theme.textPrimary
                                                      : Theme.textDisabled
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 42
                        spacing: 1

                        Text {
                            Layout.fillWidth: true
                            text: effectNode.title
                            color: effectNode.nodeEnabled ? Theme.textPrimary
                                                          : Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                            font.weight: effectNode.selected
                                ? Font.DemiBold : Font.Normal
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: effectNode.nodeEnabled ? effectNode.summary
                                                        : qsTr("Bypassed")
                            color: Theme.textDisabled
                            font.pixelSize: Theme.fontMeta
                            elide: Text.ElideRight
                        }
                    }

                    RowLayout {
                        visible: effectNode.selected && chain.editable
                        spacing: 0

                        ChainAction {
                            text: "↑"
                            accessibleName: qsTr("Move effect up")
                            enabled: effectNode.canMoveUp
                            onClicked: chain.effectMoveRequested(
                                effectNode.effectId, -1)
                        }

                        ChainAction {
                            text: "↓"
                            accessibleName: qsTr("Move effect down")
                            enabled: effectNode.canMoveDown
                            onClicked: chain.effectMoveRequested(
                                effectNode.effectId, 1)
                        }

                        ChainAction {
                            visible: !effectNode.terminal
                            text: "×"
                            accessibleName: qsTr("Delete effect")
                            onClicked: chain.effectDeleteRequested(
                                effectNode.effectId)
                        }
                    }

                    Rectangle {
                        implicitWidth: 26
                        implicitHeight: 17
                        radius: height / 2
                        opacity: chain.editable ? 1.0 : 0.45
                        color: effectNode.nodeEnabled ? Theme.accent
                                                      : Theme.track

                        Rectangle {
                            width: 11
                            height: 11
                            radius: 6
                            y: 3
                            x: effectNode.nodeEnabled
                                ? parent.width - width - 2 : 2
                            color: effectNode.nodeEnabled ? Theme.accentText
                                                          : Theme.panelRaised
                            Behavior on x {
                                NumberAnimation { duration: 90 }
                            }
                        }

                        TapHandler {
                            enabled: chain.editable
                            onTapped: chain.effectBypassRequested(
                                effectNode.effectId,
                                effectNode.nodeEnabled)
                        }
                    }
                }

                HoverHandler { id: nodeHover }
                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    gesturePolicy: TapHandler.ReleaseWithinBounds
                    onTapped: chain.effectSelected(effectNode.effectId)
                }
            }

            footer: Text {
                width: chainList.width - 12
                leftPadding: 7
                rightPadding: 5
                topPadding: 8
                bottomPadding: 3
                text: qsTr("Signal flows from top to bottom. Master stays last.")
                color: Theme.textDisabled
                font.pixelSize: Theme.fontMeta
                wrapMode: Text.WordWrap
            }
        }
    }
}

//! Complete signal-flow navigation for the adjustment workspace. Clip-level
//! processing and master output stay pinned while authored insert effects
//! scroll independently between them.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: chain

    // Insert entries have effectId, title, summary, iconSource, and enabled.
    // Source/preamp and master are fixed projections owned by this component.
    property var effectModel: []
    property int modelRevision: 0
    property string selectedEffectId: "clip"
    property bool editable: true
    property bool masterEnabled: true

    signal effectSelected(string effectId)
    signal effectBypassRequested(string effectId, bool bypassed)
    signal effectDeleteRequested(string effectId)
    signal effectMoveRequested(string effectId, int direction)
    signal addEffectRequested

    implicitWidth: Theme.editorRailWidth
    implicitHeight: 320
    radius: Theme.compactControlRadius
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong
    clip: true

    function modelCount(): int {
        if (!chain.effectModel)
            return 0;
        if (chain.effectModel.count !== undefined)
            return chain.effectModel.count;
        return chain.effectModel.length !== undefined ? chain.effectModel.length : 0;
    }

    function modelEntry(index: int): var {
        if (chain.effectModel.get !== undefined)
            return chain.effectModel.get(index);
        return chain.effectModel[index];
    }

    function rebuildProjection(): void {
        chainProjection.clear();
        for (let index = 0; index < chain.modelCount(); ++index) {
            const entry = chain.modelEntry(index);
            if (!entry)
                continue;
            chainProjection.append({
                effectId: String(entry.effectId || ""),
                title: String(entry.title || ""),
                summary: String(entry.summary || ""),
                iconSource: String(entry.iconSource || ""),
                nodeEnabled: entry.enabled === undefined ? true : Boolean(entry.enabled)
            });
        }
    }

    readonly property int observedModelCount: modelCount()

    onEffectModelChanged: rebuildProjection()
    onModelRevisionChanged: rebuildProjection()
    onObservedModelCountChanged: rebuildProjection()
    Component.onCompleted: rebuildProjection()

    ListModel {
        id: chainProjection
    }

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
            color: !action.enabled ? Theme.transparent : action.down ? Theme.buttonGhostPressed : action.hovered ? Theme.buttonGhostHover : Theme.transparent
        }

        contentItem: Text {
            text: action.text
            color: action.enabled ? Theme.textSecondary : Theme.textDisabled
            font.pixelSize: 12
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    component PinnedNode: Rectangle {
        id: pinned

        required property string effectId
        required property string title
        required property string summary
        required property string iconSource
        property bool nodeEnabled: true
        property bool toggleVisible: false

        signal activated
        signal toggled

        readonly property bool selected: pinned.effectId === chain.selectedEffectId

        radius: Theme.compactControlRadius
        color: selected ? Theme.surfaceSelected : pinnedHover.hovered ? Theme.surfaceSubtle : Theme.transparent
        border.width: selected ? 1 : 0
        border.color: selected ? Theme.accent : Theme.transparent

        Rectangle {
            x: 12
            anchors.verticalCenter: parent.verticalCenter
            width: 9
            height: 9
            radius: 5
            color: pinned.nodeEnabled ? Theme.accent : Theme.panelRaised
            border.width: 1
            border.color: pinned.nodeEnabled ? Theme.accent : Theme.textDisabled
        }

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 28
            anchors.rightMargin: 7
            spacing: 7

            EchoIcon {
                source: pinned.iconSource
                size: 16
                color: pinned.nodeEnabled ? Theme.textPrimary : Theme.textDisabled
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                Text {
                    Layout.fillWidth: true
                    text: pinned.title
                    color: pinned.nodeEnabled ? Theme.textPrimary : Theme.textSecondary
                    font.pixelSize: Theme.fontBody
                    font.weight: pinned.selected ? Font.DemiBold : Font.Medium
                    elide: Text.ElideRight
                }

                Text {
                    Layout.fillWidth: true
                    text: pinned.nodeEnabled ? pinned.summary : qsTr("Bypassed")
                    color: Theme.textDisabled
                    font.pixelSize: Theme.fontMeta
                    elide: Text.ElideRight
                }
            }

            Rectangle {
                visible: pinned.toggleVisible
                implicitWidth: 27
                implicitHeight: 17
                radius: height / 2
                opacity: chain.editable ? 1.0 : 0.45
                color: pinned.nodeEnabled ? Theme.accent : Theme.track

                Rectangle {
                    width: 11
                    height: 11
                    radius: 6
                    y: 3
                    x: pinned.nodeEnabled ? parent.width - width - 2 : 2
                    color: pinned.nodeEnabled ? Theme.accentText : Theme.panelRaised
                    Behavior on x {
                        NumberAnimation {
                            duration: 90
                        }
                    }
                }

                TapHandler {
                    enabled: chain.editable
                    onTapped: pinned.toggled()
                }
            }
        }

        HoverHandler {
            id: pinnedHover
        }
        TapHandler {
            acceptedButtons: Qt.LeftButton
            gesturePolicy: TapHandler.ReleaseWithinBounds
            onTapped: pinned.activated()
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 32

            AdjustmentPanelHeader {
                anchors.fill: parent
                title: qsTr("Signal chain")
                iconSource: "qrc:/EchoDesktop/icons/equalizer.svg"
            }

            Button {
                id: addEffectButton

                anchors.right: parent.right
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                width: 25
                height: 23
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
                    color: addEffectButton.down ? Theme.buttonGhostPressed : addEffectButton.hovered ? Theme.buttonGhostHover : Theme.surfaceSubtle
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

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 52

            Rectangle {
                x: 16
                anchors.top: parent.verticalCenter
                anchors.bottom: parent.bottom
                width: 1
                color: Theme.borderStrong
            }

            PinnedNode {
                anchors.fill: parent
                anchors.margins: 7
                effectId: "clip"
                title: qsTr("Clip / preamp")
                summary: qsTr("Trim · fades · low cut · gain")
                iconSource: "qrc:/EchoDesktop/icons/crop.svg"
                onActivated: chain.effectSelected(effectId)
            }
        }

        ListView {
            id: chainList

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.leftMargin: 7
            Layout.rightMargin: 5
            model: chainProjection
            spacing: 3
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true

            ScrollBar.vertical: ScrollBar {
                policy: chainList.contentHeight > chainList.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff
            }

            delegate: Rectangle {
                id: effectNode

                required property int index
                required property string effectId
                required property string title
                required property string summary
                required property string iconSource
                required property bool nodeEnabled

                readonly property bool selected: effectId === chain.selectedEffectId
                readonly property bool canMoveUp: chain.editable && index > 0
                readonly property bool canMoveDown: chain.editable && index + 1 < chainProjection.count

                width: chainList.width - 5
                height: 46
                radius: Theme.compactControlRadius
                color: selected ? Theme.surfaceSelected : nodeHover.hovered ? Theme.surfaceSubtle : Theme.transparent
                border.width: selected ? 1 : 0
                border.color: selected ? Theme.accent : Theme.transparent

                Rectangle {
                    x: 9
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    anchors.topMargin: effectNode.index === 0 ? 0 : -3
                    anchors.bottomMargin: -3
                    width: 1
                    color: Theme.borderStrong
                }

                Rectangle {
                    x: 5
                    anchors.verticalCenter: parent.verticalCenter
                    width: 9
                    height: 9
                    radius: 5
                    color: effectNode.nodeEnabled ? Theme.accent : Theme.panelRaised
                    border.width: 1
                    border.color: effectNode.nodeEnabled ? Theme.accent : Theme.textDisabled
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 22
                    anchors.rightMargin: 5
                    spacing: 5

                    Text {
                        text: "⠿"
                        color: Theme.textDisabled
                        font.pixelSize: 12
                    }

                    EchoIcon {
                        source: effectNode.iconSource
                        size: 15
                        color: effectNode.nodeEnabled ? Theme.textPrimary : Theme.textDisabled
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.minimumWidth: 38
                        spacing: 0

                        Text {
                            Layout.fillWidth: true
                            text: effectNode.title
                            color: effectNode.nodeEnabled ? Theme.textPrimary : Theme.textSecondary
                            font.pixelSize: Theme.fontBody
                            font.weight: effectNode.selected ? Font.DemiBold : Font.Normal
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: effectNode.nodeEnabled ? effectNode.summary : qsTr("Bypassed")
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
                            onClicked: chain.effectMoveRequested(effectNode.effectId, -1)
                        }

                        ChainAction {
                            text: "↓"
                            accessibleName: qsTr("Move effect down")
                            enabled: effectNode.canMoveDown
                            onClicked: chain.effectMoveRequested(effectNode.effectId, 1)
                        }

                        ChainAction {
                            text: "×"
                            accessibleName: qsTr("Delete effect")
                            onClicked: chain.effectDeleteRequested(effectNode.effectId)
                        }
                    }

                    Rectangle {
                        implicitWidth: 27
                        implicitHeight: 17
                        radius: height / 2
                        opacity: chain.editable ? 1.0 : 0.45
                        color: effectNode.nodeEnabled ? Theme.accent : Theme.track

                        Rectangle {
                            width: 11
                            height: 11
                            radius: 6
                            y: 3
                            x: effectNode.nodeEnabled ? parent.width - width - 2 : 2
                            color: effectNode.nodeEnabled ? Theme.accentText : Theme.panelRaised
                            Behavior on x {
                                NumberAnimation {
                                    duration: 90
                                }
                            }
                        }

                        TapHandler {
                            enabled: chain.editable
                            onTapped: chain.effectBypassRequested(effectNode.effectId, effectNode.nodeEnabled)
                        }
                    }
                }

                HoverHandler {
                    id: nodeHover
                }
                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    gesturePolicy: TapHandler.ReleaseWithinBounds
                    onTapped: chain.effectSelected(effectNode.effectId)
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.preferredHeight: 52

            Rectangle {
                x: 16
                anchors.top: parent.top
                anchors.bottom: parent.verticalCenter
                width: 1
                color: Theme.borderStrong
            }

            PinnedNode {
                anchors.fill: parent
                anchors.margins: 7
                effectId: "master"
                title: qsTr("Master output")
                summary: qsTr("Limiter · loudness")
                iconSource: "qrc:/EchoDesktop/icons/gain.svg"
                nodeEnabled: chain.masterEnabled
                toggleVisible: true
                onActivated: chain.effectSelected(effectId)
                onToggled: chain.effectBypassRequested(effectId, nodeEnabled)
            }
        }
    }
}

//! Floating toolbar for explicit multi-selection. It never owns selection or
//! single-sound affinity state; the caller supplies the count and handles all
//! resulting intents.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: toolbar

    property int selectedCount: 0
    property bool busy: false

    signal applyRecipeRequested
    signal sequenceAssemblyRequested
    signal layeredAssemblyRequested
    signal clearRequested

    visible: selectedCount > 0
    implicitWidth: operations.implicitWidth + 20
    implicitHeight: 44
    radius: 12
    color: Theme.panelRaised
    border.width: 1
    border.color: Theme.borderStrong

    Accessible.role: Accessible.ToolBar
    Accessible.name: qsTr("Selected sounds actions")

    RowLayout {
        id: operations

        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 10
        anchors.topMargin: 7
        anchors.bottomMargin: 7
        spacing: 6

        Text {
            text: qsTr("%1 sounds selected").arg(toolbar.selectedCount)
            color: Theme.textPrimary
            font.pixelSize: Theme.fontBody
            font.weight: Font.DemiBold
        }

        BusyIndicator {
            visible: toolbar.busy
            running: visible
            Layout.preferredWidth: 18
            Layout.preferredHeight: 18
            Accessible.name: qsTr("Applying processing recipe")
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.preferredHeight: 20
            color: Theme.border
        }

        EchoButton {
            text: qsTr("Sequence")
            ghost: true
            enabled: !toolbar.busy && toolbar.selectedCount > 0
            onClicked: toolbar.sequenceAssemblyRequested()
        }

        EchoButton {
            text: qsTr("Layer")
            ghost: true
            enabled: !toolbar.busy && toolbar.selectedCount > 1 && toolbar.selectedCount <= 8
            onClicked: toolbar.layeredAssemblyRequested()
        }

        EchoButton {
            text: qsTr("Apply recipe")
            ghost: true
            enabled: !toolbar.busy && toolbar.selectedCount > 0
            onClicked: toolbar.applyRecipeRequested()
        }

        EchoButton {
            text: qsTr("Clear selection")
            ghost: true
            enabled: !toolbar.busy && toolbar.selectedCount > 0
            onClicked: toolbar.clearRequested()
        }
    }
}

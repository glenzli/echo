//! Shared centered workspace navigation in both window titlebars.
import QtQuick

Item {
    id: tab
    required property url source
    required property string toolTipText
    property bool selected: false
    signal clicked()
    implicitWidth: 46
    width: implicitWidth
    EchoIconButton {
        anchors.centerIn: parent
        buttonSize: 30
        iconSize: 18
        source: tab.source
        toolTipText: tab.toolTipText
        selected: tab.selected
        onClicked: tab.clicked()
    }
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        width: 24; height: 2; radius: 1
        visible: tab.selected
        color: Theme.accent
    }
}

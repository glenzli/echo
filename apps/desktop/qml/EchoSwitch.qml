//! Compact Echo-native switch. It replaces platform-default toggles so the
//! editing surface keeps one interaction language in light and dark themes.

import QtQuick

Item {
    id: control

    property bool checked: false
    property string accessibleName: ""
    signal toggled(bool checked)

    implicitWidth: 34
    implicitHeight: 20
    opacity: enabled ? 1 : 0.45

    Accessible.role: Accessible.CheckBox
    Accessible.name: accessibleName
    Accessible.checked: checked

    Rectangle {
        anchors.fill: parent
        anchors.margins: 1
        radius: height / 2
        color: control.checked ? Theme.accent : Theme.switchOffSurface
        border.width: control.checked ? 0 : 1
        border.color: Theme.borderStrong

        Behavior on color {
            ColorAnimation {
                duration: 110
            }
        }
    }

    Rectangle {
        width: 16
        height: 16
        radius: 8
        y: 2
        x: control.checked ? control.width - width - 2 : 2
        color: Theme.switchThumb
        border.width: Theme.effectiveDark ? 0 : 1
        border.color: Theme.border

        Behavior on x {
            NumberAnimation {
                duration: 120
                easing.type: Easing.OutCubic
            }
        }
    }

    HoverHandler {
        id: hover
    }
    Rectangle {
        anchors.fill: parent
        radius: height / 2
        visible: hover.hovered
        color: Theme.accent
        opacity: 0.08
    }
    TapHandler {
        enabled: control.enabled
        onTapped: control.toggled(!control.checked)
    }
}

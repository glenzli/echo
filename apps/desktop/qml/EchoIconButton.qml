//! EchoIconButton: icon-first ghost toolbar button with tooltip and optional
//! checkable selection. Mirrors Shadow's ShadowIconButton contract.

import QtQuick
import QtQuick.Controls

Button {
    id: control

    property url source
    property string toolTipText: ""
    property string accessibleName: toolTipText
    property bool selected: checkable && checked
    property color selectedSurfaceColor: Theme.accentSurfaceQuiet
    property color selectedIconColor: Theme.accentSelectionText
    property int iconSize: 18
    property int buttonSize: Theme.compactControlHeight
    property int cornerRadius: Theme.compactControlRadius

    implicitWidth: buttonSize
    implicitHeight: buttonSize
    padding: 0
    focusPolicy: Qt.NoFocus

    ToolTip.visible: hovered && toolTipText.length > 0
    ToolTip.text: toolTipText
    ToolTip.delay: 600

    Accessible.name: accessibleName

    background: Rectangle {
        radius: cornerRadius
        color: {
            if (!control.enabled) {
                return Theme.transparent
            }
            if (control.down || control.checked) {
                return control.selected ? Theme.accentSurface
                                        : Theme.buttonGhostPressed
            }
            if (control.hovered) {
                return control.selected ? Theme.accentSurface : Theme.buttonGhostHover
            }
            return control.selected ? Theme.accentSurfaceQuiet : Theme.transparent
        }
    }

    contentItem: EchoIcon {
        anchors.centerIn: parent
        size: iconSize
        source: control.source
        color: !control.enabled ? Theme.textDisabled
                                : control.selected ? control.selectedIconColor
                                                   : Theme.textSecondary
    }
}

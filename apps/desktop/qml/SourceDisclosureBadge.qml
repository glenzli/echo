//! Visible source identity shared by the library, editor and arrangement.
import QtQuick
import QtQuick.Controls
import "SourceDisclosure.js" as Disclosure
Rectangle {
    id: badge
    property var asset: null
    property bool editable: false
    readonly property bool generated: Disclosure.generated(asset)
    readonly property bool processed: Disclosure.processed(asset)
    signal activated()
    visible: generated || processed || editable
    implicitWidth: label.implicitWidth + 16
    implicitHeight: 24
    radius: 6
    color: generated ? Theme.warningSurface : processed ? Theme.accentSurfaceQuiet : Theme.surfaceSubtle
    Text {
        id: label; anchors.centerIn: parent
        text: badge.generated ? qsTr("AI-generated source") : badge.processed ? qsTr("AI-processed source") : qsTr("Source labels")
        color: badge.generated ? Theme.warningText : badge.processed ? Theme.accent : Theme.textSecondary
        font.pixelSize: Theme.fontMeta; font.weight: Font.Medium
    }
    Accessible.role: editable ? Accessible.Button : Accessible.StaticText
    Accessible.name: label.text
    Accessible.onPressAction: if (editable) activated()
    activeFocusOnTab: editable
    Keys.onReturnPressed: if (editable) activated()
    Keys.onSpacePressed: if (editable) activated()
    border.width: activeFocus ? 1 : 0
    border.color: Theme.accent
    TapHandler { enabled: badge.editable; onTapped: badge.activated() }
    HoverHandler { id: hover }
    ToolTip.visible: hover.hovered
    ToolTip.text: generated ? qsTr("This sound references a source with declared AI-generated content. Labels apply to the source, not an exact mixed interval.") : qsTr("Source declarations may be entered here or carried by an imported file. An unmarked source is not a verified recording.")
    ToolTip.delay: 450
}

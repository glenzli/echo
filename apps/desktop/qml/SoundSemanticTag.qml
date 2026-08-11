//! Shared compact signal chip for mood, event, and detected language.

import QtQuick
import EchoDesktop

Rectangle {
    id: tag

    required property string text
    property string kind: "event"
    property int maximumWidth: 104
    property bool compact: true

    implicitWidth: Math.min(maximumWidth, tagText.implicitWidth + (compact ? 14 : 16))
    implicitHeight: compact ? 20 : 22
    radius: compact ? 7 : 8
    color: kind === "mood" ? Theme.moodSurface(text) : kind === "language" ? Theme.languageSurface : Theme.eventSurface
    visible: text.length > 0

    Accessible.role: Accessible.StaticText
    Accessible.name: text

    Text {
        id: tagText

        anchors.fill: parent
        anchors.leftMargin: tag.compact ? 7 : 8
        anchors.rightMargin: tag.compact ? 7 : 8
        text: tag.text
        color: tag.kind === "mood" ? Theme.moodText(text) : tag.kind === "language" ? Theme.languageText : Theme.eventText
        font.pixelSize: Theme.fontMeta
        font.weight: Font.Medium
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
}

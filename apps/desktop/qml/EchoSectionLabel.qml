//! EchoSectionLabel: section header with an optional hint line.

import QtQuick

Column {
    id: root

    property string text: ""
    property string hint: ""

    spacing: 2

    Text {
        text: root.text
        color: Theme.textPrimary
        font.pixelSize: Theme.fontSection
        font.bold: true
    }

    Text {
        visible: root.hint.length > 0
        text: root.hint
        color: Theme.textSecondary
        font.pixelSize: Theme.fontMeta
        wrapMode: Text.WordWrap
        width: root.width
    }
}

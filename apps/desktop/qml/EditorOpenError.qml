//! Visible terminal error for a project that cannot be opened safely.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop
ApplicationWindow {
    visible: true; width: 540; height: 250; minimumWidth: 420; minimumHeight: 220
    title: qsTr("Echo · Independent editing"); color: Theme.window
    ColumnLayout {
        anchors.fill: parent; anchors.margins: 24; spacing: 18
        Text { text: qsTr("The project could not be opened"); font.pixelSize: 20; font.weight: Font.DemiBold; color: Theme.textPrimary }
        Text { Layout.fillWidth: true; Layout.fillHeight: true; text: startupError; wrapMode: Text.WrapAnywhere; color: Theme.textSecondary }
        EchoButton { Layout.alignment: Qt.AlignRight; text: qsTr("Close"); onClicked: Qt.quit() }
    }
}

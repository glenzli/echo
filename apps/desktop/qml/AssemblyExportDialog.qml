import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
Popup {
    id:dialog
    required property var exporter
    property var revision:null
    property url destination
    parent:Overlay.overlay
    x:Math.round((parent.width-width)/2);y:Math.round((parent.height-height)/2)
    width:Math.min(520,parent.width-32);padding:20;modal:true;dim:true;focus:true
    background:Rectangle { radius:12;color:Theme.panelRaised;border.color:Theme.border }
    onOpened:destination=""
    FileDialog {
        id:fileDialog;title:qsTr("Export assembly mixdown");fileMode:FileDialog.SaveFile
        defaultSuffix:settings.extension;nameFilters:[qsTr("Audio file (*.%1)").arg(settings.extension)]
        onAccepted:dialog.destination=selectedFile
    }
    contentItem:ColumnLayout {
        spacing:14
        Text { text:qsTr("Export assembly mixdown");color:Theme.textPrimary;font.pixelSize:20;font.weight:Font.DemiBold }
        AudioExportSettings { id:settings;Layout.fillWidth:true }
        Text { Layout.fillWidth:true;text:qsTr("Source labels stay embedded in the delivered audio.");wrapMode:Text.WordWrap;color:Theme.textSecondary;font.pixelSize:Theme.fontMeta }
        RowLayout {
            Layout.fillWidth:true
            Text { Layout.fillWidth:true;elide:Text.ElideMiddle;text:dialog.destination.toString() || qsTr("Choose a destination");color:Theme.textSecondary;font.pixelSize:Theme.fontBody }
            EchoButton { text:qsTr("Choose…");ghost:true;onClicked:fileDialog.open() }
        }
        RowLayout {
            Layout.alignment:Qt.AlignRight
            EchoButton { text:qsTr("Cancel");ghost:true;onClicked:dialog.close() }
            EchoButton { objectName:"assemblyStartExport";text:qsTr("Export");enabled:!!dialog.revision && dialog.destination.toString().length>0;onClicked:{dialog.exporter.exportOptions=settings.options;dialog.exporter.exportAssembly(dialog.revision,dialog.destination);dialog.close();} }
        }
    }
}

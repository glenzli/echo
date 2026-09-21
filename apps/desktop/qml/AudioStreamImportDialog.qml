//! Explicit stream selection for manual imports; one-track sources pass through unchanged.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
Popup {
    id:dialog
    property int selectedStream:-1
    signal filesReady(var files)
    parent:Overlay.overlay
    x:Math.round((parent.width-width)/2);y:Math.round((parent.height-height)/2)
    width:Math.min(600,parent.width-32);height:Math.min(240+Math.min(audioStreamImport.streams.length,5)*80,parent.height-32)
    padding:20;modal:true;dim:true;focus:true;closePolicy:Popup.CloseOnEscape
    function present(files) { if(audioStreamImport.busy||audioStreamImport.choosing)return;selectedStream=-1;open();audioStreamImport.inspect(files); }
    onClosed:audioStreamImport.cancel()
    background:Rectangle { radius:12;color:Theme.panelRaised;border.color:Theme.border }
    Connections {
        target:audioStreamImport
        function onFilesReady(files) { if(!dialog.visible)return;const ready=Array.from(files);dialog.close();dialog.filesReady(ready); }
        function onStateChanged() { if(!audioStreamImport.choosing)dialog.selectedStream=-1; }
    }
    contentItem:ColumnLayout {
        spacing:14
        Text { text:qsTr("Choose an audio track");color:Theme.textPrimary;font.pixelSize:20;font.weight:Font.DemiBold }
        Text { Layout.fillWidth:true;text:audioStreamImport.sourceName;elide:Text.ElideMiddle;color:Theme.textSecondary;font.pixelSize:Theme.fontBody }
        Text { Layout.fillWidth:true;text:qsTr("This container has multiple audio tracks. Import one without re-encoding; the complete original container and its source record are preserved.");wrapMode:Text.WordWrap;color:Theme.textMuted;font.pixelSize:Theme.fontMeta;visible:audioStreamImport.choosing }
        ListView {
            id:list;Layout.fillWidth:true;Layout.fillHeight:true;clip:true;spacing:8;model:audioStreamImport.streams
            ScrollBar.vertical:ScrollBar {}
            delegate:ItemDelegate {
                id:row
                required property var modelData
                required property int index
                text:qsTr("Audio track %1").arg(index+1)+(modelData.title?" · "+modelData.title:"")
                width:list.width;height:72;enabled:audioStreamImport.choosing
                onClicked:dialog.selectedStream=modelData.index
                background:Rectangle { radius:8;color:dialog.selectedStream===modelData.index?Theme.surfaceSelected:Theme.surfaceSubtle;border.color:dialog.selectedStream===modelData.index?Theme.accentBorder:Theme.border }
                contentItem:ColumnLayout {
                    spacing:4
                    Text { Layout.fillWidth:true;text:row.text;elide:Text.ElideRight;color:Theme.textPrimary;font.pixelSize:Theme.fontBody }
                    Text { Layout.fillWidth:true;text:[modelData.language || qsTr("Language unspecified"),modelData.codec,qsTr("%1 channels · %2 Hz").arg(modelData.channels).arg(modelData.sampleRate)].join(" · ");elide:Text.ElideRight;color:Theme.textSecondary;font.pixelSize:Theme.fontMeta }
                }
            }
        }
        RowLayout {
            visible:audioStreamImport.busy;Layout.fillWidth:true
            BusyIndicator { running:visible;implicitWidth:24;implicitHeight:24 }
            Text { Layout.fillWidth:true;text:qsTr("Inspecting or preserving the audio source…");wrapMode:Text.WordWrap;color:Theme.textSecondary;font.pixelSize:Theme.fontBody }
        }
        Text { Layout.fillWidth:true;visible:!!audioStreamImport.errorText;text:audioStreamImport.errorText;wrapMode:Text.WordWrap;color:Theme.warningText;font.pixelSize:Theme.fontBody }
        RowLayout {
            Layout.alignment:Qt.AlignRight
            EchoButton { text:qsTr("Cancel");ghost:true;onClicked:dialog.close() }
            EchoButton { objectName:"importSelectedStream";text:qsTr("Import selected track");enabled:audioStreamImport.choosing&&dialog.selectedStream>=0;onClicked:audioStreamImport.select(dialog.selectedStream) }
        }
    }
}

//! On-demand personal context in the listening inspector.
import QtQuick
import QtQuick.Layouts

InspectorSection {
    id: section
    required property var catalogBackend
    required property var asset
    property var info: ({})
    property string errorText: ""
    readonly property bool assembly: !!(asset && asset.assemblyId)
    readonly property string targetId: asset ? String(assembly ? asset.assemblyId : asset.id) : ""
    title: qsTr("MEMORY INFORMATION")
    function refresh(): void {
        const result = targetId ? catalogBackend.memoryInfo(targetId, assembly) : {};
        info = result.info || {}; errorText = result.error || "";
    }
    onTargetIdChanged: refresh()
    Component.onCompleted: refresh()
    Connections { target: section.catalogBackend; function onAssetsChanged() { section.refresh(); } }
    MemoryInfoDialog { id: editor; catalogBackend: section.catalogBackend; onSaved: section.refresh() }
    ColumnLayout {
        Layout.fillWidth: true; spacing: 8
        Text {
            Layout.fillWidth: true; wrapMode: Text.WordWrap; maximumLineCount: 5; elide: Text.ElideRight
            text: section.errorText || section.info.notes || qsTr("Add your own context to this sound.")
            color: section.info.notes ? Theme.textPrimary : Theme.textMuted; font.pixelSize: Theme.fontBody
        }
        Text { visible: !!section.info.place; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: qsTr("Place: %1").arg(section.info.place || ""); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
        Text { visible: !!section.info.timeDescription; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: qsTr("Time: %1").arg(section.info.timeDescription || ""); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
        Text { visible: !!section.info.moments && section.info.moments.length > 0; text: qsTr("%1 moment notes").arg(section.info.moments ? section.info.moments.length : 0); color: Theme.textMuted; font.pixelSize: Theme.fontMeta }
        EchoButton { text: qsTr("Edit memory information"); ghost: true; enabled: !!section.targetId; onClicked: editor.present(section.targetId, section.assembly) }
    }
}

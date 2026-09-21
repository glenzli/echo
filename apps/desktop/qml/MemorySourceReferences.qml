//! Human-readable provenance for one accepted listening edition.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

ColumnLayout {
    id: references
    required property var asset
    property var sources: []
    readonly property var provenance: {
        try { return JSON.parse(asset ? asset.provenanceJson || "{}" : "{}"); }
        catch (error) { return ({}); }
    }
    signal openProjectRequested(string assemblyId)
    spacing: 8
    visible: asset !== null && asset !== undefined && !!asset.assemblyId
    onAssetChanged: { if (visible) sources = backend.listAssets(); }
    onVisibleChanged: { if (visible) sources = backend.listAssets(); }
    Component.onCompleted: { if (visible) sources = backend.listAssets(); }
    EchoSectionLabel { Layout.fillWidth: true; text: qsTr("Sources in this memory") }
    Text {
        Layout.fillWidth: true
        text: qsTr("Listening edition · project v%1").arg(references.provenance.assemblyRevisionNumber || 1)
        color: Theme.textSecondary; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap
    }
    Repeater {
        model: references.provenance.sources || []
        ColumnLayout {
            required property var modelData
            readonly property var source: references.sources.find(asset => asset.id === modelData.assetId)
            Layout.fillWidth: true
            spacing: 3
            Text {
                Layout.fillWidth: true
                text: source ? (source.soundCaption || source.sourceTitle || source.path.split("/").pop()) : qsTr("Unavailable source")
                color: Theme.textPrimary; font.pixelSize: Theme.fontBody; elide: Text.ElideRight
                HoverHandler { id: sourceHover }
                ToolTip.visible: sourceHover.hovered
                ToolTip.text: modelData.originalContentHash || ""
            }
            SourceDisclosureBadge { asset: source || null }
            Text {
                Layout.fillWidth: true
                text: (modelData.sourceRole === "material" ? qsTr("Material") : qsTr("Memory")) + " · " + (modelData.adjustmentRevisionId ? qsTr("Source version %1").arg(modelData.adjustmentRevisionId) : qsTr("Original source")) + " · " + (modelData.sourceStartMillis/1000).toFixed(1) + "–" + (modelData.sourceEndMillis/1000).toFixed(1) + qsTr(" s")
                color: Theme.textMuted; font.pixelSize: Theme.fontMeta; wrapMode: Text.WordWrap
            }
        }
    }
    EchoButton {
        text: qsTr("Continue editing project")
        onClicked: references.openProjectRequested(references.asset.assemblyId)
    }
}

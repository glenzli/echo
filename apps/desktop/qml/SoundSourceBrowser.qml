//! Editor source bin and global material collection. Search, audition, intake and
//! collection controls share this owner; project placement belongs to the editor.
import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: browser
    property bool editorMode: false
    property string assemblyId: ""
    property var projectDocument: ({})
    property var assets: []
    property var projectIds: []
    property var selectedAsset: null
    property int sourceTab: editorMode ? 0 : 2
    property string query: ""
    property string category: ""
    property string eventFilter: ""
    property string notice: ""
    property string auditionId: ""
    readonly property var categories: ["", "music", "ambience", "effects", "voice"]
    readonly property var categoryLabels: [qsTr("All materials"), qsTr("Music"), qsTr("Ambience"), qsTr("Sound effects"), qsTr("Voice")]
    readonly property var filteredAssets: filterAssets()
    readonly property var eventTypes: {
        let values = [];
        for (const asset of assets) {
            if (asset.inMaterials && asset.eventType && values.indexOf(asset.eventType) < 0)
                values.push(asset.eventType);
        }
        return [qsTr("All sound events")].concat(values.sort());
    }
    signal addRequested(var asset, string role)
    signal openAssemblyRequested(string assemblyId)
    color: Theme.panel
    border.color: Theme.border

    function titleFor(asset: var): string {
        return asset ? (asset.soundCaption || asset.sourceTitle || asset.path.split("/").pop()) : "";
    }
    function roleFor(asset: var): string {
        return sourceTab === 2 || (sourceTab === 0 && !asset.inMemory) ? "material" : "memory";
    }
    function refresh(): void {
        const selectedId = selectedAsset ? selectedAsset.id : "";
        assets = backend.listAssets();
        projectIds = assemblyId ? backend.projectMaterials(assemblyId) : [];
        selectedAsset = assets.find(asset => asset.id === selectedId) || null;
    }
    function filterAssets(): var {
        let ids = projectIds.slice();
        for (const track of projectDocument.tracks || []) {
            for (const clip of track.clips)
                ids.push(clip.assetId);
        }
        const terms = query.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
        return assets.filter(asset => {
            if (sourceTab === 0 && ids.indexOf(asset.id) < 0) return false;
            if (sourceTab === 1 && !asset.inMemory) return false;
            if (sourceTab === 2 && !asset.inMaterials) return false;
            if (category && asset.materialCategory !== category) return false;
            if (eventFilter && asset.eventType !== eventFilter) return false;
            const haystack = [titleFor(asset),asset.path,asset.summary,asset.eventType,asset.mood,(asset.keywords || []).join(" ")].join(" ").toLocaleLowerCase();
            return terms.every(term => haystack.indexOf(term) >= 0);
        });
    }
    function setMembership(memory: bool, materials: bool, newCategory: string): void {
        if (!selectedAsset) return;
        notice = backend.setSoundMembership(selectedAsset.id,memory,materials,newCategory);
        if (!notice) refresh();
    }
    function addSource(asset: var): void {
        if (!asset || asset.pathStatus !== "present") return;
        if (asset.assemblyId) { openAssemblyRequested(asset.assemblyId); return; }
        addRequested(asset,roleFor(asset));
    }
    function audition(asset: var): void {
        if (auditionId === asset.id && materialPlayer.active) {
            materialPlayer.togglePause(); return;
        }
        materialPlayer.stop();
        selectedAsset = asset;
        auditionId = asset.id;
        if (player.playing) player.togglePause();
        Qt.callLater(() => {
            if (browser.auditionId === asset.id && browser.visible) savedSound.play();
        });
    }
    onAssemblyIdChanged: refresh()
    onVisibleChanged: { if (!visible) { materialPlayer.stop(); auditionId = ""; } else refresh(); }
    onSourceTabChanged: { category = ""; eventFilter = ""; }
    Component.onCompleted: refresh()
    Connections { target: backend; function onAssetsChanged(): void { browser.refresh(); } }
    SoundPlaybackSource { id: savedSound; asset: browser.selectedAsset; transport: materialPlayer }

    component SourceTab: TabButton {
        id: tabControl
        contentItem: Text {
            text: tabControl.text
            color: tabControl.checked ? Theme.accentSelectionText : Theme.textSecondary
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: Theme.fontBody
        }
        background: Rectangle {
            radius: Theme.compactControlRadius
            color: tabControl.checked ? Theme.accentSurface : tabControl.hovered ? Theme.buttonGhostHover : Theme.transparent
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: browser.editorMode ? 12 : 24
        spacing: 10
        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: browser.editorMode ? qsTr("Add to your project") : qsTr("Materials")
                color: Theme.textPrimary
                font.pixelSize: browser.editorMode ? 14 : 24
                font.weight: Font.DemiBold
            }
            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/plus.svg"
                toolTipText: qsTr("Import audio materials")
                onClicked: importDialog.open()
            }
        }
        Text {
            visible: !browser.editorMode
            Layout.fillWidth: true
            text: qsTr("Find reusable sounds by name, keywords or AI sound events. Used materials stay available to their projects.")
            color: Theme.textMuted
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
        }
        TabBar {
            visible: browser.editorMode
            Layout.fillWidth: true
            Layout.maximumHeight: Theme.controlHeight
            currentIndex: browser.sourceTab
            onCurrentIndexChanged: browser.sourceTab = currentIndex
            SourceTab { text: qsTr("Project") }
            SourceTab { text: qsTr("Memories") }
            SourceTab { text: qsTr("Materials") }
        }
        EchoTextField {
            id: searchField
            Layout.fillWidth: true
            placeholderText: qsTr("Search sounds and keywords")
            text: browser.query
            onTextChanged: browser.query = text
        }
        RowLayout {
            visible: browser.sourceTab === 2
            Layout.fillWidth: true
            Layout.maximumHeight: Theme.controlHeight
            EchoComboBox {
                Layout.fillWidth: true
                visible: browser.sourceTab === 2
                model: browser.categoryLabels
                currentIndex: browser.categories.indexOf(browser.category)
                onActivated: browser.category = browser.categories[currentIndex]
            }
            EchoComboBox {
                Layout.fillWidth: true
                visible: browser.sourceTab === 2 && !browser.editorMode
                model: browser.eventTypes
                onActivated: browser.eventFilter = currentIndex === 0 ? "" : currentText
            }
        }
        CheckBox {
            id: collectGlobally
            enabled: browser.assemblyId.length > 0
            Layout.maximumHeight: Theme.controlHeight
            visible: browser.editorMode
            checked: true
            text: qsTr("Keep imports in global materials")
        }
        Text {
            Layout.fillWidth: true
            visible: browser.notice.length > 0
            text: browser.notice
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMeta
            wrapMode: Text.WordWrap
        }
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 0
            Layout.preferredHeight: 0
            clip: true
            spacing: 6
            model: browser.filteredAssets
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                id: sourceRow
                required property var modelData
                readonly property var asset: modelData
                width: ListView.view.width
                height: browser.editorMode ? 94 : 80
                radius: 8
                color: browser.selectedAsset && browser.selectedAsset.id === modelData.id ? Theme.surfaceSelected : rowHover.hovered ? Theme.buttonGhostHover : Theme.surfaceSubtle
                border.color: browser.selectedAsset && browser.selectedAsset.id === modelData.id ? Theme.accentBorder : Theme.border
                HoverHandler { id: rowHover }
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 8
                    EchoIconButton {
                        source: materialPlayer.playing && browser.auditionId === sourceRow.asset.id ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                        toolTipText: qsTr("Audition sound")
                        enabled: sourceRow.asset.pathStatus === "present"
                        onClicked: browser.audition(sourceRow.asset)
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Text { Layout.fillWidth: true; text: browser.titleFor(sourceRow.asset); color: Theme.textPrimary; font.pixelSize: Theme.fontBody; elide: Text.ElideRight }
                        Text {
                            Layout.fillWidth: true
                            text: (Number(sourceRow.asset.durationMillis)/1000).toFixed(1) + qsTr(" s") + " · " + (sourceRow.asset.assemblyId ? qsTr("Saved mix") : Number(sourceRow.asset.adjustmentRevision) > 0 ? qsTr("Adjusted recording") : qsTr("Original recording"))
                            color: Theme.textMuted; font.pixelSize: Theme.fontMeta; elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: !!sourceRow.asset.eventType || !!sourceRow.asset.materialCategory
                            text: (sourceRow.asset.materialCategory ? browser.categoryLabels[browser.categories.indexOf(sourceRow.asset.materialCategory)] : "") + (sourceRow.asset.eventType ? " · " + sourceRow.asset.eventType : "")
                            color: Theme.textSecondary; font.pixelSize: Theme.fontMeta; elide: Text.ElideRight
                        }
                        TapHandler { onTapped: browser.selectedAsset = sourceRow.asset; onDoubleTapped: { if (browser.editorMode) browser.addSource(sourceRow.asset); } }
                        DragHandler {
                            id: dragHandler
                            enabled: browser.editorMode && !sourceRow.asset.assemblyId
                            target: dragToken
                            onActiveChanged: {
                                if (active) dragToken.Drag.active = true;
                                else {
                                    dragToken.Drag.drop();
                                    dragToken.x = 0;
                                    dragToken.y = 0;
                                }
                            }
                        }
                        Item {
                            id: dragToken
                            Drag.keys: ["echo-sound"]
                            Drag.source: sourceRow
                            Drag.supportedActions: Qt.CopyAction
                            Drag.hotSpot.x: 0
                            Drag.hotSpot.y: 0
                            onVisibleChanged: { x = 0; y = 0; }
                        }
                    }
                    EchoIconButton {
                        visible: browser.editorMode
                        source: sourceRow.asset.assemblyId ? "qrc:/EchoDesktop/icons/edit.svg" : "qrc:/EchoDesktop/icons/plus.svg"
                        toolTipText: sourceRow.asset.assemblyId ? qsTr("Open source project") : qsTr("Add at playhead")
                        enabled: sourceRow.asset.pathStatus === "present" && (!sourceRow.asset.assemblyId || !!browser.assemblyId)
                        onClicked: browser.addSource(sourceRow.asset)
                    }
                    EchoIconButton {
                        source: "qrc:/EchoDesktop/icons/more-horizontal.svg"
                        toolTipText: qsTr("Collection and category")
                        onClicked: { browser.selectedAsset = sourceRow.asset; sourceMenu.popup(); }
                    }
                }
            }
            Text {
                anchors.centerIn: parent
                width: parent.width - 20
                visible: browser.filteredAssets.length === 0
                text: browser.query ? qsTr("No matching sounds") : browser.sourceTab === 0 ? qsTr("Add memories or import materials to this project.") : qsTr("No sounds in this collection yet.")
                color: Theme.textMuted
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
            }
        }
        Text {
            Layout.fillWidth: true
            text: qsTr("%1 sounds").arg(browser.filteredAssets.length)
            color: Theme.textMuted
            font.pixelSize: Theme.fontMeta
        }
    }
    Menu {
        id: sourceMenu
        MenuItem {
            text: browser.selectedAsset && browser.selectedAsset.inMemory ? qsTr("Remove from memory library") : qsTr("Keep in memory library")
            onTriggered: browser.setMembership(!browser.selectedAsset.inMemory,browser.selectedAsset.inMaterials,browser.selectedAsset.materialCategory)
        }
        MenuItem {
            text: browser.selectedAsset && browser.selectedAsset.inMaterials ? qsTr("Remove from global materials") : qsTr("Keep in global materials")
            onTriggered: browser.setMembership(browser.selectedAsset.inMemory,!browser.selectedAsset.inMaterials,browser.selectedAsset.materialCategory)
        }
        MenuSeparator {}
        Repeater {
            model: browser.categoryLabels
            MenuItem {
                required property int index
                required property string modelData
                text: index === 0 ? qsTr("Uncategorized") : modelData
                checkable: true
                checked: browser.selectedAsset && browser.selectedAsset.materialCategory === browser.categories[index]
                onTriggered: browser.setMembership(browser.selectedAsset.inMemory,browser.selectedAsset.inMaterials,browser.categories[index])
            }
        }
    }
    FileDialog {
        id: importDialog
        title: qsTr("Import audio materials")
        fileMode: FileDialog.OpenFiles
        nameFilters: [qsTr("Audio files (*.wav *.mp3 *.m4a *.aac *.flac *.ogg *.aiff *.aif *.caf)"), qsTr("All files (*)")]
        onAccepted: {
            const failures = [];
            for (const file of selectedFiles) {
                const message = backend.importMaterial(file,browser.assemblyId,!browser.editorMode || !browser.assemblyId || collectGlobally.checked,browser.category);
                if (message) failures.push(message);
            }
            browser.notice = failures.length ? failures.join("\n") : qsTr("Import queued. Sounds appear here when ready.");
        }
    }
}

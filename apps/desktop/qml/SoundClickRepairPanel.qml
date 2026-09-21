//! Deterministic Original-time findings; only explicit acceptance changes the draft.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: panel
    required property var asset
    required property var draft
    required property var controller
    required property real rangeStart
    required property real rangeEnd
    property var selectedIndices: []
    property string notice: ""
    readonly property string identity: asset ? [asset.id,asset.path,asset.pathStatus,draft.trimStartMillis,draft.trimEndMillis,draft.deClickSensitivityPercent,draft.deClickMaximumClickMicroseconds].join(":") : ""
    readonly property bool hasResult: controller.resultKey === identity && identity.length > 0
    readonly property var candidates: hasResult ? controller.result.items || [] : []
    readonly property var selectedEntries: selectedIndices.filter(i=>i>=0 && i<candidates.length).map(i=>candidates[i])
    readonly property bool canScan: asset !== null && asset.pathStatus !== "missing" && rangeEnd>rangeStart && rangeEnd-rangeStart<=300000
    signal rangeRequested(real start, real end, bool play, bool original)
    signal repairRequested(var candidates)
    color: Theme.panel; radius: 8; border.color: Theme.border

    function toggle(index: int): void { selectedIndices=selectedIndices.indexOf(index)<0 ? selectedIndices.concat([index]) : selectedIndices.filter(i=>i!==index); notice=""; }
    function selectAll(): void { selectedIndices=candidates.map((c,i)=>i); notice=""; }
    function scan(): void {
        selectedIndices=[]; notice="";
        controller.scan(asset.path,Math.round(rangeStart),Math.round(rangeEnd),draft.deClickSensitivityPercent,draft.deClickMaximumClickMicroseconds,identity);
    }
    function locate(candidate: var, play: bool, original: bool): void {
        rangeRequested(Math.max(draft.trimStartMillis,candidate.startMillis-250),Math.min(draft.trimEndMillis,candidate.endMillis+250),play,original);
    }
    function applySelected(): void { if (selectedEntries.length) repairRequested(selectedEntries); }
    function finishEdit(result: var): void {
        notice = result.ok ? (result.changed ? qsTr("Local repairs added to the draft. Audition adjusted audio or undo to compare.") : qsTr("These ranges already have local click repair."))
            : result.error === "global" ? qsTr("Click repair already covers the whole recording. Review its settings in Effects.")
            : result.error === "bypassed" ? qsTr("Existing local click repairs are bypassed. Enable or review them in Effects first.")
            : result.error === "amount" ? qsTr("Repair amount is zero. Increase it before adding local repairs.")
            : result.error === "limit" ? qsTr("This edit would exceed 64 effect ranges. Select fewer findings.")
            : result.error === "chain" ? qsTr("The effect chain is full. Remove an effect before adding click repair.")
            : qsTr("These findings are no longer available. Scan the original again.");
    }
    function channels(mask: int): string { const values=[]; for(let i=0;i<8;++i)if(mask & (1<<i))values.push(i+1); return values.join(", "); }
    onIdentityChanged: { controller.cancel(); selectedIndices=[]; notice=""; }
    onVisibleChanged: if (!visible) { controller.cancel(); selectedIndices=[]; notice=""; }
    Connections { target: panel.controller; function onStateChanged(): void { if(!panel.hasResult)panel.selectedIndices=[]; } }

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 12; spacing: 8
        RowLayout {
            Layout.fillWidth: true; spacing: 8
            Label { text: qsTr("Short click repair"); font.weight: Font.DemiBold; font.pixelSize: Theme.fontSection }
            Label { Layout.fillWidth: true; text: qsTr("Original audio · deterministic detection"); color: Theme.textMuted; elide: Text.ElideRight; font.pixelSize: Theme.fontMeta }
            EchoButton {
                objectName: "scanClicks"; Layout.preferredWidth: 174
                text: controller.running ? (controller.cancelling ? qsTr("Cancelling…") : qsTr("Cancel scan")) : qsTr("Scan selection")
                enabled: !controller.cancelling && (controller.running || panel.canScan)
                onClicked: controller.running ? controller.cancel() : panel.scan()
            }
        }
        RowLayout {
            Layout.fillWidth: true; spacing: 8
            Label { text: qsTr("Sensitivity (%)"); color: Theme.textSecondary }
            EchoValueSpinBox { objectName: "clickSensitivity"; from:0; to:100; stepSize:5; value:draft.deClickSensitivityPercent; onValueModified: draft.setDeClickParameter("sensitivity",value) }
            Label { text: qsTr("Maximum width (µs)"); color: Theme.textSecondary }
            EchoValueSpinBox { from:50; to:2000; stepSize:50; value:draft.deClickMaximumClickMicroseconds; onValueModified: draft.setDeClickParameter("maximumClick",value) }
            Label { text: qsTr("Repair (%)"); color: Theme.textSecondary }
            EchoValueSpinBox { from:0; to:100; stepSize:5; value:draft.deClickRepairPercent; onValueModified: draft.setDeClickParameter("repair",value) }
            Item { Layout.fillWidth: true }
        }
        Label {
            Layout.fillWidth:true; wrapMode:Text.Wrap; color:Theme.textSecondary; font.pixelSize:Theme.fontMeta
            text: controller.running ? qsTr("Checking the original audio for short discontinuities…") : controller.errorText || panel.notice || qsTr("Percussion can resemble clicks. Audition each finding with surrounding sound before repairing; the original stays intact.")
        }
        Label {
            Layout.fillWidth:true; visible:panel.hasResult; color:Theme.textMuted; font.pixelSize:Theme.fontMeta
            text: qsTr("Checked %1–%2 s · %3 findings").arg((controller.result.startMillis/1000).toFixed(2)).arg((controller.result.endMillis/1000).toFixed(2)).arg(controller.result.total)
                + (controller.result.total>panel.candidates.length ? qsTr(" · showing the first %1; scan a smaller range for the rest").arg(panel.candidates.length) : "")
        }
        ListView {
            objectName:"clickRows"; Layout.fillWidth:true; Layout.fillHeight:true; clip:true; spacing:4; reuseItems:true
            model:panel.candidates
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                id: row
                required property int index
                required property var modelData
                readonly property bool checked: panel.selectedIndices.indexOf(index)>=0
                width:ListView.view.width; height:44; radius:5
                color:checked ? Theme.accentSurfaceQuiet : Theme.panelRaised
                border.color:checked ? Theme.accent : Theme.transparent
                RowLayout {
                    anchors.fill:parent; anchors.margins:6; spacing:10
                    EchoCheckBox { objectName:"clickCheck-"+row.index; checked:row.checked; onToggled:panel.toggle(row.index); Accessible.name:qsTr("Select finding at %1 s").arg((row.modelData.startMillis/1000).toFixed(3)) }
                    Label { text:(row.modelData.startMillis/1000).toFixed(3)+" s"; font.family:"Menlo"; font.pixelSize:Theme.fontMeta; color:Theme.textPrimary; Layout.preferredWidth:100 }
                    Label { Layout.fillWidth:true; text:qsTr("Channels %1 · %2 ms").arg(panel.channels(row.modelData.channelMask)).arg(row.modelData.endMillis-row.modelData.startMillis); color:Theme.textSecondary; elide:Text.ElideRight }
                    EchoButton { text:qsTr("Locate"); ghost:true; onClicked:panel.locate(row.modelData,false,true) }
                    EchoButton { objectName:"clickAudition-"+row.index; text:qsTr("Audition source"); ghost:true; onClicked:panel.locate(row.modelData,true,true) }
                }
            }
            Label {
                anchors.centerIn:parent; width:Math.max(0,parent.width-32); horizontalAlignment:Text.AlignHCenter; wrapMode:Text.Wrap; color:Theme.textMuted
                visible:!panel.candidates.length && !controller.running
                text:panel.hasResult ? qsTr("No short clicks found with these settings.") : qsTr("Select up to five minutes on the timeline, then scan the original audio.")
            }
        }
        RowLayout {
            Layout.fillWidth:true; spacing:8
            EchoButton { objectName:"selectClicks"; text:qsTr("Select findings"); ghost:true; enabled:panel.candidates.length>0; onClicked:panel.selectAll() }
            EchoButton { text:qsTr("Clear"); ghost:true; enabled:panel.selectedEntries.length>0; onClicked:panel.selectedIndices=[] }
            Label { Layout.fillWidth:true; text:qsTr("%1 selected").arg(panel.selectedEntries.length); color:Theme.textSecondary }
            EchoButton { objectName:"auditionRepairedClick"; text:qsTr("Audition adjusted"); ghost:true; enabled:panel.selectedEntries.length>0; onClicked:panel.locate(panel.selectedEntries[0],true,false) }
            EchoButton { objectName:"repairClicks"; text:qsTr("Repair selected"); enabled:panel.selectedEntries.length>0; onClicked:panel.applySelected() }
        }
    }
}

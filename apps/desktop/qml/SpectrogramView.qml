//! Source-frequency selection, repair inspector and diagnostic listening.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop
import "SpectralEditing.js" as Editing

Rectangle {
    id: spectrogram
    property string scaleImageUrl: ""
    required property string imageUrl
    required property bool loading
    required property int sourceDurationMillis
    required property real viewStartRatio
    required property real viewEndRatio
    required property real progress
    required property bool hasTimeSelection
    required property real selectionStartRatio
    required property real selectionEndRatio
    required property bool layerEnabled
    required property var regions
    required property bool canCreateRenderedWorkingCopy
    required property bool hasRenderedWorkingCopy
    required property bool renderedWorkingCopyRunning
    required property bool renderedWorkingCopyReady
    required property string renderedWorkingCopyError
    required property bool renderedEraseMode
    required property int renderedWorkingCopyOperationCount
    property int lowHertz: 20
    property int highHertz: 24000
    property bool logarithmic: true
    property int windowFrames: 8192
    property int floorDecibels: -96
    property int selectedIndex: -1
    property var selection: null
    property bool showPostEffects: false
    property bool noiseTools: false
    property var noiseSettings: null
    property bool noiseLearning: false
    property int noiseLearningError: 0
    property bool noiseAvailable: true
    readonly property int noiseStartMillis: selection ? selection.startMillis : (hasTimeSelection ? Math.round(selectionStartRatio*sourceDurationMillis) : -1)
    readonly property int noiseEndMillis: selection ? selection.endMillis : (hasTimeSelection ? Math.round(selectionEndRatio*sourceDurationMillis) : -1)
    signal noiseCaptureRequested(int startMillis, int endMillis)
    signal noiseCancelRequested()
    signal noiseEdited(string key, var value)
    signal noiseClearRequested()
    signal noiseAuditionRequested(bool residue)
    readonly property bool hasOverview: imageUrl.length>0
    readonly property bool hasSelection: selection!==null
    signal viewportChanged()
    signal regionUpdated(int index, var value)
    signal regionsAppended(var values)
    signal regionRemoved(int index)
    signal gestureStarted()
    signal gestureFinished()
    signal auditionRequested(var selection, bool bandOnly)
    signal layerEnabledRequested(bool enabled)
    signal clearRequested()
    signal renderedWorkingCopyRequested()
    signal renderedEraseModeRequested(bool enabled)
    signal renderedEraseRequested(int startMillis, int endMillis, int lowHertz, int highHertz)
    signal renderedWorkingCopyAuditionRequested()
    color: Theme.panelRaised
    radius: Theme.panelRadius
    border.color: Theme.border
    implicitHeight: 490
    clip: true

    function resetSelection() { selectedIndex=-1; selection=null; lowHertz=20; highHertz=24000; }
    function setSelection(value,index) { selectedIndex=index; selection=value; }
    function editField(key,value) {
        if (!selection) return;
        const next=Object.assign({},selection); next[key]=value;
        selection=Editing.region(next,sourceDurationMillis);
        if(selectedIndex>=0) regionUpdated(selectedIndex,selection);
    }
    function applySelection() {
        if (!selection || regions.length>=64 || renderedEraseMode) return;
        const index=regions.length;
        regionsAppended([selection]); selectedIndex=index;
    }
    function addHarmonics(count) {
        if (!selection || renderedEraseMode) return;
        const baseIndex=regions.findIndex(value => value.startMillis===selection.startMillis && value.endMillis===selection.endMillis && value.lowHertz===selection.lowHertz && value.highHertz===selection.highHertz);
        const values=Editing.harmonics(selection,count,sourceDurationMillis).filter(value => !regions.some(existing =>
            existing.startMillis===value.startMillis && existing.endMillis===value.endMillis && existing.lowHertz===value.lowHertz && existing.highHertz===value.highHertz));
        if(selectedIndex<0 && baseIndex<0) values.unshift(selection);
        if(values.length+regions.length>64) return;
        const index=baseIndex>=0 ? baseIndex : regions.length;
        if(values.length>0) regionsAppended(values);
        if(selectedIndex<0 && index<regions.length) setSelection(regions[index],index);
    }
    function focusFrequency() {
        if(!selection) return;
        const padding=Math.max(20,(selection.highHertz-selection.lowHertz)*0.5);
        lowHertz=Math.max(20,Math.floor(selection.lowHertz-padding)); highHertz=Math.min(24000,Math.ceil(selection.highHertz+padding));
    }
    onRegionsChanged: { if(selectedIndex>=regions.length) {selectedIndex=-1; selection=null;} else if(selectedIndex>=0) selection=regions[selectedIndex]; }
    onLowHertzChanged: viewportChanged()
    onHighHertzChanged: viewportChanged()
    onLogarithmicChanged: viewportChanged()
    onWindowFramesChanged: viewportChanged()
    onFloorDecibelsChanged: viewportChanged()
    onViewStartRatioChanged: viewportChanged()
    onViewEndRatioChanged: viewportChanged()
    onRenderedEraseModeChanged: { selectedIndex=-1; selection=null; }

    ColumnLayout {
        anchors.fill: parent; anchors.margins: 10; spacing: 8
        RowLayout {
            Layout.fillWidth: true; spacing: 8
            Text { text: qsTr("Spectral repair"); color: Theme.textPrimary; font.pixelSize: 14; font.bold: true }
            EchoSwitch { checked: spectrogram.layerEnabled; accessibleName: qsTr("Spectral adjustment"); onToggled: value => spectrogram.layerEnabledRequested(value) }
            Text { text: spectrogram.renderedEraseMode ? qsTr("Rendered working copy") : qsTr("Original source"); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
            Text { text: qsTr("%1 / 64 repairs").arg(spectrogram.regions.length); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
            Button { text: qsTr("New selection"); onClicked: spectrogram.setSelection(null,-1) }
            Button { text: qsTr("Clear repairs"); enabled: spectrogram.regions.length>0; onClicked: spectrogram.clearRequested() }
        }
        RowLayout {
            Layout.fillWidth: true; spacing: 8
            EchoComboBox { Layout.preferredWidth: 118; selectionIndex: spectrogram.logarithmic ? 0 : 1; model: [qsTr("Log frequency"),qsTr("Linear frequency")]; onActivated: index => spectrogram.logarithmic=index===0 }
            EchoComboBox { Layout.preferredWidth: 146; selectionIndex: spectrogram.windowFrames===8192 ? 0 : 1; model: [qsTr("Tonal detail"),qsTr("Transient detail")]; onActivated: index => spectrogram.windowFrames=index===0 ? 8192 : 2048 }
            Button { text: qsTr("Low frequencies"); onClicked: {spectrogram.lowHertz=20; spectrogram.highHertz=1000;} }
            Button { text: qsTr("Focus selection"); enabled: spectrogram.hasSelection; onClicked: spectrogram.focusFrequency() }
            Button { text: qsTr("Full range"); onClicked: {spectrogram.lowHertz=20; spectrogram.highHertz=24000;} }
            Item { Layout.fillWidth: true }
            EchoParameterSlider {
                Layout.preferredWidth: 222; label: qsTr("Display floor"); labelWidth: 78; valueWidth: 56
                from: -120; to: -48; stepSize: 6; value: spectrogram.floorDecibels; valueText: value+' dB'
                onEdited: value => spectrogram.floorDecibels=Math.round(value)
            }
        }
        RowLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 10
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true; spacing: 5
                SpectralRepairSurface {
                    id: surface; objectName: 'spectralSurface'
                    Layout.fillWidth: true; Layout.fillHeight: true
                    imageUrl: spectrogram.imageUrl; durationMillis: spectrogram.sourceDurationMillis
                    startRatio: spectrogram.viewStartRatio; endRatio: spectrogram.viewEndRatio
                    lowHertz: spectrogram.lowHertz; highHertz: spectrogram.highHertz; logarithmic: spectrogram.logarithmic
                    regions: spectrogram.regions; selection: spectrogram.selection; selectedIndex: spectrogram.selectedIndex
                    layerEnabled: spectrogram.layerEnabled; progress: spectrogram.progress; eraseMode: spectrogram.renderedEraseMode
                    onSelectionChangedByUser: (value,index) => spectrogram.setSelection(value,index)
                    onRegionEdited: (index,value) => spectrogram.regionUpdated(index,value)
                    onEraseRequested: value => spectrogram.renderedEraseRequested(value.startMillis,value.endMillis,value.lowHertz,value.highHertz)
                    Text { anchors.centerIn: parent; visible: !spectrogram.hasOverview; text: spectrogram.loading ? qsTr("Analyzing visible frequencies…") : qsTr("Spectrogram unavailable"); color: '#c0c6d3'; font.pixelSize: Theme.fontBody }
                }
                RowLayout {
                    Layout.fillWidth: true
                    Text { Layout.fillWidth: true; text: qsTr("Drag to select · drag edges to resize · Alt-drag for a new selection"); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta; elide: Text.ElideRight }
                    Text { text: surface.hovered ? (surface.hoverMillis/1000).toFixed(3)+' s · '+Math.round(surface.hoverHertz)+' Hz' : spectrogram.lowHertz+'–'+spectrogram.highHertz+' Hz'; color: Theme.textSecondary; font.family: 'Menlo'; font.pixelSize: Theme.fontMeta }
                }
            }
            ScrollView {
                Layout.preferredWidth: 276; Layout.fillHeight: true; clip: true
                contentWidth: availableWidth
                ColumnLayout {
                    width: parent.width; spacing: 8
                    EchoSegmentedControl {
                        Layout.fillWidth: true; model: [qsTr("Selection"),qsTr("Noise reduction")]
                        currentIndex: spectrogram.noiseTools ? 1 : 0
                        onActivated: index => spectrogram.noiseTools=index===1
                    }
                    NoiseReductionPanel {
                        Layout.fillWidth: true; visible: spectrogram.noiseTools
                        profile: spectrogram.noiseSettings; running: spectrogram.noiseLearning; errorCode: spectrogram.noiseLearningError
                        captureStartMillis: spectrogram.noiseStartMillis; captureEndMillis: spectrogram.noiseEndMillis
                        available: spectrogram.noiseAvailable && !spectrogram.renderedEraseMode
                        onCaptureRequested: spectrogram.noiseCaptureRequested(captureStartMillis,captureEndMillis)
                        onCancelRequested: spectrogram.noiseCancelRequested()
                        onProfileEdited: (key,value) => spectrogram.noiseEdited(key,value)
                        onClearRequested: spectrogram.noiseClearRequested()
                        onGestureStarted: spectrogram.gestureStarted(); onGestureFinished: spectrogram.gestureFinished()
                    }
                    ColumnLayout {
                        Layout.fillWidth: true; visible: !spectrogram.noiseTools; spacing: 8
                    EchoComboBox {
                        objectName: 'spectralRegionPicker'; Layout.fillWidth: true
                        selectionIndex: spectrogram.selectedIndex+1
                        model: [qsTr("New selection")].concat(spectrogram.regions.map((value,index) => qsTr("Repair %1").arg(index+1)+' · '+value.lowHertz+'–'+value.highHertz+' Hz'))
                        onActivated: index => spectrogram.setSelection(index>0 ? spectrogram.regions[index-1] : null,index-1)
                    }
                    GridLayout {
                        Layout.fillWidth: true; columns: 2; columnSpacing: 10; rowSpacing: 6
                        enabled: spectrogram.hasSelection && !spectrogram.renderedEraseMode
                        Text { text: qsTr("Start (s)"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                        EchoTimeSpinBox { objectName: 'spectralStart'; Layout.fillWidth: true; to: spectrogram.hasSelection ? spectrogram.selection.endMillis-1 : 0; value: spectrogram.hasSelection ? spectrogram.selection.startMillis : 0; onValueModified: spectrogram.editField('startMillis',value) }
                        Text { text: qsTr("End (s)"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                        EchoTimeSpinBox { Layout.fillWidth: true; from: spectrogram.hasSelection ? spectrogram.selection.startMillis+1 : 0; to: spectrogram.sourceDurationMillis; value: spectrogram.hasSelection ? spectrogram.selection.endMillis : 0; onValueModified: spectrogram.editField('endMillis',value) }
                        Text { text: qsTr("Low (Hz)"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                        SpinBox { objectName: 'spectralLow'; Layout.fillWidth: true; editable: true; from: 20; to: spectrogram.hasSelection ? spectrogram.selection.highHertz-1 : 24000; value: spectrogram.hasSelection ? spectrogram.selection.lowHertz : 20; onValueModified: spectrogram.editField('lowHertz',value) }
                        Text { text: qsTr("High (Hz)"); color: Theme.textSecondary; font.pixelSize: Theme.fontBody }
                        SpinBox { Layout.fillWidth: true; editable: true; from: spectrogram.hasSelection ? spectrogram.selection.lowHertz+1 : 21; to: 24000; value: spectrogram.hasSelection ? spectrogram.selection.highHertz : 24000; onValueModified: spectrogram.editField('highHertz',value) }
                    }
                    EchoParameterSlider {
                        objectName: 'spectralReduction'; Layout.fillWidth: true; enabled: spectrogram.hasSelection && !spectrogram.renderedEraseMode
                        label: qsTr("Reduction"); labelWidth: 70; valueWidth: 56; from: 0; to: 9600; stepSize: 100
                        value: spectrogram.hasSelection ? spectrogram.selection.attenuationCentibels : 2400; valueText: '−'+(value/100).toFixed(0)+' dB'
                        onGestureStarted: spectrogram.gestureStarted(); onGestureFinished: spectrogram.gestureFinished(); onEdited: value => spectrogram.editField('attenuationCentibels',value)
                    }
                    EchoParameterSlider {
                        Layout.fillWidth: true; enabled: spectrogram.hasSelection && !spectrogram.renderedEraseMode
                        label: qsTr("Time feather"); labelWidth: 88; valueWidth: 48; from: 0; to: 250; stepSize: 1
                        value: spectrogram.hasSelection ? spectrogram.selection.timeFeatherMillis : 24; valueText: value+' ms'
                        onGestureStarted: spectrogram.gestureStarted(); onGestureFinished: spectrogram.gestureFinished(); onEdited: value => spectrogram.editField('timeFeatherMillis',value)
                    }
                    EchoParameterSlider {
                        Layout.fillWidth: true; enabled: spectrogram.hasSelection && !spectrogram.renderedEraseMode
                        label: qsTr("Frequency feather"); labelWidth: 112; valueWidth: 48; from: 0; to: 2000; stepSize: 5
                        value: spectrogram.hasSelection ? spectrogram.selection.frequencyFeatherHertz : 80; valueText: value+' Hz'
                        onGestureStarted: spectrogram.gestureStarted(); onGestureFinished: spectrogram.gestureFinished(); onEdited: value => spectrogram.editField('frequencyFeatherHertz',value)
                    }
                    Button { objectName: 'applySpectralSelection'; Layout.fillWidth: true; text: qsTr("Attenuate selection"); enabled: spectrogram.hasSelection && spectrogram.selectedIndex<0 && spectrogram.regions.length<64 && !spectrogram.renderedEraseMode; onClicked: spectrogram.applySelection() }
                    RowLayout {
                        Layout.fillWidth: true
                        Button { Layout.fillWidth: true; text: qsTr("Add harmonics"); enabled: spectrogram.hasSelection && spectrogram.regions.length<64 && !spectrogram.renderedEraseMode; onClicked: spectrogram.addHarmonics(harmonicCount.value) }
                        SpinBox { id: harmonicCount; from: 2; to: 8; value: 4; editable: true; Layout.preferredWidth: 105 }
                    }
                    Button { Layout.fillWidth: true; text: qsTr("Delete selected repair"); enabled: spectrogram.selectedIndex>=0 && !spectrogram.renderedEraseMode; onClicked: spectrogram.regionRemoved(spectrogram.selectedIndex) }
                    }
                }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            visible: spectrogram.scaleImageUrl.length>0
            spacing: 8
            Text { text: spectrogram.floorDecibels+' dB'; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
            Image { source: spectrogram.scaleImageUrl; Layout.preferredWidth: 160; Layout.preferredHeight: 8; smooth: true }
            Text { text: '0 dB'; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
            Item { Layout.fillWidth: true }
        }
        RowLayout {
            Layout.fillWidth: true
                    Button { visible: !spectrogram.noiseTools; text: qsTr("Listen to source band"); enabled: spectrogram.hasSelection && !spectrogram.renderedEraseMode; onClicked: spectrogram.auditionRequested(spectrogram.selection,true) }
                    Button { visible: !spectrogram.noiseTools; text: qsTr("Audition repaired range"); enabled: spectrogram.hasSelection && !spectrogram.renderedEraseMode; onClicked: spectrogram.auditionRequested(spectrogram.selection,false) }
            Button { visible: spectrogram.noiseTools; text: qsTr("Listen to removed sound"); enabled: spectrogram.noiseSettings!==null && !spectrogram.renderedEraseMode; onClicked: spectrogram.noiseAuditionRequested(true) }
            Button { visible: spectrogram.noiseTools; text: qsTr("Audition reduction"); enabled: spectrogram.noiseSettings!==null && spectrogram.noiseSettings.enabled && spectrogram.layerEnabled && !spectrogram.renderedEraseMode; onClicked: spectrogram.noiseAuditionRequested(false) }
            Button { text: spectrogram.showPostEffects ? qsTr("Hide post-effect tools") : qsTr("Post-effect repair"); onClicked: spectrogram.showPostEffects=!spectrogram.showPostEffects }
            Text { Layout.fillWidth: true; text: spectrogram.regions.length>=64 ? qsTr("Repair limit reached. Edit or remove an existing region.") : qsTr("Repairs keep the original intact. The image shows the source; audition to compare."); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta; elide: Text.ElideRight }
        }
        RowLayout {
            visible: spectrogram.showPostEffects; Layout.fillWidth: true
            Button { text: spectrogram.renderedWorkingCopyRunning ? qsTr("Freezing render…") : (spectrogram.hasRenderedWorkingCopy ? qsTr("Freeze new working copy") : qsTr("Create working copy")); enabled: spectrogram.canCreateRenderedWorkingCopy && !spectrogram.renderedWorkingCopyRunning && !spectrogram.renderedWorkingCopyReady; onClicked: spectrogram.renderedWorkingCopyRequested() }
            Switch { visible: spectrogram.renderedWorkingCopyReady; text: qsTr("Erase mode"); checked: spectrogram.renderedEraseMode; enabled: !spectrogram.renderedWorkingCopyRunning; onClicked: spectrogram.renderedEraseModeRequested(checked) }
            Button { visible: spectrogram.renderedWorkingCopyReady; text: qsTr("Audition rendered"); enabled: !spectrogram.renderedWorkingCopyRunning; onClicked: spectrogram.renderedWorkingCopyAuditionRequested() }
            Text { Layout.fillWidth: true; text: spectrogram.renderedWorkingCopyError.length>0 ? spectrogram.renderedWorkingCopyError : (spectrogram.renderedWorkingCopyReady ? qsTr("%1 committed repairs. Upstream changes require a new copy.").arg(spectrogram.renderedWorkingCopyOperationCount) : qsTr("Save adjustments before creating a post-effect repair copy.")); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta; elide: Text.ElideRight }
        }
    }
}

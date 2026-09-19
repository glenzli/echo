//! Explicit capture/enable lifecycle and bounded learned-noise parameters.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

ColumnLayout {
    id: panel
    required property var profile
    required property bool running
    required property int errorCode
    required property int captureStartMillis
    required property int captureEndMillis
    property bool available: true
    readonly property bool hasProfile: profile!==null && profile!==undefined
    readonly property bool validSelection: captureStartMillis>=0 && captureEndMillis-captureStartMillis>=100 && captureEndMillis-captureStartMillis<=30000
    signal captureRequested()
    signal cancelRequested()
    signal profileEdited(string key, var value)
    signal clearRequested()
    signal gestureStarted()
    signal gestureFinished()
    spacing: 10

    Text { Layout.fillWidth: true; text: qsTr("Learn a noise sample"); color: Theme.textPrimary; font.bold: true; font.pixelSize: Theme.fontBody }
    Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: qsTr("Select 0.1–30 seconds containing only steady background noise. The sample uses the full frequency range."); color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
    Text { Layout.fillWidth: true; text: panel.validSelection ? qsTr("Selected: %1–%2 s").arg((panel.captureStartMillis/1000).toFixed(3)).arg((panel.captureEndMillis/1000).toFixed(3)) : qsTr("Select a time range in the waveform or spectrogram."); wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
    Button { objectName: 'captureNoise'; Layout.fillWidth: true; text: panel.running ? qsTr("Learning noise…") : qsTr("Capture noise sample"); enabled: panel.available && panel.validSelection && !panel.running; onClicked: panel.captureRequested() }
    Button { Layout.fillWidth: true; visible: panel.running; text: qsTr("Cancel"); onClicked: panel.cancelRequested() }
    Text { Layout.fillWidth: true; visible: panel.errorCode!==0; text: panel.errorCode===3 ? qsTr("Edits changed while learning. Capture the sample again.") : panel.errorCode===1 ? qsTr("Select 0.1–30 seconds of available source audio.") : qsTr("Could not learn this sample. Choose an audible noise-only interval and try again."); wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
    Rectangle { Layout.fillWidth: true; height: 1; color: Theme.border }
    RowLayout {
        Layout.fillWidth: true
        Text { Layout.fillWidth: true; text: qsTr("Apply noise reduction"); color: Theme.textPrimary; font.pixelSize: Theme.fontBody }
        EchoSwitch { objectName: 'enableNoiseProfile'; enabled: panel.hasProfile; checked: panel.hasProfile && panel.profile.enabled; accessibleName: qsTr("Apply noise reduction"); onToggled: value => panel.profileEdited('enabled',value) }
    }
    Text { Layout.fillWidth: true; text: panel.hasProfile ? qsTr("Sample: %1–%2 s · saved with this version").arg((panel.profile.captureStartMillis/1000).toFixed(3)).arg((panel.profile.captureEndMillis/1000).toFixed(3)) : qsTr("Capture a sample first. Learning alone does not change the sound."); wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
    EchoParameterSlider {
        objectName: 'noiseReductionDepth'; Layout.fillWidth: true; enabled: panel.hasProfile
        label: qsTr("Reduction"); labelWidth: 78; valueWidth: 56; from: 0; to: 3600; stepSize: 100
        value: panel.hasProfile ? panel.profile.reductionCentibels : 1200; valueText: '−'+(value/100).toFixed(0)+' dB'
        onGestureStarted: panel.gestureStarted(); onGestureFinished: panel.gestureFinished(); onEdited: value => panel.profileEdited('reductionCentibels',Math.round(value))
    }
    EchoParameterSlider {
        Layout.fillWidth: true; enabled: panel.hasProfile; label: qsTr("Sensitivity"); labelWidth: 78; valueWidth: 56; from: 0; to: 1200; stepSize: 100
        value: panel.hasProfile ? panel.profile.sensitivityCentibels : 600; valueText: '+'+(value/100).toFixed(0)+' dB'
        onGestureStarted: panel.gestureStarted(); onGestureFinished: panel.gestureFinished(); onEdited: value => panel.profileEdited('sensitivityCentibels',Math.round(value))
    }
    EchoParameterSlider {
        Layout.fillWidth: true; enabled: panel.hasProfile; label: qsTr("Smoothing"); labelWidth: 78; valueWidth: 56; from: 0; to: 8; stepSize: 1
        value: panel.hasProfile ? panel.profile.smoothingBins : 3; valueText: qsTr("%1 bins").arg(Math.round(value))
        onGestureStarted: panel.gestureStarted(); onGestureFinished: panel.gestureFinished(); onEdited: value => panel.profileEdited('smoothingBins',Math.round(value))
    }
    Text { Layout.fillWidth: true; text: qsTr("Applies to the whole source. Listen to the removed sound; recognizable speech means the sample or reduction needs adjusting."); wrapMode: Text.WordWrap; color: Theme.textSecondary; font.pixelSize: Theme.fontMeta }
    Button { objectName: 'clearNoiseProfile'; Layout.fillWidth: true; enabled: panel.hasProfile && !panel.running; text: qsTr("Clear noise sample"); onClicked: panel.clearRequested() }
}

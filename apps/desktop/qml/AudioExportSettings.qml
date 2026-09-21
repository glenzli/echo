//! Shared delivery specification for single-source, assembly and batch exports.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
ColumnLayout {
    id: settings
    spacing: 10
    readonly property string formatKey: formats[formatBox.currentIndex].key
    readonly property bool compressed: formatKey==="mp3" || formatKey==="aac_m4a"
    property int sampleRateIndex:1
    onCompressedChanged: if (compressed && sampleRateIndex>1) sampleRateIndex=1
    readonly property string extension: formatKey==="flac24" ? "flac" : formatKey==="mp3" ? "mp3" : formatKey==="aac_m4a" ? "m4a" : "wav"
    readonly property string formatLabel: formats[formatBox.currentIndex].label
    property bool includeMemoryInfo: false
    readonly property var options: ({includeMemoryInfo:includeMemoryInfo,format:formatKey,sampleRate:Number(rate.currentText)*1000,channels:channels.currentIndex+1,bitrateKbps:Number(bitrate.currentText)})
    readonly property var formats: [
        {key:"wav_pcm24",label:qsTr("WAV · 24-bit PCM")},
        {key:"wav_pcm16",label:qsTr("WAV · 16-bit PCM")},
        {key:"wav_float32",label:qsTr("WAV · 32-bit float")},
        {key:"flac24",label:qsTr("FLAC · 24-bit lossless")},
        {key:"mp3",label:qsTr("MP3 · Compressed")},
        {key:"aac_m4a",label:qsTr("M4A · AAC compressed")}
    ]
    function configure(value) {
        includeMemoryInfo = value.includeMemoryInfo === true;
        formatBox.currentIndex = Math.max(0, formats.findIndex(item => item.key === value.format));
        sampleRateIndex = value.sampleRate === 44100 ? 0 : value.sampleRate === 96000 && !compressed ? 2 : 1;
        rate.currentIndex = sampleRateIndex;
        channels.currentIndex = value.channels === 1 ? 0 : 1;
        bitrate.currentIndex = Math.max(0, [128,192,256,320].indexOf(value.bitrateKbps || 192));
    }
    Text { text:qsTr("Format"); color:Theme.textSecondary; font.pixelSize:Theme.fontMeta }
    EchoComboBox { id:formatBox; objectName:"exportFormat"; Layout.fillWidth:true; model:settings.formats; textRole:"label" }
    RowLayout {
        Layout.fillWidth:true; spacing:10
        ColumnLayout {
            Layout.fillWidth:true
            Text { text:qsTr("Sample rate (kHz)"); color:Theme.textSecondary; font.pixelSize:Theme.fontMeta }
            EchoComboBox { id:rate; objectName:"exportSampleRate"; Layout.fillWidth:true; model:settings.compressed ? ["44.1","48"] : ["44.1","48","96"]; currentIndex:1;selectionIndex:settings.sampleRateIndex;onActivated:settings.sampleRateIndex=currentIndex }
        }
        ColumnLayout {
            Layout.fillWidth:true
            Text { text:qsTr("Channels"); color:Theme.textSecondary; font.pixelSize:Theme.fontMeta }
            EchoComboBox { id:channels; objectName:"exportChannels"; Layout.fillWidth:true; model:[qsTr("Mono"),qsTr("Stereo")]; currentIndex:1 }
        }
        ColumnLayout {
            visible:settings.compressed; Layout.fillWidth:true
            Text { text:qsTr("Bitrate (kbps)"); color:Theme.textSecondary; font.pixelSize:Theme.fontMeta }
            EchoComboBox { id:bitrate; objectName:"exportBitrate"; Layout.fillWidth:true; model:["128","192","256","320"]; currentIndex:1 }
        }
    }
    Text {
        Layout.fillWidth:true; wrapMode:Text.WordWrap; color:Theme.textMuted; font.pixelSize:Theme.fontMeta
        text:qsTr("Effects are processed at 48 kHz, then converted for delivery. WAV automatically uses RF64 for large files.")
    }
    EchoCheckBox {
        objectName: "exportMemoryInfo"
        text: qsTr("Include memory information")
        checked: settings.includeMemoryInfo
        onToggled: settings.includeMemoryInfo = checked
    }
    Text {
        Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Theme.textMuted; font.pixelSize: Theme.fontMeta
        text: settings.formatKey === "flac24"
            ? qsTr("Writes notes, place and time into comments, plus a FLAC location tag. Moment notes stay in Echo.")
            : qsTr("Writes notes, place and time into the file’s comment tag. Moment notes stay in Echo.")
    }
    Text {
        visible:settings.compressed; Layout.fillWidth:true; wrapMode:Text.WordWrap; color:Theme.textMuted; font.pixelSize:Theme.fontMeta
        text:qsTr("Compressed exports lose detail. Reported loudness is measured before compression.")
    }
}

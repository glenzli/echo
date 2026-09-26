//! Shared entry for separate generated material types and their review dialogs.
import QtQuick
import QtQuick.Controls
Menu {
    id: menu
    property string assemblyId: ""
    signal materialAccepted(string assetId)
    MenuItem {
        objectName: "generateSoundEffectAction"
        text: qsTr("Sound effect or background…")
        enabled: !generatedSoundMaterial.running && !generatedSoundMaterial.accepting
        onTriggered: soundDialog.present("stable_audio_3_small_sfx")
    }
    MenuItem {
        objectName: "generateMusicAction"
        text: qsTr("Music background…")
        enabled: !generatedSoundMaterial.running && !generatedSoundMaterial.accepting
        onTriggered: soundDialog.present("stable_audio_3_small_music")
    }
    MenuItem {
        objectName: "generateNarrationAction"
        text: qsTr("Narration…")
        enabled: !generatedNarration.running && !generatedNarration.accepting
        onTriggered: narrationDialog.present()
    }
    MenuItem {
        visible: generatedSoundMaterial.running || generatedNarration.running
        enabled: false
        text: qsTr("A local generation is still finishing…")
    }
    GeneratedSoundMaterialDialog { id: soundDialog; assemblyId: menu.assemblyId; onMaterialAccepted: assetId => menu.materialAccepted(assetId) }
    GeneratedNarrationDialog { id: narrationDialog; assemblyId: menu.assemblyId; onMaterialAccepted: assetId => menu.materialAccepted(assetId) }
}

pragma Singleton

//! User-facing normalization for model-authored semantic evidence. Raw wire
//! values stay stable in the Catalog; cards and inspectors share this bounded
//! presentation vocabulary.

import QtQuick

QtObject {
    function languageLabel(value) {
        const normalized = String(value || "").trim().toLocaleLowerCase().replace("_", "-");
        const primary = normalized.split("-")[0];
        switch (primary) {
        case "zh":
        case "cmn":
        case "yue":
            return qsTr("Chinese");
        case "en":
            return qsTr("English");
        case "ja":
            return qsTr("Japanese");
        case "ko":
            return qsTr("Korean");
        case "fr":
            return qsTr("French");
        case "de":
            return qsTr("German");
        case "es":
            return qsTr("Spanish");
        default:
            return normalized.length > 0 ? normalized.toLocaleUpperCase() : "";
        }
    }

    // Stable editing identity: analysis may enrich a source, but must not rename it.
    function sourceTitle(asset) {
        if (!asset)
            return "";
        const calibrated = asset.calibratedFields || [];
        if ((asset.assemblyId || calibrated.indexOf("sound_caption") >= 0) && asset.soundCaption)
            return asset.soundCaption;
        return asset.sourceTitle || String(asset.path || "").split("/").pop() || asset.soundCaption || "";
    }

    function eventLabel(value) {
        const normalized = String(value || "").trim().toLocaleLowerCase().replace(/[_-]+/g, " ");
        switch (normalized) {
        case "soundtrack":
            return qsTr("Soundtrack");
        case "nature":
            return qsTr("Nature");
        case "silence":
            return qsTr("Quiet");
        case "daily routine":
            return qsTr("Daily life");
        case "conversation":
            return qsTr("Conversation");
        case "rainfall":
            return qsTr("Rain");
        case "recollection":
            return qsTr("Recollection");
        case "description":
            return "";
        default:
            return String(value || "").trim();
        }
    }
}

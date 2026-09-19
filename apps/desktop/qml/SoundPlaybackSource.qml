//! Plays the exact saved processing projected on a sound summary. Collection
//! listening and editor-bin audition use independent transports with one mapping.
import QtQuick
import EchoDesktop

QtObject {
    id: source
    required property var asset
    required property var transport
    property QtObject draft: SoundAdjustmentDraft { asset: source.asset }
    function play(): void {
        if (!asset || asset.pathStatus !== "present") return;
        const d = draft;
        transport.playAdjusted(asset.path,d.trimStartMillis,d.trimEndMillis,d.fadeInMillis,d.fadeOutMillis,d.fadeInCurve,d.fadeOutCurve,d.gainCentibels,d.lowCutHertz,d.restorationValue(),d.deHumValue(),d.deClickValue(),d.channelRepairValue(),d.equalizerEnabled,d.equalizerBands,d.compressorEnabled,d.compressorThresholdCentibels,d.compressorRatioTenths,d.compressorAttackMillis,d.compressorReleaseMillis,d.compressorMakeupCentibels,d.reverbValue(),d.limiterEnabled,d.limiterCeilingCentibels,d.limiterReleaseMillis,d.effectChain,d.editSegments,d.effectMasks,d.creativeVfxValue(),d.spectralRepairValue());
    }
}

//! Dedicated sound-adjustment workspace. It coordinates immutable-source
//! playback, draft audition, selection looping, and explicit publication;
//! timeline gestures and draft history remain in their semantic owners.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import EchoDesktop

Rectangle {
    id: workspace

    required property var asset

    property var waveformLevels: []
    property string loadedPath: ""
    property string loadedBaseAdjustmentKey: ""
    property string loadedAdjustmentKey: ""
    property bool auditionOriginal: false
    property var processingRecipes: []
    property int processingRecipeModelRevision: 0
    property var processingHistory: []
    property int processingHistoryModelRevision: 0
    property string processingRecipeNotice: ""
    property string lastProcessingRecipeBatchId: ""

    readonly property bool hasAsset: asset !== null && asset !== undefined
    readonly property bool dirty: adjustmentDraft.dirty
    readonly property bool canUndo: adjustmentDraft.canUndo
    readonly property bool canRedo: adjustmentDraft.canRedo

    color: Theme.window

    function fileName(path: string): string {
        const normalized = path.replace(/\\/g, "/");
        return normalized.substring(normalized.lastIndexOf("/") + 1);
    }

    function formatDuration(millis: int): string {
        const safeMillis = Math.max(0, millis);
        const totalSeconds = Math.floor(safeMillis / 1000);
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = totalSeconds % 60;
        const tenths = Math.floor((safeMillis % 1000) / 100);
        if (hours > 0) {
            return hours + ":" + String(minutes).padStart(2, "0") + ":" + String(seconds).padStart(2, "0") + "." + tenths;
        }
        return minutes + ":" + String(seconds).padStart(2, "0") + "." + tenths;
    }

    function technicalDetails(): string {
        if (!hasAsset)
            return "";
        const values = [];
        if (asset.codec.length > 0)
            values.push(asset.codec.toUpperCase());
        if (asset.sampleRate > 0) {
            values.push((asset.sampleRate / 1000).toFixed(asset.sampleRate % 1000 === 0 ? 0 : 1) + " kHz");
        }
        if (asset.channelCount > 0) {
            values.push(qsTr("%1 channels").arg(asset.channelCount));
        }
        values.push(formatDuration(asset.durationMillis));
        return values.join(" · ");
    }

    function playbackBaseAdjustmentKey(): string {
        const prefix = auditionOriginal ? "original" : "adjusted";
        return prefix + ":" + adjustmentDraft.trimStartMillis + ":" + adjustmentDraft.trimEndMillis + ":" + (auditionOriginal ? 0 : adjustmentDraft.fadeInMillis) + ":" + (auditionOriginal ? 0 : adjustmentDraft.fadeOutMillis) + ":" + (auditionOriginal ? 0 : adjustmentDraft.fadeInCurve) + ":" + (auditionOriginal ? 0 : adjustmentDraft.fadeOutCurve) + ":" + (auditionOriginal ? 0 : adjustmentDraft.gainCentibels) + ":" + (auditionOriginal ? 0 : adjustmentDraft.lowCutHertz);
    }

    function adjustmentKey(): string {
        return playbackBaseAdjustmentKey() + ":" + JSON.stringify(auditionOriginal ? {
            enabled: false,
            dePlosiveEnabled: false,
            dePlosiveFrequencyHertz: 140,
            dePlosiveSensitivityPercent: 50,
            dePlosiveReductionCentibels: 1200,
            dePlosiveReleaseMillis: 160,
            noiseEnabled: false,
            noiseReductionCentibels: 900,
            noiseSensitivityPercent: 50,
            noiseSmoothingMillis: 240,
            deEsserEnabled: false,
            deEsserFrequencyHertz: 6500,
            deEsserThresholdCentibels: -2400,
            deEsserReductionCentibels: 600
        } : adjustmentDraft.restorationValue()) + ":" + JSON.stringify(auditionOriginal ? {
            enabled: false,
            fundamentalHertz: 50,
            harmonicCount: 4,
            qualityTenths: 300,
            depthCentibels: 2400
        } : adjustmentDraft.deHumValue()) + ":" + JSON.stringify(auditionOriginal ? {
            enabled: false,
            sensitivityPercent: 50,
            maximumClickMicroseconds: 1000,
            repairPercent: 100
        } : adjustmentDraft.deClickValue()) + ":" + JSON.stringify(auditionOriginal ? {
            enabled: false,
            invertLeft: false,
            invertRight: false,
            swapChannels: false,
            monoFoldDown: false,
            balancePercent: 0
        } : adjustmentDraft.channelRepairValue()) + ":" + (auditionOriginal ? false : adjustmentDraft.equalizerEnabled) + ":" + JSON.stringify(auditionOriginal ? adjustmentDraft.defaultEqualizerBands() : adjustmentDraft.equalizerBands) + ":" + (auditionOriginal ? false : adjustmentDraft.compressorEnabled) + ":" + (auditionOriginal ? -1800 : adjustmentDraft.compressorThresholdCentibels) + ":" + (auditionOriginal ? 30 : adjustmentDraft.compressorRatioTenths) + ":" + (auditionOriginal ? 10 : adjustmentDraft.compressorAttackMillis) + ":" + (auditionOriginal ? 120 : adjustmentDraft.compressorReleaseMillis) + ":" + (auditionOriginal ? 0 : adjustmentDraft.compressorMakeupCentibels) + ":" + JSON.stringify(auditionOriginal ? {
            enabled: false,
            mixPercent: 18,
            preDelayMillis: 20,
            decayMillis: 1800,
            sizePercent: 55,
            dampingPercent: 45,
            lowCutHertz: 120,
            highCutHertz: 10000
        } : adjustmentDraft.reverbValue()) + ":" + (auditionOriginal ? false : adjustmentDraft.limiterEnabled) + ":" + (auditionOriginal ? -100 : adjustmentDraft.limiterCeilingCentibels) + ":" + (auditionOriginal ? 100 : adjustmentDraft.limiterReleaseMillis) + ":" + JSON.stringify(auditionOriginal ? adjustmentDraft.defaultEffectChain() : adjustmentDraft.effectChain) + ":" + JSON.stringify(auditionOriginal ? adjustmentDraft.defaultEditSegments(adjustmentDraft.trimStartMillis, adjustmentDraft.trimEndMillis) : adjustmentDraft.editSegments) + ":" + JSON.stringify(auditionOriginal ? [] : adjustmentDraft.effectMasks) + ":" + JSON.stringify(auditionOriginal ? adjustmentDraft.creativeVfxForOriginal() : adjustmentDraft.creativeVfxValue());
    }

    function refreshAsset(): void {
        waveformLevels = [];
        if (!asset || !asset.id || asset.pathStatus === "missing")
            return;
        waveformLevels = backend.waveformForAsset(asset.id);
    }

    function playFrom(millis: int): void {
        if (!asset || asset.pathStatus === "missing")
            return;
        player.playAdjusted(asset.path, adjustmentDraft.trimStartMillis, adjustmentDraft.trimEndMillis, auditionOriginal ? 0 : adjustmentDraft.fadeInMillis, auditionOriginal ? 0 : adjustmentDraft.fadeOutMillis, auditionOriginal ? 0 : adjustmentDraft.fadeInCurve, auditionOriginal ? 0 : adjustmentDraft.fadeOutCurve, auditionOriginal ? 0 : adjustmentDraft.gainCentibels, auditionOriginal ? 0 : adjustmentDraft.lowCutHertz, auditionOriginal ? {
            enabled: false,
            dePlosiveEnabled: false,
            dePlosiveFrequencyHertz: 140,
            dePlosiveSensitivityPercent: 50,
            dePlosiveReductionCentibels: 1200,
            dePlosiveReleaseMillis: 160,
            noiseEnabled: false,
            noiseReductionCentibels: 900,
            noiseSensitivityPercent: 50,
            noiseSmoothingMillis: 240,
            deEsserEnabled: false,
            deEsserFrequencyHertz: 6500,
            deEsserThresholdCentibels: -2400,
            deEsserReductionCentibels: 600
        } : adjustmentDraft.restorationValue(), auditionOriginal ? {
            enabled: false,
            fundamentalHertz: 50,
            harmonicCount: 4,
            qualityTenths: 300,
            depthCentibels: 2400
        } : adjustmentDraft.deHumValue(), auditionOriginal ? {
            enabled: false,
            sensitivityPercent: 50,
            maximumClickMicroseconds: 1000,
            repairPercent: 100
        } : adjustmentDraft.deClickValue(), auditionOriginal ? {
            enabled: false,
            invertLeft: false,
            invertRight: false,
            swapChannels: false,
            monoFoldDown: false,
            balancePercent: 0
        } : adjustmentDraft.channelRepairValue(), auditionOriginal ? false : adjustmentDraft.equalizerEnabled, auditionOriginal ? adjustmentDraft.defaultEqualizerBands() : adjustmentDraft.equalizerBands, auditionOriginal ? false : adjustmentDraft.compressorEnabled, auditionOriginal ? -1800 : adjustmentDraft.compressorThresholdCentibels, auditionOriginal ? 30 : adjustmentDraft.compressorRatioTenths, auditionOriginal ? 10 : adjustmentDraft.compressorAttackMillis, auditionOriginal ? 120 : adjustmentDraft.compressorReleaseMillis, auditionOriginal ? 0 : adjustmentDraft.compressorMakeupCentibels, auditionOriginal ? {
            enabled: false,
            mixPercent: 18,
            preDelayMillis: 20,
            decayMillis: 1800,
            sizePercent: 55,
            dampingPercent: 45,
            lowCutHertz: 120,
            highCutHertz: 10000
        } : adjustmentDraft.reverbValue(), auditionOriginal ? false : adjustmentDraft.limiterEnabled, auditionOriginal ? -100 : adjustmentDraft.limiterCeilingCentibels, auditionOriginal ? 100 : adjustmentDraft.limiterReleaseMillis, auditionOriginal ? adjustmentDraft.defaultEffectChain() : adjustmentDraft.effectChain, auditionOriginal ? adjustmentDraft.defaultEditSegments(adjustmentDraft.trimStartMillis, adjustmentDraft.trimEndMillis) : adjustmentDraft.editSegments, auditionOriginal ? [] : adjustmentDraft.effectMasks, auditionOriginal ? adjustmentDraft.creativeVfxForOriginal() : adjustmentDraft.creativeVfxValue());
        loadedPath = asset.path;
        loadedBaseAdjustmentKey = playbackBaseAdjustmentKey();
        loadedAdjustmentKey = adjustmentKey();
        const start = Math.max(adjustmentDraft.trimStartMillis, Math.min(millis, adjustmentDraft.trimEndMillis));
        if (start > adjustmentDraft.trimStartMillis)
            player.seek(start);
    }

    function ownsActivePlayback(): bool {
        return player.active && hasAsset && loadedPath === asset.path && loadedAdjustmentKey === adjustmentKey();
    }

    function seekOrLoad(millis: int): void {
        if (!hasAsset)
            return;
        if (!ownsActivePlayback()) {
            playFrom(millis);
        } else {
            player.seek(Math.max(adjustmentDraft.trimStartMillis, Math.min(millis, adjustmentDraft.trimEndMillis)));
        }
    }

    function defaultPlaybackStart(): int {
        return editorTimeline.hasTimeSelection ? editorTimeline.selectionStartMillis : adjustmentDraft.trimStartMillis;
    }

    function scheduleEffectsPreview(): void {
        if (auditionOriginal || !player.active || !hasAsset || loadedPath !== asset.path || loadedBaseAdjustmentKey !== playbackBaseAdjustmentKey()) {
            return;
        }
        effectsPreviewTimer.restart();
    }

    function togglePlayback(): void {
        if (!hasAsset || asset.pathStatus === "missing")
            return;
        if (!ownsActivePlayback()) {
            playFrom(defaultPlaybackStart());
        } else {
            player.togglePause();
        }
    }

    function setOriginalAudition(enabled: bool): void {
        if (auditionOriginal === enabled)
            return;
        const resumeAt = loadedPath === (hasAsset ? asset.path : "") ? player.position : defaultPlaybackStart();
        const wasLoaded = ownsActivePlayback();
        auditionOriginal = enabled;
        if (wasLoaded)
            playFrom(resumeAt);
    }

    function undo(): void {
        adjustmentDraft.undo();
    }

    function redo(): void {
        adjustmentDraft.redo();
    }

    function save(): void {
        adjustmentDraft.save();
    }

    function refreshProcessingRecipes(): void {
        processingRecipes = backend.listProcessingRecipes();
        processingRecipeModelRevision += 1;
    }

    function presentSaveProcessingRecipe(): void {
        const baseName = fileName(asset.path).replace(/\.[^.]+$/, "");
        processingRecipeSaveDialog.suggestedName = baseName;
        processingRecipeSaveDialog.present();
    }

    function presentApplyProcessingRecipe(): void {
        refreshProcessingRecipes();
        processingRecipeApplyDialog.recipeModel = processingRecipes;
        processingRecipeApplyDialog.modelRevision = processingRecipeModelRevision;
        processingRecipeApplyDialog.selectedRecipeId = processingRecipes.length > 0 ? processingRecipes[0].id : "";
        processingRecipeApplyDialog.mergeMode = "merge";
        processingRecipeApplyDialog.targetIds = [asset.id];
        processingRecipeApplyDialog.targetLabel = "";
        processingRecipeApplyDialog.present();
    }

    function presentProcessingRecipeManager(): void {
        refreshProcessingRecipeManager("");
        processingRecipeManagerDialog.sourceAssetId = hasAsset ? asset.id : "";
        processingRecipeManagerDialog.sourceLabel = hasAsset ? fileName(asset.path) : "";
        processingRecipeManagerDialog.canUpdateFromSource = hasAsset && !dirty && Number(asset.adjustmentRevision || 0) > 0;
        processingRecipeManagerDialog.present();
    }

    function refreshProcessingRecipeManager(preferredId: string): void {
        refreshProcessingRecipes();
        processingRecipeManagerDialog.recipeModel = processingRecipes;
        processingRecipeManagerDialog.modelRevision = processingRecipeModelRevision;
        const stillExists = processingRecipes.some(recipe => recipe.id === preferredId);
        processingRecipeManagerDialog.selectedRecipeId = stillExists ? preferredId : processingRecipes.length > 0 ? processingRecipes[0].id : "";
    }

    function refreshProcessingHistory(): void {
        processingHistory = backend.listProcessingRecipeHistory();
        processingHistoryModelRevision += 1;
    }

    function presentProcessingHistory(): void {
        refreshProcessingHistory();
        processingHistoryDialog.present();
    }

    function revertLastProcessingRecipeApplication(): void {
        if (lastProcessingRecipeBatchId.length === 0)
            return;
        revertProcessingRecipeBatch(lastProcessingRecipeBatchId);
    }

    function revertProcessingRecipeBatch(batchId: string): void {
        if (batchId.length === 0)
            return;
        const receipt = backend.revertProcessingRecipeApplication(batchId);
        if (!receipt || !receipt.revertId) {
            showProcessingRecipeNotice(qsTr("The processing recipe application could not be undone."));
        } else {
            if (lastProcessingRecipeBatchId === batchId)
                lastProcessingRecipeBatchId = "";
            showProcessingRecipeNotice(qsTr("%1 restored · %2 conflicts · %3 failed").arg(receipt.restoredCount).arg(receipt.conflictCount).arg(receipt.failedCount));
        }
        refreshProcessingHistory();
    }

    function showProcessingRecipeNotice(message: string): void {
        processingRecipeNotice = message;
        processingRecipeNoticePopup.open();
        processingRecipeNoticeTimer.restart();
    }

    function debugNudgeEqualizer(): void {
        const band = adjustmentDraft.equalizerBands[2];
        const next = band.gainCentibels >= 1100 ? band.gainCentibels - 100 : band.gainCentibels + 100;
        adjustmentDraft.setEqualizerBand(2, true, band.filterKind, band.frequencyHertz, band.qHundredths, next);
    }

    function debugEnableCompressor(): void {
        adjustmentDraft.setCompressorEnabled(true);
        adjustmentDraft.setCompressorParameter("threshold", -4000);
    }

    function debugAnalyzeOutput(): void {
        adjustmentEditor.analyzeOutput();
    }

    function openExport(): void {
        exportDialog.present();
    }

    function debugExport(destination: url): void {
        exportDialog.debugExport(destination);
    }

    onAssetChanged: {
        player.stop();
        loudnessAnalyzer.cancel();
        renderExporter.cancel();
        auditionOriginal = false;
        loadedPath = "";
        loadedBaseAdjustmentKey = "";
        loadedAdjustmentKey = "";
        Qt.callLater(refreshAsset);
    }

    Component.onCompleted: refreshProcessingRecipes()

    Connections {
        target: backend

        function onAssetsChanged(): void {
            workspace.refreshAsset();
        }
        function onProcessingRecipesChanged(): void {
            workspace.refreshProcessingRecipes();
        }
    }

    SoundAdjustmentDraft {
        id: adjustmentDraft

        asset: workspace.asset

        onSaveRequested: function (startMillis, endMillis, fadeIn, fadeOut, fadeInCurve, fadeOutCurve, gain, lowCut, restorationEnabled, dePlosiveEnabled, dePlosiveFrequency, dePlosiveSensitivity, dePlosiveReduction, dePlosiveRelease, noiseEnabled, noiseReduction, noiseSensitivity, noiseSmoothing, deEsserEnabled, deEsserFrequency, deEsserThreshold, deEsserReduction, deHumEnabled, deHumFundamental, deHumHarmonicCount, deHumQuality, deHumDepth, deClickEnabled, deClickSensitivity, deClickMaximumClick, deClickRepair, channelRepairEnabled, channelRepairInvertLeft, channelRepairInvertRight, channelRepairSwapChannels, channelRepairMonoFoldDown, channelRepairBalance, equalizerEnabled, equalizerBands, compressorEnabled, compressorThreshold, compressorRatio, compressorAttack, compressorRelease, compressorMakeup, reverbCharacter, reverbEnabled, reverbMix, reverbPreDelay, reverbDecay, reverbSize, reverbDamping, reverbLowCut, reverbHighCut, limiterEnabled, limiterCeiling, limiterRelease, effectChain, editSegments, effectMasks, creativeVfx) {
            if (backend.setAssetAdjustment(workspace.asset.id, startMillis, endMillis, fadeIn, fadeOut, fadeInCurve, fadeOutCurve, gain, lowCut, restorationEnabled, dePlosiveEnabled, dePlosiveFrequency, dePlosiveSensitivity, dePlosiveReduction, dePlosiveRelease, noiseEnabled, noiseReduction, noiseSensitivity, noiseSmoothing, deEsserEnabled, deEsserFrequency, deEsserThreshold, deEsserReduction, deHumEnabled, deHumFundamental, deHumHarmonicCount, deHumQuality, deHumDepth, deClickEnabled, deClickSensitivity, deClickMaximumClick, deClickRepair, channelRepairEnabled, channelRepairInvertLeft, channelRepairInvertRight, channelRepairSwapChannels, channelRepairMonoFoldDown, channelRepairBalance, equalizerEnabled, equalizerBands, compressorEnabled, compressorThreshold, compressorRatio, compressorAttack, compressorRelease, compressorMakeup, reverbCharacter, reverbEnabled, reverbMix, reverbPreDelay, reverbDecay, reverbSize, reverbDamping, reverbLowCut, reverbHighCut, limiterEnabled, limiterCeiling, limiterRelease, effectChain, editSegments, effectMasks, creativeVfx)) {
                adjustmentDraft.markSaved();
                workspace.auditionOriginal = false;
                workspace.loadedBaseAdjustmentKey = "";
                workspace.loadedAdjustmentKey = "";
            }
        }
    }

    SoundExportDialog {
        id: exportDialog
        asset: workspace.asset
        draft: adjustmentDraft
        exporter: renderExporter
    }

    Connections {
        target: adjustmentDraft

        function onEqualizerBandsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onEqualizerEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onRestorationEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDePlosiveEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDePlosiveFrequencyHertzChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDePlosiveSensitivityPercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDePlosiveReductionCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDePlosiveReleaseMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onNoiseReductionEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onNoiseReductionCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onNoiseReductionSensitivityPercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onNoiseReductionSmoothingMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeEsserEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeEsserFrequencyHertzChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeEsserThresholdCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeEsserReductionCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeHumEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeHumFundamentalHertzChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeHumHarmonicCountChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeHumQualityTenthsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeHumDepthCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeClickEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeClickSensitivityPercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeClickMaximumClickMicrosecondsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onDeClickRepairPercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onChannelRepairEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onChannelRepairInvertLeftChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onChannelRepairInvertRightChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onChannelRepairSwapChannelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onChannelRepairMonoFoldDownChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onChannelRepairBalancePercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCompressorEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCompressorThresholdCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCompressorRatioTenthsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCompressorAttackMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCompressorReleaseMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCompressorMakeupCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbMixPercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbPreDelayMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbDecayMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbSizePercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbDampingPercentChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbLowCutHertzChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onReverbHighCutHertzChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onLimiterEnabledChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onLimiterCeilingCentibelsChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onLimiterReleaseMillisChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onCreativeVfxChanged(): void {
            workspace.scheduleEffectsPreview();
        }
        function onEffectChainChanged(): void {
            if (workspace.auditionOriginal || !player.active || !workspace.hasAsset || workspace.loadedPath !== workspace.asset.path)
                return;
            const resumeAt = player.position;
            Qt.callLater(function () {
                workspace.playFrom(resumeAt);
            });
        }
        function onEditSegmentsChanged(): void {
            if (workspace.auditionOriginal || !player.active || !workspace.hasAsset || workspace.loadedPath !== workspace.asset.path)
                return;
            const resumeAt = player.position;
            Qt.callLater(function () {
                workspace.playFrom(resumeAt);
            });
        }
        function onEffectMasksChanged(): void {
            if (workspace.auditionOriginal || !player.active || !workspace.hasAsset || workspace.loadedPath !== workspace.asset.path)
                return;
            const resumeAt = player.position;
            Qt.callLater(function () {
                workspace.playFrom(resumeAt);
            });
        }
    }

    Timer {
        id: effectsPreviewTimer

        interval: 16
        repeat: false
        onTriggered: {
            if (workspace.auditionOriginal || !player.active || !workspace.hasAsset || workspace.loadedPath !== workspace.asset.path || workspace.loadedBaseAdjustmentKey !== workspace.playbackBaseAdjustmentKey()) {
                return;
            }
            const equalizerUpdated = player.updateEqualizer(adjustmentDraft.equalizerEnabled, adjustmentDraft.equalizerBands);
            const restorationUpdated = player.updateRestoration(adjustmentDraft.restorationValue());
            const deHumUpdated = player.updateDeHum(adjustmentDraft.deHumValue());
            const deClickUpdated = player.updateDeClick(adjustmentDraft.deClickValue());
            const channelRepairUpdated = player.updateChannelRepair(adjustmentDraft.channelRepairValue());
            const compressorUpdated = player.updateCompressor(adjustmentDraft.compressorEnabled, adjustmentDraft.compressorThresholdCentibels, adjustmentDraft.compressorRatioTenths, adjustmentDraft.compressorAttackMillis, adjustmentDraft.compressorReleaseMillis, adjustmentDraft.compressorMakeupCentibels);
            const limiterUpdated = player.updateLimiter(adjustmentDraft.limiterEnabled, adjustmentDraft.limiterCeilingCentibels, adjustmentDraft.limiterReleaseMillis);
            const reverbUpdated = player.updateReverb(adjustmentDraft.reverbValue());
            const creativeVfxUpdated = player.updateCreativeVfx(adjustmentDraft.creativeVfxValue());
            if (restorationUpdated && deHumUpdated && deClickUpdated && channelRepairUpdated && equalizerUpdated && compressorUpdated && reverbUpdated && limiterUpdated && creativeVfxUpdated) {
                workspace.loadedAdjustmentKey = workspace.adjustmentKey();
            }
        }
    }

    Timer {
        interval: 40
        repeat: true
        running: workspace.hasAsset && player.playing && editorTimeline.loopSelection && editorTimeline.hasTimeSelection && workspace.loadedPath === workspace.asset.path
        onTriggered: {
            if (player.position >= editorTimeline.selectionEndMillis - 40) {
                player.seek(editorTimeline.selectionStartMillis);
            }
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, 420)
        spacing: 12
        visible: !workspace.hasAsset

        Rectangle {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 72
            Layout.preferredHeight: 72
            radius: 24
            color: Theme.surfaceSubtle

            EchoIcon {
                anchors.centerIn: parent
                source: "qrc:/EchoDesktop/icons/edit.svg"
                size: 30
                color: Theme.textDisabled
            }
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Select a sound in Audio Space")
            color: Theme.textPrimary
            font.pixelSize: 20
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            Layout.fillWidth: true
            text: qsTr("Choose a sound before opening adjustments.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontBody
            wrapMode: Text.WordWrap
            horizontalAlignment: Text.AlignHCenter
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        anchors.topMargin: 12
        spacing: 10
        visible: workspace.hasAsset

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 36
            spacing: 10

            Text {
                Layout.maximumWidth: Math.min(440, implicitWidth)
                text: workspace.hasAsset ? workspace.fileName(workspace.asset.path) : ""
                color: Theme.textPrimary
                font.pixelSize: 16
                font.weight: Font.DemiBold
                elide: Text.ElideRight

                HoverHandler {
                    id: sourceHover
                }
                ToolTip.visible: sourceHover.hovered
                ToolTip.text: workspace.hasAsset ? workspace.asset.path : ""
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.preferredHeight: 16
                color: Theme.border
            }

            Text {
                text: workspace.technicalDetails()
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMeta
                elide: Text.ElideRight
            }

            Item {
                Layout.fillWidth: true
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/recipe-save.svg"
                toolTipText: qsTr("Save as recipe")
                accessibleName: toolTipText
                enabled: workspace.hasAsset && !workspace.dirty && Number(workspace.asset.adjustmentRevision || 0) > 0
                onClicked: workspace.presentSaveProcessingRecipe()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/recipe-apply.svg"
                toolTipText: qsTr("Apply recipe")
                accessibleName: toolTipText
                enabled: workspace.hasAsset && !workspace.dirty
                onClicked: workspace.presentApplyProcessingRecipe()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/equalizer.svg"
                toolTipText: qsTr("Manage processing recipes")
                accessibleName: toolTipText
                enabled: workspace.processingRecipes.length > 0
                buttonSize: 30
                iconSize: 16
                onClicked: workspace.presentProcessingRecipeManager()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/history.svg"
                toolTipText: qsTr("Processing history")
                accessibleName: toolTipText
                buttonSize: 30
                iconSize: 16
                onClicked: workspace.presentProcessingHistory()
            }

            EchoIconButton {
                source: "qrc:/EchoDesktop/icons/export.svg"
                toolTipText: qsTr("Export")
                accessibleName: toolTipText
                enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing" && !renderExporter.running
                onClicked: workspace.openExport()
            }
        }

        SplitView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            orientation: Qt.Vertical

            handle: Rectangle {
                implicitHeight: 9
                color: SplitHandle.pressed ? Theme.surfaceSelected : SplitHandle.hovered ? Theme.surfaceSubtle : Theme.window

                Rectangle {
                    anchors.centerIn: parent
                    width: SplitHandle.hovered || SplitHandle.pressed ? 64 : 44
                    height: SplitHandle.hovered || SplitHandle.pressed ? 2 : 1
                    radius: 1
                    color: SplitHandle.pressed ? Theme.accent : Theme.borderStrong

                    Behavior on width {
                        NumberAnimation {
                            duration: 90
                        }
                    }
                }

                HoverHandler {
                    cursorShape: Qt.SplitVCursor
                }
            }

            SoundEditorTimeline {
                id: editorTimeline

                SplitView.fillWidth: true
                SplitView.preferredHeight: 286
                SplitView.minimumHeight: 176
                SplitView.maximumHeight: 420
                waveformLevels: workspace.waveformLevels
                sourceDurationMillis: adjustmentDraft.sourceDurationMillis
                trimStartMillis: adjustmentDraft.trimStartMillis
                trimEndMillis: adjustmentDraft.trimEndMillis
                fadeInMillis: adjustmentDraft.fadeInMillis
                fadeOutMillis: adjustmentDraft.fadeOutMillis
                fadeInCurve: adjustmentDraft.fadeInCurve
                fadeOutCurve: adjustmentDraft.fadeOutCurve
                gainCentibels: adjustmentDraft.gainCentibels
                playbackPositionMillis: workspace.hasAsset && workspace.ownsActivePlayback() ? player.position : adjustmentDraft.trimStartMillis
                isPlaying: workspace.ownsActivePlayback() && player.playing
                draft: adjustmentDraft
                enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"

                onTrimRequested: function (startMillis, endMillis) {
                    adjustmentDraft.setTrimRange(startMillis, endMillis);
                }
                onFadeRequested: function (fadeIn, fadeOut) {
                    adjustmentDraft.setFades(fadeIn, fadeOut);
                }
                onGainRequested: gain => adjustmentDraft.setGain(gain)
                onSeekRequested: millis => workspace.seekOrLoad(millis)
                onPlayPauseRequested: workspace.togglePlayback()
                onEditGestureStarted: adjustmentDraft.beginGesture()
                onEditGestureFinished: adjustmentDraft.endGesture()
                onUndoRequested: adjustmentDraft.undo()
                onRedoRequested: adjustmentDraft.redo()
            }

            SoundAdjustmentEditor {
                id: adjustmentEditor
                SplitView.fillWidth: true
                SplitView.fillHeight: true
                SplitView.minimumHeight: 330
                draft: adjustmentDraft
                meterSource: player
                analyzer: loudnessAnalyzer
                sourcePath: workspace.hasAsset ? workspace.asset.path : ""
                analysisKey: workspace.adjustmentKey()
                hasTimeSelection: editorTimeline.hasTimeSelection
                selectionStartMillis: editorTimeline.selectionStartMillis
                selectionEndMillis: editorTimeline.selectionEndMillis
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 46
            radius: Theme.compactControlRadius
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.border

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 6
                anchors.rightMargin: 8
                spacing: 8

                EchoIconButton {
                    source: workspace.hasAsset && player.playing && workspace.ownsActivePlayback() ? "qrc:/EchoDesktop/icons/pause.svg" : "qrc:/EchoDesktop/icons/play.svg"
                    toolTipText: player.playing ? qsTr("Pause") : qsTr("Play")
                    enabled: workspace.hasAsset && workspace.asset.pathStatus !== "missing"
                    buttonSize: 36
                    iconSize: 18
                    onClicked: workspace.togglePlayback()
                }

                EchoIconButton {
                    source: "qrc:/EchoDesktop/icons/stop.svg"
                    toolTipText: qsTr("Stop")
                    enabled: workspace.ownsActivePlayback()
                    buttonSize: 32
                    iconSize: 15
                    onClicked: player.stop()
                }

                Rectangle {
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: 20
                    color: Theme.border
                }

                Text {
                    text: workspace.formatDuration(player.position) + " / " + workspace.formatDuration(player.duration)
                    color: Theme.textPrimary
                    font.family: "Menlo"
                    font.pixelSize: Theme.fontBody
                    font.weight: Font.DemiBold
                    Layout.preferredWidth: 124
                }

                Item {
                    Layout.fillWidth: true
                }

                Text {
                    text: qsTr("Audition")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMeta
                }

                EchoButton {
                    text: qsTr("Adjusted")
                    ghost: true
                    selected: !workspace.auditionOriginal
                    onClicked: workspace.setOriginalAudition(false)
                }

                EchoButton {
                    text: qsTr("Original")
                    ghost: true
                    selected: workspace.auditionOriginal
                    onClicked: workspace.setOriginalAudition(true)
                }
            }
        }
    }

    ProcessingRecipeSaveDialog {
        id: processingRecipeSaveDialog

        onSaveRequested: function (name, componentIds) {
            const recipeId = backend.createProcessingRecipe(name, workspace.asset.id, componentIds);
            if (recipeId.length > 0) {
                workspace.refreshProcessingRecipes();
                workspace.showProcessingRecipeNotice(qsTr("Processing recipe saved."));
            } else {
                workspace.showProcessingRecipeNotice(qsTr("The processing recipe could not be saved."));
            }
        }
    }

    ProcessingRecipeApplyDialog {
        id: processingRecipeApplyDialog

        onRecipeSelected: recipeId => selectedRecipeId = recipeId
        onMergeModeSelected: mode => mergeMode = mode
        onApplyRequested: function (recipeId, mergeMode, targetIds) {
            const receipt = backend.applyProcessingRecipe(recipeId, targetIds, mergeMode);
            if (receipt && receipt.batchId) {
                workspace.lastProcessingRecipeBatchId = receipt.batchId;
                workspace.refreshProcessingHistory();
                workspace.showProcessingRecipeNotice(qsTr("Processing recipe applied."));
            } else {
                workspace.lastProcessingRecipeBatchId = "";
                workspace.showProcessingRecipeNotice(qsTr("The processing recipe could not be applied."));
            }
        }
    }

    ProcessingRecipeManagerDialog {
        id: processingRecipeManagerDialog

        onRecipeSelected: recipeId => selectedRecipeId = recipeId
        onRenameRequested: function (recipeId, name) {
            if (backend.renameProcessingRecipe(recipeId, name)) {
                workspace.refreshProcessingRecipeManager(recipeId);
                workspace.showProcessingRecipeNotice(qsTr("Processing recipe renamed."));
            } else {
                workspace.showProcessingRecipeNotice(qsTr("The processing recipe could not be renamed."));
            }
        }
        onUpdateRequested: function (recipeId, componentIds) {
            const revision = backend.updateProcessingRecipe(recipeId, sourceAssetId, componentIds);
            if (revision > 0) {
                workspace.refreshProcessingRecipeManager(recipeId);
                workspace.showProcessingRecipeNotice(qsTr("Processing recipe version %1 added.").arg(revision));
            } else {
                workspace.showProcessingRecipeNotice(qsTr("The processing recipe could not be updated."));
            }
        }
        onArchiveRequested: function (recipeId) {
            if (backend.archiveProcessingRecipe(recipeId)) {
                workspace.refreshProcessingRecipeManager("");
                workspace.showProcessingRecipeNotice(qsTr("Processing recipe archived."));
            } else {
                workspace.showProcessingRecipeNotice(qsTr("The processing recipe could not be archived."));
            }
        }
    }

    ProcessingHistoryDialog {
        id: processingHistoryDialog

        historyModel: workspace.processingHistory
        modelRevision: workspace.processingHistoryModelRevision
        onRevertRequested: batchId => workspace.revertProcessingRecipeBatch(batchId)
    }

    Popup {
        id: processingRecipeNoticePopup

        parent: Overlay.overlay
        x: Math.round((parent.width - width) / 2)
        y: 18
        implicitWidth: Math.min(540, processingRecipeNoticeRow.implicitWidth + 28)
        implicitHeight: processingRecipeNoticeRow.implicitHeight + 20
        padding: 0
        closePolicy: Popup.NoAutoClose

        background: Rectangle {
            radius: Theme.controlRadius
            color: Theme.panelRaised
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: RowLayout {
            id: processingRecipeNoticeRow

            spacing: 10

            Text {
                Layout.fillWidth: true
                text: workspace.processingRecipeNotice
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }

            EchoButton {
                visible: workspace.lastProcessingRecipeBatchId.length > 0
                text: qsTr("Undo batch")
                ghost: true
                onClicked: workspace.revertLastProcessingRecipeApplication()
            }
        }
    }

    Timer {
        id: processingRecipeNoticeTimer
        interval: workspace.lastProcessingRecipeBatchId.length > 0 ? 6000 : 2600
        onTriggered: processingRecipeNoticePopup.close()
    }
}

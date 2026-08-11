//! Authoritative editor draft for one selected immutable original. It owns
//! validation, gesture-coalesced undo/redo history, saved-state comparison,
//! and explicit publication; presentation components only mutate this owner.

import QtQuick

QtObject {
    id: draft

    required property var asset

    property int trimStartMillis: 0
    property int trimEndMillis: 0
    property int fadeInMillis: 0
    property int fadeOutMillis: 0
    property int fadeInCurve: 0
    property int fadeOutCurve: 0
    property int gainCentibels: 0
    property int lowCutHertz: 0
    property bool restorationEnabled: true
    property bool dePlosiveEnabled: false
    property int dePlosiveFrequencyHertz: 140
    property int dePlosiveSensitivityPercent: 50
    property int dePlosiveReductionCentibels: 1200
    property int dePlosiveReleaseMillis: 160
    property bool noiseReductionEnabled: false
    property int noiseReductionCentibels: 900
    property int noiseReductionSensitivityPercent: 50
    property int noiseReductionSmoothingMillis: 240
    property bool deEsserEnabled: false
    property int deEsserFrequencyHertz: 6500
    property int deEsserThresholdCentibels: -2400
    property int deEsserReductionCentibels: 600
    property bool deHumEnabled: false
    property int deHumFundamentalHertz: 50
    property int deHumHarmonicCount: 4
    property int deHumQualityTenths: 300
    property int deHumDepthCentibels: 2400
    property bool deClickEnabled: false
    property int deClickSensitivityPercent: 50
    property int deClickMaximumClickMicroseconds: 1000
    property int deClickRepairPercent: 100
    property bool channelRepairEnabled: false
    property bool channelRepairInvertLeft: false
    property bool channelRepairInvertRight: false
    property bool channelRepairSwapChannels: false
    property bool channelRepairMonoFoldDown: false
    property int channelRepairBalancePercent: 0
    property bool equalizerEnabled: true
    property var equalizerBands: defaultEqualizerBands()
    property bool compressorEnabled: false
    property int compressorThresholdCentibels: -1800
    property int compressorRatioTenths: 30
    property int compressorAttackMillis: 10
    property int compressorReleaseMillis: 120
    property int compressorMakeupCentibels: 0
    property int reverbCharacter: 0
    property bool reverbEnabled: false
    property int reverbMixPercent: 18
    property int reverbPreDelayMillis: 20
    property int reverbDecayMillis: 1800
    property int reverbSizePercent: 55
    property int reverbDampingPercent: 45
    property int reverbLowCutHertz: 120
    property int reverbHighCutHertz: 10000
    property bool limiterEnabled: false
    property int limiterCeilingCentibels: -100
    property int limiterReleaseMillis: 100
    property var effectChain: defaultEffectChain()
    property var editSegments: defaultEditSegments(0, 0)
    property var effectMasks: []

    property var _savedSnapshot: ({})
    property var _history: []
    property int _historyIndex: -1
    property var _gestureStart: null
    property bool _restoring: false

    readonly property int sourceDurationMillis: asset ? Math.max(0, Number(asset.durationMillis)) : 0
    readonly property int selectedDurationMillis: Math.max(0, trimEndMillis - trimStartMillis)
    readonly property bool canUndo: _historyIndex > 0
    readonly property bool canRedo: _historyIndex >= 0 && _historyIndex < _history.length - 1
    readonly property bool identity: trimStartMillis === 0 && trimEndMillis === sourceDurationMillis && fadeInMillis === 0 && fadeOutMillis === 0 && fadeInCurve === 0 && fadeOutCurve === 0 && gainCentibels === 0 && lowCutHertz === 0 && (!containsEffectNode(0) || !restorationEnabled || (!dePlosiveEnabled && !noiseReductionEnabled && !deEsserEnabled)) && (!containsEffectNode(5) || !deHumEnabled) && (!containsEffectNode(6) || !deClickEnabled) && (!containsEffectNode(7) || !channelRepairEnabled || (!channelRepairInvertLeft && !channelRepairInvertRight && !channelRepairSwapChannels && !channelRepairMonoFoldDown && channelRepairBalancePercent === 0)) && (!containsEffectNode(1) || !equalizerEnabled || equalizerIsFlat()) && (!containsEffectNode(2) || !compressorEnabled) && (!containsEffectNode(3) || !reverbEnabled) && !limiterEnabled && editSegmentsAreIdentity() && effectMasks.length === 0
    readonly property bool dirty: !sameSnapshot(snapshot(), _savedSnapshot)

    signal saveRequested(int startMillis, int endMillis, int fadeIn, int fadeOut, int fadeInCurve, int fadeOutCurve, int gain, int lowCut, bool restorationEnabled, bool dePlosiveEnabled, int dePlosiveFrequency, int dePlosiveSensitivity, int dePlosiveReduction, int dePlosiveRelease, bool noiseEnabled, int noiseReduction, int noiseSensitivity, int noiseSmoothing, bool deEsserEnabled, int deEsserFrequency, int deEsserThreshold, int deEsserReduction, bool deHumEnabled, int deHumFundamental, int deHumHarmonicCount, int deHumQuality, int deHumDepth, bool deClickEnabled, int deClickSensitivity, int deClickMaximumClick, int deClickRepair, bool channelRepairEnabled, bool channelRepairInvertLeft, bool channelRepairInvertRight, bool channelRepairSwapChannels, bool channelRepairMonoFoldDown, int channelRepairBalance, bool equalizerEnabled, var equalizerBands, bool compressorEnabled, int compressorThreshold, int compressorRatio, int compressorAttack, int compressorRelease, int compressorMakeup, int reverbCharacter, bool reverbEnabled, int reverbMix, int reverbPreDelay, int reverbDecay, int reverbSize, int reverbDamping, int reverbLowCut, int reverbHighCut, bool limiterEnabled, int limiterCeiling, int limiterRelease, var effectChain, var editSegments, var effectMasks)

    function clamp(value: real, minimum: real, maximum: real): real {
        return Math.max(minimum, Math.min(maximum, value));
    }

    function defaultEqualizerBands(): var {
        return [
            {
                enabled: true,
                filterKind: 1,
                frequencyHertz: 120,
                qHundredths: 71,
                gainCentibels: 0
            },
            {
                enabled: false,
                filterKind: 0,
                frequencyHertz: 250,
                qHundredths: 100,
                gainCentibels: 0
            },
            {
                enabled: true,
                filterKind: 0,
                frequencyHertz: 1000,
                qHundredths: 100,
                gainCentibels: 0
            },
            {
                enabled: false,
                filterKind: 0,
                frequencyHertz: 3000,
                qHundredths: 100,
                gainCentibels: 0
            },
            {
                enabled: false,
                filterKind: 0,
                frequencyHertz: 5000,
                qHundredths: 100,
                gainCentibels: 0
            },
            {
                enabled: true,
                filterKind: 2,
                frequencyHertz: 8000,
                qHundredths: 71,
                gainCentibels: 0
            }
        ];
    }

    function defaultEffectChain(): var {
        return [0, 1, 2, 3, 4];
    }

    function copyEffectChain(values: var): var {
        if (!values || values.length < 1 || values.length > 8)
            return defaultEffectChain();
        const result = [];
        const seen = [false, false, false, false, false, false, false, false];
        for (let index = 0; index < values.length; ++index) {
            const node = Math.round(Number(values[index]));
            if (node < 0 || node > 7 || seen[node])
                return defaultEffectChain();
            seen[node] = true;
            result.push(node);
        }
        return result[result.length - 1] === 4 ? result : defaultEffectChain();
    }

    function sameEffectChain(left: var, right: var): bool {
        if (!left || !right || left.length !== right.length)
            return false;
        for (let index = 0; index < left.length; ++index) {
            if (Number(left[index]) !== Number(right[index]))
                return false;
        }
        return true;
    }

    function containsEffectNode(kind: int): bool {
        return effectChain && effectChain.indexOf(kind) !== -1;
    }

    function defaultEditSegments(startMillis: int, endMillis: int): var {
        if (endMillis <= startMillis)
            return [];
        return [
            {
                sourceStartMillis: Math.round(startMillis),
                sourceEndMillis: Math.round(endMillis),
                state: 0,
                gainCentibels: 0,
                fadeInMillis: 0,
                fadeOutMillis: 0,
                fadeInCurve: 0,
                fadeOutCurve: 0,
                gapAfterMillis: 0
            }
        ];
    }

    function copyEditSegments(values: var, startMillis: int, endMillis: int): var {
        if (!values || values.length < 1 || values.length > 128)
            return defaultEditSegments(startMillis, endMillis);
        const result = [];
        let expectedStart = Math.round(startMillis);
        let hasOutput = false;
        for (let index = 0; index < values.length; ++index) {
            const value = values[index];
            const start = Math.round(Number(value.sourceStartMillis));
            const end = Math.round(Number(value.sourceEndMillis));
            const state = Math.round(Number(value.state));
            const gain = Math.round(Number(value.gainCentibels));
            const fadeIn = Math.round(Number(value.fadeInMillis));
            const fadeOut = Math.round(Number(value.fadeOutMillis));
            const fadeInCurve = Math.round(Number(value.fadeInCurve));
            const fadeOutCurve = Math.round(Number(value.fadeOutCurve));
            const gapAfter = Math.round(Number(value.gapAfterMillis));
            if (start !== expectedStart || end <= start || end > endMillis || state < 0 || state > 2 || gain < -2400 || gain > 1200 || fadeIn < 0 || fadeOut < 0 || fadeIn + fadeOut > end - start || fadeInCurve < 0 || fadeInCurve > 2 || fadeOutCurve < 0 || fadeOutCurve > 2 || gapAfter < 0 || gapAfter > 3600000) {
                return defaultEditSegments(startMillis, endMillis);
            }
            result.push({
                sourceStartMillis: start,
                sourceEndMillis: end,
                state: state,
                gainCentibels: gain,
                fadeInMillis: fadeIn,
                fadeOutMillis: fadeOut,
                fadeInCurve: fadeInCurve,
                fadeOutCurve: fadeOutCurve,
                gapAfterMillis: gapAfter
            });
            hasOutput = hasOutput || state !== 2 || gapAfter > 0;
            expectedStart = end;
        }
        return expectedStart === endMillis && hasOutput ? result : defaultEditSegments(startMillis, endMillis);
    }

    function sameEditSegments(left: var, right: var): bool {
        if (!left || !right || left.length !== right.length)
            return false;
        const fields = ["sourceStartMillis", "sourceEndMillis", "state", "gainCentibels", "fadeInMillis", "fadeOutMillis", "fadeInCurve", "fadeOutCurve", "gapAfterMillis"];
        for (let index = 0; index < left.length; ++index) {
            for (let fieldIndex = 0; fieldIndex < fields.length; ++fieldIndex) {
                const field = fields[fieldIndex];
                if (Number(left[index][field]) !== Number(right[index][field]))
                    return false;
            }
        }
        return true;
    }

    function copyEffectMasks(values: var, chain: var, rangeStart: int, rangeEnd: int): var {
        if (!values || values.length > 64)
            return [];
        const order = copyEffectChain(chain);
        const result = [];
        for (let index = 0; index < values.length; ++index) {
            const value = values[index];
            const start = Math.round(clamp(Number(value.startMillis), rangeStart, rangeEnd));
            const end = Math.round(clamp(Number(value.endMillis), rangeStart, rangeEnd));
            const feather = Math.round(clamp(Number(value.featherMillis === undefined ? 10 : value.featherMillis), 0, 100));
            if (end <= start || !value.effectNodes)
                continue;
            const nodes = [];
            for (let chainIndex = 0; chainIndex < order.length; ++chainIndex) {
                const node = Number(order[chainIndex]);
                if (node === 4 || node === 6 || value.effectNodes.indexOf(node) < 0)
                    continue;
                nodes.push(node);
            }
            if (nodes.length === 0)
                continue;
            result.push({
                startMillis: start,
                endMillis: end,
                featherMillis: feather,
                effectNodes: nodes
            });
        }
        return result;
    }

    function sameEffectMasks(left: var, right: var): bool {
        if (!left || !right || left.length !== right.length)
            return false;
        for (let index = 0; index < left.length; ++index) {
            const a = left[index];
            const b = right[index];
            if (Number(a.startMillis) !== Number(b.startMillis) || Number(a.endMillis) !== Number(b.endMillis) || Number(a.featherMillis) !== Number(b.featherMillis) || !sameEffectChainForMask(a.effectNodes, b.effectNodes))
                return false;
        }
        return true;
    }

    function sameEffectChainForMask(left: var, right: var): bool {
        if (!left || !right || left.length !== right.length)
            return false;
        for (let index = 0; index < left.length; ++index) {
            if (Number(left[index]) !== Number(right[index]))
                return false;
        }
        return true;
    }

    function editSegmentsAreIdentity(): bool {
        return editSegments.length === 1 && Number(editSegments[0].sourceStartMillis) === trimStartMillis && Number(editSegments[0].sourceEndMillis) === trimEndMillis && Number(editSegments[0].state) === 0 && Number(editSegments[0].gainCentibels) === 0 && Number(editSegments[0].fadeInMillis) === 0 && Number(editSegments[0].fadeOutMillis) === 0 && Number(editSegments[0].gapAfterMillis) === 0;
    }

    function copyEqualizerBands(values: var): var {
        const source = values && values.length === 6 ? values : defaultEqualizerBands();
        const result = [];
        for (let index = 0; index < source.length; ++index) {
            const band = source[index];
            result.push({
                enabled: Boolean(band.enabled),
                filterKind: Math.round(clamp(Number(band.filterKind), 0, 3)),
                frequencyHertz: Math.round(clamp(Number(band.frequencyHertz), 20, 20000)),
                qHundredths: Math.round(clamp(Number(band.qHundredths), 10, 2000)),
                gainCentibels: Math.round(clamp(Number(band.gainCentibels), -1200, 1200))
            });
        }
        return result;
    }

    function sameEqualizer(left: var, right: var): bool {
        if (!left || !right || left.length !== 6 || right.length !== 6)
            return false;
        for (let index = 0; index < 6; ++index) {
            const a = left[index];
            const b = right[index];
            if (Boolean(a.enabled) !== Boolean(b.enabled) || Number(a.filterKind) !== Number(b.filterKind) || Number(a.frequencyHertz) !== Number(b.frequencyHertz) || Number(a.qHundredths) !== Number(b.qHundredths) || Number(a.gainCentibels) !== Number(b.gainCentibels))
                return false;
        }
        return true;
    }

    function equalizerIsFlat(): bool {
        for (let index = 0; index < equalizerBands.length; ++index) {
            const band = equalizerBands[index];
            if (band.enabled && (Number(band.filterKind) === 3 || Number(band.gainCentibels) !== 0))
                return false;
        }
        return true;
    }

    function snapshot(): var {
        return {
            trimStartMillis: trimStartMillis,
            trimEndMillis: trimEndMillis,
            fadeInMillis: fadeInMillis,
            fadeOutMillis: fadeOutMillis,
            fadeInCurve: fadeInCurve,
            fadeOutCurve: fadeOutCurve,
            gainCentibels: gainCentibels,
            lowCutHertz: lowCutHertz,
            restorationEnabled: restorationEnabled,
            dePlosiveEnabled: dePlosiveEnabled,
            dePlosiveFrequencyHertz: dePlosiveFrequencyHertz,
            dePlosiveSensitivityPercent: dePlosiveSensitivityPercent,
            dePlosiveReductionCentibels: dePlosiveReductionCentibels,
            dePlosiveReleaseMillis: dePlosiveReleaseMillis,
            noiseReductionEnabled: noiseReductionEnabled,
            noiseReductionCentibels: noiseReductionCentibels,
            noiseReductionSensitivityPercent: noiseReductionSensitivityPercent,
            noiseReductionSmoothingMillis: noiseReductionSmoothingMillis,
            deEsserEnabled: deEsserEnabled,
            deEsserFrequencyHertz: deEsserFrequencyHertz,
            deEsserThresholdCentibels: deEsserThresholdCentibels,
            deEsserReductionCentibels: deEsserReductionCentibels,
            deHumEnabled: deHumEnabled,
            deHumFundamentalHertz: deHumFundamentalHertz,
            deHumHarmonicCount: deHumHarmonicCount,
            deHumQualityTenths: deHumQualityTenths,
            deHumDepthCentibels: deHumDepthCentibels,
            deClickEnabled: deClickEnabled,
            deClickSensitivityPercent: deClickSensitivityPercent,
            deClickMaximumClickMicroseconds: deClickMaximumClickMicroseconds,
            deClickRepairPercent: deClickRepairPercent,
            channelRepairEnabled: channelRepairEnabled,
            channelRepairInvertLeft: channelRepairInvertLeft,
            channelRepairInvertRight: channelRepairInvertRight,
            channelRepairSwapChannels: channelRepairSwapChannels,
            channelRepairMonoFoldDown: channelRepairMonoFoldDown,
            channelRepairBalancePercent: channelRepairBalancePercent,
            equalizerEnabled: equalizerEnabled,
            equalizerBands: copyEqualizerBands(equalizerBands),
            compressorEnabled: compressorEnabled,
            compressorThresholdCentibels: compressorThresholdCentibels,
            compressorRatioTenths: compressorRatioTenths,
            compressorAttackMillis: compressorAttackMillis,
            compressorReleaseMillis: compressorReleaseMillis,
            compressorMakeupCentibels: compressorMakeupCentibels,
            reverbCharacter: reverbCharacter,
            reverbEnabled: reverbEnabled,
            reverbMixPercent: reverbMixPercent,
            reverbPreDelayMillis: reverbPreDelayMillis,
            reverbDecayMillis: reverbDecayMillis,
            reverbSizePercent: reverbSizePercent,
            reverbDampingPercent: reverbDampingPercent,
            reverbLowCutHertz: reverbLowCutHertz,
            reverbHighCutHertz: reverbHighCutHertz,
            limiterEnabled: limiterEnabled,
            limiterCeilingCentibels: limiterCeilingCentibels,
            limiterReleaseMillis: limiterReleaseMillis,
            effectChain: copyEffectChain(effectChain),
            editSegments: copyEditSegments(editSegments, trimStartMillis, trimEndMillis),
            effectMasks: copyEffectMasks(effectMasks, effectChain, trimStartMillis, trimEndMillis)
        };
    }

    function copySnapshot(value: var): var {
        return {
            trimStartMillis: Number(value.trimStartMillis),
            trimEndMillis: Number(value.trimEndMillis),
            fadeInMillis: Number(value.fadeInMillis),
            fadeOutMillis: Number(value.fadeOutMillis),
            fadeInCurve: Number(value.fadeInCurve),
            fadeOutCurve: Number(value.fadeOutCurve),
            gainCentibels: Number(value.gainCentibels),
            lowCutHertz: Number(value.lowCutHertz),
            restorationEnabled: Boolean(value.restorationEnabled),
            dePlosiveEnabled: Boolean(value.dePlosiveEnabled),
            dePlosiveFrequencyHertz: Number(value.dePlosiveFrequencyHertz),
            dePlosiveSensitivityPercent: Number(value.dePlosiveSensitivityPercent),
            dePlosiveReductionCentibels: Number(value.dePlosiveReductionCentibels),
            dePlosiveReleaseMillis: Number(value.dePlosiveReleaseMillis),
            noiseReductionEnabled: Boolean(value.noiseReductionEnabled),
            noiseReductionCentibels: Number(value.noiseReductionCentibels),
            noiseReductionSensitivityPercent: Number(value.noiseReductionSensitivityPercent),
            noiseReductionSmoothingMillis: Number(value.noiseReductionSmoothingMillis),
            deEsserEnabled: Boolean(value.deEsserEnabled),
            deEsserFrequencyHertz: Number(value.deEsserFrequencyHertz),
            deEsserThresholdCentibels: Number(value.deEsserThresholdCentibels),
            deEsserReductionCentibels: Number(value.deEsserReductionCentibels),
            deHumEnabled: Boolean(value.deHumEnabled),
            deHumFundamentalHertz: Number(value.deHumFundamentalHertz),
            deHumHarmonicCount: Number(value.deHumHarmonicCount),
            deHumQualityTenths: Number(value.deHumQualityTenths),
            deHumDepthCentibels: Number(value.deHumDepthCentibels),
            deClickEnabled: Boolean(value.deClickEnabled),
            deClickSensitivityPercent: Number(value.deClickSensitivityPercent),
            deClickMaximumClickMicroseconds: Number(value.deClickMaximumClickMicroseconds),
            deClickRepairPercent: Number(value.deClickRepairPercent),
            channelRepairEnabled: Boolean(value.channelRepairEnabled),
            channelRepairInvertLeft: Boolean(value.channelRepairInvertLeft),
            channelRepairInvertRight: Boolean(value.channelRepairInvertRight),
            channelRepairSwapChannels: Boolean(value.channelRepairSwapChannels),
            channelRepairMonoFoldDown: Boolean(value.channelRepairMonoFoldDown),
            channelRepairBalancePercent: Number(value.channelRepairBalancePercent),
            equalizerEnabled: Boolean(value.equalizerEnabled),
            equalizerBands: copyEqualizerBands(value.equalizerBands),
            compressorEnabled: Boolean(value.compressorEnabled),
            compressorThresholdCentibels: Number(value.compressorThresholdCentibels),
            compressorRatioTenths: Number(value.compressorRatioTenths),
            compressorAttackMillis: Number(value.compressorAttackMillis),
            compressorReleaseMillis: Number(value.compressorReleaseMillis),
            compressorMakeupCentibels: Number(value.compressorMakeupCentibels),
            reverbCharacter: Number(value.reverbCharacter),
            reverbEnabled: Boolean(value.reverbEnabled),
            reverbMixPercent: Number(value.reverbMixPercent),
            reverbPreDelayMillis: Number(value.reverbPreDelayMillis),
            reverbDecayMillis: Number(value.reverbDecayMillis),
            reverbSizePercent: Number(value.reverbSizePercent),
            reverbDampingPercent: Number(value.reverbDampingPercent),
            reverbLowCutHertz: Number(value.reverbLowCutHertz),
            reverbHighCutHertz: Number(value.reverbHighCutHertz),
            limiterEnabled: Boolean(value.limiterEnabled),
            limiterCeilingCentibels: Number(value.limiterCeilingCentibels),
            limiterReleaseMillis: Number(value.limiterReleaseMillis),
            effectChain: copyEffectChain(value.effectChain),
            editSegments: copyEditSegments(value.editSegments, Number(value.trimStartMillis), Number(value.trimEndMillis)),
            effectMasks: copyEffectMasks(value.effectMasks, value.effectChain, Number(value.trimStartMillis), Number(value.trimEndMillis))
        };
    }

    function sameSnapshot(left: var, right: var): bool {
        if (!left || !right)
            return false;
        return Number(left.trimStartMillis) === Number(right.trimStartMillis) && Number(left.trimEndMillis) === Number(right.trimEndMillis) && Number(left.fadeInMillis) === Number(right.fadeInMillis) && Number(left.fadeOutMillis) === Number(right.fadeOutMillis) && Number(left.fadeInCurve) === Number(right.fadeInCurve) && Number(left.fadeOutCurve) === Number(right.fadeOutCurve) && Number(left.gainCentibels) === Number(right.gainCentibels) && Number(left.lowCutHertz) === Number(right.lowCutHertz) && Boolean(left.restorationEnabled) === Boolean(right.restorationEnabled) && Boolean(left.dePlosiveEnabled) === Boolean(right.dePlosiveEnabled) && Number(left.dePlosiveFrequencyHertz) === Number(right.dePlosiveFrequencyHertz) && Number(left.dePlosiveSensitivityPercent) === Number(right.dePlosiveSensitivityPercent) && Number(left.dePlosiveReductionCentibels) === Number(right.dePlosiveReductionCentibels) && Number(left.dePlosiveReleaseMillis) === Number(right.dePlosiveReleaseMillis) && Boolean(left.noiseReductionEnabled) === Boolean(right.noiseReductionEnabled) && Number(left.noiseReductionCentibels) === Number(right.noiseReductionCentibels) && Number(left.noiseReductionSensitivityPercent) === Number(right.noiseReductionSensitivityPercent) && Number(left.noiseReductionSmoothingMillis) === Number(right.noiseReductionSmoothingMillis) && Boolean(left.deEsserEnabled) === Boolean(right.deEsserEnabled) && Number(left.deEsserFrequencyHertz) === Number(right.deEsserFrequencyHertz) && Number(left.deEsserThresholdCentibels) === Number(right.deEsserThresholdCentibels) && Number(left.deEsserReductionCentibels) === Number(right.deEsserReductionCentibels) && Boolean(left.deHumEnabled) === Boolean(right.deHumEnabled) && Number(left.deHumFundamentalHertz) === Number(right.deHumFundamentalHertz) && Number(left.deHumHarmonicCount) === Number(right.deHumHarmonicCount) && Number(left.deHumQualityTenths) === Number(right.deHumQualityTenths) && Number(left.deHumDepthCentibels) === Number(right.deHumDepthCentibels) && Boolean(left.deClickEnabled) === Boolean(right.deClickEnabled) && Number(left.deClickSensitivityPercent) === Number(right.deClickSensitivityPercent) && Number(left.deClickMaximumClickMicroseconds) === Number(right.deClickMaximumClickMicroseconds) && Number(left.deClickRepairPercent) === Number(right.deClickRepairPercent) && Boolean(left.channelRepairEnabled) === Boolean(right.channelRepairEnabled) && Boolean(left.channelRepairInvertLeft) === Boolean(right.channelRepairInvertLeft) && Boolean(left.channelRepairInvertRight) === Boolean(right.channelRepairInvertRight) && Boolean(left.channelRepairSwapChannels) === Boolean(right.channelRepairSwapChannels) && Boolean(left.channelRepairMonoFoldDown) === Boolean(right.channelRepairMonoFoldDown) && Number(left.channelRepairBalancePercent) === Number(right.channelRepairBalancePercent) && Boolean(left.equalizerEnabled) === Boolean(right.equalizerEnabled) && sameEqualizer(left.equalizerBands, right.equalizerBands) && Boolean(left.compressorEnabled) === Boolean(right.compressorEnabled) && Number(left.compressorThresholdCentibels) === Number(right.compressorThresholdCentibels) && Number(left.compressorRatioTenths) === Number(right.compressorRatioTenths) && Number(left.compressorAttackMillis) === Number(right.compressorAttackMillis) && Number(left.compressorReleaseMillis) === Number(right.compressorReleaseMillis) && Number(left.compressorMakeupCentibels) === Number(right.compressorMakeupCentibels) && Number(left.reverbCharacter) === Number(right.reverbCharacter) && Boolean(left.reverbEnabled) === Boolean(right.reverbEnabled) && Number(left.reverbMixPercent) === Number(right.reverbMixPercent) && Number(left.reverbPreDelayMillis) === Number(right.reverbPreDelayMillis) && Number(left.reverbDecayMillis) === Number(right.reverbDecayMillis) && Number(left.reverbSizePercent) === Number(right.reverbSizePercent) && Number(left.reverbDampingPercent) === Number(right.reverbDampingPercent) && Number(left.reverbLowCutHertz) === Number(right.reverbLowCutHertz) && Number(left.reverbHighCutHertz) === Number(right.reverbHighCutHertz) && Boolean(left.limiterEnabled) === Boolean(right.limiterEnabled) && Number(left.limiterCeilingCentibels) === Number(right.limiterCeilingCentibels) && Number(left.limiterReleaseMillis) === Number(right.limiterReleaseMillis) && sameEffectChain(left.effectChain, right.effectChain) && sameEditSegments(left.editSegments, right.editSegments) && sameEffectMasks(left.effectMasks, right.effectMasks);
    }

    function assetSnapshot(): var {
        if (!asset) {
            return {
                trimStartMillis: 0,
                trimEndMillis: 0,
                fadeInMillis: 0,
                fadeOutMillis: 0,
                fadeInCurve: 0,
                fadeOutCurve: 0,
                gainCentibels: 0,
                lowCutHertz: 0,
                restorationEnabled: true,
                dePlosiveEnabled: false,
                dePlosiveFrequencyHertz: 140,
                dePlosiveSensitivityPercent: 50,
                dePlosiveReductionCentibels: 1200,
                dePlosiveReleaseMillis: 160,
                noiseReductionEnabled: false,
                noiseReductionCentibels: 900,
                noiseReductionSensitivityPercent: 50,
                noiseReductionSmoothingMillis: 240,
                deEsserEnabled: false,
                deEsserFrequencyHertz: 6500,
                deEsserThresholdCentibels: -2400,
                deEsserReductionCentibels: 600,
                deHumEnabled: false,
                deHumFundamentalHertz: 50,
                deHumHarmonicCount: 4,
                deHumQualityTenths: 300,
                deHumDepthCentibels: 2400,
                deClickEnabled: false,
                deClickSensitivityPercent: 50,
                deClickMaximumClickMicroseconds: 1000,
                deClickRepairPercent: 100,
                channelRepairEnabled: false,
                channelRepairInvertLeft: false,
                channelRepairInvertRight: false,
                channelRepairSwapChannels: false,
                channelRepairMonoFoldDown: false,
                channelRepairBalancePercent: 0,
                equalizerEnabled: true,
                equalizerBands: defaultEqualizerBands(),
                compressorEnabled: false,
                compressorThresholdCentibels: -1800,
                compressorRatioTenths: 30,
                compressorAttackMillis: 10,
                compressorReleaseMillis: 120,
                compressorMakeupCentibels: 0,
                reverbCharacter: 0,
                reverbEnabled: false,
                reverbMixPercent: 18,
                reverbPreDelayMillis: 20,
                reverbDecayMillis: 1800,
                reverbSizePercent: 55,
                reverbDampingPercent: 45,
                reverbLowCutHertz: 120,
                reverbHighCutHertz: 10000,
                limiterEnabled: false,
                limiterCeilingCentibels: -100,
                limiterReleaseMillis: 100,
                effectChain: defaultEffectChain(),
                editSegments: [],
                effectMasks: []
            };
        }
        const duration = Math.max(0, Number(asset.durationMillis));
        return {
            trimStartMillis: Math.max(0, Number(asset.trimStartMillis)),
            trimEndMillis: Number(asset.trimEndMillis) > 0 ? Number(asset.trimEndMillis) : duration,
            fadeInMillis: Math.max(0, Number(asset.fadeInMillis)),
            fadeOutMillis: Math.max(0, Number(asset.fadeOutMillis)),
            fadeInCurve: clamp(Number(asset.fadeInCurve || 0), 0, 2),
            fadeOutCurve: clamp(Number(asset.fadeOutCurve || 0), 0, 2),
            gainCentibels: clamp(Number(asset.gainCentibels), -2400, 1200),
            lowCutHertz: Number(asset.lowCutHertz) === 0 ? 0 : clamp(Number(asset.lowCutHertz), 20, 240),
            restorationEnabled: asset.restorationEnabled === undefined ? true : Boolean(asset.restorationEnabled),
            dePlosiveEnabled: Boolean(asset.dePlosiveEnabled),
            dePlosiveFrequencyHertz: clamp(Number(asset.dePlosiveFrequencyHertz ?? 140), 80, 240),
            dePlosiveSensitivityPercent: clamp(Number(asset.dePlosiveSensitivityPercent ?? 50), 0, 100),
            dePlosiveReductionCentibels: clamp(Number(asset.dePlosiveReductionCentibels ?? 1200), 0, 1800),
            dePlosiveReleaseMillis: clamp(Number(asset.dePlosiveReleaseMillis ?? 160), 40, 500),
            noiseReductionEnabled: Boolean(asset.noiseReductionEnabled),
            noiseReductionCentibels: clamp(Number(asset.noiseReductionCentibels ?? 900), 0, 2400),
            noiseReductionSensitivityPercent: clamp(Number(asset.noiseReductionSensitivityPercent ?? 50), 0, 100),
            noiseReductionSmoothingMillis: clamp(Number(asset.noiseReductionSmoothingMillis ?? 240), 20, 1000),
            deEsserEnabled: Boolean(asset.deEsserEnabled),
            deEsserFrequencyHertz: clamp(Number(asset.deEsserFrequencyHertz ?? 6500), 3000, 12000),
            deEsserThresholdCentibels: clamp(Number(asset.deEsserThresholdCentibels ?? -2400), -6000, 0),
            deEsserReductionCentibels: clamp(Number(asset.deEsserReductionCentibels ?? 600), 0, 1800),
            deHumEnabled: Boolean(asset.deHumEnabled),
            deHumFundamentalHertz: Number(asset.deHumFundamentalHertz) === 60 ? 60 : 50,
            deHumHarmonicCount: clamp(Number(asset.deHumHarmonicCount ?? 4), 1, 8),
            deHumQualityTenths: clamp(Number(asset.deHumQualityTenths ?? 300), 50, 1000),
            deHumDepthCentibels: clamp(Number(asset.deHumDepthCentibels ?? 2400), 0, 4800),
            deClickEnabled: Boolean(asset.deClickEnabled),
            deClickSensitivityPercent: clamp(Number(asset.deClickSensitivityPercent ?? 50), 0, 100),
            deClickMaximumClickMicroseconds: clamp(Number(asset.deClickMaximumClickMicroseconds ?? 1000), 50, 2000),
            deClickRepairPercent: clamp(Number(asset.deClickRepairPercent ?? 100), 0, 100),
            channelRepairEnabled: Boolean(asset.channelRepairEnabled),
            channelRepairInvertLeft: Boolean(asset.channelRepairInvertLeft),
            channelRepairInvertRight: Boolean(asset.channelRepairInvertRight),
            channelRepairSwapChannels: Boolean(asset.channelRepairSwapChannels),
            channelRepairMonoFoldDown: Boolean(asset.channelRepairMonoFoldDown),
            channelRepairBalancePercent: clamp(Number(asset.channelRepairBalancePercent ?? 0), -100, 100),
            equalizerEnabled: asset.equalizerEnabled === undefined ? true : Boolean(asset.equalizerEnabled),
            equalizerBands: copyEqualizerBands(asset.equalizerBands),
            compressorEnabled: Boolean(asset.compressorEnabled),
            compressorThresholdCentibels: clamp(Number(asset.compressorThresholdCentibels ?? -1800), -6000, 0),
            compressorRatioTenths: clamp(Number(asset.compressorRatioTenths ?? 30), 10, 200),
            compressorAttackMillis: clamp(Number(asset.compressorAttackMillis ?? 10), 1, 200),
            compressorReleaseMillis: clamp(Number(asset.compressorReleaseMillis ?? 120), 20, 2000),
            compressorMakeupCentibels: clamp(Number(asset.compressorMakeupCentibels ?? 0), 0, 2400),
            reverbCharacter: clamp(Number(asset.reverbCharacter ?? 0), 0, 2),
            reverbEnabled: Boolean(asset.reverbEnabled),
            reverbMixPercent: clamp(Number(asset.reverbMixPercent ?? 18), 0, 100),
            reverbPreDelayMillis: clamp(Number(asset.reverbPreDelayMillis ?? 20), 0, 200),
            reverbDecayMillis: clamp(Number(asset.reverbDecayMillis ?? 1800), 100, 12000),
            reverbSizePercent: clamp(Number(asset.reverbSizePercent ?? 55), 10, 100),
            reverbDampingPercent: clamp(Number(asset.reverbDampingPercent ?? 45), 0, 100),
            reverbLowCutHertz: clamp(Number(asset.reverbLowCutHertz ?? 120), 20, 1000),
            reverbHighCutHertz: clamp(Number(asset.reverbHighCutHertz ?? 10000), 1000, 20000),
            limiterEnabled: Boolean(asset.limiterEnabled),
            limiterCeilingCentibels: clamp(Number(asset.limiterCeilingCentibels ?? -100), -600, 0),
            limiterReleaseMillis: clamp(Number(asset.limiterReleaseMillis ?? 100), 20, 1000),
            effectChain: copyEffectChain(asset.effectChain),
            editSegments: copyEditSegments(asset.editSegments, Math.max(0, Number(asset.trimStartMillis)), Number(asset.trimEndMillis) > 0 ? Number(asset.trimEndMillis) : duration),
            effectMasks: copyEffectMasks(asset.effectMasks, asset.effectChain, Math.max(0, Number(asset.trimStartMillis)), Number(asset.trimEndMillis) > 0 ? Number(asset.trimEndMillis) : duration)
        };
    }

    function applySnapshot(value: var): void {
        _restoring = true;
        trimStartMillis = Number(value.trimStartMillis);
        trimEndMillis = Number(value.trimEndMillis);
        fadeInMillis = Number(value.fadeInMillis);
        fadeOutMillis = Number(value.fadeOutMillis);
        fadeInCurve = Number(value.fadeInCurve);
        fadeOutCurve = Number(value.fadeOutCurve);
        gainCentibels = Number(value.gainCentibels);
        lowCutHertz = Number(value.lowCutHertz);
        restorationEnabled = Boolean(value.restorationEnabled);
        dePlosiveEnabled = Boolean(value.dePlosiveEnabled);
        dePlosiveFrequencyHertz = Number(value.dePlosiveFrequencyHertz);
        dePlosiveSensitivityPercent = Number(value.dePlosiveSensitivityPercent);
        dePlosiveReductionCentibels = Number(value.dePlosiveReductionCentibels);
        dePlosiveReleaseMillis = Number(value.dePlosiveReleaseMillis);
        noiseReductionEnabled = Boolean(value.noiseReductionEnabled);
        noiseReductionCentibels = Number(value.noiseReductionCentibels);
        noiseReductionSensitivityPercent = Number(value.noiseReductionSensitivityPercent);
        noiseReductionSmoothingMillis = Number(value.noiseReductionSmoothingMillis);
        deEsserEnabled = Boolean(value.deEsserEnabled);
        deEsserFrequencyHertz = Number(value.deEsserFrequencyHertz);
        deEsserThresholdCentibels = Number(value.deEsserThresholdCentibels);
        deEsserReductionCentibels = Number(value.deEsserReductionCentibels);
        deHumEnabled = Boolean(value.deHumEnabled);
        deHumFundamentalHertz = Number(value.deHumFundamentalHertz);
        deHumHarmonicCount = Number(value.deHumHarmonicCount);
        deHumQualityTenths = Number(value.deHumQualityTenths);
        deHumDepthCentibels = Number(value.deHumDepthCentibels);
        deClickEnabled = Boolean(value.deClickEnabled);
        deClickSensitivityPercent = Number(value.deClickSensitivityPercent);
        deClickMaximumClickMicroseconds = Number(value.deClickMaximumClickMicroseconds);
        deClickRepairPercent = Number(value.deClickRepairPercent);
        channelRepairEnabled = Boolean(value.channelRepairEnabled);
        channelRepairInvertLeft = Boolean(value.channelRepairInvertLeft);
        channelRepairInvertRight = Boolean(value.channelRepairInvertRight);
        channelRepairSwapChannels = Boolean(value.channelRepairSwapChannels);
        channelRepairMonoFoldDown = Boolean(value.channelRepairMonoFoldDown);
        channelRepairBalancePercent = Number(value.channelRepairBalancePercent);
        equalizerEnabled = Boolean(value.equalizerEnabled);
        equalizerBands = copyEqualizerBands(value.equalizerBands);
        compressorEnabled = Boolean(value.compressorEnabled);
        compressorThresholdCentibels = Number(value.compressorThresholdCentibels);
        compressorRatioTenths = Number(value.compressorRatioTenths);
        compressorAttackMillis = Number(value.compressorAttackMillis);
        compressorReleaseMillis = Number(value.compressorReleaseMillis);
        compressorMakeupCentibels = Number(value.compressorMakeupCentibels);
        reverbCharacter = Number(value.reverbCharacter);
        reverbEnabled = Boolean(value.reverbEnabled);
        reverbMixPercent = Number(value.reverbMixPercent);
        reverbPreDelayMillis = Number(value.reverbPreDelayMillis);
        reverbDecayMillis = Number(value.reverbDecayMillis);
        reverbSizePercent = Number(value.reverbSizePercent);
        reverbDampingPercent = Number(value.reverbDampingPercent);
        reverbLowCutHertz = Number(value.reverbLowCutHertz);
        reverbHighCutHertz = Number(value.reverbHighCutHertz);
        limiterEnabled = Boolean(value.limiterEnabled);
        limiterCeilingCentibels = Number(value.limiterCeilingCentibels);
        limiterReleaseMillis = Number(value.limiterReleaseMillis);
        effectChain = copyEffectChain(value.effectChain);
        editSegments = copyEditSegments(value.editSegments, trimStartMillis, trimEndMillis);
        effectMasks = copyEffectMasks(value.effectMasks, effectChain, trimStartMillis, trimEndMillis);
        _restoring = false;
    }

    function resetFromAsset(): void {
        const persisted = assetSnapshot();
        applySnapshot(persisted);
        _savedSnapshot = copySnapshot(persisted);
        _history = [copySnapshot(persisted)];
        _historyIndex = 0;
        _gestureStart = null;
    }

    function pushCurrent(): void {
        if (_restoring || _gestureStart !== null)
            return;
        const current = snapshot();
        if (_historyIndex >= 0 && sameSnapshot(current, _history[_historyIndex]))
            return;
        const next = _history.slice(0, _historyIndex + 1);
        next.push(copySnapshot(current));
        _history = next;
        _historyIndex = next.length - 1;
    }

    function beginGesture(): void {
        if (_gestureStart === null) {
            _gestureStart = copySnapshot(snapshot());
        }
    }

    function endGesture(): void {
        if (_gestureStart === null)
            return;
        const start = _gestureStart;
        _gestureStart = null;
        if (!sameSnapshot(start, snapshot()))
            pushCurrent();
    }

    function cancelGesture(): void {
        if (_gestureStart === null)
            return;
        const start = _gestureStart;
        _gestureStart = null;
        applySnapshot(start);
    }

    function undo(): void {
        if (!canUndo)
            return;
        _historyIndex -= 1;
        applySnapshot(_history[_historyIndex]);
    }

    function redo(): void {
        if (!canRedo)
            return;
        _historyIndex += 1;
        applySnapshot(_history[_historyIndex]);
    }

    function setTrimRange(startMillis: int, endMillis: int): void {
        const minimumDuration = Math.min(50, sourceDurationMillis);
        const start = Math.round(clamp(startMillis, 0, Math.max(0, sourceDurationMillis - minimumDuration)));
        const end = Math.round(clamp(endMillis, start + minimumDuration, sourceDurationMillis));
        const previousSegments = editSegments;
        trimStartMillis = start;
        trimEndMillis = end;
        editSegments = rebaseEditSegments(previousSegments, start, end);
        effectMasks = copyEffectMasks(effectMasks, effectChain, start, end);
        fadeInMillis = Math.min(fadeInMillis, selectedDurationMillis);
        fadeOutMillis = Math.min(fadeOutMillis, Math.max(0, selectedDurationMillis - fadeInMillis));
        pushCurrent();
    }

    function cloneEditSegment(value: var): var {
        return {
            sourceStartMillis: Number(value.sourceStartMillis),
            sourceEndMillis: Number(value.sourceEndMillis),
            state: Number(value.state),
            gainCentibels: Number(value.gainCentibels),
            fadeInMillis: Number(value.fadeInMillis),
            fadeOutMillis: Number(value.fadeOutMillis),
            fadeInCurve: Number(value.fadeInCurve),
            fadeOutCurve: Number(value.fadeOutCurve),
            gapAfterMillis: Number(value.gapAfterMillis)
        };
    }

    function rebaseEditSegments(values: var, startMillis: int, endMillis: int): var {
        const result = [];
        for (let index = 0; values && index < values.length; ++index) {
            const source = values[index];
            const start = Math.max(startMillis, Number(source.sourceStartMillis));
            const end = Math.min(endMillis, Number(source.sourceEndMillis));
            if (end <= start)
                continue;
            const clippedStart = start > Number(source.sourceStartMillis);
            const clippedEnd = end < Number(source.sourceEndMillis);
            result.push({
                sourceStartMillis: start,
                sourceEndMillis: end,
                state: Number(source.state),
                gainCentibels: Number(source.gainCentibels),
                fadeInMillis: clippedStart ? 0 : Math.min(Number(source.fadeInMillis), end - start),
                fadeOutMillis: clippedEnd ? 0 : Math.min(Number(source.fadeOutMillis), end - start),
                fadeInCurve: Number(source.fadeInCurve),
                fadeOutCurve: Number(source.fadeOutCurve),
                gapAfterMillis: clippedEnd ? 0 : Number(source.gapAfterMillis)
            });
        }
        return copyEditSegments(result, startMillis, endMillis);
    }

    function splitSegmentsAt(values: var, millis: int): var {
        const point = Math.round(clamp(millis, trimStartMillis, trimEndMillis));
        const result = [];
        for (let index = 0; index < values.length; ++index) {
            const segment = cloneEditSegment(values[index]);
            if (point <= segment.sourceStartMillis || point >= segment.sourceEndMillis) {
                result.push(segment);
                continue;
            }
            const left = cloneEditSegment(segment);
            const right = cloneEditSegment(segment);
            left.sourceEndMillis = point;
            left.fadeOutMillis = 0;
            left.gapAfterMillis = 0;
            right.sourceStartMillis = point;
            right.fadeInMillis = 0;
            result.push(left);
            result.push(right);
        }
        return result;
    }

    function splitSelection(startMillis: int, endMillis: int): void {
        const start = Math.round(clamp(startMillis, trimStartMillis, trimEndMillis));
        const end = Math.round(clamp(endMillis, trimStartMillis, trimEndMillis));
        if (end <= start)
            return;
        const withStart = splitSegmentsAt(editSegments, start);
        editSegments = splitSegmentsAt(withStart, end);
        pushCurrent();
    }

    function setSelectionState(startMillis: int, endMillis: int, state: int): void {
        const start = Math.round(clamp(startMillis, trimStartMillis, trimEndMillis));
        const end = Math.round(clamp(endMillis, trimStartMillis, trimEndMillis));
        const requestedState = Math.round(clamp(state, 0, 2));
        if (end <= start)
            return;
        let next = splitSegmentsAt(editSegments, start);
        next = splitSegmentsAt(next, end);
        for (let index = 0; index < next.length; ++index) {
            const segment = cloneEditSegment(next[index]);
            if (segment.sourceStartMillis >= start && segment.sourceEndMillis <= end)
                segment.state = requestedState;
            next[index] = segment;
        }
        let hasOutput = false;
        for (let index = 0; index < next.length; ++index) {
            hasOutput = hasOutput || Number(next[index].state) !== 2 || Number(next[index].gapAfterMillis) > 0;
        }
        if (!hasOutput)
            return;
        editSegments = next;
        pushCurrent();
    }

    function insertGap(millis: int, durationMillis: int): void {
        const point = Math.round(clamp(millis, trimStartMillis, trimEndMillis));
        const duration = Math.round(clamp(durationMillis, 10, 3600000));
        let next = splitSegmentsAt(editSegments, point);
        let target = -1;
        for (let index = 0; index < next.length; ++index) {
            if (Number(next[index].sourceEndMillis) === point)
                target = index;
        }
        if (target < 0 && point === trimStartMillis)
            target = 0;
        if (target < 0)
            return;
        const segment = cloneEditSegment(next[target]);
        segment.gapAfterMillis = Math.round(clamp(segment.gapAfterMillis + duration, 0, 3600000));
        next[target] = segment;
        editSegments = next;
        pushCurrent();
    }

    function setSegmentGap(index: int, durationMillis: int): bool {
        if (index < 0 || index >= editSegments.length)
            return false;
        const next = copyEditSegments(editSegments, trimStartMillis, trimEndMillis);
        const segment = cloneEditSegment(next[index]);
        segment.gapAfterMillis = Math.round(clamp(durationMillis, 0, 3600000));
        next[index] = segment;
        let hasOutput = false;
        for (let segmentIndex = 0; segmentIndex < next.length; ++segmentIndex)
            hasOutput = hasOutput || Number(next[segmentIndex].state) !== 2 || Number(next[segmentIndex].gapAfterMillis) > 0;
        if (!hasOutput)
            return false;
        editSegments = next;
        pushCurrent();
        return true;
    }

    function canRemoveSegmentGap(index: int): bool {
        if (index < 0 || index >= editSegments.length)
            return false;
        for (let segmentIndex = 0; segmentIndex < editSegments.length; ++segmentIndex) {
            const segment = editSegments[segmentIndex];
            if (Number(segment.state) !== 2 || (segmentIndex !== index && Number(segment.gapAfterMillis) > 0))
                return true;
        }
        return false;
    }

    function setSegmentGain(index: int, centibels: int): void {
        if (index < 0 || index >= editSegments.length)
            return;
        const next = copyEditSegments(editSegments, trimStartMillis, trimEndMillis);
        const segment = cloneEditSegment(next[index]);
        segment.gainCentibels = Math.round(clamp(centibels, -2400, 1200));
        next[index] = segment;
        editSegments = next;
        pushCurrent();
    }

    function addEffectMask(startMillis: int, endMillis: int, featherMillis: int, effectNodes: var): void {
        const values = effectMasks.slice();
        values.push({
            startMillis: startMillis,
            endMillis: endMillis,
            featherMillis: featherMillis,
            effectNodes: effectNodes
        });
        effectMasks = copyEffectMasks(values, effectChain, trimStartMillis, trimEndMillis);
        pushCurrent();
    }

    function updateEffectMask(index: int, startMillis: int, endMillis: int, featherMillis: int, effectNodes: var): void {
        if (index < 0 || index >= effectMasks.length)
            return;
        const values = effectMasks.slice();
        values[index] = {
            startMillis: startMillis,
            endMillis: endMillis,
            featherMillis: featherMillis,
            effectNodes: effectNodes
        };
        effectMasks = copyEffectMasks(values, effectChain, trimStartMillis, trimEndMillis);
        pushCurrent();
    }

    function removeEffectMask(index: int): void {
        if (index < 0 || index >= effectMasks.length)
            return;
        const values = effectMasks.slice();
        values.splice(index, 1);
        effectMasks = values;
        pushCurrent();
    }

    function setFades(fadeIn: int, fadeOut: int): void {
        fadeInMillis = Math.round(clamp(fadeIn, 0, selectedDurationMillis));
        fadeOutMillis = Math.round(clamp(fadeOut, 0, Math.max(0, selectedDurationMillis - fadeInMillis)));
        pushCurrent();
    }

    function setFadeCurves(fadeIn: int, fadeOut: int): void {
        fadeInCurve = Math.round(clamp(fadeIn, 0, 2));
        fadeOutCurve = Math.round(clamp(fadeOut, 0, 2));
        pushCurrent();
    }

    function setGain(centibels: int): void {
        gainCentibels = Math.round(clamp(centibels, -2400, 1200));
        pushCurrent();
    }

    function setLowCut(hertz: int): void {
        lowCutHertz = hertz === 0 ? 0 : Math.round(clamp(hertz, 20, 240));
        pushCurrent();
    }

    function restorationValue(): var {
        return {
            enabled: restorationEnabled,
            dePlosiveEnabled: dePlosiveEnabled,
            dePlosiveFrequencyHertz: dePlosiveFrequencyHertz,
            dePlosiveSensitivityPercent: dePlosiveSensitivityPercent,
            dePlosiveReductionCentibels: dePlosiveReductionCentibels,
            dePlosiveReleaseMillis: dePlosiveReleaseMillis,
            noiseEnabled: noiseReductionEnabled,
            noiseReductionCentibels: noiseReductionCentibels,
            noiseSensitivityPercent: noiseReductionSensitivityPercent,
            noiseSmoothingMillis: noiseReductionSmoothingMillis,
            deEsserEnabled: deEsserEnabled,
            deEsserFrequencyHertz: deEsserFrequencyHertz,
            deEsserThresholdCentibels: deEsserThresholdCentibels,
            deEsserReductionCentibels: deEsserReductionCentibels
        };
    }

    function deHumValue(): var {
        return {
            enabled: deHumEnabled,
            fundamentalHertz: deHumFundamentalHertz,
            harmonicCount: deHumHarmonicCount,
            qualityTenths: deHumQualityTenths,
            depthCentibels: deHumDepthCentibels
        };
    }

    function deClickValue(): var {
        return {
            enabled: deClickEnabled,
            sensitivityPercent: deClickSensitivityPercent,
            maximumClickMicroseconds: deClickMaximumClickMicroseconds,
            repairPercent: deClickRepairPercent
        };
    }

    function channelRepairValue(): var {
        return {
            enabled: channelRepairEnabled,
            invertLeft: channelRepairInvertLeft,
            invertRight: channelRepairInvertRight,
            swapChannels: channelRepairSwapChannels,
            monoFoldDown: channelRepairMonoFoldDown,
            balancePercent: channelRepairBalancePercent
        };
    }

    function effectNodeEnabled(kind: int): bool {
        if (kind === 0)
            return restorationEnabled;
        if (kind === 1)
            return equalizerEnabled;
        if (kind === 2)
            return compressorEnabled;
        if (kind === 3)
            return reverbEnabled;
        if (kind === 4)
            return limiterEnabled;
        if (kind === 5)
            return deHumEnabled;
        if (kind === 6)
            return deClickEnabled;
        if (kind === 7)
            return channelRepairEnabled;
        return false;
    }

    function setEffectNodeEnabled(kind: int, enabled: bool): void {
        if (kind === 0)
            restorationEnabled = enabled;
        else if (kind === 1)
            equalizerEnabled = enabled;
        else if (kind === 2)
            compressorEnabled = enabled;
        else if (kind === 3)
            reverbEnabled = enabled;
        else if (kind === 4)
            limiterEnabled = enabled;
        else if (kind === 5)
            deHumEnabled = enabled;
        else if (kind === 6)
            deClickEnabled = enabled;
        else if (kind === 7)
            channelRepairEnabled = enabled;
        else
            return;
        pushCurrent();
    }

    function moveEffectNode(kind: int, direction: int): void {
        if (kind === 4 || direction === 0)
            return;
        const next = copyEffectChain(effectChain);
        const from = next.indexOf(kind);
        const target = from + (direction < 0 ? -1 : 1);
        if (from < 0 || target < 0 || target >= next.length - 1)
            return;
        const displaced = next[target];
        next[target] = kind;
        next[from] = displaced;
        effectChain = next;
        effectMasks = copyEffectMasks(effectMasks, effectChain, trimStartMillis, trimEndMillis);
        pushCurrent();
    }

    function addEffectNode(kind: int): void {
        if (kind < 0 || kind > 7 || kind === 4 || containsEffectNode(kind))
            return;
        const next = copyEffectChain(effectChain);
        next.splice(next.length - 1, 0, kind);
        effectChain = next;
        if (kind === 0)
            restorationEnabled = true;
        else if (kind === 1)
            equalizerEnabled = true;
        else if (kind === 2)
            compressorEnabled = true;
        else if (kind === 3)
            reverbEnabled = true;
        else if (kind === 5)
            deHumEnabled = true;
        else if (kind === 6)
            deClickEnabled = true;
        else if (kind === 7)
            channelRepairEnabled = true;
        pushCurrent();
    }

    function removeEffectNode(kind: int): void {
        if (kind < 0 || kind > 7 || kind === 4)
            return;
        const next = copyEffectChain(effectChain);
        const index = next.indexOf(kind);
        if (index < 0)
            return;
        next.splice(index, 1);
        effectChain = next;
        effectMasks = copyEffectMasks(effectMasks, effectChain, trimStartMillis, trimEndMillis);
        pushCurrent();
    }

    function setDeHumEnabled(enabled: bool): void {
        deHumEnabled = enabled;
        pushCurrent();
    }

    function setDePlosiveEnabled(enabled: bool): void {
        dePlosiveEnabled = enabled;
        pushCurrent();
    }

    function setDePlosiveParameter(parameter: string, value: int): void {
        if (parameter === "frequency") {
            dePlosiveFrequencyHertz = Math.round(clamp(value, 80, 240));
        } else if (parameter === "sensitivity") {
            dePlosiveSensitivityPercent = Math.round(clamp(value, 0, 100));
        } else if (parameter === "reduction") {
            dePlosiveReductionCentibels = Math.round(clamp(value, 0, 1800));
        } else if (parameter === "release") {
            dePlosiveReleaseMillis = Math.round(clamp(value, 40, 500));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetDePlosive(): void {
        dePlosiveEnabled = false;
        dePlosiveFrequencyHertz = 140;
        dePlosiveSensitivityPercent = 50;
        dePlosiveReductionCentibels = 1200;
        dePlosiveReleaseMillis = 160;
        pushCurrent();
    }

    function setDeHumParameter(parameter: string, value: int): void {
        if (parameter === "fundamental") {
            deHumFundamentalHertz = Number(value) === 60 ? 60 : 50;
        } else if (parameter === "harmonics") {
            deHumHarmonicCount = Math.round(clamp(value, 1, 8));
        } else if (parameter === "quality") {
            deHumQualityTenths = Math.round(clamp(value, 50, 1000));
        } else if (parameter === "depth") {
            deHumDepthCentibels = Math.round(clamp(value, 0, 4800));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetDeHum(): void {
        deHumEnabled = false;
        deHumFundamentalHertz = 50;
        deHumHarmonicCount = 4;
        deHumQualityTenths = 300;
        deHumDepthCentibels = 2400;
        pushCurrent();
    }

    function setDeClickEnabled(enabled: bool): void {
        deClickEnabled = enabled;
        pushCurrent();
    }

    function setDeClickParameter(parameter: string, value: int): void {
        if (parameter === "sensitivity") {
            deClickSensitivityPercent = Math.round(clamp(value, 0, 100));
        } else if (parameter === "maximumClick") {
            deClickMaximumClickMicroseconds = Math.round(clamp(value, 50, 2000));
        } else if (parameter === "repair") {
            deClickRepairPercent = Math.round(clamp(value, 0, 100));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetDeClick(): void {
        deClickEnabled = false;
        deClickSensitivityPercent = 50;
        deClickMaximumClickMicroseconds = 1000;
        deClickRepairPercent = 100;
        pushCurrent();
    }

    function setChannelRepairEnabled(enabled: bool): void {
        channelRepairEnabled = enabled;
        pushCurrent();
    }

    function setChannelRepairParameter(parameter: string, value: var): void {
        if (parameter === "invertLeft") {
            channelRepairInvertLeft = Boolean(value);
        } else if (parameter === "invertRight") {
            channelRepairInvertRight = Boolean(value);
        } else if (parameter === "swapChannels") {
            channelRepairSwapChannels = Boolean(value);
        } else if (parameter === "monoFoldDown") {
            channelRepairMonoFoldDown = Boolean(value);
        } else if (parameter === "balance") {
            channelRepairBalancePercent = Math.round(clamp(Number(value), -100, 100));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetChannelRepair(): void {
        channelRepairEnabled = false;
        channelRepairInvertLeft = false;
        channelRepairInvertRight = false;
        channelRepairSwapChannels = false;
        channelRepairMonoFoldDown = false;
        channelRepairBalancePercent = 0;
        pushCurrent();
    }

    function setRestorationParameter(parameter: string, value: int): void {
        if (parameter === "noiseReduction") {
            noiseReductionCentibels = Math.round(clamp(value, 0, 2400));
        } else if (parameter === "noiseSensitivity") {
            noiseReductionSensitivityPercent = Math.round(clamp(value, 0, 100));
        } else if (parameter === "noiseSmoothing") {
            noiseReductionSmoothingMillis = Math.round(clamp(value, 20, 1000));
        } else if (parameter === "deEsserFrequency") {
            deEsserFrequencyHertz = Math.round(clamp(value, 3000, 12000));
        } else if (parameter === "deEsserThreshold") {
            deEsserThresholdCentibels = Math.round(clamp(value, -6000, 0));
        } else if (parameter === "deEsserReduction") {
            deEsserReductionCentibels = Math.round(clamp(value, 0, 1800));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetRestoration(): void {
        dePlosiveEnabled = false;
        dePlosiveFrequencyHertz = 140;
        dePlosiveSensitivityPercent = 50;
        dePlosiveReductionCentibels = 1200;
        dePlosiveReleaseMillis = 160;
        noiseReductionEnabled = false;
        noiseReductionCentibels = 900;
        noiseReductionSensitivityPercent = 50;
        noiseReductionSmoothingMillis = 240;
        deEsserEnabled = false;
        deEsserFrequencyHertz = 6500;
        deEsserThresholdCentibels = -2400;
        deEsserReductionCentibels = 600;
        pushCurrent();
    }

    function setEqualizerBand(index: int, enabled: bool, filterKind: int, frequencyHertz: int, qHundredths: int, gainCentibels: int): void {
        if (index < 0 || index >= equalizerBands.length)
            return;
        const next = copyEqualizerBands(equalizerBands);
        next[index] = {
            enabled: enabled,
            filterKind: Math.round(clamp(filterKind, 0, 3)),
            frequencyHertz: Math.round(clamp(frequencyHertz, 20, 20000)),
            qHundredths: Math.round(clamp(qHundredths, 10, 2000)),
            gainCentibels: Math.round(clamp(gainCentibels, -1200, 1200))
        };
        equalizerBands = next;
        pushCurrent();
    }

    function resetEqualizer(): void {
        equalizerBands = defaultEqualizerBands();
        pushCurrent();
    }

    function setCompressorEnabled(enabled: bool): void {
        compressorEnabled = enabled;
        pushCurrent();
    }

    function setCompressorParameter(parameter: string, value: int): void {
        if (parameter === "threshold") {
            compressorThresholdCentibels = Math.round(clamp(value, -6000, 0));
        } else if (parameter === "ratio") {
            compressorRatioTenths = Math.round(clamp(value, 10, 200));
        } else if (parameter === "attack") {
            compressorAttackMillis = Math.round(clamp(value, 1, 200));
        } else if (parameter === "release") {
            compressorReleaseMillis = Math.round(clamp(value, 20, 2000));
        } else if (parameter === "makeup") {
            compressorMakeupCentibels = Math.round(clamp(value, 0, 2400));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetCompressor(): void {
        compressorEnabled = false;
        compressorThresholdCentibels = -1800;
        compressorRatioTenths = 30;
        compressorAttackMillis = 10;
        compressorReleaseMillis = 120;
        compressorMakeupCentibels = 0;
        pushCurrent();
    }

    function reverbValue(): var {
        return {
            character: reverbCharacter,
            enabled: reverbEnabled,
            mixPercent: reverbMixPercent,
            preDelayMillis: reverbPreDelayMillis,
            decayMillis: reverbDecayMillis,
            sizePercent: reverbSizePercent,
            dampingPercent: reverbDampingPercent,
            lowCutHertz: reverbLowCutHertz,
            highCutHertz: reverbHighCutHertz
        };
    }

    function setReverbCharacter(character: int): void {
        reverbCharacter = Math.round(clamp(character, 0, 2));
        pushCurrent();
    }

    function setReverbEnabled(enabled: bool): void {
        reverbEnabled = enabled;
        pushCurrent();
    }

    function setReverbParameter(parameter: string, value: int): void {
        if (parameter === "mix") {
            reverbMixPercent = Math.round(clamp(value, 0, 100));
        } else if (parameter === "preDelay") {
            reverbPreDelayMillis = Math.round(clamp(value, 0, 200));
        } else if (parameter === "decay") {
            reverbDecayMillis = Math.round(clamp(value, 100, 12000));
        } else if (parameter === "size") {
            reverbSizePercent = Math.round(clamp(value, 10, 100));
        } else if (parameter === "damping") {
            reverbDampingPercent = Math.round(clamp(value, 0, 100));
        } else if (parameter === "lowCut") {
            reverbLowCutHertz = Math.round(clamp(value, 20, Math.min(1000, reverbHighCutHertz - 1)));
        } else if (parameter === "highCut") {
            reverbHighCutHertz = Math.round(clamp(value, Math.max(1000, reverbLowCutHertz + 1), 20000));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetReverb(): void {
        reverbEnabled = false;
        reverbMixPercent = 18;
        reverbPreDelayMillis = 20;
        reverbDecayMillis = 1800;
        reverbSizePercent = 55;
        reverbDampingPercent = 45;
        reverbLowCutHertz = 120;
        reverbHighCutHertz = 10000;
        pushCurrent();
    }

    function setLimiterEnabled(enabled: bool): void {
        limiterEnabled = enabled;
        pushCurrent();
    }

    function setLimiterParameter(parameter: string, value: int): void {
        if (parameter === "ceiling") {
            limiterCeilingCentibels = Math.round(clamp(value, -600, 0));
        } else if (parameter === "release") {
            limiterReleaseMillis = Math.round(clamp(value, 20, 1000));
        } else {
            return;
        }
        pushCurrent();
    }

    function resetLimiter(): void {
        limiterEnabled = false;
        limiterCeilingCentibels = -100;
        limiterReleaseMillis = 100;
        pushCurrent();
    }

    function clear(): void {
        applySnapshot({
            trimStartMillis: 0,
            trimEndMillis: sourceDurationMillis,
            fadeInMillis: 0,
            fadeOutMillis: 0,
            fadeInCurve: 0,
            fadeOutCurve: 0,
            gainCentibels: 0,
            lowCutHertz: 0,
            restorationEnabled: true,
            dePlosiveEnabled: false,
            dePlosiveFrequencyHertz: 140,
            dePlosiveSensitivityPercent: 50,
            dePlosiveReductionCentibels: 1200,
            dePlosiveReleaseMillis: 160,
            noiseReductionEnabled: false,
            noiseReductionCentibels: 900,
            noiseReductionSensitivityPercent: 50,
            noiseReductionSmoothingMillis: 240,
            deEsserEnabled: false,
            deEsserFrequencyHertz: 6500,
            deEsserThresholdCentibels: -2400,
            deEsserReductionCentibels: 600,
            deHumEnabled: false,
            deHumFundamentalHertz: 50,
            deHumHarmonicCount: 4,
            deHumQualityTenths: 300,
            deHumDepthCentibels: 2400,
            deClickEnabled: false,
            deClickSensitivityPercent: 50,
            deClickMaximumClickMicroseconds: 1000,
            deClickRepairPercent: 100,
            channelRepairEnabled: false,
            channelRepairInvertLeft: false,
            channelRepairInvertRight: false,
            channelRepairSwapChannels: false,
            channelRepairMonoFoldDown: false,
            channelRepairBalancePercent: 0,
            equalizerEnabled: true,
            equalizerBands: defaultEqualizerBands(),
            compressorEnabled: false,
            compressorThresholdCentibels: -1800,
            compressorRatioTenths: 30,
            compressorAttackMillis: 10,
            compressorReleaseMillis: 120,
            compressorMakeupCentibels: 0,
            reverbCharacter: reverbCharacter,
            reverbEnabled: false,
            reverbMixPercent: 18,
            reverbPreDelayMillis: 20,
            reverbDecayMillis: 1800,
            reverbSizePercent: 55,
            reverbDampingPercent: 45,
            reverbLowCutHertz: 120,
            reverbHighCutHertz: 10000,
            limiterEnabled: false,
            limiterCeilingCentibels: -100,
            limiterReleaseMillis: 100,
            effectChain: defaultEffectChain(),
            editSegments: defaultEditSegments(0, sourceDurationMillis),
            effectMasks: []
        });
        pushCurrent();
    }

    function revert(): void {
        applySnapshot(_savedSnapshot);
        pushCurrent();
    }

    function save(): void {
        if (!asset || !dirty)
            return;
        saveRequested(trimStartMillis, trimEndMillis, fadeInMillis, fadeOutMillis, fadeInCurve, fadeOutCurve, gainCentibels, lowCutHertz, restorationEnabled, dePlosiveEnabled, dePlosiveFrequencyHertz, dePlosiveSensitivityPercent, dePlosiveReductionCentibels, dePlosiveReleaseMillis, noiseReductionEnabled, noiseReductionCentibels, noiseReductionSensitivityPercent, noiseReductionSmoothingMillis, deEsserEnabled, deEsserFrequencyHertz, deEsserThresholdCentibels, deEsserReductionCentibels, deHumEnabled, deHumFundamentalHertz, deHumHarmonicCount, deHumQualityTenths, deHumDepthCentibels, deClickEnabled, deClickSensitivityPercent, deClickMaximumClickMicroseconds, deClickRepairPercent, channelRepairEnabled, channelRepairInvertLeft, channelRepairInvertRight, channelRepairSwapChannels, channelRepairMonoFoldDown, channelRepairBalancePercent, equalizerEnabled, copyEqualizerBands(equalizerBands), compressorEnabled, compressorThresholdCentibels, compressorRatioTenths, compressorAttackMillis, compressorReleaseMillis, compressorMakeupCentibels, reverbCharacter, reverbEnabled, reverbMixPercent, reverbPreDelayMillis, reverbDecayMillis, reverbSizePercent, reverbDampingPercent, reverbLowCutHertz, reverbHighCutHertz, limiterEnabled, limiterCeilingCentibels, limiterReleaseMillis, copyEffectChain(effectChain), copyEditSegments(editSegments, trimStartMillis, trimEndMillis), copyEffectMasks(effectMasks, effectChain, trimStartMillis, trimEndMillis));
    }

    function markSaved(): void {
        _savedSnapshot = copySnapshot(snapshot());
        _history = [copySnapshot(snapshot())];
        _historyIndex = 0;
        _gestureStart = null;
    }

    onAssetChanged: resetFromAsset()
    Component.onCompleted: resetFromAsset()
}

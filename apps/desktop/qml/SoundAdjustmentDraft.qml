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
    property bool noiseReductionEnabled: false
    property int noiseReductionCentibels: 900
    property int noiseReductionSensitivityPercent: 50
    property int noiseReductionSmoothingMillis: 240
    property bool deEsserEnabled: false
    property int deEsserFrequencyHertz: 6500
    property int deEsserThresholdCentibels: -2400
    property int deEsserReductionCentibels: 600
    property var equalizerBands: defaultEqualizerBands()
    property bool compressorEnabled: false
    property int compressorThresholdCentibels: -1800
    property int compressorRatioTenths: 30
    property int compressorAttackMillis: 10
    property int compressorReleaseMillis: 120
    property int compressorMakeupCentibels: 0
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

    property var _savedSnapshot: ({})
    property var _history: []
    property int _historyIndex: -1
    property var _gestureStart: null
    property bool _restoring: false

    readonly property int sourceDurationMillis: asset
        ? Math.max(0, Number(asset.durationMillis)) : 0
    readonly property int selectedDurationMillis: Math.max(0,
        trimEndMillis - trimStartMillis)
    readonly property bool canUndo: _historyIndex > 0
    readonly property bool canRedo: _historyIndex >= 0
        && _historyIndex < _history.length - 1
    readonly property bool identity: trimStartMillis === 0
        && trimEndMillis === sourceDurationMillis
        && fadeInMillis === 0 && fadeOutMillis === 0
        && fadeInCurve === 0 && fadeOutCurve === 0
        && gainCentibels === 0 && lowCutHertz === 0
        && !noiseReductionEnabled && !deEsserEnabled
        && equalizerIsFlat()
        && !compressorEnabled
        && !reverbEnabled
        && !limiterEnabled
    readonly property bool dirty: !sameSnapshot(snapshot(), _savedSnapshot)

    signal saveRequested(int startMillis, int endMillis,
                         int fadeIn, int fadeOut,
                         int fadeInCurve, int fadeOutCurve,
                         int gain, int lowCut,
                         bool noiseEnabled, int noiseReduction,
                         int noiseSensitivity, int noiseSmoothing,
                         bool deEsserEnabled, int deEsserFrequency,
                         int deEsserThreshold, int deEsserReduction,
                         var equalizerBands,
                         bool compressorEnabled, int compressorThreshold,
                         int compressorRatio, int compressorAttack,
                         int compressorRelease, int compressorMakeup,
                         bool reverbEnabled, int reverbMix,
                         int reverbPreDelay, int reverbDecay,
                         int reverbSize, int reverbDamping,
                         int reverbLowCut, int reverbHighCut,
                         bool limiterEnabled, int limiterCeiling,
                         int limiterRelease)

    function clamp(value: real, minimum: real, maximum: real) : real {
        return Math.max(minimum, Math.min(maximum, value))
    }

    function defaultEqualizerBands() : var {
        return [
            { enabled: true, filterKind: 1, frequencyHertz: 120,
              qHundredths: 71, gainCentibels: 0 },
            { enabled: false, filterKind: 0, frequencyHertz: 250,
              qHundredths: 100, gainCentibels: 0 },
            { enabled: true, filterKind: 0, frequencyHertz: 1000,
              qHundredths: 100, gainCentibels: 0 },
            { enabled: false, filterKind: 0, frequencyHertz: 3000,
              qHundredths: 100, gainCentibels: 0 },
            { enabled: false, filterKind: 0, frequencyHertz: 5000,
              qHundredths: 100, gainCentibels: 0 },
            { enabled: true, filterKind: 2, frequencyHertz: 8000,
              qHundredths: 71, gainCentibels: 0 }
        ]
    }

    function copyEqualizerBands(values: var) : var {
        const source = values && values.length === 6
            ? values : defaultEqualizerBands()
        const result = []
        for (let index = 0; index < source.length; ++index) {
            const band = source[index]
            result.push({
                enabled: Boolean(band.enabled),
                filterKind: Math.round(clamp(Number(band.filterKind), 0, 3)),
                frequencyHertz: Math.round(clamp(
                    Number(band.frequencyHertz), 20, 20000)),
                qHundredths: Math.round(clamp(
                    Number(band.qHundredths), 10, 2000)),
                gainCentibels: Math.round(clamp(
                    Number(band.gainCentibels), -1200, 1200))
            })
        }
        return result
    }

    function sameEqualizer(left: var, right: var) : bool {
        if (!left || !right || left.length !== 6 || right.length !== 6)
            return false
        for (let index = 0; index < 6; ++index) {
            const a = left[index]
            const b = right[index]
            if (Boolean(a.enabled) !== Boolean(b.enabled)
                    || Number(a.filterKind) !== Number(b.filterKind)
                    || Number(a.frequencyHertz) !== Number(b.frequencyHertz)
                    || Number(a.qHundredths) !== Number(b.qHundredths)
                    || Number(a.gainCentibels) !== Number(b.gainCentibels))
                return false
        }
        return true
    }

    function equalizerIsFlat() : bool {
        for (let index = 0; index < equalizerBands.length; ++index) {
            const band = equalizerBands[index]
            if (band.enabled && (Number(band.filterKind) === 3
                    || Number(band.gainCentibels) !== 0)) return false
        }
        return true
    }

    function snapshot() : var {
        return {
            trimStartMillis: trimStartMillis,
            trimEndMillis: trimEndMillis,
            fadeInMillis: fadeInMillis,
            fadeOutMillis: fadeOutMillis,
            fadeInCurve: fadeInCurve,
            fadeOutCurve: fadeOutCurve,
            gainCentibels: gainCentibels,
            lowCutHertz: lowCutHertz,
            noiseReductionEnabled: noiseReductionEnabled,
            noiseReductionCentibels: noiseReductionCentibels,
            noiseReductionSensitivityPercent: noiseReductionSensitivityPercent,
            noiseReductionSmoothingMillis: noiseReductionSmoothingMillis,
            deEsserEnabled: deEsserEnabled,
            deEsserFrequencyHertz: deEsserFrequencyHertz,
            deEsserThresholdCentibels: deEsserThresholdCentibels,
            deEsserReductionCentibels: deEsserReductionCentibels,
            equalizerBands: copyEqualizerBands(equalizerBands),
            compressorEnabled: compressorEnabled,
            compressorThresholdCentibels: compressorThresholdCentibels,
            compressorRatioTenths: compressorRatioTenths,
            compressorAttackMillis: compressorAttackMillis,
            compressorReleaseMillis: compressorReleaseMillis,
            compressorMakeupCentibels: compressorMakeupCentibels,
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
            limiterReleaseMillis: limiterReleaseMillis
        }
    }

    function copySnapshot(value: var) : var {
        return {
            trimStartMillis: Number(value.trimStartMillis),
            trimEndMillis: Number(value.trimEndMillis),
            fadeInMillis: Number(value.fadeInMillis),
            fadeOutMillis: Number(value.fadeOutMillis),
            fadeInCurve: Number(value.fadeInCurve),
            fadeOutCurve: Number(value.fadeOutCurve),
            gainCentibels: Number(value.gainCentibels),
            lowCutHertz: Number(value.lowCutHertz),
            noiseReductionEnabled: Boolean(value.noiseReductionEnabled),
            noiseReductionCentibels: Number(value.noiseReductionCentibels),
            noiseReductionSensitivityPercent: Number(value.noiseReductionSensitivityPercent),
            noiseReductionSmoothingMillis: Number(value.noiseReductionSmoothingMillis),
            deEsserEnabled: Boolean(value.deEsserEnabled),
            deEsserFrequencyHertz: Number(value.deEsserFrequencyHertz),
            deEsserThresholdCentibels: Number(value.deEsserThresholdCentibels),
            deEsserReductionCentibels: Number(value.deEsserReductionCentibels),
            equalizerBands: copyEqualizerBands(value.equalizerBands),
            compressorEnabled: Boolean(value.compressorEnabled),
            compressorThresholdCentibels: Number(value.compressorThresholdCentibels),
            compressorRatioTenths: Number(value.compressorRatioTenths),
            compressorAttackMillis: Number(value.compressorAttackMillis),
            compressorReleaseMillis: Number(value.compressorReleaseMillis),
            compressorMakeupCentibels: Number(value.compressorMakeupCentibels),
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
            limiterReleaseMillis: Number(value.limiterReleaseMillis)
        }
    }

    function sameSnapshot(left: var, right: var) : bool {
        if (!left || !right) return false
        return Number(left.trimStartMillis) === Number(right.trimStartMillis)
            && Number(left.trimEndMillis) === Number(right.trimEndMillis)
            && Number(left.fadeInMillis) === Number(right.fadeInMillis)
            && Number(left.fadeOutMillis) === Number(right.fadeOutMillis)
            && Number(left.fadeInCurve) === Number(right.fadeInCurve)
            && Number(left.fadeOutCurve) === Number(right.fadeOutCurve)
            && Number(left.gainCentibels) === Number(right.gainCentibels)
            && Number(left.lowCutHertz) === Number(right.lowCutHertz)
            && Boolean(left.noiseReductionEnabled) === Boolean(right.noiseReductionEnabled)
            && Number(left.noiseReductionCentibels) === Number(right.noiseReductionCentibels)
            && Number(left.noiseReductionSensitivityPercent) === Number(right.noiseReductionSensitivityPercent)
            && Number(left.noiseReductionSmoothingMillis) === Number(right.noiseReductionSmoothingMillis)
            && Boolean(left.deEsserEnabled) === Boolean(right.deEsserEnabled)
            && Number(left.deEsserFrequencyHertz) === Number(right.deEsserFrequencyHertz)
            && Number(left.deEsserThresholdCentibels) === Number(right.deEsserThresholdCentibels)
            && Number(left.deEsserReductionCentibels) === Number(right.deEsserReductionCentibels)
            && sameEqualizer(left.equalizerBands, right.equalizerBands)
            && Boolean(left.compressorEnabled)
                === Boolean(right.compressorEnabled)
            && Number(left.compressorThresholdCentibels)
                === Number(right.compressorThresholdCentibels)
            && Number(left.compressorRatioTenths)
                === Number(right.compressorRatioTenths)
            && Number(left.compressorAttackMillis)
                === Number(right.compressorAttackMillis)
            && Number(left.compressorReleaseMillis)
                === Number(right.compressorReleaseMillis)
            && Number(left.compressorMakeupCentibels)
                === Number(right.compressorMakeupCentibels)
            && Boolean(left.reverbEnabled) === Boolean(right.reverbEnabled)
            && Number(left.reverbMixPercent) === Number(right.reverbMixPercent)
            && Number(left.reverbPreDelayMillis) === Number(right.reverbPreDelayMillis)
            && Number(left.reverbDecayMillis) === Number(right.reverbDecayMillis)
            && Number(left.reverbSizePercent) === Number(right.reverbSizePercent)
            && Number(left.reverbDampingPercent) === Number(right.reverbDampingPercent)
            && Number(left.reverbLowCutHertz) === Number(right.reverbLowCutHertz)
            && Number(left.reverbHighCutHertz) === Number(right.reverbHighCutHertz)
            && Boolean(left.limiterEnabled) === Boolean(right.limiterEnabled)
            && Number(left.limiterCeilingCentibels)
                === Number(right.limiterCeilingCentibels)
            && Number(left.limiterReleaseMillis)
                === Number(right.limiterReleaseMillis)
    }

    function assetSnapshot() : var {
        if (!asset) {
            return {
                trimStartMillis: 0, trimEndMillis: 0,
                fadeInMillis: 0, fadeOutMillis: 0,
                fadeInCurve: 0, fadeOutCurve: 0,
                gainCentibels: 0, lowCutHertz: 0,
                noiseReductionEnabled: false,
                noiseReductionCentibels: 900,
                noiseReductionSensitivityPercent: 50,
                noiseReductionSmoothingMillis: 240,
                deEsserEnabled: false,
                deEsserFrequencyHertz: 6500,
                deEsserThresholdCentibels: -2400,
                deEsserReductionCentibels: 600,
                equalizerBands: defaultEqualizerBands(),
                compressorEnabled: false,
                compressorThresholdCentibels: -1800,
                compressorRatioTenths: 30,
                compressorAttackMillis: 10,
                compressorReleaseMillis: 120,
                compressorMakeupCentibels: 0,
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
                limiterReleaseMillis: 100
            }
        }
        const duration = Math.max(0, Number(asset.durationMillis))
        return {
            trimStartMillis: Math.max(0, Number(asset.trimStartMillis)),
            trimEndMillis: Number(asset.trimEndMillis) > 0
                ? Number(asset.trimEndMillis) : duration,
            fadeInMillis: Math.max(0, Number(asset.fadeInMillis)),
            fadeOutMillis: Math.max(0, Number(asset.fadeOutMillis)),
            fadeInCurve: clamp(Number(asset.fadeInCurve || 0), 0, 2),
            fadeOutCurve: clamp(Number(asset.fadeOutCurve || 0), 0, 2),
            gainCentibels: clamp(Number(asset.gainCentibels), -2400, 1200),
            lowCutHertz: Number(asset.lowCutHertz) === 0 ? 0
                : clamp(Number(asset.lowCutHertz), 20, 240),
            noiseReductionEnabled: Boolean(asset.noiseReductionEnabled),
            noiseReductionCentibels: clamp(Number(asset.noiseReductionCentibels ?? 900), 0, 2400),
            noiseReductionSensitivityPercent: clamp(Number(asset.noiseReductionSensitivityPercent ?? 50), 0, 100),
            noiseReductionSmoothingMillis: clamp(Number(asset.noiseReductionSmoothingMillis ?? 240), 20, 1000),
            deEsserEnabled: Boolean(asset.deEsserEnabled),
            deEsserFrequencyHertz: clamp(Number(asset.deEsserFrequencyHertz ?? 6500), 3000, 12000),
            deEsserThresholdCentibels: clamp(Number(asset.deEsserThresholdCentibels ?? -2400), -6000, 0),
            deEsserReductionCentibels: clamp(Number(asset.deEsserReductionCentibels ?? 600), 0, 1800),
            equalizerBands: copyEqualizerBands(asset.equalizerBands),
            compressorEnabled: Boolean(asset.compressorEnabled),
            compressorThresholdCentibels: clamp(
                Number(asset.compressorThresholdCentibels ?? -1800), -6000, 0),
            compressorRatioTenths: clamp(
                Number(asset.compressorRatioTenths ?? 30), 10, 200),
            compressorAttackMillis: clamp(
                Number(asset.compressorAttackMillis ?? 10), 1, 200),
            compressorReleaseMillis: clamp(
                Number(asset.compressorReleaseMillis ?? 120), 20, 2000),
            compressorMakeupCentibels: clamp(
                Number(asset.compressorMakeupCentibels ?? 0), 0, 2400),
            reverbEnabled: Boolean(asset.reverbEnabled),
            reverbMixPercent: clamp(Number(asset.reverbMixPercent ?? 18), 0, 100),
            reverbPreDelayMillis: clamp(
                Number(asset.reverbPreDelayMillis ?? 20), 0, 200),
            reverbDecayMillis: clamp(
                Number(asset.reverbDecayMillis ?? 1800), 100, 12000),
            reverbSizePercent: clamp(
                Number(asset.reverbSizePercent ?? 55), 10, 100),
            reverbDampingPercent: clamp(
                Number(asset.reverbDampingPercent ?? 45), 0, 100),
            reverbLowCutHertz: clamp(
                Number(asset.reverbLowCutHertz ?? 120), 20, 1000),
            reverbHighCutHertz: clamp(
                Number(asset.reverbHighCutHertz ?? 10000), 1000, 20000),
            limiterEnabled: Boolean(asset.limiterEnabled),
            limiterCeilingCentibels: clamp(
                Number(asset.limiterCeilingCentibels ?? -100), -600, 0),
            limiterReleaseMillis: clamp(
                Number(asset.limiterReleaseMillis ?? 100), 20, 1000)
        }
    }

    function applySnapshot(value: var) : void {
        _restoring = true
        trimStartMillis = Number(value.trimStartMillis)
        trimEndMillis = Number(value.trimEndMillis)
        fadeInMillis = Number(value.fadeInMillis)
        fadeOutMillis = Number(value.fadeOutMillis)
        fadeInCurve = Number(value.fadeInCurve)
        fadeOutCurve = Number(value.fadeOutCurve)
        gainCentibels = Number(value.gainCentibels)
        lowCutHertz = Number(value.lowCutHertz)
        noiseReductionEnabled = Boolean(value.noiseReductionEnabled)
        noiseReductionCentibels = Number(value.noiseReductionCentibels)
        noiseReductionSensitivityPercent = Number(value.noiseReductionSensitivityPercent)
        noiseReductionSmoothingMillis = Number(value.noiseReductionSmoothingMillis)
        deEsserEnabled = Boolean(value.deEsserEnabled)
        deEsserFrequencyHertz = Number(value.deEsserFrequencyHertz)
        deEsserThresholdCentibels = Number(value.deEsserThresholdCentibels)
        deEsserReductionCentibels = Number(value.deEsserReductionCentibels)
        equalizerBands = copyEqualizerBands(value.equalizerBands)
        compressorEnabled = Boolean(value.compressorEnabled)
        compressorThresholdCentibels = Number(value.compressorThresholdCentibels)
        compressorRatioTenths = Number(value.compressorRatioTenths)
        compressorAttackMillis = Number(value.compressorAttackMillis)
        compressorReleaseMillis = Number(value.compressorReleaseMillis)
        compressorMakeupCentibels = Number(value.compressorMakeupCentibels)
        reverbEnabled = Boolean(value.reverbEnabled)
        reverbMixPercent = Number(value.reverbMixPercent)
        reverbPreDelayMillis = Number(value.reverbPreDelayMillis)
        reverbDecayMillis = Number(value.reverbDecayMillis)
        reverbSizePercent = Number(value.reverbSizePercent)
        reverbDampingPercent = Number(value.reverbDampingPercent)
        reverbLowCutHertz = Number(value.reverbLowCutHertz)
        reverbHighCutHertz = Number(value.reverbHighCutHertz)
        limiterEnabled = Boolean(value.limiterEnabled)
        limiterCeilingCentibels = Number(value.limiterCeilingCentibels)
        limiterReleaseMillis = Number(value.limiterReleaseMillis)
        _restoring = false
    }

    function resetFromAsset() : void {
        const persisted = assetSnapshot()
        applySnapshot(persisted)
        _savedSnapshot = copySnapshot(persisted)
        _history = [copySnapshot(persisted)]
        _historyIndex = 0
        _gestureStart = null
    }

    function pushCurrent() : void {
        if (_restoring || _gestureStart !== null) return
        const current = snapshot()
        if (_historyIndex >= 0
                && sameSnapshot(current, _history[_historyIndex])) return
        const next = _history.slice(0, _historyIndex + 1)
        next.push(copySnapshot(current))
        _history = next
        _historyIndex = next.length - 1
    }

    function beginGesture() : void {
        if (_gestureStart === null) {
            _gestureStart = copySnapshot(snapshot())
        }
    }

    function endGesture() : void {
        if (_gestureStart === null) return
        const start = _gestureStart
        _gestureStart = null
        if (!sameSnapshot(start, snapshot())) pushCurrent()
    }

    function cancelGesture() : void {
        if (_gestureStart === null) return
        const start = _gestureStart
        _gestureStart = null
        applySnapshot(start)
    }

    function undo() : void {
        if (!canUndo) return
        _historyIndex -= 1
        applySnapshot(_history[_historyIndex])
    }

    function redo() : void {
        if (!canRedo) return
        _historyIndex += 1
        applySnapshot(_history[_historyIndex])
    }

    function setTrimRange(startMillis: int, endMillis: int) : void {
        const minimumDuration = Math.min(50, sourceDurationMillis)
        const start = Math.round(clamp(startMillis, 0,
            Math.max(0, sourceDurationMillis - minimumDuration)))
        const end = Math.round(clamp(endMillis, start + minimumDuration,
            sourceDurationMillis))
        trimStartMillis = start
        trimEndMillis = end
        fadeInMillis = Math.min(fadeInMillis, selectedDurationMillis)
        fadeOutMillis = Math.min(fadeOutMillis,
            Math.max(0, selectedDurationMillis - fadeInMillis))
        pushCurrent()
    }

    function setFades(fadeIn: int, fadeOut: int) : void {
        fadeInMillis = Math.round(clamp(fadeIn, 0, selectedDurationMillis))
        fadeOutMillis = Math.round(clamp(fadeOut, 0,
            Math.max(0, selectedDurationMillis - fadeInMillis)))
        pushCurrent()
    }

    function setFadeCurves(fadeIn: int, fadeOut: int) : void {
        fadeInCurve = Math.round(clamp(fadeIn, 0, 2))
        fadeOutCurve = Math.round(clamp(fadeOut, 0, 2))
        pushCurrent()
    }

    function setGain(centibels: int) : void {
        gainCentibels = Math.round(clamp(centibels, -2400, 1200))
        pushCurrent()
    }

    function setLowCut(hertz: int) : void {
        lowCutHertz = hertz === 0 ? 0 : Math.round(clamp(hertz, 20, 240))
        pushCurrent()
    }

    function restorationValue() : var {
        return {
            noiseEnabled: noiseReductionEnabled,
            noiseReductionCentibels: noiseReductionCentibels,
            noiseSensitivityPercent: noiseReductionSensitivityPercent,
            noiseSmoothingMillis: noiseReductionSmoothingMillis,
            deEsserEnabled: deEsserEnabled,
            deEsserFrequencyHertz: deEsserFrequencyHertz,
            deEsserThresholdCentibels: deEsserThresholdCentibels,
            deEsserReductionCentibels: deEsserReductionCentibels
        }
    }

    function setRestorationParameter(parameter: string, value: int) : void {
        if (parameter === "noiseReduction") {
            noiseReductionCentibels = Math.round(clamp(value, 0, 2400))
        } else if (parameter === "noiseSensitivity") {
            noiseReductionSensitivityPercent = Math.round(clamp(value, 0, 100))
        } else if (parameter === "noiseSmoothing") {
            noiseReductionSmoothingMillis = Math.round(clamp(value, 20, 1000))
        } else if (parameter === "deEsserFrequency") {
            deEsserFrequencyHertz = Math.round(clamp(value, 3000, 12000))
        } else if (parameter === "deEsserThreshold") {
            deEsserThresholdCentibels = Math.round(clamp(value, -6000, 0))
        } else if (parameter === "deEsserReduction") {
            deEsserReductionCentibels = Math.round(clamp(value, 0, 1800))
        } else {
            return
        }
        pushCurrent()
    }

    function resetRestoration() : void {
        noiseReductionEnabled = false
        noiseReductionCentibels = 900
        noiseReductionSensitivityPercent = 50
        noiseReductionSmoothingMillis = 240
        deEsserEnabled = false
        deEsserFrequencyHertz = 6500
        deEsserThresholdCentibels = -2400
        deEsserReductionCentibels = 600
        pushCurrent()
    }

    function setEqualizerBand(index: int, enabled: bool, filterKind: int,
                              frequencyHertz: int, qHundredths: int,
                              gainCentibels: int) : void {
        if (index < 0 || index >= equalizerBands.length) return
        const next = copyEqualizerBands(equalizerBands)
        next[index] = {
            enabled: enabled,
            filterKind: Math.round(clamp(filterKind, 0, 3)),
            frequencyHertz: Math.round(clamp(frequencyHertz, 20, 20000)),
            qHundredths: Math.round(clamp(qHundredths, 10, 2000)),
            gainCentibels: Math.round(clamp(gainCentibels, -1200, 1200))
        }
        equalizerBands = next
        pushCurrent()
    }

    function resetEqualizer() : void {
        equalizerBands = defaultEqualizerBands()
        pushCurrent()
    }

    function setCompressorEnabled(enabled: bool) : void {
        compressorEnabled = enabled
        pushCurrent()
    }

    function setCompressorParameter(parameter: string, value: int) : void {
        if (parameter === "threshold") {
            compressorThresholdCentibels = Math.round(clamp(value, -6000, 0))
        } else if (parameter === "ratio") {
            compressorRatioTenths = Math.round(clamp(value, 10, 200))
        } else if (parameter === "attack") {
            compressorAttackMillis = Math.round(clamp(value, 1, 200))
        } else if (parameter === "release") {
            compressorReleaseMillis = Math.round(clamp(value, 20, 2000))
        } else if (parameter === "makeup") {
            compressorMakeupCentibels = Math.round(clamp(value, 0, 2400))
        } else {
            return
        }
        pushCurrent()
    }

    function resetCompressor() : void {
        compressorEnabled = false
        compressorThresholdCentibels = -1800
        compressorRatioTenths = 30
        compressorAttackMillis = 10
        compressorReleaseMillis = 120
        compressorMakeupCentibels = 0
        pushCurrent()
    }

    function reverbValue() : var {
        return {
            enabled: reverbEnabled,
            mixPercent: reverbMixPercent,
            preDelayMillis: reverbPreDelayMillis,
            decayMillis: reverbDecayMillis,
            sizePercent: reverbSizePercent,
            dampingPercent: reverbDampingPercent,
            lowCutHertz: reverbLowCutHertz,
            highCutHertz: reverbHighCutHertz
        }
    }

    function setReverbEnabled(enabled: bool) : void {
        reverbEnabled = enabled
        pushCurrent()
    }

    function setReverbParameter(parameter: string, value: int) : void {
        if (parameter === "mix") {
            reverbMixPercent = Math.round(clamp(value, 0, 100))
        } else if (parameter === "preDelay") {
            reverbPreDelayMillis = Math.round(clamp(value, 0, 200))
        } else if (parameter === "decay") {
            reverbDecayMillis = Math.round(clamp(value, 100, 12000))
        } else if (parameter === "size") {
            reverbSizePercent = Math.round(clamp(value, 10, 100))
        } else if (parameter === "damping") {
            reverbDampingPercent = Math.round(clamp(value, 0, 100))
        } else if (parameter === "lowCut") {
            reverbLowCutHertz = Math.round(clamp(
                value, 20, Math.min(1000, reverbHighCutHertz - 1)))
        } else if (parameter === "highCut") {
            reverbHighCutHertz = Math.round(clamp(
                value, Math.max(1000, reverbLowCutHertz + 1), 20000))
        } else {
            return
        }
        pushCurrent()
    }

    function resetReverb() : void {
        reverbEnabled = false
        reverbMixPercent = 18
        reverbPreDelayMillis = 20
        reverbDecayMillis = 1800
        reverbSizePercent = 55
        reverbDampingPercent = 45
        reverbLowCutHertz = 120
        reverbHighCutHertz = 10000
        pushCurrent()
    }

    function setLimiterEnabled(enabled: bool) : void {
        limiterEnabled = enabled
        pushCurrent()
    }

    function setLimiterParameter(parameter: string, value: int) : void {
        if (parameter === "ceiling") {
            limiterCeilingCentibels = Math.round(clamp(value, -600, 0))
        } else if (parameter === "release") {
            limiterReleaseMillis = Math.round(clamp(value, 20, 1000))
        } else {
            return
        }
        pushCurrent()
    }

    function resetLimiter() : void {
        limiterEnabled = false
        limiterCeilingCentibels = -100
        limiterReleaseMillis = 100
        pushCurrent()
    }

    function clear() : void {
        applySnapshot({
            trimStartMillis: 0,
            trimEndMillis: sourceDurationMillis,
            fadeInMillis: 0,
            fadeOutMillis: 0,
            fadeInCurve: 0,
            fadeOutCurve: 0,
            gainCentibels: 0,
            lowCutHertz: 0,
            noiseReductionEnabled: false,
            noiseReductionCentibels: 900,
            noiseReductionSensitivityPercent: 50,
            noiseReductionSmoothingMillis: 240,
            deEsserEnabled: false,
            deEsserFrequencyHertz: 6500,
            deEsserThresholdCentibels: -2400,
            deEsserReductionCentibels: 600,
            equalizerBands: defaultEqualizerBands(),
            compressorEnabled: false,
            compressorThresholdCentibels: -1800,
            compressorRatioTenths: 30,
            compressorAttackMillis: 10,
            compressorReleaseMillis: 120,
            compressorMakeupCentibels: 0,
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
            limiterReleaseMillis: 100
        })
        pushCurrent()
    }

    function revert() : void {
        applySnapshot(_savedSnapshot)
        pushCurrent()
    }

    function save() : void {
        if (!asset || !dirty) return
        saveRequested(trimStartMillis, trimEndMillis,
            fadeInMillis, fadeOutMillis, fadeInCurve, fadeOutCurve,
            gainCentibels, lowCutHertz,
            noiseReductionEnabled, noiseReductionCentibels,
            noiseReductionSensitivityPercent, noiseReductionSmoothingMillis,
            deEsserEnabled, deEsserFrequencyHertz,
            deEsserThresholdCentibels, deEsserReductionCentibels,
            copyEqualizerBands(equalizerBands),
            compressorEnabled,
            compressorThresholdCentibels, compressorRatioTenths,
            compressorAttackMillis, compressorReleaseMillis,
            compressorMakeupCentibels,
            reverbEnabled, reverbMixPercent, reverbPreDelayMillis,
            reverbDecayMillis, reverbSizePercent, reverbDampingPercent,
            reverbLowCutHertz, reverbHighCutHertz, limiterEnabled,
            limiterCeilingCentibels, limiterReleaseMillis)
    }

    function markSaved() : void {
        _savedSnapshot = copySnapshot(snapshot())
        _history = [copySnapshot(snapshot())]
        _historyIndex = 0
        _gestureStart = null
    }

    onAssetChanged: resetFromAsset()
    Component.onCompleted: resetFromAsset()
}

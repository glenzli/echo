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
    property int eqLowGainCentibels: 0
    property int eqMidGainCentibels: 0
    property int eqHighGainCentibels: 0
    property bool compressorEnabled: false
    property int compressorThresholdCentibels: -1800
    property int compressorRatioTenths: 30
    property int compressorAttackMillis: 10
    property int compressorReleaseMillis: 120
    property int compressorMakeupCentibels: 0

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
        && eqLowGainCentibels === 0 && eqMidGainCentibels === 0
        && eqHighGainCentibels === 0
        && !compressorEnabled
    readonly property bool dirty: !sameSnapshot(snapshot(), _savedSnapshot)

    signal saveRequested(int startMillis, int endMillis,
                         int fadeIn, int fadeOut,
                         int fadeInCurve, int fadeOutCurve,
                         int gain, int lowCut,
                         int eqLowGain, int eqMidGain, int eqHighGain,
                         bool compressorEnabled, int compressorThreshold,
                         int compressorRatio, int compressorAttack,
                         int compressorRelease, int compressorMakeup)

    function clamp(value: real, minimum: real, maximum: real) : real {
        return Math.max(minimum, Math.min(maximum, value))
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
            eqLowGainCentibels: eqLowGainCentibels,
            eqMidGainCentibels: eqMidGainCentibels,
            eqHighGainCentibels: eqHighGainCentibels,
            compressorEnabled: compressorEnabled,
            compressorThresholdCentibels: compressorThresholdCentibels,
            compressorRatioTenths: compressorRatioTenths,
            compressorAttackMillis: compressorAttackMillis,
            compressorReleaseMillis: compressorReleaseMillis,
            compressorMakeupCentibels: compressorMakeupCentibels
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
            eqLowGainCentibels: Number(value.eqLowGainCentibels),
            eqMidGainCentibels: Number(value.eqMidGainCentibels),
            eqHighGainCentibels: Number(value.eqHighGainCentibels),
            compressorEnabled: Boolean(value.compressorEnabled),
            compressorThresholdCentibels: Number(value.compressorThresholdCentibels),
            compressorRatioTenths: Number(value.compressorRatioTenths),
            compressorAttackMillis: Number(value.compressorAttackMillis),
            compressorReleaseMillis: Number(value.compressorReleaseMillis),
            compressorMakeupCentibels: Number(value.compressorMakeupCentibels)
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
            && Number(left.eqLowGainCentibels)
                === Number(right.eqLowGainCentibels)
            && Number(left.eqMidGainCentibels)
                === Number(right.eqMidGainCentibels)
            && Number(left.eqHighGainCentibels)
                === Number(right.eqHighGainCentibels)
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
    }

    function assetSnapshot() : var {
        if (!asset) {
            return {
                trimStartMillis: 0, trimEndMillis: 0,
                fadeInMillis: 0, fadeOutMillis: 0,
                fadeInCurve: 0, fadeOutCurve: 0,
                gainCentibels: 0, lowCutHertz: 0,
                eqLowGainCentibels: 0, eqMidGainCentibels: 0,
                eqHighGainCentibels: 0,
                compressorEnabled: false,
                compressorThresholdCentibels: -1800,
                compressorRatioTenths: 30,
                compressorAttackMillis: 10,
                compressorReleaseMillis: 120,
                compressorMakeupCentibels: 0
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
            eqLowGainCentibels: clamp(
                Number(asset.eqLowGainCentibels || 0), -1200, 1200),
            eqMidGainCentibels: clamp(
                Number(asset.eqMidGainCentibels || 0), -1200, 1200),
            eqHighGainCentibels: clamp(
                Number(asset.eqHighGainCentibels || 0), -1200, 1200),
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
                Number(asset.compressorMakeupCentibels ?? 0), 0, 2400)
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
        eqLowGainCentibels = Number(value.eqLowGainCentibels)
        eqMidGainCentibels = Number(value.eqMidGainCentibels)
        eqHighGainCentibels = Number(value.eqHighGainCentibels)
        compressorEnabled = Boolean(value.compressorEnabled)
        compressorThresholdCentibels = Number(value.compressorThresholdCentibels)
        compressorRatioTenths = Number(value.compressorRatioTenths)
        compressorAttackMillis = Number(value.compressorAttackMillis)
        compressorReleaseMillis = Number(value.compressorReleaseMillis)
        compressorMakeupCentibels = Number(value.compressorMakeupCentibels)
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

    function setEqualizerBand(band: string, centibels: int) : void {
        const gain = Math.round(clamp(centibels, -1200, 1200))
        if (band === "low") eqLowGainCentibels = gain
        else if (band === "mid") eqMidGainCentibels = gain
        else if (band === "high") eqHighGainCentibels = gain
        else return
        pushCurrent()
    }

    function resetEqualizer() : void {
        eqLowGainCentibels = 0
        eqMidGainCentibels = 0
        eqHighGainCentibels = 0
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
            eqLowGainCentibels: 0,
            eqMidGainCentibels: 0,
            eqHighGainCentibels: 0,
            compressorEnabled: false,
            compressorThresholdCentibels: -1800,
            compressorRatioTenths: 30,
            compressorAttackMillis: 10,
            compressorReleaseMillis: 120,
            compressorMakeupCentibels: 0
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
            eqLowGainCentibels, eqMidGainCentibels,
            eqHighGainCentibels, compressorEnabled,
            compressorThresholdCentibels, compressorRatioTenths,
            compressorAttackMillis, compressorReleaseMillis,
            compressorMakeupCentibels)
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

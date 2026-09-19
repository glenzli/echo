// Canonical saved evidence. Diagnostic listening is deliberately not copied here.
function copy(value, duration) {
    if (!value || value.algorithmVersion !== 1 || !value.powerCentibels
            || typeof value.powerCentibels !== 'object' || value.powerCentibels.length !== 1025) return null;
    // QVariantList is a QML sequence, not a JavaScript Array. Normalize the
    // native result before validating or keeping it in authored history.
    const powers=Array.from(value.powerCentibels);
    const start=Number(value.captureStartMillis), end=Number(value.captureEndMillis);
    if (!Number.isInteger(start) || !Number.isInteger(end) || start<0 || end>duration
            || end-start<100 || end-start>30000) return null;
    if (powers.some(power => !Number.isInteger(power) || power < -14400 || power > 1200)) return null;
    const reduction=Number(value.reductionCentibels), sensitivity=Number(value.sensitivityCentibels), smoothing=Number(value.smoothingBins);
    if (!Number.isInteger(reduction) || reduction<0 || reduction>3600 || !Number.isInteger(sensitivity)
            || sensitivity<0 || sensitivity>1200 || !Number.isInteger(smoothing) || smoothing<0 || smoothing>8) return null;
    return {algorithmVersion:1, enabled:Boolean(value.enabled), captureStartMillis:start, captureEndMillis:end,
        powerCentibels:powers, reductionCentibels:reduction, sensitivityCentibels:sensitivity, smoothingBins:smoothing};
}

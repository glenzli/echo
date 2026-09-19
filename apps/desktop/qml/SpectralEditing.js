// Source-time / Hertz geometry shared by gestures and numeric editing.
function clamp(value, low, high) { return Math.max(low, Math.min(high, value)); }
function hertzAt(ratio, low, high, logarithmic) {
    const position = clamp(ratio, 0, 1);
    return logarithmic ? low * Math.pow(high / low, position) : low + (high - low) * position;
}
function frequencyRatio(hertz, low, high, logarithmic) {
    return logarithmic ? Math.log(Math.max(1, hertz) / low) / Math.log(high / low) : (hertz - low) / (high - low);
}
function region(value, duration) {
    if (!value || !Number.isFinite(duration) || duration <= 0) return null;
    const fields = ['startMillis', 'endMillis', 'lowHertz', 'highHertz', 'attenuationCentibels', 'timeFeatherMillis', 'frequencyFeatherHertz'];
    if (fields.some(key => !Number.isFinite(Number(value[key])))) return null;
    const start = clamp(Math.round(value.startMillis), 0, duration - 1);
    const end = clamp(Math.round(value.endMillis), start + 1, duration);
    const low = clamp(Math.round(value.lowHertz), 20, 23999);
    const high = clamp(Math.round(value.highHertz), low + 1, 24000);
    return {startMillis:start, endMillis:end, lowHertz:low, highHertz:high,
        attenuationCentibels:clamp(Math.round(value.attenuationCentibels), 0, 9600),
        timeFeatherMillis:clamp(Math.round(value.timeFeatherMillis), 0, 250),
        frequencyFeatherHertz:clamp(Math.round(value.frequencyFeatherHertz), 0, 2000)};
}
function selection(start, end, low, high, duration) {
    const width = Math.abs(high - low), length = Math.abs(end - start);
    return region({startMillis:Math.min(start,end), endMillis:Math.max(start,end),
        lowHertz:Math.min(low,high), highHertz:Math.max(low,high), attenuationCentibels:2400,
        timeFeatherMillis:Math.min(24, Math.floor(length/4)), frequencyFeatherHertz:Math.min(80, Math.floor(width/4))}, duration);
}
function move(value, timeDelta, hertzDelta, duration) {
    const next = Object.assign({}, value);
    const dt = clamp(Math.round(timeDelta), -value.startMillis, duration-value.endMillis);
    const df = clamp(Math.round(hertzDelta), 20-value.lowHertz, 24000-value.highHertz);
    next.startMillis += dt; next.endMillis += dt;
    next.lowHertz += df; next.highHertz += df;
    return region(next, duration);
}
function resize(value, edge, time, hertz, duration) {
    const next = Object.assign({}, value);
    if (edge.indexOf('left') >= 0) next.startMillis = clamp(time, 0, next.endMillis-1);
    if (edge.indexOf('right') >= 0) next.endMillis = clamp(time, next.startMillis+1, duration);
    if (edge.indexOf('top') >= 0) next.highHertz = clamp(hertz, next.lowHertz+1, 24000);
    if (edge.indexOf('bottom') >= 0) next.lowHertz = clamp(hertz, 20, next.highHertz-1);
    return region(next, duration);
}
function harmonics(value, count, duration) {
    const result = [], fundamental = (value.lowHertz + value.highHertz)/2;
    const halfWidth = (value.highHertz-value.lowHertz)/2;
    // A wide selection would overlap adjacent harmonics and double attenuation.
    if (fundamental <= 2*halfWidth) return result;
    for (let harmonic=2; harmonic<=clamp(Math.round(count),2,8); ++harmonic) {
        const low = fundamental*harmonic-halfWidth, high = fundamental*harmonic+halfWidth;
        if (high>24000) break;
        result.push(region(Object.assign({},value,{lowHertz:low,highHertz:high}),duration));
    }
    return result;
}

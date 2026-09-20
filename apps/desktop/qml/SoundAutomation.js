.pragma library

// Source-time dB automation shared by the editor and deterministic ducking.
function gainAt(points, time) {
    if (!points.length) return 0;
    let low = 0, high = points.length;
    while (low < high) { const mid = (low + high) >>> 1; if (points[mid].sourceMillis <= time) low = mid + 1; else high = mid; }
    if (!low) return points[0].gainCentibels;
    if (low === points.length) return points[low - 1].gainCentibels;
    const a = points[low - 1], b = points[low];
    return a.gainCentibels + (b.gainCentibels - a.gainCentibels) * (time - a.sourceMillis) / (b.sourceMillis - a.sourceMillis);
}
function setPoint(points, time, gain) {
    const result = points.filter(point => point.sourceMillis !== Math.round(time));
    result.push({sourceMillis: Math.round(time), gainCentibels: Math.round(Math.max(-9600, Math.min(1200, gain)))});
    result.sort((a, b) => a.sourceMillis - b.sourceMillis);
    return result.length <= 2048 ? result : points;
}
// Peaks describe the original recording, not post-effect loudness or speech probability.
function activity(clip, spans, levels, originalDuration, thresholdDb) {
    if (!levels.length || originalDuration <= 0) throw new Error("waveform");
    const threshold = Math.pow(10, thresholdDb / 20), result = [];
    const bucket = originalDuration / levels.length;
    for (const span of spans) {
        if (span.silent) continue;
        const start = Math.max(span.start, clip.sourceStartMillis), end = Math.min(span.end, clip.sourceEndMillis);
        if (start >= end) continue;
        const originalStart = span.sourceStart + start - span.start;
        const originalEnd = span.sourceStart + end - span.start;
        let run = null;
        for (let i = Math.max(0, Math.floor(originalStart / bucket)); i < Math.min(levels.length, Math.ceil(originalEnd / bucket)); ++i) {
            const a = Math.max(originalStart, i * bucket), b = Math.min(originalEnd, (i + 1) * bucket);
            const timelineA = clip.timelineStartMillis + span.start + a - span.sourceStart - clip.sourceStartMillis;
            const timelineB = clip.timelineStartMillis + span.start + b - span.sourceStart - clip.sourceStartMillis;
            if (Number(levels[i]) >= threshold) {
                if (run === null) run = [timelineA, timelineB]; else run[1] = timelineB;
            } else if (run !== null) { result.push(run); run = null; }
        }
        if (run !== null) result.push(run);
    }
    return result;
}
function duckRegions(intervals, attack, hold, release) {
    const sorted = intervals.slice().sort((a, b) => a[0] - b[0]), result = [];
    for (const value of sorted) {
        const region = [Math.max(0, value[0] - attack), Math.max(0, value[0]), value[1] + hold, value[1] + hold + release];
        const previous = result[result.length - 1];
        if (previous && previous[3] >= region[0]) { previous[2] = Math.max(previous[2], region[2]); previous[3] = Math.max(previous[3], region[3]); }
        else result.push(region);
    }
    return result;
}
function duckEnvelope(clip, regions, amount) {
    const keys = [];
    for (const r of regions) {
        keys.push({sourceMillis: Math.round(r[0]), gainCentibels: 0}, {sourceMillis: Math.round(r[1]), gainCentibels: -amount},
                  {sourceMillis: Math.round(r[2]), gainCentibels: -amount}, {sourceMillis: Math.round(r[3]), gainCentibels: 0});
    }
    // Coincident boundaries retain the last value (instant attacks at timeline zero).
    const timeline = [];
    for (const p of keys) { if (timeline.length && timeline[timeline.length - 1].sourceMillis === p.sourceMillis) timeline.pop(); timeline.push(p); }
    const start = clip.timelineStartMillis, end = start + clip.sourceEndMillis - clip.sourceStartMillis;
    const points = [{sourceMillis: clip.sourceStartMillis, gainCentibels: Math.round(gainAt(timeline, start))}];
    for (const p of timeline) if (p.sourceMillis > start && p.sourceMillis < end) points.push({sourceMillis: p.sourceMillis - start + clip.sourceStartMillis, gainCentibels: p.gainCentibels});
    points.push({sourceMillis: clip.sourceEndMillis, gainCentibels: Math.round(gainAt(timeline, end))});
    if (points.length > 2048) throw new Error("complexity");
    return {enabled: true, points: points};
}

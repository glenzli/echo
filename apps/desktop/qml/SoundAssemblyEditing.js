.pragma library

// Arrangement-time geometry. All operations preserve immutable source bounds;
// callers publish one patch at gesture end and own undo/document persistence.
function clamp(value, low, high) { return Math.max(low, Math.min(high, value)); }
function duration(clip) { return clip.sourceEndMillis - clip.sourceStartMillis; }
function end(clip) { return clip.timelineStartMillis + duration(clip); }
function gridSeconds(pixelsPerSecond) {
    const steps = [0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 30, 60, 120, 300, 600, 1800, 3600];
    for (const step of steps) if (step * pixelsPerSecond >= 70) return step;
    return 3600;
}
function snap(position, length, targets, gridMillis, toleranceMillis, enabled, maximum) {
    const limit = maximum === undefined ? 14400000 - length : maximum;
    let start = clamp(position, 0, limit), guide = -1, distance = toleranceMillis + 1;
    if (!enabled) return {position: Math.round(start), guide: guide};
    // Clip/playhead boundaries win exact ties over the grid.
    const points = targets.slice();
    points.push(Math.round(start / gridMillis) * gridMillis);
    if (length > 0) points.push(Math.round((start + length) / gridMillis) * gridMillis);
    for (const target of points) {
        for (const offset of length > 0 ? [0, length] : [0]) {
            const candidate = target - offset;
            const delta = Math.abs(candidate - start);
            if (candidate >= 0 && candidate <= limit && delta <= toleranceMillis && delta < distance) {
                distance = delta; guide = target; position = candidate;
            }
        }
    }
    return {position: Math.round(guide < 0 ? start : position), guide: guide};
}
function fitFades(clip) {
    const length = duration(clip);
    clip.fadeInMillis = clamp(clip.fadeInMillis, 0, length);
    clip.fadeOutMillis = clamp(clip.fadeOutMillis, 0, length - clip.fadeInMillis);
    return clip;
}
function trim(clip, edge, deltaMillis, sourceDuration) {
    const next = Object.assign({}, clip);
    if (edge === "left") {
        const delta = Math.round(clamp(deltaMillis, Math.max(-clip.sourceStartMillis, -clip.timelineStartMillis), duration(clip) - 10));
        next.sourceStartMillis += delta;
        next.timelineStartMillis += delta;
    } else {
        next.sourceEndMillis = Math.round(clamp(clip.sourceEndMillis + deltaMillis, clip.sourceStartMillis + 10,
                                                Math.min(sourceDuration, clip.sourceStartMillis + 14400000 - clip.timelineStartMillis)));
    }
    return fitFades(next);
}
function slip(clip, deltaMillis, sourceDuration) {
    const delta = Math.round(clamp(deltaMillis, -clip.sourceStartMillis, sourceDuration - clip.sourceEndMillis));
    return Object.assign({}, clip, {sourceStartMillis: clip.sourceStartMillis + delta, sourceEndMillis: clip.sourceEndMillis + delta});
}
function split(clip, position) {
    const offset = Math.round(position - clip.timelineStartMillis);
    if (offset < 10 || offset > duration(clip) - 10) return null;
    const first = Object.assign({}, clip), second = Object.assign({}, clip);
    first.sourceEndMillis = clip.sourceStartMillis + offset;
    second.sourceStartMillis = first.sourceEndMillis;
    second.timelineStartMillis = clip.timelineStartMillis + offset;
    // Keep only the original outer fades; a cut does not add a dip in the middle.
    first.fadeOutMillis = 0;
    second.fadeInMillis = 0;
    return [fitFades(first), fitFades(second)];
}
function crossfade(a, b) {
    const first = a.timelineStartMillis <= b.timelineStartMillis ? a : b;
    const second = first === a ? b : a;
    const overlap = end(first) - second.timelineStartMillis;
    // A contained clip needs a different envelope; do not silently fade it out.
    if (overlap <= 0 || first.timelineStartMillis === second.timelineStartMillis || end(second) <= end(first)
        || overlap > duration(first) - first.fadeInMillis || overlap > duration(second) - second.fadeOutMillis) return null;
    return {first: first.id, second: second.id, duration: overlap};
}
function pinnedSource(clip, sources, assets) {
    const asset = assets.find(value => value.id === clip.assetId) || {};
    if (clip.adjustmentRevisionId === 0) return {};
    return sources.find(source => source.adjustmentRevisionId === clip.adjustmentRevisionId
        && (source.clipId === clip.id || source.path === asset.path))
        || (Number(asset.adjustmentRevision || 0) === clip.adjustmentRevisionId ? asset : null);
}
function sourceSpans(source, originalDuration) {
    if (!source || !source.editSegments || source.editSegments.length === 0) {
        const start = Number(source && source.trimStartMillis || 0);
        const end = Number(source && source.trimEndMillis || originalDuration);
        return [{start: 0, end: Math.max(0, end - start), sourceStart: start, sourceEnd: end, silent: false}];
    }
    const spans = [];
    let cursor = 0;
    for (const segment of source.editSegments) {
        if (Number(segment.state) !== 2) {
            const length = segment.sourceEndMillis - segment.sourceStartMillis;
            spans.push({start: cursor, end: cursor + length, sourceStart: segment.sourceStartMillis,
                        sourceEnd: segment.sourceEndMillis, silent: Number(segment.state) === 1});
            cursor += length;
        }
        const gap = Number(segment.gapAfterMillis || 0);
        if (gap > 0) spans.push({start: cursor, end: cursor + gap, sourceStart: 0, sourceEnd: 0, silent: true});
        cursor += gap;
    }
    return spans;
}
function visibleSpans(spans, clip) {
    const result = [];
    for (const span of spans) {
        const start = Math.max(span.start, clip.sourceStartMillis), end = Math.min(span.end, clip.sourceEndMillis);
        if (end > start) result.push({offset: start - clip.sourceStartMillis, length: end - start,
            sourceStart: span.sourceStart + start - span.start, sourceEnd: span.sourceStart + end - span.start, silent: span.silent});
    }
    return result;
}
function fadeValue(position, curve) {
    const value = clamp(position, 0, 1);
    if (curve === "equal_power") return Math.sin(value * Math.PI / 2);
    if (curve === "smooth") return value * value * (3 - 2 * value);
    return value;
}

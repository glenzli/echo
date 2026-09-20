.pragma library
// Transcript times stay in the immutable original, just like the source edit draft.
function bounds(segment, margin, trimStart, trimEnd) {
    if (!Number.isFinite(segment.start) || !Number.isFinite(segment.end) || segment.end <= segment.start) return null;
    const start = Math.max(trimStart, Math.floor(segment.start * 1000) - margin);
    const end = Math.min(trimEnd, Math.ceil(segment.end * 1000) + margin);
    return end > start ? {start:start,end:end} : null;
}

function units(segments) {
    const result = [];
    for (const segment of segments) for (const word of segment.words || []) result.push(word);
    return result;
}

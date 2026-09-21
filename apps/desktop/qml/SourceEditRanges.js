.pragma library
// Atomic batch source edits. Output retains the existing source partition,
// gain, external fades and inserted gaps; no partial result may be published.
function normalized(ranges, start, end) {
    if (!Array.isArray(ranges) || !ranges.length || !Number.isFinite(start) || !Number.isFinite(end) || end <= start) return null;
    const sorted = [];
    for (const range of ranges) {
        if (!range || !Number.isFinite(range.start) || !Number.isFinite(range.end) || range.end <= range.start) return null;
        const a = Math.max(start, Math.round(range.start)), b = Math.min(end, Math.round(range.end));
        if (b > a) sorted.push({start:a, end:b});
    }
    sorted.sort((a,b) => a.start - b.start || a.end - b.end);
    const merged = [];
    for (const range of sorted) {
        const previous = merged[merged.length - 1];
        if (previous && range.start <= previous.end) previous.end = Math.max(previous.end, range.end);
        else merged.push(range);
    }
    return merged.length ? merged : null;
}

function apply(segments, ranges, start, end, kind) {
    const selected = normalized(ranges, start, end);
    if (!selected || (kind !== "hide" && kind !== "keep")) return {ok:false, error:"invalid"};
    const next = [];
    let rangeIndex = 0, changed = false;
    for (const segment of segments) {
        const a = segment.sourceStartMillis, b = segment.sourceEndMillis;
        while (rangeIndex < selected.length && selected[rangeIndex].end <= a) ++rangeIndex;
        const cuts = [a];
        for (let i = rangeIndex; i < selected.length && selected[i].start < b; ++i) {
            if (selected[i].start > a) cuts.push(selected[i].start);
            if (selected[i].end < b) cuts.push(selected[i].end);
        }
        cuts.push(b);
        let cursor = rangeIndex;
        const pieces = [];
        for (let i = 1; i < cuts.length; ++i) {
            const left = cuts[i - 1], right = cuts[i];
            while (cursor < selected.length && selected[cursor].end <= left) ++cursor;
            const inside = cursor < selected.length && selected[cursor].start <= left && selected[cursor].end >= right;
            const state = (kind === "hide" ? inside : !inside) ? 2 : segment.state;
            const previous = pieces[pieces.length - 1];
            if (previous && previous.state === state) previous.end = right;
            else pieces.push({start:left, end:right, state:state});
        }
        if (pieces.length === 1 && pieces[0].state === segment.state) { next.push(segment); continue; }
        changed = true;
        for (const piece of pieces) {
            const copy = Object.assign({}, segment);
            copy.sourceStartMillis = piece.start; copy.sourceEndMillis = piece.end; copy.state = piece.state;
            copy.fadeInMillis = piece.start === a ? Math.min(segment.fadeInMillis, piece.end - piece.start) : 0;
            copy.fadeOutMillis = piece.end === b ? Math.min(segment.fadeOutMillis, piece.end - piece.start) : 0;
            copy.gapAfterMillis = piece.end === b ? segment.gapAfterMillis : 0;
            next.push(copy);
        }
    }
    if (next.length > 128) return {ok:false, error:"limit"};
    if (!next.some(s => s.state !== 2 || s.gapAfterMillis > 0)) return {ok:false, error:"empty"};
    return {ok:true, changed:changed, segments:next};
}

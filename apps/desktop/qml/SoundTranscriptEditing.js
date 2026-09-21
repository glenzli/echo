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

function entries(segments) {
    return segments.map((s,index) => Object.assign({},s,{key:"text:"+index,kind:"text"}));
}

// Gap evidence is the complement of speech coverage, not detected silence.
// Unknown-duration units block suggestions on either side of their cluster.
function gaps(units, minimum, padding) {
    if (!Number.isFinite(minimum) || minimum < 0 || !Number.isFinite(padding) || padding < 0) return [];
    const coverage = [];
    let previousStart = -Infinity;
    for (const unit of units) {
        if (!Number.isFinite(unit.start) || !Number.isFinite(unit.end) || unit.end < unit.start || unit.start < previousStart) return [];
        previousStart = unit.start;
        const previous = coverage[coverage.length - 1];
        if (previous && unit.start <= previous.end) {
            previous.end = Math.max(previous.end,unit.end);
            previous.uncertain = previous.uncertain || unit.end === unit.start;
            previous.lastText = unit.text;
        } else coverage.push({start:unit.start,end:unit.end,firstText:unit.text,lastText:unit.text,uncertain:unit.end===unit.start});
    }
    const result = [];
    for (let i = 1; i < coverage.length; ++i) {
        const left = coverage[i-1], right = coverage[i];
        const start = Math.ceil(left.end*1000) + padding, end = Math.floor(right.start*1000) - padding;
        if (left.uncertain || right.uncertain || Math.round((right.start-left.end)*1000) < minimum || end <= start) continue;
        result.push({key:"gap:"+i,kind:"gap",start:start/1000,end:end/1000,text:left.lastText+" → "+right.firstText});
    }
    return result;
}

function searchable(text) { return String(text || "").toLowerCase().replace(/\s+/g, ""); }

function filtered(entries, query, joined) {
    const needle = searchable(query);
    if (!needle) return entries;
    if (!joined) return entries.filter(entry => searchable(entry.text).includes(needle));
    // Map complete phrase hits back to the supplied units without inventing
    // sub-word timestamps (Chinese characters and spaced words both work).
    let haystack = "";
    const spans = entries.map(entry => {
        const start = haystack.length;
        haystack += searchable(entry.text);
        return {start:start,end:haystack.length};
    });
    const hits = [];
    for (let offset = haystack.indexOf(needle); offset !== -1; offset = haystack.indexOf(needle, offset + needle.length))
        hits.push({start:offset,end:offset+needle.length});
    let cursor = 0;
    return entries.filter((entry,index) => {
        const span = spans[index];
        while (cursor < hits.length && hits[cursor].end <= span.start) ++cursor;
        return cursor < hits.length && hits[cursor].start < span.end && hits[cursor].end > span.start;
    });
}

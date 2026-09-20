.pragma library
.import "SoundAssemblyEditing.js" as Editing

// Batch arrangement transforms. Selection is transient; every result preserves
// pinned source revisions and is published by the workspace as one undo step.
function clips(tracks, ids) {
    return tracks.reduce((all, track) => all.concat(track.clips), []).filter(clip => ids.includes(clip.id));
}
function bounds(tracks, ids) {
    const selected = clips(tracks, ids);
    return selected.length ? {start: Math.min(...selected.map(c => c.timelineStartMillis)),
        end: Math.max(...selected.map(Editing.end))} : null;
}
function moveLimits(tracks, ids) {
    const range = bounds(tracks, ids);
    return range ? {minimum: -range.start, maximum: 14400000 - range.end} : {minimum: 0, maximum: 0};
}
function move(tracks, ids, delta) {
    const limit = moveLimits(tracks, ids);
    const offset = Math.round(Editing.clamp(delta, limit.minimum, limit.maximum));
    return tracks.map(track => Object.assign({}, track, {clips: track.clips.map(clip => ids.includes(clip.id)
        ? Object.assign({}, clip, {timelineStartMillis: clip.timelineStartMillis + offset}) : clip)}));
}
function duplicate(tracks, ids, newId) {
    const range = bounds(tracks, ids), selected = clips(tracks, ids);
    if (!range) return {error: "selection"};
    if (tracks.reduce((sum, track) => sum + track.clips.length, 0) + selected.length > 256) return {error: "capacity"};
    const offset = range.end - range.start;
    if (range.end + offset > 14400000) return {error: "duration"};
    const copies = selected.map(clip => Object.assign({}, clip, {id: newId(), timelineStartMillis: clip.timelineStartMillis + offset}));
    return {tracks: tracks.map(track => Object.assign({}, track, {clips: track.clips.concat(copies.filter((copy, i) =>
        track.clips.some(clip => clip.id === selected[i].id)))})), ids: copies.map(clip => clip.id)};
}
function split(tracks, ids, position, newId) {
    let added = 0;
    for (const clip of clips(tracks, ids)) if (Editing.split(clip, position)) ++added;
    if (!added) return {error: "split"};
    if (tracks.reduce((sum, track) => sum + track.clips.length, 0) + added > 256) return {error: "capacity"};
    const selected = [];
    const result = tracks.map(track => {
        const next = [];
        for (const clip of track.clips) {
            if (!ids.includes(clip.id)) { next.push(clip); continue; }
            const halves = Editing.split(clip, position);
            if (!halves) { selected.push(clip.id); next.push(clip); continue; }
            halves[1].id = newId();
            selected.push(halves[0].id, halves[1].id);
            next.push(halves[0], halves[1]);
        }
        return Object.assign({}, track, {clips: next});
    });
    return {tracks: result, ids: selected};
}
function remove(tracks, ids) {
    const result = tracks.map(track => Object.assign({}, track, {clips: track.clips.filter(clip => !ids.includes(clip.id))}));
    return result.some(track => track.clips.length) ? {tracks: result, ids: []} : {error: "empty"};
}
function moveTracks(tracks, ids, delta) {
    if (tracks.some((track, index) => track.clips.some(clip => ids.includes(clip.id)) && (index + delta < 0 || index + delta >= tracks.length))) return {error: "track"};
    const result = tracks.map(track => Object.assign({}, track, {clips: track.clips.filter(clip => !ids.includes(clip.id))}));
    tracks.forEach((track, index) => track.clips.filter(clip => ids.includes(clip.id)).forEach(clip => result[index + delta].clips.push(clip)));
    return {tracks: result, ids: ids};
}
function ranges(tracks, ids) {
    const sorted = clips(tracks, ids).map(clip => ({start: clip.timelineStartMillis, end: Editing.end(clip)})).sort((a,b) => a.start - b.start);
    const result = [];
    for (const range of sorted) {
        const last = result[result.length - 1];
        if (last && range.start <= last.end) last.end = Math.max(last.end, range.end);
        else result.push(range);
    }
    return result;
}
function cutRange(clip, range, newId) {
    if (Editing.end(clip) <= range.start) return [clip];
    const removed = range.end - range.start;
    if (clip.timelineStartMillis >= range.end) return [Object.assign({}, clip, {timelineStartMillis: clip.timelineStartMillis - removed})];
    const result = [];
    if (clip.timelineStartMillis < range.start) result.push(Editing.fitFades(Object.assign({}, clip, {
        sourceEndMillis: clip.sourceStartMillis + range.start - clip.timelineStartMillis, fadeOutMillis: 0})));
    if (Editing.end(clip) > range.end) result.push(Editing.fitFades(Object.assign({}, clip, {
        id: result.length ? newId() : clip.id,
        sourceStartMillis: clip.sourceStartMillis + range.end - clip.timelineStartMillis,
        timelineStartMillis: range.start, fadeInMillis: 0})));
    return result;
}
function ripple(tracks, ids, allTracks, newId) {
    const removed = ranges(tracks, ids);
    if (!removed.length) return {error: "selection"};
    const result = tracks.map(track => {
        if (!allTracks && !track.clips.some(clip => ids.includes(clip.id))) return track;
        let remaining = track.clips;
        // Descending order keeps every range in the original document coordinate.
        for (let i = removed.length - 1; i >= 0; --i) remaining = remaining.reduce((all, clip) => all.concat(cutRange(clip, removed[i], newId)), []);
        return Object.assign({}, track, {clips: remaining});
    });
    const count = result.reduce((sum, track) => sum + track.clips.length, 0);
    return count === 0 ? {error: "empty"} : count > 256 ? {error: "capacity"} : {tracks: result, ids: [], position: removed[0].start};
}

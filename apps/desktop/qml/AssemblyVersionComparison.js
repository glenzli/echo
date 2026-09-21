.pragma library
// Read-only authored differences. Resolved file paths and render metadata are not edits.
function canonical(value) {
    if (Array.isArray(value)) return "["+value.map(canonical).join(",")+"]";
    if (value && typeof value === "object") return "{"+Object.keys(value).sort().map(key=>JSON.stringify(key)+":"+canonical(value[key])).join(",")+"}";
    return JSON.stringify(value);
}
function index(document) {
    const tracks={}, clips={};
    (document.tracks || []).forEach((track, order) => {
        const properties=Object.assign({},track,{order:order}); delete properties.clips;
        tracks[track.id]=properties;
        (track.clips || []).forEach(clip=>clips[clip.id]=Object.assign({},clip,{trackId:track.id}));
    });
    return {tracks:tracks,clips:clips};
}
function compare(current, saved) {
    const now=index(current), before=index(saved), rows=[];
    function changes(kind, a, b) {
        const ids=Array.from(new Set(Object.keys(a).concat(Object.keys(b))));
        for (const id of ids) {
            const left=a[id], right=b[id];
            if (canonical(left)===canonical(right)) continue;
            const groups=[];
            if (left && right && kind==="clip") {
                const fields={position:["timelineStartMillis","trackId"],range:["sourceStartMillis","sourceEndMillis"],source:["assetId","sourceRole"],processing:["adjustmentRevisionId"],mix:["gainCentibels","panPercent","muted"],fades:["fadeInMillis","fadeOutMillis","fadeInCurve","fadeOutCurve"],envelope:["gainEnvelope"]};
                for (const group of Object.keys(fields)) if (fields[group].some(key=>canonical(left[key])!==canonical(right[key]))) groups.push(group);
            }
            rows.push({kind:kind,id:id,assetId:(left||right).assetId || "",name:(left||right).name || "",change:!right?"added":!left?"removed":"changed",groups:groups});
        }
    }
    if (current.name!==saved.name) rows.push({kind:"name",change:"changed",name:current.name});
    if (canonical(current.master)!==canonical(saved.master)) rows.push({kind:"master",change:"changed"});
    changes("track",now.tracks,before.tracks); changes("clip",now.clips,before.clips);
    const markers=value=>{const result={};(value.markers||[]).forEach(marker=>result[marker.id]=marker);return result;};
    changes("marker",markers(current),markers(saved));
    return rows;
}

.pragma library

// Literal results stay first. Similarity candidates never cross collection filters.
function ranked(assets, query, candidates, candidateQuery) {
    const normalized = query.trim().replace(/\s+/g, " "), terms = normalized.toLocaleLowerCase().split(/\s+/).filter(Boolean);
    const literal = [], remaining = new Map();
    for (const asset of assets) {
        const haystack = [asset.sourceTitle,asset.path,asset.summary,asset.eventType,asset.mood,(asset.keywords||[]).join(" ")].join(" ").toLocaleLowerCase();
        if (terms.every(term => haystack.indexOf(term) >= 0)) literal.push(asset); else remaining.set(asset.id,asset);
    }
    if (!normalized || normalized !== candidateQuery) return literal;
    for (const hit of candidates) if (remaining.has(hit.id)) { literal.push(Object.assign({},remaining.get(hit.id),{semanticCandidate:true})); remaining.delete(hit.id); }
    return literal;
}

// Source-level disclosure. Unmarked sources are unknown, not verified recordings.
// CXX QVariantList values are Qt sequences, which are not always JS Arrays.
function list(value) {
    const result = [];
    if (value && typeof value.length === "number") {
        for (let i = 0; i < value.length; ++i) result.push(value[i]);
    }
    return result;
}
function sources(asset) {
    return list(asset && asset.sourceDisclosure ? asset.sourceDisclosure.sources : null);
}
function generated(asset) {
    return !!(asset && asset.hasGeneratedSource) || sources(asset).some(source => list(source.spans).some(span => span.kind === "ai_generated" || span.kind === "reconstructed_speech"));
}
function processed(asset) {
    return !!(asset && asset.hasAiProcessedSource) || sources(asset).some(source => list(source.spans).some(span => span.kind === "ai_processed"));
}
function eligible(asset, scope, includeGenerated) {
    return !generated(asset) || (scope !== "originals" && includeGenerated === true);
}
function ownRevision(asset) {
    return sources(asset).find(source => source.assetId === asset.id) || {revisionId:0,spans:[]};
}

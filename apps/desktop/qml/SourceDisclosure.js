// Source-level disclosure. Unmarked sources are unknown, not verified recordings.
function sources(asset) {
    return asset && asset.sourceDisclosure && Array.isArray(asset.sourceDisclosure.sources) ? asset.sourceDisclosure.sources : [];
}
function generated(asset) {
    return !!(asset && asset.hasGeneratedSource) || sources(asset).some(source => (source.spans || []).some(span => span.kind === "ai_generated" || span.kind === "reconstructed_speech"));
}
function processed(asset) {
    return !!(asset && asset.hasAiProcessedSource) || sources(asset).some(source => (source.spans || []).some(span => span.kind === "ai_processed"));
}
function eligible(asset, scope, includeGenerated) {
    return !generated(asset) || (scope !== "originals" && includeGenerated === true);
}
function ownRevision(asset) {
    return sources(asset).find(source => source.assetId === asset.id) || {revisionId:0,spans:[]};
}

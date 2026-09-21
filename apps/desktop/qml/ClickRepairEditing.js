.pragma library
.import "SourceEditRanges.js" as Ranges
// Add local repair scope without narrowing or waking existing authored effects.
function propose(snapshot, candidates) {
    if (!Array.isArray(candidates) || !candidates.length) return {ok:false,error:"range"};
    const ranges = Ranges.normalized(candidates.map(c => ({start:Math.max(snapshot.trimStartMillis,c.startMillis-10),end:Math.min(snapshot.trimEndMillis,c.endMillis+10)})),snapshot.trimStartMillis,snapshot.trimEndMillis);
    if (!ranges) return {ok:false,error:"range"};
    const current = snapshot.effectMasks.filter(m => m.effectNodes.indexOf(6)!==-1);
    const present = snapshot.effectChain.indexOf(6)!==-1;
    if (present && snapshot.deClickEnabled && !current.length) return {ok:false,error:"global"};
    if (present && !snapshot.deClickEnabled && current.length) return {ok:false,error:"bypassed"};
    if (snapshot.deClickRepairPercent<=0) return {ok:false,error:"amount"};
    if (!present && snapshot.effectChain.length>=22) return {ok:false,error:"chain"};
    const additions = ranges.filter(r => !current.some(m => (r.start===m.startMillis && r.end===m.endMillis) || (r.start>=m.startMillis+m.featherMillis && r.end<=m.endMillis-m.featherMillis)));
    if (snapshot.effectMasks.length+additions.length>64) return {ok:false,error:"limit"};
    if (!additions.length) return {ok:true,changed:false};
    const next = JSON.parse(JSON.stringify(snapshot));
    if (!present) next.effectChain.unshift(6);
    next.deClickEnabled=true;
    next.effectMasks=next.effectMasks.concat(additions.map(r=>({startMillis:r.start,endMillis:r.end,featherMillis:r.start===snapshot.trimStartMillis || r.end===snapshot.trimEndMillis ? 0 : 5,effectNodes:[6]})));
    return {ok:true,changed:true,snapshot:next};
}

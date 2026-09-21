import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
const d=vm.createContext({});vm.runInContext(fs.readFileSync(new URL('../qml/SourceDisclosure.js',import.meta.url),'utf8'),d);
const asset=kind=>({id:'a',sourceDisclosure:{sources:[{assetId:'a',revisionId:17,spans:[{kind}]}]}});
test('unknown and AI processing do not masquerade as generated; reconstruction is generated',()=>{
 assert.equal(d.generated({}),false);assert.equal(d.eligible({},'memories',false),true);
 assert.equal(d.processed(asset('ai_processed')),true);assert.equal(d.eligible(asset('ai_processed'),'memories',false),true);
 for(const kind of ['ai_generated','reconstructed_speech']) {
  assert.equal(d.generated(asset(kind)),true);
  for(const scope of ['memories','materials','originals'])assert.equal(d.eligible(asset(kind),scope,false),false);
  assert.equal(d.eligible(asset(kind),'memories',true),true);assert.equal(d.eligible(asset(kind),'materials',true),true);assert.equal(d.eligible(asset(kind),'originals',true),false);
 }
});
test('explicit backend flags remain conservative and cleared revision is editable',()=>{
 assert.equal(d.generated({hasGeneratedSource:true}),true);assert.equal(d.ownRevision(asset('ai_generated')).revisionId,17);
 assert.equal(d.ownRevision({id:'x'}).revisionId,0);
 assert.equal(d.generated({sourceDisclosure:{sources:[{assetId:'a',revisionId:18,spans:[]}]}}),false);
});

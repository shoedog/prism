const LIMIT=100000;
const oldReasons=new Set(['unprocessed','unresolved','target_not_in_program','missing_inclusion']);

function invalid() { throw Error('invalid_entry_obligations'); }
function validInteger(value) { return Number.isSafeInteger(value) && value>=0; }

// This is deliberately a pure classifier over already validated entry rows. It does
// not inspect targets, the Program, config ASTs, or the filesystem.
export function classifyEntryObligations(input,cap=LIMIT) {
  if(!input || typeof input!=='object' || Array.isArray(input)
    || !Number.isSafeInteger(cap) || cap<1 || cap>LIMIT
    || !validInteger(input.rootCount)
    || typeof input.configObserved!=='boolean'
    || typeof input.noLib!=='boolean'
    || typeof input.explicitTypes!=='boolean'
    || !Array.isArray(input.entries) || input.entries.length>cap) invalid();
  const noLib=input.configObserved&&input.noLib;
  const rows=[];
  for(const entry of input.entries) {
    if(!entry || typeof entry!=='object' || Array.isArray(entry)
      || !['types','lib'].includes(entry.kind)
      || !['configured','automatic','default'].includes(entry.origin)
      || entry.kind==='types'&&entry.origin==='default'
      || entry.kind==='lib'&&entry.origin==='automatic'
      || !validInteger(entry.index)
      || !['observed','unproven'].includes(entry.status)
      || entry.status==='observed'&&entry.reason!==null
      || entry.status==='unproven'&&!oldReasons.has(entry.reason)) invalid();
    let disposition,reason;
    if(entry.status==='observed') {
      disposition='selected';reason=null;
    } else if(entry.reason==='unprocessed'&&input.rootCount===0) {
      disposition='disabled';reason='no_roots';
    } else if(entry.reason==='unprocessed'&&entry.kind==='lib'&&noLib) {
      disposition='disabled';reason='no_lib';
    } else {
      disposition='unproven';reason=entry.reason==='unprocessed'&&entry.kind==='lib'?'suppression_unproven':entry.reason;
    }
    rows.push({kind:entry.kind,origin:entry.origin,index:entry.index,disposition,reason});
  }
  const reasons=[];
  if(!input.configObserved) reasons.push('configuration_unproven');
  if(input.configObserved&&input.rootCount>0&&!input.explicitTypes) reasons.push('automatic_discovery_unproven');
  if(rows.some(row=>row.disposition==='unproven')) reasons.push('unproven_entry');
  return {complete:reasons.length===0,reasons,rows};
}

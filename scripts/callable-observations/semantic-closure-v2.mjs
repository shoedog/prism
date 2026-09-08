import {classifySemanticClosure} from './semantic-closure.mjs';

export const SEMANTIC_CLOSURE_POLICY_V2='prism.semantic-closure/singleton-wildcard-v2';

// Extension over the byte-frozen v1 result. Inputs remain facts authenticated by
// the packet parser and full reproduction; this does not recreate binding proof.
export function classifySemanticClosureV2(input,cap) {
  const prior=classifySemanticClosure(input,cap);
  const rows=prior.rows.map(row=>{
    const resolution=input.resolutions[row.index];
    return row.disposition==='unproven'&&row.reason==='unadmitted_binding'
      &&resolution.target===null&&resolution.lookup.wildcard?.status==='observed'
      ?{index:row.index,disposition:'singleton_wildcard',reason:null}:row;
  });
  const reasons=prior.reasons.filter(reason=>reason!=='resolution_unproven');
  if(rows.some(row=>row.disposition==='unproven'))reasons.push('resolution_unproven');
  return {policy:SEMANTIC_CLOSURE_POLICY_V2,complete:reasons.length===0,reasons,rows};
}

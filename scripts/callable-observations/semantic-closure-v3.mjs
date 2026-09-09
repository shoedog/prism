import {classifySemanticClosureV2} from './semantic-closure-v2.mjs';

export const SEMANTIC_CLOSURE_POLICY_V3='prism.semantic-closure/merged-side-effect-v3';

// Extension over byte-frozen v2. The parser and full reproduction authenticate
// the observed merged record; this wrapper only selects its admitted row lane.
export function classifySemanticClosureV3(input,cap) {
  const prior=classifySemanticClosureV2(input,cap);
  const rows=prior.rows.map(row=>{
    const resolution=input.resolutions[row.index];
    return row.disposition==='unproven'&&row.reason==='unadmitted_binding'
      &&resolution.target===null&&resolution.lookup.merged_wildcard?.status==='observed'
      ?{index:row.index,disposition:'merged_side_effect',reason:null}:row;
  });
  const reasons=prior.reasons.filter(reason=>reason!=='resolution_unproven');
  if(rows.some(row=>row.disposition==='unproven'))reasons.push('resolution_unproven');
  return {policy:SEMANTIC_CLOSURE_POLICY_V3,complete:reasons.length===0,reasons,rows};
}

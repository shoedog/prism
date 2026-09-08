import path from 'node:path';
import {relative} from './schema.mjs';

// Pure classification; it does not mutate legacy aggregates or ledger state.
export function classifyBoundary(file) {
  const normalized=path.posix.normalize(file).replace(/\/+$/, '')||'/';
  if(normalized.split('/').some(part=>part.toLowerCase()==='.git'))return {kind:'unsafe',normalized};
  for(const root of ['project','compiler']) if(normalized===`/__prism__/${root}`||normalized.startsWith(`/__prism__/${root}/`)) {
    const id=normalized.slice('/__prism__/'.length);
    return relative(id)?{kind:'in_root',normalized,id}:{kind:'refused',normalized};
  }
  return {kind:'outside',normalized};
}

// Bounded ownership ledger for compiler module-search boundary encounters.
export function createSearchProvenance(hash,cap=100000) {
  const module_requests=[],boundary_events=[],stack=[];let claimedRequests=0;
  const bounded=array=>{if(array.length>=cap)throw Error('budget_exceeded');};
  const owner=()=>stack.length?stack.at(-1):null;
  return {
    module_requests,boundary_events,
    claimRequest() {if(claimedRequests>=cap)throw Error('budget_exceeded');return claimedRequests++;},
    withRequest(request,fn) { stack.push({channel:'module',id:request.id});try{return fn();}finally{stack.pop();} },
    boundary(kind,operation,unsafePath) {
      bounded(boundary_events);
      boundary_events.push({id:boundary_events.length,kind,operation,probe_sha256:hash(unsafePath),owner:owner()});
    },
    request(record) { bounded(module_requests);module_requests.push(record); },
  };
}

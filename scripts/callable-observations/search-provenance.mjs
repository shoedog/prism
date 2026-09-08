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
  const module_requests=[],type_batches=[],type_requests=[],type_searches=[],lib_searches=[],boundary_events=[],stack=[];
  let claimedRequests=0,claimedTypeBatches=0,claimedTypeRequests=0,claimedTypeSearches=0,claimedLibSearches=0;
  let reservedLibBeneficiaries=0,committedLibBeneficiaries=0;
  const bounded=array=>{if(array.length>=cap)throw Error('budget_exceeded');};
  const owner=()=>stack.length?stack.at(-1):null;
  const withOwner=(channel,id,fn)=>{stack.push({channel,id});try{return fn();}finally{stack.pop();}};
  return {
    module_requests,type_batches,type_requests,type_searches,lib_searches,boundary_events,
    claimRequest() {if(claimedRequests>=cap)throw Error('budget_exceeded');return claimedRequests++;},
    claimTypeBatch() {if(claimedTypeBatches>=cap)throw Error('budget_exceeded');return claimedTypeBatches++;},
    claimTypeRequest() {if(claimedTypeRequests>=cap)throw Error('budget_exceeded');return claimedTypeRequests++;},
    claimTypeSearch() {if(claimedTypeSearches>=cap)throw Error('budget_exceeded');return claimedTypeSearches++;},
    claimLibSearch() {if(claimedLibSearches>=cap)throw Error('budget_exceeded');return claimedLibSearches++;},
    withOwner,
    withRequest(request,fn) {return withOwner('module',request.id,fn);},
    withTypeSearch(search,fn) {return withOwner('type',search.id,fn);},
    withLibSearch(search,fn) {return withOwner('lib',search.id,fn);},
    boundary(kind,operation,unsafePath) {
      bounded(boundary_events);
      boundary_events.push({id:boundary_events.length,kind,operation,probe_sha256:hash(unsafePath),owner:owner()});
    },
    request(record) { bounded(module_requests);module_requests.push(record); },
    typeBatch(record) {if(record.size>cap)throw Error('budget_exceeded');bounded(type_batches);type_batches.push(record);},
    typeRequest(record) {bounded(type_requests);type_requests.push(record);},
    typeSearch(record) {bounded(type_searches);type_searches.push(record);},
    libBeneficiaries(array,records) {
      if(records.length>cap-array.length||records.length>cap-reservedLibBeneficiaries)throw Error('budget_exceeded');
      reservedLibBeneficiaries+=records.length;for(const record of records)array.push(record);
    },
    libSearch(record) {
      if(record.beneficiaries.length>cap||committedLibBeneficiaries>cap-record.beneficiaries.length)throw Error('budget_exceeded');
      committedLibBeneficiaries+=record.beneficiaries.length;bounded(lib_searches);lib_searches.push(record);
    },
  };
}

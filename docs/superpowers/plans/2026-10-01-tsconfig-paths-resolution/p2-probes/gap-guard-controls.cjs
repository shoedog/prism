// Explicit public seam controls, not purported whole-program P2 failures.
// Usage: node gap-guard-controls.cjs PUBLIC_EVIDENCE_DIR
const fs=require('fs'),path=require('path'),assert=require('assert');
const {bindingGate,projectionGate}=require('./gap-projection.cjs');
const out=process.argv[2],side=JSON.parse(fs.readFileSync(path.join(out,'gap-sidecar.json')));
const file=Object.keys(side.path_exports.true).find(f=>f.includes('tsx-relative-positive/')&&!f.includes('member'));
const actual=side.path_exports.true[file].real,site={local_binding:{Unproven:'import'},jsx_element:true};
assert(actual.span&&actual.file,'public actual export missing');
const entries=side.entries.filter(r=>r.key[0].includes('tsx-relative-positive/')&&!r.key[0].includes('member'));
const b=entries[0].binding,f=side.exports[entries[0].key[0]];
const tests=[];
function check(name,seen,want){assert.equal(seen,want);tests.push({name,result:seen});}
check('binding-positive',bindingGate(b,f,b.local,site),null);
check('binding-ineligible-negative',bindingGate({...b,eligible:false},f,b.local,site),'IMPORT_BINDING_GATE');
check('binding-kind-negative',bindingGate({...b,kind:'NamespaceImport'},f,b.local,site),'IMPORT_BINDING_GATE');
check('binding-provenance-negative',bindingGate(b,{esm_named_imports:[]},b.local,site),'IMPORT_BINDING_GATE');
check('shadow-negative',bindingGate(b,f,b.local,{...site,local_binding:{Unproven:'shadow'}}),'SITE_BINDING_GATE');
check('wrapper-call-negative',projectionGate({...actual,wrapped:true},{...site,jsx_element:false},side.functions),'WRAPPED_NON_JSX');
check('wrapper-jsx-positive',projectionGate({...actual,wrapped:true},site,side.functions),'POST_EXPORT_SITE_GATE');
check('terminal-span-negative',projectionGate({...actual,span:null},site,side.functions),'TERMINAL_SPAN_REQUIRED');
const target=f=>f.file===actual.file&&f.name===actual.local_name&&f.start_line===actual.span[0]&&f.end_line===actual.span[1];
check('span-lookup-contract-negative',projectionGate(actual,site,side.functions.filter(f=>!target(f))),'TERMINAL_FUNCTION_LOOKUP');
check('span-lookup-duplicate-negative',projectionGate(actual,site,[...side.functions,side.functions.find(target)]),'TERMINAL_FUNCTION_LOOKUP');
check('site-output-contract-negative',projectionGate(actual,site,side.functions),'POST_EXPORT_SITE_GATE');
const summary={status:'PASS',guard_contract_checks:tests.length,source:'PUBLIC_ACTUAL_EXPORT_WITH_EXPLICIT_SEAM_VARIATIONS',tests};
fs.writeFileSync(path.join(out,'gap-guard-checks.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify({status:'PASS',guard_contract_checks:tests.length}));

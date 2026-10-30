// Pure query-guard explainer. Called only after actual export-table replay.
// The POST result is explicitly unresolved, never an invented resolver cause.
function bindingGate(binding,callerFacts,local,site) {
  if(!binding?.eligible||binding.kind!=='MemberImport'||!callerFacts?.esm_named_imports.includes(local))return 'IMPORT_BINDING_GATE';
  if(JSON.stringify(site.local_binding)!==JSON.stringify({Unproven:'import'}))return 'SITE_BINDING_GATE';
  return null;
}
function projectionGate(actual,site,functions) {
  if(actual.wrapped&&!site.jsx_element)return 'WRAPPED_NON_JSX';
  if(!actual.span)return 'TERMINAL_SPAN_REQUIRED';
  const matches=functions.filter(f=>f.file===actual.file&&f.name===actual.local_name&&f.start_line===actual.span[0]&&f.end_line===actual.span[1]);
  return matches.length!==1?'TERMINAL_FUNCTION_LOOKUP':'POST_EXPORT_SITE_GATE';
}
module.exports={bindingGate,projectionGate};

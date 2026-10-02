"""Finite isolated mutants of actual P1 resolver modules; no prototype source mutation."""
import json,subprocess,sys,shutil
from pathlib import Path
repo=Path(sys.argv[1]).resolve();evidence=Path(sys.argv[2]).resolve();deps=Path(sys.argv[3]).resolve();controls=Path(sys.argv[4]).resolve()
externs=json.loads(Path(sys.argv[5]).read_text()) if len(sys.argv)>5 else None
packet=Path(__file__).resolve().parent
modules=['js_paths.rs','js_paths_snapshot.rs','js_paths_syntax.rs','js_paths_first_pass.rs']
text={p:(repo/'src'/p).read_text() for p in modules}
facts=[json.loads(l) for l in (evidence/'controls-import-facts.jsonl').read_text().splitlines()]
req=sorted({(f['file'],b['module_path']) for f in facts for b in f['bindings'] or [] if b['kind']=='MemberImport' and not b['module_path'].startswith('.')})
requests=evidence/'resolver-requests.json';requests.write_text(json.dumps(req))
mutants={
 'M01-no-exact':('js_paths.rs','if paths.contains_key(spec) {','if false {'),
 'M02-shortest-prefix':('js_paths.rs','Some((m, _, _)) if *m > n => {}','Some((m, _, _)) if *m < n => {}'),
 'M03-child-paths-origin':('js_paths.rs','c.base.as_deref().unwrap_or(&origin)','c.base.as_deref().unwrap_or(dir(file))'),
 'M04-ignore-include':('js_paths.rs','if !included {','if false && !included {'),
 'M05-ignore-exclude':('js_paths.rs','let exact = exclude_matches(p, file)?;','let exact = false;'),
 'M06-ignore-declaration-blocker':('js_paths.rs','for ext in [".ts", ".tsx", ".d.ts", ".js", ".jsx"] {','for ext in [".ts", ".tsx", ".js", ".jsx"] {'),
 'M07-ignore-package-boundary':('js_paths.rs','.contains_key(&format!("{p}/package.json"))','.contains_key(&format!("{p}/__mutant_absent.json"))'),
 'M08-accept-tied-pattern':('js_paths.rs','if tied || s.is_empty() {','if s.is_empty() {'),
 'M09-ignore-baseurl-origin':('js_paths.rs','c.base.as_deref().unwrap_or(&origin)','&origin'),
 'M10-unmatched-target-star':('js_paths.rs',"if !key.contains('*') && raw_target.contains('*') {", "if false && !key.contains('*') && raw_target.contains('*') {"),
 'M11-no-allowjs':('js_paths.rs','Some("js" | "jsx" | "mjs" | "cjs")','Some("__never")'),
 'M12-no-outdir-barrier':('js_paths.rs','if !out.is_empty() {','if false && !out.is_empty() {'),
 'M13-exclude-fullpath-only':('js_paths_syntax.rs','prefix = parent;','return Some(false);'),
 'M14-no-same-stem-barrier':('js_paths.rs','if group[..priority].iter().any(|e| {','if false && group[..priority].iter().any(|e| {'),
 'M15-include-package-folders':('js_paths_syntax.rs','if !exclude\n        && file', 'if false && !exclude\n        && file'),
 'M16-no-declaration-priority':('js_paths.rs','.max_by_key(|(_, e)| e.len())','.min_by_key(|(_, e)| e.len())'),
 'M17-byte-unicode-glob':('js_paths_syntax.rs','if !pattern.is_ascii()\n        || !file.is_ascii()','if false'),
 'M18-no-include-case-barrier':('js_paths.rs','if !exact && pattern_matches(', 'if false && !exact && pattern_matches('),
 'M19-no-exclude-case-barrier':('js_paths.rs','if !exact && exclude_matches(', 'if false && !exact && exclude_matches('),
 'M20-ignore-case-config':('js_paths_snapshot.rs','let config_name = name.to_ascii_lowercase();','let config_name = name.to_owned();'),
 'M21-restore-same-directory-jsconfig-barrier':('js_paths.rs','!self.snapshot.configs.contains_key(&p)\n                && self.snapshot.configs.contains_key(&jsconfig)','self.snapshot.configs.contains_key(&jsconfig)'),
 'M22-no-mjs-priority':('js_paths.rs','&[".mts", ".d.mts", ".mjs"]','&[".mts", ".d.mts", ".__never"]'),
 'M23-no-cjs-priority':('js_paths.rs','&[".cts", ".d.cts", ".cjs"]','&[".cts", ".d.cts", ".__never"]'),
 'M24-no-minified-barrier':('js_paths.rs','basename.ends_with(".min.js")','false'),
 'M25-no-dotfile-barrier':('js_paths.rs',"basename.starts_with('.')",'false'),
 'M26-no-question-prefix-barrier':('js_paths.rs',"segment.starts_with('?')",'false'),
 'M27-no-excluding-jsconfig-barrier':('js_paths.rs','if self.snapshot.configs.contains_key(&jsconfig)','if false'),
 'M28-no-disable-solution-barrier':('js_paths.rs','.get("disableSolutionSearching")','.get("__mutant_absent")'),
 'M29-no-tsx-ts-priority':('js_paths.rs','&[".ts", ".tsx", ".d.ts", ".js", ".jsx"]','&[".tsx", ".ts", ".d.ts", ".js", ".jsx"]'),
 'M30-no-jsx-js-priority':('js_paths.rs','&[".ts", ".tsx", ".d.ts", ".js", ".jsx"]','&[".ts", ".tsx", ".d.ts", ".jsx", ".js"]'),
}
# Rebind legacy mutations to the repaired production expressions. Redundant
# gates are disabled together when they implement the same invariant.
mutants['M03-child-paths-origin']=('js_paths.rs','c.base.as_deref().unwrap_or(origin)','c.base.as_deref().unwrap_or(dir(file))')
mutants['M09-ignore-baseurl-origin']=('js_paths.rs','c.base.as_deref().unwrap_or(origin)','origin')
mutants['M07-ignore-package-boundary']=('js_paths.rs','.kind(&format!("{p}/package.json"))','.kind(&format!("{p}/__mutant_absent.json"))')
mutants['M11-no-allowjs']=('js_paths.rs','Some("js" | "jsx" | "mjs" | "cjs")','Some("__never")')
mutants['M12-no-outdir-barrier']=('js_paths.rs','if c.exclude.is_none()','if false && c.exclude.is_none()')
mutants.update({
 'M31-no-replacement-candidate':('js_paths.rs','.kind(&format!("{stem}.d.{suffix}.ts"))','.kind(&format!("{stem}.__absent.ts"))'),
 'M32-no-spec-slash-cut':('js_paths.rs','|| directory_target(spec)','|| false'),
 'M33-no-target-slash-cut':('js_paths.rs','if directory_target(&target) {','if false && directory_target(&target) {'),
 'M34-no-js-package-cut':('js_paths.rs','self.snapshot.package_present(file, spec)','false'),
 'M35-no-ambient-cut':('js_paths.rs','if self\n            .snapshot\n            .ambient','if false && self\n            .snapshot\n            .ambient'),
 'M36-no-raw-membership-cut':('js_paths.rs','if list.iter().any(|s| !membership_pattern(s, key == "files")) {','if false && list.iter().any(|s| !membership_pattern(s, key == "files")) {'),
 'M37-no-declarationdir-cut':('js_paths.rs','"declarationDir",','"__absentDir",'),
 'M38-no-rootdir-cut':('js_paths.rs','"rootDir",','"__absentRoot",'),
})
# R2 structural cuts supersede package enumeration; keep its mutant ID bound
# to the replacement production invariant and retain the legacy population.
mutants['M34-no-js-package-cut']=('js_paths.rs','if js_family(&q)\n','if false && js_family(&q)\n')
mutants['M35-no-ambient-cut']=('js_paths.rs','if self.snapshot.ambient.values().any(|patterns| {','if false && self.snapshot.ambient.values().any(|patterns| {')
mutants.update({
 'M39-no-config-types-cut':('js_paths.rs','get("types")','get("__absent_types")'),
 'M40-no-project-reference-cut':('js_paths_snapshot.rs','let referenced = !references.is_empty();','let referenced = false;'),
 'M41-no-source-ambient-scan':('js_paths_snapshot.rs','let patterns = if ts {','let patterns = if false && ts {'),
 'M42-no-feff-whitespace':('js_paths_snapshot.rs',"c.is_whitespace() || c == '\\u{feff}'",'c.is_whitespace()'),
 'M43-ignore-malformed-candidate':('js_paths_snapshot.rs','None => return vec!["*".into()],','None => return Vec::new(),'),
 'M44-no-string-unescape':('js_paths_snapshot.rs','name.push(decoded);',"name.push('!');"),
 'M45-skip-directory-ambient':('js_paths_snapshot.rs','self.scan(root, &path, depth + 1)?;','/* mutant skips declaration directories */'),
 'M46-no-typeRoots-coverage':('js_paths.rs','.map(|root| self.snapshot.covered_root(dir(p), root))','.map(|root| Some(root.clone()))'),
 'M47-no-reference-path-arm':('js_paths_snapshot.rs','if matches!(key, "path" | "types") {','if key == "types" {'),
 'M48-no-reference-types-arm':('js_paths_snapshot.rs','if matches!(key, "path" | "types") {','if key == "path" {'),
 'M49-restore-broad-type-cut':('js_paths.rs','let opts = v.as_object()?;','let opts = v.as_object()?; if opts.contains_key("types") || opts.contains_key("typeRoots") { return None; }'),
 'M50-restore-broad-reference-cut':('js_paths_snapshot.rs','references.push((key.into(), name));','references.push((key.into(), String::new()));'),
 'M51-no-source-coverage':('js_paths_snapshot.rs','self.covered.insert(rel.into(), 0);','/* omit source coverage */'),
 'M52-no-secondary-type-lookup':('js_paths_snapshot.rs','let mut directory = base;','if roots.is_some() { return TypeInput::Absent; } let mut directory = base;'),
 'M53-no-inherited-types-recheck':('js_paths.rs','c.options.get("types")','c.options.get("__absent_types")'),
})
mutants.update({
 'M54-no-node-file':('js_paths_first_pass.rs','let suffixes: &[&str] = match original {','if stem.contains("node_modules/") && !stem.contains("/@types/") && !stem.ends_with("/index") { return true; } let suffixes: &[&str] = match original {'),
 'M55-no-node-index':('js_paths_first_pass.rs','let suffixes: &[&str] = match original {','if stem.contains("node_modules/") && !stem.contains("/@types/") && stem.ends_with("/index") { return true; } let suffixes: &[&str] = match original {'),
 'M56-no-at-types':('js_paths_first_pass.rs','absent &= pass.module(&format!("{modules}/@types"), &mangle(spec), false)?;','/* omit @types */'),
 'M57-no-module-types':('js_paths_first_pass.rs','types: Value,','#[serde(rename="__types")] types: Value,'),
 'M58-no-module-typings':('js_paths_first_pass.rs','typings: Value,','#[serde(rename="__typings")] typings: Value,'),
 'M59-no-module-versions':('js_paths_first_pass.rs','range_matches(range) == Some(true)','false'),
 'M60-no-module-typeRoots':('js_paths_first_pass.rs','for root in roots.unwrap_or_default() {','for root in roots.unwrap_or_default().iter().take(0) {'),
 'M61-no-module-main':('js_paths_first_pass.rs','main: Value,','#[serde(rename="__main")] main: Value,'),
})
mutants.update({
 'M62-no-JS-object-order':('js_paths_first_pass.rs','.filter(|n| *n != u32::MAX && n.to_string() == *key)','.filter(|_| false)'),
 'M63-no-last-duplicate-value':('js_paths_first_pass.rs','.find(|(name, _)| name == &key)','.find(|_| false)'),
 'M64-range-u64-overflow':('js_paths_first_pass.rs','let n = part.parse().ok()?;','let n = part.parse::<u64>().ok()? as f64;'),
 'M65-no-paths-first-pass':('js_paths_first_pass.rs','let mut absent = pass.relative(target, true, 0)?;','let mut absent = true;'),
 'M66-Rust-version-whitespace':('js_paths_first_pass.rs','fn version_whitespace(c: char) -> bool {','fn version_whitespace(c: char) -> bool { return c.is_whitespace();'),
 'M67-no-root-file-boundary':('js_paths_first_pass.rs','if p.is_empty() {','if false && p.is_empty() {'),
 'M68-no-physical-absence':('js_paths_snapshot.rs','fn first_pass_occupancy(&self, p: &str) -> Option<u8> {','fn first_pass_occupancy(&self, p: &str) -> Option<u8> { return self.type_entries.get(p).or_else(|| self.entries.get(p)).copied();'),
})
# Preflight the entire mutation population before compiling any mutant.
for label,(file,old,new) in mutants.items():
 expected=2 if label in ['M06-ignore-declaration-blocker','M32-no-spec-slash-cut','M39-no-config-types-cut','M59-no-module-versions'] else 1
 assert text[file].count(old)==expected,(label,text[file].count(old),expected)
assert text['js_paths.rs'].count('if self.snapshot.kind(&q).is_some() {')==1
results=[];baseline=None
for label,mutation in [('reference',None),*mutants.items()]:
 d=evidence/'mutants'/label;d.mkdir(parents=True,exist_ok=True)
 for f,s in text.items():
  if mutation and f==mutation[0]:
   old,new=mutation[1:];s=s.replace(old,new)
   if label=='M12-no-outdir-barrier':s=s.replace('if !out.is_empty() {','if false && !out.is_empty() {')
   if label=='M26-no-question-prefix-barrier':s=s.replace("['?', '[', ']', '{', '}', '\\\\', ':']", "['[', ']', '{', '}', '\\\\', ':']")
   if label=='M11-no-allowjs':s=s.replace('c.options.get("allowJs").and_then(Value::as_bool) != Some(true)','false')
  if label=='M17-byte-unicode-glob' and f=='js_paths.rs':s=s.replace('(literal || p.is_ascii())','true')
  if label=='M65-no-paths-first-pass' and f=='js_paths.rs':
   # P1's singleton-candidate gate independently covers the same path arm.
   s=s.replace('if self.snapshot.kind(&q).is_some() {','if (q.ends_with(".js") || q.ends_with(".jsx")) && self.snapshot.kind(&q).is_some() {')
  (d/f).write_text(s)
 shutil.copy2(packet/'resolver_driver.rs',d/'main.rs')
 cmd=['rustc','--edition=2021',str(d/'main.rs'),'-L',f'dependency={deps}']
 for lib in ['serde','serde_json','sha2','bincode']:
  paths=[Path(externs[lib])] if externs else sorted(deps.glob(f'lib{lib}-*.rlib'));assert paths and paths[0].is_file(),lib;cmd+=['--extern',f'{lib}={paths[0]}']
 cmd+=['-o',str(d/'driver')]
 p=subprocess.run(cmd,capture_output=True,text=True);(d/'compile.log').write_text(p.stdout+p.stderr)
 if p.returncode:raise RuntimeError(f'{label}: compile probe inadmissible; see compile.log')
 p=subprocess.run([str(d/'driver'),str(controls),str(requests)],capture_output=True,text=True);(d/'run.log').write_text(p.stdout+p.stderr)
 if p.returncode:raise RuntimeError(f'{label}: run inadmissible')
 output=json.loads(p.stdout)
 if label=='reference':baseline=output;continue
 changed=[{'request':a[:2],'base':a[2],'mutant':b[2]} for a,b in zip(baseline,output) if a!=b]
 if label in ('M29-no-tsx-ts-priority','M30-no-jsx-js-priority'):
  prefix='C85-' if label.startswith('M29-') else 'C86-'
  witnesses=[c for c in changed if c['request'][0].startswith(prefix)]
  assert witnesses and all(c['base'] is None and c['mutant'] is not None for c in witnesses), (label,witnesses)
 (d/'changes.json').write_text(json.dumps(changed,indent=2))
 results.append({'mutant':label,'killed':bool(changed),'changed_requests':len(changed),'first_control':changed[0]['request'][0].split('/')[0] if changed else None})
(evidence/'mutants-summary.json').write_text(json.dumps(results,indent=2))
print(json.dumps(results));assert all(r['killed'] for r in results),'survivor requires classification'

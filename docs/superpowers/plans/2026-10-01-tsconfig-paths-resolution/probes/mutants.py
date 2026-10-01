"""Finite isolated mutants of actual P1 resolver modules; no prototype source mutation."""
import json,subprocess,sys,shutil
from pathlib import Path
repo=Path(sys.argv[1]).resolve();evidence=Path(sys.argv[2]).resolve();deps=Path(sys.argv[3]).resolve();controls=Path(sys.argv[4]).resolve()
packet=Path(__file__).resolve().parent
modules=['js_paths.rs','js_paths_snapshot.rs','js_paths_syntax.rs']
text={p:(repo/'src'/p).read_text() for p in modules}
facts=[json.loads(l) for l in (evidence/'controls-import-facts.jsonl').read_text().splitlines()]
req=sorted({(f['file'],b['module_path']) for f in facts for b in f['bindings'] or [] if b['kind']=='MemberImport' and not b['module_path'].startswith('.')})
requests=evidence/'resolver-requests.json';requests.write_text(json.dumps(req))
mutants={
 'M01-no-exact':('js_paths.rs','if paths.contains_key(spec) {','if false {'),
 'M02-shortest-prefix':('js_paths.rs','Some((m, _, _)) if *m > n => {}','Some((m, _, _)) if *m < n => {}'),
 'M03-child-paths-origin':('js_paths.rs','c.base.as_deref().unwrap_or(&origin)','c.base.as_deref().unwrap_or(dir(file))'),
 'M04-ignore-include':('js_paths.rs','Some(true) => break Some(c),','Some(true) | Some(false) => break Some(c),'),
 'M05-ignore-exclude':('js_paths.rs','let exact = exclude_matches(p, file)?;','let exact = false;'),
 'M06-ignore-declaration-blocker':('js_paths.rs','for ext in [".ts", ".tsx", ".d.ts", ".js", ".jsx"] {','for ext in [".ts", ".tsx", ".js", ".jsx"] {'),
 'M07-ignore-package-boundary':('js_paths.rs','.contains_key(&format!("{p}/package.json"))','.contains_key(&format!("{p}/__mutant_absent.json"))'),
 'M08-accept-tied-pattern':('js_paths.rs','if tied || s.is_empty() {','if s.is_empty() {'),
 'M09-ignore-baseurl-origin':('js_paths.rs','c.base.as_deref().unwrap_or(&origin)','&origin'),
 'M10-unmatched-target-star':('js_paths.rs',"if !key.contains('*') && raw_target.contains('*') {", "if false && !key.contains('*') && raw_target.contains('*') {"),
 'M11-no-allowjs':('js_paths.rs','Some("js" | "jsx" | "mjs" | "cjs")','Some("__never")'),
 'M12-no-outdir-barrier':('js_paths.rs','if !out.is_empty() {','if false && !out.is_empty() {'),
 'M13-exclude-fullpath-only':('js_paths_syntax.rs','prefix = parent;','return Some(false);'),
 'M14-no-same-stem-barrier':('js_paths.rs','["js", "jsx", "mjs", "cjs"].contains(&ext)','["__never"].contains(&ext)'),
 'M15-include-package-folders':('js_paths_syntax.rs','if !exclude\n        && file', 'if false && !exclude\n        && file'),
 'M16-no-declaration-priority':('js_paths.rs','file.strip_suffix(".d.ts")','file.strip_suffix(".__never")'),
 'M17-byte-unicode-glob':('js_paths_syntax.rs','if !pattern.is_ascii()\n        || !file.is_ascii()','if false'),
 'M18-no-include-case-barrier':('js_paths.rs','if !exact && pattern_matches(', 'if false && !exact && pattern_matches('),
 'M19-no-exclude-case-barrier':('js_paths.rs','if !exact && exclude_matches(', 'if false && !exact && exclude_matches('),
 'M20-ignore-case-config':('js_paths_snapshot.rs','let config_name = name.to_ascii_lowercase();','let config_name = name.to_owned();'),
}
results=[];baseline=None
for label,mutation in [('reference',None),*mutants.items()]:
 d=evidence/'mutants'/label;d.mkdir(parents=True,exist_ok=True)
 for f,s in text.items():
  if mutation and f==mutation[0]:
   old,new=mutation[1:];assert s.count(old)==1,(label,s.count(old));s=s.replace(old,new)
   if label=='M04-ignore-include':s=s.replace('Some(false) => {}','')
  (d/f).write_text(s)
 shutil.copy2(packet/'resolver_driver.rs',d/'main.rs')
 cmd=['rustc','--edition=2021',str(d/'main.rs'),'-L',f'dependency={deps}']
 for lib in ['serde','serde_json','sha2','bincode']:
  paths=sorted(deps.glob(f'lib{lib}-*.rlib'));assert paths,lib;cmd+=['--extern',f'{lib}={paths[0]}']
 cmd+=['-o',str(d/'driver')]
 p=subprocess.run(cmd,capture_output=True,text=True);(d/'compile.log').write_text(p.stdout+p.stderr)
 if p.returncode:raise RuntimeError(f'{label}: compile probe inadmissible; see compile.log')
 p=subprocess.run([str(d/'driver'),str(controls),str(requests)],capture_output=True,text=True);(d/'run.log').write_text(p.stdout+p.stderr)
 if p.returncode:raise RuntimeError(f'{label}: run inadmissible')
 output=json.loads(p.stdout)
 if label=='reference':baseline=output;continue
 changed=[{'request':a[:2],'base':a[2],'mutant':b[2]} for a,b in zip(baseline,output) if a!=b]
 (d/'changes.json').write_text(json.dumps(changed,indent=2))
 results.append({'mutant':label,'killed':bool(changed),'changed_requests':len(changed),'first_control':changed[0]['request'][0].split('/')[0] if changed else None})
(evidence/'mutants-summary.json').write_text(json.dumps(results,indent=2))
print(json.dumps(results));assert all(r['killed'] for r in results),'survivor requires classification'

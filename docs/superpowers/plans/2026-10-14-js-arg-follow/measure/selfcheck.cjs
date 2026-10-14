#!/usr/bin/env node
'use strict';
// Self-check of census.cjs against the authored truths of the matrix's `step: "arg"` rows.
//   node selfcheck.cjs <fixtures.jsonl> <prism-binary> <scratch-dir>
// Writes each synthetic fixture repo to scratch, runs prism's site dump and the census, and
// compares the census's per-site follow result with the truth. Synthetic fixtures only.
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const [fixturesFile, bin, scratch] = process.argv.slice(2);
const census = path.join(__dirname, 'census.cjs');
const rows = fs.readFileSync(fixturesFile, 'utf8').split('\n').filter(Boolean).map(JSON.parse).filter((f) => f.step === 'arg');
const out = { agree: 0, disagree: [], nosite: [] };
for (const f of rows) {
  const dir = path.join(scratch, f.id);
  fs.rmSync(dir, { recursive: true, force: true });
  for (const [p, src] of Object.entries(f.files)) { fs.mkdirSync(path.dirname(path.join(dir, 'repo', p)), { recursive: true }); fs.writeFileSync(path.join(dir, 'repo', p), src); }
  const d = spawnSync(bin, ['nav', '--no-cache', 'call-stats', '--repo', path.join(dir, 'repo'), '--dump-sites'], { encoding: 'utf8' });
  fs.writeFileSync(path.join(dir, 'sites.jsonl'), d.stdout);
  spawnSync('node', [census, path.join(dir, 'repo'), path.join(dir, 'sites.jsonl'), path.join(dir, 'out')], { encoding: 'utf8' });
  const p = path.join(dir, 'out', 'param-sites.jsonl');
  const sites = fs.existsSync(p) ? fs.readFileSync(p, 'utf8').split('\n').filter(Boolean).map(JSON.parse) : [];
  const s = sites.find((x) => x.file === f.site_file && x.line === f.site_line);
  if (!s) { out.nosite.push(f.id); continue; }
  const names = s.targets.map((t) => t.k.replace(/@\d+$/, ''));
  const t = f.truth;
  const want = t.kind === 'none' ? [] : t.kind === 'anon' ? [`${t.file}:<anon>`] : t.targets.map((x) => `${x.file}:${x.name}`);
  const sameSet = names.length === want.length && want.every((w) => names.includes(w));
  let ok;
  if (t.kind === 'one' || t.kind === 'set' || t.kind === 'anon') ok = s.status === 'closed' && sameSet;
  else if (t.kind === 'none') ok = names.length === 0;
  else ok = s.status !== 'closed' && names.every((n) => want.includes(n));
  if (ok) out.agree++; else out.disagree.push({ id: f.id, truth: t.kind, want, got: names, status: s.status, open: s.open });
}
console.log(JSON.stringify(out, null, 1));

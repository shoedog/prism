// Two-binary check of slice L's promise over every matrix fixture and EVERY call-site record:
// each site either has a proven hit through the alias route on the head (one Source record with
// one Exact target, where the base differed), or its records are identical on both binaries.
// A base IndirectResolution record may be absent on the head only at a site (same file and line)
// where the head's Source record is such a hit.
//   node sweep_promise.cjs <variants-dir> <work-dir> <base-binary> <head-binary>
'use strict';
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const [variantsDir, workDir, baseBin, headBin] = process.argv.slice(2);
const { all } = require(path.join(variantsDir, 'variants.cjs'));
function dump(bin, repo, cache) {
  fs.rmSync(cache, { recursive: true, force: true });
  const r = spawnSync(bin, ['nav', '--cache-dir', cache, 'call-stats', '--repo', repo, '--dump-sites'], { encoding: 'utf8', maxBuffer: 1 << 26 });
  fs.rmSync(cache, { recursive: true, force: true });
  if (r.status !== 0) throw new Error(`${bin} exit ${r.status}`);
  const m = new Map();
  for (const line of r.stdout.split('\n')) {
    if (!line.startsWith('{')) continue;
    let o; try { o = JSON.parse(line); } catch { continue; }
    if (o.record_kind !== 'call_site' || !o.source_span) continue;
    const s = o.source_span;
    const key = `${s.file}|${s.start_byte}|${s.end_byte}|${o.callee_text}|${o.origin}`;
    const tg = (o.resolved_targets || []).map((t) => `${t.confidence}:${t.kind}:${t.function_id.file}:${t.function_id.name}:${t.function_id.start_line}-${t.function_id.end_line}`).sort();
    const st = JSON.stringify({ tg, drop: o.drop || null });
    if (!m.has(key)) m.set(key, { states: [], file: s.file, line: s.line, origin: o.origin, tg });
    m.get(key).states.push(st);
  }
  return m;
}
const tally = { fixtures: 0, sites: 0, identical: 0, proven_hit: 0, textual_stood_down_at_hit: 0, violations: 0 };
const violations = [];
for (const v of all()) {
  const dir = path.join(workDir, v.id);
  fs.rmSync(dir, { recursive: true, force: true });
  for (const [f, src] of Object.entries(v.files)) { const p = path.join(dir, 'repo', f); fs.mkdirSync(path.dirname(p), { recursive: true }); fs.writeFileSync(p, src); }
  const b = dump(baseBin, path.join(dir, 'repo'), path.join(dir, 'cb'));
  const h = dump(headBin, path.join(dir, 'repo'), path.join(dir, 'ch'));
  fs.rmSync(dir, { recursive: true, force: true });
  tally.fixtures += 1;
  const hits = new Set(); // "file|line" of head Source records that are a proven hit differing from base
  const keys = new Set([...b.keys(), ...h.keys()]);
  const pending = [];
  for (const k of keys) {
    tally.sites += 1;
    const bs = b.get(k); const hs = h.get(k);
    if (bs && hs && JSON.stringify(bs.states.slice().sort()) === JSON.stringify(hs.states.slice().sort())) { tally.identical += 1; continue; }
    if (hs && hs.origin === 'Source' && hs.states.length === 1 && hs.tg.length === 1 && hs.tg[0].startsWith('exact:')) { tally.proven_hit += 1; hits.add(`${hs.file}|${hs.line}`); continue; }
    pending.push([k, bs, hs]);
  }
  for (const [k, bs, hs] of pending) {
    if (bs && !hs && bs.origin === 'IndirectResolution' && hits.has(`${bs.file}|${bs.line}`)) { tally.textual_stood_down_at_hit += 1; continue; }
    tally.violations += 1;
    violations.push({ fixture: v.id, key: k, base: bs ? bs.states : null, head: hs ? hs.states : null });
  }
}
console.log(JSON.stringify(tally));
for (const x of violations.slice(0, 40)) console.log('VIOLATION', JSON.stringify(x));
process.exitCode = violations.length ? 1 : 0;

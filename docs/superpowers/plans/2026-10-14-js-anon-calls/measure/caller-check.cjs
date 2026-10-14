#!/usr/bin/env node
'use strict';
// Lane A2 step 0: check the authored `caller` truth of the anon matrix rows against a prism binary.
//   node caller-check.cjs <prism-binary> <fixtures.jsonl> <out.jsonl>
// Materializes each fixture that carries `caller` into a temp dir, runs
// `nav --no-cache call-stats --dump-sites`, keeps the call_site records on the SITE line and
// reports their callers next to the authored truth. Synthetic fixtures only.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');
const [bin, fixturesFile, outFile] = process.argv.slice(2);
const rows = [];
const tally = {};
for (const line of fs.readFileSync(fixturesFile, 'utf8').split('\n').filter(Boolean)) {
  const f = JSON.parse(line);
  if (!f.caller) continue;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'anon-'));
  try {
    for (const [rel, src] of Object.entries(f.files)) {
      fs.mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true });
      fs.writeFileSync(path.join(dir, rel), src);
    }
    const outp = execFileSync(bin, ['nav', '--no-cache', 'call-stats', '--repo', dir, '--dump-sites'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
    const recs = outp.split('\n').filter(Boolean).map(JSON.parse)
      .filter((r) => r.record_kind === 'call_site' && r.source_span.file === f.site_file && r.source_span.line === f.site_line);
    const callers = [...new Set(recs.map((r) => r.caller.name))].sort();
    const want = f.caller.kind === 'named' ? f.caller.name : `<cb@${f.caller.line}:${f.caller.col}>`;
    const verdict = !recs.length ? 'NO-RECORD' : callers.length === 1 && callers[0] === want ? 'CALLER-OK'
      : callers.includes(want) ? 'CALLER-OK+EXTRA' : 'CALLER-WRONG';
    tally[verdict] = (tally[verdict] || 0) + 1;
    rows.push({ id: f.id, want, callers, verdict });
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}
fs.writeFileSync(outFile, rows.map((r) => JSON.stringify(r)).join('\n') + '\n');
console.log(JSON.stringify(tally));

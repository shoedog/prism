// node measure-indirect-default.mjs <base.jsonl> <candidate.jsonl> <archived-repo>
// Paired call-site evidence; deliberately not an oracle or corpus-wide census.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
const [basePath, candidatePath, root] = process.argv.slice(2);
const read = (p) => readFileSync(p, "utf8").trim().split("\n").map(JSON.parse)
  .filter((r) => r.record_kind === "call_site");
const base = read(basePath), candidate = read(candidatePath);
const key = (r) => JSON.stringify([r.caller, r.source_span, r.call_kind, r.callee_text]);
const old = new Map(base.map((r) => [key(r), r]));
if (old.size !== base.length || new Set(candidate.map(key)).size !== candidate.length)
  throw new Error("duplicate site identity");
if (base.length !== candidate.length || candidate.some((r) => !old.has(key(r))))
  throw new Error("site universe changed");
const exact = (r) => r.resolved_targets.filter((t) => t.confidence === "exact");
const changed = candidate.filter((r) => JSON.stringify(r) !== JSON.stringify(old.get(key(r))));
const sourceFiles = [...new Set(candidate.map((r) => r.source_span.file))].sort();
const sources = new Map(sourceFiles.map((p) => [p, readFileSync(join(root, p))]));
const text = (r) => sources.get(r.source_span.file)
  .subarray(r.source_span.start_byte, r.source_span.end_byte).toString("utf8");
const relevant = candidate.filter((r) => {
  const match = /^(?:this\.)?library\s*(?:\?\.|\.)\s*(destroy|resetLibrary|getLatestLibrary|updateLibrary|setLibrary)\s*\(/.exec(text(r));
  return match && r.callee_text === match[1]; // exclude outer .catch() calls
});
console.log(JSON.stringify({
  sites: candidate.length,
  exact_sites_before: base.filter((r) => exact(r).length).length,
  exact_sites_after: candidate.filter((r) => exact(r).length).length,
  changed_sites: changed,
  call_bearing_source_sha256: Object.fromEntries([...sources].map(([p, s]) => [p, createHash("sha256").update(s).digest("hex")])),
  relevant_unique_source_spans: new Set(relevant.map((r) => JSON.stringify(r.source_span))).size,
  relevant_sites: relevant.map((r) => ({ file: r.source_span.file, line: r.source_span.line,
    caller: r.caller.name, expression: text(r), before: old.get(key(r)).resolved_targets,
    after: r.resolved_targets })),
}, null, 2));

# Bounded local callable interfaces — receiver evidence

Status: verified within the stated bounds; publication pending. Base PR252 merge
04bb5583897dcd839e2d8905e43bea78368b8bc2; branch feat/local-callable-interfaces.
Spec: docs/superpowers/specs/2026-09-05-local-callable-interfaces.md.

## Scope and corrected assumptions

One module-private interface, one call signature, no heritage. Non-generic direct
or local Props-alias parameter types; optional one plain binder used as the entire
parameter type, with one concrete direct/Props argument. Implementation is a direct
variable-initializer arrow/function expression with the existing required
destructuring, explicit-annotation, shadow, duplicate and write barriers.

WRONG, corrected during round2: the first implementation treated top-level script
interfaces as local proof. With a local Client.m and interface F plus another script's
F overload, it minted Exact Client.m despite unproven merged callable authority.
The complete module-boundary collector observed four failures (TS/TSX × full/subset)
and passing explicit-module controls before the fix. Require an import/export marker.

SMELL, conservatively excluded: exported interfaces may be externally augmented;
the file-local identity lookup does not prove absence of that augmentation. Reject
direct, named/type-list/aliased and default interface exports. The six exclusion
probes failed before this guard. This is a narrowed proof boundary, not a claim that
every exported interface's inferred edge would be wrong. Exported consumers may use
private interfaces; a same-spelled re-export from another module does not export the
private declaration. No prior-main regression attribution: the discovered WRONG was
in this turn's proposed implementation. Existing main lacked interface authority.

Props interfaces, imported/ambient/qualified callable types, inheritance, merging,
overloads, extra optional members, generic signatures, binder constraints/defaults/
variance, alias chains, inference, React.FC and hooks remain outside this slice.

## Architecture and refutation

src/ast.rs factors a kind-bounded local declaration lookup. Props use alias-only
lookup. Callable lookup accepts aliases or a private, heritage-free interface;
body normalization preserves the original single call_signature node. Non-generic
property type scope remains at the declaration; generic type scope remains at the
original argument; write timing remains at the implementation parameter end.
No resolver fallback or class-owner policy change. CPG72→73/nav41→42.

Round1: base release saved before production edits. RED32 newly supported cases
missed Exact; two explicit controls and both negative groups passed. Same-environment
existing callable-alias positive control passed, ruling out a broken class-owner
route. First GREEN3/3. Round2: module/export corrections above; final receiver
groups22/22, 36 positive TS/TSX cases (including two explicit controls), 40 member
barriers, 51 authority/receiver barriers and the paired script/module fixture.
Collectors enumerate failures instead of silently stopping at the first case.

Persisted tests cover five new good↔bad declaration transitions, four interface
owner-change modes in both TS/TSX and both A→B/B→A directions (16 new transitions),
full call-site equality after reload/incremental rebuild, and four sidecar fixture
rows. Prior CPG72/nav41 are rejected. Focused cache groups:2/0 plus owner groups:2/0.

Round3 SELF-PASS (NOT INDEPENDENT), cap3: source trace through the sole contextual
consumer, alias-only props, implementation write anchor, TypedParam class-owner
resolution, changed-file removal/re-extraction, and cache projections. No further
in-scope WRONG found; no cap extension. The prism-nav indexed helper was absent;
SymbolNotFound was not treated as no callers. Current source supplied the fallback.

## Verification

Full default:3837 passed/0 failed/1 ignored; MCP4027/0/1. Clippy all targets/MCP
completed with warnings retained (raw clippy.log); not a warning-free gate claim.
Existing ignored control: resolution_test::slice_elem_variant_reserved.
Immediate release rebuild then Tier-A matrix:104/104. Immediate second rebuild then
quick exit2: baseline-invalid solely SHA drift04bb5583897d versus pinned20c8490591a3.
Oracle quiescent, oracle/SUT error rates0, stale adjudications4. Not a green quick
gate. Fmt/diff checks and verifier syntax check pass. No full multicorpus or baseline
rewrite. No same-environment base quick rerun; sampled differences below are not
attributed regressions or fixes.

Pins: target-c-method flip_candidate, supplementary Exact5/0/0 plus28 default
candidates. module-deps-feature-gated missing literal pin: oracle-only[], Prism-only
src/mcp/tools.rs:230. load-repo-feature-gated missing literal pin: oracle-only
tests/integration/resolution_test.rs:5299/5368; Prism-only src/mcp/freshness.rs:401,
src/mcp/session.rs:359/374/400. ambiguous-symbol-contract ok. Full site lists in archive.

Sampled Exact tp/fp/fn, C-method/C-name/Q-scoped/U-free/U-method order:
callers4/0/4,36/0/0,0/0/1,2/0/0,7/0/0;
callees4/0/2,8/0/1,2/0/2,1/1/0,11/0/0. Preserve these gaps and the flip candidate
for adjudication, not rebaselining.

## Paired measurement

Saved base/candidate release binaries, `nav --no-cache call-stats --dump-sites`,
same fixed five-file Excalidraw source archive at0642e72cfa2d9a71198200e52f37399384610ee3.
2780 records, Exact376→376, changed records0; 11 unique relevant receiver spans
unchanged. No gain promised for the six remaining React.FC/useContext spans.
Prior Black, small Excalidraw and JavaScript control JSONL outputs byte-identical.

Separate served fixture: Exact callers0→4 (plain, named Props, generic direct,
generic Props); each callee query0→1 client.ts:2:m, TypedParam, no fallback,
warnings or truncation. Eight excluded callers: overloaded, extra, inherited,
merged, optional, written, explicitUnknown, inner. Same-named decoy present.
This is synthetic capability evidence, not real-site recall improvement or an oracle.

Replay: `node docs/eval/receiver-closure/verify-callable-interfaces.mjs /private/tmp/prism-callable-interfaces-mLbaAN`.
Committed companion measurement records fixture/call-bearing source hashes and
control JSONL hashes. Raw source, paired JSONL, served JSON and logs remain in
that evidence directory. Archive: /private/tmp/prism-callable-interfaces-mLbaAN-evidence.tgz;
SHA25628d85df81e4f3fbbd167b03b907bcb760c2572d8ec8ceb47df0e0102796db099.
Includes saved binaries, raw quick JSON/Markdown/snapshot, logs, fixtures and fixed
source. Generated quick artifacts moved recoverably there; no tracked baseline
changed. Checkpoint documents inside the archive are historical; this committed
readout/handoff is the final operational record.

## Custody and next boundary

PR252 merge records reconciled in this branch; original .superpowers/ and
eval/snapshots/prism-fb81481dafa7.json untouched. No memory edit, merge or rebaseline.
Recommendation only: reuse private declaration proof for a separately specified
non-generic Props-interface slice, retaining own-property/duplicate/write barriers.
Export/import/augmentation evidence must precede lifting the private restriction;
neither next step grants React.FC by spelling.

# S1b-4 planning evidence — 2026-10-01

MEASURED: branch `plan/s1b-4`, HEAD/source base
`915fca43d84ea1730959453091fbf8ae97763af8`; initially clean. Main's local branch
name is absent, so the source base is bound by SHA, not a local `main` ref.
No prototype branch/commit exists and no plan commit has been created: READ
environment `.git` is read-only, escalation disabled, controller answer pending.
All current tracked edits are packet documentation/probes; production source,
tests, eval fixtures, Cargo files and old expected JSON files are unchanged.

MEASURED: base built with `cargo build --offline --release`, copied to
`target/plan-s1b4/base/prism`. Version is `slicing 3.1.2 (915fca43d84e)`; SHA256
is `d3fc31233253ddfec36f9f623d780c1cc6d9e376f31806ff516965a141974859`.
Logs: `target/plan-s1b4/base/{build,rebuild-tier-a}.log`.

| MEASURED baseline | Result | Evidence under target/plan-s1b4/ |
|---|---|---|
| cargo test --offline --no-fail-fast | 4,750 passed / 0 failed / 1 ignored; 29 result groups | base/test.log |
| cargo test --offline --features mcp --no-fail-fast | 4,943 / 0 / 1; 31 result groups | base/test-mcp.log |
| cargo fmt --all -- --check | passes; empty output | base/fmt.log |
| cargo clippy --offline --all-targets --features mcp -- -W clippy::all | finishes; inherited warnings (181 for lib test, 136 duplicates); not clean | base/clippy.log |
| Tier-A matrix with base release binary | 165 ok / 0 regression | base/tier-a-matrix.log |
| original controls | 287 scenarios, no stderr | base/controls/{SUMMARY.txt,*.dump.jsonl}; base/controls.log |
| extended controls | 349 scenarios, no stderr | base/controls-r2/{SUMMARY.txt,*.dump.jsonl}; base/controls-r2.log |
| original source/summary preservation | 287 original scenarios / 0 changed generated files / 0 changed complete base summary sections | controls/ vs controls-r2/; base summaries; results.json |
| RP replay | 46 scenarios, no stderr; RP2-c still UnknownName on base | base/replay/SUMMARY.txt; base/replay.log |

MEASURED: Tier-A's first system-python probe failed before evaluation because
Python 3.9 lacks tomllib; inadmissible. `uv python find --offline` was refused
while initializing its default cache; inadmissible, no further uv attempted.
The repaired run used already-installed Python 3.12 directly, after a release
rebuild in the same worktree:

```bash
cargo build --offline --release
PYTHONPATH=eval /Users/wesleyjinks/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/bin/python3 -m tier_a.cli --matrix-only --allow-stale-sut
```

| Corpus | MEASURED base sites | MEASURED base R3 rows / edges | Head changes / classes |
|---|---:|---:|---|
| X | 19,219 | 15 / 89 | **not measured** |
| R | 953 | 0 / 0 | **not measured** |
| T | 61,712 | 26 / 27 | **not measured** |
| F | controller-only | not read/run | **not measured** |

MEASURED: `run_dumps.sh BASE OUT X R T` reports rc=0 and stderr=0 for each
corpus; complete dumps are `base/dumps/{X,R,T}-dump-sites.jsonl`. READ / hand
check: T's 24 namespace rows are compiler performance (18) and service classifier
(6) imports ending in .js with indexed .ts siblings. Its 2 additional R3 rows
are named IO imports in harness/tsserverLogger.ts:151,153 and are non-goals.
This rebinds the old T exposure to the landed code without assuming the head
preserves it. X's import-qualified rows include named-object imports and must
not all be treated as namespace rows merely from the resolution-kind label.

READ: old expected S1b-4-{X,R,T}.json are empty; SPEC §8 forecast was X/R/T 0,
F 4 Exact->NameOnly with the same edges. Current head and r2 expected files are
pending; no zero-right-loss claim is made. The C220 counterexample is why a
lexical classifier is insufficient for removals.

MEASURED C220 independent flow check: `make()` returns unique nested f@2 and
exported f receives make(). Base R3 retains that target in both grammars. Node
executed the unchanged lib sources renamed .mjs, verifying the exported
callable's exact function source and result: `valueflow-c220/check.mjs` and
`valueflow-c220/output.jsonl`. READ: D4 maps Alias to UnprovenLocal; literal
reuse of its table would remove this edge. This is a design forecast, not a
prototype measurement. OQ-S1b4-1 is open.

MEASURED auditor checks: a complete X base self-comparison reports 0 lost IDs;
this checks the inventory tool, not a head. A **simulated**, explicitly named
C220 removal reports `lost_target_count:1` and `removed_needs_valueflow:1`, never
removed_wrong (`valueflow-c220/*simulation*`). This simulates audit input only;
it is not a synthesized head or acceptance evidence. Every actual removed
identity must receive an independent value-flow hand audit before acceptance.

READ (still required): controller custody/prototype authorization, owner alias
policy, prototype source+commit/build, head X/R/T dumps, fresh r2 expected rows,
complete changed-row classifications/flow audit, proto controls reference,
RP2-c green, mutant kills, prototype full/mcp suites, new Tier-A fixture RED/GREEN,
Tier-A quick and Node gate, and F controller aggregates. No review round has
been dispatched. `OQ-S1b4.md` and `HANDOFF-s1b4.md` carry the resume gates.

# S1b-4 measured prototype — 2026-10-01, round 0 continuation

MEASURED: the full SPEC §3.4 design, including resolved Alias opacity, is built.
No worker Git writes, F reads, provider use, installs or network acquisition
were performed. No plan review has been dispatched; review cap remains two.
READ: OQ-S1b4-1 applies existing OQ12/Option K rulings; no owner question remains.

## Custody and binaries

MEASURED: planning tree is plan/s1b-4 at
`bd93e7e801b9b5a06801acc11a27cf9bbb77f88d` (draft predecessor 53eb3155), dirty
packet only. The prototype started at source base
`915fca43d84ea1730959453091fbf8ae97763af8` and is now proto/s1b-4 at the
controller's observed WIP commit `39faa3aac79c71ca4b41423caacf9cead50da86d`.
All 17 owned source/test/fixture files are byte-identical to that commit.
No additional prototype source edit is waiting to commit. The controller WIP
also captured a 109,118-line generated inventory and two quick reports; those
are evidence artifacts, excluded from implementation LOC and the owned list.
Later quick outputs were copied to evidence and tracked WIP report bytes restored
without Git writes. No WIP artifact is silently treated as a valid quick baseline.

MEASURED: public corpus/control measurements preserve the binary built before
that external commit, from the same final owned source bytes, with dirty build
identity. Clean rebuilt binary is separately retained for quick/warm-sidecar
checks. No commit-only identity change is falsely reported as source change.
`target/plan-s1b4/BUILD-MANIFEST.json` records all owned file hashes, the tracked
crate-input hash map, source patch hash and actual identities:

| Artifact | Version identity | SHA256 |
|---|---|---|
| `target/plan-s1b4/base/prism` | 915fca43d84e | d3fc31233253ddfec36f9f623d780c1cc6d9e376f31806ff516965a141974859 |
| `target/plan-s1b4/head/prism` (public/controls) | 915fca43d84e-dirty | 8fb02039dc4fbf64cbc160905ab16270c7be99da90eee269ebbb3f31abe90867 |
| `target/plan-s1b4/head/prism-clean` (quick/cache) | 39faa3aac79c | 146b21178a59daf35f8008c9dd0a9e5f8189226117e1d6a8290836a2b851b318 |

MEASURED: owned diff against 915fca43 has SHA256
`001f532de6a5299449f654c1b0e530d7bea4a1617b5643ca77f163cd8b287cab`.
Crate-input hash-map digest is
`00ac5a3d8914c965c2c1996c5a10d1b547b32187869068ca5a94fdd57507d64c`.
Snapshots/manifest and exact separate commit lists are under target/plan-s1b4
and in `COMMIT-FILES-s1b4.md`. Controller copies ignored evidence to durable
storage before cleanup. The source base remains 915fca43 regardless of WIP HEAD.

## Public row populations and classifications

MEASURED: complete fresh head dumps versus complete same-environment base dumps,
all stderr zero. No intersection-only/empty-dump shortcut is admissible.

| Corpus | Base sites | Head sites | Changed rows | Removed / retargeted / added / demoted / relabeled / accepted-cost | Lost target IDs | Key differences |
|---|---:|---:|---:|---|---:|---:|
| X | 19,219 | 19,219 | 0 | 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 |
| R | 953 | 953 | 0 | 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 |
| T | 61,712 | 61,712 | 0 | 0 / 0 / 0 / 0 / 0 / 0 | 0 | 0 |

MEASURED: expected files `probes/expected/S1b-4-r2-{X,R,T}.json` are fresh `[]`.
Old expected files stay unchanged. Evidence is `base/dumps/*`,
`head/dumps-final/*`, and `head/{X,R,T}-{rowdiff,audit,valueflow}-final.*` below
`target/plan-s1b4`. The old SPEC §8 0 / four-F-demotions / 0 / 0 row is historical:
public zero counts agree; F cannot be reconciled without controller aggregates.

READ: audit_s1b4.py annotates syntax and certifies identity-preserving demotions,
but cannot follow Alias initializers, function/hook returns, parameter suppliers,
defaults/destructuring, callable object members, merged namespaces or runtime
writes. It never calls a removed target wrong from a non-callable qualifier.
MEASURED: valueflow_guard_s1b4.py compares every site's target multiset and whole
key population independently of those lexical assumptions. Zero missing target
identities proves **zero public right target identities lost**, without deciding
whether retained baseline targets were right. There are zero uncertified changed
public rows to hand-audit. This is not a claim about private F or all future inputs.

MEASURED commands (P packet absolute path; E evidence root; B/H manifest binaries):

```bash
bash "$P/probes/run_dumps.sh" "$B" "$E/base/dumps" X R T
bash "$P/probes/run_dumps.sh" "$H" "$E/head/dumps-final" X R T
python3 "$P/probes/rowdiff.py" "$BASE_DUMP" "$HEAD_DUMP" "$EXPECTED"
python3 "$P/probes/audit_s1b4.py" "$CORPUS_ROOT" "$EXPECTED" "$AUDIT_JSON"
python3 "$P/probes/valueflow_guard_s1b4.py" "$BASE_DUMP" "$HEAD_DUMP" "$VALUEFLOW_JSON"
```

READ: wait for all dump writers, require successful receipts/nonempty valid
JSONL/empty stderr/equal unique keys before acceptance. run_dumps.sh prints each
status but masks failures as a shell return value; inspect its receipts/artifacts.
The F script uses direct commands with preserved failure status instead.

## Synthetic controls, independent flow and export telemetry

MEASURED: all original 287 generated sources are byte-identical to the earlier
base population. Full summaries, function inventories and dumped key sets were
compared with compare_controls_s1b4.py, not just controls_diff.py.

| Population | Total | Identical sections | Changed sections/rows |
|---|---:|---:|---:|
| Original | 287 | 269 | 18 |
| New C190–C220, jsx/tsx | 62 | 34 | 28 |
| Combined | 349 | 303 | 46 |

MEASURED: every changed row is hand-audited one by one in `S1b-4-CONTROLS.md`;
all new rows reach their registered correct column, including preservation rows.
All function inventories agree, keys agree and stderr is empty. Final comparison
matches the earlier v2 comparison exactly. References:
`probes/S1b-controls-s1b4-r2-proto.txt`,
`head/controls-comparison-final.{json,log}`, `head/controls-hand-audit.json`.
For dynamic `with` receivers the class describes wrong **Exact namespace
authority**, not proof that every runtime target is impossible: an object supplying
its own ns/Lib.f is a concrete counterexample to that authority. Neither this
lexical classifier nor the control table supports a zero-synthetic-loss claim.
Constructors, subscripts, nested receivers and existing non-namespace routes
are preservation guards, not newly granted callable authority.

MEASURED: RP replay has 46 complete sections. RP2-c js/jsx/ts/tsx changes
UnknownName→Exact/import_qualified, targeting lib:f@1-1 through exported member
g. Other 42 sections remain byte-identical. Reference:
`probes/S1b-replay-s1b4-r2-proto.txt`; final raw evidence `head/replay-final/`.

MEASURED: C220's independently executed Node check establishes the returned
callable's **source identity**, f@2, not merely its return value. Evidence:
`valueflow-c220/{check.mjs,output.jsonl}`. C220 rows remain identical. d10 also
checks direct/list Alias, named/star barrels with a base-row decoy, eligible
ImportForward and a D4 named-import consumer that remains unresolved. READ:
ImportForward's original whole-file competitor proof is unchanged; an imported
local named f with a nested f competitor is not eligible. A distinct imported
local alias tests admitted forwarding without relaxing that contract.

MEASURED: every js_export_* counter is identical on **351 repositories**: all
349 controls plus X/R. `head/export-counters-final/comparison.json` retains the
complete counter maps and successful outputs. The base T aggregate stats call
hit its 180-second bound, so T aggregate telemetry comparison is **excluded**,
not green or attributed to head. Complete T site dumps/comparison did finish.
READ: original D4 traversal/facts remain the telemetry owner; the namespace clone
projection's counters are discarded. Qualified MayCall/Position counts are the
intentional new accounting in existing maps, once per site.

## Mutant results

MEASURED: one mutation per run, bounded 180 seconds each, no compile/setup/zero-test
kills admitted. All originals restored byte-for-byte before final verification.
Both the initial and final rerun kill all 18 variants. Final patches, command,
assertion log and results are `head/mutants-final/`; driver is
`probes/mutate_s1b4.py`. No review round was dispatched by this exercise.

| §7 mutant | Variants | Actual killing test(s) | Result |
|---|---|---|---|
| D-M1 legacy stem instead of namespace outcome | 1 | d1_direct_and_directory_decoys | KILLED |
| D-M2 bypass scope/declaration identity proof | 1 | d4_scope_write_recovery_and_positions | KILLED |
| D-M3 fallback singleton Exact | 1 | d5_authoritative_missing_member_and_fallback | KILLED |
| D-M4 lossy flat non-namespace import authority | 1 | d6_non_namespace_imports_keep_base | KILLED |
| D-M5 ignore writes / refuse written kinds | 2 | d9_written_import_and_export_keep_base | KILLED ×2 |
| D-M6 omit terminal span / wrapper gate | 2 | d1 / d2_wrapped_call_and_jsx | KILLED ×2 |
| D-M7 functions.get(member) before export rename | 1 | d3_rename_named_and_star_barrels | KILLED |
| D-M8 inner write globally poisons outer import | 1 | d4 | KILLED |
| D-M9 remove recovery sealing/refusal | 1 | d4 | KILLED |
| D-M10 legacy shadow veto at proven position | 1 | d4 | KILLED |
| D-M11 authoritative missing member falls to decoy | 1 | d5 | KILLED |
| D-M12 drop named / star opacity propagation | 2 | d10_alias_opacity_direct_named_star_forwarded_and_d4_pin | KILLED ×2 |
| D-M13 grant namespace opacity at D4 | 1 | d10 actual named-import consumer | KILLED |
| D-M14 reuse site/epoch / revert cache pins | 2 | d8_serde_cache_and_incremental_epochs; both version-pin tests | KILLED ×2 |

READ: the cache-pin kill proves the declared versions are guarded; it does not
uniquely prove rejection of every serialized parent cache from version alone.
Actual parent→head caches also differ in binary identity and serialized layout.

## Suites, harnesses and caches

MEASURED final commands ran in the proto worktree against the unmutated body:

| Check | Pass | Fail | Ignored/skip | Evidence under target/plan-s1b4 |
|---|---:|---:|---:|---|
| Full default cargo test --offline, 29 groups | 4,761 | 0 | 1 | head/tests-default-final.log |
| Full cargo test --offline --features mcp, 31 groups | 4,954 | 0 | 1 | head/tests-mcp-final.log |
| Focused namespace matrix | 11 | 0 | 0 | full integration log + namespace-test.log |
| Tier-A matrix | 166 | 0 | 0 | head/tier-a-matrix-final.log |
| Node gate | 853 | 0 | 1 | head/node-gate-final/{receipt.json,log.txt} |

MEASURED: same-machine base suites were 4,750/0/1 and 4,943/0/1; head adds
11 matrix tests. New Tier-A fixture fails behaviorally on the base binary in
the same harness (`base/tier-a-new-fixture.log`: 165 ok + 1 regression) and passes
head; d1 has a separate positive recall/caller assertion. Node's pre-existing
skip is `RED adapter exposes compressed alias binding ordinal`; the gate's
pre-existing excluded file is `docs/eval/receiver-closure/audit-imported-props-source.test.mjs`
with reason `inputs-not-reconstructible`. No acquisition/network was run; existing
validated inputs were copied to `target/plan-s1b4/node-inputs`.

MEASURED: fmt and source/packet whitespace checks pass. Clippy completes with
inherited warnings: 181 lib-test warnings (136 duplicates) on both same-machine
base/head with identical mcp/all-targets flags. There are 123 unique warning-kind/
normalized-file pairs in each and zero new pairs; it is not warning-clean.
`head/clippy-comparison-final.json` records the comparison. Grammar closure:
186 named kinds, 76 suspect, zero unclassified; 74 E_TABLE positions/rows,
zero missing/extra/different (`head/grammar-closure-final.log`).

MEASURED: release build immediately preceded matrix and the corrected quick run.
Installed Python 3.12 invokes tier_a.cli.main directly because uv is blocked;
`python -m tier_a` lacks __main__ and was an inadmissible setup attempt.
The bounded quick wrapper finally completed in 223.68 seconds (300-second bound),
using only a writable nav cache and preserving the matrix's no-cache flag. Its
artifact has 166 ok matrix, SUT error rate 0, settled rust-analyzer, oracle error
rate .0667, but **baseline_invalid** because corpus 39faa3aac79c differs from
pinned 20c8490591a3 and C-method has only 4/6 successful probes. Thus quick is
**not GREEN as an acceptance baseline**. The 18 pending research rows were not
adjudicated or attributed to S1b-4; no same-environment full base-quick replay
was performed, so there is no claim that those rows are new or inherited.
No allow-drift, baseline change or oracle/query/grade override was used.
Evidence `head/quick-verified/{receipt.json,summary.json,*.artifact}`. Earlier
interrupted/bad-flag wrapper probes are inadmissible, retained in PROBE-LOG.

MEASURED: real parent caches carry CPG103/sidecar59; head rebuild carries104/60,
with identical head rebuilt/fresh site output. The dirty public binary explicitly
skips warm sidecar reuse, so a separate clean39faa3aa run exercises actual warm
reuse: subsequent head hit has empty stderr and equal site/caller bytes.
`head/cache-transition-clean/` retains parent/head metadata, outputs and receipt;
`head/cache-transition-final/` retains dirty-binary diagnostics. Cache version,
source layout and build identity all change; no version-only causal claim.
d8 additionally covers Bincode graph serde, CallSite defaults, real CPG disk
cold/full/partial hits, per-file incremental export/qualifier/reaching-write
changes and two positions in one file. No cache re-baselining.

## Size and remaining work

MEASURED: after rustfmt, nonblank/non-// diff lines against 915fca43:

| Bucket | Added | Removed | Net |
|---|---:|---:|---:|
| Production Rust | 335 | 18 | 317 |
| Tests and fixture | 442 | 6 | 436 |

MEASURED: tests include 407 new namespace matrix lines and 19 fixture lines;
`size-final.json` attributes every owned file. Generated control repositories,
planning scripts and controller WIP inventory/reports are excluded from code
LOC, explicitly. ASSUMPTION: landing forecast 335–400 added src and 442–550
added tests/fixtures, with shared fixtures rather than the earlier duplicated
1,000–1,400 test forecast. Reuse of the landed core and the finite matrix make
one slice reasonable for two review rounds; no numerical cap or split question.

READ remaining operational work: controller commits the dirty packet, retains
or finalizes its existing prototype WIP commit, copies ignored evidence, runs
private F and supplies aggregates/hand audit, then dispatches plan review round
1 of 2. Prototype work is complete; F and review are not fabricated approvals.
Full multi-corpus Tier-A is human-triggered and was not run. Private F was never
opened or executed. T aggregate telemetry remains excluded as described above.

READ exact controller F invocation (set both variables privately):

```bash
CORPUS_F_ROOT="$PRIVATE_CORPUS_ROOT" PRIVATE_F_EVIDENCE="$PRIVATE_OUTPUT_DIR" \
  bash /Users/wesleyjinks/code/prism-s1b-4-plan/docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/CONTROLLER-S1b4.sh
```

READ: the script checks binary manifest hashes, preserves command failures,
checks complete nonempty populations/stderr, runs rowdiff/lexical audit/full
identity guard/export counter comparison, and keeps private rows/source local.
Controller hand-audits every uncertified changed row and returns aggregate
classes/costs/completeness/hashes only. Script syntax was checked; F behavior was
not exercised. A telemetry timeout is an exclusion, never implicit acceptance.

## Controller F acceptance (private corpus, aggregates only; 2026-10-01)

`CONTROLLER-S1b4.sh` was run with base `915fca43` and the prototype head (binaries checked against the custody manifest). Results:

- Sites: 13,299 on base and on head, with no key present on only one side.
- Changed rows: **4**, all Exact → NameOnly, `import_qualified`, with the edge kept.
- Lost targets: 0.
- Export counters: 0 changed.

This matches the historical §8 forecast of 4 demotions with the edge kept. Evidence is retained privately.

# Sonnet dispatch — P1 tsconfig paths import members

READ: planning continues under the controller interim OQ1–OQ4 dispositions in OQ-paths.md, pending owner confirmation. P1 implementation dispatch still requires fresh F aggregates, Opus plan approval and controller commit/authority binding. Read SPEC, OQ-paths, MEASUREMENTS and BUILD-MANIFEST.md. Use the retained bound TypeScript package path as TS_JS. READ: Opus round 1 returned FIX (2 WRONG / 5 SMELL), all dispositions are folded; round 2 returned FIX (2 WRONG / 4 SMELL); this is the disclosed targeted fold at the cap, with S6 parked for the owner. Review cap is two rounds; numeric LOC caps do not exist. Do not restart the prototype. Implementer starts from the controller-applied **cumulative P1.diff** off bound base `e61d52b8`, not from a rewrite.

READ: previous reviewed artifact was `proto/tsconfig-paths-final @ bab21e62`; controller must fill `__PROTO_COMMIT__ = __CONTROLLER_FOLDED_PROTO_COMMIT__` after committing this residual. Owner answers and dispatch authority remain with the controller. Implementation parent is fixed to `e61d52b8`. If parent, scope or membership ruling differs, stop the dependent work and ask the controller; do not infer a ruling. Existing user authorization covers completing P1, tests and fixes within the answered scope, not P2, pushing or merging.

## P1 dispatch and starting artifact

READ: Sonnet starts from **proto/tsconfig-paths-final @ __CONTROLLER_FOLDED_PROTO_COMMIT__**, the controller-committed residual fold. Implementation parent: **e61d52b8**. Previous reviewed final was bab21e62. The planner writable prototype HEAD is 32a5e893, whose initial tree exactly equals bab21e62; the three incremental paths are in BUILD-MANIFEST. Preserve this body and never apply the cumulative patch to an integrated prototype.

MEASURED: cumulative **40-path** `target/paths-proto/P1.diff` SHA-256 **`92e5c1212915f12049fdf3d02e583e1df572a4c415d58201dc59cf23cbb8da95`** replays exactly on a Gitless `e61d52b8` archive. `target/paths-proto/source-hashes.json` and the archive bind the same owned files; `target/paths-plan/extension-priority/evidence/source-binding.json` also records each base hash or required absence. Earlier patches and receipts are historical. Use current BUILD-MANIFEST hashes.

READ: exact owned-file list (40 paths, relative to the prototype repository; also retained as the newline-delimited `P1-owned-files.txt`):

```text
eval/fixtures/javascript/tsconfig_paths_positive/app.jsx
eval/fixtures/javascript/tsconfig_paths_positive/decoy/util.jsx
eval/fixtures/javascript/tsconfig_paths_positive/expected.toml
eval/fixtures/javascript/tsconfig_paths_positive/src/util.jsx
eval/fixtures/javascript/tsconfig_paths_positive/tsconfig.json
eval/fixtures/javascript/tsconfig_paths_refusal/app.jsx
eval/fixtures/javascript/tsconfig_paths_refusal/decoy/util.jsx
eval/fixtures/javascript/tsconfig_paths_refusal/expected.toml
eval/fixtures/javascript/tsconfig_paths_refusal/src/util.d.ts
eval/fixtures/javascript/tsconfig_paths_refusal/src/util.jsx
eval/fixtures/javascript/tsconfig_paths_refusal/tsconfig.json
eval/fixtures/typescript/tsconfig_paths_positive/app.tsx
eval/fixtures/typescript/tsconfig_paths_positive/decoy/util.tsx
eval/fixtures/typescript/tsconfig_paths_positive/expected.toml
eval/fixtures/typescript/tsconfig_paths_positive/src/util.tsx
eval/fixtures/typescript/tsconfig_paths_positive/tsconfig.json
eval/fixtures/typescript/tsconfig_paths_refusal/app.tsx
eval/fixtures/typescript/tsconfig_paths_refusal/decoy/util.tsx
eval/fixtures/typescript/tsconfig_paths_refusal/expected.toml
eval/fixtures/typescript/tsconfig_paths_refusal/src/util.d.ts
eval/fixtures/typescript/tsconfig_paths_refusal/src/util.tsx
eval/fixtures/typescript/tsconfig_paths_refusal/tsconfig.json
src/ast.rs
src/call_graph.rs
src/cpg/build.rs
src/cpg_cache.rs
src/js_exports.rs
src/js_paths.rs
src/js_paths_snapshot.rs
src/js_paths_syntax.rs
src/lib.rs
src/navigation/call_edge_cache.rs
src/navigation/call_resolve.rs
src/repo_loader.rs
src/resolution.rs
tests/integration/fixtures/js_paths_r1.json
tests/integration/fixtures/js_paths_r2.json
tests/integration/fixtures/js_paths_priority.json
tests/integration/js_paths_test.rs
tests/integration/main.rs
```

ASSUMPTION: controller custody sequence: verify the final patch/archive/list hashes; check every parent file hash against `base_sha256` and absence for null base hashes; apply the cumulative patch in the new worktree; check every resulting owned file against `prototype_sha256`; commit only the listed paths and fill the dispatch placeholders. Stop if any path/hash/parent differs. Prototype commit recommendation: `fix(paths): apply TypeScript root-file extension priorities`. The planner performs none of these Git writes.

## Model and scope

READ: Exact is static binding. Option K keeps every unproven complete base row. No runtime-mutation guard, package/workspace resolver, namespace/class change, new corpus, speculative NameOnly-to-Exact upgrade or permissive name fallback. P1’s module match alone is insufficient: retain the landed import eligibility, local shadow, export/wrapper/span/unique callable gates. CJS span-less exports, require bindings (including a module shared with ESM) and positions lacking the specific imported binding proof stay at base through the new non-relative route.

ASSUMPTION: the SPEC §3 cut is the dispatch recommendation. Node/Node10, exact and one-star singleton paths, local single-parent captured extends, effective property origins, nearest including ancestor, corrected parent/dotted exclude and JavaScript-family membership, same-stem/declaration sibling barrier, package/Unicode/case membership barriers, jsconfig barriers where the directory lacks tsconfig and where an excluding tsconfig has a sibling jsconfig; disableSolutionSearching also blocks fall-through from an excluding config, unchanged references/files-empty barriers, supported JSONC/globs, unknown dotted suffix probes and unique physical indexed candidates. Unsupported input preserves base. Do not “complete TypeScript semantics” beyond that cut.

## Owned files

READ: the cumulative patch enumerates its exact paths. Production ownership: `src/ast.rs` initializes the landed namespace terminal provenance to false; `src/js_exports.rs` skipped-star provenance; new `src/js_paths.rs`, `src/js_paths_snapshot.rs`, `src/js_paths_syntax.rs`; `src/lib.rs`; `ScopeGraphBuildInputs` and the derived map/full recompute in `src/call_graph.rs`; loader-created snapshot/topology in `src/repo_loader.rs`; R4c member-module helper/span requirement in `src/resolution.rs`; `src/navigation/call_resolve.rs`; incremental recomputation in `src/cpg/build.rs`; cache versions and pin tests in `src/cpg_cache.rs` and `src/navigation/call_edge_cache.rs`.

READ: tests: `tests/integration/js_paths_test.rs`, its main module entry; four new Tier-A directories `eval/fixtures/{javascript,typescript}/tsconfig_paths_{positive,refusal}`. R1 also owns src/js_exports.rs and tests/integration/fixtures/js_paths_r1.json; R2 adds tests/integration/fixtures/js_paths_r2.json. The landed namespace resolver, `JsExportFacts::is_empty` namespace completeness, and positional/callable classification are reused without a second namespace or wrapper resolver. Any extra P1 value-empty facts have `namespace_proof_complete=false`, so they cannot add namespace proof. A local namespace terminal gets false provenance; unresolved namespace branches already return Err and emit no identity. The alias-only skipped-star provenance leaves base relative binding results intact; value-empty parsed modules retain empty facts. All relative/alias prefix guards trim first; alias map keys retain the raw spelling. No other existing test expectation is changed. Do not edit CLAUDE.md or existing Tier-A baselines. The Go symlink manifest test remains unchanged: its initially observed regression was fixed by omitting JS occupancy topology when no config exists.

## Execution order

ASSUMPTION: perform the following bounded build sequence, retaining prototype code unless a constructible finding requires a targeted correction.

1. MEASURED requirement: bind checkout, parent, dirty/owned paths, source bundle, cumulative patch and binaries. Controller creates the Git worktree/commits. Confirm cache parent versions; 105/61 applies only to this 104/60 parent.
2. MEASURED requirement: reproduce behavioral RED on the retained base. `probes/P1-tests.rs` is a standalone public-API harness; the controller can compile it with the base library and matching dependencies. At least the exact/wildcard, inherited path origin, baseUrl origin, discovery, JSONC and index/barrel groups must fail with empty UnknownName targets, not setup errors. Synthetic base/head rows retain exact caller and byte spans. The existing negative rows must equal base.
3. ASSUMPTION: examine the three small kernel modules first. Resolve any owner/model amendments in place. Configuration bytes and occupancy must be read into loader inputs, never fetched by a call-resolution helper. Convention-only inputs remain empty.
4. ASSUMPTION: retain the one R4c helper integration and nav incoming-caller helper. Do not change relative export closure or R3. Fresh whole-graph inputs recompute the derived map on incremental/config-only rebuilds before graph/DFG assembly. Retain 105/61 (or actual parent +1) and both pins.
5. MEASURED requirement: run all integration rows in JSX and TSX, `controls_gen.py`, public rowdiff and TypeScript audit. Every changed row must independently pass site-import, module, callable/name/span identity checks. Any unproven changed row is an open gate. Controller alone runs private F; do not read private inputs or raw private evidence.
6. MEASURED requirement: rerun all thirty kernel mutants and the eleven integration mutants; each alone, with admissible behavioral output. Serialize mutation and suite builds when using a shared target directory; never replace a live binary pathname. Use immutable target/paths-plan/extension-priority/head/prism for measurements. Kernel mutation does not certify graph wiring. The CJS span-less case is the I03 boundary control, not a claim that every CJS callable is wrong. I04 kills bypass of the specific binding/position guard; preserve Position/Unchecked/MayCall/unbound states.
7. MEASURED requirement: verify with the full suite, release build, Tier-A matrix/quick and formatter; run scoped clippy. Largest runnable subsets and exact exclusions are reported. Do not install/fetch. Full `--corpus all` Tier-A remains human-triggered.
8. READ custody requirement: refresh HANDOFF at each stable point using the steering template. Controller commits explicit owned paths. Report source/test/fixture size, remaining work, commands/results and every exclusion; no push, merge or P2 adoption.

## Commands

ASSUMPTION: from the implementation worktree, use its rebuilt binary and local evidence root; these commands are executable recommendations, not receipts:

```bash
cargo fmt --check
PRISM_TYPESCRIPT="$TS_JS" cargo test --offline --all-features
cargo build --release --offline
cd eval
uv run tier-a --matrix-only --allow-stale-sut
uv run tier-a --quick --allow-stale-sut
```

READ: in this sandbox `uv`’s normal cache is not writable. The planner used installed Python 3.12 directly for the stdlib Tier-A harness, without installs; see MEASUREMENTS. Do not repeat the denied uv cache write. A controller-created actual Git worktree is required for the prototype’s own tracked-universe quick run. `quick_control.py` against real main is a separately labeled preservation control, not that check.

ASSUMPTION: public and synthetic measurement:

```bash
bash probes/run_dumps.sh "$BASE_BIN" "$OUT/base" X R T
bash probes/run_dumps.sh "$HEAD_BIN" "$OUT/head" X R T
python3 probes/rowdiff.py "$OUT/base/X-dump-sites.jsonl" "$OUT/head/X-dump-sites.jsonl" "$OUT/X-changes.json"
# Run oracle.cjs for each corpus with its base import facts and changed-row file;
# exact positional arguments are recorded by probes/reproduce_public.py.
python3 probes/controls_gen.py "$OUT/controls"
python3 probes/mutants.py "$WORKTREE" "$OUT" "$RELEASE_DEPS" "$OUT/controls"
python3 probes/integration_mutants.py "$WORKTREE" "$OUT" "$CARGO_TARGET_DIR_FOR_MUTANTS"
python3 probes/cache_probe.py "$BASE_BIN" "$HEAD_BIN" "$OUT/cache"
```

ASSUMPTION: size forecast **750–900 source / 650–850 tests / 70–100 fixtures**; no cap. Stop only for changed scope/authority or a non-converging open-class model issue. For closed enumerable review findings, fold bounded fixes on this body and rerun the discriminating evidence. At round two classify convergence before acting; disclose any owner-authorized extension.

## Round-1 fold and controller remeasurement

READ: W1-r2 and W3 remain WRONG, corrected in place with matcher barriers and Fix A; the post-cap confirmation residual (1 WRONG / 0 SMELL) is now corrected with the generic compiler priority table, C85/C86/O, literal/declaration exceptions and M29/M30; S6 remains SMELL and an owner question because Fix B would remove 89 X gains. ProjectService ownership independently classifies every disagreement UNPROVEN. W1 and W2 remain WRONG, corrected in place; S1–S5 remain SMELL, addressed without downgrading findings. See MEASUREMENTS for killing controls and per-row results. If independent round 2 finds further membership mismatches, use the controller’s refuse-on-any-doubt disposition; do not widen the model piecemeal.

Controller F command (defaults bind the fresh immutable binary):

```bash
bash docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh
```

Provide CORPUS_F_ROOT, PRIVATE_EVIDENCE_ROOT and TS_JS privately. Stdout contains counts/classes, the head hash and one aggregate refusal-reason code per unchanged oracle-recoverable row. Reasons are independent ordered P1 cuts, not production telemetry; UNCLASSIFIED_P1_PROOF stays open. The revised oracle takes owning projects from real offline ProjectService, classifies every root-file disagreement UNPROVEN and reports disagreements separately; it requires explicit effective Node/Node10 and applies P1 barriers; prior F 2,345/3,102 (75.60%) is controller-supplied and uses the earlier denominator. Never open private source or raw receipts in the planner lane.

Controller commit recommendations: plan `docs(paths): refresh extension-priority evidence and dispatch`; prototype `fix(paths): apply TypeScript root-file extension priorities` (cumulative patch starts at e61d52b8).

READ: current residual receipts supersede spec-r2 where source or counts differ. See BUILD-MANIFEST and MEASUREMENTS for 197 controls, fresh default/MCP suites, matrix, S1b identity and exclusions. No new review round is requested.

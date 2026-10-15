# Go Type Extraction Speedup Implementation Plan

> **For agentic workers:** Execute task by task as **gpt-6.1-sol (Codex), xhigh**. Use `superpowers:executing-plans` if installed; otherwise follow these checkbox steps directly. The implementer receives this plan, so the constraints, code, tests and acceptance decisions are included here. The controller owns commits, probe worktrees and real-corpus measurements.

**Goal:** preserve byte-identical navigation, review, SARIF, targets and MCP Evidence while making Prometheus cache-hit indexing well under 5 seconds and at least halving the combined time of its two cold-build Go extractions. This plan defines “well under” as both measured warm repetitions at or below **4.0 seconds**. That is an acceptance threshold, not a measured result.

**Architecture:** retain the serial path-ordered extraction and all writes to `GoTypeData`. Give each `extract_from_file` invocation a lazy `FileSignatureImports` that computes the existing signature import map once and lends it to every signature in that file. Keep the existing import-aware and plain constructors separate. A test-only mode executes the original recompute-per-signature behavior through the same walker; compare the entire `GoTypeData` Debug representation, including telemetry, and count scans without a timing assertion. Nothing is cached across files or provider constructions.

**Tech Stack:** existing Rust 2021, `std::cell::OnceCell`, `std::borrow::Cow`, `BTreeMap`, tree-sitter; no dependency, serialization, public API, thread-pool or cache-format change.

**Spec/evidence:** owner request dated 2026-10-14; `.review/FINDINGS.md:3-21`; `.review/probe-summary-prometheus.md:3-39,63-87`; `~/prism-evidence/mcp-memory/optimization-2026-10-14/PROCEDURE-probe.md`; probe commit `c3162f9419f371fa4333ef0f038b48845691e310` on local `scratch-opt-probe`; this plan's measured prototype evidence in `.review/go-extract-plan-probe/`. Source references below bind to **`8b2744b3154b3664ffacb23c42f042a67981967a`**, this clone's initial HEAD and `origin/main`. The relevant Go extraction/alias files have no diff from the parent of the historical probe commit (command: `git diff 'c3162f94^' HEAD -- src/type_providers/go.rs src/go_type_alias.rs src/go_type_alias/canon.rs`; observed no output).

## Global Constraints

- Preserve byte-identical outputs on identical inputs: navigation, review, SARIF, targets and MCP Evidence. Use BTreeMap/BTreeSet and ordered iteration throughout. Keep every first-wins, last-wins, tie-break and Vec order as it actually exists; “first-wins” is not a license to change last-wins maps.
- **Files under 600 lines; tests split by category.** The proposed new files are 58/105/99/48 lines before wiring additions (measured with `wc -l` on the plan's source blocks). Existing `src/type_providers/go.rs` is **3,596 lines**. **O1 (resolved, see Owner Questions):** recommended narrow legacy-file exception for this PR, with every new file below 600 and no unrelated movement. This plan does not silently declare that exception approved. If the owner requires the existing file to be under 600 too, park implementation and commission separate bounded mechanical split plans before this PR; do not turn the speedup into a multi-thousand-line rewrite. Writing this plan is authorized regardless of O1.
- Run `cargo fmt`, full `cargo test`, and full `cargo test --features mcp`; report totals including failures/ignored tests. A zero-test selection is not GREEN. Every new behavior has a failing pre-fix test and an edge/negative case below. Same-environment base controls precede regression attribution. Report unrelated failures; never rebaseline or silently fix them.
- Type providers feed call resolution: immediately rebuild with `cargo build --release`, then run `cd eval && uv run tier-a --matrix-only --allow-stale-sut` before each task commit. Before review, rebuild immediately before `--quick --allow-stale-sut`. Paste regressions/flip candidates into the PR; never rebaseline. Full `--corpus all` remains human-triggered. If facilities are unavailable, report exact exclusions and leave the affected acceptance condition open.
- Run `python3 scripts/mutgate/mutgate.py --since main --scope fn` per review round; the authoritative full `python3 scripts/mutgate/mutgate.py` is mandatory before merge. Append `--jobs 1` on this host. The current manifests under `mutants/` do not name `src/type_providers/go.rs`; scoped zero selection is an explicit coverage gap, not certification. The targeted mutants in Task 2 cover this change independently.
- Any change to serialized data/meaning requires the CPG cache version (`src/cpg_cache.rs`) and/or navigation sidecar version to be bumped. This plan changes neither data nor meaning; a dump difference is a failed equivalence gate, not a reason to bump a version and accept it. Current source values are CPG 117 (`src/cpg_cache.rs:261`) and nav sidecar 65 (`src/navigation/call_edge_cache.rs:110`); rebind before execution.
- Cargo runs **only** as `nice -n 19 env CARGO_BUILD_JOBS=4 cargo ...`, with output redirected to a file and read afterwards; never pipe a running cargo into grep/tail. One cargo process at a time, including subprocesses of mutation tooling. Use the wrapper below for tools that invoke bare cargo. No installs/network in the sandbox; route required host facilities to the controller upfront. A denied priority adjustment is not a successful low-priority run.
- Never `git stash`, never `git commit --amend`, never normalize another worker's changes. `.git` writes are controller-only on this machine. Every task has a controller commit step with both exact trailers below. No commit/push is authorized during plan authorship.
- Use a two-round review cap per PR, reviewers **Opus 5.5 + gpt-6-astra**. The controller dispatches them; this plan does not ask the implementer to spawn agents. Tag every finding WRONG (concrete input/state and wrong result) or SMELL, WRONG first. At round 2 classify CLOSED-ENUMERABLE versus OPEN-CLASS: fold bounded fixes on this artifact; extend only with a one-line convergence justification; park open-class findings for design. Never restart the artifact to escape the cap.
- Before every diagnostic probe, record hypothesis, predicted result, falsifier; afterward record the actual result. Name an alternative cause and the separating observation before editing. A broken probe is inadmissible, not disproof. At stable points snapshot artifacts and refresh the lane handoff using `~/.codex/handoff-template.md`; keep evidence logs. Do not update personal memory without a direct owner request.

Commit trailers (controller, every task):

```text
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh
```

## Diagnosis: hypothesis, probe, result

**Inherited measurement, not rerun here:** the historical instrumented full-corpus runs reported 975 Go files, 13.6/13.8 s cache-hit index time, 10.8/11.2 s `go.file_extract`, about 37 ms satisfaction, and approximately one second decoding/reconstruction (`.review/FINDINGS.md:10-19`). Host contention and swap were present (`:4`); CPU approximately equaled wall in the extraction. This author did not run prism-mcp on either real corpus.

**H1:** repeated whole-file work inside signature extraction dominates. **Prediction:** `signature_imports` and its two AST scans occupy most of extraction; reducing scan frequency leaves canonicalization counts and full data unchanged. **Falsifier:** those scans are a small fraction, or caching fails to lower file extraction. **Alternative H2:** expensive alias resolution or profile lookup dominates, potentially scanning all files per signature. **Discriminator:** separately time those callees and inspect their lookup population.

Source trace:

- `go.rs:1783-1803,1902-1917` and `1397-1498` route method/interface signatures through `canon_sig`; `1639` rebuilds imports on every invocation.
- `src/go_type_alias/canon.rs:392-443` calls both `ParsedFile::extract_imports` and recursive `has_dot_import`. `src/ast.rs:1608-1621,5655-5684` recursively traverses method bodies looking for imports. A file with S signatures and N AST nodes pays O(S×N) traversal, not O(N+S). Growing methods and bodies together yields quadratic within-file work.
- `src/go_type_alias.rs:447-522` performs BTreeMap lookups and walks declarations for the selected owner; it does not enumerate every file on every signature. Pathological same-owner variants/alias graphs remain input-sensitive; do not claim all Go type extraction is linear. Profiles are built over files once (`go.rs:515`) and also redundantly per declaration (`874,973,1022,1809`), but the measured profile cost is negligible in this experiment.

**Probe performed:** `setup.py` copied 24 evenly spaced paths from the sorted list of 715 readable non-vendored `.go` files under `~/code/bench-repos/prometheus` into `.review/go-extract-plan-probe/corpus/`: 300,299 bytes. This population differs from the historical loader's 975; it is a small subset experiment, not a census reconciliation. `corpus.tsv` records paths/sizes/SHA-256. Parsing is outside the timed extraction. One temporary instrumented release example compares alternating serial/cached modes twice, with cache state reset per construction; it also generates files with 16, 32 and 64 methods. The prototype returns cloned cached maps and uses a temporary thread-local cache; the production design below instead borrows a map scoped to the file. Neither prototype state nor instrumentation is retained in production.

Commands actually executed:

```sh
nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --offline --release --example plan_go_probe > .review/go-extract-plan-probe/build.log 2>&1
```
Expected: one optimized example built without fetching dependencies. **Observed:** `Finished release ... in 1m 13s`; however `nice: setpriority: Operation not permitted` occurred first. Cargo continued despite it. This is a compiled prototype receipt, **not proof of low host priority**. `pgrep` also could not inspect processes. No second cargo build was run.

```sh
cat .review/go-extract-plan-probe/build.log
```
Expected: inspect compiler diagnostics, not just exit status. Observed the warning above, successful compilation, no Rust compile error.

```sh
target/release/examples/plan_go_probe .review/go-extract-plan-probe/corpus > .review/go-extract-plan-probe/results.log 2>&1
```
Expected: alternating timing rows and full Debug equality for all four input groups. Observed all four equality checks; 16 provider constructions, 12 comparisons against each group's first serial result.

```sh
cat .review/go-extract-plan-probe/results.log
```
Expected: read counts and times. Observed:

| Input | Old file extraction, ms | Cached file extraction, ms | Evidence |
|---|---:|---:|---|
| Prometheus 24 files | 278.882 / 277.527 | 9.981 / 9.483 | `results.log:1-30` |
| 16 methods, one file | 1.398 / 1.379 | 0.136 / 0.135 | `results.log:31-60` |
| 32 methods, one file | 5.375 / 5.419 | 0.270 / 0.267 | `results.log:61-90` |
| 64 methods, one file | 21.202 / 21.211 | 0.532 / 0.531 | `results.log:91-120` |

On the 24-file set, serial `signature_imports` costs 285.3–286.7 ms **including resolver setup outside the file loop**; its constituent `extract_imports` is 143.5–144.3 ms and `has_dot_import` 141.3–141.9 ms. Do not add nested timers or claim 286 ms is a subset of 278 ms. Calls fall from 187 to 38 (24 resolver-setup scans plus 163→14 signature-phase scans). Alias canonicalization stays at 330 calls and 0.15–0.27 ms; profiles stay at 281 calls and 0.27–0.34 ms (`results.log:1-29`). H1 is supported; H2 does not explain this subset's cost. The cached/serial file-loop ratio is 0.034–0.036 (**derived**). Extrapolating this to the full corpus suggests about 0.4 s extraction and roughly 3.1 s indexing (**guessed projection**, not an acceptance result).

A second run used eight hand-written files with aliases/cycles, embedding, pointer embedding, generic receivers, build tags, package-name collisions, external test packages, dot imports and versioned import aliases:

```sh
target/release/examples/plan_go_probe .review/go-extract-plan-probe/fixtures > .review/go-extract-plan-probe/fixture-results.log 2>&1
```
Expected: equal dumps for that set plus the three synthetic groups. Observed all four equality checks, 16 constructions and 12 comparisons. The example's first label is hardcoded `prometheus24`; its `files=8` row identifies the fixture run.

```sh
cat .review/go-extract-plan-probe/fixture-results.log
```
Expected: inspect equality, counts and timings. Observed equal full dumps; these are plain-constructor prototype tests, **not** compiled validation of the plan's final code or the import-aware oracle.

**Validated in authorship:** one release example build; two example runs, 32 constructions, 24 full-data comparisons; the exact wiring script below replayed on a copied original `go.rs` and all anchors matched; four new Rust source blocks parsed/formatted with rustfmt. All five instrumented tracked files were restored byte-for-byte. No full suite, Tier-A, mutation gate, final helper compilation, final nine tests, import-aware prototype, real-corpus measurement or independent review ran. Evidence source snapshots and instrumentation remain under `.review/go-extract-plan-probe/`; those ignored files are not automatically included in a PR, so the controller must archive them with the plan.

## Decisions

1. **Remove repeated whole-AST import scans before parallelism.** A lazy map lives only for one file extraction and uses the unmodified signature-import builder, including dot-import poison, empty-key local identity and version alias inference. No profile optimization is bundled. **Cost if wrong:** target performance may fail; Task 4 records that failure and a new measured residual bottleneck is needed before broadening scope.
2. **Retain serial BTreeMap path order and AST source order.** No rayon work in this PR. `GoAliasResolver` has `RefCell` telemetry (`src/go_type_alias.rs:66-71,254-273,569`), so simply capturing it in `par_iter` is not a correct parallel design. **Cost if wrong:** unnecessary residual latency; much lower correctness risk than prematurely changing state ownership.
3. **Do not share final plain/import-aware providers.** Both cold extractions already happen inside call-graph construction, in sequence (`src/call_graph.rs:2094-2113,4094-4098`). `CpgContext` takes the already-built plain provider (`src/cpg/context.rs:101-113`). Correct the interpretation of `.review/FINDINGS.md:19`: the registry does not perform a third extraction on this path. Historical cold counters differ: 722 versus 491 satisfied pairs (`probe-summary-prometheus.md:37-39`), and registry.go is 0 ms (`:25`). Package paths change local `T` from bare identity to `~example/p::T`, imported `p.T` uses `@example/p::T` (`canon.rs:16-64`), and paths populate alias package resolution (`go_type_alias.rs:118-130,467-480`). Task 1 pins a concrete difference. **Cost if wrong:** sharing these final results changes dispatch. Reduce both calls' cost by over half instead; removing the remaining two cheap scans/constructors is outside scope.
4. **Oracle recomputes imports per signature; it does not normalize output.** The change is input reuse, so both paths share the unchanged extraction/satisfaction code. The oracle differs at the replaced work: always executing the original builder. Full Debug compares every field, Vec order and telemetry, including the same-thread measurement generation. No hashing away mismatches, ignoring fields or blessing changed goldens. **Cost if wrong:** a future shared mutation could fool both paths; explicit winner/path tests and targeted mutants supplement the oracle.
5. **One small implementation PR, Tasks 1–4.** Task 1 installs the mode seam and seven parity tests with old behavior; Task 2 adds two scan-cost tests and the actual optimization; Task 3 closes output/gate evidence; Task 4 supplies clean paired binaries and controller acceptance results before merge. No parallelism PR or final-provider-sharing PR is authorized. O1 is the only scope decision remaining. **Cost if wrong:** strict legacy file-size enforcement requires separately planned mechanical PRs; do not hide that work inside this optimization.

## Review Focus

| Input class / failure | Owning test (task) |
|---|---|
| Method/interface signatures repeatedly traversing large method bodies | `repeated_signatures_scan_imports_once_per_file_not_once_per_signature` (T2; baseline RED) |
| Empty maps, non-Go files, structs without signatures | `files_without_signatures_and_non_go_files_do_not_scan` (T2) |
| Aliases, alias cycles, parameterized aliases, generic receivers, interface/struct/pointer embedding | `aliases_generics_embedding_and_pointer_embeds_are_identical` (T1) |
| Linux/Windows variants, malformed build headers, `p_test` namespaces | `profile_variants_and_external_test_packages_are_identical` (T1) |
| Duplicate type names and invalid duplicate method declarations | `last_map_write_and_first_method_winner_keep_path_and_source_order` (T1) |
| Version suffix inference, explicit alias override, inferred collisions, dot-import poison | `version_aliases_explicit_aliases_dot_imports_and_collisions_are_identical` (T1) |
| Missing import qualifier/package clause | `missing_qualifier_and_no_signature_files_are_identical` (T1) |
| Distinct plain/import-aware data and satisfaction | `plain_and_import_aware_constructors_must_remain_distinct` (T1) |
| Same path, changed source/imports across constructions | `changing_source_at_the_same_path_cannot_reuse_old_imports` (T1) |
| Cold-build retained provider/lifetime | existing `p15a_fix1_single_plain_provider_peak_live_one_through_full_context_path` (T3; `call_graph.rs:7060`) |
| Downstream outputs and cached versus cold behavior | complete Go/navigation/CLI/MCP suites and unchanged goldens (T3), paired valid cache-hit measurements (T4) |

## State-write and ordered-merge proof

`extract_from_file` reads the current AST, package path and alias resolver. It does **not read prior files' accumulated `GoTypeData` to decide what to extract**: its transitive raw writers only insert, append or extend. Their source is `go.rs:759-1035,1129-1177,1783-1847`. Therefore a future pure phase can produce per-file facts with fixed immutable resolver inputs; an ordered merge can reproduce the serial writes. This PR retains the existing writes, avoiding that new representation altogether. A parallel design would additionally separate resolver facts from its non-Sync telemetry, aggregate **per-file** telemetry without changing expansion-call counts, run within `build_pool::install`, collect an indexed path-sorted Vec, and merge serially. No unordered reduction, resolver clone per file, shared mutable map, or global lock is accepted as an equivalent shortcut.

Every field in `GoTypeData` (`go.rs:147-254`):

| Field | Raw extraction effect / exact merge rule |
|---|---|
| `structs`, `interfaces`, `aliases` | `.insert`: last declaration in source order, then last lexicographic path wins; do not change to `or_insert`. |
| `methods` | Per receiver Vec append in source order; concatenate per-file vectors in path order. Downstream `insert_concrete_method` uses first-wins `or_insert` (`2480-2504`). |
| `struct_identities`, `func_typed_fields` | BTreeSet union. |
| `struct_declaration_files`, `type_declaration_files` | Per-owner BTreeSet union of paths. |
| `type_declarations`, `struct_declarations`, `interface_declarations`, `method_declarations` | Per-owner BTreeSet union of complete declaration values, not replacing a whole owner's set. |
| `field_types` | Last write per `(owner,field)` wins; union maps with replacement in path order. Keep earlier keys that a later declaration lacks, as current code does. |
| `struct_embeds`, `struct_embed_files` | Last write per owner, keep the pair in lockstep; embedded Vec order is AST order. |
| `interface_name_owners` | Per-name BTreeSet union of package directories. |
| `dispatch_overapprox` | Append raw per-file Vec in source/path order (currently interface extractor produces no entries); later satisfaction behavior remains unchanged. |
| `alias_expanded`, `alias_unresolved` | Not written by raw file extraction; copy resolver telemetry after all files. Future isolated telemetry merges by integer addition/per-key addition with unchanged overflow behavior and invocation count. |
| `satisfaction`, `sat_keys`, `dispatch_gaps` | No raw write; compute once after ordered merge. Preserve satisfaction BTreeSet order, sat-key Vec insertion order and dispatch-gap order. |
| `field_targets` | No raw write; run `remove_unproven_pointer_embed_pseudo_fields` once after merge with the complete profiles. That pass also removes unproven entries from `field_types`. |
| `embedded_interface_promotions`, `embedded_interface_routes` | No raw write; satisfaction computes them after all facts exist. |
| `measurement_generation` (test only) | Set once per complete provider, not per file. File partials must not increment/decrement provider counters or trigger `GoTypeData::Drop` accounting. |

## File Map

| Path | Change |
|---|---|
| `src/type_providers/go.rs` | T1 private mode entry, per-file context, signature argument forwarding, child-module declarations; all data writes retained. Legacy size decision O1 applies. |
| `src/type_providers/go/signature_inputs.rs` | T1 private lazy-cache holder initially recomputing; T2 activates OnceCell; test-only scan counter and recompute mode. |
| `src/type_providers/go/extraction_equivalence_tests.rs` | T1 full-data oracle helpers and type/profile/order tests. |
| `src/type_providers/go/extraction_import_tests.rs` | T1 import/provenance/freshness tests. |
| `src/type_providers/go/extraction_cost_tests.rs` | T2 deterministic scan-count and lazy-zero-path tests. |
| `docs/superpowers/plans/2026-10-14-go-type-extract-speedup-validation.md` | T1–T4 exact receipts, totals, failures/exclusions, review decisions and controller acceptance record. No invented totals. |
| `.review/go-type-extract/` | Local logs, baseline snapshots, temporary wiring script; archive externally at each stable point. Do not merge prototype timers. |
| `src/go_type_alias*`, `src/ast.rs`, `src/call_graph.rs`, `src/cpg*`, `src/navigation/`, `Cargo.toml`, golden files | No implementation changes. Probe-only worktrees may carry the historical instrumentation. |

## Command protocol and controller setup

Before task execution, record O1 and bind `main`. This clone has `origin/main` at the source base but **no local main** (authorship: `git rev-parse main` failed). The controller creates the local ref from verified `origin/main`; do not silently substitute another comparison base.

```sh
git branch main origin/main
```
Expected: controller-only creation when `main` is absent; it resolves to `8b2744b3154b3664ffacb23c42f042a67981967a` on this source cut. If the base advanced, rebind and review anchor drift before edits; never force-update an existing main.

```sh
mkdir -p .review/go-type-extract/logs .review/go-type-extract/tools
```
Expected: local log/tool directories exist. Create this wrapper as `.review/go-type-extract/tools/cargo`:

```sh
#!/bin/sh
exec nice -n 19 env CARGO_BUILD_JOBS=4 "$PRISM_REAL_CARGO" "$@"
```

```sh
export PRISM_REAL_CARGO="$(command -v cargo)"
```
Expected: absolute real cargo path captured **before** extending PATH.

```sh
chmod +x .review/go-type-extract/tools/cargo
```
Expected: wrapper executable.

For mutation tool commands only, prefix `env PATH="$PWD/.review/go-type-extract/tools:$PATH" PRISM_REAL_CARGO="$PRISM_REAL_CARGO"`; run with `--jobs 1`. This preserves the required low priority and cargo job count for its subprocesses. Stop and route to the controller if priority/process inspection is denied. Do not repeat authorship's ineffective priority adjustment as if it succeeded.

The following shell helpers make task command receipts uniform. Define once in the controller/implementer shell:

```sh
run_cargo() {
  log="$1"; shift
  nice -n 19 env CARGO_BUILD_JOBS=4 cargo "$@" > "$log" 2>&1
  result=$?
  cat "$log"
  return "$result"
}
commit_task() {
  git commit -m "$1" \
    -m 'Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>' \
    -m 'Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh'
}
```
Expected: `run_cargo` saves and reads complete logs with one process at a time; `commit_task` is used **only by the controller**, after staging explicit task files and gates. No shell pipeline consumes cargo output.

```sh
unset PRISM_MCP_BLESS_GOLDEN
```
Expected: all baseline and candidate MCP suites run with golden blessing disabled.

### Task 1: Pin the old behavior and introduce per-file signature inputs (no speed change)

**Files:** modify `go.rs`; create `signature_inputs.rs`, `extraction_equivalence_tests.rs`, `extraction_import_tests.rs`; temporarily create an empty `extraction_cost_tests.rs` so all declared modules resolve. Create the validation receipt document. **Commit:** `refactor(go): isolate per-file signature import inputs with parity oracle`.

- [ ] **Step 1 — baseline control.** Freeze HEAD/status and run the same environment's full baseline suites before editing. Store all totals and any failures in the validation document.

```sh
git status --short
```
Expected: only this lane's known plan/evidence; stop on unrelated changes in implementation targets.

```sh
run_cargo .review/go-type-extract/logs/base-default.log test
```
Expected: full default suite completes with 0 failed; record actual totals, not inherited counts.

```sh
run_cargo .review/go-type-extract/logs/base-mcp.log test --features mcp
```
Expected: full MCP-feature suite completes with 0 failed; preserve baseline failures if any and route unavailable facilities to controller.

- [ ] **Step 2 — write seven tests first.** Immediately after `use std::sync::Arc;` in `go.rs`, insert exactly these declarations, then create the two complete test files below and the empty cost-test file. At this point the mode/helper APIs are intentionally absent. Do not add the helper module/import until Step 4's edit script.

```rust
#[cfg(test)]
mod extraction_equivalence_tests;
#[cfg(test)]
mod extraction_import_tests;
#[cfg(test)]
mod extraction_cost_tests;
```

Create `src/type_providers/go/extraction_equivalence_tests.rs`:

```rust
use super::*;

pub(super) fn files(rows: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    rows.iter()
        .map(|(path, source)| {
            (
                (*path).into(),
                ParsedFile::parse(path, source, Language::Go).unwrap(),
            )
        })
        .collect()
}

pub(super) fn check(rows: &[(&str, &str)], paths: &BTreeMap<String, String>) -> GoTypeProvider {
    let files = files(rows);
    for aware in [false, true] {
        let empty = BTreeMap::new();
        let imports = if aware { paths } else { &empty };
        let old = GoTypeProvider::from_parsed_files_with_import_scan(
            &files,
            imports,
            !aware,
            ImportScan::EverySignature,
        );
        let new = if aware {
            GoTypeProvider::from_parsed_files_with_package_import_paths(&files, imports)
        } else {
            GoTypeProvider::from_parsed_files(&files)
        };
        assert_eq!(format!("{:#?}", old.data), format!("{:#?}", new.data));
    }
    GoTypeProvider::from_parsed_files_with_package_import_paths(&files, paths)
}

#[test]
fn aliases_generics_embedding_and_pointer_embeds_are_identical() {
    let rows = [
        (
            "p/a.go",
            r#"package p
        type Word = int
        type BrokenA = BrokenB
        type BrokenB = BrokenA
        type I interface { M(Word) }
        type Inner struct { Callback func(int) }
        func (Inner) M(x Word) {}
        type Outer struct { *Inner }
        type Unknown struct { *Missing }
        type Embedded interface { I }
        type Holder struct { I }
        type Box[T any] struct { Value T }
        func (b Box[T]) Get() T { return b.Value }
        type IntBox = Box[int]
        type Param[T any] = []T
        func (Inner) Slice(x Param[int]) {}
        func (Outer) Bad(x BrokenA) {}
        "#,
        ),
        (
            "q/a.go",
            "package q\ntype Inner struct { Other string }\nfunc (*Inner) M(x string) {}\n",
        ),
    ];
    let p = check(&rows, &BTreeMap::new());
    assert!(p.data.alias_unresolved.values().sum::<usize>() > 0);
    assert_eq!(p.data.structs["Inner"].file, "q/a.go");
}

#[test]
fn profile_variants_and_external_test_packages_are_identical() {
    check(&[
        ("p/a_linux.go", "//go:build linux\n\npackage p\ntype T struct { X int }\nfunc (T) M(x int) {}\n"),
        ("p/a_windows.go", "//go:build windows\n\npackage p\ntype T struct { X string }\nfunc (T) M(x string) {}\n"),
        ("p/b.go", "package p\ntype Outer struct {*T}\ntype I interface {M(int)}\n"),
        ("p/a_test.go", "package p_test\ntype T struct{}\nfunc (T) M(x bool) {}\n"),
        ("p/bad.go", "//go:build (\n\npackage p\ntype A = T\nfunc (T) Bad(x A) {}\n"),
    ], &BTreeMap::new());
}

#[test]
fn last_map_write_and_first_method_winner_keep_path_and_source_order() {
    let p = check(
        &[
            (
                "z/t.go",
                "package z\ntype T struct {Z string}\nfunc (T) M() {}\n",
            ),
            (
                "a/t.go",
                "package a\ntype T struct {A int}\nfunc (T) M() {}\nfunc (T) M() {}\n",
            ),
        ],
        &BTreeMap::new(),
    );
    assert_eq!(p.data.structs["T"].file, "z/t.go");
    let methods = &p.data.methods["T"];
    assert_eq!(
        methods.iter().map(|m| m.file.as_str()).collect::<Vec<_>>(),
        vec!["a/t.go", "a/t.go", "z/t.go"]
    );
    assert!(methods[0].start_line < methods[1].start_line);
    let winner = p.resolve_dispatch("T", "M", &BTreeSet::new());
    assert_eq!(winner[0].file, "a/t.go");
    assert_eq!(winner[0].start_line, methods[0].start_line);
}
```

Create `src/type_providers/go/extraction_import_tests.rs`:

```rust
use super::extraction_equivalence_tests::{check, files};
use super::*;

#[test]
fn version_aliases_explicit_aliases_dot_imports_and_collisions_are_identical() {
    check(
        &[
            (
                "p/a.go",
                r#"package p
        import ("example/widget/v2"; "gopkg.in/yaml.v3")
        type I interface { M(widget.T, yaml.T) }
        type T struct{}
        func (T) M(x widget.T, y yaml.T) {}
        "#,
            ),
            (
                "p/b.go",
                r#"package p
        import (widget "different/widget"; "example/widget/v2")
        func (T) Explicit(x widget.T) {}
        "#,
            ),
            (
                "q/a.go",
                r#"package q
        import ("first/widget/v2"; "second/widget/v3")
        type T struct{}
        func (T) Ambiguous(x widget.T) {}
        "#,
            ),
            (
                "q/b.go",
                r#"package q
        import . "example/dot"
        func (T) Dot(x Unbound) {}
        "#,
            ),
        ],
        &BTreeMap::new(),
    );
}

#[test]
fn missing_qualifier_and_no_signature_files_are_identical() {
    check(
        &[
            (
                "p/missing.go",
                "package p\ntype T struct{}\nfunc (T) M(x missing.T) {}\n",
            ),
            (
                "p/no_methods.go",
                "package p\ntype Fields struct{ X int }\n",
            ),
            ("p/empty.go", "package p\n"),
            ("p/broken.go", "type T struct{}\nfunc (T) M(x int) {}\n"),
        ],
        &BTreeMap::new(),
    );
}

#[test]
fn plain_and_import_aware_constructors_must_remain_distinct() {
    let rows = [
        ("p/t.go", "package p\ntype T struct{}\nfunc (T) M(x T) {}\n"),
        (
            "q/i.go",
            "package q\nimport p \"example/p\"\ntype I interface {M(p.T)}\n",
        ),
    ];
    let paths = BTreeMap::from([
        ("p/t.go".into(), "example/p".into()),
        ("q/i.go".into(), "example/q".into()),
    ]);
    let aware = check(&rows, &paths);
    let plain = GoTypeProvider::from_parsed_files(&files(&rows));
    assert_ne!(
        plain.data.methods["T"][0].signature,
        aware.data.methods["T"][0].signature
    );
    assert!(!plain.data.satisfaction["I"].contains("T"));
    assert!(aware.data.satisfaction["I"].contains("T"));
}

#[test]
fn changing_source_at_the_same_path_cannot_reuse_old_imports() {
    let make = |import: &str| {
        format!("package p\nimport q \"{import}\"\ntype T struct{{}}\nfunc (T) M(x q.T) {{}}\n")
    };
    let a = make("example/a");
    let b = make("example/b");
    let first = check(&[("p/t.go", &a)], &BTreeMap::new());
    let second = check(&[("p/t.go", &b)], &BTreeMap::new());
    assert_ne!(
        first.data.methods["T"][0].signature,
        second.data.methods["T"][0].signature
    );
}
```

- [ ] **Step 3 — observe the infrastructure RED.** This is a missing-seam compiler failure, not the performance regression proof (that is T2).

```sh
run_cargo .review/go-type-extract/logs/t1-red.log test --lib type_providers::go::extraction_
```
Expected: compilation fails naming missing `ImportScan` / `from_parsed_files_with_import_scan`; do not count this as a behavioral failure or zero-tests GREEN.

- [ ] **Step 4 — wire the old behavior through the new context.** Create `signature_inputs.rs` with the following full code, but for **Task 1 only** replace the whole `get` body with `Cow::Owned(self.scan())`. Thus both modes still recompute, matching the pre-change code. Its `OnceCell` and mode fields become used in T2.

Create `src/type_providers/go/signature_inputs.rs`:

```rust
//! One signature-import scan per file extraction; no cache survives the file.
use crate::ast::ParsedFile;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) enum ImportScan {
    Once,
    #[cfg(test)]
    EverySignature,
}

pub(super) struct FileSignatureImports<'a> {
    parsed: &'a ParsedFile,
    local_import_path: Option<&'a str>,
    mode: ImportScan,
    cached: OnceCell<BTreeMap<String, String>>,
}

impl<'a> FileSignatureImports<'a> {
    pub(super) fn new(
        parsed: &'a ParsedFile,
        local_import_path: Option<&'a str>,
        mode: ImportScan,
    ) -> Self {
        Self {
            parsed,
            local_import_path,
            mode,
            cached: OnceCell::new(),
        }
    }

    fn scan(&self) -> BTreeMap<String, String> {
        #[cfg(test)]
        SCANS.with(|n| n.set(n.get() + 1));
        crate::go_type_alias::signature_imports(self.parsed, self.local_import_path)
    }

    pub(super) fn get(&self) -> Cow<'_, BTreeMap<String, String>> {
        match self.mode {
            ImportScan::Once => Cow::Borrowed(self.cached.get_or_init(|| self.scan())),
            #[cfg(test)]
            ImportScan::EverySignature => Cow::Owned(self.scan()),
        }
    }
}

#[cfg(test)]
thread_local! { static SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
pub(super) fn measured<R>(f: impl FnOnce() -> R) -> (R, usize) {
    SCANS.with(|n| n.set(0));
    let result = f();
    (result, SCANS.with(std::cell::Cell::get))
}
```

The only Task-1 override is exactly:

```rust
    pub(super) fn get(&self) -> Cow<'_, BTreeMap<String, String>> {
        Cow::Owned(self.scan())
    }
```

Save the following exact edit program as `.review/go-type-extract/wire.py`. It uses the repository's Rust lexer only to delimit existing functions; it does not invoke the mutation gate or cargo. Every anchor must match once. It tolerates only the three test-module declarations installed in Step 2, removing those before installing the complete declarations. Re-read the target region before applying; an anchor mismatch stops for a source rebind, never an approximate substitution.

```python
from pathlib import Path
import re, runpy
p = Path('src/type_providers/go.rs')
s = p.read_text()
lex = runpy.run_path('scripts/mutgate/mutgate.py')
def transform(name, f):
    global s
    hits = [(a, c + 1) for a, b, c in lex['fn_bodies'](s, lex['code_mask'](s))
            if re.match(r'fn\s+' + re.escape(name) + r'\b', s[a:])]
    assert len(hits) == 1, (name, hits)
    a, b = hits[0]
    s = s[:a] + f(s[a:b]) + s[b:]
def one(text, before, after):
    assert text.count(before) == 1, before
    return text.replace(before, after)
test_modules = """#[cfg(test)]
mod extraction_equivalence_tests;
#[cfg(test)]
mod extraction_import_tests;
#[cfg(test)]
mod extraction_cost_tests;
"""
if test_modules in s:
    s = one(s, test_modules, '')
s = one(s, 'use std::sync::Arc;', '''use std::sync::Arc;

mod signature_inputs;
use signature_inputs::{FileSignatureImports, ImportScan};
#[cfg(test)]
mod extraction_equivalence_tests;
#[cfg(test)]
mod extraction_import_tests;
#[cfg(test)]
mod extraction_cost_tests;''')
wrapper = '''fn from_parsed_files_inner(
        files: &BTreeMap<String, ParsedFile>,
        package_import_paths: &BTreeMap<String, String>,
        plain_entry: bool,
    ) -> Self {
        Self::from_parsed_files_with_import_scan(
            files, package_import_paths, plain_entry, ImportScan::Once,
        )
    }

    '''
def constructor(t):
    t = one(t, 'fn from_parsed_files_inner(', 'fn from_parsed_files_with_import_scan(')
    t = one(t, 'plain_entry: bool,', 'plain_entry: bool,\n        import_scan: ImportScan,')
    t = one(t, '                &alias_resolver,\n', '                &alias_resolver,\n                import_scan,\n')
    return wrapper + t
transform('from_parsed_files_inner', constructor)
def file_extract(t):
    t = one(t, 'alias_resolver: &crate::go_type_alias::GoAliasResolver,',
            'alias_resolver: &crate::go_type_alias::GoAliasResolver,\n        import_scan: ImportScan,')
    t = one(t, '        let root = parsed.tree.root_node();',
            '        let signature_imports = FileSignatureImports::new(parsed, local_import_path, import_scan);\n        let root = parsed.tree.root_node();')
    assert t.count('                        local_import_path,') == 2
    return t.replace('                        local_import_path,', '                        &signature_imports,')
transform('extract_from_file', file_extract)
def forwarding(t):
    t = one(t, 'local_import_path: Option<&str>', "signature_imports: &FileSignatureImports<'_>")
    return t.replace('local_import_path,', 'signature_imports,')
for name in ['extract_type_declaration', 'extract_type_spec', 'record_interface_type',
             'extract_type_alias', 'extract_interface_methods', 'walk_interface_body',
             'extract_method_signature', 'extract_method', 'extract_func_signature']:
    transform(name, forwarding)
def canonical(t):
    t = one(t, 'let imports = Self::signature_imports(parsed, local_import_path);',
            'let imports = signature_imports.get();')
    return forwarding(t)
transform('canon_sig', canonical)
transform('first_canon_sig_for_test', lambda t: one(t, '            None,\n            &alias_resolver,',
    '            &FileSignatureImports::new(parsed, None, ImportScan::Once),\n            &alias_resolver,'))
# The old wrapper has no remaining caller. Remove its documentation with it.
a = s.index('    /// Import aliases used only for signature identity.')
b = s.index('    pub(crate) fn canon_sig_with_imports(', a)
s = s[:a] + s[b:]
assert 'Self::signature_imports(' not in s
p.write_text(s)
print('wired per-file signature imports; extraction writes and iteration order untouched')
```

```sh
python3 .review/go-type-extract/wire.py
```
Expected: `wired per-file signature imports; extraction writes and iteration order untouched`. Review the diff: only constructor/mode plumbing, one context construction per Go file, forwarding references, the `canon_sig` input and test helper, and removal of the now-unused wrapper. Do not change profile extraction or alias resolver internals.

- [ ] **Step 5 — GREEN and verification.**

```sh
run_cargo .review/go-type-extract/logs/t1-fmt.log fmt
```
Expected: formatting completes; inspect the final diff.

```sh
run_cargo .review/go-type-extract/logs/t1-parity.log test --lib type_providers::go::extraction_
```
Expected: **7 passed, 0 failed** (predicted count from the seven test declarations, not an authorship execution). Both modes still perform old work; parity tests must pass before activating reuse.

```sh
run_cargo .review/go-type-extract/logs/t1-default.log test
```
Expected: full suite 0 failed; record exact total and delta from baseline.

```sh
run_cargo .review/go-type-extract/logs/t1-mcp.log test --features mcp
```
Expected: full feature suite 0 failed; existing provider-lifetime, navigation and output tests remain green.

```sh
run_cargo .review/go-type-extract/logs/t1-release.log build --release
```
Expected: current dirty task source rebuilt successfully, immediately before the next command.

```sh
(cd eval && uv run tier-a --matrix-only --allow-stale-sut) > .review/go-type-extract/logs/t1-matrix.log 2>&1
```
Expected: no regressions/flip candidates; failure remains a gate, never rebaseline.

```sh
cat .review/go-type-extract/logs/t1-matrix.log
```
Expected: read the actual matrix result and record it, including exclusions.

- [ ] **Step 6 — controller commit, after all gates.** Update the validation document with the missing-seam RED and seven-test GREEN, full-suite totals and matrix receipt. Snapshot logs and handoff.

```sh
git add src/type_providers/go.rs src/type_providers/go/signature_inputs.rs src/type_providers/go/extraction_equivalence_tests.rs src/type_providers/go/extraction_import_tests.rs src/type_providers/go/extraction_cost_tests.rs docs/superpowers/plans/2026-10-14-go-type-extract-speedup-validation.md
```
Expected: controller stages only Task 1 paths.

```sh
commit_task 'refactor(go): isolate per-file signature import inputs with parity oracle'
```
Expected: controller creates one commit with both required trailers; no push/amend.

### Task 2: Prove bounded scan work, then activate reuse

**Files:** create contents of `extraction_cost_tests.rs`; change only `FileSignatureImports::get`; update validation receipts. **Commit:** `perf(go): reuse signature imports within each file extraction`.

- [ ] **Step 1 — write the behavioral regression and zero-work edge test.**

Create `src/type_providers/go/extraction_cost_tests.rs`:

```rust
use super::extraction_equivalence_tests::files;
use super::*;

#[test]
fn repeated_signatures_scan_imports_once_per_file_not_once_per_signature() {
    for count in [1, 8, 32] {
        let mut source = "package p\ntype T struct{}\ntype I interface {I(int)}\n".to_string();
        for i in 0..count {
            source.push_str(&format!("func (T) M{i}(x int) int {{return x}}\n"));
        }
        let parsed = files(&[("a/t.go", &source), ("b/t.go", &source)]);
        for paths in [
            BTreeMap::new(),
            BTreeMap::from([
                ("a/t.go".into(), "example/a".into()),
                ("b/t.go".into(), "example/b".into()),
            ]),
        ] {
            let (old, old_count) = signature_inputs::measured(|| {
                GoTypeProvider::from_parsed_files_with_import_scan(
                    &parsed,
                    &paths,
                    true,
                    ImportScan::EverySignature,
                )
            });
            let (new, new_count) = signature_inputs::measured(|| {
                GoTypeProvider::from_parsed_files_inner(&parsed, &paths, true)
            });
            assert_eq!(format!("{:#?}", old.data), format!("{:#?}", new.data));
            assert_eq!(old_count, 2 * (count + 1));
            assert_eq!(new_count, 2, "one scan in each file extraction");
        }
    }
}

#[test]
fn files_without_signatures_and_non_go_files_do_not_scan() {
    let mut parsed = files(&[("a.go", "package a\ntype S struct{X int}\n")]);
    parsed.insert(
        "a.rs".into(),
        ParsedFile::parse("a.rs", "fn f() {}", Language::Rust).unwrap(),
    );
    for parsed in [BTreeMap::new(), parsed] {
        let (_, count) = signature_inputs::measured(|| GoTypeProvider::from_parsed_files(&parsed));
        assert_eq!(count, 0);
    }
}
```

- [ ] **Step 2 — observe the actual RED on Task 1's recompute body.**

```sh
run_cargo .review/go-type-extract/logs/t2-red.log test --lib type_providers::go::extraction_cost_tests
```
Expected: **1 passed, 1 failed**. At `count=1`, the failing assertion is `new_count == 2`, actual 4, proving the production constructor still rebuilds for every signature. Full-data equality and the old-count assertion must already pass. Any compiler failure is inadmissible as this regression proof; repair the test/seam first. This same-environment pre-fix run separates missing caching from parser/fixture/canonicalization problems.

- [ ] **Step 3 — GREEN.** Re-read the exact `get` body, then replace it with:

```rust
    pub(super) fn get(&self) -> Cow<'_, BTreeMap<String, String>> {
        match self.mode {
            ImportScan::Once => Cow::Borrowed(self.cached.get_or_init(|| self.scan())),
            #[cfg(test)]
            ImportScan::EverySignature => Cow::Owned(self.scan()),
        }
    }
```

Do not cache canonicalized signatures or alias expansion results: that would change telemetry counts and possibly profile-sensitive behavior. Do not store the map on `GoTypeData`, `ParsedFile`, or a global/path-keyed cache; one context is dropped at file extraction completion.

```sh
run_cargo .review/go-type-extract/logs/t2-green.log test --lib type_providers::go::extraction_
```
Expected: **9 passed, 0 failed**. For each two-file fixture, old signature scan counts are 4/18/66, new is exactly 2, and all full-data dumps match; the empty/non-Go/no-signature test records zero. These counts intentionally exclude the unchanged alias-resolver setup scan.

- [ ] **Step 4 — targeted mutation controls, one at a time, restoring exact saved bytes between them.** No timing threshold in a unit test. Record every mutant's output, then restore and rerun the nine tests.

| Mutant | Exact temporary change | Required failure |
|---|---|---|
| M1: remove reuse | Replace `get` with Task 1's `Cow::Owned(self.scan())` body | `repeated_signatures_scan_imports_once_per_file_not_once_per_signature`, actual 4 vs 2 at first case |
| M2: eager extra scan | Add `let _ = signature_imports.get();` immediately after `FileSignatureImports::new` in `extract_from_file` | `files_without_signatures_and_non_go_files_do_not_scan`, one scan instead of zero |
| M3: lose local identity | In the `ImportScan::Once` arm's initializer, temporarily call `crate::go_type_alias::signature_imports(self.parsed, None)` | import-aware full-data oracle / `plain_and_import_aware_constructors_must_remain_distinct` |
| M4: reorder writes | Temporarily change the constructor's file loop from `for (path, parsed) in files` to `for (path, parsed) in files.iter().rev()` | `last_map_write_and_first_method_winner_keep_path_and_source_order`; this pins order even though both oracle and candidate share the walker |

For each applied mutant, run (use M1/M2/M3/M4 as the log name, sequentially):

```sh
run_cargo .review/go-type-extract/logs/t2-mutant.log test --lib type_providers::go::extraction_
```
Expected: the table's named assertion fails with the stated mechanism; archive the log under its mutant ID before the next run. Compile failure, timeout, zero tests or environmental refusal does not kill a mutant. Re-read target bytes before each edit; snapshot/restore owned files, never use git stash/reset.

```sh
run_cargo .review/go-type-extract/logs/t2-restored.log test --lib type_providers::go::extraction_
```
Expected: 9 passed after exact restoration of the final implementation.

- [ ] **Step 5 — full gates and controller commit.**

```sh
run_cargo .review/go-type-extract/logs/t2-fmt.log fmt
```
Expected: formatted source; all four new Rust files remain below 600 lines.

```sh
run_cargo .review/go-type-extract/logs/t2-default.log test
```
Expected: full default suite 0 failed; record actual totals.

```sh
run_cargo .review/go-type-extract/logs/t2-mcp.log test --features mcp
```
Expected: full feature suite 0 failed; record actual totals and unchanged goldens.

```sh
run_cargo .review/go-type-extract/logs/t2-release.log build --release
```
Expected: candidate immediately rebuilt before the next command.

```sh
(cd eval && uv run tier-a --matrix-only --allow-stale-sut) > .review/go-type-extract/logs/t2-matrix.log 2>&1
```
Expected: no regressions/flip candidates; report failures without rebaseline.

```sh
cat .review/go-type-extract/logs/t2-matrix.log
```
Expected: inspect actual matrix result.

```sh
git add src/type_providers/go/signature_inputs.rs src/type_providers/go/extraction_cost_tests.rs docs/superpowers/plans/2026-10-14-go-type-extract-speedup-validation.md
```
Expected: controller stages final cache body, two cost tests and receipts; no mutant remains.

```sh
commit_task 'perf(go): reuse signature imports within each file extraction'
```
Expected: controller creates the Task 2 commit with both trailers.

### Task 3: Close equivalence, full-suite and review evidence

**Files:** validation document only unless a demonstrated defect requires a bounded T2 correction. **Commit:** `docs(go): record extraction parity and verification receipts`.

- [ ] **Step 1 — evidence RED/GREEN, not another implementation.** RED means a missing/failed prerequisite remains open; GREEN means actual T1/T2 full-suite receipts, nine-test GREEN, four killed targeted mutants and a clean final diff are recorded. The seven parity tests intentionally pass before the optimization; M1 and the watched scan-count RED establish the performance behavior. Keep this distinction in the PR. Inspect each field in the table against the diff. Full `GoTypeData` identity plus unchanged deterministic consumers is the equivalence argument; neither timing nor matching cardinalities alone proves output bytes.
- [ ] **Step 2 — output coverage.** Read T2's full-suite logs and identify executed `lang_go`, `navigation`, `cli`, `output`, and `mcp` targets, including the MCP golden transcript test. The final full default and MCP runs must execute all tests, not just provider tests. Ensure `PRISM_MCP_BLESS_GOLDEN` is unset for all feature runs and no golden file changed. If it was set, rerun the feature suite unblessed before crediting that receipt. The `output` target alone is not the SARIF/targets oracle: inspect the CLI/API integration tests inside the full suite too. Run no bless/rebaseline command.

```sh
git diff 8b2744b3154b3664ffacb23c42f042a67981967a -- src/ast.rs src/go_type_alias.rs src/go_type_alias src/call_graph.rs src/cpg src/cpg_cache.rs src/navigation tests/fixtures Cargo.toml Cargo.lock
```
Expected: no output; downstream consumers, serialization, mutation populations, dependencies and golden artifacts unchanged. If this fails, this narrow plan's proof no longer applies; examine the actual delta.

```sh
run_cargo .review/go-type-extract/logs/review-release.log build --release
```
Expected: current worktree rebuilt immediately before the quick gate.

```sh
(cd eval && uv run tier-a --quick --allow-stale-sut) > .review/go-type-extract/logs/review-quick.log 2>&1
```
Expected: no new regressions/flip candidates; needs rust-analyzer. Unavailable host/LSP/uv resources are an explicit open gate for the controller, not a passed check.

```sh
cat .review/go-type-extract/logs/review-quick.log
```
Expected: record actual quick totals and paste every regression/flip candidate into PR review material.

For **each** review round, after the candidate is fixed and before dispatch:

```sh
env PATH="$PWD/.review/go-type-extract/tools:$PATH" PRISM_REAL_CARGO="$PRISM_REAL_CARGO" python3 scripts/mutgate/mutgate.py --since main --scope fn --jobs 1 > .review/go-type-extract/logs/mutgate-scoped.log 2>&1
```
Expected: admissible scoped report; record selected IDs and verdict. A zero selection is reported as no coverage of this change; it does not replace M1–M4 or the full gate.

```sh
cat .review/go-type-extract/logs/mutgate-scoped.log
```
Expected: read baseline admission, selected population and results; archive with round number.

- [ ] **Step 3 — controller review and commit.** Dispatch Opus 5.5 and gpt-6-astra with the two-round cap already stated. Include the complete field/ordering audit, explicit constructor-difference test, measured versus guessed performance, O1 disposition, full-suite totals, all exclusions and Tier-A deltas. No reviewers are dispatched during authorship. No source/test edit after a passed gate inherits that gate automatically.

```sh
git add docs/superpowers/plans/2026-10-14-go-type-extract-speedup-validation.md
```
Expected: controller stages evidence only.

```sh
commit_task 'docs(go): record extraction parity and verification receipts'
```
Expected: controller creates the Task 3 evidence commit with both trailers. Documentation-only changes need no invented RED test; source verification receipts bind to the unchanged source tree.

### Task 4: Prepare paired probe binaries; controller measures acceptance

**Files:** probe-only worktrees and archived evidence; validation document on the implementation branch. **Commit:** `docs(go): record paired cache-hit and cold extraction acceptance`. **Roles:** controller makes clean worktrees/commits and performs measurements; implementer prepares the two binaries and verifies their source/instrumentation manifests. Never run prism-mcp on the real corpora as implementer.

**Pre-registered readings — freeze this section before any new real-corpus run:**

| Reading | Success | Failure / inconclusive interpretation |
|---|---|---|
| Admission | Every warm run has driver `status=ok`, report `cache.cpg_hit=true`, `call_edges_loaded=true`, and probe meta `cpg_hit=true`; one cold prime and two warm processes per corpus/version. Same corpus commits/workload hashes, same driver/env, clean build identities, same instrumentation, no concurrent cargo/prism-mcp. | Miss, partial hit, dirty-sidecar rejection, timeout, absent stage, drifted input or overlapping measurement: inadmissible; no speed claim. |
| Prometheus cache hit | Both candidate index intervals ≤4.0 s and mean candidate index time ≤40% of the paired base mean. Candidate mean `go.file_extract` wall and CPU each ≤25% of paired base means. | >4.0 s on either repetition: goal not met. 4–5 s is improvement, not “well under 5”. Falling wall without corresponding extraction CPU/count evidence is insufficient causal attribution. |
| Cold Prometheus | The driver's cold primes contain two distinct Go extractions on both versions. Sum of both candidate `go.file_extract` wall times ≤50% of base sum, and CPU sum ≤50% too. Report setup and satisfaction separately; final provider census/semantics unchanged. | Two calls remain by design; count alone is not failure. Failing the halving threshold means the cold goal remains open. Cold end-to-end speedup is informative, not promised: DFG/assembly dominate other intervals. |
| Slicing control | Mean candidate index ≤max(1.10×base mean, base mean +0.20 s), and no semantic/count changes. | A larger increase is a control regression or environment confound, not an automatic Go optimization defect. Same-environment repeat/control needed. |
| Memory | Candidate warm live heap and peak footprint no more than 5% above paired base, accounting for run variance; report absolute values as well. | Larger change requires attribution. A per-file map should not survive extraction; retained heap growth is not traded for speed without owner review. |
| Equivalence | Nine full-data tests and all full suites/Tier-A/goldens pass; probe's type/call-graph counters agree with their corresponding constructor on base and candidate. | Any changed full-data dump, alias telemetry, winner, or output byte is failure even if performance improves. Do not compare the plain constructor's counters with the import-aware constructor's counters. |

These thresholds are **chosen acceptance criteria**, not measurements. Two repetitions are a bounded acceptance sample, not a statistical generalization. Cap: one paired run set, plus **one** targeted repeat set only for an explicitly identified admission/host confound, then classify and report. A valid slower result is not grounds for indefinite retries. No automatic parallelization or shared-final-provider fallback is authorized by a missed target.

- [ ] **Step 1 — bind base/candidate and historical instrumentation.** From the implementation worktree, after the controller committed the candidate:

```sh
export GO_SPEEDUP_REPO="$PWD"
```
Expected: absolute implementation checkout path.

```sh
export GO_SPEEDUP_CANDIDATE="$(git rev-parse HEAD)"
```
Expected: clean committed candidate source including T1/T2; record SHA in the validation document. Do not use a dirty working copy's HEAD as if it contained its edits.

```sh
export GO_SPEEDUP_EVIDENCE=/private/tmp/prism-go-extract-acceptance-2026-10-14
```
Expected: dedicated evidence/worktree root name.

```sh
test ! -e "$GO_SPEEDUP_EVIDENCE"
```
Expected: path absent. If present, preserve it and have the controller assign a new unique evidence root; do not delete prior evidence.

```sh
mkdir -p "$GO_SPEEDUP_EVIDENCE/bin" "$GO_SPEEDUP_EVIDENCE/logs"
```
Expected: fresh evidence root, owned by this measurement.

```sh
git rev-parse scratch-opt-probe
```
Expected: `c3162f9419f371fa4333ef0f038b48845691e310`. The historical probe binary was built on older main; it is **not** the acceptance baseline binary. Transplant its instrumentation onto this plan's base and candidate.

Controller-only worktree/commit operations:

```sh
git worktree add -b scratch/go-extract-acceptance-base "$GO_SPEEDUP_EVIDENCE/base" 8b2744b3154b3664ffacb23c42f042a67981967a
```
Expected: clean worktree at the reviewed pre-change source; no source checkout/reset in the implementation lane.

```sh
git worktree add -b scratch/go-extract-acceptance-candidate "$GO_SPEEDUP_EVIDENCE/candidate" "$GO_SPEEDUP_CANDIDATE"
```
Expected: clean worktree at the committed candidate.

```sh
git -C "$GO_SPEEDUP_EVIDENCE/base" cherry-pick c3162f9419f371fa4333ef0f038b48845691e310
```
Expected: probe instrumentation only, committed on base scratch branch. On conflict, stop and enumerate every conflicting hunk before one targeted resolution; preserve source behavior and named timers, do not discard the artifact.

```sh
git -C "$GO_SPEEDUP_EVIDENCE/candidate" cherry-pick c3162f9419f371fa4333ef0f038b48845691e310
```
Expected: corresponding instrumentation on candidate. If constructor plumbing shifted, place `go.profile_alias_extract` around profiles/alias setup and `go.file_extract` around the entire file loop and telemetry assignment, exactly as historical probe intent; keep satisfaction counters and other stages. Both scratch commits carry the required trailers (the historical commit already has them). Do not merge instrumentation.

```sh
git diff --no-index "$GO_SPEEDUP_EVIDENCE/base/src/probe.rs" "$GO_SPEEDUP_EVIDENCE/candidate/src/probe.rs"
```
Expected: no differences in timer implementation. Also review the two instrumentation commit diffs: same stage scopes and census semantics; stage names present once per corresponding constructor invocation, no candidate-only timer omissions. A conflict resolution changing logic invalidates the binary pair until corrected.

- [ ] **Step 2 — implementer builds sequentially, no corpus process.** Source/probe worktrees must be tracked-clean before builds. In particular, dirty-tree build metadata can prevent warm sidecar admission (`scripts/memreport/memreport.py:37-46`). Record source SHA, probe SHA, compiler version, binary SHA-256 and source-tree status in the evidence manifest.

```sh
(cd "$GO_SPEEDUP_EVIDENCE/base" && nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --release --features mcp --bin prism-mcp) > "$GO_SPEEDUP_EVIDENCE/logs/base-build.log" 2>&1
```
Expected: successful clean instrumented base binary; do not start candidate build before completion.

```sh
cat "$GO_SPEEDUP_EVIDENCE/logs/base-build.log"
```
Expected: inspect all compiler/priority diagnostics; controller corrects any denied host setup.

```sh
cp "$GO_SPEEDUP_EVIDENCE/base/target/release/prism-mcp" "$GO_SPEEDUP_EVIDENCE/bin/base-prism-mcp"
```
Expected: retained base executable from that build.

```sh
(cd "$GO_SPEEDUP_EVIDENCE/candidate" && nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --release --features mcp --bin prism-mcp) > "$GO_SPEEDUP_EVIDENCE/logs/candidate-build.log" 2>&1
```
Expected: successful clean instrumented candidate binary, same host/toolchain/options.

```sh
cat "$GO_SPEEDUP_EVIDENCE/logs/candidate-build.log"
```
Expected: inspect compiler/priority diagnostics.

```sh
cp "$GO_SPEEDUP_EVIDENCE/candidate/target/release/prism-mcp" "$GO_SPEEDUP_EVIDENCE/bin/candidate-prism-mcp"
```
Expected: retained candidate executable.

```sh
shasum -a 256 "$GO_SPEEDUP_EVIDENCE/bin/base-prism-mcp" "$GO_SPEEDUP_EVIDENCE/bin/candidate-prism-mcp" > "$GO_SPEEDUP_EVIDENCE/bin/SHA256SUMS"
```
Expected: two hashes bound to the recorded builds.

```sh
cat "$GO_SPEEDUP_EVIDENCE/bin/SHA256SUMS"
```
Expected: two readable records; hand these, manifests and binaries to the controller. Implementer stops here for real-corpus execution.

- [ ] **Step 3 — controller-only paired measurement.** Read the inherited procedure and `run-probe.sh` under `~/prism-evidence/mcp-memory/optimization-2026-10-14/`; retain its instrumentation/summarizer but use the newly built pair. Unlike the historical script, do not proceed under a known competing build. Controller verifies process inventory and records load/swap at every run boundary. If the host cannot become comparable, report the gate open.

```sh
pgrep -fl 'prism-mcp|cargo|rustc|nextest|mutgate|memreport.py'
```
Expected: controller has process visibility and no competing index/build/measurement; unrelated resident daemons are recorded and held constant or scheduled around. Do not kill another session's processes.

```sh
unset RAYON_NUM_THREADS PRISM_MCP_BLESS_GOLDEN
```
Expected: historical default pool settings and unblessed outputs; record this environment choice. Keep remaining relevant environment identical between base and candidate.

Run sequentially in this exact order: base Prometheus, candidate Prometheus, base slicing, candidate slicing. Each `--out` is fresh, so each version primes its own cache. Do not compare cache bytes directly: build identity differs. Each command below entails one cold priming process followed by two warm processes, never concurrent prism-mcp instances.

```sh
PRISM_PROBE_OUT="$GO_SPEEDUP_EVIDENCE/base-prometheus.jsonl" python3 scripts/memreport/memreport.py run --bin "$GO_SPEEDUP_EVIDENCE/bin/base-prism-mcp" --corpus "prometheus=$HOME/code/bench-repos/prometheus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$GO_SPEEDUP_EVIDENCE/run-base-prometheus" > "$GO_SPEEDUP_EVIDENCE/logs/base-prometheus.log" 2>&1
```
Expected: one prime and two admissible true cache hits. Record host state before proceeding.

```sh
cat "$GO_SPEEDUP_EVIDENCE/logs/base-prometheus.log"
```
Expected: successful driver run; inspect raw reports too, not just process status.

```sh
PRISM_PROBE_OUT="$GO_SPEEDUP_EVIDENCE/candidate-prometheus.jsonl" python3 scripts/memreport/memreport.py run --bin "$GO_SPEEDUP_EVIDENCE/bin/candidate-prism-mcp" --corpus "prometheus=$HOME/code/bench-repos/prometheus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$GO_SPEEDUP_EVIDENCE/run-candidate-prometheus" > "$GO_SPEEDUP_EVIDENCE/logs/candidate-prometheus.log" 2>&1
```
Expected: same inputs/env, one prime plus two valid hits; judge against pre-registered thresholds only after admission.

```sh
cat "$GO_SPEEDUP_EVIDENCE/logs/candidate-prometheus.log"
```
Expected: inspect driver and raw report admission; record host state.

```sh
PRISM_PROBE_OUT="$GO_SPEEDUP_EVIDENCE/base-slicing.jsonl" python3 scripts/memreport/memreport.py run --bin "$GO_SPEEDUP_EVIDENCE/bin/base-prism-mcp" --corpus "slicing=$HOME/code/prism-c1-corpus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$GO_SPEEDUP_EVIDENCE/run-base-slicing" > "$GO_SPEEDUP_EVIDENCE/logs/base-slicing.log" 2>&1
```
Expected: Rust control baseline, one prime and two valid hits.

```sh
cat "$GO_SPEEDUP_EVIDENCE/logs/base-slicing.log"
```
Expected: inspect admission and record host state.

```sh
PRISM_PROBE_OUT="$GO_SPEEDUP_EVIDENCE/candidate-slicing.jsonl" python3 scripts/memreport/memreport.py run --bin "$GO_SPEEDUP_EVIDENCE/bin/candidate-prism-mcp" --corpus "slicing=$HOME/code/prism-c1-corpus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$GO_SPEEDUP_EVIDENCE/run-candidate-slicing" > "$GO_SPEEDUP_EVIDENCE/logs/candidate-slicing.log" 2>&1
```
Expected: same Rust corpus and no control regression beyond the registered tolerance.

```sh
cat "$GO_SPEEDUP_EVIDENCE/logs/candidate-slicing.log"
```
Expected: inspect admission and record final host state.

```sh
python3 "$HOME/prism-evidence/mcp-memory/optimization-2026-10-14/summarize_probe.py" "$GO_SPEEDUP_EVIDENCE/base-prometheus.jsonl" "$GO_SPEEDUP_EVIDENCE/candidate-prometheus.jsonl" "$GO_SPEEDUP_EVIDENCE/base-slicing.jsonl" "$GO_SPEEDUP_EVIDENCE/candidate-slicing.jsonl" > "$GO_SPEEDUP_EVIDENCE/probe-summary.md"
```
Expected: twelve process sections total: four primes and eight cache hits; two extraction spans per Go prime and one per Go warm hit. A slicing run with no Go files need not have Go stage spans.

```sh
cat "$GO_SPEEDUP_EVIDENCE/probe-summary.md"
```
Expected: derive sums/means from the admitted raw records and compare every row of the registered table. Save the exact report index intervals separately; stage totals are not a substitute for them.

`memreport.py --kinds warm` is a performance/cache-admission experiment, **not** an output-transcript comparison: `command_for` only supplies `--mem-workload` for kind `workload` (`scripts/memreport/memreport.py:63-71`). Do not claim its matching counters prove byte identity. Equivalence is held by the full-data oracle, direct ordering/constructor tests, unchanged downstream code, full suites, goldens and Tier-A. Any desired new real-corpus query transcript experiment is a separately bounded controller follow-up, not silently substituted for these mandated runs.

- [ ] **Step 4 — authoritative gate and acceptance commit.** Back on the implementation checkout, with no prism-mcp or other cargo running:

```sh
env PATH="$PWD/.review/go-type-extract/tools:$PATH" PRISM_REAL_CARGO="$PRISM_REAL_CARGO" python3 scripts/mutgate/mutgate.py --jobs 1 > .review/go-type-extract/logs/mutgate-full.log 2>&1
```
Expected: the complete authoritative full population, admitted unmutated baseline tests and all required mutants killed. Scoped GREEN is insufficient. Report absent Go-manifest coverage beside the T2 targeted mutation receipts.

```sh
cat .review/go-type-extract/logs/mutgate-full.log
```
Expected: read complete gate totals/results; any invalid baseline or surviving mutant remains open, with same-environment attribution before blaming this change.

Record base/candidate source/probe/binary identities, corpus hashes, every per-repetition index/stage wall/CPU measurement, both cold extraction sums, memory/control deltas, admission flags, host states, all suite/gate totals and the final PASS/FAIL/INCONCLUSIVE for each criterion. Do not invent numbers while waiting for the controller. Archive raw evidence under the controller's durable evidence store and link it from the validation document and handoff. Reconcile earlier “pending” claims in that document when results arrive.

```sh
git add docs/superpowers/plans/2026-10-14-go-type-extract-speedup-validation.md
```
Expected: controller stages the actual acceptance receipt, with failures/exclusions retained.

```sh
commit_task 'docs(go): record paired cache-hit and cold extraction acceptance'
```
Expected: evidence commit with both trailers. Merge remains controller-owned and requires accepted review, full gates and all acceptance criteria. If any criterion fails, report the bounded next diagnostic; do not claim the target achieved.

## Owner Questions and Final Handoff

**O1 — existing oversized file: RESOLVED by the controller from a standing owner rule.** The owner dropped numeric line caps on 2026-09-30 ("No LOC caps": keep slice-size forecasting and splitting, no numeric src/test caps). Narrow edits to the existing `go.rs` (3,596 lines) are allowed; every new file stays focused (the plan's new files are under 600 lines anyway); no mechanical split of `go.rs` in this PR.

**No other owner decision is needed for the proposed fix.** The 4.0-second interpretation of “well under 5”, ≤50% combined cold extraction time, control/memory tolerances and one-repeat cap are specified defaults that the owner may revise **before** the controller starts measurements.

Final implementer report must state: exact changed files and why; actual default/MCP suite totals; nine-test and mutation results; matrix/quick results and exclusions; base/candidate acceptance table; still-distinct cold constructor inputs; no serialized/output changes; O1 disposition; controller commit/probe identities and durable evidence location. Mark the performance goal unverified until T4 produces admissible results. No Linux claim is made without Linux execution.

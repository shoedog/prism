> **Status: SHIPPED — PR [#157](https://github.com/shoedog/prism/pull/157), merged 2026-07-03 (main e959f00).** As-executed brief incl. folded codex spec-review corrections (BoundaryExited mandate, per-probe schema rules, deterministic triage strings). As-shipped deltas beyond this text (two review waves): taint probes hard-fail on absent `reasoning` (never null-equals-None) + `frontier_count_min` assertion (type-strict `type() is int`, load-time >=1); per-probe section AND [expect]-key whitelists with named errors + >=1-assertion requirement + load-time reachability-value validation; seed normalization extracted to ungated `reasoning::seeds::{normalize_loc_seed,parse_file_line_spec}` shared by CLI and MCP. Fixtures pin CURRENT truth: boundary=BoundaryExited (P14 flip target), sanitized=Reached+Cleansed (P10 flip target). Matrix 79/79 at merge. No cache bumps.

# Task P6(b)(c) — taint_reaches fixtures + module-edge contracts (measurement remainder)

You work in the git worktree `/private/tmp/prism-p6bc-measurement` on branch `p6bc-taint-module-contracts` (based on main @ 6a47eba). The repo is prism (Rust CPG engine + Python Tier-A eval harness in eval/). Follow TDD. P6(a) (confidence-stratified M2) shipped in PR #151; this task lands the remaining measurement surfaces: **(b)** by-construction `taint_reaches` fixtures and **(c)** module-edge contracts. These become the gates for the future P10 (sanitizer verdicts) and P14 (interprocedural taint) items — the fixtures pin CURRENT truthful behavior; P10/P14 will flip expectations later.

## Ground truth (all verified)

- `taint_reaches` has **no CLI**: `NavQuery` (src/main.rs:245-338) has `NodesAt, Callers, Callees, Ego, ModuleDeps, RepoMap, CallStats, InterfaceManifest, Functions` only. It is MCP-only: src/mcp/tools_reasoning.rs:52-56 calls `crate::reasoning::taint_reaches::taint_reaches(ctx.session, &sources, sinks.as_deref())` with `SeedSpec` seeds parsed from `{name,file}` | `{file,line}` inputs.
- Reasoning entry: `pub fn taint_reaches(session: &NavigationSession, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>) -> Result<Evidence, QueryError>` (src/reasoning/taint_reaches.rs:33-37). `sinks: None` → frontier mode; `Some` → witness mode.
- Output: `Evidence` (src/navigation/types.rs:138-149) with `reasoning: Option<ReasoningSummary>` — `reachability` serializes `"Reached" | "NotReached" | "BoundaryExited" | null` (src/reasoning/types.rs:172-181), `per_sink[]` `SinkResult{sink, reachability, sources, sources_omitted}` (:164-170), per-source `SinkSourceResult{source, reachability, graph_node, sanitizers_present_in_source_fn}` (:91-99). Warnings: `WarningKind::Reasoning(ReasoningWarning::Cleansed{source_function})` (types.rs:38-40).
- **Sanitizer behavior TODAY** (src/reasoning/taint_reaches.rs:184-198): reachability computed independently; a sanitizer in the source fn only pushes the `Cleansed` warning; `reachability` stays `Reached`. NO test currently pins Cleansed+Reached together. P10 will make this verdict-affecting — the fixture pins the CURRENT truth and gets flipped by P10.
- Matrix harness: `_run_matrix_inner` (eval/tier_a/matrix.py:87-95) hardcodes `sut.callers(...)` per fixture; discovery = `(fixtures_root/<lang>).glob("*/expected.toml")` over `MATRIX_LANGUAGES` (matrix.py:13). Schema (`load_case` matrix.py:48-68): `[case]{language,capability,status}`, `[seed]{symbol,file,line}`, `[[expect.callers]]{file,line}`, `[expect]{exact(default true),resolution_kind,forbid_resolution_kind}`. NO probe/mode selector key exists. Outcome mapping matrix.py:114-117 (pass→ok/regression; else→flip_candidate/expected_gap). Run-report serializer: `matrix_result_to_json` (eval/tier_a/cli.py:448-461). `--matrix-only` fast path (cli.py:781-787) builds only `PrismCli` — shells `target/release/prism nav ... --format json` (eval/tier_a/sut.py:139,187) — NO LSP; any new fixture type MUST stay a prism-CLI shell-out to keep this gate seconds-fast.
- module-deps/repo-map already have CLI + stable JSON: handlers src/main.rs:460-471 → `module_graph::{module_deps,repo_map}` → `Evidence`. module-deps: `query="module-deps:<file>"`, `items[].location.file` = target file, `why[]` variants `Calls/Resolution/ResolvedImport/UnresolvedImport`, `graph` omitted (module_graph.rs:361). repo-map: `items=[]`, `graph.edges[]{from,to,kind:"ModuleDep"}` (module_graph.rs:386-410). Golden tests exist: tests/cli/nav_compat_test.rs:419 (`module_deps_golden`), :447 (`repo_map_golden`).
- Existing library-level taint test pattern to mirror: tests/reasoning/taint_reaches_test.rs (tempfile repo → NavigationSession → direct call → assert `evidence.reasoning.reachability` + warnings).
- Pinned probes (eval/tier_a/pinned.py:9-22) are oracle-dependent caller probes that run ONLY on the `prism` corpus in full runs (cli.py:675-679) — NOT the fast gate. **Deviation from the plan text (which suggested pinned.py:13-18 for module contracts): put module-edge contracts in MATRIX fixtures instead** — by-construction, deterministic, and they run in the seconds-fast `--matrix-only` gate. Note this deviation in the PR description.

## Changes

### 1. `prism nav taint-reaches` CLI subcommand (Rust, additive — the enabler for (b))

Add `TaintReaches` to `NavQuery` (src/main.rs): `--repo <dir>`, repeatable `--source <file:line>`, repeatable optional `--sink <file:line>`, `--format json|text`. Parse seeds to the same `SeedSpec::Loc` form the MCP path builds (reuse/extract the MCP seed-parsing helper from src/mcp/tools_reasoning.rs:43-51 if cleanly shareable WITHOUT enabling the mcp feature in default builds — the reasoning layer is NOT feature-gated, only the MCP adapter is; if the helper lives behind the `mcp` feature, write a small parallel parser in main.rs and note it). Handler mirrors the ModuleDeps handler shape (src/main.rs:460-471): build session, call `reasoning::taint_reaches`, render via `prism::output::navigation::render(&ev, format)`. Symbol-form seeds (`--source-symbol name --source-file f`) are OPTIONAL — include only if trivial; location seeds suffice for fixtures.
- NO cache version bumps (no serialized-state change).
- Tests: a `tests/cli/` test (assert_cmd, temp repo) asserting `reasoning.reachability` appears in `--format json` output for a trivial same-function source→sink; an error-path test (malformed `--source` spec).

### 2. Matrix fixture-type selector (Python harness)

- `[case] probe = "callers" | "taint" | "module_deps"` (default `"callers"` — absent key means callers; ALL 73 existing fixtures must load and run byte-identically). Add the field to `Case` (matrix.py:16-29) + `load_case`, and dispatch in `_run_matrix_inner` (matrix.py:87-95). **Schema validation (spec-review): `[seed]` is currently mandatory for every case (matrix.py:48) — make it required ONLY for `callers` probes; `taint` probes require `[taint]` (and forbid `[seed]`/`[[expect.callers]]`); `module_deps` probes require `[module]` (and forbid the same); unknown `probe` values and mixed schemas are explicit load errors with a clear message.**
- New SUT methods on `PrismCli` (eval/tier_a/sut.py, beside `callers` :209): `taint_reaches(corpus_root, sources, sinks)` shelling `prism nav taint-reaches --repo ... --source f:l [--sink f:l] --format json`; `module_deps(corpus_root, file)` shelling the existing `nav module-deps`.
- Taint fixture schema (new sections; validate + reject unknown combos in `load_case`):
  - `[taint] sources = ["app.py:3"]`, optional `sinks = ["app.py:9"]`.
  - `[expect] reachability = "Reached" | "NotReached" | "BoundaryExited"`; optional `warning_kinds_present = ["Cleansed"]` (subset match on warnings[].kind discriminant); optional `sanitizers_present = true` (any per-source `sanitizers_present_in_source_fn` non-empty).
- Module fixture schema: `[module] file = "a.py"`; `[[expect.module_edges]] to = "b.py"` (assert `items[].location.file` contains each `to`; `exact = true` means set equality on target files, default subset). Optionally `forbid_to = [...]`.
- Outcome mapping: reuse ok/regression/flip_candidate/expected_gap semantics keyed off `[case] status` exactly as today (matrix.py:114-117). Extend `matrix_result_to_json` (cli.py:448-461) generically — `got`/`expected` stay strings, but they must be DETERMINISTIC and triage-useful (spec-review): taint → reachability + sorted warning discriminants + `sanitizers_present` (e.g. `"BoundaryExited|warnings=Boundary|sanitizers=false"`); module → sorted target-file list. A regression line must tell the reader what changed without re-running.
- Python-side tests in eval/tests/ mirroring test_matrix.py patterns: loader accepts/rejects schemas; a fake-SUT dispatch test per probe type; existing tests stay green (`uv run pytest -q --ignore=adoption`).

### 3. Fixtures (the deliverable)

**(b) taint — `eval/fixtures/python/` (Python: best-supported taint language):**
- `taint_reach_positive/`: source var at line L flows to a sink call in the SAME function → `reachability = "Reached"` (witness mode: give both source and sink).
- `taint_boundary_negative/`: source in `f`, sink only reachable through a call into `g` (cross-function) → **`reachability = "BoundaryExited"` plus the boundary warning** (spec-review confirmed the contract from code: cross-function traversal records the interprocedural boundary at src/cpg/trace.rs:317, shape returns `BoundaryExited` at src/reasoning/shape.rs:99, pinned by tests/reasoning/taint_reaches_test.rs:343). Fixture comment: P14 flips this to `Reached` when interprocedural descent lands.
- `taint_sanitized_current/`: source fn contains a recognized sanitizer (pick one from `cleansed_categories_for_source`, src/algorithms/taint.rs:10703 — verify which sanitizer names Python detection actually matches, e.g. `html.escape`) + sink in same function → `reachability = "Reached"` AND `warning_kinds_present = ["Cleansed"]` AND `sanitizers_present = true`. Comment: P10 flips the reachability expectation.
- One frontier-mode fixture (source only, no sinks): assert it runs and reachability is null/omitted per current behavior (verify empirically; pin truthfully).

**(c) module — one per language where module-deps support is real (verify with the binary; at minimum python + rust):**
- `eval/fixtures/python/module_edges_basic/`: `a.py` imports `b.py`, seed file `a.py` → edge to `b.py`, `exact` per actual output (b + stdlib noise? verify — if stdlib/externals appear, use subset).
- `eval/fixtures/rust/module_edges_basic/`: `main.rs` with `mod util;` + call into `util.rs` → edge to `util.rs`.

**Baseline note:** docs/eval/tier-a/baseline.md's matrix counts (e.g. ":250 33 ok") are already stale from P3–P7 fixture additions and are regenerated only by human-triggered full runs — do NOT regenerate; add one line to the PR description noting the count drift.

## Constraints (binding)

- `--matrix-only` stays seconds-fast and LSP-free (everything shells the release binary).
- All 73 existing fixtures byte-untouched and passing; existing matrix/pinned Python tests green.
- Determinism: fixture assertions must not depend on ordering beyond what the harness already sorts.
- Rust: no new warnings, `cargo fmt`, files <600 lines. The CLI addition must not change any existing `nav` subcommand's output (nav_compat goldens are byte-pinned — run `cargo test --test cli`).
- Python harness code follows the existing style (dataclasses, no new deps).
- Commit trailer: `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`

## Done-checks (run and paste)

```
cargo build --release && cargo test                        # full suite green incl. new cli test
cargo test --test cli                                      # nav_compat goldens byte-identical
cd eval && uv run pytest -q --ignore=adoption              # harness tests green (516+ new)
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # ALL fixtures ok incl. new taint/module ones; 0 regressions
./target/release/prism nav taint-reaches --repo <fixture dir> --source app.py:3 --sink app.py:9 --format json | head -20   # paste
./target/release/prism nav module-deps --repo <fixture dir> --file a.py --format json | head -20                          # paste
```

## Commit style
Small logical commits: CLI subcommand / harness probe-dispatch / taint fixtures / module fixtures. End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>

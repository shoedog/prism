# Lane 8: one taint answer from both engines — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** every difference between review taint and the trace engine behind `taint_reaches` disappears or
carries a named, tested reason; the trace engine crosses the three hop classes of DESIGN 5.2; both engines
take the same crossing knobs; `taint_reaches` gains a sink catalog mode, first in the CLI, then in MCP.

**Architecture:** the descent gate moves from `src/cpg/trace.rs` to `src/cpg/trace_descent.rs` and returns
a reason instead of a boolean; the reason rides on `BoundaryEdge` into the reasoning warnings. A descent
policy (ceiling, depth) is threaded beside PR #841's crossing policy. A `SinkCatalog` trait in core is
implemented in the review crate over the existing classifier and structured tables, and passed in.

**Tech stack:** Rust; Python 3 for census scripts; the SecBench harness (`eval/secbench`). **Spec:**
`DESIGN.md` in this directory (5.2 to 5.6 rules, 7 gates, 8 owner decisions).

## Global constraints

- **Start:** PR #841 merged (DESIGN decision 1). First check that `src/cpg/taint_policy.rs` still exports
  `TaintCrossingPolicy` and `CrossingCeiling { Exact, NameOnly }`, and that `taint_trace_from_root` takes
  `policy: Option<&TaintCrossingPolicy<'_>>`, as at 493ab5ee. If not: I-5 defines its own
  `DescentCeiling { Exact, NameOnly }`, and `policy_ceiling` exists only if a crossing policy still reaches
  the descent test. Record the outcome in the ledger.
- Worktree `~/code/prism-taint-parity` (sparse, without `eval/secbench`). Measurement checkout
  `~/code/prism-taint-parity-bench` (full), used only by the census runner (a Claude seat: the benchmark is
  a security corpus, so no codex seat). Evidence `~/prism-evidence/taint-parity/`.
- **Defaults unchanged** unless a row says otherwise: default review taint findings byte-identical on
  population A after every slice (gate Defaults); `taint_reaches` with default knobs moves only where a row of
  the slice's group says so.
- **File size** (weighted): `src/cpg/trace.rs` is 1,113.5 on main plus #841's lines and must not grow (new
  logic in `trace_descent.rs`). `crates/prism-review/src/main.rs` is 990.5: I-5 moves the `nav taint-reaches`
  handler to a new `crates/prism-review/src/nav_taint.rs` (`mod nav_taint;` in `main.rs`).
  `src/reasoning/taint_reaches.rs` is 583: catalog mode goes in `src/reasoning/sink_catalog.rs`.
- **MCP goldens** (`tests/fixtures/mcp/golden/*.jsonl`) hold `taint_reaches`'s schema and description. Only
  I-5 and I-9 re-record them: `PRISM_MCP_BLESS_GOLDEN=1 cargo test -p prism-mcp --test mcp
  golden_transcript_test`, with the reason in the commit message.
- **Tier-A** (ORCHESTRATION 5 item 5) for every slice touching `src/cpg/` (I-1 to I-5; I-6 if it does). No
  slice changes stored CPG or nav data (one that does bumps `CACHE_VERSION`). Core never names
  `prism_review`; engine-parity assertions live in `tests/algo/taxonomy/`.

## Review focus

1. One site of the callee on the line, source Use carrying the bytes of an earlier occurrence outside it
   (`log(t); g(t)`): descends today by line, must still descend. Row `l01`, I-same-line, `parity_taint_reaches`.
2. A binding edge ending at a Use that is not a read of the target's formal (a read in a callable nested in
   the callee): refused, `target_not_binding`. Row `b05`, I-binding, `parity_taint_reaches`.
3. An aliased call resolving to one of two same-named functions in different files: descent reaches only
   the resolved one. Row `s05`, I-spelling, `aliased_site_descends_only_into_resolved_target`.
4. Equal knobs, equal answers under `(exact,2)`, `(nameonly,2)`, `(exact,3)` on every row. I-5,
   `parity_knobs_agree`.
5. Catalog operands when the graph keeps one Use per path and line (`log(x); exec(x)`; `exec(y, x)` with
   only `x` tainted): catalog mode answers as review does. Rows `c03`, `c04`, I-8, `parity_catalog`.

## Stage M: issue and parity census (3 days)

### Task M-1: workspace, issue, binaries

- [ ] Worktrees and evidence directory (ORCHESTRATION 4). `memguard.sh 8 cargo build --release`; copy to
      `~/prism-evidence/taint-parity/bin/prism-base`; record `--version`.
- [ ] File the tracking issue "Lane 8: one taint answer from both engines (trace parity, sink catalog)",
      linking `DESIGN.md`, this plan, #602 (parent), #596 (lane 3's F5 moves here) and #376 item 1.
- [ ] Census binary (measurement apparatus, never pushed; local branch `eval/taint-parity-census`): patch
      `src/cpg/trace.rs` so `PRISM_CENSUS_DESCENT_LOG=<file>` appends a JSON line per refused cross-function
      hop (root; `from`, `to` as file, function, line, path, bytes, access; relation; depth; first failing
      condition; the callee's sites matching by line and by containment, with grades; whether a caller site
      resolves to the target under another spelling), and `PRISM_CENSUS_RELAX=<class>` relaxes one condition:
      `binding` (Use target, one matching site Exact to its function), `same_line` (the one containing site),
      `spelling` (caller sites by resolved target), `not_exact` (any grade, descent and ascent), `depth` (64).
      Save as `census/census-relax.patch`; build `bin/prism-census`.

### Task M-2: populations and runs

**Files (evidence directory):** `census/select_population.py`, `population.jsonl`, `run_census.py`,
`classify.py`, `CENSUS.md`.

- [ ] Population A (#602 M2): `~/code/bench-repos/{directus-directus, calcom-cal.com, apache-superset,
      saleor-saleor, miniflux-v2, apache-answer}`, from each the 20 latest non-merge commits touching a route,
      handler or controller path with at most 15 files changed. Use the original list if
      `tar tzf ~/prism-evidence/archive/prism-602-*.tar.gz | grep -i m2` finds it; else select by the rule and
      say so in `CENSUS.md`.
- [ ] Per diff, in a detached scratch worktree at the commit with patch `git diff <commit>^ <commit>`,
      through memguard: default `prism-base --repo <wt> --diff <patch> --algorithm taint --resolution scoped
      --format sarif`; trace mode, the same plus `--taint-return-flow`. A finding is a result with `ruleId`
      `prism/taint/taint_sink`, matched across modes by (file, start line), graded by
      `properties.confidence`.
- [ ] Lost = default minus trace; gained = trace minus default (listed in full). Each diff with a lost
      finding reruns trace mode with `prism-census` once per relaxation, log on. **Class** of a lost finding:
      the one relaxation that recovers it; `multi` if several; `other` if none (CFG, granularity, captures,
      callbacks). **Boundary**: the log line whose `to` function holds the sink line or calls into it.
- [ ] Population B: in the bench checkout, `python3 -m eval.secbench --no-build --binary bin/prism-base
      --binary-sha <sha256> --sut-commit <commit> --out census/secbench-base` (the pin options come from lane
      4 Task M-2; if unmerged, do that task here as written there). Record traced counts per vulnerability
      class (the Benchmark base) and the traced cases where classic taint (`taint.json.gz`) reports the
      target line and the witness (`witness.json.gz`) does not, with its boundary warning.
- [ ] `CENSUS.md`: per population, default sinks, trace survivors, lost and gained; lost by class with three
      examples each; grades of default findings that survive in trace mode (input to stage D's grade).

**Ordering rule.** I-binding, I-same-line and I-spelling run in descending order of lost findings in their
class on A plus B; ties keep DESIGN 5.2's order. A class with zero members on both populations becomes an
owner question (skip, or keep for the matrix); other slices continue. `not_exact` and `depth` sizes go to
the owner with I-5 (decisions 3 and 5). No lane stop gate: reasons, knobs and catalog do not depend on it.

## Stage T and E: the parity matrix (3 days)

### Task T-1: rows, fixtures, data-driven tests

**Files:** Create `tests/fixtures/taint_parity/rows.jsonl`, `tests/fixtures/taint_parity/README.md`, one
directory per fixture under `tests/fixtures/taint_parity/` (sources marked `@SRC` and `@SINK` in comments, as
`~/prism-evidence/lanes-2026-11-20/probes/lane8/make_fixtures.py` does), `tests/algo/taxonomy/taint_parity_test.rs`;
Modify `tests/algo/taxonomy/main.rs` (`mod` line), `tests/integration/coverage_test.rs` (all three lists).

Row fields: `id`, `group`, `lang`, `fixture`, `axes` (`mechanism`, `layout`, `resolution`, `depth`,
`sanitizer`), `expect` (`review`, `trace_review`: `exact|nameonly|unlabeled|none`; `reaches`:
`Reached|NotReached|BoundaryExited|Sanitized`; optional `descent_depth`), `reason` (closed list or null),
`knobs` (`{edges, depth}` or null), `catalog` (list of `{line, rule}` or null), `truth` (`flows|no_flow`),
`class`, `note`. `expect` is the behaviour after the row's group ships. Rows true on main are `control`; a
slice that moves a control cell edits the row in its PR and names it.

Harness: copy the fixture to a temp directory; `prism::repo_loader::load_repo`. **review** / **trace_review**:
`CpgContext::build(&repo.files, None)`, `prism_review::algorithms::taint::slice` with `TaintConfig { sources:
vec![src], taint_from_diff: false, extra_sinks: vec![], return_flow: false | true }` and a diff listing every
line; the cell is `prism::finding_confidence::classify_with_evidence("taint", ParseQuality::Clean, ev)` of the
`taint_sink` finding at the sink line (`result.evidence` is index-aligned), else `none`. **reaches**:
`NavigationIndex::build`, `taint_reaches(&session, &[Loc src], Some(&[Loc sink]))`, cell
`reasoning.reachability` (as `tests/reasoning/taint_reaches_test.rs` builds its sessions).

```rust
// tests/algo/taxonomy/taint_parity_test.rs
const ENABLED: &[&str] = &["control"];   // "reasons": also check the reason column of every enabled row
#[test] fn parity_rows_are_well_formed()  // unique ids; fixture dir exists; one @SRC, one @SINK; a reason wherever
                                         // reaches != Reached and review != none; closed vocabularies
#[test] fn parity_default_review()
#[test] fn parity_trace_review()
#[test] fn parity_taint_reaches()        // `reason`, `knobs`, `catalog` are data only until I-1, I-5, I-8
```

**Seed rows, research probes** (today's values observed on a0f2c427):

| Row | Mechanism | review | trace today | reaches today | Group: after |
|---|---|---|---|---|---|
| js00, js10 | source and sink in one function; `exec(f(t))` | exact | exact | Reached | control |
| js01, js02, js08 | argument, same file / ESM import; `this.run(t)` | exact | unlabeled | Reached d1 | control (grade per D) |
| js03, js04 | `P({html:t})` into `props.html` / bare `props` | exact | none | BoundaryExited | binding: unlabeled, Reached d1 |
| js05, py03, go03 | `y = f(t); sink(y)` | unlabeled | unlabeled | Reached d1 | control |
| js09 | `o.x=t; exec(o.x)` | unlabeled | unlabeled | Reached | control |
| js06 | capture read in nested function | none | none | NotReached | control; truth flows, accepted loss |
| js07 | `arr.forEach(v => exec(v))` | none | none | Reached at `arr` | control; note: granularity, lane 9 |
| js11 | `run(t); run(u);` on one line | exact | none | BoundaryExited | same_line: unlabeled, Reached d1 |
| js11b | the same on two lines | exact | unlabeled | Reached d1 | control |
| js12 | `handler.run(t)`, name-only single owner | nameonly | none | BoundaryExited | control, reason `not_exact` |
| js13 | a → b → c, sink in c | exact | none | BoundaryExited | control, reason `depth_cap` |
| js13b | a → b | exact | unlabeled | Reached d2 | control |
| js14, py04 | sanitizer, then sink | none | none | Sanitized | control |
| js14c | js14 without sanitizer | unlabeled | unlabeled | Reached | control |
| py01, py02 | argument, same file / import | nameonly | unlabeled | Reached d1 | control |
| go01, go02 | argument, same file / sibling file | exact | unlabeled | Reached d1 | control (grade per D) |
| go04 | `exec.Command(c).Run()` | none | none | NotReached | control; truth flows, accepted loss |

**Further seed rows:**

| Row | Shape | Group: required |
|---|---|---|
| l01 | `log(t); g(t)`, one site of `g`, Use bytes inside `log(t)` | control: Reached d1 |
| l02, l03 | js11 in Python and Go | same_line: Reached d1 |
| l04 | `g(g(t))` | same_line: BoundaryExited, `ambiguous_site` |
| l05 | `g(user); g(user)` (pinned by `test_two_same_line_same_name_calls_do_not_descend`, `src/cpg/tests.rs`) | same_line: set at stage D |
| s01 to s04 | `import { run as go }`; default import under another name; `const go = run; go(t)` (#376 item 1); Python `from b import run as go` | spelling: Reached d1 |
| s05 | alias to one of two `run` functions in different files | spelling: Reached, resolved target only |
| b01 | `<D a={x} />` into `props.a` (needs lane 3 S3, else `#[ignore = "#596"]`) | binding: Reached d1 |
| b02, b03 | descent, then an object literal in the callee; one level deeper | binding: Reached d2; `depth_cap` |
| b04 | `f({a:x})` recursive inside `f` | binding: set at stage D |
| b05 | literal leaf into a read inside an arrow nested in the callee | binding: `target_not_binding` |
| k01 to k03 | js12 under `nameonly,2`; js13 under `exact,3`; return ascent from a name-only callee under `nameonly,2` | knobs: Reached d1; d3; Reached |
| c01, c02 | `document.write(v)` (flat); `exec(t)` (structured, operand 0) | catalog: one sink each, with its rule |
| c03, c04 | `exec(y, x)`, only `x` tainted; `log(x); exec(x)` on one line | catalog: as review |
| c05 | sanitizer on the path before an HTML sink | catalog: Sanitized; review: none |
| c06 | sanitizer in the source function, off the path | catalog: Reached; review: none; note `sanitizer_model` |

- [ ] Write fixtures, rows, README (schema, vocabularies) and test. Control rows GREEN on main; every other
      row RED once with its group enabled by hand (ledger). Commit; CI green; one reviewer (test-only).

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION 6 on `rows.jsonl`. Axes: mechanism of flow (argument, object literal, JSX
      attribute, return, field write, capture, callback, receiver, callee alias); call-site layout (one or
      several per line, nested, multi-line argument, occurrence outside the site, chained call); resolution
      grade (Exact, name-only single owner, several targets, unresolved); depth (0 to 3, descent and ascent
      mixed); sanitizer position (none, on path, off path, in callee, on one of two paths); language (JS, TS,
      TSX, Python, Go). Census `other` examples become rows.

## Stage D: design of record (1 day)

- [ ] Brief the design seat with DESIGN 5.2, 5.3, 5.5, the rows, `CENSUS.md`, the accepted-loss candidates
      (js06, js07, go04, census `other`) and these facts: `descend_target` requires a `Def` target; a site
      matches by line or by byte containment; `CallGraph::callers` is keyed by the site's spelling;
      object-literal edges end at the Use one hop from the formal's `Def` (`src/cpg/object_literal_args.rs`
      header); the depth test runs before `descend_target`.
- [ ] The amendment fixes: (1) the proof that a Use target is a read of a formal of the callee, and the site
      proof it shares with a `Def` target; (2) site selection by byte containment, including whether it
      applies only when several sites match by line (rows l01, l04, l05); (3) site matching by resolved target,
      and a site resolving to several targets; (4) the closed reason list and its precedence (candidates:
      `target_not_binding`, `no_matching_site`, `ambiguous_site`, `not_exact`, `depth_cap`, `policy_ceiling`,
      names for today's self-call, self-return and refused-ascent boundaries); (5) the grade of a trace-mode
      finding reached through a justified crossing. The implementer updates rows. Both reviewers; cap 2.

## Stage I: slices

Order: I-1 first (every later row asserts a reason; gate Explained reads them); the descent slices by the
ordering rule; I-5 after them, so `parity_knobs_agree` runs on the final rules; I-6 (rewrites trace grade
cells); I-7 and I-8 after the descent slices so the Catalog gate measures the final engine (I-7 shares no
file with I-2 to I-4 and may run beside them); I-9 last.

**Gates per slice** (DESIGN 7): Matrix and Defaults (all); Sound and Benchmark (descent slices); Catalog
(I-8, I-9); Recovered and Explained (close-out). ORCHESTRATION 5 items 1 to 4, 8 to 10; two reviewers. No
slice adds or relabels graph edges, so item 6 does not apply; the census rerun replaces it.

### Slice I-1: boundary reasons (2 days)

**Files:** Create `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs` (`mod trace_descent;` and
`#[cfg(test)] mod trace_descent_tests;` in `src/cpg.rs`, which also re-exports `BoundaryReason`); Modify
`src/cpg/trace.rs`, `src/reasoning/types.rs`, `src/reasoning/taint_reaches.rs` (`boundary_warnings`),
`src/navigation/render.rs`, `tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED` gains `"reasons"`),
`CLAUDE.md`.

```rust
// src/cpg/trace_descent.rs — variants as stage D names them
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")] #[non_exhaustive]
pub enum BoundaryReason { TargetNotBinding, NoMatchingSite, AmbiguousSite, NotExact, DepthCap, PolicyCeiling, /* … */ }
impl CodePropertyGraph {
    pub(super) fn descend_target(&self, from: NodeIndex, to: NodeIndex, rel: Relation,
        cache: &mut BTreeMap<DescentGateKey, Result<(), BoundaryReason>>) -> Result<(), BoundaryReason>;
}
// src/cpg/trace.rs: BoundaryEdge gains `pub reason: BoundaryReason`
// src/reasoning/types.rs: InterproceduralBoundary { sink: String, reason: BoundaryReason }   (additive)
```

- [ ] Commit 1, move only: `DescentGateKey`, `descend_target`, `call_site_matches_source_occurrence`,
      `is_parameter_binding_from`, `return_flow_ascent_allowed` to `trace_descent.rs`.
- [ ] Unit tests first (builders `build`, `spell` of `object_literal_arg_tests.rs` are `pub(super)`):
      `use_target_reports_target_not_binding`, `no_site_reports_no_matching_site`,
      `two_sites_report_ambiguous_site`, `nameonly_site_reports_not_exact`, `third_descent_reports_depth_cap`,
      `context_policy_refusal_reports_policy_ceiling`, one per self or ascent reason; the enum and field land
      with a placeholder in the same commit so they compile and fail. Implement in stage D's precedence;
      message `taint reached a {kind} boundary at {path} ({reason})`.
- [ ] Enable `"reasons"`: RED, GREEN. `test_self_call_param_stays_boundary` and
      `test_two_same_line_same_name_calls_do_not_descend` pass unchanged. PR body: reasons, move commit, Tier-A.

### Slices I-binding, I-same-line, I-spelling (3 days each)

One slice per change of DESIGN 5.2: they edit one function, but each has its own group, census class and
sample, and merging them would make the Sound sample unattributable.

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs`,
`tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED`),
`tests/fixtures/taint_parity/rows.jsonl` (moved control cells), `CLAUDE.md`; I-same-line also `src/cpg/tests.rs`
(`test_two_same_line_same_name_calls_do_not_descend` takes stage D's expectation, named in the PR);
I-spelling also `docs/features/language-coverage/js-ts-known-gaps.md` (section 3, taint descent entry).

| Slice | Group | Rule | Unit tests first |
|---|---|---|---|
| I-binding | `binding` | stage D (1) | `objlit_edge_into_formal_member_read_descends`, `objlit_edge_into_bare_formal_read_descends`, `use_in_callable_nested_in_callee_refuses`, `use_target_with_two_matching_sites_refuses` |
| I-same-line | `same_line` | stage D (2) | `two_sites_one_line_choose_containing_site`, `single_site_outside_occurrence_still_descends`, `nested_same_callee_refuses_ambiguous` |
| I-spelling | `spelling` | stage D (3) | `aliased_import_site_descends`, `const_alias_site_descends`, `aliased_site_descends_only_into_resolved_target` |

- [ ] Enable the group: RED. Implement. GREEN, every other enabled group unchanged.
- [ ] Write the prediction; rerun `run_census.py` on A with the head build: default byte-identical; trace
      gains listed by class; any trace loss is a WRONG to explain or fix.
- [ ] Benchmark: harness run with the head build; traced count per class not below base.
- [ ] Sound: a reader (Opus-5.5, high, blind to slice) labels a random min(100, all) newly reached sinks
      from A ("is this path real"); at least 95 real per 100.
- [ ] Tier-A; cloud tests at 1 and 4 threads (spec: `--test algo_taxonomy taint_parity_test::`,
      `-p prism --lib trace_descent_tests`, `--test reasoning taint_reaches_test::`); two reviewers.
- [ ] PR body: prediction and result per population, sample tally, Tier-A, moved rows. I-spelling comments
      on #376 (item 1); I-binding comments on #596 (F5 done).

### Slice I-5: knobs on both engines (2.5 days)

PR #841's review options govern context files (import depth, the label of an edge entering a context file);
the default review walk has no descent. So this slice adds descent knobs to the trace engine, exposed on
`taint-reaches`, the MCP tool and trace-mode review. The ceiling governs the call-resolution grade at descent
and at return ascent; edge labels stay ungated.

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace.rs` (thread `descent` beside `policy`;
`MAX_TAINT_DESCENT_DEPTH` becomes the default), `src/cpg.rs`, `src/reasoning/taint_reaches.rs`,
`crates/prism-review/src/{cli.rs, main.rs}`, create `crates/prism-review/src/nav_taint.rs`,
`crates/prism-review/src/algorithms/taint.rs` (`TaintConfig.descent`), `algorithms/taint/run.rs`,
`crates/prism-review/src/api/run.rs` (`AlgorithmParams.taint_descent`), every `TaintConfig {` literal
(`rg -n "TaintConfig \{" crates tests`), `crates/prism-mcp/src/{input.rs, tools_reasoning.rs}` (schema;
description now covers knobs and reasons), goldens, `tests/algo/taxonomy/taint_parity_test.rs`,
`docs/MCP.md`, `CLAUDE.md`.

```rust
// src/cpg/trace_descent.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaintDescentPolicy { pub ceiling: CrossingCeiling, pub max_depth: usize }   // Default: Exact, 2
// src/cpg/trace.rs (existing entry points delegate with the default)
pub fn taint_trace_nodes_with_descent(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>,
    mode: ReturnFlowMode, descent: TaintDescentPolicy) -> Trace;
pub fn taint_trace_nodes_excluding_with_descent(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>,
    excluded: &BTreeSet<(NodeIndex, NodeIndex)>, mode: ReturnFlowMode, descent: TaintDescentPolicy) -> Trace;
// src/reasoning/taint_reaches.rs (taint_reaches delegates with the default)
#[derive(Clone, Default)] #[non_exhaustive] pub struct TaintReachesOptions { pub descent: TaintDescentPolicy }
pub fn taint_reaches_with(session: &NavigationSession, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>,
    options: &TaintReachesOptions) -> Result<Evidence, QueryError>;
```

CLI: `nav taint-reaches --edges exact|nameonly` (default `exact`) `--depth <0..=8>` (default 2). Review:
`--taint-descent-edges`, `--taint-descent-depth`, each `requires = "taint_return_flow"`. MCP: optional
`edges` and `depth` (0 to 8). The sanitizer bypass re-walk in `witness_mode` uses the same policy.

- [ ] Tests first: `parity_knobs_agree` (each enabled row under `(exact,2)`, `(nameonly,2)`, `(exact,3)` and
      its own `knobs`: trace_review present iff reaches is `Reached`); enable `knobs`; CLI parse tests in
      `cli.rs` (values, range, review flags without `--taint-return-flow` rejected); MCP input tests (accepted,
      out of range, unknown value). RED, implement, GREEN.
- [ ] Defaults gate; goldens re-recorded with the reason; Tier-A; cloud tests; two reviewers.
- [ ] PR body: `not_exact` and `depth` census sizes for owner decisions 3 and 5.

### Slice I-6: trace-mode grade (1 day; only if stage D changes it)

If stage D keeps `unlabeled`, there is no slice. Otherwise: Modify `src/finding_confidence.rs`
(`EvidencePath::from_trace`: a graded `CallDescent` or `ReturnFlow` window becomes an `EvidenceHop::Call` with
the site's resolved edge and confidence instead of setting `crossed_unlabeled`), the trace (record the
resolved edge per accepted crossing in `Trace`), `src/finding_confidence/tests.rs`, `tests/fixtures/taint_parity/rows.jsonl`,
`crates/prism-review/tests/cli/min_confidence_test.rs` if it pins trace grades, `CLAUDE.md`.

- [ ] Tests first: `justified_descent_window_grades_as_its_call`, `assignment_propagation_still_unlabeled`;
      edit affected trace_review cells. RED, GREEN. Census grade histogram before and after in the PR body.

### Slice I-7: catalog trait and catalog mode in core (3 days)

**Files:** Create `src/reasoning/sink_catalog.rs` (in `src/reasoning/mod.rs`), `tests/reasoning/sink_catalog_test.rs`
(in `tests/reasoning/main.rs`; a fixed in-test catalog); Modify `src/reasoning/types.rs`,
`src/reasoning/taint_reaches.rs` (`witness_mode` becomes `pub(crate)`), `CLAUDE.md`.

DESIGN 5.6's `CatalogSink` and `SinkCatalog` verbatim, plus:

```rust
// src/reasoning/types.rs
#[derive(Debug, Clone, serde::Serialize, PartialEq)] pub struct CatalogRule { pub rule: String, pub cwe: Option<String> }
// SinkResult gains: #[serde(skip_serializing_if = "Option::is_none")] pub catalog: Option<CatalogRule>
// src/reasoning/sink_catalog.rs
pub fn taint_reaches_catalog(session: &NavigationSession, sources: &[SeedSpec], catalog: &dyn SinkCatalog,
    options: &TaintReachesOptions) -> Result<Evidence, QueryError>;
```

Rule: trace as frontier mode does; ask `sinks_on_line` for each line holding a frontier `Use`; a sink's nodes
are that line's frontier `Use` nodes whose path base equals the text of an identifier inside one of
`tainted_operands` (review's `arg_node_taints_match` test), or all of them when `tainted_operands` is empty; a
sink with none is dropped; the rest go through `witness_mode`; each `SinkResult` aggregates its nodes, its
`sink` is a `SymbolRef::Statement` of kind `CatalogSink` with the sink's bytes, `catalog` holds rule and CWE.
Only `Reached` and `Sanitized` sinks are reported.

- [ ] Tests first: `catalog_reports_reached_sink_with_rule`, `catalog_operand_requires_tainted_identifier`,
      `catalog_empty_operands_accept_any_tainted_use`, `catalog_sanitized_sink_reports_sanitized`,
      `catalog_unreached_line_is_not_asked`. RED, GREEN. Existing wire output unchanged. Two reviewers.

### Slice I-8: review implementation and CLI catalog mode (3 days)

**Files:** Create `crates/prism-review/src/algorithms/taint/catalog.rs` (`pub mod catalog;` in
`algorithms/taint.rs`; inside `taint` it reaches the `pub(super)` matchers); Modify
`algorithms/taint/sink_match.rs` (extract, behaviour unchanged, `structured_sink_calls_on_line(parsed, line)
-> Vec<(Node, &'static SinkPattern)>` from `line_matches_structured_sink` and the per-language outcomes),
`crates/prism-review/src/api/` (re-export), `crates/prism-review/src/{cli.rs, nav_taint.rs}`,
`tests/algo/taxonomy/taint_parity_test.rs`, `CLAUDE.md`;
Create `crates/prism-review/tests/cli/nav_taint_catalog_test.rs` (in that directory's `main.rs`).

```rust
pub struct ReviewSinkCatalog { wrapper_sinks: Vec<String> }   // Sync: no RefCell inside
impl ReviewSinkCatalog { pub fn for_files(files: &BTreeMap<String, ParsedFile>) -> Self; }
impl prism::reasoning::sink_catalog::SinkCatalog for ReviewSinkCatalog { /* … */ }
```

Each call builds a fresh `SinkClassifier` (it holds `RefCell`/`Rc`). Flat: `identifiers_on_line` →
`classify(&parsed.path, …, SINK_PATTERNS)`; rule `SinkHit.rule`, CWE `SinkHit.cwe`, no operands. Structured:
rule `structured-<category>` with lane 7 DESIGN 5.2's CWEs (79, 89, 918, 502, 78, 22); operands are the
argument nodes at `tainted_arg_indices` (the attribute for `dangerouslySetInnerHTML`). Once lane 7 merges,
ids come from its registry. Line-only exclusions of the review loop (framework source lines, cleansed
structured ranges) apply. CLI: `nav taint-reaches --source … --sinks catalog`, `conflicts_with = "sink"`.

- [ ] Step 1: the extraction alone; `cargo test --test algo_taxonomy taint_sink` unchanged. Commit.
- [ ] Tests first: `parity_catalog` (per enabled row with `catalog`: reported (line, rule) set equals the row;
      reported lines equal the review column's `taint_sink` lines unless the row has a reason or a note);
      enable `catalog`; CLI tests: `--sinks catalog` on c02 prints the rule; with `--sink` it is rejected.
- [ ] Catalog-mode wall time against frontier mode on A's largest JS repository in the PR body (over 3× is
      a follow-up). Two reviewers.

### Slice I-9: the catalog in MCP — owner decision before starting (DESIGN decision 2; default branch A)

Ask at the end of I-8, with its numbers.

**Branch A: move into core (I-9a move-only, 4 days; I-9b exposure, 2 days).**
- [ ] I-9a step 0: list the closure (`rg -n "use crate::algorithms::taint"` over `taint_sinks/` and
      `taint/catalog.rs`, then over each file named). Expected: `taint_sinks/` (23 files) → `src/taint_sinks/`;
      `taint/js_ts/sink_tables.rs`, `taint/python/sink_tables.rs`, the `GO_CWE*` tables (first split from
      `taint/go/sinks.rs` into `taint/go/sink_tables.rs`, move-only); `taint/sink_match.rs`, the three
      per-language `sinks.rs` and their helpers; `taint/catalog.rs` → `src/reasoning/catalog_impl.rs`.
      Untangle the two review helpers: `matches_sink` (`algorithms/taint.rs`, used by `taint_sinks/legacy.rs`)
      moves to `src/taint_sinks/legacy.rs`; the imports of `taint_sinks/structured.rs` follow the tables. If
      the closure reaches `cleansing.rs`, `seeds.rs` or source detection beyond one helper function each, stop
      and send the list to the owner. Review keeps re-exports; moved bytes checked identical; `CLAUDE.md`
      code-organisation section updated.
- [ ] I-9b: MCP input `sink_catalog: true` (rejected with `sinks`); `tools_reasoning.rs` calls
      `taint_reaches_catalog`; `max_results` clips `per_sink`; goldens re-recorded; tests
      `taint_reaches_catalog_mode_reports_rules` and an input test.

**Branch B: the MCP crate depends on the review crate (2.5 days).**
- [ ] `crates/prism-mcp/Cargo.toml` gains `prism-review = { path = "../prism-review" }`; only a new
      `crates/prism-mcp/src/catalog.rs` names it (`prism_review::api::ReviewSinkCatalog`).
      `tests/integration/layering_test.rs`: `MCP_REVIEW_PATTERNS` stay for every other file; in
      `catalog.rs` only that path is allowed; add `mcp_catalog_file_may_name_only_the_catalog`; update its
      module doc and the `CLAUDE.md` layering paragraph. Then I-9b's input, call, goldens and tests.
- [ ] Run the `macos-footprint` idle guard locally (command in `docs/MCP.md`); report binary size and idle
      footprint against main.

## Close-out (1.5 days)

- [ ] Census rerun on A and B, final head: Explained (every default sink the trace engine misses has a
      closed-list reason, read with `taint-reaches` on the post-diff checkout); Recovered (share by class).
- [ ] Docs: `CLAUDE.md` (one paragraph on descent, reasons, knobs, catalog mode, beside the reasoning-layer
      line); `docs/MCP.md` (`taint_reaches` row); `skills/prism-code-navigation/SKILL.md` (catalog mode,
      reasons). Comments on #602 (census table, what remains by design) and #596 (F5 closed).
- [ ] Follow-ups: lane 7 registry ids if not done; unexplained classes; catalog performance if flagged.
      Refresh `HANDOFF.md` and `LEDGER.md`; offload raw census output; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN 7 holds on the merged head (numbers in the close-out report); both reviewers approved
every product slice; census and matrix are on the tracking issue. A failing gate stops the lane for the
owner; the orchestrator does not adjust thresholds.

## Estimate

| Stage or slice | Days |
|---|---|
| M, T and E, D | 3, 3, 1 |
| I-1 reasons | 2 |
| I-binding, I-same-line, I-spelling | 3 each (9) |
| I-5 knobs; I-6 grade (conditional) | 2.5; 0 or 1 |
| I-7 catalog core; I-8 review catalog and CLI | 3; 3 |
| I-9 branch A / branch B | 6 / 2.5 |
| Close-out | 1.5 |
| **Total** | **34 to 35** (branch A), **30.5 to 31.5** (branch B) |

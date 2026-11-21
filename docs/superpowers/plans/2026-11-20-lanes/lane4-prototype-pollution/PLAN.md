# Lane 4: prototype-pollution tracing — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** first, an experiment that says how many benchmark cases a prototype-pollution sink would report
and how often it would fire on ordinary code; then, if the pre-stated gate passes, syntactic facts for
pollution-capable dynamic-key writes, a CLI listing of them, and a review-taint sink that fires when taint
reaches a key operand of an unguarded one.

**Architecture:** facts are computed per file from syntax in the core crate
(`src/ast/js_ts/proto_writes*`); `prism nav proto-writes` lists them; the review crate's sink check consumes
them from one call in the taint sink loop. No data-flow producer or cache change.

**Tech stack:** Rust (core and prism-review crates, tree-sitter JavaScript/TypeScript); Python 3 for the
benchmark harness and joins; Node for the truth runner (lane-written fixtures only).

**Spec:** `DESIGN.md` in this directory (classes 5.1, guards 5.2, sink 5.3, listing 5.4, experiment 6,
truth 7, gates 8).

## Global constraints

- Two worktrees: `~/code/prism-pp` for product slices (sparse-checkout excludes `eval/secbench/`), and
  `~/code/prism-pp-meas` for everything under `eval/secbench/` and for measurement (full checkout). Changes
  to the harness are separate eval-only PRs made in the second worktree. Evidence `~/prism-evidence/pp/`.
- Benchmark inputs are read, never executed: `~/prism-evidence/inputs/secbench-js`, `…/secbench-pkgs`.
  Baseline rows: `~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl`.
- Only fixtures written for this lane run under Node: one child process per row and payload, 5 second
  limit, no network.
- Briefs for codex seats use neutral wording: "benchmark accounting", "entry point", "target write",
  "key operand". Product implementers work from `tests/fixtures/proto_writes/` and do not open the
  benchmark tree.
- The harness run is a measurement: host free, through memguard, one at a time (ORCHESTRATION section 4).
- No change to `src/ast/lvalues.rs`, `src/access_path.rs`, `src/data_flow/`, `src/cpg/` or the cache version.
- `src/navigation/queries.rs` is over the size ceiling: the listing's logic goes in a new module.
- PR #841 edits `taint/run.rs`: the sink slice rebases over it (DESIGN decision 8).
- Rule tag `js-proto-write`, CWE `CWE-1321`. Class names in output: `nested_keys`, `path_walk`,
  `recursive_merge`. Guard names: `none`, `partial`, `sufficient`.

## Review focus

1. TypeScript wrappers on the target or key (`(o as any)[k] = v`, `o![k] = v`, `o[k as string] = v`): the
   write is seen through them. (S1)
2. Logical and destructuring assignment targets (`o[k] ??= {}`, `[o[k]] = [v]`, `({ a: o[k] } = src)`):
   counted as dynamic-key writes. (S1)
3. A very large generated or minified file: analysis stays linear; a function with more than 10,000
   subscript writes is skipped and counted in a warning, never a hang. (S1)
4. Two dynamic-key writes on one line with different guard states: the sink matches the tainted occurrence
   to its own write by byte span (a flow path's end always carries one). (S2)
5. A cursor captured by a nested callback (`path.split('.').forEach(p => { o = o[p]; })`): classified with
   the function that declares the cursor. (S1)

---

## Stage M: the experiment (2 to 3 days)

### Task M-1: workspaces and a main build

- [ ] Create both worktrees and the evidence directory. In `prism-pp-meas`, build release through memguard
      and record `prism --version` and the binary's SHA-256.
- [ ] Comment on #603: the lane has started; the issue's premise is corrected in `DESIGN.md` section 2.

### Task M-2: harness class filter and recorded pin overrides (eval-only PR, in `prism-pp-meas`)

**Files:**
- Modify: `eval/secbench/run.py`, `eval/secbench/README.md`
- Test: `eval/secbench/test_cli.py` (it already drives `run.main()`); run with
  `python3 -m unittest discover -s eval/secbench -t .`

**Interfaces (produced):** command-line options
`--class <name>` (repeatable), `--binary-sha <hex>`, `--sut-commit <sha>`.

- `--sut-commit` replaces the built-in `SUT_COMMIT` everywhere `main()` uses it: the
  `git diff --exit-code <commit> -- src Cargo.toml Cargo.lock build.rs vendor` check in `--sut-repo`, the
  check that the binary's `--version` names the commit, and the `sut_head` recorded in `binding.json`.
  `--binary-sha` replaces `BINARY_SHA` in the binary authentication.
- `--class` restricts the **measured** population only. Inspection verification, the comparison with
  `input-pins.json` and the acquired-population count still run on all entries. The measurement loop, the
  progress counter, `entries.jsonl` and the summary cover only entries of the named classes.
- The summary JSON gains `"class_filter": [...]` and `"pins_overridden": {"binary_sha": …, "sut_commit": …}`
  (both absent when unused). Without any of the three options, behaviour and output are unchanged.
- A run with overrides is not comparable byte for byte with the baseline (`harness_sha256` in `binding.json`
  changes with the harness edit); the README says so.

- [ ] Tests first, in `test_cli.py`: `test_class_filter_measures_only_that_class`,
      `test_pin_overrides_replace_both_checks_and_are_recorded`, `test_default_run_is_unchanged`.
- [ ] Implement; tests pass; one reviewer (eval-only); merge.

### Task M-3: measure E1

- [ ] From the repository root of `prism-pp-meas`, through memguard, host free:
      `python3 -m eval.secbench --no-build --binary <main build> --binary-sha <its sha256> --sut-commit
      <the build's commit> --class prototype-pollution --inspection
      ~/prism-evidence/meas/secbench/repair-r1/inspection-final.jsonl --out ~/prism-evidence/pp/e1`.
      The worktree must be at the build's commit with no change under `src`.
- [ ] `e1/SUMMARY.md`: outcome counts beside the baseline's (41 traced of 95 with ground truth), and the
      list of case ids whose outcome changed, each with the baseline and current outcome.

### Task M-4: E2, E3, E4 scripts and runs

**Files (evidence directory):** `m/key_reached.py`, `m/classify_proto.mjs`, `m/benign.sh`, outputs under `m/`.

- [ ] `key_reached.py <entries.jsonl> <packages dir>`: for each traced case, take
      `trace_detail.parameter_results` (one verdict per occurrence on the sink line, with the occurrence's
      byte span) and `sink.{file,line,column}`; parse the sink file (read, never run) to find the write at
      that line and column and the byte spans of its index expressions; the key is reached when a Reached
      occurrence's span lies inside one of them. Output: case id, key reached yes or no, the occurrence.
- [ ] `classify_proto.mjs <file> <line>` and a directory mode: a prototype of the classes of DESIGN 5.1 and
      a guard-token report (does the function mention `__proto__`, `constructor`, `prototype`,
      `hasOwnProperty`, `Object.create(null)`), written with the pinned TypeScript parser. It is a
      measuring tool, not the product rule; keep it under 400 lines.
- [ ] Run it on every prototype-pollution row that has a sink coordinate (E3), and in directory mode on
      Excalidraw, `~/code/bench-repos/TypeScript/src`, `~/code/bench-repos/express`,
      `~/code/bench-repos/directus-directus/api` and `~/code/bench-repos/calcom-cal.com/apps/api` (E4).
- [ ] A Sonnet-5.5 reader labels up to 40 E4 hits (sort by file and line, then sample with
      `random.Random(20261120)`), reading each function: can pollute if the key is attacker-controlled /
      cannot / guarded.
- [ ] Fixed versions (optional): for up to 20 converted packages, find the fixed version in the benchmark
      entry's own metadata under `~/prism-evidence/inputs/secbench-js` (look at one entry to see which field
      names it; skip a package whose entry names none). If the network allows, fetch each with
      `npm pack <name>@<version>` into `~/prism-evidence/pp/fixed/` (archives unpacked, never installed or
      run), run the classifier on the same functions, and record which guard forms the fixes use. Otherwise
      write "fixed versions owed" in the ledger.

### Task M-5: memo and gate

**Files:** `docs/superpowers/plans/2026-11-20-lanes/lane4-prototype-pollution/EXPERIMENT.md`

- [ ] Write the memo per DESIGN section 6: E1 to E4; C and P by shape; the converted case ids; class hits
      per benign corpus with the reader's tally; guard forms seen in fixed versions.
- [ ] Apply the gate. C ≥ 25: commit the memo, post the table on #603, continue. 15 ≤ C < 25: post and stop
      for the owner. C < 15: post, close the lane.

---

## Stage T and E: executable matrix (3 days)

### Task T-1: fixtures and the truth runner

Stage T produces data, not Rust tests: the modules the tests would name do not exist yet. Each slice adds
its Rust test in its first commit and shows it RED before implementing.

**Files:**
- Create: `tests/fixtures/proto_writes/cases/<row>.js` (one exported function per file),
  `tests/fixtures/proto_writes/rows.jsonl`, `tests/fixtures/proto_writes/sink_rows.md` (the sink rows
  below, as source text with `// SEED` and `// SINK` markers, for slice S2),
  `tests/fixtures/proto_writes/truth/run_truth.mjs`, `tests/fixtures/proto_writes/truth/truth.jsonl`
  (committed output), `tests/fixtures/proto_writes/truth/check.sh`, `tests/fixtures/proto_writes/README.md`

`rows.jsonl` fields: `id`, `file`, `write_line`, `call` (how the runner invokes it: which argument takes the
payload), `truth` (`pollutes` or `safe`, copied from `truth.jsonl`), `want_class` (a class name or `none`),
`want_guard`, `class` (what the row is an instance of), `accepted_loss` (optional reason). After stage D the
implementer updates `want_class` and `want_guard` from the amended design in the same commit as the design;
`truth` never changes by hand.

`run_truth.mjs <cases dir> <rows.jsonl>`: for each row and each payload family of DESIGN section 7, spawn
`node` on a small driver that imports the case, calls it as `call` says, and prints whether
`({}).polluted !== undefined`. Writes `truth.jsonl`. A row whose function throws on every payload is
`safe` with `threw: true`.

Seed rows (truth is whatever the runner prints; the expectation column is the hypothesis to check):

| Row | Function | Expect |
|---|---|---|
| pw01 | the path setter of DESIGN section 1 | pollutes; `path_walk`; guard none |
| pw02 | `while (parts.length > 1) { const k = parts.shift(); obj = obj[k] = obj[k] \|\| {}; } obj[parts[0]] = v;` | pollutes; `path_walk` |
| pw03 | `path.split('.').forEach(function (p, i, a) { if (i === a.length - 1) obj[p] = v; else obj = obj[p] = obj[p] \|\| {}; });` | pollutes; `path_walk` |
| pw04 | `keys.slice(0, -1).reduce((o, k) => (o[k] = o[k] \|\| {}), obj)[keys[keys.length - 1]] = v;` | pollutes; `path_walk` |
| pw05 | recursive setter: `function set(o, keys, v) { if (keys.length === 1) o[keys[0]] = v; else set(o[keys[0]] = o[keys[0]] \|\| {}, keys.slice(1), v); }` | pollutes; `recursive_merge` or `path_walk` (stage D names it) |
| pw06 | JSON-pointer setter over `pointer.split('/')` | pollutes; `path_walk` |
| pw07 | unflatten: keys such as `"a.b"` expanded into nested objects | pollutes; `path_walk` |
| nk01 | `function put(a, k1, k2, v) { a[k1][k2] = v; }` | pollutes; `nested_keys` |
| nk02 | `for (let r = 0; r < n; r++) for (let c = 0; c < m; c++) grid[r][c] = 0;` | safe; no fact |
| nk03 | `a["fixed"][k] = v` | safe; no fact |
| rm01 | the recursive merge of issue #603 | pollutes; `recursive_merge` |
| rm02 | the same with `Object.keys(source).forEach(key => { … })` | pollutes; `recursive_merge` |
| rm03 | merge that recurses through a same-file helper (`mergeObject` ↔ `mergeValue`) | pollutes; `recursive_merge` |
| rm04 | one-level copy: `for (const k in s) t[k] = s[k];` | safe; no fact |
| rm05 | deep clone that builds a new object and never writes into an argument | by the runner |
| g01 | rm01 with `if (key === '__proto__' \|\| key === 'constructor') continue;` | safe; guard sufficient |
| g02 | rm01 with `if (key === '__proto__') continue;` | pollutes; guard partial |
| g03 | rm01 with `if (!source.hasOwnProperty(key)) continue;` | pollutes; guard none |
| g04 | pw01 with `if (!Object.prototype.hasOwnProperty.call(o, k)) o[k] = {};` before each step | by the runner |
| g05 | pw01 with the root from `Object.create(null)` | by the runner |
| g06 | pw01 with `if (['__proto__', 'constructor', 'prototype'].includes(k)) return;` | safe; guard sufficient |
| g07 | pw01 with a regular-expression test of the key | safe; expected accepted loss (fact not sufficient) unless stage D admits the form |
| g08 | guard written after the write | pollutes; guard none |
| g09 | guard on a different variable than the key | pollutes; guard none |
| g10 | guard inside a helper `isSafeKey(k)` | safe; expected accepted loss |

- [ ] Write the cases, the runner and `rows.jsonl`; run the runner; commit `truth.jsonl`. Where the runner
      contradicts the "Expect" column, the runner wins: correct the row and note it in the ledger.
- [ ] `truth/check.sh` re-runs the runner and diffs against the committed `truth.jsonl`. The orchestrator
      runs it before stage D and before each slice merges; it is not part of `cargo test`.
- [ ] Write `sink_rows.md` (all in one file per row, so the existing single-file sink test support can run
      them): `sk01` the path setter called with a seeded path, fires at the write; `sk02` only the value
      seeded, silent; `sk03` only the receiver seeded, silent; `sk04` sufficient guard, silent; `sk05`
      partial guard, fires and the description says so; `sk06` `cache[id] = v` with seeded `id`, silent;
      `sk07` a caller function in the same file passes the seeded value to the path setter, fires at the
      setter's write line; `sk08` the key is a member expression (`o[opts.key][name] = v` with `opts.key`
      seeded), fires; `sk09` a recursive merge whose key comes from `for…in` over a seeded object, fires
      (slice S3); `sk10` the same with a sufficient guard, silent (S3); `sk11` a recursive merge whose key
      comes from `Object.keys(source).forEach(key => …)`: silent today because callback parameters are not
      bound (#434), recorded as an accepted loss. A helper in another file is not a row: review taint keeps
      findings in the diff's file until PR #841 lands.

### Task T-2: enumeration

- [ ] Run the tiers on `rows.jsonl`. Axes: how the cursor moves (assignment forms, loops, array methods,
      recursion, helper calls); where keys come from (`split`, `for…in`, `Object.keys/entries`, parameters,
      destructuring, `shift`/`pop`, regular-expression matches); write forms (plain, compound, logical,
      destructuring targets, `Object.defineProperty`, `Reflect.set`); guard forms and placements; module and
      function kinds; TypeScript syntax. The benchmark shapes of the memo and the fixed-version guard forms
      are rows. Every accepted row gets its truth from the runner before it is committed.

## Stage D: design of record (1 day)

- [ ] The design seat (ORCHESTRATION section 3, stage D) amends DESIGN 5.1 and 5.2 against the full row
      set: exact class definitions, the key-operand rule, the guard forms and placements that are
      sufficient (each backed by `safe` rows) and the accepted-loss list in both directions. The
      implementer updates `rows.jsonl` to match. Both reviewers; cap 2 rounds.

---

## Stage I: slices

### Slice S1: facts for `nested_keys` and `path_walk`, guards, the listing (4 to 5 days)

**Files:**
- Create: `src/ast/js_ts/proto_writes.rs`, `src/ast/js_ts/proto_writes/classes.rs`,
  `src/ast/js_ts/proto_writes/guards.rs`, `src/ast/js_ts/proto_writes_tests.rs`,
  `src/navigation/proto_writes.rs`, `crates/prism-review/tests/cli/nav_proto_writes_test.rs`
- Modify: `src/ast/js_ts.rs` (the `mod proto_writes;` and `#[cfg(test)] mod proto_writes_tests;` lines),
  `src/navigation/mod.rs`, `crates/prism-review/src/cli.rs` (`NavQuery`),
  `crates/prism-review/src/main.rs` (`run_nav` arm), `crates/prism-review/tests/cli/main.rs`, `CLAUDE.md`,
  `docs/features/language-coverage/js-ts-known-gaps.md`
- Separate eval-only PR from `prism-pp-meas`, after this slice merges: `eval/secbench/pp_join.py` and
  `eval/secbench/test_pp_join.py`

**Interfaces (produced):**

```rust
// src/ast/js_ts/proto_writes.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtoWriteClass { NestedKeys, PathWalk, RecursiveMerge }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState { None, Partial, Sufficient }
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct KeyOperand { pub name: String, pub base: String, pub start_byte: usize, pub end_byte: usize }
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProtoWrite { pub line: usize, pub start_byte: usize, pub end_byte: usize,
    pub function: String, pub function_start_line: usize, pub function_end_line: usize,
    pub class: ProtoWriteClass, pub key_operands: Vec<KeyOperand>, pub guard: GuardState }
impl ParsedFile {
    /// Pollution-capable dynamic-key writes of this file, sorted by (line, start_byte).
    /// Empty for languages other than JavaScript, TypeScript and TSX.
    pub fn js_ts_proto_writes(&self) -> Vec<ProtoWrite>;
}

// src/navigation/proto_writes.rs
#[derive(serde::Serialize)]
pub struct ProtoWriteRow { pub file: String, #[serde(flatten)] pub write: ProtoWrite }
pub fn proto_writes(session: &NavigationSession) -> Vec<ProtoWriteRow>;   // sorted by (file, line, start_byte)
```

`KeyOperand.name` is the identifier or member chain as the data-flow graph spells it (`keys`,
`opts.key`); `base` is its first identifier. CLI: `prism nav proto-writes --repo <dir> [--format json|text]`
(JSON: one object per line). `eval/secbench/pp_join.py <harness out dir> <packages dir> <prism binary>`
runs the listing per package and prints, per case: traced, key reached, fact class, guard, and
`finding_equivalent` (DESIGN 5.4).

- [ ] Module skeleton returning no facts; `proto_writes_tests.rs` is one data-driven test that reads
      `rows.jsonl` and asserts, for every row, the fact at `write_line` (class and guard) or its absence.
      Rows whose `want_class` is `recursive_merge` are skipped through a constant `ENABLED_CLASSES` until
      slice S3. RED for the rows that want a fact.
- [ ] Implement `classes.rs` (the two classes), then `guards.rs`. GREEN for the non-ignored rows.
- [ ] Review focus items 1, 2, 3, 5: one test each.
- [ ] The listing and its CLI test (`lists_facts_sorted_as_json_lines`, `other_languages_list_nothing`).
- [ ] Measure before review, host free: the listing on the E4 corpora (fact count per class and guard; the
      full list for non-test directories goes in the PR body or an attached file).
- [ ] After the merge: the `pp_join.py` PR (with a unit test on a two-case fixture), then `pp_join.py` on
      the E1 output with the merged binary (finding-equivalents by shape).
- [ ] Gates of ORCHESTRATION section 5: items 1 to 5 (Tier-A matrix only: `src/ast` is touched but no
      resolution or graph behaviour changes; say so in the PR), 8, 9, 10. Two reviewers.

**Gate:** finding-equivalents for the path-setter and flatten shapes ≥ 0.9 × their share of C; the E4
sample re-labelled against the product facts shows at least 80% able to pollute.

### Slice S2: the sink (3 days)

**Files:**
- Create: `crates/prism-review/src/algorithms/taint_sinks/proto_write.rs`,
  `tests/algo/taxonomy/taint_sink_proto_write_test.rs` (and its `mod` line in `tests/algo/taxonomy/main.rs`)
- Modify: `crates/prism-review/src/algorithms/taint_sinks/mod.rs` (module line),
  `crates/prism-review/src/algorithms/taint/run.rs` (the per-edge check, the side set, the emission block's
  description and strength), `CLAUDE.md` (the `taint_sinks/` paragraph), the known-gaps doc

**Interfaces (produced):**

```rust
// crates/prism-review/src/algorithms/taint_sinks/proto_write.rs
pub(crate) const RULE: &str = "js-proto-write";
pub(crate) const CWE: &str = "CWE-1321";
pub(crate) struct ProtoWriteIndex { /* built once per file from js_ts_proto_writes() */ }
pub(crate) struct ProtoHit { pub partial_guard: bool }
impl ProtoWriteIndex {
    pub(crate) fn for_file(parsed: &ParsedFile) -> ProtoWriteIndex;
    /// Some when the occurrence at `bytes` on `line` lies inside a key operand of a pollution-capable write
    /// whose guard is not sufficient.
    pub(crate) fn key_hit(&self, line: usize, bytes: (usize, usize)) -> Option<ProtoHit>;
}
```

In `run.rs` (DESIGN 5.3): the check runs once per flow-path edge, outside the per-identifier loop and before
the framework-source-line skip, on the edge's end (`edge.to`: line and byte span); a hit records the line
with its `ProtoHit` in a side set. The emission block, for a line in that set, writes the description of
DESIGN 5.3 and forces Strong (severity `warning`) whatever `line_strength` returns. Category stays
`taint_sink` (DESIGN decision 9).

- [ ] Rebase on `main`; if PR #841 has merged, read the new shape of the sink loop first.
- [ ] Write `taint_sink_proto_write_test.rs` from `sink_rows.md` rows `sk01` to `sk08` with the existing
      single-file support (`taint_sink_rules_support.rs`: `Case { label, source }`, `// SEED`, `// SINK`);
      add a helper there only if a row needs to read the finding's description (`sk05`). RED.
- [ ] Implement the index, the check and the emission changes. GREEN.
- [ ] Review focus item 4. A row with a weak SQL call on the same line as a hit: still `warning`.
- [ ] No-regression check: run the existing taint test targets (`--test algo_taxonomy`); the only changed
      expectations are new findings with the DESIGN 5.3 description, each listed in the PR body. Note that
      `test_taint_js_prototype_pollution` in `taint_lang_test.rs` passes through a `res.send` sink and is
      not evidence for this rule; leave it as it is.
- [ ] Two reviewers.

### Slice S3: `recursive_merge` (3 to 4 days)

**Files:** `src/ast/js_ts/proto_writes/classes.rs` (or a new `merge.rs` beside it if `classes.rs` would pass
600 weighted lines), `src/ast/js_ts/proto_writes_tests.rs` (`ENABLED_CLASSES`),
`tests/algo/taxonomy/taint_sink_proto_write_test.rs` (rows `sk09` to `sk11`), docs.

- [ ] Add `recursive_merge` to `ENABLED_CLASSES`: RED. Implement direct recursion and one same-file helper
      hop, parameter-position matching of the dynamic read, single-alias bases. GREEN.
- [ ] Measure: listing on the E4 corpora (new facts listed), `pp_join.py` on E1 (merge-shape
      finding-equivalents; the potential set P re-counted with the product facts).
- [ ] Sink rows `sk09` (fires), `sk10` (silent) and `sk11` (silent, the accepted loss of #434, asserted so
      a later fix shows up). Two reviewers.

### Slice S4: acceptance and close-out (1 to 2 days)

- [ ] Re-run E1 with the final build and `pp_join.py`: the **benchmark yield** gate of DESIGN section 8.
      List converted cases the final rules call guarded, for a human check.
- [ ] Re-label the E4 sample against the final facts: the **ordinary code** gate.
- [ ] Run `truth/check.sh`.
- [ ] `g/REPORT.md` in the evidence directory and a summary on #603: C, finding-equivalents by shape,
      potential P by first break (linking #595 and #434), ordinary-code counts, accepted losses.
- [ ] Correct `docs/features/language-coverage/js-ts-known-gaps.md` (the statement that the write is not a
      definition) and close #603. File the follow-ups of DESIGN section 9 that the numbers justify.
- [ ] Refresh the handoff; offload raw harness output; delete both worktrees' `target/`.

## Acceptance for the lane

The experiment memo is merged; if the gate passed: every gate of DESIGN section 8 holds on the merged head,
both reviewers approved S1, S2 and S3, and the report is posted on #603.

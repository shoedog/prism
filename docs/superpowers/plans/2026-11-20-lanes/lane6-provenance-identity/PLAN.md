# Lane 6: provenance findings by proven identity — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is dispatched
> to an implementer (Sonnet-5.5, high) with its section, plus the Global constraints section, as the brief. Steps
> use checkbox syntax.

**Goal:** every provenance finding names a variable, rests on a definition that reaches the use on a live route,
and carries an origin proven from that definition's expression (library, framework or built-in identity for Python,
JavaScript/TypeScript and Go; gated code-only patterns elsewhere), measured before and after on 240 real diffs.

**Architecture:** `provenance_slice.rs` keeps `slice()` and delegates to `algorithms/provenance/` (value-read
occurrences, reaching definitions, join, pattern tables, identity rules per family). Identity comes from the import
proof in `targets/dependency_identity.rs` (made visible in the crate, plus a non-call entry point, plus site
refusals on the provenance side) and from the taint request-source recogniser (exposed by one function, matched by
position). The review entry point passes the repository root. The targets mapping keeps parsing the unchanged
description and attaches a hint only for identity languages. No change under `src/` or the cache.

**Tech stack:** Rust (prism-review crate, tree-sitter); Python 3 for the population scripts.

**Spec:** `DESIGN.md` here (5.1 examined set and reaching definitions, 5.2 origins, tables and propagation, 5.3
other languages, 5.4 shared identity, 5.5 output and cross-check, 5.6 layout, 5.7 losses, 5.8 changed legacy
assertions, 6 measurement, 7 gates, 8 owner decisions). Use `D-rows.tsv` and `D-groups.tsv` in this directory.

## Global constraints

- Worktree `~/code/prism-provenance` (sparse, without `eval/secbench/`); evidence `~/prism-evidence/provenance/`
  (`m/`, `t/`, `s<N>/`, `g/`, `LEDGER.md`, `HANDOFF.md`); corpus clones `~/code/prism-provenance-corpora/`.
- Unchanged: category `untrusted_origin`; `algorithm` `provenance`; the severity map (`user_input`, `hardware` →
  `concern`; `database`, `external_call` → `warning`; `function_param`, `env_var`, `config` → `info`; `constant`,
  `unknown` → none); the description `variable '<name>' has <origin> origin: <risk>` from
  `Origin::name()`/`risk_level()` (`src/frameworks/origin.rs`, not edited); `function_name: None`; provenance in
  `review_suite()`, `all()` and the `prism targets` default set; the public items
  `provenance_slice::{slice, Origin, ProvenanceFinding}` (`slice(ctx, diff)` keeps its signature and runs with no
  repository root).
- Deliberate, disclosed in each PR body: `<name>` is the use's `path.base`; `related_lines` are the selected
  definitions' lines plus the proving chain's def lines (provenance `targets[].id` hashes them, so ids change with
  each slice that changes them, the description or the severity); blocks are emitted for examined variables only.
- Weighted sizes: `provenance_slice.rs` 616, `taint.rs` 323, `dependency_identity.rs` 537, `dependency_hint.rs`
  277.5, `mapping.rs` 335, `targets/mod.rs` 338.5, `algorithms/mod.rs` 278, `api/run.rs` 419,
  `targets_mapping_test.rs` 560; `tests/common/mod.rs` 1,226 and `coverage_test.rs` 1,044 take edits only. New files
  aim for 600 or fewer.
- New test files go into the three lists of `tests/integration/coverage_test.rs` (`all_test_files` in
  `test_algorithm_language_matrix` and `test_language_coverage_minimum`, `test_files` in
  `test_coverage_matrix_validation`), after `provenance_lang_test.rs`. Unit-test files of the new tree are declared
  in `provenance/mod.rs` as `#[cfg(test)] mod <name>;`.
- Lane 2 owns `crates/prism-review/src/targets/` (ORCHESTRATION section 11). This lane's edits there are additive
  (visibility, one optional field, one function, one guard in a mapping arm) and preserve the existing dependency resolver.
  S6 changes the provenance hint projection. S3 changes the one legacy targets fixture assertion named in DESIGN 5.8.
  S2 rebases over lane 2's slice S1 if merged; otherwise S2 carries S1's `root_identity` as lane 2's PLAN states it
  and the orchestrator records that on lane 2's issue.
- `taint.rs`/`taint/` are shared with lane 4 and PR #841: one new module and one `mod`/`use` pair, nothing else.
- `PRISM_PROVENANCE_TRACE=1` prints the trace DESIGN 5.5 defines (stderr; default output unchanged). Every
  population run sets it.
- Tests and `prism` runs go through `~/prism-evidence/tools/memguard.sh 8`; do not run mutgate; implementers do not
  push (the orchestrator does).

## Review focus

1. Drop a trigger-spelled token by role or classify its actual binding (`st-py-args-local`, `py-unproven-local-request`). A helper's unknown call cannot return proven input (`py-helper-request-param`, L2).
2. Use the live write and expression slot. Check both branch orders, overwrites, early exits, loop-carried writes and Go package initialization (DESIGN 5.1.2). Only a reachable write participates in the severity join.
3. Select real one-byte, module-level and receiver reads. Keep intact portions of a malformed file (`py-module-level-env`, `st-py-parse-error`).
4. Pair every handle/negative with the library's payload or output slot, including `*sql.Rows` versus a `Scan` destination. Audit the projected variable on the changed line, the direct accessor, and a later read independently.

## Stage M: measure first (4.5 days)

### Task M-1: issue, workspace, population (240 diffs)

- [ ] File the tracking issue "provenance: findings by proven identity (lane 6)" (links `DESIGN.md`, this plan,
      design section 2's table); record its number in `LEDGER.md`. Worktree per ORCHESTRATION section 4 (branch
      `eval/provenance-census`); release build through memguard; copy it to
      `~/prism-evidence/provenance/bin/prism-<sha8>` with its SHA-256 and `--version`.
- [ ] Extract `next-five-plan-fable-20261114/final-gate-campaign-preparation/population.jsonl` from
      `~/prism-evidence/archive/prism-crossfile-docs-493ab5ee-receipts.tar.gz` into `m/` (SHA-256
      `c819737a8f64552cffa56f393b797e21d63aea327f890247f4963f9f2dca9e40`): 120 rows, 20 each from apache-answer and
      miniflux-v2 (Go), calcom-cal.com and directus-directus (JS/TS), apache-superset and saleor-saleor (Python),
      with commit, parent, tree, selection head, `diff_files`, `diff_lines`.
- [ ] `m/select_extra.py`: the rule of `~/code/prism-evidence-repo/jsts-lane/602/m2/scripts/select.py` (its `RULES`
      for saleor-saleor and apache-superset, `is_test`, `SRC_EXT`, at most 15 changed files, one parent,
      `git log --no-merges -M --name-status` from the same selection head); candidates 21 to 50 of each stream give
      the 60 extra Python-heavy diffs. The same rule over `redis` (C), `leveldb` (C++), `guava` (Java) and
      `ripgrep` (Rust), requiring a changed non-test file of that language, takes the first 15 candidates each:
      the 60 pattern-language diffs (DESIGN 8 decision 6). Commit order decides, no seed. `~/code/bench-repos`
      clones are read only.
- [ ] `m/specs.py`: diff specs by `m2run.py`'s `added_lines` and filter (supported extension, not a test, present at
      the commit), written with `json.dump(spec, f)` as there; for the 120 rows the SHA-256 must equal
      `base_spec_sha256` (a mismatch stops the task). `m/population.jsonl`: 240 rows (`ordinal`, `repo`, `family`
      py/js/go/c/cpp/java/rust, `commit`, `parent`, `tree`, `spec`, `spec_sha256`, `origin` `602`/`lane6`). Corpus
      clones by `git clone --shared`; each run checks out its commit detached and verifies `HEAD^{tree}` and a clean
      status.

### Task M-2: driver and base run

- [ ] If `scripts/measure/` on `main` has `review-diff` (lane 11; `python3 -m measure review-diff --help` from
      `scripts/`), convert the manifest and use it, with the resume rule below. Otherwise write
      `m/run_population.py` (`--population P --binary B --out D [--targets] [--only FAMILY]`): serial, resumable,
      per row `memguard.sh 8 B --repo <clone> --diff <spec> --algorithm provenance --format json --no-cache` with
      `PRISM_PROVENANCE_TRACE=1`, timeout 900 s, writing `<ordinal>.json`, `.err`, `meta.json` (argv, exit,
      elapsed, binary SHA-256, `spec_sha256`, tree, options, status); `--targets` adds
      `B targets --repo <clone> --diff <spec> --algorithm provenance --no-cache` into `<ordinal>.targets.json`.
      A row is skipped only when its `meta.json` matches binary SHA-256, `spec_sha256`, tree and options, its status
      is `complete` (exit 0, no timeout), and every requested artifact exists and parses; otherwise it is rerun.
- [ ] `m/compare.py BASE HEAD`: findings by family × origin × severity; added, removed, changed keyed by (ordinal,
      file, line, name); every base and head key reconciled to exactly one of unchanged, changed, removed (with the
      head's `prov-unknown` or `prov-drop` reason, or `absent` when the row did not complete) and added, and the four
      sums printed against the key totals; it fails on a row missing from either side. `m/test_compare.py` on a
      two-row fixture, including a removed key with no head trace line (must fail). Base run with `--targets`, one
      measurement at a time, no host-quiet marker (counts only).

### Task M-3: labelled base sample, go gate, memo

- [ ] `m/sample.py` (DESIGN 6, precision samples): findings at every severity, sorted by (ordinal, file, line,
      description), stratified by the finding file's language × origin, at least 30 per cell or the whole cell,
      `random.Random(20261121)`, inclusion weights written per item. Each item: the use line, the whole enclosing
      function, the file's imports (`git show <commit>:<path>`), the selected definition lines, the variable, and the
      claimed origin in a separate field shown after the reader's first answer.
- [ ] Two independent readers: Sonnet-5.5 high (Agent tool) and gpt-6.1-terra (`codex exec`, read-only, item files
      only). Question, verbatim: "First: is the named token a variable read at this line? If yes, find the definition
      that determines its value here and name the origin you see, as defined below, or answer
      `insufficient_evidence`. Then, shown the claimed origin: answer `right`, `wrong_origin`, `source_object` (the
      request or handle object itself), `not_variable` (module, attribute, callee, function, type or keyword name, or
      text in a comment or string) or `insufficient_evidence`. Unless `right`, pick a cause: trigger substring in a
      name, comment or string, non-variable token, line inheritance, other function's definition, killed or later
      definition, precedence, constant masking, signature substring, one-sided boundary, other." The brief carries
      DESIGN 5.2's origin definitions and the frozen rubric (DESIGN 6, readers). Disagreements get a third read
      (Opus-5.5 high). Store `m/labels/base-r1.jsonl`, `base-r2.jsonl`, `base-final.jsonl`; report per-rater,
      adjudicated and unresolved counts and Cohen's kappa. In the completed base sample the independent readers
      were gpt-6.1-terra and gpt-6.2-sol. The census readers were Sonnet-5.5 and gpt-6.1-terra, with Opus-5.5 on disputes. Use the actual reader record from `m/census-tables.md`.
- [ ] Go gate (DESIGN 7): precision at `concern`/`warning` = `right` / labelled, computed with `source_object` counted
      right and counted wrong. Below 60% both ways: continue; at or above both ways: stop and report on the issue;
      straddling: the owner decides. `CENSUS.md` in this directory (docs PR, one reviewer): population, volumes per
      diff and diff line, precision per cell with intervals and causes, rater counts, gate result; aggregates only.

### Task M-4: source census (recall set)

- [ ] `m/census.py` (DESIGN 6, source census), run before any head build exists: 400 changed lines of the 180
      py/js/go diffs with at least one value-read identifier (the base trace's `prov-*` lines are not used to
      select), `random.Random(20261122)`, stratified by family, with an oversampled stratum of lines inside functions
      that a syntax scan in `m/census.py` marks as handlers (a route decorator or registration, or a parameter
      named `request`, `req` or `ctx` or typed `*http.Request` or `*gin.Context`); weights per item. Each item
      carries the line, its whole function and the file's imports.
- [ ] The M-3 readers label every value read on each line (variable, determining definition line, origin class of
      any severity, or `insufficient_evidence`); third read on disagreement; `m/labels/census-final.jsonl`. Score the
      base build on it (recall per class, weighted) in `CENSUS.md`. The completed M-4 used a syntax-free candidate scan,
      with 400 lines and 422 final tokens in `m/census2/`. M-4 did not tally definition-only changes. Preserve that
      limitation at G. See DESIGN 7 for the measured premise conflict.

## Stage T and E: executable matrix (3.5 days)

### Task T-1: rows, fixtures, data-driven test

**Files:** create `tests/provenance_matrix/rows.jsonl`, `tests/provenance_matrix/cases/<primary-extension>/<id>/` (one
directory per row, outside the golden walker over `tests/fixtures/`), `tests/provenance_matrix/README.md`, `tests/algo/novel/provenance_matrix_test.rs`. Modify
`tests/algo/novel/main.rs` (`mod provenance_matrix_test;`) and the three coverage lists.

Row fields: `id`, `group`, `class`, `files` (under the row's directory; several for repository-local shadowing),
`diff` (file → lines; only these files are parsed, as the review loader does), `want` (`[{file, line, var,
origin}]`, `origin` an `Origin::name()` or `none`: the expected product output, always executed), `exact_lines` (no
other provenance finding on those lines, always executed), `truth` (what the value really is), `accepted_loss`
(null or the L-entry for a genuine gap between `want` and projected `truth`), `source` (`probe`, `corpus`, `grammar`,
`enumerator:<seat>`). The test parses the `diff` files with `Language::from_path`, builds `CpgContext::build(&files,
None)`, runs `provenance_slice::slice_in_repo(&ctx, &diff, Some(row_dir))` (from S2; until then `slice(&ctx,
&diff)`), reads origins by the prefix `variable '<var>' has <origin> origin:`, fails naming every failing row id,
and reports a row whose output now equals its projected `truth` while `accepted_loss` is set. Archive the original truth and retain the richer labels in settled rows. Apply each reviewer-confirmed fixture correction under DESIGN 5.7.
Map `constant`, `derived`, `non_variable` and `source_object` to `none` in row validation, matching and accepted-loss
improvement detection. Compare the complete vector and every `exact_lines` expectation. Retain adjudication for
genuine improvements. DESIGN 5.7 describes invalid original assertions and their corrected fixtures.

```rust
// tests/algo/novel/provenance_matrix_test.rs
const ENABLED_GROUPS: &[&str] = &["control"];
#[test]
fn test_provenance_matrix_rows() { /* rows.jsonl; skips disabled groups only; accepted-loss rows run */ }
```

The original T checkout contains 220 rows. D applies to all 893 enumerated fixtures in `d/rows-flat.jsonl`.
`D-groups.tsv` assigns each input ID exactly once. A control already satisfies the final vector on base including
all exact-line negatives. S1 owns `structural`, S3 `py`, S4 `js` and S5 `go`. S2 and S6 have no matrix group.
For a retired assertion preserve the key-to-successor mapping and verify the corrected fixture on base. Keep already-green corrected sites in `control` and give any
additional payload twin a separate base-gradeable key. A retirement is never behavioural RED. Matrix axes and operations are specified in DESIGN 5.1–5.5. The authoritative
tentative-row and additional expectation decisions are `D-rows.tsv`. In particular a bare HTTP response or cursor
is silent, a fetched value is payload, and source selectors are judged at the read. Input checks, comparisons,
status flags and producer handles have their own slots. Java, C/C++, Rust, Lua, Bash and Terraform use the
language-specific receiver/output rules in DESIGN 5.3.

- [ ] Write rows and fixtures (sources: `~/prism-evidence/lanes-2026-11-20/probes/lane6/`, `research/lane6-data/`,
      the design review's probe cases under `~/prism-evidence/lanes-2026-11-20/reviews/astra-r1/lane6/probes/cases/`;
      public code shapes only).
- [ ] Enable every group once on base and record the failing rows in `t/red-on-base.txt`; commit with
      `ENABLED_GROUPS = &["control"]`; a `control` row red on base moves to the group of the slice that changes it.
      Rows that pin a main defect (DESIGN 9) are named in `t/main-defects.txt` with the issue to file.
      Gates: section 5 items 1, 2, 4, 8; one reviewer (test-only).

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION section 6. Axes: origin × family × proof route; predecessor labels and kills (mixed
      live and doubtful, early exits, loops, same-line writes, field siblings, genuine joins); occurrence roles
      (member and qualified names, computed keys, shorthand properties, callees, receivers, module versus local
      import, types, declarations, repeated tokens); binding forms (tuple, destructuring, walrus, `with`, `for`,
      `:=`, `var`, multi-value, out-parameters); positions (module, package, class body, lambda, closure, multiline
      and member-only formals, grouped versus initialised declarators); handler shapes and framework × accessor ×
      binding (registered and not, renamed and grouped parameters, rebinding, fields versus calls, tuple and
      multiline assignments); per-callable value semantics (a real read paired with a helper, constructor, close or
      metadata call of the same library); transfers (ignored, selected and transformed operands, unknown calls);
      import forms (alias, `from`, star, `require`, dynamic, Go alias, dot and versioned) and site refusals;
      repository-local modules on disk but not in the diff; decoy placements; pattern-table triggers per language
      (ordinary and `~`, suffixes, overlapping tables, embedded expansions, foreign-library calls); parse errors;
      joins. M-3's causes are rows. Every new row carries a truth.

## Stage D: design of record (1.5 days)

- [x] The design seat reconciled 893 fixtures and their supplied base outputs against the frozen v1.1 labels.
      DESIGN 5 covers occurrence kinds, reaching writes, callable/argument/return contracts, parser input,
      data and handle depths, handler formals, browser and receiver rules, code-only language tables, identity,
      trace, legacy fixtures, cross-check exceptions and accepted losses. The two D ledgers are the implementer's
      per-row instructions. DESIGN 7 records the measured gate conflict. Every tier was still adding cases at
      its cap. Preserve this open-class limitation for the reviewers and controller under ORCHESTRATION §§7–8.
- [ ] Both reviewers independently decide the amended record. Design review cap is two rounds. Route the
      frozen-recall premise conflict under ORCHESTRATION §8. Keep the numerical gates and the M labels in custody.
- [ ] After D approval the implementer materializes the full `tests/provenance_matrix/rows.jsonl` and fixtures
      from the enumeration and ledgers, sets `want` and `accepted_loss`, settles all tentative flags and retains the
      evidence for every retired assertion. Replace the provisional two-step text in the matrix README with both
      three-step budgets and their boundary pairs. Run all groups on the exact M/T base with a harness that accepts
      projected truths. Record `t/red-on-base-final.txt` with the final file SHA-256, binary identity, per-ID
      output and exact-line differences. Reclassify already-green expectations as controls and reconcile any
      difference from the D group ledger. Each slice cites only actual REDs from that receipt. The recorded
      252 green / 641 red counts describe the previous vectors.

## Stage I: slices

Order: S1 removes most noise in every language without identity; S2 makes identity reachable with no behaviour
change; S3 to S5 move one family each to proofs (Python first: largest share of the population and of the noise);
S6 changes the hint once all three are on proofs. Each slice branches from updated `main`. S1 and S3–S5 enable their
group in the first commit and cite behavioural REDs from `t/red-on-base-final.txt`. S2 and S6 use the unit and
projection checks in their own sections. Before review, write the prediction and run the
population with merge base and head (the previous slice's head run serves as base only when
`git diff --quiet <previous head> <merge base> -- src crates/prism-review/src Cargo.lock vendor` holds; otherwise
build and run the merge base); put `m/compare.py`'s tables, its reconciliation sums and each removed class with its
count in the PR body. **Precision check before merge** (DESIGN 7): a slice that changes default output for a
language labels a sample of its head's findings for that language (M-3's sampler, cells and readers; S1 for every
measured family, S3 Python, S4 JS/TS, S5 Go) and scores the census recall per class; a cell under the Precision gate
blocks the merge and goes to the design seat. Each slice publishes its actual recall and all misses. DESIGN 7
documents why the original final recall gate is blocked on all six populated classes. Route that premise conflict
under ORCHESTRATION §8 before treating an implementation result as acceptance. Each slice also covers ORCHESTRATION section 5 items 1 to 4,
8, 9, 10 (`CLAUDE.md` provenance paragraph, `ALGORITHMS.md` entry 23), 11; two reviewers.

### Slice S1: structural fixes for every language (4 days)

**Files:** create `crates/prism-review/src/algorithms/provenance/mod.rs`, `provenance/occurrence.rs`,
`provenance/patterns.rs`, `provenance/patterns_tests.rs`, `provenance/occurrence_tests.rs`,
`crates/prism-review/tests/cli/provenance_trace_test.rs`; modify `crates/prism-review/src/algorithms/mod.rs`
(`mod provenance;`), `provenance_slice.rs` (tables and classification move out; `slice()` delegates;
`selected_origin_evidence` is replaced by the selected edge), `crates/prism-review/tests/cli/main.rs`
(`mod provenance_trace_test;`), coverage lists, the re-fixtured tests below.

Implement DESIGN 5.1, 5.2.1, 5.3 and 5.5. Select runtime AST occurrences per grammar, resolve lexical bindings
and pair expression, result and destination slots before joining anything. Keep value-bearing imported objects
and ordinary receiver formals. Drop declaration positions, syntax, selector spellings and callees. Attribute the
actual selected accessor to its receiver at the read. Emit one public finding per name and line and trace all
contributing and excluded occurrences.

Use a graph Use only for the matching nonempty bytes, path and execution owner. Inspect individual predecessor
edges with `dfg.label` and temporal/path feasibility, including must-overwrites, output arguments, loop exits
and deferred or eager evaluation. Use syntax when semantic evidence is absent and preserve explicit kills.
Never infer a loop-carried write from lexical containment alone. Respect same-file module/class scopes, captures,
structured assignments and branches, per-slot overwrites and actual exceptions. Join by DESIGN 5.1.2's stable
risk and write/operand ordering. Use the selected edge's evidence grade. A propagated chain or syntax-only proof
is unlabeled.

Set `PROPAGATION_DEPTH = 3`. Implement only the value-preserving forms in DESIGN 5.2.1 and the applicable
language tables, with an explicit boundary twin. Match gated code tokens on the supplying operation/slot, using
both boundaries, real substitutions and the operation's actual value/destination semantics. S1 owns the complete
C/C++, Java, Rust, Lua, Bash and Terraform tables, including JDBC cursor versus column, Rust/Lua explicit C FFI,
Bash read/fd state and HCL traversal-root selection. Its provisional Python/JS/Go patterns use only their own
families. The public origin, category and severity contract stays in force.

- [ ] Enable `structural`: RED. Unit tests in `patterns_tests.rs`: `boundary_rejects_format_for_form`,
      `boundary_rejects_fetch_config_for_fetch`, `boundary_checks_every_occurrence` (`formatted form`),
      `call_entry_needs_whole_callee_token` (`spread(9, 5)` does not match `read(`),
      `classifier_reads_value_not_defined_name` (`args = sorted(tokens)` → none),
      `code_text_blanks_comments_and_strings`, `code_text_keeps_shell_substitution`,
      `terraform_tables_do_not_run_on_python`, `join_prefers_concern_over_info`, a tie on reaching write order and a user-input/hardware risk pair. In `occurrence_tests.rs`:
      `zero_width_use_is_not_examined` (the `occ-py-attr-same-name` shape: no `label` occurrence on line 15),
      `local_shadowing_an_import_is_examined`, `callee_is_dropped_with_role`, `killed_predecessor_contributes_nothing`
      (`branch.py` shape → no origin, reason `killed-only`), `later_definition_without_loop_contributes_nothing`,
      `loop_carried_definition_contributes`, `module_level_read_takes_syntax_route`. CLI tests:
      `trace_reconciles_every_examined_occurrence` (each examined occurrence has exactly one `prov-var` or
      `prov-unknown`), `no_trace_without_the_variable`.
- [ ] Implement; GREEN for `control` and `structural`.
- [ ] Apply the existing-fixture inventory in DESIGN 5.8 to S1's affected tests. For confidence delivery, prove
      the Django request by type or same-file registration and give the nameonly case an actual surviving
      `CfgIncomplete` edge. Check the emitted `user_input` origin as well as the category and grade. Check the selected DFG output, an exact edge, a propagated same-function alias and
      a both-arms kill. Preserve taint's own tests. Generic ioctl status, recv's count and Rust read_line's
      result are payload negatives. Pair each with the read of its true destination. Tighten C/C++ getenv, cin, recv, fgets and copy_from_user to the actual changed value read. Record the source, old false assertion and
      replacement for each changed test.

- [ ] Run (memguard, `RAYON_NUM_THREADS` 1 and 4): `cargo test --test algo_novel provenance`;
      `cargo test --test integration -- dfg_label_parity_test:: targets_mapping_test:: coverage_test::`;
      `cargo test --test cli -- min_confidence provenance_trace_test:: targets_test:: validation_test::`;
      `cargo test -p prism-review --lib provenance`; and filter `provenance` in `lang_c`, `lang_cpp`, `lang_bash`,
      `lang_terraform`, `lang_java`, `lang_go`, `lang_typescript`, `lang_tsx`.
- [ ] Gates include Contract, accumulated matrix groups, precision on all seven measured families and census
      recall on Python, JS/TS and Go. Lua, Bash and Terraform have matrix coverage only (DESIGN 8 decision 6).
      Report cell sizes, every genuine loss and every changed control. A bare API spelling is insufficient to
      predict a green check.

### Slice S2: shared identity access and the repository root, no behaviour change (2 days)

**Files:** create `crates/prism-review/src/algorithms/taint/request_sources.rs`, `provenance/identity.rs`,
`provenance/identity_tests.rs`; modify `taint.rs` (`mod request_sources;` and
`pub(crate) use request_sources::{framework_request_sources, RequestSource};`), `targets/mod.rs` (both
`dependency_*` modules `pub(crate)`; `repo_root: Some(&meta.repo_root)` in `project`), `dependency_hint.rs`
(`SiteContext.repo_root: Option<&'a Path>`; `SiteContext`, `root_identity`, `RootIdentity` `pub(crate)`, or added
as lane 2's PLAN S1 states, with its three tests, if not on `main`), `dependency_identity.rs` (`proven_path`;
`library_kind` visible), the `SiteContext` literals in `dependency_hint_tests.rs` and
`dependency_hint_binding_scope_tests.rs`, `provenance_slice.rs` (`slice_in_repo`), `crates/prism-review/src/api/run.rs`
(a `SlicingAlgorithm::ProvenanceSlice` arm in `run_algorithm_raw` calling `slice_in_repo(ctx, &inputs.diff,
Some(repo))`), `provenance/mod.rs`.

```rust
// provenance_slice.rs
pub fn slice_in_repo(ctx: &CpgContext, diff: &DiffInput, repo: Option<&Path>) -> Result<SliceResult>; // slice() = None
// taint/request_sources.rs: wraps the per-language detectors detect_framework_sources dispatches
pub(crate) struct RequestSource { pub line: usize, pub target: Option<AccessPath>,
    pub start_byte: Option<usize>, pub byte_scope: Option<(usize, usize)> }
pub(crate) fn framework_request_sources(file_path: &str, parsed: &ParsedFile) -> Vec<RequestSource>; // sorted, deduped
// targets/dependency_identity.rs: kind_for_binding's proof without the table lookup
pub(crate) enum PathProof { Import { module: String, member: Option<String>, rest: Vec<String> },
    Builtin { path: String } }
pub(crate) fn proven_path(parsed: &ParsedFile, ctx: &SiteContext<'_>, at: Node<'_>, root: &str,
    rest: &[&str]) -> Option<PathProof>;
pub(crate) fn library_kind(language: Language, module: &str, rest: &[&str]) -> Option<&'static str>; // was private
// provenance/identity.rs
pub(crate) struct IdCx<'a> { pub site: SiteContext<'a>, pub requests: Vec<RequestSource> }
pub(crate) fn id_cx<'a>(file: &'a str, parsed: &'a ParsedFile, ctx: &'a CpgContext<'_>,
    repo: Option<&'a Path>) -> IdCx<'a>;
pub(crate) fn call_path(cx: &IdCx<'_>, parsed: &ParsedFile, call: Node<'_>) -> Result<PathProof, &'static str>;
pub(crate) fn read_path(cx: &IdCx<'_>, parsed: &ParsedFile, expr: Node<'_>) -> Result<PathProof, &'static str>;
// Represent the registration, receiver binding, accessor bytes, supplying expression and return/output slot.
// Keep the underlying raw seed independently so the census can audit a refused or augmented match.
```

`CpgContext` has no repository root. Pass the actual root through `run_algorithm_raw` and keep `slice(ctx,diff)`
as the no-root entry. Without a root a Python import proof is refused (`no-repo-root`). Pure built-ins can
still be proven. Check on-disk import shadowing beyond changed files. Use lane 2's shared import proof at
the **specific AST occurrence** for calls and non-call paths, preserving the original exported member. Layer
DESIGN 5.4's use-site binding, alias/replacement, repository and runtime reachability checks on the provenance
side. Keep native targets resolution and taint seeds behaviourally identical. Expose their existing logic once.
A raw handler line/target is only a candidate. A matched accessor must select this receiver, subexpression,
binding and data slot under DESIGN 5.2.3. Framework-spec strings alone cannot identify data.

- [ ] Tests first in `identity_tests.rs`: `proven_path_os_environ_get_is_import_os` (module `os`, rest
      `["environ","get"]`), `proven_path_refuses_shadowing_parameter`, `proven_path_refuses_repo_local_module_on_disk`
      (`requests.py` in a temp repository root, not in `known_files`), `proven_path_refuses_python_import_without_root`,
      `proven_path_process_env_is_builtin`, `proven_path_go_os_getenv`, `named_import_keeps_original_member`,
      `refuses_reassigned_require_binding`, `refuses_local_require_loader`, `refuses_catch_bound_fetch`,
      `refuses_replaced_member`, `refuses_go_receiver_named_like_package`, `request_sources_flask_route_lists_assignment_line`,
      `request_sources_express_lists_handler_param`, `request_sources_net_http_lists_formvalue_line`,
      `request_match_go_same_line_other_target_is_false` (`name := r.FormValue("q"); count := 7` → `count` false),
      `request_match_django_tuple_pairs_slots`. Record missing symbols as a compile-only observation. Demonstrate
      a behavioural RED on the compiling refusal/slot assertions before claiming RED credit.
- [ ] Add paired cases for static CJS destructuring, Python `TYPE_CHECKING` versus runtime writes, a `.pyi`
      beside an actual runtime module, renamed registered formals, a zero-target Go seed, response/request
      slots on the same line and a shared AST accessor absent from a spec's declarative sources.
- [ ] Build `IdCx` per diff file. Trace every raw `prov-request-source` to every candidate changed read of its
      binding, retaining refused keys, per-slot results, byte ranges and independently observable witnesses.
      Freeze `g/request-uses.tsv` before S3 and publish raw, missing-target and augmented inventories separately.
      The provenance classifier cannot choose its own denominator. Follow DESIGN 5.5 for the four permitted
      false-seed predicates and the empty genuine-payload exception set.
- [ ] Gates include accumulated matrix groups, `--test cli targets_test::`, `--lib targets::dependency_hint`
      and identical product findings to S1 on the population. Trace-only additions supply the independent
      request cross-check, which is scored separately from the M-4 census.

### Slice S3: Python origins by proof (3.5 days)

**Files:** create `provenance/python.rs`, `provenance/python_tests.rs`; modify `provenance/identity.rs`,
`provenance/mod.rs` (Python on the identity regime), `crates/prism-review/tests/cli/targets_test.rs`, and the tests
stage D named for Python.

```rust
// provenance/identity.rs
pub(crate) const HANDLE_HOP_DEPTH: usize = 3;
pub(crate) trait DefReader {
    /// The value node a definition binds (the slot paired with it); None when the binding form is not modelled.
    fn value_of<'t>(&self, parsed: &'t ParsedFile, def: &VarLocation) -> Option<Node<'t>>;
    fn origin_of_value(&self, cx: &IdCx<'_>, parsed: &ParsedFile, value: Node<'_>, target: Node<'_>)
        -> Result<(Origin, Route), &'static str>;
}
pub(crate) fn origin_by_proof(ctx: &CpgContext, cx: &IdCx<'_>, env: &FileEnv<'_>, reader: &dyn DefReader,
    def: &VarLocation, depth: usize) -> Result<Verdict, &'static str>;
// provenance/python.rs
pub(crate) struct PythonReader;   // impl DefReader
```

Apply DESIGN 5.2.1–5.2.3 and 5.4. On Python enable only proven built-in/library/framework operations.
Use `HANDLE_HOP_DEPTH = 3` including constructor and handle transitions, with separate three-transfer alias
budget and one live receiver definition (or a specified concrete formal). Keep database cursors and HTTP
responses as internal handles. Fetch/iteration, selected body, remote headers and status text are payload.
Bind imported Flask request in and outside routes. Use actual typed/registered Django request, registered
FastAPI parameter/dependency position and canonical Django settings. Generic parses preserve argument 0 and
the exact supplied origin, including proven config-file inputs. Python built-ins, argv/stdin, environment,
ORM/DB and HTTP operations and transfer exceptions are enumerated in DESIGN 5.2.2.

- [ ] Enable `py` against the final base receipt. Pair sqlite cursor/scan materialization and HTTP body with
      bare connection, cursor, session and response negatives. Cover the third/fourth handle and alias
      boundaries, constructor under `with`, one killed versus two live receiver definitions, local runtime
      shadow versus `.pyi`, decorators/handlers, Django metadata, ignored arguments, parser literals and an
      HTTP-fed parser. Include original-member import, no-root refusal, opaque helper and factory cases.
      Verify every enabled group and the recorded accepted-loss reason.

- [ ] `targets_test.rs` `default_run_emits_all_five_producers_and_live_mappings`: its `user_input` target came from
      the repository-local `request = Request()` (`tests/fixtures/targets/client.py:10,15`), a substring artefact.
      Assert a provenance target for `variable 'user'` with `function_param` (line 14) and none containing
      `user_input`; the fixture stays.
- [ ] Preserve `test_provenance_python_request_form_origin` and `test_provenance_python_cursor_execute_origin`
      as payload positives via a repo-aware entry. Add a dedicated no-root negative for the compatibility
      entry. See DESIGN 5.8 for the corrected Django evidence fixtures and their expected grades.
      Gates include the per-cell precision check and weighted class recall on the 100 Python diffs,
      the Python request cross-check, and Contract. Report DESIGN 7's actual affected keys.

### Slice S4: JavaScript/TypeScript origins by proof (3.5 days)

**Files:** create `provenance/js_ts.rs`, `provenance/js_ts_tests.rs`; modify `provenance/mod.rs`,
`docs/features/language-coverage/js-ts-known-gaps.md` (a provenance entry: what is proven, what is not).

Implement DESIGN 5.1, 5.2.1, 5.2.3, 5.2.4 and 5.4 for JavaScript, JSX, TypeScript and TSX. Pair declarator,
destructuring, callback, Promise result and output slots. Peel erased type syntax. Respect overwrite, default,
key, rest and comma-expression value semantics. Bind Express, Fastify, Koa and NestJS by installed handler
and exact request slot, including error middleware, decorators and renamed formals. Request/context objects,
writers and metadata are silent. Proven inbound fields are input. Browser cookie, URL text, prompt and
actual form values are input. Process environment/argv require an unshadowed global or canonical import.
A fetch/Axios response is a handle. Its wire payload is external. A pg materialized query result/row is
database. A cursor, status, helper, replaced parser/adapter or callback error is not a payload.

- [ ] Enable `js` against the final base receipt. Pair each library's positive payload with its same-library
      negative. Include browser-global shadowing, an unregistered form versus registered renamed or
      destructured formal, both middleware error slots, static named CJS/ESM imports and replacement cases,
      pg row vs rowCount, HTTP body vs bare response, metadata, config file vs literal/reviver and the limits
      of data/handle transfer. Keep the genuine cookie and process.env existing fixtures.
- [ ] Gates include the per-cell precision check and weighted class recall on the 40 JS/TS diffs,
      the JS/TS request cross-check, accumulated matrix groups and Contract.

### Slice S5: Go origins by proof (3 days)

**Files:** create `provenance/go.rs`, `provenance/go_tests.rs`; modify `provenance/mod.rs`.

Implement DESIGN 5.1, 5.2.1, 5.2.3, 5.2.5 and 5.4. Bind `:=`, var specs, package initializers, blank/error
slots, receiver formals and actual output addresses. Type or registered position proves the inbound net/http
request or gin context, including non-call fields. Pair every accessor with its own RHS, argument or changed
receiver occurrence. Use per-operation CLI/flag parse, os environment, Viper and JSON input rules. Go
`database/sql` and pgx `Query`/`QueryRow` give cursor/row handles. Only an actual addressable `Scan` destination
on its write route is database. Traditional GORM writes output arguments. Generic `Find` returns records in
slot zero. HTTP clients return response handles. `Body`, headers and remote status text can supply external
payload, and `io.ReadAll`/decoder operations preserve the selected reader. Go errors, presence flags, counts,
writer state and control metadata have independent slots.

- [ ] Enable `go` against the final base receipt. Check `Query` vs `Scan`, `ErrNoRows` preserving the previous
      value, an opaque partial-conversion failure (L4), GORM writes/status and a typed receiver. Check
      lookup/presence and parse/error pairs, stdin or request-body decoder versus local bytes, gin output
      arguments, Viper instance input and ignored fields. Pair a one-byte request value with another
      assignment on the same line, and a true request accessor with block-local shadowing. Keep the real
      FormValue and Viper payload fixtures. Verify all groups.
- [ ] Gates include the per-cell precision check and weighted class recall on the 40 Go diffs, the Go
      request cross-check, accumulated groups and Contract.

### Slice S6: the targets hint rule (0.5 day)

**Files:** modify `crates/prism-review/src/targets/mapping.rs` (the `("provenance", "untrusted_origin")` arm of
`map_finding`), `tests/integration/targets_mapping_test.rs`.

`db` and `network` hints are attached only when `Language::from_path(&finding.file)` is Python, JavaScript,
TypeScript, TSX or Go: after S3 to S5 those languages give `database` and `external_call` only by library or
built-in identity, and the pattern languages never do. Detail text unchanged.

- [ ] Test first: `provenance_hints_only_for_identity_languages` (`database` in `a.c` → no hint; `external_call` in
      `a.go` → `network`; `database` in `a.ts` → `db`). RED, implement, GREEN.
      `maps_provenance_taint_peer_and_fallback_rows` passes unmodified.
- [ ] Gates: Hints on the population (`--targets`; every ordinal's targets artifact complete, else the gate fails):
      every hinted database/external-call target joins the winning `prov-var` record and the matching terminal
      library/built-in proof, directly or through the recorded aliases, callback or output slots. A request seed,
      opaque handle, pattern or package category alone cannot discharge the check. Include direct accessors,
      propagated values and a pattern-language finding in projection tests. Apply Contract.

## Stage G: lane gate (2.5 days)

- [ ] Final head run with `--targets`. `g/REPORT.md`: volume by family × origin × severity, base against head, with
      `m/compare.py`'s reconciliation, both cumulative (M base to final head) and per slice; each removed class with
      its count (owner decision 2's non-variable names from `prov-drop`, examined unknowns by `prov-unknown`
      reason). Report that the M-4 frame did not count definition-only changes. Label any later separate tally with its own frame.
- [ ] Head samples (DESIGN 6): M-3's sampler, cells, seed and readers over every severity (`g/labels/head-*.jsonl`).
      **Precision** per DESIGN 7 in every cell, with intervals; small cells labelled in full and listed.
      **Recall** per origin class against `census-final.jsonl`, weighted, ≥ 95%, every loss listed with its
      `prov-unknown`, `prov-drop` or actual wrong-origin trace and stage-D accepted losses marked. Base retention
      reported beside it. Include class denominators, the object-negative ledger, and insufficient/unresolved
      counts. Preserve the original gate and raw labels. DESIGN 7 bounds every populated class below 95% under
      the frozen frame and proof scope. Re-scoring requires the per-key evidence and independent review in
      ORCHESTRATION §§7–8. Publish any authorised successor score alongside the original.
- [ ] **Cross-check.** Each genuine live payload read in the frozen `g/request-uses.tsv` has a `user_input` finding.
      Reconcile all raw entries and only exclude the four independently witnessed false-seed predicates in
      DESIGN 5.5. List exact keys, bytes and witnesses. **Hints**, **Contract** on the final head. Post the tables on the tracking issue.

## Close-out (1 day)

- [ ] Final docs check: the `CLAUDE.md` provenance paragraph (examined set, regimes, proofs, trace, losses), the
      algorithm-map line, `ALGORITHMS.md` entry 23, the S4 known-gaps entry. Optional
      `mutants/provenance-identity.json` (boundary, join, kill and later-definition rules, hop depth, hint; not run
      per PR).
- [ ] File the main defects the review found, each with its probe and the row that pins it (`t/main-defects.txt`):
      a Python attribute read `obj.label` gives a zero-width `Use(label)` fed by an `exact` edge from a same-named
      local; a Java `T x = e;` Def is keyed by the whole declarator text; backward edges from a definition after the
      use with no loop (JS hoisted `var`, Python formal in `try`, C buffer before `recv`) are labelled
      `cfg_incomplete`; the taint request recogniser walks only Go calls though its table lists non-call fields,
      gives Go seeds no target, accepts a Django context by substring anywhere in the file, seeds every name of a
      Django tuple assignment, and accepts Express/Fastify/Koa handlers only by parameter name; the targets
      dependency identity does not refuse JS reassignment, a local `require` loader, catch-bound names, Go method
      receivers or member replacement, drops a named import's original member, and matches Go and JS libraries by
      package without the call path.
- [ ] Follow-up issues the numbers justify: helpers, factories, containers, cross-file origins; more identity
      languages (owner decision 4), and observed gaps in pattern operations, shell/HCL slots or browser sources beyond the tables.
      Refresh `HANDOFF.md`; summarise and offload raw outputs; delete the corpus clones and `target/`.

## Acceptance for the lane

`CENSUS.md` merged with the go gate passed; S1 to S6 merged with both reviewers approving and each behaviour slice's
precision check passed; on the final head every DESIGN 7 gate holds (Precision, Recall, Cross-check, Hints, Contract)
with losses listed. Post the report on the tracking issue and refresh the docs and handoff. The original Recall gate is
unmet under DESIGN 7's frozen-census bounds. An approved design or a passing matrix cannot close that gate.

**Estimates:** M 4.5, T and E 3.5, D 1.5, S1 4, S2 2, S3 3.5, S4 3.5, S5 3, S6 0.5, G 2.5, close-out 1; total 29.5 days.

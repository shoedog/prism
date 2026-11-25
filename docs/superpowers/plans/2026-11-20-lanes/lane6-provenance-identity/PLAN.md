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
position). Decision 7 authorizes one statically imported forwarding level. The loader supplies the reviewed snapshot for that proof and its resolution metadata. Decision 5 adds Project payload contracts after S5/S6. The public review entry keeps its no-snapshot refusal path. The targets mapping keeps parsing the unchanged
description and attaches a hint only for identity languages. The reviewed-input loader under `src/api/` carries the optional immutable snapshot. The graph and persistent cache keep their existing formats.

**Tech stack:** Rust (prism-review crate, tree-sitter); Python 3 for the population scripts.

**Spec:** `DESIGN.md` here (5.1 examined set and reaching definitions, 5.2 origins, tables and propagation, 5.3
other languages, 5.4 shared identity, 5.5 output and cross-check, 5.6 layout, 5.7 losses, 5.8 changed legacy
assertions, 5.9 new fixtures, 6 measurement, 7 frozen keys and gates, 8 owner decisions). Use `D-rows.tsv` and `D-groups.tsv` in this directory.

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
- Deliberate, disclosed in each PR body: `<name>` is the use's `path.base`, except for the qualified HCL binding address in DESIGN 5.1.1; `related_lines` are the selected
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
  The first affected S1c activation lands the pattern-language hint guard. S6 verifies the complete identity-terminal join. S3 changes the one legacy targets fixture assertion named in DESIGN 5.8.
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
`diff` (file → lines used to build the review CPG; S4b reads one permitted adapter from the same loader snapshot and the later opt-in pack verifies only named project contracts), `want` (`[{file, line, var,
origin}]`, `origin` an `Origin::name()` or `none`: the expected product output, always executed), `exact_lines` (no
other provenance finding on those lines, always executed), `truth` (what the value really is), `accepted_loss`
(null or the L-entry for a genuine gap between `want` and projected `truth`), `source` (`probe`, `corpus`, `grammar`,
`enumerator:<seat>`). The test parses the `diff` files with `Language::from_path`, builds `CpgContext::build(&files,
None)`, runs the loader-bound provenance entry for immutable snapshot fixtures (from S2, with `slice(&ctx, &diff)` and a separately supplied path as refusal controls for the extra source read), reads origins by the prefix `variable '<var>' has <origin> origin:`, fails naming every failing row id,
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

The original T checkout contains 220 rows. Apply the two D ledgers to the 893 enumerated fixtures. Materialize the 47 independently keyed fixtures in DESIGN 5.9 in addition (940 specified IDs, including parameterized twins). `want_origin` sets the original sites in order; append each explicit `add_sites` entry and retain original select/drop negatives and exact lines. `rename_sites` applies HCL binding-address probes. Record each confirmed positive or loss at its qualified address in the settled truth vector and the bare root as a negative. Preserve raw labels and historical keys. Keep per-site evidence and both reviewers' confirmations for oracle corrections, including HCL binding names. Preserve whole materialized pg results under v1.1. A corrected vector must pin both its supported and silent slots. Never count a retired assertion as a RED.

`D-groups.tsv` predicts one group per original ID. Enable `control` continuously. Noncontrols `py/js/go` exercise complete family activation including structure; `pattern-c/cpp/java/rust/lua/bash/hcl` each exercise one structural language (full names carry the `pattern-` prefix). S1a, S1b, S2 and S6 have no product matrix group of their own. Assign each of the 47 additional fixtures after base replay, to control if already exact, otherwise its responsible family or its forwarding/contract increment. Seed the seven repaired original fixture groups conservatively as JS and regrade their new bytes on base. Always execute adapter/native-resolver positives and their negatives on the candidate activation head even if assigned to control. HCL names affect the public target key, so a base finding on bare `var` cannot satisfy `var.x`. Selected payloads, bare envelopes, Go request direction and deferred language operations follow DESIGN 5.1–5.7.

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

## Stage D • section 8 amendment

- [x] Write independent alternatives before reading the held sheet, then pool and compare them in `S8-OPTIONS.md`. DESIGN 7 joins every one of the 180 decided source tokens to the final census and supplied base outputs. The ledgers retain all 893 original IDs and the old base receipt.
- [x] Both final amendment reviews rejected the pre-fold design. The owner authorized one priced fold. DESIGN’s change log disposes of each closed finding and records B-1’s framework-wide supplier rule.
- [x] Decision 7 is YES. Decision 5 selects option (iii), with no unproved findings visible. The recovery slice is in lane scope. Default publication awaits the owner’s acceptance or rejection of measured class residuals at G.
- [ ] Materialize, hash and run the **final** 940-ID matrix on the exact M/T base. Record the base binary, fixture/harness hashes, per-site actuals, unexpected exact-line names and ID/grade changes in `t/red-on-base-final.txt`. Publish the base regrade separately from the supplied 252/641 old vectors and the pre-fold 303 and current 308 predicted controls among 893. Settle tentative flags; retain any accepted loss that now improves for independent adjudication. Keep each new ID separate from the D group inventory.

## Stage I • slices and activation

S1a and S1b build the shared reader. S2 freezes the native request inventory before any new reader activates. S1c activates each structural family on integration and ships the pattern hint guard with the first affected family. S3–S5 activate identity families on integration. S6 verifies hints. Project payload contracts follows both S5 and S6 and precedes G. Keep all default-output changes on integration until the owner decides on the measured residuals at G.

For each behavior slice compare merge-base and head through the same product entry on the affected population with `m/compare.py`. Reuse a receipt only when binary/source, trees, options and diff specs match. Report same-origin weighted recall, retention, failed and wrong-origin keys and the 85% gate in every populated emitted cell (including a fully adjudicated small cell). Check the amended R set at family activation and the recovery R set after the verified pack. Apply ORCHESTRATION §§5/7 for product review and yield loss. Decision 6 retains matrix-only evidence for Lua, Bash and Terraform.

### Slice S1a • value-read occurrences, no default-output switch

Create `provenance/{mod,occurrence}.rs` and the occurrence tests under the review crate. Keep the public algorithm delegating to the old output path while building the new reader. Implement DESIGN 5.1.1 roles, byte spans, execution owners, binding identity and HCL qualified names. Cover each grammar incrementally; use the AST for interpolations and code-only segments. Preserve lexical iterator names, separately evaluated indexes and HCL traversals sharing a line.

- [ ] Pin a one-byte read, same-spelling binder/selector, method base, direct callee, declaration, import-value versus namespace, malformed sibling and escaped interpolation on the baseline. Assert the exact admitted/dropped bytes and name. Include `var.a` and `var.b` on one line and a lexical binder named `var`. Keep old public outputs identical on controls.
- [ ] Separate AST-role work into grammar-sized changes if needed; a single review may not introduce all grammar-specific flow models.

### Slice S1b • live definitions, transfers, joins and trace

Implement DESIGN 5.1.2, 5.2.1 and 5.5 using the S1a occurrences. Add trace and route tests before family activation. Reuse the selected edge's actual grade. Syntax and propagated evidence remain unlabeled. Pure helper tests need a compiling behavioural RED and a counterexample.

- [ ] Bind nonempty graph Uses at the correct bytes. Account for path feasibility, must-writes, projection kills, loop zero-iteration/back edges and exits. Pair a repeated unchanged truthiness test with a version that writes to the guarded field between tests (c0207). Syntax can supply missing semantics but cannot resurrect a killed graph value. Pair tuple/destructuring/output slots and test earlier-write and same-write operand tie ordering, including the pre-recovery c0068 ordinary-formal winner. Preserve both operands and their terminal evidence for the priced equal-tier change.
- [ ] Keep independent value and handle counters on each path bounded by distinct normalized states. Terminate a cycle without losing an independent reaching terminal. Test a fourth acyclic alias, four distinct handle-producing transitions and mixed alias/handle edges. Record distinct states, visit counts, peak memory and elapsed time. Avoid an arbitrary alias count chosen from matrix IDs.
- [ ] Test `prov-var/unknown/drop` reconciliation, exact-line deduplication, both counts, winning/dropped alternatives and selected Def lines. Test default trace off; maintain an output-identical production comparison. Split binding/flow work at grammar boundaries if it cannot fit the review cap.

### Slice S1c • structural pattern activations, one family per review

After S2, activate the AST/flow machinery and **only the gated legacy surface** in DESIGN 5.3. Ship C, C++, Java, Rust, Lua, Bash and Terraform as separate small activations with their respective `pattern-*` groups. Add only the applicable grammar adapters and tests for that activation. This is owner decision 4. JDBC, new stream/FFI/device protocols, shell implicit outputs and provider catalogs need later owner-authorised slices. The L7 vectors and raw truths remain executable and measurable in this lane. Python/JS/Go keep the existing default until their proof reader activates.

- [ ] Before or atomically with the first affected activation, install and test the target mapper’s pattern-language hint guard. Join an actual Bash curl/backtick body finding to no network hint and a Lua cursor-fetch finding to no db hint. Replay at each affected activation. Check the complete §5.3 catalog-to-row inventory, including libpq and seven inherited environment L7 outcomes.
- [ ] Match code tokens on the determining value or paired data slot, both boundaries, comments/literals shielded, no line inheritance. Couple every retained operation with a metadata/handle/suffix/local-shadow negative. Check scoped formals, a later write, a reachable competing branch and a failed/discarded operation. Check the qualified HCL namespace negatives as well as each named binding.
- [ ] Apply DESIGN 5.8 to changed legacy tests. Keep the evidence for deferred true reads. Direct C recv buffer and fgets, supported stdin read and C/C++ getenv need a changed **value** site. Generic ioctl status is silent; command-specific device outputs remain L7. Give each class removed on real diffs a trace count.
- [ ] Run affected provenance, language, CLI trace/threshold, integration mapping/DFG and coverage checks at thread counts 1 and 4 (ORCHESTRATION §5). Reconcile controls and enabled groups. Sample C/C++/Java/Rust at their own activation. Lua/Bash/HCL use matrix evidence only. No M-4 denominator exists for pattern languages. A failed applicable precision cell or an unapproved material loss blocks default activation.

### Slice S2: shared identity access and the repository root, no behaviour change (2 days)

**Files:** extend `src/api/review.rs` and the associated loader to bind the optional snapshot to `ReviewInputs`. Create `crates/prism-review/src/algorithms/taint/request_sources.rs`, `provenance/identity.rs`,
`provenance/identity_tests.rs`; modify `taint.rs` (`mod request_sources;` and
`pub(crate) use request_sources::framework_request_sources;`), `targets/mod.rs` (both
`dependency_*` modules `pub(crate)`; `repo_root: Some(&meta.repo_root)` in `project`), `dependency_hint.rs`
(`SiteContext.repo_root: Option<&'a Path>`; `SiteContext`, `root_identity`, `RootIdentity` `pub(crate)`, or added
as lane 2's PLAN S1 states, with its three tests, if not on `main`), `dependency_identity.rs` (`proven_path`;
`library_kind` visible), the `SiteContext` literals in `dependency_hint_tests.rs` and
`dependency_hint_binding_scope_tests.rs`, `provenance_slice.rs` (`slice_in_repo`), `crates/prism-review/src/api/run.rs`
(carry the loader’s immutable reviewed-source authority with its inputs into the provenance arm of `run_algorithm_raw`), `provenance/mod.rs`.

```rust
// provenance_slice.rs
// Keep existing slice(ctx, diff) and path-only callers. Bind additional source reads only via the snapshot attached to reviewed inputs.
// taint/request_sources.rs: wraps the per-language detectors detect_framework_sources dispatches
pub(crate) fn framework_request_sources(file_path: &str, parsed: &ParsedFile) -> Vec<TaintSeed>; // native file/line, target, start_byte, scope, byte_scope
// targets/dependency_identity.rs: kind_for_binding's proof without the table lookup
pub(crate) enum PathProof { Import { module: String, member: Option<String>, rest: Vec<String> },
    Builtin { path: String } }
pub(crate) fn proven_path(parsed: &ParsedFile, ctx: &SiteContext<'_>, at: Node<'_>, root: &str,
    rest: &[&str]) -> Option<PathProof>;
pub(crate) fn library_kind(language: Language, module: &str, rest: &[&str]) -> Option<&'static str>; // was private
// provenance/identity.rs
pub(crate) struct IdCx<'a> { pub site: SiteContext<'a>, pub requests: Vec<TaintSeed> }
pub(crate) fn id_cx<'a>(file: &'a str, parsed: &'a ParsedFile, ctx: &'a CpgContext<'_>,
    repo: Option<&'a Path>, snapshot: Option<&'a ReviewedSnapshot>) -> IdCx<'a>;
pub(crate) fn call_path(cx: &IdCx<'_>, parsed: &ParsedFile, call: Node<'_>) -> Result<PathProof, &'static str>;
pub(crate) fn read_path(cx: &IdCx<'_>, parsed: &ParsedFile, expr: Node<'_>) -> Result<PathProof, &'static str>;
// Represent the registration, receiver binding, accessor bytes, supplying expression and return/output slot.
// Keep the underlying raw seed independently so the census can audit a refused or augmented match.
```

`CpgContext` has no repository root. Pass the actual root through `run_algorithm_raw` and keep `slice(ctx,diff)`
as the no-root entry. `ReviewedSnapshot` above denotes loader authority over the immutable tree that provided these inputs, including directory entries and resolution metadata. For an immutable revision, load changed-file contents and resolution inputs from the same pinned tree reader. Leave the snapshot absent for a mutable worktree load. Consume the loader-issued reader from `ReviewInputs` in `run_algorithm_raw`. A path or a new tree digest supplied alongside preloaded inputs cannot populate it. Exercise inputs from A with repo B as a refusal. Without a root a Python import proof is refused (`no-repo-root`). Pure built-ins can
still be proven. Check on-disk import shadowing beyond changed files. Use lane 2's shared import proof at
the **specific AST occurrence** for calls and non-call paths, preserving the original exported member. Layer
DESIGN 5.4's use-site binding, alias/replacement, repository and runtime reachability checks on the provenance
side. Keep native targets resolution and taint seeds behaviourally identical. Expose their existing logic once.
Keep the native seed through inventory and deduplication. Use both line `scope` and `byte_scope` when expanding alias reads. Test a JS overwrite that narrows the former but leaves the latter intact. Both intervals must admit the occurrence. A raw handler line/target is only a candidate. A matched accessor must select this receiver, subexpression,
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
      The provenance classifier cannot choose its denominator. Apply DESIGN 5.5's four false-seed predicates separately from genuine payload losses. Retain a wrapper parent call, unresolved request direction or detached binding, its raw span, and all candidate keys. A genuine loss needs its own witness and owner disposition for limited release.
- [ ] Gates include accumulated matrix groups, `--test cli targets_test::`, `--lib targets::dependency_hint`
      and identical product findings to the S1b baseline on the population. Trace-only additions supply the independent
      request cross-check, which is scored separately from the M-4 census.

### Slice S3: Python origins by proof (3.5 days)

**Files:** create `provenance/python.rs`, `provenance/python_tests.rs`; modify `provenance/identity.rs`,
`provenance/mod.rs` (Python on the identity regime), `crates/prism-review/tests/cli/targets_test.rs`, and the tests
stage D named for Python.

```rust
// provenance/identity.rs
pub(crate) struct ProofSteps { value_steps: usize, handle_steps: usize }
// ProofGraph owns the per-root finite value/handle state sets, worklist and cycle checks.
pub(crate) trait DefReader {
    /// The value node a definition binds (the slot paired with it); None when the binding form is not modelled.
    fn value_of<'t>(&self, parsed: &'t ParsedFile, def: &VarLocation) -> Option<Node<'t>>;
    fn origin_of_value(&self, cx: &IdCx<'_>, parsed: &ParsedFile, value: Node<'_>, target: Node<'_>,
        states: &mut ProofGraph, steps: ProofSteps)
        -> Result<(Origin, Route), &'static str>;
}
pub(crate) fn origin_by_proof(ctx: &CpgContext, cx: &IdCx<'_>, env: &FileEnv<'_>, reader: &dyn DefReader,
    def: &VarLocation, states: &mut ProofGraph, steps: ProofSteps) -> Result<Verdict, &'static str>;
// provenance/python.rs
pub(crate) struct PythonReader;   // impl DefReader
```

Apply DESIGN 5.2.1–5.2.3 and 5.4. On Python enable only proven built-in/library/framework operations.
Use S1b's two finite-state budgets. Each constructor/handle transition increments handle_steps, each data or handle alias increments value_steps. Copy counters per branch. A receiver needs one live producer or a specified concrete formal. Keep database cursors and HTTP
responses as internal handles. Fetch/iteration, selected body, remote headers and status text are payload.
Bind imported Flask request in and outside routes. Use actual typed/registered Django request, registered
FastAPI parameter/dependency position and canonical Django settings. Generic parses preserve argument 0 and
the exact supplied origin, including proven config-file inputs. Python built-ins, argv/stdin, environment,
ORM/DB and HTTP operations and transfers are enumerated in DESIGN 5.2.2. Pair a documented mixed envelope with its actual payload selection. Add the native Graphene mutate/Arguments/Field/Schema proof and the project BaseMutation.perform_mutation ordinary-formal twin in DESIGN 5.2.3 and 5.9. Use the known Python mapping representation and key/default/overwrite semantics on both c0095 reads. Keep project JSON parsers and virtual helpers at L2 until a verified recovery contract binds the actual dispatch. Exercise the HttpRequest/Starlette whole-object negatives, local homonym and a supplied data-field twin.

- [ ] Enable `py` against the final base receipt. Pair sqlite cursor/scan materialization and HTTP body with
      bare connection, cursor, session and response negatives. Cover the fourth acyclic alias, sessionmaker → session → execute → scalars → all, mapping rows versus result keys, cycles with and without a seed, constructor under `with`, one killed versus two live receiver definitions, local runtime
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
      Activate the Python structural and proof reader together. Check the affected precision cells over the 100 diffs, both c0095 keys, the full family R ledger, actual retention including below-base config, the Python raw cross-check, accumulated groups and Contract. Apply the chosen DESIGN 7 release path before default merge.

### Slice S4a • JavaScript/TypeScript reader and native registration, integration first

**Files:** create `provenance/js_ts.rs`, `provenance/js_ts_tests.rs`; modify `provenance/mod.rs`,
`docs/features/language-coverage/js-ts-known-gaps.md` (a provenance entry: what is proven, what is not).

Implement DESIGN 5.1, 5.2.1, 5.2.3, 5.2.4 and 5.4 for JavaScript, JSX, TypeScript and TSX. Pair declarator,
destructuring, callback, Promise result and output slots. Peel erased type syntax. Respect overwrite, default,
key, rest and comma-expression value semantics. Bind Express, Fastify, Koa and NestJS by installed handler
and exact request slot, including error middleware, decorators and renamed formals. Request/context objects,
writers and metadata are silent. Proven inbound fields are input. Browser cookie, URL text, prompt and
actual form values are input. Process environment/argv require an unshadowed global or canonical import.
Fetch/Axios responses are handles until a wire-data selection. A whole materialized pg query Result and its rows both carry database under v1.1. Bind native tRPC input only at the proven builder, transparent parser and installed callback. A detached handler or project-imported procedure retains ordinary-formal semantics as specified in DESIGN 5.2.3. A cursor, status, helper, replaced parser/adapter or callback error is not a payload.

- [ ] Enable `js` against the final base receipt. Pair each library's positive payload with its same-library
      negative. Include browser-global shadowing, an unregistered form versus registered renamed or
      destructured formal, both middleware error slots, static named CJS/ESM imports and replacement cases,
      both whole pg Result and rows versus rowCount/fields/command, Axios data vs whole response, metadata, config file vs literal/reviver, and the independent data/handle work bounds. Restore all 14 pg result rows, including callback slot 1, and verify the facade-result twin has no finding or hint. Repair the six supplier fixtures and node-config singleton call from D-rows. Run all §5.9 absence/reordering/replacement and Fastify overload twins. Check type-imported Request/Response/Context and untyped residuals. Test the invalid named node-config import separately.
- [ ] Compare S4a on the 40 JS/TS diffs and the raw request inventory. Keep this a preparatory integration result while the promised wrapper keys are missing. Preserve controls and publish its precision sample and raw recall beside S4b.

### Slice S4b • verified forwarding and JS/TS integration activation

Implement DESIGN 5.2.3's single forwarding edge in the existing JS/TS and identity modules under the settled decision 7. Reuse the S2 raw registration/accessor candidates. Recognise the installed expression and immutable named result, the relative module's actual exported binding, and the one argument-preserving callback invocation. Apply source-field and live-alias checks on the use side. Read adapter bytes and every resolution input from the loader-bound immutable snapshot. Refuse a path-only or mutable run and substituted preloaded inputs. Check observed in-repo tsconfig remapping, sibling ambiguity and non-module mode. Admit the Directus-shaped ESM fixture whose external extends cannot be resolved and has no observed override. Apply per-file 16-module / 64-KiB limits and run-local reuse. No adapter is executed or inserted in the diff CPG. Keep native taint and targets identity unchanged.

- [ ] Base-grade the forwarding, snapshot and resolution fixtures in DESIGN 5.9, test scoped shadowing, changed bytes, escaped root, nested/oversize adapter, unknown registration and replaced slot. Trace the module digest and both byte ranges for a positive and the specific refusal for a negative. Exercise both the product repo entry and the missing-root compatibility entry.
- [ ] Measure counts of attempted/resolved/refused adapters, actual bytes, parses and elapsed time on the same population and hardware at S4a and S4b. Inspect all 20 Directus labels. Require all twelve R keys, count the eight residuals, and keep c0207/c0208 response objects silent. Account for the five tRPC misses independently. Gate the family on all applicable DESIGN 7 conditions, including the full JS/TS cell sample, raw recall and cross-check before any default merge.

### Slice S5: Go origins by proof (3 days)

**Files:** create `provenance/go.rs`, `provenance/go_tests.rs`; modify `provenance/mod.rs`.

Implement DESIGN 5.1, 5.2.1, 5.2.3, 5.2.5 and 5.4. Bind `:=`, var specs, package initializers, blank/error
slots, receiver formals and actual output addresses. Admit the verified server installation, canonical writer/request pair or http.HandlerFunc conversion for inbound role. Suppress RoundTrip, CheckRedirect and known outbound invocation/Client.Do evidence. A lone request remains a silent handle. A path key still needs a capture. Suppress whole canonical context, writer, request and database/sql handle formals before the fallback. A typed gin.Context has the separate gin contract. Pair every accessor with its own RHS, argument or changed
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
      FormValue in a registered twin and the Viper payload fixtures. Assert silence for RoundTrip, CheckRedirect and a signing helper called with an outbound request, and check a still-live environment branch. Keep project route helpers/BindAndCheck distinct from direct net/http and gin outputs. Verify all enabled groups.
- [ ] Measure the pair/conversion positives and their true/false server assignments separately at S5, including whole handles and outbound twins. Publish all affected head cells, raw weighted recall on the 40 Go diffs, every R key, the Go cross-check and genuine unknown-direction cases. Apply the DESIGN 7 release decision, accumulated groups and Contract before default merge.

### Slice S6: verify target hints after identity activation

**Files under verification:** `crates/prism-review/src/targets/mapping.rs` (the `("provenance", "untrusted_origin")` arm of `map_finding`) and `tests/integration/targets_mapping_test.rs`. The S1c guard owns the pattern-language change.

The language guard already landed with S1c. Verify that `db` and `network` hints are attached only when `Language::from_path(&finding.file)` is Python, JavaScript,
TypeScript, TSX or Go: after S3 to S5 those languages give `database` and `external_call` only by library or
built-in identity, and the pattern languages never do. Detail text unchanged.

- [ ] Re-run the guard tests first landed with S1c, including `provenance_hints_only_for_identity_languages` (`database` in `a.c` → no hint; `external_call` in
      `a.go` → `network`; `database` in `a.ts` → `db`). Check them against real emitted terminals.
      `maps_provenance_taint_peer_and_fallback_rows` passes unmodified.
- [ ] Gates: Hints on the population (`--targets`; every ordinal's targets artifact complete, else the gate fails):
      every hinted database/external-call target joins the winning `prov-var` record and the matching terminal
      library/built-in proof, directly or through the recorded aliases, callback or output slots. A request seed,
      opaque handle, pattern or package category alone cannot discharge the check. Include direct accessors,
      propagated values and a pattern-language finding in projection tests. Apply Contract.

### Recovery slice • Project payload contracts (after S5 and S6, before G)

Price is +10–16 person-days estimated, including the equal-risk tie. The opt-in contract pack and the resolver/loader are part of this lane. Deploying or maintaining a pack without independent authority earns zero credit.

1. **Snapshot-bound contracts and resolution inputs (3–5 days E).** Pin the reviewed tree, pack/version, module resolution, implementation digests, dispatch, slot maps and declared supplier/terminal. Record them on the winning and refused traces. Test missing pack, outdated body or metadata, wrong tree, alternate implementation and mutable runs through the normal entry point. A refused proof falls back to the amended classifier.
2. **Verified ingress contracts (3–5 days E).** Verify the five project JSON keys, two body-helper keys, two route-helper keys, five tRPC installation/builder keys and the GraphQL mutation argument. Exercise matching captures, effective parser and virtual dispatch on real source, with absent/replaced/overridden twins. Check all 15 specified promotions against the frozen 76.55 target weight. Nine formerly correct base hits could return (E).
3. **Verified database slots and equal-risk tie (3–5 days plus 1 day E).** Follow the 13 repository/DAO/store result and iteration keys and five materialized imported-model keys to a database terminal. Reject cached/local fallbacks, builders and dynamic or stale dispatch. Check the 18 promotions, 115.18 target weight, and one regained base hit. Prefer a proven terminal over an ordinary formal only at equal rubric risk. Recheck both operand orders, alternate same-grade sources, the opaque min arm and restored c0068 config (22.92 weight, one base hit). A correct tie alone can improve config without a pack.

- [ ] Take amended and recovery snapshots. Exercise the verified pack and its missing/stale twins through the product loader. Publish measured per-key outcomes and newly lost hits, trace/target ID changes, per-cell precision, raw cross-check joins and any refused promised keys. The matrix’s original 893 fixtures contain no project contract pack. Run the new contract rows and frozen real keys as separate obligations.

## Stage G: lane gate (2.5 days)

- [ ] Final head run with `--targets`. `g/REPORT.md`: volume by family × origin × severity, base against head, with
      `m/compare.py`'s reconciliation, both cumulative (M base to final head) and per slice; each removed class with
      its count (owner decision 2's non-variable names from `prov-drop`, examined unknowns by `prov-unknown`
      reason). Report that the M-4 frame did not count definition-only changes. Label any later separate tally with its own frame.
- [ ] Head samples (DESIGN 6): M-3's sampler, cells, seed and readers over every severity (`g/labels/head-*.jsonl`).
      **Precision** must reach 85% per adjudicated finding-language × claimed-origin cell, including info and complete small cells. Publish intervals and all unresolved/empty cells.
- [ ] Publish the 180-key raw weighted table at base, amended S1–S6 and recovery head with all misses, wrong-origin traces and base-hit retention. Show the original 95% shortfall per class, both design ceilings and the separately measured head scores. Preserve all original reader labels, the 45 census handle negatives, definition-only exclusion and 33 unresolved/insufficient tokens. Confirm every applicable R key at family activation and all 132 on the recovery head. A missing promised key returns to the owner. For empty reachable cells publish the raw zero and independent payload/negative matrix pair. Hardware has no denominator.
- [ ] Reconcile all raw request keys and supporting supplier evidence. Publish each independently witnessed false seed, genuine miss and unexplained residual. Require every supported key and the Hints and Contract joins on final output. Lay the measured residuals before the owner by class (key, reason, lost base hit, weight and cross-check impact). Record acceptance or refusal at G before any default-output merge.

## Close-out (1 day)

- [ ] Final docs check: the `CLAUDE.md` provenance paragraph (examined set, regimes, proofs, trace, losses), the
      algorithm-map line, `ALGORITHMS.md` entry 23, the S4 known-gaps entry. Optional
      `mutants/provenance-identity.json` (boundary, join, kill and later-definition rules, value/handle cycle guards, hint; not run
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

The base M gate is GO. Deliver the base-graded 940-ID matrix, measured family and final precision, 180-key recall and retention at each checkpoint, raw request reconciliation, Hints and Contract receipts. Include affected-test receipts and review dispositions. Keep the 95% full-denominator objective’s observed shortfall visible.

**Default publication stays on hold through G.** Option (iii) requires every applicable amended R key at family activation, all post-recovery R keys at G, ≥85% precision in every populated adjudicated cell including small cells, supported raw request coverage and no unexplained losses, zero-coverage matrix controls, Hints and Contract. Decision 5 leaves the actual class residuals and genuine cross-check misses for owner acceptance or refusal at G. The owner has authorized the recovery work and one wrapper level. Neither selection signs off reduced recall.

**Scheduling.** The 29.5-day estimate predates the amendment. S4b’s wrapper is already included at 2–4 days E. The shared fold and bounded implementation repairs add 4–7 days E. With recovery, option (iii) adds 14–23 days E over the amended plan. Ongoing pack maintenance is unpriced. No head measurement exists at design stage.

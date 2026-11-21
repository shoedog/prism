# Lane 6: provenance findings by proven identity — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is dispatched
> to an implementer (Sonnet-5.5, high) with its section, plus the Global constraints section, as the brief. Steps
> use checkbox syntax.

**Goal:** every provenance finding names a variable, rests on a definition that reaches the use in its own function,
and carries an origin proven from that definition's expression (library, framework or built-in identity for Python,
JavaScript/TypeScript and Go; gated code-only patterns elsewhere), measured before and after on 180 real diffs.

**Architecture:** `provenance_slice.rs` keeps `slice()` and delegates to `algorithms/provenance/` (variable
selection, reaching definitions, join, pattern tables, identity rules per family). Identity comes from the import
proof in `targets/dependency_identity.rs` (made visible in the crate, plus a non-call entry point) and from the
taint request-source recogniser (exposed by one function). The targets mapping keeps parsing the unchanged
description and attaches a hint only for identity languages. No change under `src/` or the cache.

**Tech stack:** Rust (prism-review crate, tree-sitter); Python 3 for the population scripts.

**Spec:** `DESIGN.md` here (5.1 examined set, 5.2 origins, 5.3 other languages, 5.4 shared identity, 5.5 output,
5.6 layout, 6 measurement, 7 gates, 8 owner decisions); after stage D, "DESIGN" is the amended design of record.

## Global constraints

- Worktree `~/code/prism-provenance` (sparse, without `eval/secbench/`); evidence `~/prism-evidence/provenance/`
  (`m/`, `t/`, `s<N>/`, `g/`, `LEDGER.md`, `HANDOFF.md`); corpus clones `~/code/prism-provenance-corpora/`.
- Unchanged: category `untrusted_origin`; `algorithm` `provenance`; the severity map (`user_input`, `hardware` →
  `concern`; `database`, `external_call` → `warning`; `function_param`, `env_var`, `config` → `info`; `constant`,
  `unknown` → none); the description `variable '<name>' has <origin> origin: <risk>` from
  `Origin::name()`/`risk_level()` (`src/frameworks/origin.rs`, not edited); `function_name: None`; provenance in
  `review_suite()`, `all()` and the `prism targets` default set; the public items
  `provenance_slice::{slice, Origin, ProvenanceFinding}`.
- Deliberate, disclosed in each PR body: `<name>` is the use's `path.base`; `related_lines` are the reaching
  definitions' lines plus the proving chain's def lines (provenance `targets[].id` hashes them, so ids change once);
  blocks are emitted for examined variables only.
- Weighted sizes: `provenance_slice.rs` 616, `taint.rs` 323, `dependency_identity.rs` 537, `dependency_hint.rs`
  277.5, `mapping.rs` 335, `targets/mod.rs` 338.5, `algorithms/mod.rs` 278, `targets_mapping_test.rs` 560;
  `tests/common/mod.rs` 1,226 and `coverage_test.rs` 1,044 take edits only. New files aim for 600 or fewer.
- New test files go into the three lists of `tests/integration/coverage_test.rs` (`all_test_files` in
  `test_algorithm_language_matrix` and `test_language_coverage_minimum`, `test_files` in
  `test_coverage_matrix_validation`), after `provenance_lang_test.rs`. Unit-test files of the new tree are declared
  in `provenance/mod.rs` as `#[cfg(test)] mod <name>;`.
- Lane 2 owns `crates/prism-review/src/targets/` (ORCHESTRATION section 11). This lane's edits there are additive
  (visibility, one optional field, one function, one guard in a mapping arm) and change no existing result:
  `--test cli targets_test::` and `--lib targets::dependency_hint` pass unchanged unless a slice names an artefact.
  S2 rebases over lane 2's slice S1 if merged; otherwise S2 carries S1's `root_identity` as lane 2's PLAN states it
  and the orchestrator records that on lane 2's issue.
- `taint.rs`/`taint/` are shared with lane 4 and PR #841: one new module and one `mod`/`use` pair, nothing else.
- `PRISM_PROVENANCE_TRACE=1` prints tab-separated lines to stderr (default output unchanged):
  `prov-var file line name origin route origin_line`, `prov-drop file line name reason`, and from S2
  `prov-request-source file line target|-`. Every population run sets it.
- Tests and `prism` runs go through `~/prism-evidence/tools/memguard.sh 8`; do not run mutgate; do not push.

## Review focus

1. A variable named like a trigger word: no `user_input`. Rows `st-py-args-local`, `py-unproven-local-request`.
2. A request object passed through a helper: not `user_input`; the loss is listed. Row `py-helper-request-param` (S3).
3. Two definitions of different origins reach one use: the higher severity wins. Rows `st-c-join-getenv-recv`,
   `py-join-env-request`.
4. A use at module level: no panic; the scope-aware fallback finds the definition. Row `py-module-level-env` (S3).
5. A parse error in one function: no panic, other functions unchanged, nothing from it. Row `st-py-parse-error`.

## Stage M: measure first (3 days)

### Task M-1: issue, workspace, population (180 diffs)

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
      the 60 extra Python-heavy diffs. Commit order decides, no seed. `~/code/bench-repos` clones are read only.
- [ ] `m/specs.py`: diff specs by `m2run.py`'s `added_lines` and filter (supported extension, not a test, present at
      the commit), written with `json.dump(spec, f)` as there; for the 120 rows the SHA-256 must equal
      `base_spec_sha256` (a mismatch stops the task). `m/population.jsonl`: 180 rows (`ordinal`, `repo`, `family`
      py/js/go, `commit`, `parent`, `tree`, `spec`, `spec_sha256`, `origin` `602`/`lane6`). Corpus clones by
      `git clone --shared`; each run checks out its commit detached and verifies `HEAD^{tree}` and a clean status.

### Task M-2: driver and base run

- [ ] If `scripts/measure/` on `main` has `review-diff` (lane 11; `python3 -m measure review-diff --help` from
      `scripts/`), convert the manifest and use it. Otherwise write `m/run_population.py`
      (`--population P --binary B --out D [--targets] [--only FAMILY]`): serial, resumable (skips rows whose
      `meta.json` names the same binary SHA-256), per row
      `memguard.sh 8 B --repo <clone> --diff <spec> --algorithm provenance --format json --no-cache` with
      `PRISM_PROVENANCE_TRACE=1`, timeout 900 s, writing `<ordinal>.json`, `.err`, `meta.json` (argv, exit,
      elapsed, binary SHA-256); `--targets` adds
      `B targets --repo <clone> --diff <spec> --algorithm provenance --no-cache` into `<ordinal>.targets.json`.
- [ ] `m/compare.py BASE HEAD`: findings by family × origin × severity; added, removed, changed keyed by (ordinal,
      file, line, name); removed grouped by the head's `prov-drop` reason; `m/test_compare.py` on a two-row fixture.
      Base run with `--targets`, one measurement at a time, no host-quiet marker (counts only).

### Task M-3: labelled base sample (300), go gate, memo

- [ ] `m/sample.py`: findings at `concern`/`warning`, sorted by (ordinal, file, line, description), stratified by
      family × origin, proportional with at least 30 per family where available, `random.Random(20261121)`. Each
      item: the file's imports and lines `line-60 .. line+10` (`git show <commit>:<path>`), the variable, the
      claimed origin.
- [ ] Two independent readers: Sonnet-5.5 high (Agent tool) and gpt-6.1-terra (`codex exec`, read-only, item files
      only). Question, verbatim: "Is the named token a variable read at this line, and can its value here come from
      the claimed origin as defined below? Answer `right`, `wrong_origin` (name the origin you see), `source_object`
      (the request or handle object itself) or `not_variable` (module, attribute, callee, function, type or keyword
      name, or text in a comment or string). Unless `right`, pick a cause: trigger substring in a name, comment or
      string, non-variable token, line inheritance, other function's definition, precedence, constant masking,
      signature substring, one-sided boundary, other." The brief carries DESIGN 5.2's origin definitions.
      Disagreements get a third read (Opus-5.5 high). Store `m/labels/base-r1.jsonl`, `base-r2.jsonl`,
      `base-final.jsonl`; record Cohen's kappa. Recall set = the `right` items; `source_object` items are reported
      apart.
- [ ] Cross-check input: S2's `prov-request-source` trace (the taint recogniser itself), run on the population in
      S2's measurement into `g/request-sources.tsv`; the memo records the procedure.
- [ ] Go gate (DESIGN 7): precision = `right` / all labelled, `source_object` counted as right. Below 60%: continue;
      otherwise stop and report on the issue. `CENSUS.md` in this directory (docs PR, one reviewer): population,
      volumes per diff and diff line, precision by family and origin with causes, kappa, gate result; aggregates
      only.

## Stage T and E: executable matrix (3 days)

### Task T-1: rows, fixtures, data-driven test

**Files:** create `tests/fixtures/provenance/rows.jsonl`, `tests/fixtures/provenance/cases/<lang>/<row>.<ext>`,
`tests/fixtures/provenance/README.md`, `tests/algo/novel/provenance_matrix_test.rs`; modify
`tests/algo/novel/main.rs` (`mod provenance_matrix_test;`) and the three coverage lists.

Row fields: `id`, `group`, `class`, `files` (under `cases/`; several for repository-local shadowing), `diff` (file →
lines), `want` (`[{file, line, var, origin}]`, `origin` an `Origin::name()` or `none`), `exact_lines` (no other
provenance finding on those lines), `truth` (what the value really is), `accepted_loss` (null or a reason, stage D),
`source` (`probe`, `corpus`, `grammar`, `enumerator:<seat>`). The test parses each file with `Language::from_path`,
runs `algorithms::run_slicing_compat` with `SlicingAlgorithm::ProvenanceSlice`, reads origins by the prefix
`variable '<var>' has <origin> origin:`, and fails naming every failing row id.

```rust
// tests/algo/novel/provenance_matrix_test.rs
const ENABLED_GROUPS: &[&str] = &["control"];
#[test]
fn test_provenance_matrix_rows() { /* rows.jsonl; skips disabled groups and accepted losses */ }
```

Groups: `control` (green on base, stays green), `structural` (S1), `py` (S3), `js` (S4), `go` (S5). Seed rows:

| Family | Rows and required behaviour |
|---|---|
| Python origins | `py-ui-flask-args` route + `request.args.get` → user_input; `py-ui-django-get`; `py-ui-fastapi-query` annotated parameter; `py-ui-argv`; `py-ui-input`; `py-db-sqlite-hop2` connect → cursor → fetchone → database; `py-db-sqlalchemy`; `py-ext-requests` `requests.get` and `resp.json()` → external_call; `py-env-environ-get`, `py-env-subscript`, `py-env-getenv`, `py-env-from-import` → env_var; `py-cfg-configparser`, `py-cfg-yaml` → config; `py-const` → none; `py-param` → function_param; `py-prop-1` one step keeps the origin; `py-prop-limit` one past the depth → none |
| JS/TS origins | `js-ui-express-query`, `js-ui-koa-ctx`, `js-ui-fastify`, `ts-ui-nest-query`; `js-ui-argv`; `js-env-process`, `js-env-destructure`; `js-ext-axios`, `js-ext-fetch`; `js-db-pg-pool`; `js-cfg-require-config`, `js-cfg-readfile-json`; `ts-ui-as-request` `(req as Request).body`; `tsx-control`; `js-browser-cookie` `document.cookie` (truth user_input, behaviour per stage D) |
| Go origins | `go-ui-formvalue`; `go-ui-one-byte-q` `q := r.URL.Query().Get("q")`; `go-ui-gin-query`; `go-ui-os-args`; `go-ui-flag` (`*p`); `go-env-getenv`, `go-env-lookupenv`; `go-db-queryrow-scan` (`row.Scan(&name)`); `go-ext-http-get` and `resp.Body`; `go-cfg-viper` |
| Decoys, truth none, each identity family | `requested_at`/`requestedAt`, `form_factor`, `fetch_config(name)`/`fetchConfig(name)`, a module named `requests`, a `settings` literal, `SETTINGS`; the trigger in a comment, docstring, string, f-string or template literal, type annotation, decorator; wrong-language patterns `self.data.x`, `module.exports`, `x = read(fd)`; precedence `await db.query(sql)` on a proven `pg` handle → database; `if n == 1:` is no constant; Go `err` in a function whose signature says `error` |
| Scoping | `st-py-args-local`; one name, different origins in two functions; nested function reading an outer variable; comprehension variable; `global`; shadowing parameter; `py-module-level-env` |
| Hops and helpers | depth 1, 2, one past the depth; factory `conn = get_conn()` → none; handle defined twice → none; `with httpx.Client() as c`; `self.client` set in `__init__`; `py-helper-request-param`; `py-flask-request-outside-route` (truth user_input, behaviour per stage D) |
| Unproven bindings | `py-unproven-local-request` (`tests/fixtures/targets/client.py` shape); `from mylib import request`; local `requests = []`; repository-local `requests.py`; import visible only in another function; JS `const req = build()` outside a handler; unregistered `(req, res)` function; Go local `r` not `*http.Request` |
| Joins, edges | `st-c-join-getenv-recv`, `py-join-env-request`, `st-py-parse-error`, a 1-byte variable per family |
| Pattern languages | `control`: C `ioctl` → hardware, C `recv` → user_input, C `getenv` → env_var, Rust stdin, Lua `io.read`, `os.getenv`, Terraform `var.`; `structural`: C `p = 0` no constant, `recv(` in a C comment, `getenv(` in a C string, Bash `read x` and `$1`, Java `System.getenv` |

- [ ] Write rows and fixtures (sources: `~/prism-evidence/lanes-2026-11-20/probes/lane6/`, `research/lane6-data/`;
      public code shapes only).
- [ ] Enable every group once on base and record the failing rows in `t/red-on-base.txt`; commit with
      `ENABLED_GROUPS = &["control"]`; a `control` row red on base moves to the group of the slice that changes it.
      Gates: section 5 items 1, 2, 4, 8; one reviewer (test-only).

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION section 6. Axes: origin × family × proof route; binding forms (tuple, destructuring,
      walrus, `with`, `for`, `:=`, `var`, multi-value, out-parameters); handler shapes; accessor forms; hops;
      import forms (alias, `from`, star, `require`, dynamic, Go alias and dot); shadowing; decoy placements;
      propagation (containers out of model); positions (module, class body, lambda, closure); parse errors;
      joins. M-3's causes are rows. Every new row carries a truth.

## Stage D: design of record (1 day)

- [ ] The design seat amends DESIGN 5 from `rows.jsonl`, `CENSUS.md`, `base-final.jsonl`, `t/red-on-base.txt`. It
      fixes: per-language origin tables (configuration, environment, command-line; which `db`/`http` entries count;
      Django `settings`; file parsers as configuration); handle-hop and propagation depths; the recognised handler
      parameter per framework (Flask's imported `request` inside and outside a route, Django, FastAPI, Express,
      Fastify, Koa, NestJS, net/http, gin) and the request object's own origin; JS browser sources; receiver names
      (`self`, `cls`, `this`, Go receivers); Terraform and Bash variable selection and the tables' language map;
      which existing expectations are substring artefacts (research section 3). It lists accepted losses; the
      implementer then sets `want` and `accepted_loss`. Both reviewers; cap 2 rounds.

## Stage I: slices

Order: S1 removes most noise in every language without identity; S2 makes identity reachable with no behaviour
change; S3 to S5 move one family each to proofs (Python first: largest share of the population and of the noise);
S6 changes the hint once all three are on proofs. Each slice branches from updated `main`, enables its group in
its first commit and shows RED. Before review: write the prediction, run the population with merge base and head
(the previous slice's head run serves as base if no commit between touches provenance, `targets/` or
`src/frameworks/`), put `m/compare.py`'s tables and each removed class with its count in the PR body. Every slice:
section 5 items 1 to 4, 8, 9, 10 (`CLAUDE.md` provenance paragraph, `ALGORITHMS.md` entry 23), 11; two reviewers.

### Slice S1: structural fixes for every language (3 days)

**Files:** create `crates/prism-review/src/algorithms/provenance/mod.rs`, `provenance/patterns.rs`,
`provenance/patterns_tests.rs`, `crates/prism-review/tests/cli/provenance_trace_test.rs`; modify
`crates/prism-review/src/algorithms/mod.rs` (`mod provenance;`), `provenance_slice.rs` (tables and classification
move out; `slice()` delegates), `crates/prism-review/tests/cli/main.rs` (`mod provenance_trace_test;`), coverage
lists, the re-fixtured tests below.

```rust
// provenance/mod.rs
pub(crate) const PROPAGATION_DEPTH: usize = /* DESIGN, stage D */;
pub(crate) struct FileEnv<'a> { pub file: &'a str, pub parsed: &'a ParsedFile, pub suppressed: BTreeSet<String> }
pub(crate) enum Route { Pattern { table: &'static str, pattern: &'static str }, Literal, Parameter,
    RequestSource, Proven { path: String }, Propagated { depth: usize } }
pub(crate) struct Verdict { pub origin: Origin, pub origin_file: String, pub origin_line: usize,
    pub route: Route, pub chain: Vec<usize> }
pub(crate) struct Examined { pub name: String, pub use_loc: VarLocation, pub defs: Vec<VarLocation>,
    pub verdict: Option<Verdict> }
pub(crate) fn examine_line(ctx: &CpgContext, env: &FileEnv<'_>, line: usize) -> Vec<Examined>;
pub(crate) fn join(verdicts: Vec<Verdict>) -> Option<Verdict>;   // highest severity, then earliest origin line
pub(crate) fn severity(origin: &Origin) -> Option<&'static str>; // today's map, moved
// provenance/patterns.rs
pub(crate) fn code_text(parsed: &ParsedFile, node: tree_sitter::Node<'_>) -> String; // comments, strings blanked
pub(crate) fn matches_word(text: &str, word: &str) -> bool;      // every occurrence, both boundaries
pub(crate) fn classify_text(language: Language, code: &str) -> Option<(Origin, &'static str, &'static str)>;
```

Behaviour (DESIGN 5.1, 5.3): examined variables are the `Use` locations at the line (`ctx.cpg.nodes_at` and
`to_var_location`), one per `path.base`, minus callee and type positions and bases bound by a file import
(`ctx.cpg.call_graph.imports[file]`); every other identifier on the line gets a `prov-drop` line with its reason.
Reaching definitions: `ctx.cpg.dfg.predecessors(use)` filtered to `Def` in the same function; with none,
`ctx.cpg.all_defs_of` in the same function kept when `find_variable_references_scoped(func, base, def.line)`
contains the use line (the root node at module level). A definition's origin: a parameter → `function_param`; a
literal value (string without interpolation, number, boolean, null, an empty list, map, tuple, array or object) →
`constant`; else the tables on `code_text` of the defining statement; else one propagation step through the
variables its value reads, up to `PROPAGATION_DEPTH`. Tables keep today's entries, keyed by their
`// === <Language> ===` sections and the stage-D language map; `~` words need both boundaries; precedence stays.
The diff-line and signature-substring fallbacks are removed; the import suppression stays (in `patterns.rs`).
Python, JS/TS and Go use this regime until S3 to S5. `selected_origin_evidence` is unchanged, called with the
proving origin line and the used name, so a propagated origin is graded `unlabeled`.

- [ ] Enable `structural`: RED. Unit tests in `patterns_tests.rs`: `boundary_rejects_format_for_form`,
      `boundary_rejects_fetch_config_for_fetch`, `boundary_checks_every_occurrence` (`formatted form`),
      `code_text_blanks_comments_and_strings`, `terraform_tables_do_not_run_on_python`,
      `join_prefers_concern_over_info`. CLI tests: `trace_lists_examined_and_dropped_names`,
      `no_trace_without_the_variable`.
- [ ] Implement; GREEN for `control` and `structural`.
- [ ] Re-fixture to a Django view (`from django.http import HttpResponse`, then `def view(request):`), which both
      regimes call user input, so S3 leaves them alone; lines and `kill_line` shift by one.
      `tests/common/mod.rs` `assert_finding_evidence_provenance_cases`: the bridge case becomes
      `term = request.GET['q']`, `t2 = term.strip()`, `cursor.execute(t2)`, needle `variable 't2'`, still
      `unlabeled` (propagated). `min_confidence_test.rs`: add `provenance_flow_source(killed)` beside
      `flow_source` (taint keeps it) for `evidence_delivery_provenance_reaches_sarif_with_all_three_grades` and its
      bridge; `evidence_delivery_local_clean_flow_still_reaches_sink_when_d_drops_the_shortcut` uses `term`, diff
      line 5, `related_lines` `[3, 4]`. `min_confidence_admission_test.rs`
      `default_nameonly_threshold_retains_an_unlabeled_finding`: same bridge. Any other flipped test must be on
      stage D's artefact list and is named with its reason in the PR body.
- [ ] Run (memguard, `RAYON_NUM_THREADS` 1 and 4): `cargo test --test algo_novel provenance`;
      `cargo test --test integration -- dfg_label_parity_test:: targets_mapping_test:: coverage_test::`;
      `cargo test --test cli -- min_confidence provenance_trace_test:: targets_test:: validation_test::`;
      `cargo test -p prism-review --lib provenance`; and filter `provenance` in `lang_c`, `lang_cpp`, `lang_bash`,
      `lang_terraform`, `lang_java`, `lang_go`, `lang_typescript`, `lang_tsx`.
- [ ] Expected to stay green: the C, C++, Rust, Lua tests (def lines hold the API); Bash and Terraform tests cannot
      flip. Gates: Contract (`targets_mapping_test::` unmodified); pattern-language recall is unmeasurable on the
      population (Python, JS/TS, Go only): report recall-set retention for all families as information.

### Slice S2: shared identity access, no behaviour change (1.5 days)

**Files:** create `crates/prism-review/src/algorithms/taint/request_sources.rs`, `provenance/identity.rs`,
`provenance/identity_tests.rs`; modify `taint.rs` (`mod request_sources;` and
`pub(crate) use request_sources::{framework_request_sources, RequestSource};`), `targets/mod.rs` (both
`dependency_*` modules `pub(crate)`; `repo_root: Some(&meta.repo_root)` in `project`), `dependency_hint.rs`
(`SiteContext.repo_root: Option<&'a Path>`; `SiteContext`, `root_identity`, `RootIdentity` `pub(crate)`, or added
as lane 2's PLAN S1 states, with its three tests, if not on `main`), `dependency_identity.rs`, the `SiteContext`
literals in `dependency_hint_tests.rs` and `dependency_hint_binding_scope_tests.rs`, `provenance/mod.rs`.

```rust
// taint/request_sources.rs: wraps the per-language detectors detect_framework_sources dispatches
pub(crate) struct RequestSource { pub line: usize, pub target: Option<AccessPath>,
    pub start_byte: Option<usize>, pub byte_scope: Option<(usize, usize)> }
pub(crate) fn framework_request_sources(file_path: &str, parsed: &ParsedFile) -> Vec<RequestSource>; // sorted, deduped
// targets/dependency_identity.rs: kind_for_binding's proof without the table lookup
pub(crate) enum PathProof { Import { module: String, rest: Vec<String> }, Builtin { path: String } }
pub(crate) fn proven_path(parsed: &ParsedFile, ctx: &SiteContext<'_>, at: Node<'_>, root: &str,
    rest: &[&str]) -> Option<PathProof>;
pub(crate) fn library_kind(language: Language, module: &str, rest: &[&str]) -> Option<&'static str>; // was private
// provenance/identity.rs
pub(crate) struct IdCx<'a> { pub site: SiteContext<'a>, pub requests: Vec<RequestSource> }
pub(crate) fn id_cx<'a>(file: &'a str, parsed: &'a ParsedFile, ctx: &'a CpgContext<'_>) -> IdCx<'a>; // repo_root None
pub(crate) fn call_path(cx: &IdCx<'_>, parsed: &ParsedFile, call: Node<'_>) -> Option<PathProof>; // via root_identity
pub(crate) fn read_path(cx: &IdCx<'_>, parsed: &ParsedFile, expr: Node<'_>) -> Option<PathProof>; // attribute, subscript
```

`CpgContext` has no repository root (`src/cpg/context.rs:39`): provenance passes `repo_root: None`, and
`repo_has_local_module` then checks `known_files` (`ctx.files`) only. `Builtin` means the root is neither imported
nor bound locally. `kind_for_binding` keeps its results.

- [ ] Tests first in `identity_tests.rs`: `proven_path_os_environ_get_is_import_os` (module `os`, rest
      `["environ","get"]`), `proven_path_refuses_shadowing_parameter`, `proven_path_refuses_repo_local_module` (a
      `requests.py` in `known_files`), `proven_path_process_env_is_builtin`, `proven_path_go_os_getenv`,
      `request_sources_flask_route_lists_assignment_line`, `request_sources_express_lists_handler_param`,
      `request_sources_net_http_lists_formvalue_line`. RED (do not compile).
- [ ] Implement; `provenance/mod.rs` builds `IdCx` per diff file and prints `prov-request-source`.
- [ ] Gates: matrix, `--test cli targets_test::`, `--lib targets::dependency_hint` unchanged; population findings
      byte-identical to the base (only stderr differs); the same run writes `g/request-sources.tsv`.

### Slice S3: Python origins by proof (3 days)

**Files:** create `provenance/python.rs`, `provenance/python_tests.rs`; modify `provenance/identity.rs`,
`provenance/mod.rs` (Python on the identity regime), `crates/prism-review/tests/cli/targets_test.rs`, and the tests
stage D named for Python.

```rust
// provenance/identity.rs
pub(crate) const HANDLE_HOP_DEPTH: usize = /* DESIGN, stage D */;
pub(crate) trait DefReader {
    /// The value node a definition binds; None when the binding form is not modelled.
    fn value_of<'t>(&self, parsed: &'t ParsedFile, def: &VarLocation) -> Option<Node<'t>>;
    fn origin_of_value(&self, cx: &IdCx<'_>, parsed: &ParsedFile, value: Node<'_>) -> Option<(Origin, Route)>;
}
pub(crate) fn origin_by_proof(ctx: &CpgContext, cx: &IdCx<'_>, env: &FileEnv<'_>, reader: &dyn DefReader,
    def: &VarLocation, depth: usize) -> Option<Verdict>;
// provenance/python.rs
pub(crate) struct PythonReader;   // impl DefReader
```

Rules (DESIGN 5.2 as amended): user input when a `RequestSource` is on the definition's line with the definition's
base as target (or no target), or the value is a listed command-line read; database or external call when
`call_path` gives a `db` or `http` `library_kind`, or the receiver's single reaching definition is proven so, up to
`HANDLE_HOP_DEPTH`; environment and configuration by the amended Python tables over `PathProof`; literal; parameter;
propagation as in S1; anything else `unknown`, traced with the reason.

- [ ] Enable `py`: RED. Unit tests: `cursor_two_hops_is_database`, `factory_handle_is_unknown`,
      `requests_get_is_external_call`, `environ_subscript_is_env`, `local_request_object_is_unknown`,
      `with_target_handle_follows_amended_rule`. Implement; GREEN.
- [ ] `targets_test.rs` `default_run_emits_all_five_producers_and_live_mappings`: its `user_input` target came from
      the repository-local `request = Request()` (`tests/fixtures/targets/client.py:10,15`), a substring artefact.
      Assert a provenance target for `variable 'user'` with `function_param` (line 14) and none containing
      `user_input`; the fixture stays.
- [ ] Other expectations: `test_provenance_python_request_form_origin` (Flask `request` in a function with no route)
      follows stage D's Flask rule; `test_provenance_python_cursor_execute_origin` needs a hop depth of at least 2.
      Gates: Recall on the Python part (≥ 95%, losses with reasons), Cross-check on Python diffs, Contract; measure
      on the 100 Python diffs (`--only py`).

### Slice S4: JavaScript/TypeScript origins by proof (3 days)

**Files:** create `provenance/js_ts.rs`, `provenance/js_ts_tests.rs`; modify `provenance/mod.rs`,
`docs/features/language-coverage/js-ts-known-gaps.md` (a provenance entry: what is proven, what is not).

`JsTsReader` (`impl DefReader`) for JavaScript, TypeScript and TSX, with `as`, `!` and `satisfies` peeled and
declarators and destructuring as amended. Request sources from Express, Fastify, Koa, NestJS; `process.argv` and
`process.env` as built-ins; built-in `fetch` and the JS `http` entries as external call; `db` entries with handle
hops; browser sources as stage D decided.

- [ ] Enable `js`: RED. Unit tests: `express_query_field_is_user_input`, `unregistered_req_param_is_param`,
      `pg_pool_query_is_database_not_user_input`, `process_env_destructure_follows_amended_rule`,
      `as_request_wrapper_is_seen_through`. Implement; GREEN.
- [ ] `test_provenance_js_document_cookie_origin` follows stage D's browser rule; `..._process_env_origin` stays.
      Gates: Recall and Cross-check on the JS/TS part (40 diffs, `--only js`), Contract.

### Slice S5: Go origins by proof (2.5 days)

**Files:** create `provenance/go.rs`, `provenance/go_tests.rs`; modify `provenance/mod.rs`.

`GoReader` (`impl DefReader`): `:=`, `var`, multi-value assignment (each name its own value position) and
out-parameters as amended; net/http and gin request sources (line seeds); `os.Args`, `flag.*`, `os.Getenv`,
`os.LookupEnv` via `PathProof`; `database/sql`, `pgx`, `gorm` handles; `net/http` client calls; viper.

- [ ] Enable `go`: RED. Unit tests: `formvalue_on_request_param_is_user_input`, `one_byte_variable_is_examined`,
      `err_is_not_function_param_by_signature`, `queryrow_handle_is_database`, `viper_getstring_is_config`.
      Implement; GREEN.
- [ ] `test_provenance_go_form_value_origin` and `..._viper_config_origin` stay (viper must be in the Go
      configuration table). Gates: Recall and Cross-check on the Go part (40 diffs, `--only go`), Contract.

### Slice S6: the targets hint rule (0.5 day)

**Files:** modify `crates/prism-review/src/targets/mapping.rs` (the `("provenance", "untrusted_origin")` arm of
`map_finding`), `tests/integration/targets_mapping_test.rs`.

`db` and `network` hints are attached only when `Language::from_path(&finding.file)` is Python, JavaScript,
TypeScript, TSX or Go: after S3 to S5 those languages give `database` and `external_call` only by library identity,
and the pattern languages never do. Detail text unchanged.

- [ ] Test first: `provenance_hints_only_for_identity_languages` (`database` in `a.c` → no hint; `external_call` in
      `a.go` → `network`; `database` in `a.ts` → `db`). RED, implement, GREEN.
      `maps_provenance_taint_peer_and_fallback_rows` passes unmodified.
- [ ] Gates: Hints on the population (`--targets`): every row with a `dependency_hint` joins a `prov-var` line whose
      route is `Proven`, or `Propagated` from one; Contract.

## Stage G: lane gate (2 days)

- [ ] Final head run with `--targets`. `g/REPORT.md`: volume by family × origin × severity, base against head, each
      removed class with its count (owner decision 2's non-variable names from `prov-drop`).
- [ ] Head sample: 200 at `concern`/`warning`, M-3's sampler, seed and readers (`g/labels/head-*.jsonl`).
      **Precision** ≥ 85% per family with at least 30 sampled findings. **Recall**: each `right` base item has a
      head finding with the same (ordinal, file, line, name) and origin, ≥ 95%, every loss listed with its
      `prov-var` route, stage-D accepted losses marked.
- [ ] **Cross-check**: each `g/request-sources.tsv` line whose variable is used on a diff line with that definition
      reaching it (per `prov-var`) has a `user_input` finding; exceptions explained (the report says the definitions
      come from the same code). **Hints**, **Contract** on the final head. Post the tables on the tracking issue.

## Close-out (1 day)

- [ ] Final docs check: the `CLAUDE.md` provenance paragraph (examined set, regimes, proofs, trace, losses), the
      algorithm-map line, `ALGORITHMS.md` entry 23, the S4 known-gaps entry. Optional
      `mutants/provenance-identity.json` (boundary, join, hop depth, hint; not run per PR).
- [ ] Follow-up issues the numbers justify: helpers, factories, containers, cross-file origins; more identity
      languages (owner decision 4); pattern-language precedence; Terraform/Bash selection, browser sources if lost.
      Refresh `HANDOFF.md`; summarise and offload raw outputs; delete the corpus clones and `target/`.

## Acceptance for the lane

`CENSUS.md` merged with the go gate passed; S1 to S6 merged with both reviewers approving; on the final head every
DESIGN 7 gate holds (Precision, Recall, Cross-check, Hints, Contract) with losses listed; the report is on the
tracking issue; docs and handoff current.

**Estimates:** M 3, T and E 3, D 1, S1 3, S2 1.5, S3 3, S4 3, S5 2.5, S6 0.5, G 2, close-out 1; total 23.5 days.

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
other languages, 5.4 shared identity and refusals, 5.5 output and trace, 5.6 layout, 6 measurement, 7 gates, 8
owner decisions); after stage D, "DESIGN" is the amended design of record.

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
  (visibility, one optional field, one function, one guard in a mapping arm) and change no existing result:
  `--test cli targets_test::` and `--lib targets::dependency_hint` pass unchanged unless a slice names an artefact.
  S2 rebases over lane 2's slice S1 if merged; otherwise S2 carries S1's `root_identity` as lane 2's PLAN states it
  and the orchestrator records that on lane 2's issue.
- `taint.rs`/`taint/` are shared with lane 4 and PR #841: one new module and one `mod`/`use` pair, nothing else.
- `PRISM_PROVENANCE_TRACE=1` prints the trace DESIGN 5.5 defines (stderr; default output unchanged). Every
  population run sets it.
- Tests and `prism` runs go through `~/prism-evidence/tools/memguard.sh 8`; do not run mutgate; implementers do not
  push (the orchestrator does).

## Review focus

1. A variable named like a trigger word: no `user_input`. Rows `st-py-args-local`, `py-unproven-local-request`.
2. A request object passed through a helper: not `user_input`; the loss is listed. Row `py-helper-request-param` (S3).
3. Two live definitions of different origins reach one use: the higher severity wins; a `killed` or later
   definition contributes nothing. Rows `st-c-join-getenv-recv`, `py-join-env-request`, `pred-py-branch-killed`.
4. A use at module level: no panic; the syntax route finds the definition. Row `py-module-level-env` (S1).
5. A parse error in one function: no panic, other functions unchanged, nothing from it. Row `st-py-parse-error`.

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
      adjudicated and unresolved counts and Cohen's kappa.
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
      base build on it (recall per class, weighted) in `CENSUS.md`. Definition-only changes are tallied apart.

## Stage T and E: executable matrix (3.5 days)

### Task T-1: rows, fixtures, data-driven test

**Files:** create `tests/fixtures/provenance/rows.jsonl`, `tests/fixtures/provenance/cases/<lang>/<row>/` (one
directory per row), `tests/fixtures/provenance/README.md`, `tests/algo/novel/provenance_matrix_test.rs`; modify
`tests/algo/novel/main.rs` (`mod provenance_matrix_test;`) and the three coverage lists.

Row fields: `id`, `group`, `class`, `files` (under the row's directory; several for repository-local shadowing),
`diff` (file → lines; only these files are parsed, as the review loader does), `want` (`[{file, line, var,
origin}]`, `origin` an `Origin::name()` or `none`: the expected product output, always executed), `exact_lines` (no
other provenance finding on those lines, always executed), `truth` (what the value really is), `accepted_loss`
(null or the reason `want` differs from `truth`, stage D), `source` (`probe`, `corpus`, `grammar`,
`enumerator:<seat>`). The test parses the `diff` files with `Language::from_path`, builds `CpgContext::build(&files,
None)`, runs `provenance_slice::slice_in_repo(&ctx, &diff, Some(row_dir))` (from S2; until then `slice(&ctx,
&diff)`), reads origins by the prefix `variable '<var>' has <origin> origin:`, fails naming every failing row id,
and reports a row whose output now equals its `truth` while `accepted_loss` is set (an improvement, for
adjudication).

```rust
// tests/algo/novel/provenance_matrix_test.rs
const ENABLED_GROUPS: &[&str] = &["control"];
#[test]
fn test_provenance_matrix_rows() { /* rows.jsonl; skips disabled groups only; accepted-loss rows run */ }
```

Groups: `control` (green on base, stays green), `structural` (S1), `py` (S3), `js` (S4), `go` (S5). S2 and S6 have
no matrix group (no behaviour change; projection only): their tests are unit and projection tests. Seed rows:

| Family | Rows and required behaviour |
|---|---|
| Python origins | `py-ui-flask-args` route + `request.args.get` → user_input; `py-ui-django-get`; `py-ui-fastapi-query` annotated parameter; `py-ui-argv`; `py-ui-input`; `py-db-sqlite-hop2` connect → cursor → fetchone → database; `py-db-sqlalchemy`; `py-ext-requests` `requests.get` and `resp.json()` → external_call; `py-env-environ-get`, `py-env-subscript`, `py-env-getenv`, `py-env-from-import` → env_var; `py-cfg-configparser`, `py-cfg-yaml` → config; `py-const` → none; `py-param` → function_param; `py-prop-1` one alias step keeps the origin; `py-prop-limit` one past the depth → none |
| JS/TS origins | `js-ui-express-query`, `js-ui-koa-ctx`, `js-ui-fastify`, `ts-ui-nest-query`; `js-ui-argv`; `js-env-process`, `js-env-destructure`; `js-ext-axios`, `js-ext-fetch`; `js-db-pg-pool`; `js-cfg-require-config`, `js-cfg-readfile-json`; `ts-ui-as-request` `(req as Request).body`; `tsx-control`; `js-browser-cookie` `document.cookie` (truth user_input, behaviour per stage D) |
| Go origins | `go-ui-formvalue`; `go-ui-one-byte-q` `q := r.URL.Query().Get("q")`; `go-ui-url-path` (non-call `r.URL.Path`); `go-ui-gin-query`; `go-ui-os-args`; `go-ui-flag` (`*p`); `go-env-getenv`, `go-env-lookupenv`; `go-db-queryrow-scan` (`row.Scan(&name)`); `go-ext-http-get` and `resp.Body`; `go-cfg-viper` |
| Predecessors (`pred-*`) | `pred-py-branch-killed` (input overwritten by literals in both arms → none: the review's `branch.py`); `pred-py-future-try`, `pred-js-hoisted-var`, `pred-c-future-recv` (a read before a later source definition, no loop → none at the earlier read); `pred-py-loop-carried` (source defined later in a loop body, read at the loop top → user_input); `pred-py-try-cfg-incomplete` (live `cfg_incomplete` edge from a request read → user_input, grade nameonly); `pred-py-sameline` |
| Occurrence roles (`occ-*`) | `occ-py-attr-same-name` (local `label = input()`, then `first = obj.label`: no finding for `label` on that line); `occ-ts-local-over-import` (`const lib = process.argv[2]` beside `import * as lib`: user_input); `occ-py-local-os`, `occ-go-local-os`; `occ-py-callee-sorted`, `occ-go-callee-println` (dropped as callees); two occurrences of one name with different roles on one line |
| Positions (`pos-*`) | `py-module-level-env`, `pos-js-module`, `pos-ts-module`, `pos-go-package-init` (syntax route); module-level name read in a function; closure capture; class body versus method; multiline formals; `pos-go-member-only-formal` (`obj.Label` → function_param); `pos-java-declarator` (`ResultSet rows = stmt.executeQuery(..)`) and `st-java-jdbc-separated` (declaration, then assignment) |
| Value semantics (`val-*`), truth none | `sqlite3.complete_statement(..)`, `requests.utils.quote(..)`, `http.StatusText(200)`, `axios.isAxiosError({})`, `conn.close()`, a session constructor, `flag.NewFlagSet(..)`, `yaml.safe_load` of a literal and of an HTTP body (keeps the input's origin), Go `v, err := <db read>` (`err` none), `Scan(&name)` returns only `err` |
| Transfers (`xfer-*`) | `number = fixed(raw)` with `fixed` returning 7 → none; JS `const n = (raw, 7)` → none; sibling and unused arguments; direct alias at depth D (kept) and D+1 (none, traced `depth`); a closure over a proven handle |
| Request position (`rq-*`) | Go `id, err := strconv.Atoi(r.FormValue("id"))` (`err` none, `id` per stage D's pass-through list); Go `name := r.FormValue("q"); count := 7` on one line (`count` none); Django `name, count = request.GET["q"], 7` (`count` none); a multiline call whose start line differs from the assignment's; Go block-rebound `r` of a local type |
| Decoys, truth none, each identity family | `requested_at`/`requestedAt`, `form_factor`, `fetch_config(name)`/`fetchConfig(name)`, a module named `requests`, a `settings` literal, `SETTINGS`; the trigger in a comment, docstring, string, f-string or template literal, type annotation, decorator; wrong-language patterns `self.data.x`, `module.exports`, `x = read(fd)`; precedence `await db.query(sql)` on a proven `pg` handle → database; `if n == 1:` is no constant; Go `err` in a function whose signature says `error` |
| Scoping | `st-py-args-local` (`args = sorted(tokens)`; `return args` → none); one name, different origins in two functions; nested function reading an outer variable; comprehension variable; `global`; shadowing parameter |
| Hops and helpers | depth 1, 2, one past the depth; factory `conn = get_conn()` → none; handle defined twice → none; `with httpx.Client() as c`; `self.client` set in `__init__`; `py-helper-request-param`; `py-flask-request-outside-route` (truth user_input, behaviour per stage D) |
| Unproven bindings | `py-unproven-local-request` (`tests/fixtures/targets/client.py` shape); `from mylib import request`; local `requests = []`; `id-py-local-module-unchanged` (`requests.py` with a constant `get` beside `app.py`, only `app.py` in `diff` → none, no hint), also as `requests/__init__.py`, aliased and `from` imports; import visible only in another function; JS `const req = build()` outside a handler; unregistered `(req, res)` function; Go local `r` not `*http.Request`; refusals: JS reassigned `require` binding, a `require` parameter, a catch parameter `fetch`, Python `requests.get = lambda *_: 7`, a Go receiver named `http`, `import {get as read}` and `import {createServer as read}` from `node:https`, Go `pgx/v5` and dot imports |
| Joins, edges | `st-c-join-getenv-recv`, `py-join-env-request`, `st-py-parse-error`, a 1-byte variable per family |
| Pattern languages | `control`: C `ioctl` → hardware, C `recv` → user_input, C `getenv` → env_var, Rust stdin, Lua `io.read`, `os.getenv`, Terraform `var.`; `structural`: C `p = 0` no constant, `recv(` in a C comment, `getenv(` in a C string, `st-c-spread-not-read` (local `spread(9, 5)` → none), Bash `read x`, `read -r value` and `$1`, a shell `$(curl ..)` inside a quoted string, Java `System.getenv`, Rust `libc::getenv` (behaviour per stage D) |

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

- [ ] The design seat amends DESIGN 5 from `rows.jsonl`, `CENSUS.md`, `base-final.jsonl`, `census-final.jsonl`,
      `t/red-on-base.txt`. It fixes: the value-read node kinds per language (5.1); per-language per-callable tables
      with value semantics (database, HTTP, configuration, environment, command-line; payload slots of multi-result
      calls; output arguments; Django `settings`; which parsers' inputs count as configuration sources); the
      pass-through call list and its depth; the handle-hop depth; the recognised handler parameter per framework
      (Flask's imported `request` inside and outside a route, Django, FastAPI, Express, Fastify, Koa, NestJS,
      net/http, gin) and the request object's own origin, under the rubric frozen at M-3 (a change re-adjudicates
      both samples); JS browser sources; receiver names (`self`, `cls`, `this`, Go receivers); Terraform and Bash
      variable selection and the tables' language map; the pattern-language additions or losses of DESIGN 5.3 (Java
      JDBC, C-API calls from Rust and Lua, Bash `read -r`); which existing expectations are substring artefacts
      (research section 3); the finite list of accepted cross-check exceptions. It lists accepted losses; the
      implementer then sets `want` and `accepted_loss`. Both reviewers; cap 2 rounds.
- [ ] After D (ORCHESTRATION section 7): run the final `rows.jsonl` on base with every group enabled; record
      `t/red-on-base-final.txt` with the file's SHA-256 and each row's base output. A slice's RED is its group's
      rows in that file; a row added in E or amended in D that already passes on base is a control, not a RED.

## Stage I: slices

Order: S1 removes most noise in every language without identity; S2 makes identity reachable with no behaviour
change; S3 to S5 move one family each to proofs (Python first: largest share of the population and of the noise);
S6 changes the hint once all three are on proofs. Each slice branches from updated `main`, enables its group in its
first commit and shows RED against `t/red-on-base-final.txt`. Before review: write the prediction; run the
population with merge base and head (the previous slice's head run serves as base only when
`git diff --quiet <previous head> <merge base> -- src crates/prism-review/src Cargo.lock vendor` holds; otherwise
build and run the merge base); put `m/compare.py`'s tables, its reconciliation sums and each removed class with its
count in the PR body. **Precision check before merge** (DESIGN 7): a slice that changes default output for a
language labels a sample of its head's findings for that language (M-3's sampler, cells and readers; S1 for every
measured family, S3 Python, S4 JS/TS, S5 Go) and scores the census recall per class; a cell under the Precision gate
blocks the merge and goes to the design seat, never to an orchestrator decision. Every slice: section 5 items 1 to 4,
8, 9, 10 (`CLAUDE.md` provenance paragraph, `ALGORITHMS.md` entry 23), 11; two reviewers.

### Slice S1: structural fixes for every language (4 days)

**Files:** create `crates/prism-review/src/algorithms/provenance/mod.rs`, `provenance/occurrence.rs`,
`provenance/patterns.rs`, `provenance/patterns_tests.rs`, `provenance/occurrence_tests.rs`,
`crates/prism-review/tests/cli/provenance_trace_test.rs`; modify `crates/prism-review/src/algorithms/mod.rs`
(`mod provenance;`), `provenance_slice.rs` (tables and classification move out; `slice()` delegates;
`selected_origin_evidence` is replaced by the selected edge), `crates/prism-review/tests/cli/main.rs`
(`mod provenance_trace_test;`), coverage lists, the re-fixtured tests below.

```rust
// provenance/mod.rs
pub(crate) const PROPAGATION_DEPTH: usize = /* DESIGN, stage D */;
pub(crate) struct FileEnv<'a> { pub file: &'a str, pub parsed: &'a ParsedFile, pub suppressed: BTreeSet<String> }
pub(crate) enum Route { Pattern { table: &'static str, pattern: &'static str }, Literal, Parameter,
    RequestSource, Proven { path: String }, Builtin { path: String }, Propagated { depth: usize } }
pub(crate) enum DefSource { Graph(FlowConfidence), Syntax }
pub(crate) struct Verdict { pub origin: Origin, pub origin_file: String, pub origin_line: usize,
    pub def_bytes: (usize, usize), pub source: DefSource, pub route: Route,
    pub parents: Vec<(String, usize)>, pub chain: Vec<usize> }
pub(crate) struct Examined { pub name: String, pub occurrence: (usize, usize), pub use_loc: Option<VarLocation>,
    pub defs: Vec<VarLocation>, pub verdict: Result<Verdict, &'static str> } // Err = prov-unknown reason
pub(crate) fn examine_line(ctx: &CpgContext, env: &FileEnv<'_>, line: usize) -> Vec<Examined>;
pub(crate) fn live_defs(ctx: &CpgContext, use_loc: &VarLocation) -> Vec<(VarLocation, FlowConfidence)>;
pub(crate) fn join(verdicts: Vec<Verdict>) -> Option<Verdict>;   // highest severity, then earliest origin line
pub(crate) fn severity(origin: &Origin) -> Option<&'static str>; // today's map, moved
// provenance/occurrence.rs
pub(crate) struct Occurrence<'t> { pub node: tree_sitter::Node<'t>, pub name: String }
pub(crate) fn value_reads<'t>(parsed: &'t ParsedFile, line: usize) -> (Vec<Occurrence<'t>>, Vec<(String, &'static str)>); // reads, drops with role
pub(crate) fn syntax_defs<'t>(parsed: &'t ParsedFile, occ: &Occurrence<'t>) -> Vec<tree_sitter::Node<'t>>;
// provenance/patterns.rs
pub(crate) fn code_text(parsed: &ParsedFile, node: tree_sitter::Node<'_>) -> String; // comments, strings blanked
pub(crate) fn matches_word(text: &str, word: &str) -> bool;      // every occurrence, both boundaries
pub(crate) fn classify_text(language: Language, code: &str) -> Option<(Origin, &'static str, &'static str)>;
```

Behaviour (DESIGN 5.1, 5.3): examined occurrences are `value_reads` at the line (per-language role table; an
occurrence named in `ctx.cpg.call_graph.imports[file]` is dropped as a module unless the enclosing function binds
the name, by a graph Def or a syntax binding, so a local of the same spelling is examined); every other identifier on the line gets a
`prov-drop`. A graph Use (`ctx.cpg.nodes_at`, `to_var_location`) is matched to an occurrence by equal start and end
bytes; an unmatched graph Use is ignored. Reaching definitions: `live_defs` keeps `ctx.cpg.dfg.predecessors(use)`
that are `Def`s of the same function, dropping an edge whose `ctx.cpg.dfg.label(def, use)` is
`NameOnly(FlowDoubt::Killed { .. })` and a def whose `start_byte` follows the use's unless a loop node of the
function encloses both. An occurrence with no matched Use, or a Use with no predecessor at all, takes `syntax_defs`
(DESIGN 5.1, syntax route). A definition's origin: a parameter → `function_param`; a literal value (string without
interpolation, number, boolean, null, an empty list, map, tuple, array or object) → `constant`; else the tables on
`code_text` of the defining value expression only (the right-hand side, or the call an output argument belongs to;
shell `$(..)`, backticks and HCL `${..}` stay code); else the propagation forms of DESIGN 5.2 up to
`PROPAGATION_DEPTH` (until stage D: plain alias and member or element only). Tables keep today's entries, keyed by
their `// === <Language> ===` sections and the stage-D language map; `~` words need both boundaries; an entry ending
in `(` must start a whole callee token; precedence stays. The diff-line and signature-substring fallbacks are
removed. Python, JS/TS and Go use this regime until S3 to S5. The finding's evidence is
`EvidencePath::from_hops(vec![EvidenceHop::DataFlow { from: def, to: use, confidence }])` of the selected graph edge;
a propagated or syntax-route origin is `EvidencePath::unlabeled_for("provenance")`. Every examined occurrence prints
one `prov-var` or `prov-unknown` (DESIGN 5.5).

- [ ] Enable `structural`: RED. Unit tests in `patterns_tests.rs`: `boundary_rejects_format_for_form`,
      `boundary_rejects_fetch_config_for_fetch`, `boundary_checks_every_occurrence` (`formatted form`),
      `call_entry_needs_whole_callee_token` (`spread(9, 5)` does not match `read(`),
      `classifier_reads_value_not_defined_name` (`args = sorted(tokens)` → none),
      `code_text_blanks_comments_and_strings`, `code_text_keeps_shell_substitution`,
      `terraform_tables_do_not_run_on_python`, `join_prefers_concern_over_info`. In `occurrence_tests.rs`:
      `zero_width_use_is_not_examined` (the `occ-py-attr-same-name` shape: no `label` occurrence on line 15),
      `local_shadowing_an_import_is_examined`, `callee_is_dropped_with_role`, `killed_predecessor_contributes_nothing`
      (`branch.py` shape → no origin, reason `killed-only`), `later_definition_without_loop_contributes_nothing`,
      `loop_carried_definition_contributes`, `module_level_read_takes_syntax_route`. CLI tests:
      `trace_reconciles_every_examined_occurrence` (each examined occurrence has exactly one `prov-var` or
      `prov-unknown`), `no_trace_without_the_variable`.
- [ ] Implement; GREEN for `control` and `structural`.
- [ ] Re-fixture to a Django view (`from django.http import HttpResponse`, then `def view(request):`), which both
      regimes call user input, so S3 leaves them alone; lines and `kill_line` shift by one.
      `tests/common/mod.rs` `assert_finding_evidence_provenance_cases`: the killed case (`clean(query)` in both arms)
      now asserts no finding for `query` (the only source route is `killed`; `clean(..)` is no pass-through); the
      nameonly case becomes `query = request.GET['q']`, `try:`, `query = c(query)`, `finally:`,
      `cursor.execute(query)` (its source edge is `nameonly/cfg_incomplete` on the review binary; confirm with
      `nav --no-cache dfg-stats --edges` when writing), expected hop `FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)`;
      the bridge case becomes `term = request.GET['q']`, `t2 = term`, `cursor.execute(t2)`, needle `variable 't2'`,
      still `unlabeled` (propagated). `min_confidence_test.rs`: add `provenance_flow_source(name_only)` beside
      `flow_source` (taint keeps it), with the `try` fixture for the nameonly grade, for
      `evidence_delivery_provenance_reaches_sarif_with_all_three_grades` and its bridge;
      `evidence_delivery_local_clean_flow_still_reaches_sink_when_d_drops_the_shortcut` uses `term`, diff line 5,
      `related_lines` `[3, 4]`. `min_confidence_admission_test.rs`
      `default_nameonly_threshold_retains_an_unlabeled_finding`: same bridge. Any other flipped test must be on
      stage D's artefact list and is named with its reason in the PR body.
- [ ] Run (memguard, `RAYON_NUM_THREADS` 1 and 4): `cargo test --test algo_novel provenance`;
      `cargo test --test integration -- dfg_label_parity_test:: targets_mapping_test:: coverage_test::`;
      `cargo test --test cli -- min_confidence provenance_trace_test:: targets_test:: validation_test::`;
      `cargo test -p prism-review --lib provenance`; and filter `provenance` in `lang_c`, `lang_cpp`, `lang_bash`,
      `lang_terraform`, `lang_java`, `lang_go`, `lang_typescript`, `lang_tsx`.
- [ ] Expected to stay green: the C, C++, Rust, Lua tests (def lines hold the API); Bash and Terraform tests cannot
      flip. Gates: Contract (`targets_mapping_test::` unmodified); the precision check (Stage I) for py, js, go and the
      four pattern-language families; census recall for py, js, go; Lua, Bash, Terraform by matrix only (DESIGN 8
      decision 6).

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
pub(crate) fn request_match(cx: &IdCx<'_>, parsed: &ParsedFile, value: Node<'_>, target: Node<'_>) -> bool;
```

`CpgContext` has no repository root (`src/cpg/context.rs:39`); provenance gets it from `run_algorithm_raw`, which
receives it. With `repo_root: None` a Python import proof is refused (`no-repo-root`), never checked against
`known_files` alone; `kind_for_binding` always has the root and keeps its results. `Builtin` means the root is
neither imported nor bound locally. `call_path` and `read_path` apply DESIGN 5.4's site refusals on top of
`proven_path` and return the refusal reason. `request_match` implements DESIGN 5.2's position rule: a source whose
`start_byte` is the target's, or, for a seed without a target, an accessor of the framework spec's `sources` table
(`prism::frameworks`, calls and non-call fields alike) inside `value`'s slot for `target`, rooted at a handler
formal by binding at the accessor.

- [ ] Tests first in `identity_tests.rs`: `proven_path_os_environ_get_is_import_os` (module `os`, rest
      `["environ","get"]`), `proven_path_refuses_shadowing_parameter`, `proven_path_refuses_repo_local_module_on_disk`
      (`requests.py` in a temp repository root, not in `known_files`), `proven_path_refuses_python_import_without_root`,
      `proven_path_process_env_is_builtin`, `proven_path_go_os_getenv`, `named_import_keeps_original_member`,
      `refuses_reassigned_require_binding`, `refuses_local_require_loader`, `refuses_catch_bound_fetch`,
      `refuses_replaced_member`, `refuses_go_receiver_named_like_package`, `request_sources_flask_route_lists_assignment_line`,
      `request_sources_express_lists_handler_param`, `request_sources_net_http_lists_formvalue_line`,
      `request_match_go_same_line_other_target_is_false` (`name := r.FormValue("q"); count := 7` → `count` false),
      `request_match_django_tuple_pairs_slots`. RED (do not compile; the behavioural assertions are the refusal and
      match tests, which must fail on a stub returning a proof).
- [ ] Implement; `provenance/mod.rs` builds `IdCx` per diff file and prints `prov-request-source` (DESIGN 5.5) with
      the diff lines that read each matched target's binding.
- [ ] Gates: matrix, `--test cli targets_test::`, `--lib targets::dependency_hint` unchanged; population findings
      byte-identical to the base (only stderr differs); the same run writes `g/request-uses.tsv` (the cross-check
      list and the census's request-source part, fixed here before any behaviour slice).

### Slice S3: Python origins by proof (3.5 days)

**Files:** create `provenance/python.rs`, `provenance/python_tests.rs`; modify `provenance/identity.rs`,
`provenance/mod.rs` (Python on the identity regime), `crates/prism-review/tests/cli/targets_test.rs`, and the tests
stage D named for Python.

```rust
// provenance/identity.rs
pub(crate) const HANDLE_HOP_DEPTH: usize = /* DESIGN, stage D */;
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

Rules: DESIGN 5.2 as amended. User input when `request_match` holds for the definition's value slot and target, or
the value is a listed command-line read; database or external call only for an operation the Python per-callable
table lists as payload, on a proven module or on a handle whose single reaching definition is a listed constructor
or transition, up to `HANDLE_HOP_DEPTH`; environment and configuration by the amended Python tables over
`PathProof`; literal; parameter; propagation only through the listed forms; anything else `unknown`, traced with the
reason.

- [ ] Enable `py`: RED. Unit tests: `cursor_two_hops_is_database`, `factory_handle_is_unknown`,
      `requests_get_is_external_call`, `requests_utils_quote_is_unknown`, `sqlite_complete_statement_is_unknown`,
      `environ_subscript_is_env`, `local_request_object_is_unknown`, `fixed_call_does_not_inherit_argument`,
      `with_target_handle_follows_amended_rule`. Implement; GREEN.
- [ ] `targets_test.rs` `default_run_emits_all_five_producers_and_live_mappings`: its `user_input` target came from
      the repository-local `request = Request()` (`tests/fixtures/targets/client.py:10,15`), a substring artefact.
      Assert a provenance target for `variable 'user'` with `function_param` (line 14) and none containing
      `user_input`; the fixture stays.
- [ ] Other expectations: `test_provenance_python_request_form_origin` (Flask `request` in a function with no route)
      follows stage D's Flask rule; `test_provenance_python_cursor_execute_origin` needs a hop depth of at least 2.
      Gates: the precision check (Stage I) and census recall per class on the 100 Python diffs (`--only py`);
      Cross-check on Python rows of `g/request-uses.tsv`; Contract.

### Slice S4: JavaScript/TypeScript origins by proof (3.5 days)

**Files:** create `provenance/js_ts.rs`, `provenance/js_ts_tests.rs`; modify `provenance/mod.rs`,
`docs/features/language-coverage/js-ts-known-gaps.md` (a provenance entry: what is proven, what is not).

`JsTsReader` (`impl DefReader`) for JavaScript, TypeScript and TSX, with `as`, `!` and `satisfies` peeled, the
comma operator's last operand only, and declarators and destructuring as amended. Request sources from Express,
Fastify, Koa, NestJS by `request_match`; `process.argv` and `process.env` as built-ins; built-in `fetch` and the
listed JS `http` operations as external call; listed `db` operations with handle hops; browser sources as stage D
decided.

- [ ] Enable `js`: RED. Unit tests: `express_query_field_is_user_input`, `unregistered_req_param_is_param`,
      `pg_pool_query_is_database_not_user_input`, `axios_is_axios_error_is_unknown`,
      `process_env_destructure_follows_amended_rule`, `as_request_wrapper_is_seen_through`,
      `comma_expression_takes_last_operand`. Implement; GREEN.
- [ ] `test_provenance_js_document_cookie_origin` follows stage D's browser rule; `..._process_env_origin` stays.
      Gates: the precision check and census recall on the JS/TS part (40 diffs, `--only js`); Cross-check on JS/TS
      rows; Contract.

### Slice S5: Go origins by proof (3 days)

**Files:** create `provenance/go.rs`, `provenance/go_tests.rs`; modify `provenance/mod.rs`.

`GoReader` (`impl DefReader`): `:=`, `var`, multi-value assignment (each name its own value position; only a listed
payload slot carries an origin) and out-parameters as amended; net/http and gin request sources by `request_match`
(line seeds are hints; non-call fields such as `r.URL.Path` from the spec table); `os.Args`, `flag.*`, `os.Getenv`,
`os.LookupEnv` via `PathProof`; listed `database/sql`, `pgx`, `gorm` operations and handles; listed `net/http` client
calls (not `http.StatusText`); viper.

- [ ] Enable `go`: RED. Unit tests: `formvalue_on_request_param_is_user_input`, `one_byte_variable_is_examined`,
      `same_line_other_assignment_is_not_user_input`, `err_slot_carries_no_origin`,
      `err_is_not_function_param_by_signature`, `queryrow_handle_is_database`, `status_text_is_unknown`,
      `viper_getstring_is_config`. Implement; GREEN.
- [ ] `test_provenance_go_form_value_origin` and `..._viper_config_origin` stay (viper must be in the Go
      configuration table). Gates: the precision check and census recall on the Go part (40 diffs, `--only go`);
      Cross-check on Go rows; Contract.

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
      every row with a `dependency_hint` joins a `prov-var` line whose route is `Proven` or `Builtin`, or
      `Propagated` with a parent chain ending in one; Contract.

## Stage G: lane gate (2.5 days)

- [ ] Final head run with `--targets`. `g/REPORT.md`: volume by family × origin × severity, base against head, with
      `m/compare.py`'s reconciliation, both cumulative (M base to final head) and per slice; each removed class with
      its count (owner decision 2's non-variable names from `prov-drop`, examined unknowns by `prov-unknown`
      reason); definition-only changes as their own class.
- [ ] Head samples (DESIGN 6): M-3's sampler, cells, seed and readers over every severity (`g/labels/head-*.jsonl`).
      **Precision** per DESIGN 7 in every cell, with intervals; small cells labelled in full and listed.
      **Recall** per origin class against `census-final.jsonl`, weighted, ≥ 95%, every loss listed with its
      `prov-unknown` or `prov-drop` reason and stage-D accepted losses marked; base retention reported beside it.
- [ ] **Cross-check**: every line of `g/request-uses.tsv` has a `user_input` finding on the head, except the
      finite stage-D list. **Hints**, **Contract** on the final head. Post the tables on the tracking issue.

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
      languages (owner decision 4); pattern-language precedence; Terraform/Bash selection, browser sources if lost.
      Refresh `HANDOFF.md`; summarise and offload raw outputs; delete the corpus clones and `target/`.

## Acceptance for the lane

`CENSUS.md` merged with the go gate passed; S1 to S6 merged with both reviewers approving and each behaviour slice's
precision check passed; on the final head every DESIGN 7 gate holds (Precision, Recall, Cross-check, Hints, Contract)
with losses listed; the report is on the tracking issue; docs and handoff current.

**Estimates:** M 4.5, T and E 3.5, D 1.5, S1 4, S2 2, S3 3.5, S4 3.5, S5 3, S6 0.5, G 2.5, close-out 1; total 29.5 days.

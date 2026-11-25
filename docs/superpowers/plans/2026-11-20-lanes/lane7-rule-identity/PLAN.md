# Lane 7: rule identity on findings — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a finding produced by a rule says which rule and, where one exists, which CWE, in JSON, review and
SARIF output, with no change to finding counts, severities, target ids, SARIF fingerprints or default SARIF
rule ids; default output gains only the additive `rules` key (DESIGN decision 7).

**Architecture:** `SliceFinding` gains `rules: Vec<RuleRef>` (core crate, skipped when empty). The taint sink
loop attributes, per sink occurrence whose operand the tainted use reaches, the rule of each flat hit and
every structured match, and attaches them to the `taint_sink` finding (DESIGN 5.2). One registry
(`output/rule_registry.rs`) lists every rule with its CWE and text, is the only way production code names a
rule (DESIGN 5.3) and generates `docs/rules.md`. SARIF always adds `properties.rules`;
`--sarif-rule-ids specific` writes one result per rule with registry descriptors and CWE tags. Primitive and
absence findings carry their rule last. DESIGN Appendices A–F are the Stage D record. Appendix E lists
the 540 matrix expectations and 300 source-only candidates, including the accepted corrections.

**Tech stack:** Rust (`prism`, `prism-review`); Python 3 for the gate script. **Spec:** `DESIGN.md` here
(field 5.1, taint 5.2, registry 5.3, others 5.4, SARIF 5.5, review/JSON 5.6, targets 5.7, gates 6, owner 7).

## Global constraints

- Worktree `~/code/prism-rule-identity`; evidence `~/prism-evidence/rule-identity/`. Branches
  `test/rule-identity-matrix`, `feat/rule-identity-s1` to `-s5`.
- Must not change: finding counts, categories, descriptions, severities, the order of `findings` and its
  index alignment with `evidence`, target ids, the targets schema, SARIF fingerprints, default-mode SARIF rule
  ids and descriptors, `MAPPING_VERSION` (`"1"`, `output/sarif.rs`). No CPG or cache change. No edit under
  `crates/prism-review/src/targets/` (lane 2 owns it). `rules` is the **last** field of `SliceFinding`, so
  the key order of existing output does not move.
- PR #841 (open; head `493ab5ee`) edits `taint/run.rs`, `cli.rs`, `main.rs`, `output/mod.rs`,
  `output/sarif.rs`, `output/sarif_model.rs`, `api/run.rs` and adds nine `SliceFinding` literals in five
  files (`taint/result_union/tests.rs`, `api/context_matrix_tests/formats/policy.rs`, `api/context_view_tests/fixture.rs`,
  `api/map_view_tests.rs`, `output/context_render_tests.rs`). Whichever of #841 and a slice merges second owns
  the conflict: it rebases, re-reads the touched code, runs `rg -n "SliceFinding \{" -g '*.rs'` and compares
  the literal inventory and the gate comparison against the integrated base. #841's SARIF loop picks an
  original or context map per finding and both CLI branches attach context and diagnostics; S4 keeps those
  choices before expanding. Its result union keeps the baseline finding per `(category, file, line)`
  verbatim, so a line found by both of its runs keeps the baseline run's rules (DESIGN goal 2); S2 adds an
  overlap and an expanded-only test if #841 has merged.
- Lane 4's sink (DESIGN 5.2, last paragraph). Before lane 4 dispatches its sink slice, the orchestrator posts
  the field contract on #603 and in lane 4's ledger: category `taint_sink`, `js-proto-write`/`CWE-1321` in
  `rules`, no tag in the description. Lane 4 merging second: its slice writes the field (and, after S3, the
  registry row). Lane 4 merging first: S2 carries the migration step of DESIGN 5.2; S3 adds the registry row.
- Weighted sizes now: `taint/run.rs` 601.5, `absence_slice.rs` 906, `primitive_slice.rs` 794,
  `contract_slice.rs` 1447, `main.rs` 990.5, `cli.rs` 483.5, `src/slice.rs` 572, `output/sarif.rs` 380.5,
  `sarif_rules.rs` 436.5. Only `contract_slice.rs` stays over 1,000 (12 one-line additions in S1; its PR
  says so); new logic goes in the new modules named below.
- Briefs say: no `mutgate`; every `cargo` and `prism` run through `~/prism-evidence/tools/memguard.sh 8`.

## Review focus

1. Two flat rules on one line (`res.send(eval(x))`): one finding, `rules` = `js-eval`, `js-response` and the
   `structured-code-eval@CWE-95`; one `specific`-mode result per distinct id, sharing a fingerprint.
   Row `ri01` (S2); `specific_mode_writes_one_result_per_rule_sharing_fingerprint` (S4).
2. Attribution per occurrence (DESIGN 5.2): a sibling sink that does not take the tainted value lends neither
   rule nor strength (`ri08`, `ri25`, `ri26`); a continued call on an admitted use line carries the eligible enclosing rules (`ri20`);
   two structured calls on one line carry both rules in either order (`ri21a`, `ri21b`); a cleansed first
   match beside a live second match (`ri22`). A cleansed structured call beside an unrelated flat sink:
   neither the structured rule nor the rules of flat identifiers inside the cleansed call appear (`ri15`).
3. Lines inserted outside the per-edge loop (Python inline Flask source, Go source equal to sink, JS YAML
   inline source) carry their structured rule; caller-supplied and wrapper callees carry `configured-sink` and `format-wrapper`, both without a CWE.
   Rows `ri11` to `ri14`, `ri23*` (S2).
4. Default-mode SARIF for findings that carry rules: no new descriptor or descriptor key, `mapping_version`
   still `"1"`, only `results[].properties.rules`. `default_mode_adds_properties_rules_and_keeps_rule_ids` (S4).
5. `specific` mode with admission, indexes and odd inputs: a multi-rule finding removed by
   `--min-confidence exact` writes nothing; each expanded result's `ruleIndex` names its own descriptor;
   repeated or conflicting references to one id give one result; an unregistered id keeps its supplied CWE
   as a tag (S4 tests of the same names).

## Stage M: issue and output baseline (1 day)

No go/no-go threshold (the research probe verified the premise). This stage builds the comparison that
G-additive, G-fingerprint and G-targets use.

- [ ] File the tracking issue "Rule identity on findings: rule id and CWE in JSON, review and SARIF", linking
      `DESIGN.md` and `PLAN.md`; every lane PR references it. Create the worktree and evidence directory.
- [ ] `m/make_all_lines_diff.py <repo dir>` prints a JSON diff (`{"files":[{"file_path", "modify_type":
      "Modified", "diff_lines":[1..N]}]}`) marking every line of every source file.
- [ ] `m/inputs/primitive/`: one file per primitive rule id, each the source string of one test in
      `tests/algo/novel/primitive_test.rs` that fires it (test names in `m/inputs/README`), with an all-lines diff.
- [ ] `m/inputs.tsv` (repo, diff; all under `tests/fixtures/`): `targets` and `targets_hint` with their
      `diff.json`; `hapi-4552-source` with `hapi-4552.diff`; `bash` with `firmware_update.diff`; `python`
      with `calc.diff`; `c` with each of its 25 `*.diff`; `sanitizer-suite-{go,js-ts,python}` with an
      all-lines diff in `m/diffs/`; `review_no_diagrams` with `test.patch`; plus `m/inputs/primitive`.
- [ ] `m/pin_inputs.sh <out dir>` copies every input repo and diff once, from the lane's merge base, into
      `<out dir>` (outside any checkout, `git init` + one commit so `repo.sha` is fixed). Every gate run of
      both binaries reads this one tree, so the targets document's `repo` block is the same on both sides.
- [ ] `m/run_outputs.sh <prism binary> <pinned inputs> <out dir>`: per input, with `ALGOS=taint,primitive,
      absence,contract,echo,membrane,symmetry,provenance,peer_consistency,callback_dispatcher` (every finding
      emitter, no git-history algorithm), write `json`, `review` (`--review-min-severity info`), `sarif`,
      the single-run `json-taint` and `sarif-taint` (`--algorithm taint`), and `targets` (`prism targets
      --algorithm $ALGOS`; its default leaves out taint and primitive). A failing input goes to
      `<out dir>/FAILED` with its stderr.
- [ ] `m/compare.py <base out> <head out> [--new-rules <scope>...]`, one line per output, non-zero exit on
      any failure. It never re-serialises: it scans each original buffer with an offset-keeping JSON
      tokenizer and works on bytes.
      - Masked on both sides, value replaced by a fixed token at the exact path only: SARIF
        `runs[*].properties.{prism_git_sha,prism_build_identity,binary_input_dirty}`, targets
        `producer.build_identity`.
      - Allowed keys: `rules` of a finding object (an object with `algorithm` and `severity`) and SARIF
        `runs[*].results[*].properties.rules`. Where both sides have the key, the values must be byte-equal.
        Where only the head has it, it is cut out (from the comma before the key through the end of its
        value) only if the finding's algorithm and output kind are in a `--new-rules` scope
        (`finding:taint`, `sarif:primitive`, ...); otherwise that is a failure. A key only on the base side
        is a failure (a dropped rule).
      - After that, the two buffers must be byte-equal. SARIF also compares the sorted
        `partialFingerprints["prism/finding/v1"]` lists; targets the ordered `targets[].id` lists.
      - Fails on a `FAILED` entry on either side, a missing output, a non-empty `errors` list in either
        document, or zero findings of an algorithm named in a `--new-rules` scope across all inputs.
- [ ] `m/test_compare.py` (finite self-test, run in M and after any edit to `compare.py`): identical enriched
      documents pass; build-metadata-only differences pass; a dropped existing rule, a changed rule, rules
      added outside the given scope, an extra or missing finding, a changed escape (`\u00e9` against `é`),
      an extra trailing newline or space, a FAILED input and an algorithm error each fail.
- [ ] Control: main's binary run twice on the pinned tree; `compare.py` shows no difference. `m/BASELINE.md`
      records finding counts per input and algorithm, so an input with no finding of an algorithm is visible.

**Gate procedure (S1 to S5):** in the lane worktree, build the slice's merge base (detached) and the
head through memguard, copying each `target/release/prism` to `bin/prism-<sha>`; `run_outputs.sh` both on
the pinned tree; `compare.py` with the slice's `--new-rules` scopes (S1 and S3 none, S2 `finding:taint`, S4
`sarif:taint`, S5 `finding:primitive finding:absence sarif:primitive sarif:absence`); paste its summary in
the PR body. At close-out, the same comparison against the pre-lane base (the merge base of S1) with every
scope shows the cumulative change.

## Stage T and E: the matrix (2.5 days, then 1 day)

The sink-rule tables (`tests/algo/taxonomy/taint_sink_rules_*_test.rs`, 503 labelled rows, about 350 of them
positive) already have one rule per row. Stage T adds an expectations file keyed by table and label, and a
table of identity rows. Nothing in stage T names `SliceFinding.rules`, so the branch compiles before S1. The seed table below
records the pre-D work order. DESIGN Appendix E now supersedes its provisional `?` and concrete values.

### Task T-1: expectations and identity rows

**Files:** create `tests/fixtures/taint_sink_rules/expected_rules.tsv`,
`tests/algo/taxonomy/taint_sink_rules_identity_test.rs`, `tests/algo/taxonomy/taint_sink_rules_expect_test.rs`;
modify `tests/algo/taxonomy/main.rs` (two `mod` lines), `tests/algo/taxonomy/taint_sink_rules_support.rs`.

TSV columns: `table` (test file stem, e.g. `taint_sink_rules_js_test`), `label` (the row's label, exactly),
`rules` (the expected identity: `;`-separated, sorted by id, each `id`, `id@CWE-n`, optionally followed by
`!suggestion` for a per-rule severity; `-` empty; `?` fixed by stage D), `observed` (what the code emits
today, read from the code; `=` when equal to `rules`), `class`, `note`. One line per label of the eight
tables and the identity table, negatives included (class `negative`, rules `-`), and the three
`assert_severities` rows of `taint_sink_rules_go_test.rs`; a row run under JS and TS has one line. Classes:
`flat`, `flat-multi`, `flat-dup`, `structured`, `structured-multi`, `flat+structured`, `cleansed-structured`,
`continued`, `sibling`, `legacy`, `source-line`, `yaml-inline`, `wrapper`, `caller-sink`, `weak-mixed`,
`negative`. The `rules` value states what the operation and its operand are (a SQL statement, code text, a
command, a path, a URL, HTML), never only what the code says; where they differ, `observed` records the
code's value and stage D gives the disposition (DESIGN 5.2).

```rust
// taint_sink_rules_support.rs (no field access)
pub struct Expectation { pub table: String, pub label: String,
    pub rules: Option<Vec<String>> /* None = "?" */, pub observed: String, pub class: String }
/// Rejects a bad column count, an unsorted or duplicated rule list, an id outside `[a-z0-9-]+`,
/// a CWE outside `CWE-<digits>`, a severity mark other than `!suggestion`, a duplicate (table, label).
pub fn parse_expected_rules(tsv: &str) -> Result<Vec<Expectation>, String>;
/// Taint findings with caller-supplied sink names, through `prism_review::algorithms::taint::slice` with
/// `TaintConfig { extra_sinks, ..Default::default() }` on `CpgContext::build` (the compatibility runner
/// always uses the default configuration).
pub fn findings_with_sinks(path: &str, language: Language, source: &str, extra_sinks: &[&str])
    -> prism::slice::SliceResult;
```

`taint_sink_rules_expect_test.rs`: `expected_rules_tsv_is_well_formed`,
`tsv_and_tables_name_the_same_labels` (per table, the set of `label: "<label>"` strings plus the first
argument of each `go("<label>", ..)` row in the file equals the set of TSV labels for that table: both
directions), `parser_rejects_unsorted_rules`, `parser_rejects_bad_cwe`, `parser_rejects_bad_severity_mark`.
`taint_sink_rules_identity_test.rs`: `const IDENTITY_ROWS: &[Row]` and `identity_rows_fire_as_marked`
(`assert_rows`), and `const CONFIG_ROWS: &[(Row, &[&str])]` with `config_rows_fire_as_marked`
(`findings_with_sinks`), green on main (sink presence only until S2). Seed rows:

| Row | Line | Expected rules |
|---|---|---|
| ri01 (ri19: TSX) | JS `res.send(eval(x));` | `js-eval@CWE-95;js-response@CWE-79` and the structured `eval` match (`?`) |
| ri02 | JS `el.innerHTML = x; cp.exec(x);` (`cp` from `child_process`) | `js-child-process@CWE-78;js-dom-html@CWE-79` and the structured `exec` match (`?`) |
| ri03, ri04 | JS `fetch(x)`; Python `pickle.loads(x)` | structured SSRF; structured deserialization (`?`) |
| ri05, ri05b, ri06 | Go `os.Remove(x)`; Go `os.ReadFile(x)`; Python `cur.execute("SELECT " + x)` | `go-file-op@CWE-22` plus structured path; structured path only; `py-db-call@CWE-89` plus structured SQL (`?` for structured) |
| ri07 | JS `res.send(x); res.send(x);` | `js-response@CWE-79` once |
| ri08 | Python `cur.execute("SELECT 1"); os.system(x)` | `py-os-command@CWE-78` only |
| ri09, ri10, ri16–ri18, ri27 | C `system(x)`; Bash `eval "$x"`; one Java, Rust, Lua sink; one Terraform/HCL attribute | `legacy` (ri27: or a recorded limit if no fixture reaches the matcher) |
| ri11, ri12, ri13 | Go gin `c.File(c.Param("f"))`; Python Flask sink with inline `request.args[..]`; JS `yaml.load(req.body)` | each its structured rule (`?`) |
| ri14 | C variadic wrapper forwarding to `vprintf`, called with the tainted value | `?` |
| ri15 | Go: a cleansed structured call and an unrelated flat sink on one line | the flat sink's rule only |
| ri20 | ri06 with the argument on the next line (`cur.execute(` / `"SELECT " + x` / `)`, SINK on the argument line) | as ri06 |
| ri21a, ri21b | Python `requests.get(x); pickle.loads(x)` and the reverse order | both structured rules (`?`) |
| ri22 | a cleansed first structured match and a live second one, on a line a flat sink admits | the live match's rule and the flat rule (`?`) |
| ri23a–ri23f (config rows) | a caller-supplied name only; a wrapper only; both; a name that overlaps a built-in sink; `=name` exact against a substring; the same source without the option (negative) | `?` |
| ri24 | JS `db.exec(sql)` on a `sqlite3` handle | `js-db-call@CWE-89` and the structured `exec` match (`?`) |
| ri25 | Go `c.HTML(200, "t", x)` on a declared `*gin.Context`, with `exec.Command("ls")` on the line | `go-html-render@CWE-79!suggestion` |
| ri26 | JS `res.send("ok"); el.innerHTML = x;` | `js-dom-html@CWE-79` only |

- [ ] Write the rows; one that does not fire on main is reshaped until it fires, or kept with class and a
      note saying why it cannot fire (ledger note); no mandatory row is dropped.
- [ ] Fill `rules` for every row from the operation and the operand, and `observed` by reading the rule code
      (`taint_sinks/{js,js_modules,python,py_modules,go,legacy}.rs`: tags are chosen where `SinkHit::new` or
      a local `hit(..)` is called; structured tables under `taint/{js_ts,python,go}/`). A never-committed
      patch printing each row's hits may draft `observed`; each value is then checked by reading. Uncertain
      rows get `?` and a note.
- [ ] Gates: `cargo fmt --all --check`; `cargo test --test algo_taxonomy taint_sink_rules` at
      `RAYON_NUM_THREADS` 1 and 4. One reviewer (test-only).

### Task E-1: enumeration

- [ ] Run the tiers (ORCHESTRATION section 6) on the identity table. Axes: hits per line (none, one, several,
      one rule twice); flat and structured mixes; cleansed structured calls; the non-edge insertion branches
      of `taint/run.rs` (YAML inline, Python inline Flask, Go source line with and without an originating
      path, unquoted expansion); framework-source lines; weak SQL beside strong sinks; caller-supplied and
      wrapper sinks; each language without positional rules; TS and TSX; several paths reaching one line
      through different identifiers; a sibling sink that does not take the tainted value (weak or strong,
      literal or other argument); callees and arguments continued over lines (first, middle, last line;
      nested calls; cleansed and live); two or more structured calls on one line, both orders, and several
      originating Go paths; operand swaps (the tainted value in the data rather than the path, the template
      rather than the context, the format rather than the writer). Accepted rows go into both files.

## Stage D: design of record (1.5 days)

- [ ] Inputs: the TSV and identity rows; the 34 flat tags with their CWEs (research section 2) and `legacy`;
      every structured pattern, framework sink table and special matcher (`taint/{js_ts,python,go}/`), with
      its `SanitizerCategory`; the six primitive categories with their `rule_description` sentences
      (`output/sarif_rules.rs`); the 65 `PairedPattern` descriptions (`default_pairs`) and the four absence
      categories each can be emitted under; `js-proto-write`/CWE-1321 if lane 4's sink has merged. The
      design seat amends DESIGN 5.2 to 5.4 (and every PLAN slice section the amendment contradicts) and fixes:
      (1) **the expected identity of every rule** from the operation and the operand, beside the code's
      value, as an appendix table with one disposition per difference: same; corrected (a tag, a CWE or the
      structured mapping; only where no finding's presence or severity changes); or filed, published without
      a CWE until corrected. It covers at least the three known differences (DESIGN 5.2) and the candidates
      the design review listed: the JS `vm` family and `Function`, Mongoose `$where`, Flask
      `render_template_string` with a tainted template against a tainted context value, fixed-destination
      content writes (`fs.appendFileSync('a', x)`, an open stream's `write`), browser `open(url)` and
      `webbrowser.open`, `setattr` on a plain record, `fmt.Fprintf(w, x)` to a response writer. No row of
      the appendix may publish an identity it records as wrong;
      (2) the structured rule ids and CWEs per pattern (`structured_rule`, decision 3);
      (3) what a caller-supplied (`TaintConfig.extra_sinks`) or format-string wrapper sink carries: nothing
      or a named rule, and whether the two origins are distinct (`ri14`, `ri23*`);
      (4) the per-rule severity rule of DESIGN 5.2 against the weak emitters (JS/Go/Python SQL, Python SQL
      clause, Python sandboxed template, Go HTML render);
      (5) absence: each pair's id (a stable slug, unique across algorithms) and CWE or none, and what a
      `double_close` finding carries (a double free, a repeated unlock and a generic release are not one
      weakness); (6) the CWE of each of the six primitive categories (DESIGN names five classes; two
      categories are hash truncation); (7) conventions for registry `name`, `short`, `help`, and the
      `legacy` row's text. Output: amended sections with appendix tables (identities, structured, absence,
      primitive). D-1 delivers only these two design documents. In the matrix implementation commit, replace all
      125 `?` entries and 26 revised concrete values on the reviewed matrix `5e9e4d864cbb`. Preserve the
      observations. Replay those 151 cases plus `ri23e`, `ri23f` and `ri27` on the base (ORCHESTRATION
      section 7). This covers the 154 cells from the brief's original matrix `a3986ce333f1`. Appendix E
      reconciles the counts. Preserve all 540 keys. Candidate rows and open grids are source-only until replayed.
      Both reviewers; cap 2 rounds.

## Stage I: slices

S1 may run during T and E (it needs no rule names). S2 to S5 follow D in this order:

| Slice | Why here |
|---|---|
| S1 field | every later slice writes or reads it; a behaviour-free PR merges fast and narrows the #841 window |
| S2 taint emission | the main value and lane 4's prerequisite; needs D's identity table |
| S3 registry | SARIF descriptors and the primitive/absence CWEs read it; it closes S2's producers by type |
| S4 SARIF | needs the registry; serves whatever carries rules |
| S5 primitive, absence | DESIGN's second slice and the smallest value; needs the registry's CWEs |

DESIGN 5.6 needs no slice: `--format review` and `--format json` both serialise `SliceFinding` (no other
format prints more of a finding than its description), so the field appears with no renderer change. S2 pins
it with tests.

### Slice S1: the field (0.5 day)

**Files:** modify `src/slice.rs` (type, field, `Default`, tests in its `#[cfg(test)] mod tests`); every
`SliceFinding {` literal: 27 production (contract 12, absence 4, taint 3, callback dispatcher 2; primitive,
provenance, echo, symmetry, membrane, peer consistency 1 each) and about 23 test literals
(`rg -n "SliceFinding \{" -g '*.rs'`), plus any the compiler reports; `CLAUDE.md` (`slice.rs` bullet).

```rust
// src/slice.rs
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleRef { pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cwe: Option<String>,
    /// The rule's own severity, only when lower than the finding's (DESIGN 5.2).
    #[serde(default, skip_serializing_if = "Option::is_none")] pub severity: Option<String> }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]   // `Default` added
pub struct SliceFinding { /* existing fields unchanged, then, last: */
    /// Rules this finding is an instance of, sorted by id, at most one per id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub rules: Vec<RuleRef> }
```

- [ ] Tests first: `finding_without_rules_serializes_without_the_key`, `rules_serialize_as_id_and_optional_cwe`
      (`{"id":"legacy"}` has no `cwe` and no `severity`), `missing_rules_key_deserializes_to_empty`,
      `default_finding_has_empty_rules`. RED: `cargo test -p prism --lib slice::tests` does not compile.
      Then the type and field, `rules: Vec::new(),` in every literal (one explicit line each),
      `cargo build --workspace --all-targets`: GREEN.
- [ ] Gates: section 5 items 1, 2, 4, 8, 9, 10; G-additive and G-targets (`compare.py` with no scope: every
      output byte-equal after masking the per-build values).
      One reviewer (Opus-5.5 xhigh): no behaviour change, proven by the gate. PR body: literals per file, gate
      summary, why `contract_slice.rs` stays over 1,000, #841 state.

### Slice S2: taint emission (3 days)

**Files:** create `crates/prism-review/src/algorithms/taint/sink_rules.rs` (declared in
`crates/prism-review/src/algorithms/taint.rs`: `mod sink_rules;`) and
`crates/prism-review/tests/cli/rule_identity_output_test.rs` (`mod` line in
`crates/prism-review/tests/cli/main.rs`); modify `taint/run.rs`, `taint/sink_match.rs`,
`taint/{python,js_ts,go}/sinks.rs`, `taint_sinks/{mod,structured,tests}.rs` and any `taint_sinks/*.rs` file
holding a value stage D corrects, the support, identity and TSV files of stage T,
`tests/algo/taxonomy/taint_sink_strength_test.rs` (`assert_severities`),
`tests/fixtures/review_no_diagrams/golden/review_unflagged.json`, `CLAUDE.md` (`taint_sinks/` paragraph).

**Implement DESIGN 5.2 and Appendices A, B, E and F.** A detector's admission tag and strength
still drive today's finding. Public metadata is computed separately per reached operand. This slice
does not require the S3 registry to exist.

- In `taint/sink_match.rs` and the three language matchers expose attribution evidence for every
  eligible (pattern, call/JSX attribute, operand span, semantic role) with its path or inline source.
  Preserve the existing outcome function for admission, including an early special
  `SemanticallyExcluded` and Go's call-start filter. A vector's first successful element alone cannot
  reproduce every old outcome. Use the original pattern category and the per-call cleanser on each
  candidate. Check both arms of a duplicate pattern and both operands of a multi-slot pattern.
- In `taint/sink_rules.rs` collect flat occurrences from edge target bytes and the explicit inline
  witnesses. Handle named arguments, JSX, nested calls, both path positions, assignment RHS and the
  `Path(x)` and TypeORM consuming chains. Enforce A's operand roles and 5.2's binding, source-line,
  safe-range and statement boundaries. Withhold an ambiguous occurrence rather than borrowing another
  use's bytes. Keep configured predicates and detected wrapper predicates separate for attribution.
- `structured_rule` takes the matched pattern, parsed occurrence, reached operand/role and binding or
  matching flat evidence for **that call**. It returns the public reference/strength for that role or
  no contribution. Use B's explicit dispatch, including JS code execution, Mongoose, SQLite and both
  Flask roles. A per-role optional result permits several roles of the same call to survive. The same
  occurrence can supply a neutral F1/F3 id through both detectors.
- `run.rs` collects candidates independently of insertion order. After ordinary edge, YAML, Flask and Go
  source-line insertions, emit rules only on the resulting physical keys. Go unions every uncleansed
  originating path with every valid inline witness, including the no-path and secondary fallbacks. A
  framework source line may specifically exclude the flat half. Preserve scope and the original line
  severity, descriptions, evidence alignment and order. Attach `configured-sink` and `format-wrapper`
  only to reached matching callees, `legacy` only for an eligible built-in line hit. Leave source and
  unquoted-expansion rules empty. Use `js-proto-write@CWE-1321` on lane 4's side-set lines if merged.
- Aggregate strengths by public id across attached hits, strong winning over weak. Sort by id. Set
  `severity = Some("suggestion")` only on an all-weak id in a warning. Keep the original finding's
  `line_strength`, including the line-only vetoes. Inspect the actual finding severity in tests.

- [ ] First commit, RED. Transcribe Appendix E into the TSV in the matrix implementation commit if
      it has not already landed. Use table plus exact label, preserve all 540 labels and the historical
      meaning of `observed` (expand an old `=` to its old concrete value when changing `rules`).
      Replay changed cases on the exact base. Separately assert **presence** on every marked line and
      ids/CWEs/optional severity on every positive, including positive `-` rows. Reject a residual
      `?`, missing positive or extra negative. Retain `#[track_caller]` in the matrix helpers.
      `cargo test --test algo_taxonomy taint_sink_rules` is RED on the S1 base for nonempty expected ids.
- [ ] Add both orders and a negative for sibling and continued placements, same-name binding collisions,
      safe and live nested calls, Go's start-line limit, source-only vs incoming vs mixed paths and the
      byte/inline source exception. Cover TypeORM and pathlib receiver operands, role swaps in A/B,
      SQLModel exec, typed/opaque receivers, Flask source vs escaped/unsafe context, JS VM, Mongoose,
      browser-only versus server-capable requests and the shell/ordinary argv split in syscall.Exec.
      Check weak JS/Go/Python SQL, Python clause, sandbox and Gin both alone and on a warning, plus
      same-id weak+strong and a weak structured SQL role. Include ri01–ri27 and both dynamic name
      origins on one call, clean siblings, `=name` and substring negatives. For #841 if merged, test
      overlapping and expanded-only findings. Preserve baseline presence even on a design-predicted
      empty/rejected case.
- [ ] Before adopting the refusals, run the paired projection and per-reason census in Appendix F
      on the pinned inputs. Use the 300 labelled E candidates to choose regression witnesses. Check
      candidate admission on the base before adding assertions, and disclose any unbuilt/unexecuted rows
      or grid cells. Implement and run the affected suites GREEN. Classify each disagreement as
      expectation, classifier/projection or pre-existing admission, with the base control and disposition.
      Use the filed F1–F5 entries for neutral rows.

- [ ] `rule_identity_output_test.rs`: `json_taint_sink_carries_rules`, `review_taint_sink_carries_rules`
      (single and multi run, per-result and aggregate copies, with and without diagrams, retained and
      floor-filtered findings), `findings_without_rules_have_no_rules_key` (echo and contract on
      `tests/fixtures/targets`, recursive key check).
- [ ] The `review_no_diagrams` golden gains `rules` on both copies of its one `taint_sink` finding, re-recorded
      once with the reason in the commit (DESIGN decision 7); no other existing expectation changes.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-additive, G-fingerprint, G-targets (`--new-rules
      finding:taint`), G-coverage. Two reviewers. PR body: mismatch classification, the Appendix F census, including the `taint_sink` denominator, unadmitted candidates, admitted
      empties and CWE-less occurrences by reason,
      golden diff, gate summaries, #841 state, lane 4 state.

### Slice S3: registry, closed producers and generated page (2 days)

**Files:** create `crates/prism-review/src/output/rule_registry.rs` (declared in `output/mod.rs`:
`pub mod rule_registry;`), `crates/prism-review/src/output/rule_registry/taint_rows.rs` and
`.../rule_registry/tests.rs` (declared in `rule_registry.rs`), `docs/rules.md` (generated); modify
`taint_sinks/{mod,structured,js,js_modules,python,py_modules,go,legacy,tests,tests_js_modules,tests_py_modules}.rs`
(every published role selects a registry constant), `taint/sink_rules.rs`, the stage T support file (`check_emitted` on
every emitted reference), `scripts/ci/classify-changes.sh` (`docs/rules.md) code=true ;;` before `docs/*`,
since a test reads it) and `scripts/ci/classify-changes.test.sh` (a `check true true` case), `CLAUDE.md`
(`output/` bullet).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleInfo { pub id: &'static str, pub cwe: Option<&'static str>, pub name: &'static str,
    pub short: &'static str, pub help: &'static str, pub algorithms: &'static [&'static str] }
impl RuleInfo { pub fn rule_ref(&'static self) -> prism::slice::RuleRef; }   // the only RuleRef maker
// rule_registry/taint_rows.rs: one `pub const <NAME>: RuleInfo` per rule, and `TAINT_ROWS: &[&RuleInfo]`
/// All rules sorted by id: one table assembled once (`OnceLock`) from the per-algorithm row files.
pub fn rules() -> &'static [&'static RuleInfo];
pub fn lookup(id: &str) -> Option<&'static RuleInfo>;
/// DESIGN 5.3: Err for an unknown id, a CWE other than the row's, an algorithm the row does not list.
pub fn check_emitted(algorithm: &str, rule: &prism::slice::RuleRef) -> Result<(), String>;
pub fn render_markdown() -> String;   // the text of docs/rules.md
// The occurrence projection selects Option<&'static RuleInfo> per matched role.
// Private classification evidence and Strength remain available for presence and line grading.
```

Register every distinct **published id** of DESIGN A/B once, including neutral rows, six structured
category defaults and their code/template exceptions, `configured-sink`, `format-wrapper` and `legacy`.
Register `js-proto-write` only when its emitter lands. Former internal tag/CWE combinations in the
observation column are not registry entries. Several detectors can select the same constants
(e.g. the bound-value role). Verified and unresolved cases must never produce two CWEs for one id.
Use `name = id`, the short/inspection-help conventions and the exact legacy strings in DESIGN 5.3.
Generate the page in id order with id, optional CWE, name, algorithms, short and help, after a
"generated, do not edit" line naming the regeneration command.

- [ ] Tests first (`cargo test -p prism-review --lib output::rule_registry`, RED): `ids_are_unique_and_sorted`,
      `cwe_strings_are_well_formed`, `rows_have_text_and_known_algorithms`,
      `every_row_constant_is_in_its_table` (the `pub const .. : RuleInfo` names of each row file against its
      table), `no_production_source_builds_a_rule_ref_by_hand` (scan of every non-test `.rs` file under
      `crates/prism-review/src/` and `src/`, the registry excepted: no `RuleRef {` or `RuleRef::`, no
      `SinkHit {` outside `taint_sinks/mod.rs`), `scanner_flags_a_hand_built_rule_ref` (the synthetic
      `RuleRef { id: format!("js-{}", k), cwe: None, severity: None }` is flagged, `JS_EVAL.rule_ref()` is
      not), `check_rejects_unknown_id`, `check_rejects_cwe_that_disagrees_with_row`,
      `check_rejects_algorithm_not_listed`, `check_accepts_a_registry_ref`,
      `structured_ids_are_rows_with_their_cwe` (all 107 built-in and framework entries, eligible slots
      and exceptions), `legacy_text_matches_design`, `neutral_rows_never_gain_a_cwe`, `rules_page_is_current` (`PRISM_BLESS_RULES=1` rewrites it).
      The stage T helpers call `check_emitted("taint", ..)` on every emitted reference of every positive row.
- [ ] Implement, bless once, GREEN; the S2 rows stay GREEN unchanged; `bash scripts/ci/classify-changes.test.sh`
      passes.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-closed (registry part); G-additive with no scope (the
      producer change must not move a byte). One reviewer (Opus-5.5 xhigh), who checks each row against stage
      D's tables.

### Slice S4: SARIF (2.5 days)

**Files:** create `crates/prism-review/tests/cli/sarif_rule_ids_test.rs` (`mod` line in `tests/cli/main.rs`);
modify `output/{sarif,sarif_model,sarif_rules}.rs`, `crates/prism-review/src/api/mod.rs` (re-export, doc),
`cli.rs` (`ReviewArgs`), `main.rs` (both `SarifInputs` chains), `crates/prism-review/tests/cli/sarif_shape_test.rs`
(mode parameter), `README.md` (SARIF section, with a `--sarif-rule-ids specific` example that `readme_test`
parses, and the unverified GitHub points of DESIGN section 2), `CLAUDE.md` (`output/` bullet).

```rust
// output/sarif.rs
#[non_exhaustive]   // reachable through prism_review::api
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum SarifRuleIds { #[default] Category, Specific }
pub struct SarifInputs<'a> { /* … */ pub rule_ids: SarifRuleIds }   // `new` sets Category
impl<'a> SarifInputs<'a> { pub fn rule_ids(mut self, rule_ids: SarifRuleIds) -> Self; }
// output/sarif_rules.rs: ["external/cwe/cwe-<n>"] for Some("CWE-<n>"), empty otherwise
pub fn cwe_tags(cwe: Option<&str>) -> Vec<String>;
// cli.rs, ReviewArgs. Doc (the help text): "--format sarif only: `category` (default) keeps
// prism/<algorithm>/<category>; `specific` writes one result per rule as prism/<algorithm>/<rule id>, with
// CWE tags. Switching modes can start new code-scanning alert histories for the affected findings; how
// GitHub shows the CWE tags is unverified."
#[arg(long, value_enum, default_value_t = SarifRuleIds::Category)] pub sarif_rule_ids: SarifRuleIds,
```

Model: `ResultProperties.rules: Vec<RuleRef>`, `Rule.help: Option<Text>`, `RuleProperties.category:
Option<String>`, `RuleProperties.tags: Vec<String>`, each skipped when empty or `None`; default mode fills
them as today, so its bytes do not move except for `properties.rules`. Other formats ignore the flag. An empty admitted finding keeps its default descriptor in both modes.
The named neutral roles (including `configured-sink` and `format-wrapper`) do have specific-mode rule
ids and registry descriptions. They have no CWE tag. Never infer a tag from the old detector, a sibling
or the public id. Carry the full sorted references on each expanded result, including a weak id's
`suggestion` only when it is lower than the finding severity.
`specific` mode follows DESIGN 5.5: after admission (and after #841's per-finding map and diagnostics
choices, if merged), a finding's references are grouped by id and the finding becomes one result per
distinct id, each a copy of the default result (message, locations, fingerprint, evidence computed once,
properties with the full `rules`) with `ruleId` `prism/{algorithm}/{rule id}` and the level of the
reference's `severity` when present. Registered descriptor: `name`, `shortDescription` = `short`,
`fullDescription` and `help` = `help`, `properties { algorithm, tags: cwe_tags(row cwe) }`, no `category`
(a rule can span categories); it wins over any other descriptor of the same id, whatever the input order.
Unregistered: `name` = id, both descriptions `"{algorithm}: {id}"`, no help, tags from the sorted distinct
well-formed CWEs supplied for that id in the run. Findings without rules are written as today. Results are
sorted after expansion and `ruleIndex` assigned after both sorts; the module doc's "one result per finding"
becomes "in default mode", and its consumer key gains `ruleId`.

- [ ] Tests first (in-process through `prism_review::api` with `SliceFinding { rules: …, ..Default::default() }`,
      plus CLI runs): review-focus tests 4 and 5; `default_mode_without_rules_has_no_rules_key`,
      `specific_mode_writes_one_result_per_rule_sharing_fingerprint`,
      `specific_mode_descriptor_has_registry_text_and_cwe_tag`, `specific_mode_rule_without_cwe_has_no_tags`
      (use named neutral output, wrapper and configuration rows, including a mixed-CWE finding),
      `specific_mode_finding_without_rules_keeps_category_rule`, `specific_mode_every_cwe_descriptor_has_its_tag`
      (CLI on identity fixtures), `specific_mode_weak_rule_result_is_note`,
      `specific_mode_repeated_id_gives_one_result`, `specific_mode_conflicting_cwes_for_one_id_give_one_result`,
      `specific_mode_unknown_rule_keeps_supplied_cwe_tag`, `specific_mode_unknown_rule_without_cwe_has_no_tags`,
      `specific_mode_two_unknown_inputs_with_different_cwes_tag_both`,
      `specific_mode_registry_wins_over_disagreeing_cwe`,
      `specific_mode_mixed_primitive_descriptor_is_order_independent`, `specific_mode_is_byte_deterministic`,
      `sarif_rule_ids_flag_rejects_unknown_values`, `sarif_rule_ids_help_mentions_alert_history`. RED:
      `cargo test -p prism-review --test cli sarif_rule_ids_test::`.
- [ ] `sarif_shape_test.rs`: `run_sarif` gains a mode argument; every assertion that does not name a
      category-mode rule id runs in both modes (the category-id literals stay default-mode only).
- [ ] Implement; GREEN; `sarif_test` passes unchanged.
- [ ] G-sarif-valid as far as S4 can: the shape tests in both modes, and, if `python3 -c 'import jsonschema'`
      works and a SARIF 2.1.0 schema copy is in the evidence directory (record its SHA-256), validation of a
      mixed `specific`-mode log with non-empty results; otherwise "schema not validated" and the assertions
      that ran, in the PR body. S5 repeats it on the final head.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-fingerprint (default mode), G-additive (`--new-rules
      sarif:taint`), G-closed (SARIF part), G-sarif-valid. Two reviewers.

### Slice S5: primitive and absence (1.5 days)

**Files:** create `crates/prism-review/src/output/rule_registry/{primitive_rows,absence_rows}.rs` (declared
in `rule_registry.rs`), `crates/prism-review/src/algorithms/absence_slice/pairs.rs` (declared in
`absence_slice.rs`: `mod pairs; pub use pairs::{default_pairs, PairedPattern};`),
`tests/algo/novel/rule_identity_test.rs` (`mod` line in `tests/algo/novel/main.rs`); modify
`primitive_slice.rs`, `absence_slice.rs`, `rule_registry/tests.rs`, `docs/rules.md`, `CLAUDE.md`.

- [ ] First commit, move-only: `PairedPattern` and `default_pairs()` into `absence_slice/pairs.rs`, text
      unchanged; `targets_mapping_test` green. This keeps `absence_slice.rs` well under 1,000.
- [ ] Tests first. `primitive_findings_carry_their_rule_and_cwe` covers all six categories in
      DESIGN C: direct and called digest truncation and weak identity hash /328, shell interpolation
      /78, disabled certificate validation /295, hardcoded credential /798. Keep the verbatim category
      and test both colliding category/specific descriptors. `every_absence_pair_id_is_a_unique_row`
      compares the entire 65-member description/id/optional-CWE inventory in D and checks cross-algorithm
      uniqueness. Assert one missing-counterpart result with a CWE, one without, and both error-path
      categories. Use the goto inputs of `tests/lang/c/cve_test.rs` plus repeated unlock and generic
      close controls. For `double_close` assert `absence-double-free@CWE-415`,
      `absence-double-unlock@CWE-765` and `absence-repeated-counterpart` without a CWE. Include different
      operands, substring-only names and an intervening reset/reacquisition as controls for specialization.
      Other algorithms (contract, echo, membrane, symmetry, provenance, peer consistency, callback
      dispatcher) have empty rules. RED: `cargo test --test algo_novel rule_identity_test::`.
- [ ] Primitive: each emitting site passes its Appendix C constant. `make_finding` takes the immutable
      row, preserves the category and adds its one reference. Absence: use each Appendix D constant
      directly from its `PairedPattern` for the three missing categories. For a repeated operation,
      check both concrete release sites, exact callee and resource operand on the goto path before
      selecting the free/unlock constant. Fall back to the generic repeated-counterpart constant
      while retaining the finding. Do not construct an id from a description or add the missing-release
      id to a repeated result. These references have no per-rule severity override.
- [ ] Run the Appendix F measurement for every primitive and absence category on the pinned inputs,
      showing which pairs/close roles actually occur, with zeros explicit. Compare the original
      and projected category/severity/description multisets. Re-bless the generated rules page and
      run the affected checks GREEN.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-additive and G-fingerprint (`--new-rules finding:primitive
      finding:absence sarif:primitive sarif:absence`), G-targets, G-closed, and G-sarif-valid on this head
      (the last producer slice): the shape tests in both modes on logs with non-empty taint, primitive and
      absence results, and the schema step of S4. Two reviewers.

## Close-out (0.5 day)

- [ ] The `CLAUDE.md` paragraphs the slices added read as one account; `docs/rules.md` is current.
- [ ] The cumulative comparison against the pre-lane base (stage M), with every scope, in the close-out note.
- [ ] Follow-up issues, each with its evidence: the CWE-less entries F1–F7 in DESIGN Appendix F; the main defects of DESIGN
      section 8 (presence decided by the first structured match; Python call-level cleansing on continued
      calls); the GitHub check of DESIGN section 2, for someone with upload rights; the `security` tag and
      `security-severity` (decision 5); rules for a targets consumer (decision 6); more algorithms
      (decision 4).
- [ ] Ledger and `HANDOFF.md` refreshed; outputs summarised and offloaded; `target/` deleted; issue closed.

## Acceptance for the lane

Every slice passed its gates against its merge base (G-additive, G-fingerprint, G-targets where listed);
every positive row of the sink-rule and identity tables asserts the rule ids and CWEs stage D fixed
(G-coverage), and no published rule carries an identity D recorded as wrong; the registry tests pass,
including their failing controls, and `docs/rules.md` is current (G-closed); on the final head the shape
tests pass in both modes (and the schema, or "schema not validated" with the assertions that ran); both
reviewers approved S2, S4 and S5.

**Estimate:** M 1, T 2.5, E 1, D 1.5, S1 0.5 (during T), S2 3, S3 2, S4 2.5, S5 1.5, close-out 0.5:
16 days of work, 15.5 on the critical path.

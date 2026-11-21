# Lane 7: rule identity on findings — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a finding produced by a rule says which rule and, where one exists, which CWE, in JSON, review and
SARIF output, with no change to finding counts, target ids, SARIF fingerprints or default SARIF rule ids.

**Architecture:** `SliceFinding` gains `rules: Vec<RuleRef>` (core crate, skipped when empty). The taint sink
loop records the rule of every flat hit and structured match per sink line and attaches the set to the
`taint_sink` finding. One registry (`output/rule_registry.rs`) lists every rule with its CWE and text, is
proven closed by tests and generates `docs/rules.md`. SARIF always adds `properties.rules`;
`--sarif-rule-ids specific` writes one result per rule with registry descriptors and CWE tags. Primitive and
absence findings carry their rule last.

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
- PR #841 (open) edits `taint/run.rs`, `cli.rs`, `main.rs`, `output/mod.rs` and adds test literals. Before
  each slice: if merged, rebase and re-read the touched code; if open, comment once after S1 merges (its new
  literals need `rules: Vec::new()`). Its result union keeps the baseline finding per `(category, file,
  line)` verbatim, so a line found by both of its runs keeps the baseline run's rules. Accepted.
- Lane 4's sink slice starts after S2 merges and reports `js-proto-write`/`CWE-1321` through this field, not
  the description (DESIGN 5.2; lane 4 decision 9). If S3 has merged, it also adds a registry row.
- Weighted sizes now: `taint/run.rs` 601.5, `absence_slice.rs` 906, `primitive_slice.rs` 794,
  `contract_slice.rs` 1447, `main.rs` 990.5, `cli.rs` 483.5, `src/slice.rs` 572, `output/sarif.rs` 380.5,
  `sarif_rules.rs` 436.5. Only `contract_slice.rs` stays over 1,000 (12 one-line additions in S1; its PR
  says so); new logic goes in the new modules named below.
- Briefs say: no `mutgate`; every `cargo` and `prism` run through `~/prism-evidence/tools/memguard.sh 8`.

## Review focus

1. Two flat rules on one line (`res.send(eval(x))`): one finding, `rules` = `js-eval`, `js-response`; two
   `specific`-mode results sharing a fingerprint. Row `ri01` (S2); `specific_mode_writes_one_result_per_rule_sharing_fingerprint` (S4).
2. A cleansed structured call beside an unrelated flat sink on one line: neither the structured rule nor the
   rules of flat identifiers inside the cleansed call appear; the other sink's rule does. Row `ri15` (S2).
3. Lines inserted outside the per-edge loop (Python inline Flask source, Go source equal to sink, JS YAML
   inline source) carry their structured rule; a line inserted only by a format-string wrapper carries what
   stage D decides. Rows `ri11` to `ri14` (S2).
4. Default-mode SARIF for findings that carry rules: no new descriptor or descriptor key, `mapping_version`
   still `"1"`, only `results[].properties.rules`. `default_mode_adds_properties_rules_and_keeps_rule_ids` (S4).
5. `specific` mode with admission and indexes: a multi-rule finding removed by `--min-confidence exact`
   writes nothing; each expanded result's `ruleIndex` names its own descriptor; an unregistered rule id still
   gets a descriptor. `specific_mode_respects_min_confidence`, `specific_mode_rule_index_matches_descriptor`,
   `specific_mode_unknown_rule_id_gets_fallback_descriptor` (S4).

## Stage M: issue and output baseline (0.5 day)

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
- [ ] `m/run_outputs.sh <prism binary> <out dir>`: per input, with `ALGOS=taint,primitive,absence,contract,
      echo,membrane,symmetry,provenance,peer_consistency,callback_dispatcher` (every finding emitter, no
      git-history algorithm), write `json`, `review` (`--review-min-severity info`), `sarif`, the single-run
      `json-taint` and `sarif-taint` (`--algorithm taint`), and `targets` (`prism targets`). A failing input
      goes to `<out dir>/FAILED` with its stderr.
- [ ] `m/compare.py <base out> <head out>`, one line per output, non-zero exit on any difference:
      json and review: assert the base has no finding-level `rules`; delete `rules` from every head object
      that has both `algorithm` and `severity`; compare `json.dumps(indent=2)`; print whether raw bytes are
      equal (required when nothing was deleted). sarif: delete `runs[].properties.{prism_git_sha,
      prism_build_identity, binary_input_dirty}` from both and `runs[].results[].properties.rules` from the
      head; compare dumps; compare the sorted `partialFingerprints["prism/finding/v1"]` lists. targets: delete
      `producer.build_identity`; compare dumps and the `targets[].id` lists.
- [ ] Control: main's binary run twice; `compare.py` shows no difference. `m/BASELINE.md` records finding
      counts per input and algorithm, so an input with no finding of an algorithm is visible.

**Gate procedure (S1, S2, S4, S5):** in the lane worktree, build the merge base (detached) and the head
through memguard, copying each `target/release/prism` to `bin/prism-<sha>`; `run_outputs.sh` both;
`compare.py`; paste its summary in the PR body.

## Stage T and E: the matrix (2 days, then 1 day)

The sink-rule tables (`tests/algo/taxonomy/taint_sink_rules_*_test.rs`, about 350 positive rows) already have
one rule per row. Stage T adds an expectations file keyed by table and label, and a table of identity rows.
Nothing in stage T names `SliceFinding.rules`, so the branch compiles before S1.

### Task T-1: expectations and identity rows

**Files:** create `tests/fixtures/taint_sink_rules/expected_rules.tsv`,
`tests/algo/taxonomy/taint_sink_rules_identity_test.rs`, `tests/algo/taxonomy/taint_sink_rules_expect_test.rs`;
modify `tests/algo/taxonomy/main.rs` (two `mod` lines), `tests/algo/taxonomy/taint_sink_rules_support.rs`.

TSV columns: `table` (test file stem, e.g. `taint_sink_rules_js_test`), `label` (the row's `label`, exactly),
`rules` (`;`-separated, sorted by id, each `id` or `id@CWE-n`; `-` empty; `?` fixed by stage D), `class`,
`note`. One line per positive row (`assert_all_sinks` cases, `sink: true` rows) of the eight tables and the
identity table; a row run under JS and TS has one line. Classes: `flat`, `flat-multi`, `flat-dup`,
`structured`, `flat+structured`, `cleansed-structured`, `legacy`, `source-line`, `yaml-inline`, `wrapper`,
`weak-mixed`.

```rust
// taint_sink_rules_support.rs (no field access)
pub struct Expectation { pub table: String, pub label: String,
    pub rules: Option<Vec<String>> /* None = "?" */, pub class: String }
/// Rejects a bad column count, an unsorted or duplicated rule list, an id outside `[a-z0-9-]+`,
/// a CWE outside `CWE-<digits>`, a duplicate (table, label).
pub fn parse_expected_rules(tsv: &str) -> Result<Vec<Expectation>, String>;
```

`taint_sink_rules_expect_test.rs`: `expected_rules_tsv_is_well_formed`,
`every_expectation_names_exactly_one_row` (`label: "<label>"` occurs exactly once in
`tests/algo/taxonomy/<table>.rs`), `parser_rejects_unsorted_rules`, `parser_rejects_bad_cwe`.
`taint_sink_rules_identity_test.rs`: `const IDENTITY_ROWS: &[Row]` and `identity_rows_fire_as_marked`
(`assert_rows`), green on main (sink presence only until S2). Seed rows:

| Row | Line | Expected rules |
|---|---|---|
| ri01 (ri19: TSX) | JS `res.send(eval(x));` | `js-eval@CWE-95;js-response@CWE-79` |
| ri02 | JS `el.innerHTML = x; cp.exec(x);` (`cp` from `child_process`) | `js-child-process@CWE-78;js-dom-html@CWE-79` |
| ri03, ri04 | JS `fetch(x)`; Python `pickle.loads(x)` | structured SSRF; structured deserialization (`?`) |
| ri05, ri06 | Go `os.ReadFile(x)`; Python `cur.execute("SELECT " + x)` | flat plus structured hits, from the tables (`?` for structured) |
| ri07 | JS `res.send(x); res.send(x);` | `js-response@CWE-79` once |
| ri08 | Python `cur.execute("SELECT 1"); os.system(x)` | `py-db-call@CWE-89;py-os-command@CWE-78` |
| ri09, ri10, ri16–ri18 | C `system(x)`; Bash `eval "$x"`; one Java, Rust, Lua sink | `legacy` |
| ri11, ri12, ri13 | Go gin `c.File(c.Param("f"))`; Python Flask sink with inline `request.args[..]`; JS `yaml.load(req.body)` | each its structured rule (`?`) |
| ri14 | C variadic wrapper forwarding to `vprintf`, called with the tainted value | `?` |
| ri15 | Go: a cleansed structured call and an unrelated flat sink on one line | the flat sink's rule only |

- [ ] Write the rows; one that does not fire on main is reshaped until it fires or dropped (ledger note).
- [ ] Fill `rules` for every positive row by reading the rule code (`taint_sinks/{js,js_modules,python,
      py_modules,go,legacy}.rs`: tags are chosen where `SinkHit::new` or a local `hit(..)` is called). A
      never-committed patch printing each row's hits may draft values; each value is then checked by reading.
      Uncertain rows get `?` and a note.
- [ ] Gates: `cargo fmt --all --check`; `cargo test --test algo_taxonomy taint_sink_rules` at
      `RAYON_NUM_THREADS` 1 and 4. One reviewer (test-only).

### Task E-1: enumeration

- [ ] Run the tiers (ORCHESTRATION section 6) on the identity table. Axes: hits per line (none, one, several,
      one rule twice); flat and structured mixes; cleansed structured calls; the non-edge insertion branches
      of `taint/run.rs` (YAML inline, Python inline Flask, Go source line with and without an originating
      path, unquoted expansion); framework-source lines; weak SQL beside strong sinks; caller-supplied and
      wrapper sinks; each language without positional rules; TS and TSX; several paths reaching one line
      through different identifiers. Accepted rows go into both files.

## Stage D: design of record (1 day)

- [ ] Inputs: the TSV and identity rows; the 34 flat tags with their CWEs (research section 2) and `legacy`;
      the six `SanitizerCategory` values; the six primitive categories with their `rule_description`
      sentences (`output/sarif_rules.rs`); the 66 `PairedPattern` descriptions (`default_pairs`) and the four
      absence categories each can be emitted under. The design seat amends DESIGN 5.2 to 5.4 and fixes:
      (1) the six structured rule ids (DESIGN proposes `structured-<category>`) and CWEs (decision 3);
      (2) what a line inserted only by a caller-supplied (`TaintConfig.extra_sinks`) or format-string wrapper
      sink carries: nothing or a named rule (`ri14`); (3) absence: each pair's id (slug rule, unique across
      algorithms) and CWE or none, and what a `double_close` finding carries (DESIGN lists "double close" as a
      CWE class, but rules are per pair and `double_close` is emitted for any pair); (4) the CWE of each of
      the six primitive categories (DESIGN names five classes; two categories are hash truncation);
      (5) conventions for registry `name`, `short`, `help`, and the `legacy` row's text. Output: amended
      sections with appendix tables (structured, absence, primitive). The implementer replaces every `?` in
      the TSV in the same commit. Both reviewers; cap 2 rounds.

## Stage I: slices

S1 may run during T and E (it needs no rule names). S2 to S5 follow D in this order:

| Slice | Why here |
|---|---|
| S1 field | every later slice writes or reads it; a behaviour-free PR merges fast and narrows the #841 window |
| S2 taint emission | the main value and lane 4's prerequisite; needs only D's structured ids |
| S3 registry | SARIF descriptors and the primitive/absence CWEs read it; its closure tests pin S2's tags |
| S4 SARIF | needs the registry; serves whatever carries rules |
| S5 primitive, absence | DESIGN's second slice and the smallest value; needs the registry's CWEs |

DESIGN 5.6 needs no slice: `--format review` and `--format json` both serialise `SliceFinding` (no format
prints a finding as a text line), so the field appears with no renderer change. S2 pins it with tests.

### Slice S1: the field (0.5 day)

**Files:** modify `src/slice.rs` (type, field, `Default`, tests in its `#[cfg(test)] mod tests`); every
`SliceFinding {` literal: 27 production (contract 12, absence 4, taint 3, callback dispatcher 2; primitive,
provenance, echo, symmetry, membrane, peer consistency 1 each) and 13 test literals
(`rg -n "SliceFinding \{" -g '*.rs'`), plus any the compiler reports; `CLAUDE.md` (`slice.rs` bullet).

```rust
// src/slice.rs
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleRef { pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cwe: Option<String> }
#[derive(Debug, Clone, Default, Serialize, Deserialize)]   // `Default` added
pub struct SliceFinding { /* existing fields unchanged, then, last: */
    /// Rules this finding is an instance of, sorted by id, no duplicates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub rules: Vec<RuleRef> }
```

- [ ] Tests first: `finding_without_rules_serializes_without_the_key`, `rules_serialize_as_id_and_optional_cwe`
      (`{"id":"legacy"}` has no `cwe`), `missing_rules_key_deserializes_to_empty`,
      `default_finding_has_empty_rules`. RED: `cargo test -p prism --lib slice::tests` does not compile.
      Then the type and field, `rules: Vec::new(),` in every literal (one explicit line each),
      `cargo build --workspace --all-targets`: GREEN.
- [ ] Gates: section 5 items 1, 2, 4, 8, 9, 10; G-additive and G-targets (raw bytes equal for every output).
      One reviewer (Opus-5.5 xhigh): no behaviour change, proven by the gate. PR body: literals per file, gate
      summary, why `contract_slice.rs` stays over 1,000, #841 state.

### Slice S2: taint emission (2 days)

**Files:** create `crates/prism-review/src/algorithms/taint/sink_rules.rs` (declared in
`crates/prism-review/src/algorithms/taint.rs`: `mod sink_rules;`) and
`crates/prism-review/tests/cli/rule_identity_output_test.rs` (`mod` line in
`crates/prism-review/tests/cli/main.rs`); modify `taint/run.rs`, `taint_sinks/{mod,structured,tests}.rs`, the
support, identity and TSV files of stage T, `tests/fixtures/review_no_diagrams/golden/review_unflagged.json`,
`CLAUDE.md` (`taint_sinks/` paragraph).

```rust
// taint_sinks/mod.rs: drop the "no slot" doc sentence and `#[cfg_attr(not(test), allow(dead_code))]`
impl SinkHit { pub(crate) fn rule_ref(&self) -> prism::slice::RuleRef; }
pub(crate) use structured::{structured_rule, STRUCTURED_CATEGORIES};
// taint_sinks/structured.rs
pub(crate) const STRUCTURED_CATEGORIES: [SanitizerCategory; 6];
/// Stage D's id and the category's CWE; an exhaustive match.
pub(crate) fn structured_rule(category: SanitizerCategory) -> prism::slice::RuleRef;
// taint/sink_rules.rs: a BTreeMap<(String, usize), BTreeSet<RuleRef>> behind three methods
#[derive(Default)] pub(super) struct SinkRules { /* … */ }
impl SinkRules {
    pub(super) fn add(&mut self, sink: &(String, usize), rule: RuleRef);
    pub(super) fn add_structured(&mut self, sink: &(String, usize), category: SanitizerCategory);
    pub(super) fn take(&mut self, sink: &(String, usize)) -> Vec<RuleRef>; // sorted, deduplicated
}
```

`run.rs` keeps one `SinkRules` beside `sink_lines` and records only where a line is inserted:
per-identifier branch (`:256`): keep `classify`'s `Option<SinkHit>` and `add` its `rule_ref()` at the
insertion (an identifier admitted only by `extra_sinks` adds what stage D says); structured branch (`:298`):
`add_structured(p.category)` for the inserted `Match(p)`; YAML inline lines (`:328`):
`add_structured(Deserialization)` (that helper scans only `JS_CWE502_SINKS`); Python inline Flask (`:355`)
and Go source lines (`:386`, `:434`): the category of the pattern that caused the insertion (for `:434` the
first uncleansed matching pattern, else the inline one); emission (`:577`): `rules: sink_rules.take(..)`.
`unquoted_expansion` and `taint_source` keep `Vec::new()`. Inserted lines, severity, category, description
and order do not change.

- [ ] First commit, RED: `assert_all_sinks` and `assert_rows` gain `#[track_caller]`, take the table from
      the stem of `std::panic::Location::caller().file()` and, for each positive row whose class is in
      `pub const RULE_CLASSES_ENABLED: &[&str]` (every class now), compare the SINK-line `taint_sink`
      finding's rules (as `id` or `id@CWE-n`) with the TSV; a missing or `?` entry fails naming table and
      label. In the identity file: `taint_source_and_unquoted_expansion_carry_no_rules` (a source line, and a
      Bash unquoted `$x` argument of a non-sink command) and `rules_are_sorted_and_unique`. In
      `taint_sinks/tests.rs`: `rule_ref_carries_tag_and_cwe`, `legacy_rule_ref_has_no_cwe`,
      `structured_rule_maps_every_category`. `cargo test --test algo_taxonomy taint_sink_rules` is RED.
- [ ] Implement; GREEN. Classify every RED-run mismatch in the PR body: TSV wrong (fix it, ledger note) or
      code tag wrong (the TSV keeps the code's value with note `wrong-tag #<issue>`; file the issue; #646 is
      one known case). No TSV value changes without a classification.
- [ ] `rule_identity_output_test.rs`: `json_taint_sink_carries_rules`, `review_taint_sink_carries_rules`
      (single and multi run), `findings_without_rules_have_no_rules_key` (echo and contract on
      `tests/fixtures/targets`, recursive key check).
- [ ] The `review_no_diagrams` golden gains `rules` on its two `taint_sink` findings; no other existing
      expectation changes.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-additive, G-fingerprint, G-targets, G-coverage. Two
      reviewers. PR body: mismatch classification, golden diff, gate summaries, #841 state.

### Slice S3: registry and generated page (1.5 days)

**Files:** create `crates/prism-review/src/output/rule_registry.rs` (declared in `output/mod.rs`:
`pub mod rule_registry;`), `crates/prism-review/src/output/rule_registry/taint_rows.rs` and
`.../rule_registry/tests.rs` (declared in `rule_registry.rs`), `docs/rules.md` (generated); modify
`scripts/ci/classify-changes.sh` (`docs/rules.md) code=true ;;` before `docs/*`, since a test reads it) and
`scripts/ci/classify-changes.test.sh` (a `check true true` case), `CLAUDE.md` (`output/` bullet).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleInfo { pub id: &'static str, pub cwe: Option<&'static str>, pub name: &'static str,
    pub short: &'static str, pub help: &'static str, pub algorithms: &'static [&'static str] }
/// All rules sorted by id: one table assembled once (`OnceLock`) from the per-algorithm row files.
pub fn rules() -> &'static [RuleInfo];
pub fn lookup(id: &str) -> Option<&'static RuleInfo>;
/// The row's CWE; `cwe: None` when `id` has no row.
pub fn rule_ref(id: &str) -> prism::slice::RuleRef;
pub fn render_markdown() -> String;   // the text of docs/rules.md
```

Rows: the 34 flat tags, `legacy`, the six structured ids, with stage D's text. The page opens with a
"generated, do not edit" line naming the regenerate command, then id, CWE, name, algorithms, short, help.

- [ ] Tests first (`cargo test -p prism-review --lib output::rule_registry`, RED): `ids_are_unique_and_sorted`,
      `cwe_strings_are_well_formed`, `rows_have_text_and_known_algorithms`, `every_taint_sink_tag_has_a_row`
      (hand scanner over every non-test `.rs` file in `src/algorithms/taint_sinks/`: string literals matching
      `(js|py|go)-[a-z0-9-]+`, plus `legacy`), `every_adjacent_tag_cwe_pair_matches_its_row` (a tag literal,
      `,`, a `"CWE-n"` literal; unpaired tags such as `go-html-render` are covered by G-coverage),
      `scanner_finds_tags_in_a_synthetic_source` (positive and negative strings),
      `structured_ids_are_rows_with_their_cwe`, `rules_page_is_current` (`PRISM_BLESS_RULES=1` rewrites it).
- [ ] Implement, bless once, GREEN; `bash scripts/ci/classify-changes.test.sh` passes.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-closed (registry part). No output changes, so no comparison
      run. One reviewer (Opus-5.5 xhigh), who checks each row against stage D's tables.

### Slice S4: SARIF (2 days)

**Files:** create `crates/prism-review/tests/cli/sarif_rule_ids_test.rs` (`mod` line in `tests/cli/main.rs`);
modify `output/{sarif,sarif_model,sarif_rules}.rs`, `crates/prism-review/src/api/mod.rs` (re-export, doc),
`cli.rs` (`ReviewArgs`), `main.rs` (both `SarifInputs` chains), `README.md` (SARIF section, with a
`--sarif-rule-ids specific` example that `readme_test` parses), `CLAUDE.md` (`output/` bullet).

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
// CWE tags. Switching modes starts new code-scanning alert histories for the affected findings."
#[arg(long, value_enum, default_value_t = SarifRuleIds::Category)] pub sarif_rule_ids: SarifRuleIds,
```

Model: `ResultProperties.rules: Vec<RuleRef>`, `Rule.help: Option<Text>`, `RuleProperties.category:
Option<String>`, `RuleProperties.tags: Vec<String>`, each skipped when empty or `None`; default mode fills
them as today, so its bytes do not move except for `properties.rules`. Other formats ignore the flag.
`specific` mode, after admission: a finding with rules becomes one result per rule, each a copy of the
default result (message, locations, fingerprint, properties with the full `rules`) with `ruleId`
`prism/{algorithm}/{rule id}`; descriptor: registry `name`, `shortDescription` = `short`, `fullDescription`
and `help` = `help`, `properties { algorithm, tags: cwe_tags(cwe) }`, no `category` (a rule can span
categories); an unregistered id gets `name` = id, both descriptions `"{algorithm}: {id}"`, no help or tags.
Findings without rules are written as today. Sorting and `ruleIndex` stay as they are; the module doc's
"one result per finding" becomes "in default mode".

- [ ] Tests first (in-process through `prism_review::api` with `SliceFinding { rules: …, ..Default::default() }`,
      plus CLI runs): review-focus tests 4 and 5; `default_mode_without_rules_has_no_rules_key`,
      `specific_mode_writes_one_result_per_rule_sharing_fingerprint`,
      `specific_mode_descriptor_has_registry_text_and_cwe_tag`, `specific_mode_rule_without_cwe_has_no_tags`,
      `specific_mode_finding_without_rules_keeps_category_rule`, `specific_mode_every_cwe_descriptor_has_its_tag`
      (CLI on identity fixtures), `specific_mode_is_byte_deterministic`,
      `sarif_rule_ids_flag_rejects_unknown_values`, `sarif_rule_ids_help_mentions_alert_history`. RED:
      `cargo test -p prism-review --test cli sarif_rule_ids_test::`.
- [ ] Implement; GREEN; `sarif_test` and `sarif_shape_test` pass unchanged.
- [ ] G-sarif-valid: the shape tests, and, if `python3 -c 'import jsonschema'` works and a SARIF 2.1.0 schema
      copy is in the evidence directory (record its SHA-256), validation of `specific`-mode output for every
      gate input; otherwise "schema validation not run" in the PR body.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-fingerprint (default mode), G-additive, G-closed (SARIF
      part), G-sarif-valid. Two reviewers.

### Slice S5: primitive and absence (1.5 days)

**Files:** create `crates/prism-review/src/output/rule_registry/{primitive_rows,absence_rows}.rs` (declared
in `rule_registry.rs`), `crates/prism-review/src/algorithms/absence_slice/pairs.rs` (declared in
`absence_slice.rs`: `mod pairs; pub use pairs::{default_pairs, PairedPattern};`),
`tests/algo/novel/rule_identity_test.rs` (`mod` line in `tests/algo/novel/main.rs`); modify
`primitive_slice.rs`, `absence_slice.rs`, `rule_registry/tests.rs`, `docs/rules.md`, `CLAUDE.md`.

- [ ] First commit, move-only: `PairedPattern` and `default_pairs()` into `absence_slice/pairs.rs`, text
      unchanged; `targets_mapping_test` green. This keeps `absence_slice.rs` well under 1,000.
- [ ] Tests first: `primitive_findings_carry_their_rule_and_cwe` (one row per category, sources from
      `primitive_test.rs`), `absence_findings_carry_their_pair_rule` (one row per absence category, inputs
      from `absence_test.rs`), `other_algorithms_carry_no_rules` (contract, echo, membrane, symmetry,
      provenance, peer consistency, callback dispatcher); in `rule_registry/tests.rs`
      `every_primitive_rule_id_is_a_row`, `every_absence_pair_id_is_a_unique_row`. RED:
      `cargo test --test algo_novel rule_identity_test::`.
- [ ] Primitive: `pub(crate) const RULE_IDS: [&str; 6]`; `make_finding` sets
      `rules: vec![rule_registry::rule_ref(rule_id)]` and `debug_assert!(RULE_IDS.contains(&rule_id))`.
- [ ] Absence: `PairedPattern` gains `pub id: &'static str` (stage D's table); the four finding sites set
      `rules` per stage D. Descriptions do not change. Registry rows with D's CWEs; re-bless; GREEN.
- [ ] Gates: section 5 items 1 to 4, 8, 9, 10; G-additive, G-fingerprint, G-targets, G-closed. Two reviewers.

## Close-out (0.5 day)

- [ ] The `CLAUDE.md` paragraphs the slices added read as one account; `docs/rules.md` is current.
- [ ] Comment on #603: lane 4 decision 9 can now use the field (category `taint_sink`; `js-proto-write`,
      CWE-1321 in `rules`; a registry row; no tag in the description, which keeps the fingerprint free of it).
- [ ] Follow-up issues, each with its evidence: `wrong-tag` rows from S2; structured matching returns one
      pattern per edge (two structured calls on a line may report one); the `security` tag and
      `security-severity` (decision 5); rule and CWE in targets (decision 6); more algorithms (decision 4).
- [ ] Ledger and `HANDOFF.md` refreshed; outputs summarised and offloaded; `target/` deleted; issue closed.

## Acceptance for the lane

Every slice passed its gates against its merge base (G-additive, G-fingerprint, G-targets where listed);
every positive row of the sink-rule and identity tables asserts its rule ids and CWEs (G-coverage); the
registry tests pass and `docs/rules.md` is current (G-closed); `specific`-mode output passes the shape tests
(and the schema, if available); both reviewers approved S2, S4 and S5.

**Estimate:** M 0.5, T 2, E 1, D 1, S1 0.5 (during T), S2 2, S3 1.5, S4 2, S5 1.5, close-out 0.5:
12.5 days of work, 12 on the critical path.

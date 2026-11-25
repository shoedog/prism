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
rule (DESIGN 5.3) and generates `docs/rules.md`. SARIF adds `properties.rules` for a nonempty normalized list;
`--sarif-rule-ids specific` writes one result per rule with registry descriptors and CWE tags. Primitive and
absence findings carry their rule last. DESIGN Appendices A–F are the Stage D record. Appendix E lists
the 540 matrix expectations and 300 source-only candidates, including the accepted corrections.

**Tech stack:** Rust (`prism`, `prism-review`); Python 3 for the gate script. **Spec:** `DESIGN.md` here
(field 5.1, taint 5.2, registry 5.3, others 5.4, SARIF 5.5, review/JSON 5.6, targets 5.7, gates 6, owner 7).

## Global constraints

- Worktree `~/code/prism-rule-identity`; evidence `~/prism-evidence/rule-identity/`. Use the pinned
  Stage T matrix and one product branch/PR per Stage I slice below.
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
  verbatim, so a line found by both of its runs keeps the baseline run's rules (DESIGN goal 2).
  S2-W pins this boundary and the emitting language slice checks overlap and expanded-only cases if merged.
- Lane 4's sink (DESIGN 5.2, last paragraph). Before lane 4 dispatches its sink slice, the orchestrator posts
  the field contract on #603 and in lane 4's ledger: category `taint_sink`, `js-proto-write`/`CWE-1321` in
  `rules`, no tag in the description. S2-R precedes any lane 7 id publication. Whichever slice first
  publishes lane 4's row adds its registry constant in the same PR. S2-JS migrates existing side-set
  evidence if lane 4 merged first (DESIGN 5.2).
- Weighted sizes now: `taint/run.rs` 601.5, `absence_slice.rs` 906, `primitive_slice.rs` 794,
  `contract_slice.rs` 1447, `main.rs` 990.5, `cli.rs` 483.5, `src/slice.rs` 572, `output/sarif.rs` 380.5,
  `sarif_rules.rs` 436.5. Only `contract_slice.rs` stays over 1,000 (12 one-line additions in S1; its PR
  says so); new logic goes in the new modules named below.
- Use the host slot procedure in ORCHESTRATION for product runs. Inside an already guarded seat use
  normal `cargo` and `prism` commands. No mutgate.

## Review focus

1. Two flat rules on one line (`res.send(eval(x))`): one finding, `rules` = `js-eval`, `js-response` and the
   `structured-code-eval@CWE-95`; one `specific`-mode result per distinct id, sharing a fingerprint.
   Row `ri01` (S2-JS) and `specific_mode_writes_one_result_per_rule_sharing_fingerprint` (S4).
2. Attribution per occurrence (DESIGN 5.2): a sibling sink that does not take the tainted value lends neither
   rule nor strength (`ri08`, `ri25`, `ri26`); a continued call on an admitted use line carries the eligible enclosing rules (`ri20`);
   two structured calls on one line carry both rules in either order (`ri21a`, `ri21b`); a cleansed first
   match beside a live second match (`ri22`). A cleansed structured call beside an unrelated flat sink:
   neither the structured rule nor the rules of flat identifiers inside the cleansed call appear (`ri15`).
3. Lines inserted outside the per-edge loop (Python inline Flask source, Go source equal to sink, JS YAML
   inline source) carry their structured rule; caller-supplied and wrapper callees carry `configured-sink` and `format-wrapper`, both without a CWE.
   S2-JS owns YAML, S2-PY owns signature-bound Flask source and unescaped context use in all three source orders,
   S2-GO binds the receiver or mux.Vars argument to the handler parameter on both fallback paths,
   and S2-L owns the C wrapper/config fixtures. Check each with the admitted wrong-slot control.
4. Default-mode SARIF for findings that carry rules: no new descriptor or descriptor key, `mapping_version`
   still `"1"`, only `results[].properties.rules`. `default_mode_adds_properties_rules_and_keeps_rule_ids` (S4).
5. `specific` mode with admission, indexes and odd inputs: a multi-rule finding removed by
   `--min-confidence exact` writes nothing; each expanded result's `ruleIndex` names its own descriptor;
   normalized duplicates of one id give one result. Exercise wrong registered CWE, wrong algorithm,
   unknown external ids and conflicting levels in both orders. Check every result's full normalized
   properties **and** its indexed descriptor together. Group an empty primitive finding with a verified
   one of the same category. The specific fallback must index a separate tagless descriptor (S4).
6. S2-R closes the public ids before any producer. S2-W checks all 540 keys and the occurrence join.
   At each language PR check that language's complete findings and every already activated partition.
   Use A's closed nominal exceptions, including the neutral unbound redirects and local SQL export,
   and its SQL-literal witness for Python `query` alone on an unresolved receiver.
7. S3 registers all 12 primitive and 90 absence rows. S5 reviewers compare one proved occurrence and
   admitted neutral controls per conditional row, including effective or unresolved shell/TLS
   protection, the closed credential slots, and cleanup on the path the absence finding names. Check
   the two dropped lifetime tags. Compare finding counts and severity independently of metadata.

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

**Gate procedure.** Build the exact integrated slice base and head in separate checkouts under the
host slot procedure. Identify each binary and run all 35 fixed inputs with both. Run the M comparator
on all outputs. Pass each named scope as a separate `--new-rules` argument. S1, S2-R, S2-W,
S3-G, S3-N and S5-A1 have empty scope. S2-JS/PY/GO/L each allow only `finding:taint` additions,
S4 only `sarif:taint`, S5-P only `finding:primitive` and `sarif:primitive`, S5-A2 only
`finding:absence` and `sarif:absence`. These comparator scopes operate by algorithm and output.
G-coverage checks language isolation on the matrix using each fixture's language, the exact
active-partition expectations and empty rules on every inactive partition (shared S2/S3 gates).
The comparator still requires byte equality for a rules key present on both sides. At close-out
repeat against the pre-lane base with all six scopes. Include the comparison summary in every PR.

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
      primitive). D-2 and D-3 amend DESIGN and PLAN. S2-W transcribes the 125 `?` and 35 revised concrete
      cells at `5e9e4d864cbb`, preserving the original observations. Replay all 160 plus `ri23e`,
      `ri23f` and `ri27` on the base (ORCHESTRATION section 7). These cover the original matrix's 163
      transcription cells at `a3986ce333f1`. E records the eleven TSV and one candidate changes from
      D-1. D-3 changes no TSV or candidate expectation relative to D-2. Preserve all 540 keys.
      Candidate rows and open grids are source-only until replayed. Both reviewers rejected D-1.
      Round 2 of the cap of 2 returned B REJECT and A APPROVE on `be906634a77c`. D-3 folds their
      bounded findings on that artifact. The design approval gate awaits controller disposition.

## Stage I: slices

S1 is present. Use one PR per row below in the listed order. Estimates include tests in weighted
lines of hand-edited code, excluding generated page/TSV bytes and the identified move-only commit.
Each PR has a review cap of **3 rounds**, declared before dispatch. Both reviewers cover product
changes. At the cap apply ORCHESTRATION's convergence/owner procedure. Split a forecast that outgrows
its boundary before dispatch. Include the slice's row, shared gates and Stage M scopes in each
dispatched brief so its files, dependencies, size, review cap and acceptance travel together.

| Slice | Depends on | Scope | Days | Weighted lines | Cap |
|---|---|---|---:|---:|---:|
| S1 | T/E may overlap | Field (present) | 0.5 | mechanical literals, ~70–110 | 3 |
| S2-R | S1, approved D, pinned T | Minimum taint registry and producer checks | 0.75 | 250–450 | 3 |
| S2-W | S2-R, pinned T | Shared collection, measurement and matrix harness | 1 | 300–500 | 3 |
| S2-JS | S2-W | JS/TS/TSX including YAML | 1.5 | 450–700 | 3 |
| S2-PY | S2-JS | Python including Flask and review golden | 1.5 | 450–700 | 3 |
| S2-GO | S2-PY | Go including both inline routes | 1.25 | 350–600 | 3 |
| S2-L | S2-GO | Legacy languages and config/wrapper fixtures | 0.5 | 120–250 | 3 |
| S3-G | S2-L | Registry completeness and generated page | 0.75 | 180–320 | 3 |
| S3-N | S3-G | Primitive and absence registry rows | 0.75 | 220–400 | 3 |
| S4 | S3-N | Both SARIF modes and external inputs | 2.5 | 450–700 | 3 |
| S5-P | S4 | Primitive occurrence projection | 1.25 | 300–500 | 3 |
| S5-A1 | S5-P | Absence inventory move, native acquisition and repeat proofs | 1 | 280–450 | 3 |
| S5-A2 | S5-A1 | Remaining acquisition proofs and all absence publication | 1.25 | 300–550 | 3 |

Each language publishes **complete findings** including all its source insertions and any configured
or wrapper role on the same line. A later source-only PR could change a rules key already shipped
by an earlier language PR, failing G-additive. S2-W supplies common code without a public producer.
S3-G and S3-N add documentation/inventories behind the already enforceable registry contract.
Landing the former S2 and S3 together would combine 34 tags, 107 patterns and registry/page review
in one PR. Each language now has one complete, independently testable publication boundary.
JSON and review serialize the field directly (DESIGN 5.6).

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

### S2 and S3 gates shared by the following PRs

- [ ] Use ORCHESTRATION section 5 items 1–4 and 8–10 for each PR. Bind the RED regression and a
      negative/edge to the exact pre-slice base and re-run affected tests on the integrated head at
      `RAYON_NUM_THREADS=1` and `4`. Record commands, totals, evidence paths, exclusions and pending
      full-suite status in VERIFICATION.md. Include the behavior paragraph in `CLAUDE.md` when it
      changes and the JS/TS coverage entry in `docs/features/language-coverage/js-ts-known-gaps.md`
      in S2-JS. Follow the repository's main integration and pre-merge gate.
- [ ] Run G-additive, G-fingerprint (default mode) and G-targets on the pins for **every** row, with
      the exact scopes in Stage M. No change to a previously present key is allowed. Run G-closed
      on every active producer, including unknown/wrong CWE and wrong-algorithm failing controls.
- [ ] G-coverage always checks the 540-key inventory in both directions and presence on every marked
      line. From S2-W, assert ids, immutable CWEs and relative severity for every activated partition,
      including admitted-empty rows. Assert empty rules on unactivated lane 7 partitions. Fixtures run
      under multiple grammars still count once per (table, label). Retain the admission and line-strength
      assertions and `#[track_caller]`. Check each new language partition RED on its own base.
- [ ] Before activating a producer run DESIGN F's joined census for its language. Attach the old/new
      identity and reason counts, sink denominator, unadmitted keys and admitted empties, and one
      location per nonzero reason. Include pre-existing discrepancies with the base observation,
      #841/lane 4 state and any unexecuted candidate or grid cells. The 300 candidate decisions
      select extra positive/negative controls. Check base admission before asserting a new example.

### Slice S2-R. Minimum taint registry

**Files.** `crates/prism-review/src/output/{mod.rs,rule_registry.rs,rule_registry/taint_rows.rs,rule_registry/tests.rs}`;
the Stage T test support. Expose `RuleInfo { id, cwe, name, short, help, algorithms }`, its
`rule_ref(&'static self) -> RuleRef` maker, `rules()` (sorted immutable table), `lookup(id)` and
`check_emitted(algorithm, rule)`. Only this maker creates a producer reference. Keep admission
`SinkHit` and `Strength` private to detection/grading. No language publishes here.

- [ ] Register each distinct public A/B id once, including `taint-redirect-name`, the structured
      exceptions, neutral roles, config, wrapper and legacy. Use DESIGN 5.3's text conventions and
      exact legacy strings. Register lane 4's conditional row with its first producer.
- [ ] RED/GREEN `cargo test -p prism-review --lib output::rule_registry`. Check unique sorted ids,
      CWE grammar, known algorithms, text, constant/table membership, canonical refs and rejection of
      unknown id, wrong/extra/missing CWE or unlisted algorithm. A neutral row accepts only no CWE.
- [ ] Scan parsed **Rust expressions** in production modules under `src/` and `crates/prism-review/src/`
      for struct construction (qualified paths and local import aliases included) or a constructor through `RuleRef::`, outside
      the registry maker. Check `SinkHit` literal construction outside its defining module the same way.
      Respect test-only modules. Pass controls for `pub struct RuleRef {`, type/import references,
      comments, strings and `JS_EVAL.rule_ref()`. Fail a qualified literal with a computed id.
      S4 uses this maker for registered ids and clones an external ref to normalize an unknown id.
      Keep the scanner active on the SARIF module.
      A substring ban rejects S1's declaration. A text exemption for that one declaration would
      still match examples in strings and comments. Pair the scan with producer return types and
      `check_emitted` on actual output.
- [ ] Shared gates, empty scope. G-coverage on the Stage T base checks presence/keys only. Show that
      the maker/validation controls fail before implementation. The generated page belongs to S3-G.

### Slice S2-W. Collection and matrix support

**Files.** `algorithms/taint.rs`, `taint/{run,sink_match,sink_rules}.rs` (under `crates/prism-review/src`),
Stage T's support, expectation, identity and TSV files, and `tests/algo/taxonomy/taint_sink_strength_test.rs`.
Keep census instrumentation in test/diagnostic support; observe the real old visits as specified in F.

- [ ] Transcribe all 160 reviewed TSV cells by exact key. Expand an old `observed =` to its original
      value wherever the concrete expectation changes, while preserving explicit observations. Replay
      the Stage D base controls. Reject residual `?`, missing/duplicate keys or silent empty selection.
      Use the fixture's language for mixed-table `ri*` rows. Run `cargo test --test algo_taxonomy taint_sink_rules`.
- [ ] Collect occurrences and reached operand byte spans on final admitted keys. Keep the existing
      admission function (including first exclusions), source/scope filters and `line_strength`.
      Implement coalesced-use and rebinding checks, per-call cleansing hooks, and a sorted id reducer
      (strong wins, suggestion only for an all-weak id in a warning). Prepare config/wrapper origins
      separately for every language. Return registry constants from projection helpers. Tests cover
      duplicate paths to one operand, sibling binding collision, different slots, empty admission,
      weak+strong same id and the separately reported spanless old-hit group.
- [ ] Shared gates, empty scope. Keep all 540 presence controls and zero activated partitions.
      Exercise the shared collector/reducer via tests before any public activation. Pin evidence
      and the baseline/expanded union choice if #841 is present.

### Slice S2-JS. JavaScript, TypeScript and TSX

**Files.** `taint/{run,sink_rules,js_ts/sinks}.rs`, dedicated JS projection module under
`taint/sink_rules/`, Stage T's JS, hybrid_js and identity tests, and
`crates/prism-review/tests/cli/{main,rule_identity_output_test}.rs`. Use existing classifier facts by
read-only accessors if necessary. Across all S2 PRs, projection corrections belong in the projection.
Do not rewrite private classifier tags, tables, strengths or exclusions to satisfy the identity TSV.

- [ ] Collect every eligible flat/structured occurrence and operand in the 40 JS entries, both
      rename slots, JSX/RHS, continuations and nested calls. Include per-call guards, the TypeORM
      clause, SQLite, VM/Function, Mongoose, browser-only destinations, fixed-path data, SQL bind
      severity and both orders of a clean sibling. Include inline YAML and the framework-source
      range filter, all config/wrapper overlaps and lane 4's side-set migration if present.
- [ ] Test A's nominal matrix with an unbound callback, the unproved local query export, and both
      bound cross-spawn labels plus unbound/shadowed spawn. Check JSON/review single/multi-run
      findings, both result and aggregate copies, diagrams on/off, severity floor and empty omission.
- [ ] Shared gates with `finding:taint`. G-coverage adds js + hybrid_js (197) and `ri01,02,03,07,13,19,24,26`
      (8). Require **205/540** active keys and both negative and positive rows at each test thread count.

### Slice S2-PY. Python

**Files.** `taint/{run,sink_rules,python/sinks,python/sanitizers}.rs` (read existing guards),
the Python projection module, py/hybrid_py/identity tests, CLI output tests and
`tests/fixtures/review_no_diagrams/golden/review_unflagged.json`.

- [ ] Project the 48 structured entries and flat roles, pathlib's consuming receiver, continued SQL,
      both orders of live/cleansed siblings, SQLModel exec and proven ORM versus text versus opaque
      query (all A branches). Test the lowercase parameter, variable-only formatting, reordered
      SQL keywords, bound SQL, weak clause and sandboxed/normal template roles on both finding levels.
      Replay `astra-r2-04`: `conn.execute("SELECT " + x)` on an unresolved receiver gets
      `taint-query-name` and the ORM sibling only the weak value id. Add negative controls for an
      unresolved `executemany` and an injected `executor.execute("SELECT ..." + x)` (no SQL id), and
      for an opaque statement without a literal, a bound value and a known non-SQL receiver.
- [ ] Test B's four Flask shapes on edges and inline request values. Bind positional/keyword
      source despite reordering; refuse duplicate or unresolved splats, unused context and outputs
      after endautoescape or inside nested true. Derive the projection's context exclusion from
      the bound source and scoped use. On an eligible edge assert the two `@CWE-79` refs `py-html`
      and `structured-xss` for each of `render_template_string("{{ name | safe }}", name=x)`,
      `render_template_string(source="{{ name | safe }}", name=x)` and
      `render_template_string(name=x, source="{{ name | safe }}")`. Check inline-only context
      yields just the structured ref on admission. Retain the source-operand pair of template ids,
      the source-line flat exclusion, a real admitting sibling and an unadmitted control.
      Keep the first-outcome admission semantics.
- [ ] Shared gates with `finding:taint`. G-coverage adds py + hybrid_py (147) and
      `ri04,06,08,12,20,21a,21b,22` (8), **360/540** cumulative. Re-record only the two copies of
      the Python sink in the review golden, with the decision 7 reason and exact diff in the PR.

### Slice S2-GO. Go

**Files.** `taint/{run,sink_rules,go/sinks}.rs`, Go projection module, go/hybrid_go/identity and
sink-strength tests. Read the existing source table and exact receiver/argument bindings.

- [ ] Cover 17 ordinary and two framework patterns, context offsets, shell/ordinary argv, writer
      and format slots, typed Gin, opaque DB methods, weak SQL on a warning, generic/continued
      callees and Go's call-start limit. A later successful path cannot hide a live sibling.
- [ ] Union every eligible originating path and inline witness on a source line. In *both* inline
      routes test a mux.Vars argument resolving to the handler request (including a unique alias),
      a fresh request, a reassigned parameter, another binding and a shadow. Test source-only, incoming-only and mixed
      provenance, wrong sink slot and shape-only admission. Preserve the independent path contribution
      and the established flat-source exclusion. Include the bound `astra-r2-15` shape.
- [ ] Shared gates with `finding:taint`. G-coverage adds go + hybrid_go (109) and
      `ri05,05b,11,15,25` (5), **474/540** cumulative.

### Slice S2-L. Legacy and remaining name-only fixtures

**Files.** `taint/{run,sink_rules}.rs`, the legacy projection and Stage T's other, hybrid_other
and identity tests, CLI output tests and `CLAUDE.md` (taint paragraph).

- [ ] Enable the remaining grammars together. Match the legacy token on the admitted physical line.
      For config/wrapper project only a reached argument to its matching callee, with separate exact
      and substring predicates. Check duplicates, overlapping built-ins, unquoted expansion, source,
      a wrapper name with a clean sibling, `ri27` and the Java non-admission. JS/PY/GO must already
      handle these name origins on their own lines.
- [ ] Shared gates with `finding:taint`. G-coverage adds other + hybrid_other (53) and
      `ri09,10,14,16,17,18,23a–23f,27` (13), **540/540** cumulative. Reconcile the per-language
      censuses, the 300 candidate dispositions and the two open grids, naming unexecuted cases.

### Slice S3-G. Registry completeness and generated page

**Files.** `output/rule_registry{.rs,/tests.rs}`, `docs/rules.md`, `scripts/ci/classify-changes{,.test}.sh`,
and `CLAUDE.md` (output paragraph).

- [ ] Generate id, optional CWE, name, algorithms, short and help in id order, with the regeneration
      command in the header. Test `rules_page_is_current` and its controlled `PRISM_BLESS_RULES=1`
      regeneration. Classify the generated page as code before the generic docs case and test it.
      Cross-check all 34 flat tags and all 107 structured entries including their role splits against
      A/B. Confirm neutral rows have no CWE and the legacy text matches 5.3.
- [ ] Shared gates, empty scope, all 540 active. Run `cargo test -p prism-review --lib output::rule_registry`
      and `bash scripts/ci/classify-changes.test.sh`. The public producers are already closed by S2-R.

### Slice S3-N. Register the non-taint rows

**Files.** `output/rule_registry/{primitive_rows,absence_rows,tests}.rs`, `output/rule_registry.rs`
and the generated `docs/rules.md`. No algorithm emits here.

- [ ] Register C's six category ids with their fixed CWEs plus six literal `_MATCH` ids without CWE
      (algorithm primitive). Register D's 65 primary missing-counterpart ids, 22 additional `-match`
      rows and three repeat ids (algorithm absence). Of the 65, 22 have the verified CWE and 43 have
      none, including `absence-connection` and `absence-go-sql-db`. Repeat rows are double-free/415,
      double-unlock/765 and repeated-counterpart without CWE. Register each literal once.
- [ ] Compare membership to C and to all 65 `default_pairs()` descriptions and the D fallback map.
      Test exact immutable CWEs and algorithms, cross-algorithm uniqueness, neutral help and rejected
      mismatches. Reflect C's shell/TLS protection and credential slots, and D's unmet obligation
      on the reported path, in the verified and neutral rows' help. Regenerate the page.
      Shared gates, empty scope, **540/540** G-coverage and G-closed.

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
// optional CWE tags. An empty rule list uses prism-fallback/<algorithm>/<category> in specific mode.
// Switching modes can start new code-scanning alert histories for the affected findings; how
// GitHub shows the CWE tags is unverified."
#[arg(long, value_enum, default_value_t = SarifRuleIds::Category)] pub sarif_rule_ids: SarifRuleIds,
```

Add `ResultProperties.rules`, optional descriptor help/category and optional tags, skipped when
empty. After admission and #841's map/diagnostic choices, normalize incoming references by DESIGN
5.5 before constructing **any** result properties or specific descriptors. Use the registry maker
for an authorized row (including its optional CWE) and drop registered ids under any other algorithm.
Group duplicates using the strongest effective severity and sort by id. Category mode uses this
canonical list with the ordinary category descriptor. Specific mode copies this same full list on
each result, selects its level from the canonical ref and uses the authorized row's optional tag.
If the list is empty on entry or after normalization, omit the rules key and use the ordinary
category id in default mode. In specific mode emit one finding-level result with the separate
`prism-fallback/{algorithm}/{category}` id and a tagless category descriptor. For an unknown external
id, clone a supplied ref in that group and overwrite both public fields `cwe` and `severity` with
the reduced per-finding values from 5.5. Its descriptor uses the per-(algorithm,id) run-wide tag
union. No `RuleRef` literal or new public id factory is needed in `sarif.rs`.

Compute message, evidence, fingerprint and locations once. A registered specific descriptor uses
`name`, `shortDescription = short`, `fullDescription = help`, `help` and algorithm/optional tags.
It omits category. The disjoint fallback prefix also covers external ids containing separators. Unknown ids
use the id as name and `"{algorithm}: {id}"` for both descriptions. Sort results and descriptors
before assigning `ruleIndex`. Document the default-mode result count and the consumer key with ruleId.

- [ ] Tests first (in-process through `prism_review::api` with `SliceFinding { rules: …, ..Default::default() }`,
      plus CLI runs): review-focus tests 4 and 5; `default_mode_without_rules_has_no_rules_key`,
      `specific_mode_writes_one_result_per_rule_sharing_fingerprint`,
      `specific_mode_descriptor_has_registry_text_and_cwe_tag`, `specific_mode_rule_without_cwe_has_no_tags`
      (use named neutral output, wrapper and configuration rows, including a mixed-CWE finding),
      `specific_mode_finding_without_rules_indexes_tagless_fallback`, `specific_mode_every_cwe_descriptor_has_its_tag`
      (CLI on identity fixtures), `specific_mode_weak_rule_result_is_note`,
      `specific_mode_repeated_id_gives_one_result`, `specific_mode_conflicting_cwes_for_one_id_give_one_result`,
      `specific_mode_unknown_rule_keeps_supplied_cwe_tag`, `specific_mode_unknown_rule_without_cwe_has_no_tags`,
      `specific_mode_two_unknown_inputs_with_different_cwes_tag_both`,
      `specific_mode_registry_wins_over_disagreeing_cwe`,
      `specific_mode_mixed_primitive_keeps_fallback_tagless_in_both_orders`, `specific_mode_is_byte_deterministic`,
      `sarif_rule_ids_flag_rejects_unknown_values`, `sarif_rule_ids_help_mentions_alert_history`. RED:
      `cargo test -p prism-review --test cli sarif_rule_ids_test::`.
- [ ] Through public `SarifInputs`, pass `js-eval@CWE-502` on taint alone and beside valid rows.
      Every copy carries `js-eval@CWE-95`, and its specific descriptor has only the 95 tag.
      Supply `js-eval` on absence alone and beside a valid absence id. Assert its absence from
      properties and specific descriptors, the mode-specific empty fallback, and the surviving valid id.
      Also supply a CWE for a neutral row, omit one for a tagged row, reverse duplicate severities
      (including absent), and reverse conflicting unknown CWEs. Check indexed descriptors, result
      levels and the full canonical list in each mode. Category descriptors gain no tags.
- [ ] Mix two primitive inputs of category `SHELL_TRUE_WITH_INTERPOLATION`, one with that verified
      ref and one with `rules=[]`. In both input orders follow each result's `ruleIndex` and check
      the indexed id equals its `ruleId`. Only `prism/primitive/SHELL_TRUE_WITH_INTERPOLATION` gets
      the CWE-78 tag and the verified rule in result properties. The empty result indexes
      `prism-fallback/primitive/SHELL_TRUE_WITH_INTERPOLATION` with no tags or result rules key.
      Check fingerprints against their respective category-mode inputs and keep one result per input.
      Count the specific-mode empty fallbacks by algorithm/category on the pins and check each
      indexed fallback descriptor is tagless.
- [ ] `sarif_shape_test.rs`: `run_sarif` gains a mode argument; every assertion that does not name a
      category-mode rule id runs in both modes (the category-id literals stay default-mode only).
- [ ] Implement; GREEN; `sarif_test` passes unchanged.
- [ ] G-sarif-valid as far as S4 can: the shape tests in both modes, and, if `python3 -c 'import jsonschema'`
      works and a SARIF 2.1.0 schema copy is in the evidence directory (record its SHA-256), validation of a
      mixed `specific`-mode log with non-empty results; otherwise "schema not validated" and the assertions
      that ran, in the PR body. S5-A2 repeats it on the final head.
- [ ] Gates include section 5 items 1–4 and 8–10, G-fingerprint (default mode), G-additive
      (`sarif:taint`), G-targets, all 540 G-coverage keys, G-closed and G-sarif-valid. Two reviewers, cap 3.

### Slice S5-P. Primitive projection

**Files.** `algorithms/{primitive_slice.rs,primitive_slice/projection.rs}`, `tests/algo/novel/{main,rule_identity_test}.rs`
and the primitive tests. Reuse the twelve S3-N rows. All three S5 PRs use the section 5 product gates,
the RED/base control, both test thread counts, all 540 G-coverage keys and the M comparator on all pins.

- [ ] RED/GREEN `cargo test --test algo_novel rule_identity_test::` and the affected primitive and SARIF
      suites. Make each existing emitter pass its site to the projection. Select the category id with
      C's CWE on proof, else its literal `_MATCH` row without CWE. Preserve category, description,
      severity and multiplicity. Give each C row a genuine tagged positive and an **admitted** neutral
      control through the normal algorithm entry, verifying both category and specific SARIF.
- [ ] Pin the non-hash digest, non-returned/textual helper slice, shadowed hashlib, fixed subprocess
      command beside unrelated interpolation, unrelated verify option and unused/public token literal.
      Bind the actual callee, positional/keyword operand and result use where C requires it.
- [ ] On admitted POSIX shell findings compare an unquoted field from an unconstrained string
      (CWE-78), `subprocess.run(f"printf '%s' {shlex.quote(x)}", shell=True)` (`_MATCH`), and an
      opaque quoting helper (`_MATCH`). Test the effective insertion context and all fields.
      On Go TLS client handshakes compare a fixed `InsecureSkipVerify: true` with no replacement
      checks (CWE-295), a `VerifyConnection` that enforces chain and hostname (`_MATCH`), an opaque
      verifier (`_MATCH`) and a proved no-op verifier with no other checks (CWE-295).
      Assert the admitted category and exactly the corresponding tagged or CWE-less id in each case.
- [ ] Use exactly C's credential slots on resolved requests/httpx module calls (auth tuple password
      and Authorization bearer value) and `redis.Redis(password=…)`. Cover each family and slot,
      plus a username-only use, a rebound literal and an unlisted client's `password=` on admitted
      assignments. All unproved uses yield `HARDCODED_SECRET_MATCH` without a CWE.
- [ ] Measure the six pinned primitive findings by C row and witness, including the D-2 → D-3
      tag/fallback transitions and their reasons, before enabling publication.
      G-additive permits only `finding:primitive` and `sarif:primitive`. G-fingerprint, G-targets,
      G-closed and both SARIF modes pass. Two reviewers, cap 3.

### Slice S5-A1. Native acquisition and repeated-operation evidence

**Files.** `algorithms/absence_slice.rs`, `absence_slice/{pairs,native,repeat}.rs` and their
projection tests. First commit moves `PairedPattern/default_pairs()` unchanged and re-exports them
from the same module. Verify the 65 descriptions and `targets_mapping_test` on that commit.

- [ ] Implement private selection for D's 16 kernel/POSIX acquisition rows, with an exact typed or
      library-bound full callee and a manual ownership/lock obligation proved outstanding at the
      exit on D's category-specific path. Return the S3-N verified constant or its CWE-less fallback.
      No producer attaches an absence ref yet.
      Data-drive each row with a positive and shadow/tail/managed/ownership negative. Include
      devm_kzalloc, devm_kstrdup, devm_request_irq, a managed DMA spelling, NumPy/Python mmap and
      an application open/socket helper.
- [ ] Extend those same 16 row controls. For each, check an admitted open with a feasible local exit
      and no release (tag), a release of that resource via a unique callee alias (`-match`) and an
      unresolved possible cleanup (`-match`). Trace the acquisition result or lock/reference through
      release and exit. For the goto categories include normal-return versus error-label release,
      cleanup before the named goto and label fall-through. Releasing a different resource or on a
      different path cannot discharge the reported obligation. Keep the controls on admitted sites
      by using alias calls the legacy scan misses. Use the original emitter's named normal return/goto.
- [ ] For `double_close`, check actual release functions, their designated resource operands at both
      goto-path sites and absence of a reset/reacquisition. Test free/415, unlock/765 and generic
      repeat against the C goto fixtures, distinct operands, wrong free slot and substring names.
- [ ] G-additive empty scope, G-fingerprint, G-targets and G-closed. Include private proof RED/GREEN
      controls and rerun the novel and C goto suites. Two reviewers, cap 3.

### Slice S5-A2. Remaining acquisitions and absence publication

**Files.** `algorithms/absence_slice.rs`, `absence_slice/{pairs,projection}.rs` and focused
language modules as needed, novel rule-identity tests, SARIF shape tests and `CLAUDE.md`.

- [ ] Add the six remaining conditional pairs (one Node fd, two Go, two Lua, one shell) and their
      proof/fallback controls. Test GORM/client Create, cache.Get/once.Do, local `open(1)`, real
      Node fs.open, Lua io.open and a real versus quoted shell descriptor redirect. Even a real
      process-lifetime connection or sql.Open uses its dash id. Check signal.connect and os.Open
      misidentified by the old matcher.
- [ ] Apply D's same three cleanup controls to each of these six rows on admitted findings. For
      Node use `const release = fs.closeSync; const fd = fs.openSync(path, "r"); release(fd)` with
      the open marked. Expect exactly `absence-node-file-descriptor-match`. For Go file/HTTP body
      use a bound Close method value, for Lua file/socket an aliased close method with its resource.
      For shell use a function in a continuing shell with a local fd obligation, `exec 3>file` then
      `exec 3<&-`. Expect exactly `absence-shell-file-descriptor-match`. Check both close directions
      and both modeled fd numbers in the proof helper. For each pair also exercise a witnessed
      unmet obligation (tag) and unresolved cleanup (`-match`). Run the controls for all 16 + 6 pairs
      through the public emitter, checking base admission, the exact rule/CWE, and multiplicity per marker.
- [ ] Select exactly one of S3-N's 87 ordinary constants for each finding in the three missing
      categories. Associate the pair definition with its literal primary/optional fallback constants
      in the same internal inventory. Preserve the public `default_pairs()` representation and order.
      Use the exact acquisition, bound API, resource identity and outstanding obligation on the
      reported path through its exit. When the only relevant paths have proved or unresolved cleanup,
      select the neutral alternate.
      Ambiguous same-line acquisitions fall back. For repeats select exactly one of the three repeat
      constants. Preserve admission, severity, descriptions and per-pair multiplicity. Verify the 65
      primary ids, 22 literal neutral alternates, both error-path categories and the two lifetime-only
      neutral rows against D. Other algorithms remain empty. Reuse registered rows directly.
- [ ] Before attaching rules, run F's census of all 34 pinned absence findings by pair, category,
      full matched callee, language, resource, path/exit, tagged/neutral/repeated and reason.
      Show D-2 → D-3 totals for outstanding, discharged and unresolved obligations. Reconcile multiplicities
      against the base and show zeros. Run the novel rule-identity, C goto and SARIF suites.
- [ ] G-additive scopes `finding:absence`, `sarif:absence`. G-fingerprint, G-targets, G-closed,
      540/540 G-coverage and final G-sarif-valid on nonempty taint, primitive and absence logs in
      both modes. Apply S4's checksum-pinned schema step or report “schema not validated” and the
      assertions that ran. Two reviewers, cap 3.

## Close-out (0.5 day)

- [ ] The `CLAUDE.md` paragraphs the slices added read as one account; `docs/rules.md` is current.
- [ ] The cumulative comparison against the pre-lane base (stage M), with every scope, in the close-out note.
- [ ] Follow-up issues, each with its evidence: the CWE-less entries F1–F8 and the owner question about fork in DESIGN Appendix F; the main defects of DESIGN
      section 8 (presence decided by the first structured match; Python call-level cleansing on continued
      calls); the GitHub check of DESIGN section 2, for someone with upload rights; the `security` tag and
      `security-severity` (decision 5); rules for a targets consumer (decision 6); more algorithms
      (decision 4).
- [ ] Ledger and `HANDOFF.md` refreshed; outputs summarised and offloaded; `target/` deleted; issue closed.

## Acceptance for the lane

- [ ] The stage D review record has both approvals at the design cap. Each Stage I PR has its own
      integrated-base receipt, sized diff, assigned approvals and review-round count. All four
      producer partitions pass Stage M's algorithm-scoped G-additive check. G-coverage verifies
      the language boundary on active and inactive matrix fixtures. Previously published rule values,
      every fingerprint, target id, finding and its severity pass the comparator.
      The cumulative check against the pre-lane base passes with the six stated scopes.
- [ ] G-coverage progresses through 205, 360, 474 and 540 active keys. All 540 assert admission and
      all positives assert ids/CWEs/relative severity, including empty references. No residual `?`.
      Record candidate/grid execution separately. F's pinned censuses reconcile all stages and list
      exclusions. Every production id was registry-backed at its first publication.
- [ ] G-closed checks A–D, both scanner controls, per-algorithm validation, and the current generated
      page. C/D projection tests exercise effective protections, closed credential slots and path-specific
      cleanup, including admitted neutral occurrences. Both SARIF modes agree on normalized result
      properties. Verified specific descriptors retain their tags and empty results index a tagless fallback.
      The final mixed-output shape and schema receipts meet G-sarif-valid. Run and record the full
      suite before merging to main, including any exclusions under the repository policy.

**Effort estimate.** M 1, T 2.5, E 1, D 1.5, S1 0.5 (during T), S2 6.5, S3 1.5, S4 2.5,
S5 3.5 and close-out 0.5 give 21 days of work and 20.5 on the critical path.

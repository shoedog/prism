# Lane 7: rule identity on findings — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a finding produced by a rule says which rule and, where one exists, which CWE, in JSON, review and
SARIF output, with no change to finding counts, severities, target ids, SARIF fingerprints or default SARIF
rule ids; default output gains only the additive `rules` key (DESIGN decision 7).

**Architecture:** `SliceFinding` gains `rules: Vec<RuleRef>` (core crate, skipped when empty). The taint sink
loop attributes, per sink occurrence whose operand the tainted use reaches, the rule of each flat hit and
every structured match, and attaches them to the `taint_sink` finding (DESIGN 5.2). One registry
in core (`prism::rule_registry`) supplies the named rows for built-in projections and the metadata
for `docs/rules.md`. DESIGN 5.3 defines the compiler, test and review guarantees.
SARIF adds `properties.rules` for a nonempty normalized list;
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
- PR #841 merged at `9d1a7d06`. Inventory `SliceFinding` literals on the integrated base with
  `rg -n 'SliceFinding \{' -g '*.rs'` and compare outputs to that base. S4 preserves #841's per-finding
  original/context map and both CLI branches' context and diagnostics before expanding results.
  The union keeps the baseline finding per overlapping `(category, file, line)` and the expanded
  finding on expanded-only keys. S2-W pins this choice and each emitting taint slice checks both cases.
- Lane 4's sink (DESIGN 5.2, last paragraph). Before lane 4 dispatches its sink slice, the orchestrator posts
  the field contract on #603 and in lane 4's ledger: category `taint_sink`, `js-proto-write`/`CWE-1321` in
  `rules`, no tag in the description. Built-in refs come only from named `prism::rule_registry`
  rows through the shared writer. S2-R precedes any lane 7 id publication. Whichever slice first
  publishes lane 4's row adds its core constant in the same PR. S2-JS migrates existing side-set
  evidence if lane 4 merged first (DESIGN 5.2). Lane 4 dispatches its sink slice only after S2-W has
  merged (the shared `attach_rules` writer exists from then on); its non-publishing work continues meanwhile.
- Before lane 8 I-9 dispatch, the orchestrator sends lane 8 the S2-R dependency on its tracking
  issue (I-9 plan in #881). Reconcile the move manifest and stop rule on the integrated tree.
  The registry lives in core. `prism-review` re-exports it. I-9 imports `prism::rule_registry`
  directly in `prism-taint`, omits the old review registry file from its moves, and reads the
  classifier's `SinkHit` through accessors. The same accessor contract is sent to lane 8 before its I-8
  dispatch (I-8 reads `SinkHit.rule`/`.cwe` today), so no lane 8 slice is dispatched on a base where those
  fields are private without it.
- Weighted lines measured with `python3 scripts/file_size.py` at `30216157813a` follow.
  Recount touched files on the dispatch base. Only `contract_slice.rs` in this inventory exceeds
  1,000. S1 added 12 lines there. New logic belongs in the slice modules named below.

  | File | Weighted lines |
  |---|---:|
  | `taint/run.rs` | 680.5 |
  | `absence_slice.rs` | 910 |
  | `primitive_slice.rs` | 795 |
  | `contract_slice.rs` | 1459 |
  | `main.rs` | 674 |
  | `cli.rs` | 557.5 |
  | `src/slice.rs` | 644.5 |
  | `output/sarif.rs` | 490 |
  | `sarif_rules.rs` | 437.5 |

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
   D-6 reviewers trace def token → read bytes → operation/operand → both indexed and semantic
   checks → per-path guard → reduced id. Reverse representative bytes without changing the input,
   including a clean shadow, a deferred write, and a third or nested live use. Require the same
   justified rows in both orders. Compare missing operand bytes with an unlocated detector visit.
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
6. S2-R reviewers inspect the public API from outside the registry. Try a macro/alias literal,
   an id/CWE edit after cloning a ref, and a forged static row. Check privacy diagnostics, serde
   compatibility, the unchanged 64-row inventory and the empty-scope byte receipt. Check the
   classifier's separate `SinkHit` visibility. Trace every built-in producer through `attach_rules`
   to named row selection. Check for bypasses, including vector replacements and deserialized/cloned
   findings. Inspect helpers and macros for raw factories, serde synthesis and computed
   lookups. Check the actual algorithm and the choice of row against the witness. S4 reviewers
   trace external ids to the admitted input and the private SARIF property type in both modes.
   S2-W checks all 540 keys and the shared writer. S2-WC checks the occurrence join.
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
S2-WC, the four S2 language proof PRs, S3-G, S3-N and S5-A1 have empty scope.
S2-JS/PY/GO/L each allow only `finding:taint` additions,
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
      bounded findings on that artifact. D-5 folds the single §8 review of D-4 (B REJECT,
      A APPROVE at `30216157813a`). D-6 changes one JS candidate and zero TSV cells.
      The one §8 review of D-6 at `45264869` returned B REJECT and A APPROVE. D-7 folds both
      reviews on that design; E retains all D-6 decisions. Hold collector dispatch for owner
      disposition under §8, including the S5 threshold and S6 budget below.

## Stage I: slices

S1 is present. Use one PR per row below in dependency order. Estimates include tests in weighted
lines of hand-edited code, excluding generated page/TSV bytes and the identified move-only commit.
Declare the cap before dispatch. A new product blocker has **3 rounds**. The #901 successors
share the remaining round for each inherited blocker as set out below. Both reviewers cover product
changes. At the cap apply ORCHESTRATION's convergence/owner procedure. Split a forecast that outgrows
its boundary before dispatch. Include the slice's row, shared gates and Stage M scopes in each
dispatched brief so its files, dependencies, size, review cap and acceptance travel together.

| Slice | Depends on | Scope | Days | Weighted lines | Cap |
|---|---|---|---:|---:|---:|
| S1 | T/E may overlap | Field (present) | 0.5 | mechanical literals, ~70–110 | 3 |
| S2-R | S1, approved D, pinned T | Core registry, field privacy, serde and producer checks | 1 | 300–450 | 3 |
| S2-W | S2-R, pinned T, owner disposition on D-7 | Fold #901 in place, shared evidence interface, writer and matrix | 1.25 | 350–550 | 1 left (#901) |
| S2-WC | S2-W | Located old visits, diagnostic join and controls | 0.75 | 250–400 | 1 left (#901) |
| S2-JS-proof | S2-WC | JS/TS/TSX read/value proofs and operand layout, dormant | 1.5 | 350–550 | 1 left (#901) |
| S2-JS | S2-JS-proof | JS/TS/TSX including YAML | 1.5 | 450–700 | 3 |
| S2-PY-proof | S2-WC | Python read/value proofs and signature binding, dormant | 1.25 | 300–500 | 1 left (#901) |
| S2-PY | S2-JS, S2-PY-proof | Python including Flask and review golden | 1.5 | 450–700 | 3 |
| S2-GO-proof | S2-WC | Go bindings, roles and source keys, dormant | 1 | 250–450 | 1 left (#901) |
| S2-GO | S2-PY, S2-GO-proof | Go including both inline routes | 1.25 | 350–600 | 3 |
| S2-L-proof | S2-GO | Legacy-language callee/value adapters for name origins, dormant | 1 | 250–450 | 1 left (#901) |
| S2-L | S2-L-proof | Legacy languages and config/wrapper fixtures | 0.5 | 120–250 | 3 |
| S3-G | S2-L | Registry completeness and generated page | 0.75 | 180–320 | 3 |
| S3-N | S3-G | Primitive and absence registry rows | 0.75 | 220–400 | 3 |
| S4 | S3-N | Both SARIF modes and external inputs | 2.5 | 450–700 | 3 |
| S5-P | S4 | Primitive occurrence projection | 1.25 | 300–500 | 3 |
| S5-A1 | S5-P | Absence inventory move, native acquisition and repeat proofs | 1 | 280–450 | 3 |
| S5-A2 | S5-A1 | Remaining acquisition proofs and all absence publication | 1.25 | 300–550 | 3 |

D-6 raises the S2-W final shared-code estimate by 50 at each end and adds S2-WC.
The four publication budgets stay as listed before D-6; the proof rows give the incremental
cost for each language. These are forecasts, including tests. The existing #901 diff at
`b3bfd551` against `ae36762e` already has 1,900 raw insertions including the TSV and fold.
Keep accepted writer, matrix, tests and control history. Budget 150–250 weighted lines of new
or revised behavior for the W fold, separately recording removals/moves of superseded dormant
code and the final diff. Reuse discriminating regression fixtures at their new proof boundary.

**Owner question S6.** Should the design decision give each proof PR a fresh three-round cap?
PR #901 spent 2 of 3 rounds. Under §7 we recommend carrying round 3 to the closure slice for each
inherited obligation. Where multiple adapters must satisfy one obligation, review their parts of
the same round. Splitting a PR, running proofs concurrently or deferring a control to publication
does not reset its count. A rejection on an inherited blocker goes straight to the owner at cap
before another fold. Use fresh proof caps only with explicit owner sign-off. Activation caps in
the table cover new publication work.

| Inherited #901 blocker | Closure slice and evidence | Remaining |
|---|---|---|
| Generic representative/slot credit and unrelated insertion keys | W removes the shared inference and checks parse, operand and route key. Each affected proof below demonstrates the actual language witness. | Round 3 |
| Old visit counted by destination bytes, missing parents/children | WC records actual detector sites and both spanless controls in F. JS-proof replays recovered/refused children with real binding proofs. | Round 3 |
| Declaration, coalesced read, reaching value and effects | JS-proof (shadow, destructuring/deferred writes, escapes in identifiers), PY-proof (targets, keywords, late writes), GO-proof (formals, init, value copies/escapes), L-proof (C value and macro). Each covers non-bare, later and continued reads where supported. | Round 3 |
| Wrong callee, operand index or semantic role | JS-proof (wrappers, comments, tag/spread), PY-proof (keywords, splats), GO-proof (nested command, indices), L-proof (seven grammar layouts and both C name origins). | Round 3 |
| Inline/source provenance and continued read key | JS-proof (YAML insertion/field binding), PY-proof (Flask call and role), GO-proof (ordinary edge, originating path, no-path and secondary inline). The corresponding activations replay the same controls through finding assembly. | Round 3 |

Each language publishes **complete findings** including all its source insertions and any configured
or wrapper role on the same line. A later source-only PR could change a rules key already shipped
by an earlier language PR, failing G-additive. S2-W supplies common code without a public producer.
S3-G and S3-N add documentation/inventories behind the already enforceable registry contract.
Landing the former S2 and S3 together would combine 34 tags, 107 patterns and registry/page review
in one PR. Each language now has one complete, independently testable publication boundary.
JSON and review serialize the field directly (DESIGN 5.6).

### Slice S1: the field (0.5 day)

The code below records the landed S1 API. S2-R retains its wire shape and type path and introduces
private fields and accessors before the first producer (DESIGN 5.1 and 5.3).

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
      summary, why `contract_slice.rs` stays over 1,000, and the integrated #841 literal inventory.

### S2 and S3 gates shared by the following PRs

- [ ] Use ORCHESTRATION section 5 items 1–4 and 8–10 for each PR. Bind the RED regression and a
      negative/edge to the exact pre-slice base and re-run affected tests on the integrated head at
      `RAYON_NUM_THREADS=1` and `4`. Record commands, totals, evidence paths, exclusions and pending
      full-suite status in VERIFICATION.md. Include the behavior paragraph in `CLAUDE.md` when it
      changes and the JS/TS coverage entry in `docs/features/language-coverage/js-ts-known-gaps.md`
      in S2-JS. Follow the repository's main integration and pre-merge gate.
- [ ] Run G-additive, G-fingerprint (default mode) and G-targets on the pins for **every** row, with
      the exact scopes in Stage M. No change to a previously present key is allowed.
- [ ] For a proof PR run the real parsed-file/flow controls on the pre-slice base and head in
      addition to injected representative-order tests. Assert declaration/read/role spans and
      refusal causes. Affirmative tests at a callback alone cannot discharge publication.
      No new active partition in W, WC or a proof PR. Inspect **each final PR diff against its
      integrated base**. Any `src/ast*`, `src/cpg/`, call-graph or navigation change triggers
      ORCHESTRATION §5 item 5 on that PR, including a read-only accessor. Run Tier-A matrix-only
      before commit and quick before review. Use `--allow-stale-sut` only immediately after a
      release rebuild of the same worktree.
      A core flow modification is outside D-6 and requires a separately approved design, a cache
      bump and §5 item 6's full base/head multiset and counter differential (DESIGN 5.2).
- [ ] From S2-R onward, G-closed calls core `check_emitted` with
      the actual enclosing algorithm on **every** built-in ref returned on every matrix row and
      in every finding copy on the 35 Stage M pins (JSON, review and SARIF properties when present,
      including single-run outputs). Walk all findings, including unmarked lines. Read the same
      captured head outputs the comparator used, invoking S2-R's ignored `check_stage_m_pins`
      test explicitly in each slice gate. Record inputs, rows, findings and refs checked by
      algorithm and format, including expected empties. Require each activated producer to have
      nonempty coverage across its controls. Exercise the walker with a bad ref in a later copy,
      a wrong algorithm, and a valid neutral ref. Validate raw finding output before SARIF
      normalization can repair it. S4's synthetic external-input tests have the separate 5.5 oracle.
- [ ] G-coverage always checks the 540-key inventory in both directions and presence on every marked
      line. From S2-W, assert ids, immutable CWEs and relative severity for every activated partition,
      including admitted-empty rows. Assert empty rules on unactivated lane 7 partitions. Fixtures run
      under multiple grammars still count once per (table, label). Retain the admission and line-strength
      assertions and `#[track_caller]`. Check each new language partition RED on its own base.
- [ ] Before activating a producer run DESIGN F's joined census for its language. Attach the old/new
      identity and reason counts, sink denominator, unadmitted keys and admitted empties, and one
      location per nonzero reason. Include pre-existing discrepancies with the base observation,
      #841 overlap/expanded-only counts, lane 4 state and any unexecuted candidate or grid cells. The 300 candidate decisions
      select extra positive/negative controls. Check base admission before asserting a new example.
      Run F's D-6 join with per-channel rescues. Report after-reduction losses against the
      keyed D-5 E values for labels and F's independently reviewed D-5 references on pins.
      Give the comparable denominator, unresolved refusals and the S5 trigger result. Replay the changed
      candidate on the exact base before JS activation. If its admission differs, report the
      discrepancy separately. Do not weaken a different E cell to get a producer through its gate.

### Slice S2-R. Minimum taint registry

**Files.** Carry the registry, A/B rows and behavioral tests from the rejected head's
`crates/prism-review/src/output/rule_registry{.rs,/}` into core `src/rule_registry{.rs,/}`.
Update `src/{lib,slice}.rs`, review's `output/mod.rs` re-export, the classifier's
`algorithms/taint_sinks/{mod,hit,legacy,strength}.rs` and its field readers/tests. Use Stage T
test support and review's `output/rule_pin_tests.rs` for the gates. In `CLAUDE.md`, replace the
rejected head's `output::rule_registry` ownership paragraph with an entry in the core module list
covering the private fields, accessors and review re-export.

- [ ] **Measure first.** On the integrated head inventory ref/row constructors, field readers and
      writers, classifier callers and all 64 complete row tuples. At `e53d01c8` the maker is the
      only production ref literal and S1 has two test literals. The three registry files total
      525.5 weighted lines, including 321.5 from the first scanner test to EOF. Budget 300–450
      weighted lines from S1 and one day. About 204 weighted lines of registry, rows and behavioral
      tests survive the deletion. Allow for accessors, the classifier move, consumer and S1 test
      migration, compile-fail and serde controls, and the Stage M pin reader. Report file
      moves and scanner deletions separately from new code. Recount after any independent row fold.
- [ ] Move the implemented rows as data, preserving each symbol, id, optional CWE, text, algorithm
      and sorted membership. Compare all six fields of every row before/after. Keep lane 4's
      conditional row for its first producer. Delete the source walker, exemption logic and scanner
      fixtures. The core registry owns both types. Preserve `prism::slice::RuleRef` by re-export
      and the review registry path by `pub use prism::rule_registry` in review's output module.
      Pass this ownership and the direct core import to lane 8 before I-9 (Global constraints).
- [ ] Make all fields private. Expose borrowed `id() -> &str`, `cwe()/severity() -> Option<&str>`
      on refs, corresponding read accessors for row metadata, and the 5.3 maker and severity
      method. Preserve ref derives, serde field order, optional defaults/omissions and the public
      finding vector. Use one declaration per row for the named constant and its table entry.
      Keep row construction within the registry tree. Test both type paths, all named rows against
      lookup by value, applying and clearing severity, and the sorted inventory. Preserve the existing CWE
      grammar, algorithm and text controls in `cargo test -p prism --lib rule_registry`.
- [ ] Use two controls. The rejected product head `e53d01c8` supplies the implemented registry and
      the pre-privacy API. The integrated main base supplies Stage M byte comparisons. At the D-4
      branch point (`1cbc5400`) main has S1 and lacks `RuleInfo`. A missing type there cannot
      demonstrate a privacy failure. Record both control SHAs in the dispatch and receipt.
      On the rejected head, using its public paths from a consumer, compile a computed
      ref literal inside `vec![]` through a type alias, assign id and CWE on a cloned ref in separate
      examples, and construct `static BAD: RuleInfo` with struct update from `JS_EVAL`. Record
      successful compilation on `e53d01c8` for each control. If the independent private-row fix
      lands first, attribute the forged-row failure to that fold. Keep separate compile-fail
      doctests (review-path examples in review). On the fold inspect the privacy error for each
      expression and pair with compiling use of the same imports, a valid named row and its getters.
      Add two independent `compile_fail` doctests using `prism::slice::RuleRef`, one calling
      `RuleRef::default()` and one evaluating `"x".parse::<RuleRef>()`. Check the missing
      `Default` and `FromStr` diagnostics respectively. Both traits are already absent on S1.
      Run `cargo test -p prism -p prism-review --doc`. A missing import, malformed fixture or
      uncompiled doctest does not satisfy any control.
- [ ] Move `SinkHit` and its impl to a leaf `taint_sinks::hit` module. Keep its fields private,
      re-export the type at its existing visibility, and expose the existing classifier factories
      and grading operation only within `taint_sinks` (`pub(super)` on `new`, `weak_if` and
      `LEGACY` in the leaf). In `taint_sinks/legacy.rs` replace main's literal with
      `SinkHit::LEGACY` as on the rejected head. Supply read accessors at each caller's visibility.
      Check legacy hits still return `Strength::Strong` with no CWE, and run the classifier grading tests.
      Drop the sibling-literal compile-fail receipt. The type is crate-private and its classifier
      siblings can still call the factories. Review descendants of the leaf as privileged.
      Public rule metadata comes from the core rows.
- [ ] Through the existing public ref path, deserialize an unknown id with and without optional
      fields, a registered id with the wrong CWE, and a neutral id with an extra CWE. Rewrite the two
      literals in S1's `slice::tests::rules_serialize_as_id_and_optional_cwe` as
      `JS_DOM_HTML.rule_ref()` and `LEGACY.rule_ref()` from core. Retain the exact byte, omission
      and round-trip assertions. Run `cargo test -p prism --lib slice::tests`. Feed those inputs,
      a missing required CWE and a valid ref under the wrong algorithm to `check_emitted` and
      verify each expected rejection. Exercise the matrix and pin readers with known good/bad
      synthetic values now so zero producer refs cannot make their controls pass vacuously.
- [ ] In `output::rule_pin_tests`, run the synthetic reader controls in ordinary CI. Mark only
      the host capture test `check_stage_m_pins` with `#[ignore]`. From the workspace root, set
      `input_root` and `head_out` to the absolute paths for the pinned input tree passed to
      `run_outputs.sh` (its `inputs.tsv`, header `name`, `repo`, `diff`, is the manifest) and the captured head outputs.
      Invoke the slice gate with
      `PRISM_RULE_PIN_MANIFEST="$input_root/inputs.tsv" PRISM_RULE_PIN_INPUT_ROOT="$input_root" PRISM_RULE_PIN_OUTPUTS="$head_out" cargo test -p prism-review --lib output::rule_pin_tests::check_stage_m_pins -- --ignored --exact --nocapture`.
      Require that exact test to run. Fail on missing manifest, capture, input or expected format.
      Reconcile all 35 input entries and report the shared-gate per-format totals, including zero refs.
- [ ] Shared gates with empty scope. Require byte equality on all Stage M outputs after only the
      existing per-build masks. Record zero emitted refs and the Stage T presence/key inventory.
      S2-R has no producer. The page and cross-detector inventory belong to S3-G.

### Slice S2-W. Collection and matrix support

**Files.** Core `src/slice.rs` and `src/slice/attach_rules.rs` for the writer and its tests,
`algorithms/taint.rs`, `taint/{run,sink_match,sink_rules}.rs` (under `crates/prism-review/src`),
Stage T's support, expectation, identity and TSV files, and `tests/algo/taxonomy/taint_sink_strength_test.rs`.
Retire the diagnostic that classifies a detector parent by the edge's operand bytes. Carry its
raw-visit fixtures into S2-WC, which implements the complete census before any activation.
Defer #901's `src/ast/js_binding_site.rs` and `src/ast/rvalue_identifiers.rs` additions to the
first proof PR that needs a core query. W should have no final diff in those paths. If either
helper stays, W itself takes Tier-A matrix-only and quick. Recheck the trigger on the consumer PR.

- [ ] Preserve #901's transcription of all 160 reviewed TSV cells by exact key, including expanded
      `observed =` values wherever the concrete expectation changed and all explicit observations. Replay
      the Stage D base controls. Reject residual `?`, missing/duplicate keys or silent empty selection.
      Use the fixture's language for mixed-table `ri*` rows. Run `cargo test --test algo_taxonomy taint_sink_rules`.
- [ ] Replace the generic `located_uses` expansion and generic `call_operand` index in #901 with
      inputs from the language adapters. Remove inference from adjacency, enclosing declaration
      spans, counts of bare arguments and shared representative bytes. Keep NameSinks origins and
      the exact admission iteration order. The common input carries final finding key, parse/owner
      identity, operation and callee/target span, reached read/source and operand span, role,
      detector/entry and path or keyed inline provenance. A language can return no witness.
      Use parse-tied/private construction. Verify file/tree, range, role containment, callable and
      route-specific line on receipt; retain each path for its guard. Pass selected named rows and
      strength to the existing writer only after all channels join. Check unrelated keys/trees,
      same-call wrong slot, duplicate paths, empty admission and continuations where the insertion
      key differs from the read line. Exercise keyed inline evidence and Go originating-path
      evidence separately. Reject a read in a sibling call outside the selected operand.
      Preserve #841's selected variant.
- [ ] Retain #901's `prism::slice::attach_rules(&mut SliceFinding, rows)` as the single built-in writer.
      Group by row id, reduce strength (any strong wins), sort by id and assign the vector once.
      Call `row.rule_ref().with_severity(override_value)` per id, setting `suggestion` only for
      an all-weak id on a warning. Test empty input, duplicate weak/strong rows in both orders,
      unsorted distinct rows and an all-weak id at warning and suggestion levels. S2-JS/PY/GO/L,
      S5-P and S5-A2 each pass the complete selection to this helper once per published finding.
- [ ] Shared gates, empty scope. Keep all 540 presence controls and zero activated partitions.
      Exercise the shared collector/reducer via tests before any public activation. Pin evidence
      and the baseline/expanded union choice from merged #841.

### Slice S2-WC. Old detector visits and the joined census

Extend the test-only `sink_rules/old_visits.rs` observer at actual old selection sites in
`taint/{run,sink_match,js_ts/sinks,python/sinks,go/sinks}`. Preserve first-outcome order.
Observe a parent's real identifier/operator span or selected call's callee span on flat,
structured, configured/wrapper, JS YAML, Flask and both Go paths. Distinguish Go command
table entries with the same spelling. Record raw visits and missing operand bytes independently.
Exercise zero/one/two unlocated detector visits separately from missing destination spans. On the
actual #901 `audit(x)` visits, with its exact tainted formal, require one located parent, two raw
visits, two missing operand spans and one proved child (`configured-sink`). With unprovable source
provenance, keep the same admission, parent and visit counts and require zero proved children and
an empty final list. WC exercises the join with supplied child evidence. S2-JS-proof must
produce those children from the parsed fixture and paths. Join F's parent, operand, path/inline
and final-publication populations. On the 35 pins record the evidence for F's D-5 references
and report the single-mention/multiple-reaching-definitions bucket separately. Show unavailable
adapter decisions as pending. Rerun with the respective proof before activation and apply F's
owner threshold (proposed default ≥1 distinct unrescued admitted key across the pins).
Keep test instrumentation off in both output-gate binaries. Run shared gates at 0/540 active.

### Dormant proof PR before each language activation

Use a projection module and fixture tests per language in `taint/sink_rules/`. Factor a small
read-only core query at the first consumer only if positive facts are otherwise inaccessible;
keep its owner indices, declarations and AST nodes tied to the same parse. Each adapter implements
the three proof routes of DESIGN 5.2, with a closed grammar inventory (including unknown/parse-error
fallback), a callee/target extractor and a semantic operand binder. Show why an unclassified
effect cannot pass. Compare binding at the exact read to the def token and test the old value
of a read-modify-write separately from the resulting store. No edge/confidence/cache mutation.
Before each affected proof fold, enumerate the pinned grammar's binders and shadows, deferred and
destructuring writes, escapes, read/non-read roles and callee/argument layouts. Mark each supported
or refused with a control. Test old and semantic argument checks on the same child. Each proof PR
takes the empty-scope gates. JS/PY/GO proofs may run concurrently on WC (0/540 active). L-proof
starts after Go activation (474/540). Recheck against the actual integrated base at landing,
preserving every active key there. Only activations raise the count, in JS → PY → GO → L order.

- **S2-JS-proof. Files.** Use `taint_sinks/{js_binding,js_scope,callee}.rs` for lexical and
  call-position facts, and `src/ast/js_write_census/` (including `alias_cert.rs`) for binders,
  destructuring, writes and alias hazards. Prove token identity and initialization before each
  evaluation, including within one statement. Include JS/TS/TSX immutable locals, normal parameters and block/catch/for shadows
  (including an inner def inside a scope-wide binder span), all five Opus destructuring writes,
  mapped arguments/eval, closure/class field/static writes and uncertain escapes. Reverse the
  representative and assert identities of the *actual reads* at nested, non-bare, continued and
  third-statement uses. Check asserted member paths, unique versus repeated request ranges,
  computed/indexed callee refusal, constructors, sequence and static wrapper forms. Test both
  comment-shift directions, tag substitutions after slot 0 and unknown-width spread. In the
  nominal `res.send` fixture, `const x=i.v; const s='\n'; res.send(x)` retains `js-response@CWE-79`.
  Substitute an inert regex such as `/\d/` and require the same result. For a read of `\u0078`
  whose identifier binding the adapter cannot normalize, require refusal of the dependent proof.
  Replay both WC `audit` controls from their actual formal/provenance and edges.
- **S2-PY-proof. Files.** Use `taint_sinks/py_scope.rs::Sites` for the binding-site inventory and
  `taint_sinks/callee.rs` for call/receiver facts. Resolve scope, reaching value and evaluation
  order at the particular read. Inventory formals and assignments including nonlocal/global,
  comprehension and closure scopes, pattern targets and `del`. Check keyword name versus value, Python ellipsis,
  positional/keyword permutation, splats and compound writes. Three nonadjacent reads and a
  continued call must each carry their own operation and original indexed operand. A preceding
  prefixed literal such as `s = r'\n'` preserves the control's `os.system(x)` identity. Inspect
  formatted-string replacement expressions. Exercise F's conditional-default single-read refusal
  and a direct dominating definition at the same sink.
- **S2-GO-proof. Files.** Use `taint_sinks/{go_scope,callee}.rs` for formals, init scopes,
  receiver/argument and callee facts. Prove the exact declaration, RHS/read order and slot
  exposure. Normalize formal/body and statement-list scope via that declaration identity.
  Cover shadowing, `:=` reuse versus declaration, init scopes, parallel values, address-taking,
  capture and self-write. Verify non-bare arguments, comments, generic callees, start-line and
  source-operand ownership. On `ri15` the path guard applies to the file, leaving the format.
  On `exec.Command(x).Run()` retain the inner executable read. `f(&T{N: x})` copies the value
  of `x`, so a subsequent `exec.Command(x)` still qualifies. `f(&x)` exposes its slot and defeats
  that proof. Repeat the clean control with an inert Go raw string containing a backslash.
  Exercise `go: exec.Command`, `hybrid_go: aliased os/exec`, `hybrid_go: exec.CommandContext`,
  `hybrid_go: probe: go aliased osexec.Command` against their individual E rows, plus the
  separate `exec.Command` warning in `taint_sink_strength_test.rs`. Check the selected inner
  operand, row/strength where applicable and the four Go routes in S2-GO below.
- **S2-L-proof. Files.** Use the legacy projection and grammar fixtures in `taint/sink_rules/`.
  `taint_sinks/callee.rs` covers JS/Python/Go. Extract the legacy callee and operand from each
  grammar's own fields and prove the token, order and storage effects for supported locals.
  Under DESIGN 5.2's D2 macro contract, the repo macro census finds no definition for undeclared
  `audit_payload` in `ri23c`. The modeled function receives a copy of the automatic pointer.
  `audit_payload(x); report(x);` therefore proves both configured and wrapper origins. Contrast
  an in-repo macro that writes its argument, an incomplete census, `&x` and an intervening store.
  Withhold any dependent role whose value or call cannot be proved. For each
  remaining grammar exercise supported configured/wrapper forms and refusal on an unresolved
  binder, dynamic callee or ambiguous slot. Cover qualified C++, Java creation, Rust scoped
  generics, Lua's call form, HCL argument container and static/dynamic Bash command words.
  The line-based legacy scan remains independently testable with no operand proof.

  L-proof allocates 25–45 weighted lines to shared dispatch and controls, 60–95 to C,
  30–60 to C++, 25–45 to Java, 35–55 to Rust, 20–40 to Lua, 25–50 to HCL and 30–60 to Bash.
  The 250–450 total includes fixtures. Reforecast each grammar on the dispatch base. If the upper total exceeds 450,
  split by grammar before dispatch, keep C's two `ri23c` origins together, and carry the
  inherited round into each affected part. S2-L waits for all parts before publication.

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
      range filter, all config/wrapper overlaps and lane 4's side-set migration if present. Route
      YAML and any side-set rows through the same typed attachment as ordinary JS sinks.
- [ ] Consume S2-JS-proof's read and operation evidence for flat and structured projection.
      Gate on both orders of `ri02`, nested `res.send(eval(x))`, continued `send(prefix + x)`,
      indexed comment disagreements, JSX `__html` versus a plain object and wrong-key inline YAML.
      On admitted `astra-r1-47` expect empty, on a single eligible request-field access retain
      `js-location-header`. Record the D-5 → D-6 change and positive control in F.
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
- [ ] Wire S2-PY-proof through all path and inline branches. Assert `ri20,21a,21b,22` through
      normal finding assembly with old-index, per-call guard and exact read controls. Compare the
      old visit and F parent for a multiline Flask call. An unrelated inline call in the same file
      and a name in `audit(x="fixed")` supply no role. Exercise both grades in `astra-r1-01`.
- [ ] Test B's four Flask shapes on edges and inline request values. Bind positional/keyword
      source despite reordering; refuse duplicate or unresolved splats, unused context and outputs
      after endautoescape or inside nested true. Derive the projection's context exclusion from
      the bound source and scoped use. On an eligible edge assert the two `@CWE-79` refs `py-html`
      and `structured-xss` for each of `render_template_string("{{ name | safe }}", name=x)`,
      `render_template_string(source="{{ name | safe }}", name=x)` and
      `render_template_string(name=x, source="{{ name | safe }}")`. Check inline-only context
      yields just the structured ref on admission. Retain the source-operand pair of template ids,
      the source-line flat exclusion, a real admitting sibling and an unadmitted control.
      Use S2-W's named-row reducer for both edge and inline contributions. Keep the first-outcome
      admission semantics.
- [ ] Shared gates with `finding:taint`. G-coverage adds py + hybrid_py (147) and
      `ri04,06,08,12,20,21a,21b,22` (8), **360/540** cumulative. Re-record only the two copies of
      the Python sink in the review golden, with the decision 7 reason and exact diff in the PR.

### Slice S2-GO. Go

**Files.** `taint/{run,sink_rules,go/sinks}.rs`, Go projection module, go/hybrid_go/identity and
sink-strength tests. Read the existing source table and exact receiver/argument bindings.

- [ ] Cover 17 ordinary and two framework patterns, context offsets, shell/ordinary argv, writer
      and format slots, typed Gin, opaque DB methods, weak SQL on a warning, generic/continued
      callees and Go's call-start limit. A later successful path cannot hide a live sibling.
- [ ] Wire S2-GO-proof through edge, primary and secondary admission. Assert a formal used
      in a body statement, both `ri15` representative orders, distinct source bindings for
      parallel RHSs, wrong sink slot and both same-name `exec.Command` table entries in F.
- [ ] Check the four route keys. An ordinary edge nominates its destination line. On the
      originating path in `x := c.Param("f"); os.Remove(` followed by `x,` on the next line,
      assert the later read/slot and `structured-path-traversal@CWE-22` at the admitted opening
      line. Repeat with a cleansed path and with the read in a sibling call outside `os.Remove`.
      Neither witness supplies the path-traversal id. On no-path admission require a bound inline source in the selected
      operand for credit. On secondary inline fallback anchor the source to the actual call
      and admitting key. Exercise the same-line read as a positive control.
- [ ] Union every eligible originating path and inline witness on a source line. In *both* inline
      routes test a mux.Vars argument resolving to the handler request (including a unique alias),
      a fresh request, a reassigned parameter, another binding and a shadow. Test source-only, incoming-only and mixed
      provenance, wrong sink slot and shape-only admission. Preserve the independent path contribution
      and the established flat-source exclusion. Include the bound `astra-r2-15` shape. Trace
      both inline routes to named core rows in the shared G-closed review.
- [ ] Shared gates with `finding:taint`. G-coverage adds go + hybrid_go (109) and
      `ri05,05b,11,15,25` (5), **474/540** cumulative.

### Slice S2-L. Legacy and remaining name-only fixtures

**Files.** `taint/{run,sink_rules}.rs`, the legacy projection and Stage T's other, hybrid_other
and identity tests, CLI output tests and `CLAUDE.md` (taint paragraph).

- [ ] Use S2-L-proof for the two name origins. Assert `ri23c` emits configured and wrapper
      once in either representative order, `ri23d` also selects its independent legacy token,
      and an admitted shadow/escaped-pointer control retains only independently supported rows.
- [ ] Enable the remaining grammars together. Match the legacy token on the admitted physical line.
      For config/wrapper project only a reached argument to its matching callee, with separate exact
      and substring predicates. Check duplicates, overlapping built-ins, unquoted expansion, source,
      a wrapper name with a clean sibling, `ri27` and the Java non-admission. JS/PY/GO must already
      handle these name origins on their own lines. Select the three named rows directly for
      legacy, configured and wrapper hits. A configured callee's spelling is never a lookup key.
- [ ] Shared gates with `finding:taint`. G-coverage adds other + hybrid_other (53) and
      `ri09,10,14,16,17,18,23a–23f,27` (13), **540/540** cumulative. Reconcile the per-language
      censuses, the 300 candidate dispositions and the two open grids, naming unexecuted cases.

### Slice S3-G. Registry completeness and generated page

**Files.** Core `src/rule_registry/tests.rs`, review-side `output/rule_inventory_tests.rs` and
its test module declaration, `docs/rules.md`, `scripts/ci/classify-changes{,.test}.sh` and
`CLAUDE.md` (extend the core module list's registry entry with the page and its review-side inventory check).

- [ ] Read `prism::rule_registry::rules()` through its accessors in the review-side test. Generate
      id, optional CWE, name, algorithms, short and help in id order, with the regeneration
      command in the header. Test `rules_page_is_current` and its controlled `PRISM_BLESS_RULES=1`
      regeneration. Classify the generated page as code before the generic docs case and test it.
      Cross-check all 34 flat tags and all 107 structured entries including their role splits against
      A/B. Confirm neutral rows have no CWE and the legacy text matches 5.3.
- [ ] Shared gates, empty scope. Run the all-output G-closed test on all 540 active keys and the
      35 pins, including aggregate copies. Check each actual taint ref against its finding's algorithm.
      Run `cargo test -p prism --lib rule_registry`, `cargo test -p prism-review --lib output::rule_inventory_tests`
      and `bash scripts/ci/classify-changes.test.sh`. Report the page and output checks separately.

### Slice S3-N. Register the non-taint rows

**Files.** Core `src/rule_registry/{primitive_rows,absence_rows,tests}.rs`, `src/rule_registry.rs`,
review-side `output/rule_inventory_tests.rs` and the generated `docs/rules.md`. No algorithm emits here.

- [ ] Register C's six category ids with their fixed CWEs plus six literal `_MATCH` ids without CWE
      (algorithm primitive). Register D's 65 primary missing-counterpart ids, 22 additional `-match`
      rows and three repeat ids (algorithm absence). Of the 65, 22 have the verified CWE and 43 have
      none, including `absence-connection` and `absence-go-sql-db`. Repeat rows are double-free/415,
      double-unlock/765 and repeated-counterpart without CWE. Register each literal once.
- [ ] Keep `rules()` sorted across A–D and test lookup of every row through the public core API.
      Compare membership to C and to all 65 `default_pairs()` descriptions and the D fallback map
      on the review side, which owns the detector inventory.
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
// prism/<algorithm>/<category>. `specific` uses prism/<encoded algorithm>/<rule id> with optional CWE
// tags, or prism-fallback/<encoded algorithm>/<category> for an empty list. In the algorithm segment
// each % becomes %25 and each / becomes %2F.
// Switching modes can start new code-scanning alert histories for the affected findings; how
// GitHub shows the CWE tags is unverified."
#[arg(long, value_enum, default_value_t = SarifRuleIds::Category)] pub sarif_rule_ids: SarifRuleIds,
```

Add `ResultProperties.rules`, optional descriptor help/category and optional tags, skipped when
empty. After admission and #841's map/diagnostic choices, normalize incoming references by DESIGN
5.5 before constructing **any** result properties or specific descriptors. Resolve supplied ids with
core `lookup`. For an authorized row call `row.rule_ref()`, apply the reduced severity and run
`check_emitted` on that canonical ref. Drop registered ids under any other algorithm.
Group duplicates using the strongest effective severity and sort by id. Category mode uses this
canonical list with the ordinary category descriptor. Specific mode copies this same full list on
each result, selects its level from the canonical ref and uses the authorized row's optional tag.
If the list is empty on entry or after normalization, omit the rules key and use the ordinary
category id in default mode. Both specific-mode forms use the same algorithm-segment encoder
(DESIGN 5.5). Append the verbatim rule id or category after the delimiter. For an empty list emit
one finding-level result with `prism-fallback/{encoded algorithm}/{category}` and a tagless category
descriptor. Use one serialize-only rule property type private to review's SARIF output.
It has the 5.1 wire keys and omits absent
optionals. Fill it from the canonical ref's accessors for a known row. For an unknown external id,
copy the supplied id verbatim into the property and fill its CWE and severity from the group
reduction in 5.5. Build the descriptor tags from the per-(algorithm,id) run-wide union. The
property has no conversion to a producer ref. Keep deserialization at input and avoid a core
id/CWE setter, serde round-trip or scanner exemption in the normalizer.

Compute message, evidence, fingerprint and locations once. A registered specific descriptor uses
`name`, `shortDescription = short`, `fullDescription = help`, `help` and algorithm/optional tags.
It omits category. The fallback prefix keeps empty results distinct from rule-bearing ones. Unknown ids
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
- [ ] Deserialize the external fixtures through `prism::slice::RuleRef` and exercise public
      `SarifInputs`. Check the input refs after rendering and the serialized property keys and
      omitted optionals in both modes. Pass `js-eval@CWE-502` on taint alone and beside valid rows.
      Every copy carries `js-eval@CWE-95`, and its specific descriptor has only the 95 tag.
      Supply `js-eval` on absence alone and beside a valid absence id. Assert its absence from
      properties and specific descriptors, the mode-specific empty fallback, and the surviving valid id.
      Also supply a CWE for a neutral row, omit one for a tagged row, reverse duplicate severities
      (including absent), and reverse conflicting unknown CWEs. Check indexed descriptors, result
      levels and the full canonical list in each mode. Category descriptors gain no tags.
- [ ] In `specific_mode_escapes_algorithm_in_both_namespaces_in_both_orders`, use the default
      floor and submit external `(algorithm, id, CWE)` tuples `(taint, vendor/check, CWE-79)` and
      `(taint/vendor, check, CWE-22)` together in both orders. Follow each result's
      `ruleIndex` and assert its descriptor id equals its `ruleId`. Verify `prism/taint/vendor/check`
      has only the 79 tag and `prism/taint%2Fvendor/check` only the 22 tag. Check the raw
      algorithm and id in result properties. Repeat with empty rules using those ids as categories.
      Expect `prism-fallback/taint/vendor/check` and `prism-fallback/taint%2Fvendor/check`
      with distinct indices, matching raw algorithm/category, no tags and no rules key. In both
      namespaces include the literal-percent control `(taint%2Fvendor, check)` and check that
      its `taint%252Fvendor` segment yields a third descriptor.
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
      (`sarif:taint`), G-targets, all 540 G-coverage keys, G-closed and G-sarif-valid. For
      G-closed, also capture the specific-mode output on the Stage M pins and run the same
      per-ref check as in category mode. Two reviewers, cap 3.

### Slice S5-P. Primitive projection

**Files.** `algorithms/{primitive_slice.rs,primitive_slice/projection.rs}`, `tests/algo/novel/{main,rule_identity_test}.rs`
and the primitive tests. Reuse the twelve S3-N rows. All three S5 PRs use the section 5 product gates,
the RED/base control, both test thread counts, all 540 G-coverage keys, the M comparator and the
shared per-ref G-closed check on the pins. Include both SARIF modes once available.

- [ ] RED/GREEN `cargo test --test algo_novel rule_identity_test::` and the affected primitive and SARIF
      suites. Make each existing emitter pass its site to the projection. Select the category id with
      C's CWE on proof, else its literal `_MATCH` row without CWE. Return a named core
      `&'static RuleInfo` on each branch and pass the single row to `attach_rules` at the finding's
      own severity. Preserve category, description, severity and multiplicity.
      Give each C row a genuine tagged positive and an **admitted** neutral
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
      exit on D's category-specific path. Return a `&'static RuleInfo` for the S3-N verified
      constant or its CWE-less fallback.
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
      neutral rows against D. Pass the single selected row to `attach_rules` at the finding's own
      severity. Other algorithms remain empty.
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

- [ ] Record the Stage D approvals and the owner's §8 authorization to resume #901 after the
      split verdict on D-6 and this D-7 fold. Each Stage I PR has its own
      integrated-base receipt, sized diff, assigned approvals and review-round count. All four
      producer partitions pass Stage M's algorithm-scoped G-additive check. G-coverage verifies
      the language boundary on active and inactive matrix fixtures. Previously published rule values,
      every fingerprint, target id, finding and its severity pass the comparator.
      The cumulative check against the pre-lane base passes with the six stated scopes.
- [ ] G-coverage progresses through 205, 360, 474 and 540 active keys. All 540 assert admission and
      all positives assert ids/CWEs/relative severity, including empty references. No residual `?`.
      Record candidate/grid execution separately. F's pinned censuses reconcile all stages and list
      exclusions. Every production id was registry-backed at its first publication.
- [ ] G-closed checks the A–D inventory, compiler privacy and positive serde controls, and the
      current generated page. Repeat the all-output test on all 540 keys and all 35 pins with
      taint, primitive and absence active. Include every finding copy and both SARIF modes.
      Report per-algorithm totals and trace each built-in producer to its named rows and the
      single `attach_rules` writer. C/D projection tests exercise effective protections, closed
      credential slots and path-specific
      cleanup, including admitted neutral occurrences. Both SARIF modes agree on normalized result
      properties. Verified specific descriptors retain their tags and empty results index a tagless fallback.
      The final mixed-output shape and schema receipts meet G-sarif-valid. Run and record the full
      suite before merging to main, including any exclusions under the repository policy.

**Effort estimate.** M 1, T 2.5, E 1, D 1.5, S1 0.5 (during T), S2 12.5, S3 1.5, S4 2.5,
S5 3.5 and close-out 0.5 give 27 days of work. D-6 added 5.75 workdays to S2. With the JS,
Python and Go proof PRs staffed concurrently after WC, S2 takes 10.25 elapsed days and the lane
critical path is 24.25 days. Serial staffing gives 26.5. These estimates exclude review queues
and elapsed host-gate time.

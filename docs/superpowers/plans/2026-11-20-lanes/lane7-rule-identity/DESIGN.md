# Lane 7: rule identity on findings — design

Status: Stage D design of record, 2026-11-24. S1 is present. Sections 5.2–5.4 and Appendices A–F
specify the identities for the implementation slices. Historical research record:
`~/prism-evidence/lanes-2026-11-20/research/lane7-rule-identity.md`.

## 1. Intent

A finding should say which rule produced it and, where one exists, which CWE it is an instance of, in every
output: JSON, review, SARIF. Today the taint sink classifier knows that one line is an HTML injection
(`js-dom-html`, CWE-79) and another a command execution (`js-child-process`, CWE-78), and then discards it:
both come out as the same `taint_sink` finding and the same SARIF rule. Code-scanning users cannot filter,
suppress or triage by weakness class, and the work that went into the sink rule tables is invisible.

The change must not disturb existing consumers: no change to finding counts, target ids or SARIF
fingerprints, and no change to SARIF rule ids unless the user asks for it. Default JSON, review and SARIF
output does change in one way: a finding that has rules gains a `rules` key (decision 7); a reader that
rejects unknown keys sees that. A published rule never carries an identity known to be wrong (5.2).

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| `SliceFinding` has eleven fields (algorithm, file, line, severity, description; and, skipped when empty: function name, related lines, related files, category, parse quality, diagrams). It is not `#[non_exhaustive]` and has no constructor or `Default`. There are 27 production struct literals (contract 12, absence 4, taint 3, callback dispatcher 2, one each in primitive, provenance, echo, symmetry, membrane, peer consistency) and about 23 in tests. The public API documents it as a handle to read and pass back. | `src/slice.rs:23-44`; `src/api/` module docs |
| The taint sink classifier returns a rule tag and a CWE per hit (34 tags: `js-dom-html`/79, `js-child-process`/78, `js-eval`/95, `py-template`/1336, `go-format-string`/134, …; other languages get `legacy` with no CWE). Nothing outside its own tests reads them: the sink loop classifies every identifier on the edge's target line, turns a hit into a boolean and records only the line, so a sink call that does not take the tainted value still admits the line. Structured sink tables carry a category (six values: XSS, SQL injection, SSRF, deserialization, OS command, path traversal) and no CWE as data; each language's matcher returns the first matching pattern. Some table placements are not the weakness of the operation: JS `eval`, `Function` and the `vm` runners sit in the deserialization table, Mongoose `$where` in the SQL table, and the structured `exec` command pattern also accepts a database handle's `db.exec(sql)`; the flat tag of Python `session.exec(stmt)` (SQLModel) is `py-eval`/CWE-95 (#646). A line can hold several hits. | `crates/prism-review/src/algorithms/taint/run.rs:240-277`; `taint_sinks/`; `taint/js_ts/sink_tables.rs:322-382`; `taint_sinks/js.rs:335-343`; `src/frameworks/mod.rs` |
| Other algorithms: primitive already stores its rule name in `category`; absence's pair identity (65 pairs) and provenance's origin live only in the description text; contract kinds are a category suffix. | research section 2 |
| SARIF: `ruleId` is `prism/{algorithm}/{category}`; one rule descriptor per distinct id, with id, name, descriptions and `properties {algorithm, category}`. No tags, no help text, no taxonomies. The stable fingerprint hashes algorithm, category, file, function name, the description with digit runs masked, and the source line's text; the rule id is not an input. An earlier design note records "rule ids keep category strings verbatim; stability wins". | `crates/prism-review/src/output/sarif.rs:377`, `sarif_rules.rs` |
| Targets: the row id hashes category, description and severity among nine fields; the schema is closed, so a new row property needs a new contract minor. | `crates/prism-review/src/targets/mod.rs:279` |
| JSON and review output serialise the struct, so a new field that is skipped when empty appears automatically. The category strings `taint_source` and `taint_sink` are read by the review renderer, the confidence classifier and the targets mapping. | `crates/prism-review/src/output/review.rs:164` |
| Pinned by tests: exact SARIF rule ids and fingerprint presence; the targets id recipe and schema; review-compact key presence. | `sarif_test`, `sarif_shape_test`, `targets_mapping_test`, `review_compact_test` |

Inferred from GitHub's documented behaviour, not from this repository, and **not verified by this lane**:
code scanning takes CWE labels from a rule descriptor's `properties.tags` (`external/cwe/cwe-79`), and
identifies an alert by rule id together with its own location hash, so changing a rule id starts a new alert
history (former category alerts may close as fixed; two specific ids at one location are two alerts). Also
unverified: whether the unpadded `cwe-79` displays like the common zero-padded `cwe-079`, and whether a
separate `security` tag is needed for the filters. The check a human with upload rights would run: upload a
`category`-mode log, then the same analysis in `specific` mode under the same analysis category, with one
two-digit and one four-digit CWE, and record which alerts close, which open and how each CWE shows; then the
`specific` log under a second analysis category. Until then the documents and the flag's help say these
points are unverified.

## 3. Goals, non-goals, anti-goals

Goals:

1. A finding carries zero or more rule references (rule id, optional CWE, optional per-rule severity).
2. A taint finding carries the rule of every sink occurrence on its line whose operand the tainted value
   reaches, flat and structured (5.2), and only those; the rule's identity is the one stage D fixed from the
   operation and the operand. (Under PR #841's union, a line found by both runs keeps the baseline run's
   rules.)
3. SARIF output can present one descriptor per rule with CWE tags and help text, on request; by default it
   keeps today's rule ids and adds the rule references to each result's properties.
4. One registry lists every rule id with its CWE, name and description; production code can name a rule only
   through it, and every emitted reference agrees with its row (5.3); a generated page documents it.
5. Fingerprints, target ids, finding counts, severities and default SARIF rule ids are unchanged, and default
   JSON, review and SARIF output equals main's once the `rules` keys and the per-build keys are removed
   (decision 7), proven by comparison with main on fixed inputs.

Non-goals: new sinks or new detections (a corrected identity for an existing detection is in scope, 5.2); CWE labels invented for algorithms that have no weakness class (contract,
echo, membrane, symmetry, peer consistency, provenance); `security-severity` scores; a targets contract
change; changing categories or descriptions.

Anti-goals: splitting one finding into several; putting the rule in the description or the category (both
feed fingerprints and target ids); a second place where rule metadata can drift from the code.

## 4. Approaches considered

**A. Encode the rule in `category`** (`taint_sink.js-dom-html`). No struct change, and SARIF rule ids would
follow. It changes every taint fingerprint and target id, breaks the three readers of the literal category
strings, and cannot express two rules on one line. Rejected.

**B. Encode it in the description text.** Changes fingerprints (a rule name is not masked) and makes
consumers parse prose. Lane 4's design used this as a stopgap because no field existed. Rejected.

**C. A new optional field on the finding, plus a registry** (chosen). `rules` is a list, empty for findings
without a rule and skipped in serialisation when empty, so findings without rules serialise as today.
SARIF gets a rule-id mode: the default keeps `prism/{algorithm}/{category}`; `specific` uses the finding's
rule.

**D. One finding per (line, rule).** Natural for SARIF, where a result has one rule, but it changes finding
counts everywhere and duplicates target ids. Rejected; SARIF's `specific` mode expands a multi-rule finding
into one result per rule at serialisation time only.

## 5. Architecture

### 5.1 The field

```rust
// src/slice.rs
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RuleRef {
    pub id: String,                                   // e.g. "js-dom-html"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwe: Option<String>,                          // e.g. "CWE-79"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,                     // only when lower than the finding's (5.2)
}
pub struct SliceFinding {
    // existing fields unchanged
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleRef>,                          // sorted by id, at most one per id
}
```

`SliceFinding` gains `Default` and stays constructible by struct literal; the 27 production literals and the
test literals add `rules: Vec::new()` (or `..Default::default()`), in a first commit that changes nothing
else. The field is additive under the API's handle promise.

### 5.2 Taint emission

**Admission and attribution have different inputs.** Keep the existing `sink_lines`, first-outcome
priority (including special early exclusions), source/scope filters, evidence, order and line severity.
For a physical `(file, line)` that survives admission, collect rule evidence from **all** paths and inline
sources for that key. A path can supply a rule even when another path, processed later, admits the line.
Buffer candidates and intersect with the final set of admitted keys. An empty set on an admitted line
serializes with the `rules` key omitted. No identity creates, moves or splits a finding.

1. Identify the occurrence by file, AST span, callee/operator and operand span. Start a flat walk at the
   reached use (`VarLocation` bytes), through enclosing calls up to its statement or function boundary.
   Test the operation's operand in Appendix A, including named arguments, aliases and generic callees.
   Inspect the RHS of an actual DOM/location assignment, the value of JSX `__html`, and the named
   `Location` header value. The receiver of `x.send("ok")`, an assignment's LHS, an unrelated argument,
   an ordinary object key and a function value merely mentioned on the line supply no such evidence.
   Two receiver chains carry a value by construction: `Path(x).open()/unlink()` uses the constructed
   path, and a TypeORM builder's `where(SQL)` uses its own clause. Attribute that role at the consuming
   node on an already admitted line. Never borrow the literal alias in `createQueryBuilder('u')`.
2. Enumerate each matching structured **occurrence** and each eligible operand, including repeats of one
   pattern, special Flask/JSX forms and framework tables (Appendix B). Preserve the path matcher's
   file/name/identifier-line, semantic, import and JS source-access-range checks. Bind the candidate to
   the reached operand. Use the edge bytes when available. If a graph key coalesced same-line uses
   (`VarLocation` equality omits bytes), an additional spelling needs the same reaching binding/value,
   in the same file and scope, with no intervening write. A shadow, different request-field range or
   ambiguous binding supplies no additional occurrence. This rule applies equally to flat duplicates
   and structured siblings. A nested result inside a consuming operand counts under the existing
   expression-containment model. This is evidence of syntactic flow, not proof of a callee's return value.
3. Check cleansing on the **particular path, call and pattern**, using its original sanitizer category.
   A public reclassification does not change which guard admits a finding. Apply safe-call ranges to
   the flat callee/target identifier (a range may cover just the callee), and apply call-specific
   cleansing to each structured match. An outer safe-call range can exclude flat identifiers inside it.
   A reached structured inner call gets its own check, including when its outer SQL call binds a
   parameter. A live call elsewhere cannot uncleanse its sibling. A callee-only index miss excludes the
   corresponding indexed role. A distinct proven role, e.g. file contents, may still be named if the
   line is admitted. Flask's flat safe-context range alone does not refute a live template-source match.
   A full parameterization, safe-loader, literal-binary or category guard continues to exclude the
   affected call's fallback. A line with only a cleansed first match and no other admission stays absent.
4. Preserve the framework flat filter: on a Go/Python framework-source line it excludes flat hits even
   from incoming edges. On JS-family source lines use the existing receiver, source/target byte ranges
   and tainted-argument/RHS checks. A Flask diff seed is not necessarily a detected framework-source
   line. For JS YAML, enumerate unsafe calls with an inline request in the document slot, keyed to the
   call's opening line. For Flask, enumerate all qualifying inline calls spanning the selected source line,
   including template context keywords selected by the special matcher. For Go source lines union **all**
   uncleansed matching originating paths **and** inline witnesses even if a path already succeeded.
   With no originating path, `go_sink_outcome(None)` can admit the line on shape alone. A rule then
   requires a descendant framework-source call in the selected operand, with the existing function-local
   request-parameter binding and source-line guard. This includes the secondary inline fallback.
   A literal path beside a source, an HTTP request object passed as a writer, and a binder in another
   function supply no rule. These branches do not manufacture flat rules.
5. A configured name (`TaintConfig.extra_sinks`) contributes `configured-sink` for a reached argument of
   the matching callee. Keep the existing substring and `=name` exact predicates. A discovered variadic
   wrapper contributes `format-wrapper` under its exact name. Preserve both origins before their lists
   are combined for presence. Both rules are strong and published without a CWE. A wrapper name proves
   neither forwarding nor the format slot, even in `report("%s", x)`. A matching configured and wrapper
   name contributes both ids once, and a built-in occurrence can contribute alongside them. A name in a
   receiver, reference, clean sibling or function declaration cannot supply either id. A shared name-only
   id would hide which detector fired. Deriving an id from user text would open the registry. Assigning
   CWE-134 from the wrapper name would classify an ordinary substitution as the format program.
6. `legacy` is the sole line-based exception. In C, C++, Java, Rust, Lua, Terraform/HCL and Bash
   (and any other language dispatched to the legacy list), attach it once if an eligible legacy token
   survives the existing language/name filters on the admitted physical line. It can name a literal
   sibling or a variable. It cannot appear on a configured-only line without a built-in hit.
   `unquoted_expansion` and `taint_source` always carry `[]`, including when the Bash pass yields to an
   existing `taint_sink` at that line.

**The two open grids.** In every sibling cell choose the reached and clean operations independently:
strong flat, weak flat, structured, flat/structured SQL overlap, or an ordinary use. Check each operand
before union, for literal and independent values, either source order, separate statements, nested
expressions, JSX attributes, receiver position, assignment target and nested non-indexed slots. Crossing
JS/TS/TSX, Python or Go with these axes does not grant an identity to the clean operand or lend it
strength. If neither operation reaches a modeled role, the result is empty on an admitted line and
absent otherwise. Use step 6 for the seven legacy grammars. Same-name binding collisions use step 2.

For every continued cell, first decide admission **on the marked physical line**. JS/TS/TSX and Python
structured matchers use containment. Go structured calls must start on that line. A flat-only callee
above the use does not itself admit the use line. On an admitted use line, walk every enclosing flat
operation, including split/optional/generic callees, nested calls spanning different lines and continued
RHS/JSX values. Require that line's reached operand or the insertion branch's explicit inline witness.
Test the outer and inner slots and their guards separately. First/middle/last use, opening/closing
delimiter alone, split leaf, extra admitting sibling, clean literal/independent inner argument, weak SQL,
sandbox and Gin follow the same procedure. Two use lines have two independent decisions. Only the
specified inline insertion may attach a later operand's evidence to its designated earlier line. The
legacy scan stays on its own physical line. The enumerators' grid sizes describe possible tests, not
coverage. Appendix E gives the individual static expectations.

**Severity is relative to the actual finding.** Reduce strengths by public id across attributable
occurrences/paths, strong winning over weak. Emit `severity: "suggestion"` only for an all-weak id
on a `warning` finding. A suggestion finding needs no override. JS, Python and Go SQL keep their
own call grades. Bound parameters are value roles and weak when binding is established, even if
unrelated SQL text is dynamic. Python's proven ORM value/comparison or statement uses its clause/statement
grader. A SQL pattern that reaches the same defused statement inherits that occurrence's grade.
An opaque predicate with no weak proof keeps the classifier's grade. A sandboxed Jinja `from_string`
is weak on its source, a normal environment strong. Three-argument render on a declared Gin context
is weak, raw Echo/Iris HTML is strong. All other structured roles are strong unless the same-call
SQL rule above applies. A raw-SQL line veto, structured non-SQL veto, configured name or constant command
elsewhere can raise the **finding** alone. Keep `line_strength` intact. Copying line severity onto
each rule would hide a weak sibling. Serializing a level on every occurrence would duplicate the
finding's level and give a single id conflicting overrides when both strengths occur.

Keep separate flat and structured ids when each documents an established detector (including two SQL ids
for the same SQL text). A single neutral row can describe both detectors' evidence when neither establishes
a weakness. Category-only translation would publish 502 for JS code and 78 for SQLite. A blanket
any-argument rule would publish path traversal for file contents. Dropping all weak or opaque hits
would hide which operation or name admitted the finding. Assigning new lines from the attribution walk
would change counts. Appendix F states the losses and their measurement.

PR #841's union retains the baseline finding on overlap and the expanded finding on an expanded-only
key. Lane 4's side-set rule is `js-proto-write@CWE-1321` for a reached prototype write. If lane 4 lands
first, S2 migrates its side-set evidence, preserving descriptions, and S3 registers it. If it lands
second, lane 4 writes the field and adds the row if S3 has landed. Any migration of its description
and benchmark join stays with lane 4.

### 5.3 The registry

`RuleInfo { id, cwe, name, short, help, algorithms }` has one named constant per public id and one
immutable optional CWE. Built-in producers select those constants. Only `RuleInfo::rule_ref` constructs a
production reference. Strength is reduced after selecting the row. `rules()` sorts by id. `check_emitted`
rejects unknown ids, mismatched CWEs (including an unexpected value for a CWE-less row), and unlisted
algorithms. Retain the failing controls for these cases, the source scan with its computed-id control,
the table-membership test, and validation on every emitted matrix reference. External inputs follow 5.5.

Use the exact id as the stable SARIF `name` (including uppercase primitive ids). Write `short` as one
sentence naming the observed operation and operand. Write `help` as one or two sentences explaining the
match and a specific inspection or mitigation. Use conditional language for nominal/weak matches and
name the missing evidence for an unresolved rule. Keep ids, CWEs and severity out of prose where they
would duplicate metadata. Generate the page and specific-mode descriptors from these same strings.
For `legacy` use `name = "legacy"`, `short = "A legacy sink name matched on a tainted line."` and
`help = "Inspect the matched name and trace the input to the operation. This detector does not identify a particular operand or weakness."`
Use `algorithms = ["taint"]` and `cwe = None`. A generated display name would change when copy changes.
Generic mitigation boilerplate would imply identical evidence for a format argument and a name match.

### 5.4 Other algorithms

For each primitive, publish exactly its existing category id and the CWE in Appendix C. For the three
missing-counterpart categories in absence, use the emitting pair's constant in Appendix D. For
`double_close` use the repeated operation and operand test there. A stable literal slug survives edits
to the pair description. A slug derived at rendering time would silently change filters. All these
references inherit the finding's severity. Other algorithms continue to produce an empty rule list.

### 5.5 SARIF

- In default category mode, each result gains `properties.rules` when the finding has references
  (decision 7). This preserves the existing rule ids, descriptors and fingerprints.
- `--sarif-rule-ids specific` (default `category`): after admission, a finding's references are grouped by
  id (repeated or conflicting references to one id give one result), and the finding is written as one
  result per distinct id, with `ruleId` `prism/{algorithm}/{rule id}` and level from that reference's
  `severity` when present; evidence, locations and fingerprint are computed once from the finding, and
  `ruleIndex` is assigned after expansion and sorting. A registered id's descriptor comes from the registry
  (`name`, short and full descriptions, `help`, and `properties.tags` holding `external/cwe/cwe-<n>` when the
  row has a CWE); the registry wins over a disagreeing supplied CWE, and over a category-mode descriptor of
  the same id (primitive ids equal their categories), whatever the input order. An unregistered id gets
  `name` = id, generic descriptions, and tags from the sorted distinct well-formed CWEs supplied for that id
  in the run. The fingerprint is unchanged (it does not include the rule id), so the results of one
  multi-rule finding share it and differ by rule id; a consumer keying on it combines it with `ruleId` and
  the primary location. Findings without rules are written as today.
- The flag's help text and the README say that switching modes can start new code-scanning alert histories
  for the affected findings, and that GitHub's display of the CWE tags is unverified (section 2).

### 5.6 Review and JSON

`--format json` and `--format review` both serialise the finding, so the `rules` key appears in both, in
every copy of a finding (per result and in the aggregate list, single and multi run), and is omitted when
empty (decision 7). The text, paper, Mermaid and callers outputs print at most a finding's description
(Mermaid's diagram headings), so they show no rules and need no renderer change. One byte-golden holds a
taint finding (`tests/fixtures/review_no_diagrams/golden/review_unflagged.json`, two copies of one finding);
it is re-recorded once in the taint slice, with the reason in the commit.

### 5.7 Targets

No change: ids hash neither the new field nor anything this lane touches. A test compares target ids with
main's on the targets fixtures. A targets-only consumer therefore still sees every taint sink as
`taint_sink`; how it could get the weakness later is decision 6, owned outside this lane.

## 6. Gates

| Gate | Statement |
|---|---|
| G-additive | both builds run on one pinned copy of the fixture inputs; `--format json` and `--format review` output equals the base's byte for byte once the text spans of the `rules` keys the slice adds and the per-build values at their exact paths are removed from the original buffers; `rules` keys present on both sides are equal; any other byte difference, a missing output or an algorithm error fails (PLAN stage M) |
| G-fingerprint | the multiset of SARIF fingerprints in default mode equals the base's; default-mode SARIF equals the base's by the same byte rule, with `results[].properties.rules` as the allowed key |
| G-targets | target ids (in order) and the whole targets document, per-build values masked, equal the base's, with every finding-producing algorithm passed explicitly |
| G-closed | registry tests of 5.3 pass, including the failing controls for a hand-built or computed id and a disagreeing CWE; in `specific` mode every result's rule id has exactly one descriptor, and every descriptor of a rule with a CWE has the tag |
| G-coverage | every positive row of the sink-rule tables (`tests/algo/taxonomy/taint_sink_rules_*`, whatever helper runs it) and of the identity table asserts the finding's rules against the identity stage D fixed, not against what the code emitted; every label in those tables has exactly one expectation line, checked in both directions |
| G-sarif-valid | on the final head, after the last producer slice, the SARIF shape assertions pass in both modes on logs with non-empty results from taint, primitive and absence; where a validator is available, a mixed log validates against a checksum-pinned SARIF 2.1.0 schema; otherwise the result says "schema not validated" and names the assertions that ran |

## 7. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Default SARIF rule-id mode | `category` (today's ids; alert history preserved); `specific` is opt-in | flipping the default later is one line and a release note; every user of the default then gets new alert histories once | at any release |
| 2 | Several rules on one line | one finding with a list; SARIF `specific` mode writes one result per rule | — | before S1 |
| 3 | CWE for structured sinks | from the matched pattern by one function in `prism-review`: the category's CWE by default, with the exceptions stage D lists, and no CWE where the operation is not established | a CWE field on the pattern tables in the core crate would move this data next to the patterns | stage D |
| 4 | Non-taint algorithms in this lane | primitive and absence | others can be added one registry row at a time | stage D |
| 5 | `security-severity` in SARIF | not emitted (no scores are invented) | adding a fixed band per severity is a small change | — |
| 6 | Rule and CWE for a targets consumer | not added; targets output unchanged (the contract is lane 2's) | a targets consumer can get the weakness later through a contract minor that adds the rules to a row (agreed with lane 2, opt-in or next version, same id recipe), or a companion document keyed by target id (no schema change, but a join and a rule for dropped or duplicate targets); ids would not change either way. Not designed here | when a consumer asks |
| 7 | Default JSON, review and SARIF output | (**confirmed by the owner, 2026-11-21**; the optional per-rule `severity` key rides on the same answer unless the owner says otherwise) a finding with rules gains the additive `rules` key (SARIF: `results[].properties.rules`) by default, as other additive fields have shipped; the review golden is re-recorded once; additivity is "equal to main once the `rules` keys and per-build keys are removed" | the alternative is the key only behind an explicit option (every existing byte kept; default users get no rule identity). Moving to it after a release removes a key readers may rely on, a breaking change for them; before S2 merges it is a small change to S2's serialisation and tests, with the golden left as it is | before S2 merges |

## 8. Risks and accepted limits

- **Twenty-seven struct literals change in one commit.** It is mechanical and reviewed as such, with no
  behaviour change in that commit.
- **`legacy` hits have no CWE.** Languages without positional sink rules get a rule id that says so.
- **Rule ids become a public surface.** Renaming one later is a breaking change for anyone filtering on it;
  the registry test and the generated page make renames visible in review.
- **Other output consumers.** Nothing in `eval/` parses finding JSON by key list, and the sibling
  fault-injection product reads targets only, which do not change. A reader that rejects unknown keys sees
  the new key (decision 7).
- **Coalesced graph occurrences.** The path matcher can nominate two same-spelled uses. Section 5.2
  requires the reached range or corroborating same-binding evidence for publication. Appendix F measures refusals.
- **Existing admission limitations:** the first structured match decides
  presence, so a line whose first match is cleansed and whose second is live gets no finding unless another
  sink admits it; Python call-level cleansing requires the call to start on the finding line while matching
  uses containment (a continued call's guard can be skipped). Appendix F records how to measure these cases.

Recorded, not pursued:

- Adding a field breaks a downstream full struct literal of `SliceFinding` at compile time; the API promises
  it only as a handle to read and pass back.
- A raw SQL operation matched by both detectors has two ids and produces two `specific`-mode results at
  one location. The neutral bound-value role has one id (5.2).

## Appendix A. Flat and name-based identity decisions

The observation column is the classifier's internal tag/CWE on this checkout. S1 serializes no rule on
these findings yet. `id/95` below means `id@CWE-95`. A bare id is **published without a CWE**. `∅`
means that this occurrence contributes no reference. Dispositions are **same**, **corrected** (identity
projection only) and **filed** (the neutral public row and its follow-up in F). Each line has one
disposition. Conditional lines partition occurrences of a detector. Columns A and B together cover
the 34 flat tags, all structured patterns, and the additional public rows.

Use the API/receiver, argument role and actual binding at the call. The matrix uses nominal API shapes
such as `res.send`, `db.query`, `cur.execute`, `db.Queryx` and `r.pg.Exec(ctx, sql)`. A distinctive API
signature, receiver type/import, constructor, same-scope alias or SQL/HTML syntax can identify that
operation. A generic name such as `thing.query(x)`, a shadowed import or mutually incompatible reaching
definitions cannot. A dominating, unconditional reassignment can resolve the operation. Use the neutral
row when a candidate receiver remains ambiguous. No test label, unrelated import or string elsewhere
in the file may be a runtime discriminator. A known inert operand (status, writer handle, flags, SQL
connection, HTTP method, insertion position, unrelated options) supplies ∅ unless a row explicitly
names that operand. A structured/public CWE is a rule's weakness class, never an assertion that an
attack succeeded.

| Classifier today | Operation and reached operand | Published identity | Disposition |
|---|---|---|---|
| js-eval/95 | Global/unshadowed or indirect JS eval, source text | js-eval/95 | same |
| js-function-ctor/95 | Function/new Function, dynamically parsed body or formal-parameter text | js-function-ctor/95 | same |
| js-child-process/78 | Bound child_process, execa or shelljs launch, selected program or shell command. With shell enabled, arguments interpolated into command text qualify | js-child-process/78 | same |
| js-child-process/78 | Node fork selects the child **module filename** | js-child-module/73 | corrected |
| js-exec-call/78 | Proven remote command API, including node-ssh exec/execCommand, command text | js-exec-call/78 | same |
| js-child-process/78 or js-exec-call/78 | Bare unbound exec/spawn/fork, unknown/injected/shadowed receiver, or mixed regex/command binding | taint-exec-name | filed F1 |
| js-exec-call/78 | A regex variable is definitely reassigned to child_process before this call | js-child-process/78 | corrected |
| js-eval/95, js-function-ctor/95 or py-eval/95 | An eval-like spelling, no evidence the receiver evaluates code | taint-eval-name | filed F1 |
| js-db-call/89 | SQL text/fragment, including SQLite exec, raw constructors, query/execute and the reached SQL where-clause of a builder chain | js-db-call/89 | same |
| js-redirect/601 | Redirect destination, including a shifted URL after a status | js-redirect/601 | same |
| js-location/601 | Navigation assignment/method URL, or reached Location value on a known redirect response (including writeHead status/object) | js-location/601 | same |
| js-location/601 | Location header setter without a known redirect status | js-location-header | filed F2 |
| js-response/79 | Nominal HTML-capable response send/write, string/raw body. Explicit response type and payload shape take precedence | js-response/79 | same |
| js-response/79, py-output/79 | Unknown message/write destination, default Fastify plain/JSON response, Express object/array serialized as JSON, or unproven self.write handler | taint-write-data | filed F2 |
| js-write/79 | Unclassified write payload, no established HTML interpreter | js-write | filed F2 |
| js-dom-html/79 | DOM HTML assignment RHS, document.write/writeln HTML, insertAdjacentHTML argument 1, actual JSX __html value, document.execCommand("insertHTML", …, html) | js-dom-html/79 | same |
| js-dom-html/79 | JSX-like object key, insertion-position argument, computed LHS base or non-HTML execCommand operand | ∅ | corrected |
| js-storage-write/22 | Storage API destination argument, e.g. disk.write(path, stream) with the reached path | js-storage-write/22 | same |
| js-fs/22 | Path of the resolved fs/fs-extra operation, both paths for TWO_PATH_API (e.g. rename, copyFile, link). Reuse its receiver/alias tables | js-fs/22 | same |
| js-fs/22, js-storage-write/22, py-output/79 or go-file-op/22 | Bytes written to a fixed path or already-open stream, including appendFileSync("a", x) | taint-write-data | filed F2 |
| js-location/601 or py-file-open/22 | window.open or webbrowser.open, new browsing target. No redirect response or filesystem operation is established | browser-open | filed F2 |
| js-fs/22 | Imported open package hands a target to the OS default handler | external-open | filed F2 |
| py-db-call/89 | Statement text in execute/executemany/executescript/exec_driver_sql/raw/query or pandas read_sql family. Respect SQL/query keyword names and positional SQL slot | py-db-call/89 | same |
| py-eval/95 | SQLModel Session.exec with a SQL or raw SQL-bearing statement. Bind Session by its import/type/constructor or demonstrated statement API (#646) | py-db-call/89 | corrected |
| py-eval/95 | Built-in eval/exec or builtins alias, evaluated source | py-eval/95 | same |
| py-sql-clause/89 | Actual SQL syntax supplied to a clause or SQLAlchemy text constructor | py-sql-clause/89 | same |
| js-db-call/89, py-db-call/89, py-sql-clause/89, py-eval/95 (SQLModel), go-db-call/89 | Bound SQL data, an ORM comparison/opaque query operand with no demonstrated raw clause, including a SQLModel expression. A dynamic statement independent of a reached bind value gives the value role only | taint-query-value | filed F3 |
| js-db-call/89, py-sql-clause/89 or go-db-call/89 | Unknown query/filter API, untyped q.filter/q.where with an opaque argument, DataFrame label filter or Cache.Query(key) | taint-query-name | filed F1 |
| py-os-command/78 | os.system/popen or exec/spawn family, asyncio subprocess or pty: command text or program selector in the API's slot (spawn mode, context and argv have separate roles) | py-os-command/78 | same |
| py-subprocess/78 | subprocess run/call/Popen/check_*: shell text or selected executable (including the program element of argv) | py-subprocess/78 | same |
| py-module-command/78 | Proven paramiko, pexpect, fabric or invoke callable: command/program operand | py-module-command/78 | same |
| JS/Python/Go process tags /78 | Data passed only in fixed-program argv, options/environment, or after fixed shell command text, without evidence it is interpreted as code | taint-process-argument | filed F3 |
| py-template/1336 | Template constructor or Flask render_template_string: template source, including a reached source keyword | py-template/1336 | same |
| py-template/1336 | Flask literal template renders the reached context keyword unescaped (matching safe variable or disabled autoescape) | py-html/79 | corrected |
| py-template/1336 | Autoescaped Flask context only, unrelated context keyword or fixed template with no unsafe context use | ∅ | corrected |
| py-module-template/1336 | Jinja environment from_string: positional/source= template, including a sandboxed environment with weaker grade | py-module-template/1336 | same |
| py-html/79 | Markup or mark_safe marks its payload as trusted HTML | py-html/79 | same |
| py-output/79 | Demonstrated raw HTML output body, e.g. actual HTML-producing RequestHandler | py-output/79 | same |
| py-redirect/601 | Redirect/HttpResponseRedirect destination | py-redirect/601 | same |
| py-file-open/22, py-file-remove/22 | File path opened or removed, including the path captured by pathlib.Path and consumed by its open/unlink method | respective original id /22 | same |
| py-file-remove/22 or go-file-op/22 | Untyped/mixed collection, asset or store remove, path role unestablished | taint-remove-name | filed F1 |
| py-dynamic-attr/470 | getattr reads a computed attribute name (plain record lookup) | py-dynamic-attr | filed F4 |
| py-dynamic-attr/470 | Untrusted name selects an attribute to modify via setattr on an object | py-attribute-write/915 | corrected |
| py-dynamic-attr/470 | A computed getattr name directly selects the callable subsequently invoked | py-reflective-dispatch/470 | corrected |
| go-exec-command/78 | Reached executable in Command, CommandContext or syscall.Exec, or actual shell-wrapper command text | go-exec-command/78 | same |
| go-db-call/89 | SQL text in DB/Tx/sqlx/pgx/Gorm execution, including context-shifted signatures, method values and generic callees | go-db-call/89 | same |
| go-format-string/134 | fmt.Sprintf format 0 or Fprintf format 1 (buffer or other destination) | go-format-string/134 | same |
| go-writer-output/79 | fmt.Fprintf(w, x): x controls format 1, including a response writer. A second, independently reached HTML output value qualifies under the HTML row | go-format-string/134 | corrected |
| go-format-string/134 or go-writer-output/79 | Only Sprintf substitution reaches an unrelated nonconstant format, or only the writer handle is tainted | ∅ | corrected |
| go-writer-output/79 | Raw output data to a generic writer or text-oriented HTTP body with no HTML evidence (e.g. "data: %s") | go-writer-output | filed F2 |
| go-writer-output/79 or go-template-html/79 or go-html-render/79 | HTML response content, e.g. literal HTML framing of the reached interpolation, explicit HTML media type, Echo/Iris raw HTML body | go-html-output/79 | corrected |
| go-file-op/22 | os/fs/afero path, including both Rename paths and the path after afero's filesystem handle | go-file-op/22 | same |
| go-file-op/22 | Chmod on an open file, mode only (including an os.File named list) | go-file-mode | filed F4 |
| go-redirect/601 | Redirect URL in net/http or framework-specific slot | go-redirect/601 | same |
| go-html-render/79 | Three-argument typed Gin HTML render, template selector or escaping data context | go-html-render | filed F2 |
| go-template-html/79 | html/template.HTML trust conversion, including import aliases | go-template-html/79 | same |
| go-template-html/79 or go-html-render/79 | Application HTML helper with unknown implementation, including response.HTML(w, r, x) | taint-html-name | filed F1 |
| legacy (no CWE) | Eligible legacy name on admitted line | legacy | same |
| Name match, no tag | Caller-supplied callee name, reached argument | configured-sink | filed F5 |
| Wrapper name, no tag | Exact detected variadic-wrapper callee, reached argument | format-wrapper | filed F5 |

For a genuine syntax/operand correction, change only the projection on an existing finding. Keep the
classifier result used for presence and line strength available separately. This matters for SQLModel,
writer-first `Fprintf`, and Gin. Distinct zero-CWE ids preserve the documented role. Removing the CWE
from an otherwise precise shared row for every occurrence would throw away evidence on its valid cases.
Lane 4's conditional row in 5.2 has no emitter in the D checkout's taint loop.

## Appendix B. Structured pattern dispatch

`structured_rule` consumes the matched pattern **and the occurrence, including the reached operand and
receiver/binding evidence**. Position is zero-based. The current value in this table is the internal
`SanitizerCategory`, which has no CWE field. Numbers in parentheses are the old category-default
translation, never a second producer of a CWE. The default rows apply when the stated operation is
established. Apply the narrower split rows first. Distinct public ids for flat and structured matches
preserve which detectors fired. Mapping every pattern solely by category, or changing the core sanitizer
tables to describe publication, would conflate the operation with the guard used for admission.

| Language and current table | Patterns, eligible operand and condition | Public rule | Disposition |
|---|---|---|---|
| JS/TS/TSX Xss (79) | insertAdjacentHTML[1] HTML, dangerouslySetInnerHTML special JSX attribute's actual __html value (table index 0). A property in an ordinary object is ineligible | structured-xss/79 | same |
| JS Sqli (89) | query, execute, raw, literal, $queryRawUnsafe, $executeRawUnsafe [0], SQL syntax. Keep the parameterization and Prisma checks | structured-sqli/89 | same |
| JS Sqli (89) | Mongoose $where[0] evaluates a JavaScript predicate supplied as code | structured-code-eval/95 | corrected |
| JS Ssrf (918) | fetch, get, post, axios, got [0], request [0,1]. Require the matcher-recognized client/global and a URL or destination option (host/hostname/port/protocol/path as it participates in a destination). These APIs can issue server-side requests. Body, method or unrelated header alone gives ∅ | structured-ssrf/918 | same |
| JS Ssrf (918) | A matched destination in an explicitly browser-only request. Require evidence of the browser-only binding or execution context. A TSX extension alone cannot establish it | structured-client-request | filed F2 |
| JS Deserialization (502) | unserialize, deserialize, yaml.load and bound js-yaml load [0], serialized document under accepted schema | structured-deserialization/502 | same |
| JS Deserialization (502) | eval, Function, vm.runInNewContext, vm.runInThisContext, vm.runInContext, vm.Script [0], JS source or executable formal text. A new-expression contributes only if the matcher actually returns it | structured-code-eval/95 | corrected |
| JS OsCommand (78) | exec, execSync [0], proven process/SSH command. spawn, spawnSync [0,1] with the existing shell:true check, program or shell-interpreted arguments | structured-os-command/78 | same |
| JS OsCommand (78) | exec[0] on SQLite/database handle executing SQL, including ri24 | structured-sqli/89 | corrected |
| JS PathTraversal (22) | readFile, readFileSync, writeFile, writeFileSync, createReadStream, createWriteStream, unlink, rm, sendFile, download [0], rename [0,1]. Only file paths | structured-path-traversal/22 | same |
| Python Xss (79) | mark_safe, Markup, markupsafe.Markup [0], trusted HTML. format_html[0], reached HTML format text, excluding escaped substitutions | structured-xss/79 | same |
| Python Xss (79) | render_template_string source at 0 in the special matcher, overriding the table's [1] | structured-template/1336 | corrected |
| Python Xss (79) | Same pattern, unsafe context keyword value for a named safe-filter use or disabled autoescape. Ordinary escaped context gives ∅. Use actual keyword/value, independent of ordinal | structured-xss/79 | same |
| Python Sqli (89) | execute, executemany, raw [0], reached SQL syntax | structured-sqli/89 | same |
| Python Ssrf (918) | requests and httpx: get/post/put/delete/patch/head/options [0], request [1]. urllib.request.urlopen/Request [0], urllib3.PoolManager.request [1] | structured-ssrf/918 | same |
| Python Ssrf (918) | aiohttp.request [1], aiohttp.ClientSession get/post/put/delete/patch/head/options [0], request [1]. Include the imported/bound client special matchers. Attribute the URL value | structured-ssrf/918 | same |
| Python Deserialization (502) | pickle, cPickle, cloudpickle, marshal and dill: load/loads [0]. yaml.load [0] under accepted loader, jsonpickle.decode [0]. Serialized bytes/document/input stream | structured-deserialization/502 | same |
| Go OsCommand (78) | exec.Command[2] and exec.CommandContext[3], checked shell-wrapper payload. Separate exec.Command[0] and exec.CommandContext[1], selected executable. syscall.Exec[0], executable | structured-os-command/78 | same |
| Go OsCommand (78) | syscall.Exec[1], the reached argument supplies command text to a recognized shell invocation | structured-os-command/78 | same |
| Go OsCommand (78) | syscall.Exec[1], ordinary or opaque fixed-program argv. envv[2] has no structured match | taint-process-argument | filed F3 |
| Go PathTraversal (22) | os.Open/OpenFile/ReadFile/Create/WriteFile/Remove/RemoveAll/Mkdir/MkdirAll and ioutil.ReadFile/WriteFile [0], os.Rename [0,1] | structured-path-traversal/22 | same |
| Go framework PathTraversal (22) | Gin c.File[0] or net/http http.ServeFile[2], path. gorilla/mux has sources but an empty sink table. The JS-family and Python framework specs also have empty sink tables | structured-path-traversal/22 | same |
| SQL category or SQLite exception | An indexed ORM statement contains solely a reached bound comparison/value and survives its own exclusion checks | taint-query-value | filed F3 |
| Ambiguous exec/eval/query match | Same unresolved operation as in A across both detectors. Use respectively taint-exec-name, taint-eval-name or taint-query-name, once per id | respective neutral row | filed F1 |
| Other ambiguous nominal pattern | A matched tail with an unresolved API and no other evidence of the named operation, e.g. an application method named deserialize | structured-name-match | filed F1 |

These rows cover 40 JS-family entries, 48 Python entries, 17 ordinary Go entries and two Go framework
entries. Materially different slots in one occurrence can produce different rows. Framework-source and
semantic special cases in 5.2 decide if there is an occurrence at all. A rejected match contributes ∅.

## Appendix C. Primitive registry rows

Today the emitter puts the id in `category` and `rule_description` describes the operand. Each has
empty `rules`. The id disposition is **same** for all six. The new CWE is the class of the matched
fingerprint. The algorithm's existing proximity-based severity still grades the finding.

| Current category = published id | Operation in the emitter and SARIF description | Published CWE |
|---|---|---|
| HASH_TRUNCATED_BELOW_128_BITS | A literal slice shortens a hex or byte digest below 128 bits, reducing collision resistance | CWE-328 |
| HASH_TRUNCATION_VIA_CALL | A literal argument selects that short digest width through a helper | CWE-328 |
| WEAK_HASH_FOR_IDENTITY | MD5 or SHA-1 digest used as an identity/integrity token | CWE-328 |
| SHELL_TRUE_WITH_INTERPOLATION | Interpolated command string reaches a shell interpreter | CWE-78 |
| CERT_VALIDATION_DISABLED | TLS peer/certificate verification is disabled | CWE-295 |
| HARDCODED_SECRET | A credential, key or token is a source literal | CWE-798 |

Using one truncation id would lose the direct-versus-helper detector. Using an encryption or randomness
CWE for digest width would describe a different primitive. The credential rule spans passwords and keys,
so a password-only CWE is too narrow.

## Appendix D. Absence registry rows

In `default_pairs()` order, the first column is the exact current pair description. Today the algorithm
embeds that text in the three missing-counterpart findings, with no rule or CWE. The new id is a
literal constant independent of the description. Read each row as **corrected** (the pair now has a
public identity), or **filed F6** where the rule is **published without a CWE** (dash). A resource
CWE describes the manual-release obligation modeled by the pair. Mixed ownership or advisory pairs
get a single CWE-less row. Inferring a leak, missing authentication, or cloud exposure from a missing
code spelling would publish the wrong claim. This table applies to `missing_counterpart`,
`close_only_on_error_path` and `missing_close_on_error_path`.

| Observed pair description | Stable public id | CWE | Basis |
|---|---|---|---|
| file open without close | `absence-file-handle` | — | Generic open/close spellings include scope-owned files. |
| lock without unlock | `absence-lock` | — | Generic lock/acquire includes scope guards. |
| connection opened without close | `absence-connection` | CWE-772 | Connection acquisition and release. |
| event subscription without unsubscribe | `absence-subscription` | — | Registration may live for the lifetime of the listener. |
| transaction begin without commit/rollback | `absence-transaction` | — | The intended commit or rollback boundary is unknown. |
| allocation without free | `absence-allocation` | — | Mixed malloc/alloc/new and release targets include managed storage. |
| timer set without clear | `absence-timer` | — | One-shot timers can finish by themselves. |
| item added without removal path | `absence-collection-item` | — | Persistent membership can be intentional. |
| span/timer started without end | `absence-span` | — | Instrumentation lifecycle. |
| resource acquisition without defer cleanup (Go) | `absence-go-defer` | — | The spelling of defer does not identify an acquired resource. |
| kernel allocation without free | `absence-kernel-allocation` | CWE-401 | Kernel heap or virtual allocation. |
| DMA allocation without free | `absence-dma-allocation` | CWE-401 | Coherent DMA memory allocation. |
| IRQ registered without free | `absence-irq` | CWE-772 | Registered interrupt resource. |
| spinlock without unlock | `absence-spinlock` | CWE-667 | Acquired kernel spinlock. |
| clock enabled without disable | `absence-clock-enable` | CWE-772 | Clock enable reference requires a balancing disable. |
| platform driver registered without unregister | `absence-platform-driver` | — | Registration lifetime depends on driver ownership. |
| device tree node get without put | `absence-device-tree-node` | CWE-772 | Reference obtained by get/find requires put. |
| kernel mutex lock without unlock | `absence-kernel-mutex` | CWE-667 | Acquired kernel mutex. |
| rtnl lock without unlock | `absence-rtnl-lock` | CWE-667 | Acquired network configuration lock. |
| kstrdup allocation without kfree | `absence-kernel-string` | CWE-401 | Duplicated kernel heap string. |
| slab cache allocation without free | `absence-slab-object` | CWE-401 | Allocated slab object. |
| RCU read lock without unlock | `absence-rcu-read` | — | RCU read-side nesting is a synchronization protocol, not a held mutex. |
| pthread mutex lock without unlock | `absence-pthread-mutex` | CWE-667 | Acquired POSIX mutex. |
| semaphore wait without post | `absence-semaphore` | — | A semaphore permit may intentionally be consumed or posted by another party. |
| mmap without munmap | `absence-memory-map` | CWE-772 | Unreleased mapping. |
| POSIX file descriptor opened without close | `absence-posix-file-descriptor` | CWE-775 | Owned POSIX file descriptor. |
| directory opened without closedir | `absence-directory-stream` | CWE-775 | Owned directory handle. |
| pthread rwlock without unlock | `absence-pthread-rwlock` | CWE-667 | Acquired POSIX read/write lock. |
| Python threading lock without release | `absence-python-thread-lock` | — | Constructing a Lock/RLock does not acquire it. |
| Python multiprocessing pool without close/join | `absence-python-process-pool` | — | Scheduling work is mixed with creating the pool. |
| socket created without close | `absence-socket` | CWE-775 | Owned socket handle. |
| temporary file without cleanup | `absence-temporary-file` | — | mkstemp and auto-cleaning NamedTemporaryFile have different ownership. |
| Node.js stream without destroy/close/end | `absence-node-stream` | — | An auto-closing stream can finish normally. |
| server created without close | `absence-server` | — | A long-lived listener need not close in its creator. |
| database pool connection without release | `absence-database-pool-client` | — | pool.query can release internally, while pool.connect checks out a client. |
| fs.open without fs.close | `absence-node-file-descriptor` | CWE-775 | fs.open acquires a file descriptor. |
| lock/acquire without release/unlock | `absence-acquire-release` | — | Generic names identify multiple resource and scope-guard contracts. |
| Go sql.Open without db.Close | `absence-go-sql-db` | CWE-772 | Opened database handle. |
| Go file created without Close | `absence-go-file` | CWE-775 | Opened Go file descriptor. |
| Go context without cancel (may leak goroutine) | `absence-go-context` | — | Cancellation, parent lifetime and deadline matter. |
| WaitGroup Add without Wait | `absence-go-wait-group` | — | A different goroutine can own the wait. |
| Go HTTP response body not closed | `absence-go-http-body` | CWE-772 | Response body requires Close. |
| Rust file opened without explicit flush/drop | `absence-rust-file-scope` | — | RAII closes the file. An explicit flush is a distinct operation. |
| advisory: Rust mutex lock held to end of scope (explicit drop() releases sooner) | `absence-rust-lock-scope` | — | Scope exit releases the guard. Earlier drop is advisory. |
| unsafe block without safety assertion or comment | `absence-rust-unsafe-documentation` | — | Presence of an assertion/comment is a review convention. |
| Rust TCP connection without shutdown/drop | `absence-rust-tcp-scope` | — | RAII drops the socket. |
| Rust Command created but never executed | `absence-rust-command-builder` | — | Creating a command need not execute it. |
| Lua file opened without close | `absence-lua-file` | CWE-775 | Opened file handle. |
| Lua socket opened without close | `absence-lua-socket` | CWE-775 | Opened socket handle. |
| Lua coroutine created but never resumed | `absence-lua-coroutine` | — | Creation does not require an immediate resume. |
| S3 bucket missing encryption configuration | `absence-s3-encryption-config` | — | A separate resource is only one way to configure encryption. |
| S3 bucket missing public access block | `absence-s3-public-access-block` | — | A missing companion does not establish public exposure. |
| S3 bucket missing versioning configuration | `absence-s3-versioning` | — | Versioning is a policy choice. |
| Lambda function missing CloudWatch log group | `absence-lambda-log-group` | — | A log group can be created outside this declaration. |
| Security group missing explicit rule resource | `absence-security-group-rule` | — | Inline or externally managed rules are possible. |
| RDS instance missing storage_encrypted configuration | `absence-rds-storage-encryption` | — | A missing local key does not establish the effective encryption state. |
| Temp file created with mktemp but never cleaned up | `absence-shell-temporary-file` | — | Ownership and the required deletion point are unknown. |
| Filesystem mounted but never unmounted | `absence-mount` | — | A mount can be intended to outlive the script. |
| pushd without matching popd | `absence-directory-stack` | — | Shell working-directory convention. |
| Signal trap set but never restored/cleared | `absence-signal-trap` | — | A trap may legitimately last until exit. |
| File descriptor opened but never closed | `absence-shell-file-descriptor` | CWE-775 | Explicit shell descriptor acquisition and closure. |
| Lock file acquired but never released | `absence-shell-lock-file` | — | FD-scoped flock and persistent lock files differ. |
| Firmware flash write (mtd) without hash verification | `absence-firmware-verification` | — | A local checksum call alone does not establish an authenticity/integrity policy. |
| UCI config set without commit | `absence-uci-commit` | — | A caller may own the batch commit. |
| Kernel module loaded without unload in cleanup path | `absence-kernel-module` | — | Loaded modules may have system lifetime. |

**Repeated counterpart.** Today `double_close` records `close_base` plus inline and label lines.
Its `contains(close_base)` search can match different callees or different resources. Emit exactly
one of the following rows. Leave the finding and its related lines intact.

| Current category and actual operations | Public id | CWE | Disposition |
|---|---|---|---|
| double_close, two actual memory deallocations (e.g. free/kfree/vfree/kmem_cache_free/dma_free_coherent) of the same still-released allocation | absence-double-free | CWE-415 | corrected |
| double_close, two actual unlocks (e.g. pthread_mutex_unlock, mutex_unlock, spin_unlock, matching rwlock forms) of the same lock without an intervening acquisition | absence-double-unlock | CWE-765 | corrected |
| double_close, generic close/release, permissive substring, uncertain resource, mixed operands or missing lifetime witness | absence-repeated-counterpart | — | filed F7 |

Use the exact callee and its resource operand at **both** concrete sites on the goto path (for example,
`kmem_cache_free(cache, object)` uses the object, not the cache). Require the same binding/access path,
no intervening rebind, acquire, reset or conflicting alias evidence. Otherwise use the third row.
The pair's missing-release CWE would reverse the event. Giving every double close CWE-415 would label
a repeated unlock as a memory free. CWE-1341 also requires a repeated release of the same resource,
which the generic name evidence cannot establish. Keep the ordinary category and its default SARIF id.

## Appendix E. Matrix decisions

### Committed TSV

Read the four columns as a keyed patch to `test/rule-identity-matrix` at `5e9e4d864cbb`.
The source table stem is `taint_sink_rules_<table>_test`. The first two columns reproduce `rules`
and `observed` (`=` means equal to the original expectation). D gives the complete sorted wire
expectation at the marked physical line. `-` means zero references. The fixture's independent presence
assertion distinguishes a missing finding from an admitted finding with empty rules. In particular the
four substitution-only Sprintf rows are positive with empty rules. HCL `ri27` and `probe: java Runtime.exec`
are negative at the marker, though a reached legacy token would have identity `legacy`. No positive
identity assertion may succeed vacuously on a missing finding.

| Source table | Rows | Previously `?` | Concrete expectations revised |
|---|---:|---:|---:|
| identity | 34 | 21 | 1 |
| js | 70 | 17 | 4 |
| hybrid_js | 127 | 48 | 0 |
| py | 46 | 5 | 4 |
| hybrid_py | 101 | 13 | 8 |
| go | 39 | 6 | 5 |
| hybrid_go | 70 | 15 | 4 |
| other | 8 | 0 | 0 |
| hybrid_other | 45 | 0 | 0 |
| **Total** | **540** | **125** | **26** |

The reviewed Stage T fixture has 415 concrete rows. D retains 389 and revises 26. Together with
125 unknowns, 151 cells need transcription. Ten concrete rows have an explicit observation: seven
differ from the expected value (`ri08`, `ri25`, `ri26`, the Java probe, the Go io.Writer case and
the Echo/Iris raw HTML cases). Three agree (`ri23e`, `ri23f` and `ri27`). The other 405 use `=`.
The observations describe the classifiers. The D column specifies the new publication behavior.

The brief's original fixture at `a3986ce333f1` has the same 540 keys and fixture bodies.
It had 124 unknowns, 30 revised concrete expectations and 386 retained. The intervening Stage T
commit set `ri23e`, `ri23f` and `ri27` to negative, and made `generic enclosing Exec is strong`,
`non-generic enclosing Exec is strong` and `response.HTML helper keeps firing` unknown. These six
input-status changes account for the two sets of totals. All 540 final D decisions are identical on
both revisions. Keep the three newly settled negatives as replay controls.

The concrete edits distinguish a forked module filename from a shell command, a Location setter from
a known redirect and an attribute read from reflective dispatch. For SQL they distinguish statement
text from a parameter, an ORM expression or an unknown predicate. Go's enclosing-call cases keep a
strong finding and mark the reached bound value weak. The output cases distinguish an untyped writer,
an escaping template render and a raw HTML body. Gin keeps its weak grade. HCL has no sink at the marker.

#### identity

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `?` | `js-eval@CWE-95;js-response@CWE-79+structured:deserialization` | `js-eval@CWE-95;js-response@CWE-79;structured-code-eval@CWE-95` | `ri01`, `ri19` |
| `?` | `js-child-process@CWE-78;js-dom-html@CWE-79+structured:os-command` | `js-child-process@CWE-78;js-dom-html@CWE-79;structured-os-command@CWE-78` | `ri02` |
| `?` | `structured:ssrf` | `structured-ssrf@CWE-918` | `ri03` |
| `?` | `structured:deserialization` | `structured-deserialization@CWE-502` | `ri04`, `ri12`, `ri13` |
| `?` | `go-file-op@CWE-22+structured:path-traversal` | `go-file-op@CWE-22;structured-path-traversal@CWE-22` | `ri05` |
| `?` | `structured:path-traversal` | `structured-path-traversal@CWE-22` | `ri05b`, `ri11` |
| `?` | `py-db-call@CWE-89+structured:sqli` | `py-db-call@CWE-89;structured-sqli@CWE-89` | `ri06` |
| `js-response@CWE-79` | `=` | `js-response@CWE-79` | `ri07` |
| `py-os-command@CWE-78` | `py-db-call@CWE-89;py-os-command@CWE-78` | `py-os-command@CWE-78` | `ri08` |
| `legacy` | `=` | `legacy` | `ri09`, `ri10`, `ri16`, `ri17`, `ri18` |
| `?` | `-` | `format-wrapper` | `ri14`, `ri23b` |
| `go-format-string@CWE-134` | `=` | `go-format-string@CWE-134` | `ri15` |
| `?` | `structured:sqli` | `py-db-call@CWE-89;structured-sqli@CWE-89` | `ri20` |
| `?` | `structured:ssrf` | `structured-deserialization@CWE-502;structured-ssrf@CWE-918` | `ri21a`, `ri21b` |
| `?` | `py-os-command@CWE-78` | `py-os-command@CWE-78;structured-ssrf@CWE-918` | `ri22` |
| `?` | `-` | `configured-sink` | `ri23a` |
| `?` | `-` | `configured-sink;format-wrapper` | `ri23c` |
| `?` | `legacy` | `configured-sink;legacy` | `ri23d` |
| `-` | `-` | `-` | `ri23e`, `ri23f` |
| `?` | `js-db-call@CWE-89+structured:os-command` | `js-db-call@CWE-89;structured-sqli@CWE-89` | `ri24` |
| `go-html-render@CWE-79!suggestion` | `go-exec-command@CWE-78;go-html-render@CWE-79!suggestion` | `go-html-render!suggestion` | `ri25` |
| `js-dom-html@CWE-79` | `js-dom-html@CWE-79;js-response@CWE-79` | `js-dom-html@CWE-79` | `ri26` |
| `-` | `-` | `-` | `ri27` |

#### js

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `?` | `js-eval@CWE-95+structured:deserialization` | `js-eval@CWE-95;structured-code-eval@CWE-95` | `direct eval`, `window.eval`, `globalThis.eval` |
| `js-eval@CWE-95` | `=` | `js-eval@CWE-95` | `indirect eval (0, eval)` |
| `js-function-ctor@CWE-95` | `=` | `js-function-ctor@CWE-95` | `new Function` |
| `?` | `js-function-ctor@CWE-95+structured:deserialization` | `js-function-ctor@CWE-95;structured-code-eval@CWE-95` | `Function call` |
| `?` | `js-child-process@CWE-78+structured:os-command` | `js-child-process@CWE-78;structured-os-command@CWE-78` | `require alias`, `arbitrary alias execSync`, `destructured exec`, `inline require` |
| `js-child-process@CWE-78` | `=` | `js-child-process@CWE-78` | `node: prefix`, `destructured rename`, `esm namespace`, `esm named`, `promisified exec` |
| `js-child-process@CWE-78` | `=` | `js-child-module@CWE-73` | `esm default` |
| `?` | `js-child-process@CWE-78+structured:os-command` | `taint-exec-name` | `bare exec wrapper import elsewhere` |
| `?` | `js-db-call@CWE-89+structured:sqli` | `js-db-call@CWE-89;structured-sqli@CWE-89` | `db.query`, `knex.raw`, `prisma raw unsafe`, `pool.execute` |
| `?` | `js-db-call@CWE-89+structured:os-command` | `js-db-call@CWE-89;structured-sqli@CWE-89` | `sqlite db.exec` |
| `js-response@CWE-79` | `=` | `js-response@CWE-79` | `res.send`, `res.write` |
| `js-redirect@CWE-601` | `=` | `js-redirect@CWE-601` | `res.redirect`, `bare redirect call` |
| `js-storage-write@CWE-22` | `=` | `js-storage-write@CWE-22` | `disk.write` |
| `js-location@CWE-601` | `=` | `js-location-header` | `res.setHeader Location`, `res.set lowercase location`, `res.location` |
| `js-location@CWE-601` | `=` | `js-location@CWE-601` | `res.writeHead Location`, `location.replace`, `window.location.href assignment`, `location assignment` |
| `js-dom-html@CWE-79` | `=` | `js-dom-html@CWE-79` | `innerHTML assignment` |
| `?` | `js-dom-html@CWE-79+structured:xss` | `js-dom-html@CWE-79;structured-xss@CWE-79` | `insertAdjacentHTML` |
| `?` | `js-fs@CWE-22` | `taint-write-data` | `appendFileSync` |
| `?` | `js-fs@CWE-22+structured:path-traversal` | `js-fs@CWE-22;structured-path-traversal@CWE-22` | `writeFileSync path` |
| `js-fs@CWE-22` | `=` | `js-fs@CWE-22` | `unlinkSync` |
| `-` | `=` | `-` | `sanitizedQuery name`, `readByQuery call`, `InvalidQueryError ctor`, `req.query read`, `destructured query binding`, `revalidate (eval substring)`, `evaluate (eval substring)`, `Array.find`, `regex literal exec`, `variable named source`, `variable named raw`, `variable named run`, `variable named redirect`, `redirectUrl variable`, `property named redirect (read)`, `removed flag`, `list.remove`, `res.render dropped`, `emitter send on a non-response receiver`, `sendEvent helper`, `write to a non-sink receiver` |
| `-` | `=` | `-` | `Rewrite / system key`, `Function in a type position`, `freeze (free substring)`, `widgets (gets substring)`, `innerHTML read`, `location read`, `Map.set with a location key`, `str.replace`, `member named location on data object`, `setHeader for a non-Location header` |

#### hybrid_js

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `js-response@CWE-79` | `=` | `js-response@CWE-79` | `res.status().send`, `res.status().send concat`, `res.type().send`, `res.status().write`, `parenthesised chain`, `optional-chain receiver`, `optional-chain call on a chain`, `this.res.send`, `TSX res.status().send`, `probe: control: res.send`, `probe: res.status(400).send`, `probe: res.status(200).send input param`, `probe: res.type(html).send` |
| `?` | `js-response@CWE-79` | `taint-write-data` | `reply.code().send (fastify)`, `unknown receiver send`, `probe: reply.code(400).send fastify` |
| `js-location@CWE-601` | `=` | `js-location@CWE-601` | `res.status().set Location`, `document.location assignment`, `location.href through optional chain`, `probe: document.location assign` |
| `?` | `js-write@CWE-79` | `js-write` | `unknown receiver write` |
| `-` | `=` | `-` | `socket.send`, `ws.send`, `socket.to(room).send chain`, `process.send ipc`, `process.stdout.write`, `console.write`, `logger.send`, `logger.write`, `Buffer.write`, `regex literal exec`, `new RegExp exec`, `RegExp(..) call exec`, `local const regex`, `module const regex`, `module const new RegExp`, `local regex shadows a cp namespace`, `TS param typed RegExp`, `TSX regex literal exec`, `local helper named exec`, `spawn property read`, `computed member is not an identifier (unchanged vs base)` |
| `-` | `=` | `-` | `window['eval'] computed (unchanged vs base)`, `local helper named eval`, `String.raw`, `el.remove() DOM`, `Handlebars.compile (deliberate drop)`, `setTimeout string (not a base sink)`, `xhr.open is not fs/window`, `local redirect helper`, `probe: Handlebars.compile`, `probe: setTimeout string`, `r2: unchanged let regex receiver` |
| `js-dom-html@CWE-79` | `=` | `js-dom-html@CWE-79` | `document.write`, `document.writeln`, `window.document.write`, `document?.write`, `TSX document.write`, `nested innerHTML assignment`, `TSX innerHTML ref`, `probe: document.write`, `probe: document.writeln`, `probe: TSX innerHTML ref` |
| `js-child-process@CWE-78` | `=` | `js-child-process@CWE-78` | `promisify(require(cp).exec)`, `promisify(require(cp).execFile)`, `execaCommand`, `execa default`, `probe: promisify(require(cp).exec) -> execAsync`, `probe: promisify(require(cp).execFile)`, `probe: execa execaCommand` |
| `?` | `js-child-process@CWE-78+structured:os-command` | `js-child-process@CWE-78;structured-os-command@CWE-78` | `dynamic import namespace`, `destructure from namespace`, `namespace alias const shell = cp`, `cross-spawn`, `optional chain cp?.exec`, `TS (cp as any).exec`, `TS cp!.exec`, `TS import = require`, `TSX cp.exec`, `probe: dynamic import namespace`, `probe: destructure from cp namespace: spawn`, `probe: cross-spawn`, `probe: optional chain cp?.exec`, `probe: TS (cp as any).exec`, `probe: TSX cp.exec` |
| `?` | `js-exec-call@CWE-78+structured:os-command` | `taint-exec-name` | `this.cp.exec injected`, `untyped param named re (unknown receiver keeps firing)`, `exec on an unbound receiver`, `other.execSync with an unrelated cp import`, `param shadows a cp namespace`, `probe: control: re.exec`, `probe: this.cp.exec injected`, `r2: conditionally reassigned regex receiver`, `r2d: for-of rebinds a regex receiver` |
| `?` | `js-child-process@CWE-78` | `taint-exec-name` | `bare spawn, no binding`, `bare fork, no binding` |
| `?` | `js-eval@CWE-95+structured:deserialization` | `taint-eval-name` | `page.eval unknown receiver` |
| `?` | `js-eval@CWE-95+structured:deserialization` | `js-eval@CWE-95;structured-code-eval@CWE-95` | `optional-chain eval` |
| `js-db-call@CWE-89` | `=` | `js-db-call@CWE-89` | `db.queryAsync`, `typeorm createQueryBuilder`, `probe: db.queryAsync bluebird`, `probe: typeorm createQueryBuilder().where concat` |
| `?` | `js-db-call@CWE-89+structured:sqli` | `js-db-call@CWE-89;structured-sqli@CWE-89` | `mssql request.query`, `bare imported query`, `optional-chain query`, `TSX db.query`, `probe: mssql request.query` |
| `?` | `js-db-call@CWE-89+structured:sqli` | `taint-query-name` | `unknown receiver query` |
| `js-fs@CWE-22` | `=` | `js-fs@CWE-22` | `fs-extra remove`, `fs-extra removeSync`, `fs.open`, `fs.promises.open`, `probe: fs-extra remove`, `probe: fs.open` |
| `?` | `js-location@CWE-601` | `browser-open` | `window.open`, `probe: window.open` |
| `?` | `js-fs@CWE-22` | `external-open` | `bare open` |
| `?` | `js-dom-html@CWE-79+structured:xss` | `js-dom-html@CWE-79;structured-xss@CWE-79` | `TSX dangerouslySetInnerHTML`, `probe: TSX dangerouslySetInnerHTML` |
| `?` | `structured:path-traversal` | `structured-path-traversal@CWE-22` | `res.sendFile (structured)`, `probe: res.sendFile (structured)` |
| `?` | `structured:deserialization` | `structured-code-eval@CWE-95` | `vm.runInNewContext (structured)`, `probe: vm.runInNewContext (structured)` |
| `js-redirect@CWE-601` | `=` | `js-redirect@CWE-601` | `koa ctx.redirect`, `imported redirect`, `probe: ctx.response.redirect koa` |
| `?` | `js-exec-call@CWE-78+structured:os-command` | `js-child-process@CWE-78;structured-os-command@CWE-78` | `r2: reassigned regex receiver to child_process` |

#### py

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `py-eval@CWE-95` | `=` | `py-eval@CWE-95` | `eval`, `exec` |
| `py-os-command@CWE-78` | `=` | `py-os-command@CWE-78` | `os.system`, `os.popen` |
| `py-subprocess@CWE-78` | `=` | `py-subprocess@CWE-78` | `subprocess.run`, `aliased subprocess module`, `from subprocess import run`, `Popen`, `check_output` |
| `?` | `py-db-call@CWE-89+structured:sqli` | `py-db-call@CWE-89;structured-sqli@CWE-89` | `cursor.execute`, `objects.raw` |
| `py-sql-clause@CWE-89` | `=` | `taint-query-name` | `orm .filter clause`, `orm .where clause` |
| `py-sql-clause@CWE-89!suggestion` | `=` | `taint-query-value` | `orm .filter with an expression` |
| `py-sql-clause@CWE-89` | `=` | `py-sql-clause@CWE-89` | `sqlalchemy text with f-string`, `sa.text with concatenation` |
| `?` | `py-html@CWE-79+structured:xss` | `py-html@CWE-79;structured-xss@CWE-79` | `mark_safe`, `Markup` |
| `?` | `py-template@CWE-1336+structured:xss` | `py-template@CWE-1336;structured-template@CWE-1336` | `render_template_string` |
| `py-dynamic-attr@CWE-470` | `=` | `py-dynamic-attr` | `getattr with a computed name` |
| `py-redirect@CWE-601` | `=` | `py-redirect@CWE-601` | `redirect` |
| `py-file-open@CWE-22` | `=` | `py-file-open@CWE-22` | `open call` |
| `py-file-remove@CWE-22` | `=` | `py-file-remove@CWE-22` | `os.remove` |
| `-` | `=` | `-` | `get_user_id call (kernel pattern)`, `get_user call outside C`, `def get_user_id signature`, `getattr with a literal name`, `setattr with a literal name`, `compile`, `run name without subprocess binding`, `run call without subprocess binding`, `variable named source`, `variable named raw`, `revalidate (eval substring)`, `evaluate (eval substring)`, `render helper`, `data.find`, `socket send`, `text with a literal`, `text without sqlalchemy binding`, `execution_result variable`, `django filter with keyword lookups` |
| `-` | `=` | `-` | `filter_by with keyword lookups`, `lookup_where helper`, `redirect_url variable`, `freeze (free substring)` |

#### hybrid_py

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `py-os-command@CWE-78` | `=` | `py-os-command@CWE-78` | `import os as o`, `from os import popen`, `from os import system as sh`, `os.execvp`, `os.spawnlp`, `from os import execv`, `asyncio.create_subprocess_exec`, `asyncio.create_subprocess_shell`, `pty.spawn`, `probe: py control: os.system`, `probe: py import os as o`, `probe: py os.execvp`, `probe: py os.spawnlp`, `probe: py asyncio.create_subprocess_exec`, `probe: py asyncio.create_subprocess_shell`, `probe: py from os import popen` |
| `py-db-call@CWE-89` | `=` | `py-db-call@CWE-89` | `cursor.executescript`, `conn.exec_driver_sql`, `client.query f-string`, `client.query concatenation`, `client.query percent format`, `client.query .format`, `pd.read_sql_query`, `pd.read_sql`, `probe: py cursor.executescript`, `probe: py conn.exec_driver_sql`, `probe: py bigquery client.query`, `probe: py pd.read_sql_query`, `r2: query keyword f-string`, `r2: query reordered keywords`, `r2: query sql= keyword percent`, `r2: query lower var percent`, `r2: query CONST percent`, `r2: query CONST plus var` |
| `py-db-call@CWE-89` | `=` | `py-db-call@CWE-89` | `r2d: parenthesized implicit-concat SQL keyword`, `r2d: parenthesized concatenation SQL` |
| `py-db-call@CWE-89` | `=` | `taint-query-value` | `session.query(sql_var)` |
| `py-file-open@CWE-22` | `=` | `py-file-open@CWE-22` | `aiofiles.open`, `Path(x).open`, `probe: py aiofiles.open`, `probe: py Path(x).open` |
| `py-file-remove@CWE-22` | `=` | `py-file-remove@CWE-22` | `Path(x).unlink`, `shutil.rmtree`, `probe: py Path(x).unlink`, `probe: py shutil.rmtree`, `r2: reassigned list to os`, `r2d: tuple unpacking rebinds a list`, `r2d: walrus rebinds a list` |
| `?` | `py-file-open@CWE-22` | `browser-open` | `webbrowser.open` |
| `py-output@CWE-79` | `=` | `taint-write-data` | `tornado self.write`, `probe: py tornado self.write` |
| `?` | `py-output@CWE-79` | `taint-write-data` | `unknown receiver send`, `untyped file param write`, `r2: reassigned file-like to a socket-less client`, `r2d: with-as then plain rebinding`, `r2d: two with-as on the same name` |
| `py-template@CWE-1336` | `=` | `py-template@CWE-1336` | `jinja2 Template(x)`, `probe: py jinja2 Template(x).render` |
| `?` | `py-file-remove@CWE-22` | `taint-remove-name` | `list.remove on an untyped param`, `r2: AssetStore annotation remove`, `r2d: generic argument is not the base (Storage[list])`, `r2d: union with a non-collection alternative` |
| `?` | `py-eval@CWE-95` | `taint-eval-name` | `obj.eval(x)` |
| `py-eval@CWE-95` | `=` | `py-eval@CWE-95` | `builtins.exec` |
| `py-sql-clause@CWE-89!suggestion` | `=` | `taint-query-value` | `owner decision: orm .filter(expression) stays a weak TP`, `owner decision: select().where(expr)` |
| `py-sql-clause@CWE-89` | `=` | `taint-query-value` | `owner decision: django filter positional`, `probe: py django filter positional Q (expected sink per rule)` |
| `?` | `py-sql-clause@CWE-89` | `taint-query-name` | `owner decision: df.filter positional is accepted`, `probe: py pandas df.filter(items) FP check` |
| `-` | `=` | `-` | `logger.filter`, `builtin filter(fn, xs)`, `model.eval() zero args`, `platform.system() zero args`, `Image.open`, `session.query(Model attr)`, `session.query(Model)`, `list literal remove`, `list() constructor remove`, `set annotation remove`, `with open as f write`, `StringIO write`, `sys.stdout.write`, `sock.send`, `logger.write`, `re.compile (deliberate drop)`, `text with constant argument`, `probe: py builtin filter(fn, xs) FP check`, `probe: py logging Filter: logger.filter(record) FP` |
| `-` | `=` | `-` | `probe: py re.compile(x).search FP?`, `r2: unchanged list remove (single binding)`, `r2d: query keyword parameterized (tainted job_config)`, `r2d: query attribute arithmetic (tainted attribute operand)`, `r2d: single with-as file write stays suppressed`, `r2d: list[str] \| None annotation remove`, `r2d: Optional[set] annotation remove`, `r2: set[str] annotation remove`, `r2: typing Set[str] annotation remove` |
| `py-subprocess@CWE-78` | `=` | `py-subprocess@CWE-78` | `subprocess.call with an alias-resolved module`, `probe: py subprocess.call shell` |
| `py-file-remove@CWE-22` | `=` | `taint-remove-name` | `r2: conditionally reassigned list` |

#### go

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `go-db-call@CWE-89!suggestion` | `=` | `taint-query-value` | `db.Exec`, `field handle s.db.Exec`, `tx.QueryRow`, `generic constant query stays weak` |
| `go-db-call@CWE-89` | `=` | `go-db-call@CWE-89` | `QueryContext on any receiver`, `generic flat callee` |
| `go-writer-output@CWE-79` | `=` | `go-writer-output` | `Fprintf to http.ResponseWriter with constant format` |
| `go-writer-output` | `go-writer-output@CWE-79` | `go-writer-output` | `Fprintln to an io.Writer parameter` |
| `?` | `go-format-string@CWE-134` | `-` | `Sprintf with a non-constant format`, `Sprintf with a format read from a request field` |
| `go-format-string@CWE-134` | `=` | `go-format-string@CWE-134` | `Sprintf fed the tainted value as the format` |
| `go-template-html@CWE-79` | `=` | `go-template-html@CWE-79` | `template.HTML` |
| `?` | `go-exec-command@CWE-78+structured:os-command` | `go-exec-command@CWE-78;structured-os-command@CWE-78` | `exec.Command` |
| `go-file-op@CWE-22` | `=` | `go-file-op@CWE-22` | `os.Chmod`, `generic file constructor` |
| `go-redirect@CWE-601` | `=` | `go-redirect@CWE-601` | `http.Redirect` |
| `-` | `=` | `-` | `constant-format Sprintf`, `package-const format Sprintf`, `imported-package constant format Sprintf`, `versioned import package constant format`, `constant-format Fprintf to a file`, `sanitizedQuery name`, `InvalidQueryError name`, `URL.Query() call is a source, not a sink`, `variable named source`, `variable named raw`, `variable named run`, `variable named redirect`, `removeUser helper`, `Find on a list`, `sendEvent helper`, `field HTML read`, `func signature naming a sink word`, `freeze (free substring)` |
| `-` | `=` | `-` | `Writer-less Fprintln to a buffer`, `slice and map of sink-named functions` |
| `?` | `structured:path-traversal` | `structured-path-traversal@CWE-22` | `generic structured path` |
| `?` | `go-db-call@CWE-89` | `taint-query-value!suggestion` | `generic enclosing Exec is strong`, `non-generic enclosing Exec is strong` |

#### hybrid_go

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `go-db-call@CWE-89` | `=` | `go-db-call@CWE-89` | `sqlx Queryx`, `sqlx MustExec`, `sqlx NamedExec`, `sqlx QueryRowx`, `dbConn.Exec`, `handle *sql.DB Query`, `pgx r.pg.Exec`, `gorm chain Exec`, `method value query := db.Query`, `probe: go control: db.Query`, `probe: go sqlx db.Queryx`, `probe: go sqlx db.MustExec`, `probe: go dbConn.Exec`, `probe: go gorm db.WithContext(ctx).Exec`, `probe: go pgx r.pg.Exec` |
| `?` | `go-db-call@CWE-89` | `taint-query-name` | `Query on a non-db-named receiver` |
| `go-exec-command@CWE-78` | `=` | `go-exec-command@CWE-78` | `aliased os/exec`, `probe: go aliased osexec.Command` |
| `?` | `go-exec-command@CWE-78+structured:os-command` | `go-exec-command@CWE-78;structured-os-command@CWE-78` | `exec.CommandContext`, `syscall.Exec`, `probe: go syscall.Exec (structured)` |
| `go-template-html@CWE-79` | `=` | `go-template-html@CWE-79` | `aliased html/template`, `template.HTML`, `probe: go template.HTML via alias htmpl` |
| `?` | `go-template-html@CWE-79` | `taint-html-name` | `response.HTML helper keeps firing` |
| `?` | `go-file-op@CWE-22+structured:path-traversal` | `go-file-op@CWE-22;structured-path-traversal@CWE-22` | `os.Remove`, `os.RemoveAll`, `os.Rename`, `probe: go os.RemoveAll (structured)` |
| `go-file-op@CWE-22` | `=` | `go-file-op@CWE-22` | `aliased os Chmod`, `afero WriteFile`, `r2: afero.Fs Remove named cache`, `r2: afero.Fs Rename named cache`, `r2d: var-declared afero.Fs named cache`, `r2d: afero.NewOsFs() local named cache` |
| `?` | `go-file-op@CWE-22` | `taint-remove-name` | `store.Remove unknown receiver` |
| `go-format-string@CWE-134` | `=` | `go-format-string@CWE-134` | `Sprintf where a local shadows a const`, `r2: init-stmt shadows package const format` |
| `?` | `go-format-string@CWE-134` | `-` | `Sprintf where a param shadows a const`, `r2: range var shadows package const format` |
| `-` | `=` | `-` | `Sprintf with a const not shadowed`, `Sprintf with a function-local const`, `Sprintf through aliased fmt with a literal`, `u.Query() zero args`, `r.URL.Query() zero args`, `gin ctx.Query`, `echo c.QueryParam is another name`, `cobra.Command (imported package other than os/exec)`, `container/list Remove`, `lru cache Remove`, `gin ctx.Query(x) stays suppressed`, `field .HTML read`, `Writer-less Fprintln to a buffer`, `gorm Raw (not a base sink)`, `sqlx db.Select (not a base sink)`, `probe: go sqlx db.Select` |
| `-` | `=` | `-` | `probe: go gorm db.Raw`, `probe: go w.Write([]byte(x))`, `probe: go io.WriteString(w,x)`, `probe: go c.String gin`, `r2d: var-declared container type named cache`, `r2: untyped-name container Remove`, `r2: unshadowed package const format` |
| `?` | `go-template-html@CWE-79` | `taint-html-name` | `imported helper package HTML keeps firing` |
| `go-html-render@CWE-79!suggestion` | `=` | `go-html-render` | `gin ctx.HTML template render fires (weak-pending)` |
| `go-writer-output@CWE-79` | `go-template-html@CWE-79` | `go-html-output@CWE-79` | `echo c.HTML raw string fires`, `iris ctx.HTML raw markup fires` |
| `go-writer-output@CWE-79` | `=` | `go-html-output@CWE-79` | `probe: go fmt.Fprintf(w) writer` |
| `?` | `go-file-op@CWE-22` | `go-file-mode` | `r2: os.File typed named list`, `r2d: os.Open() local named list` |

#### other

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `legacy` | `=` | `legacy` | `C get_user`, `bash eval`, `bash source`, `bash find`, `strcpy`, `system` |
| `-` | `=` | `-` | `C get_user_id`, `java variable named source` |

#### hybrid_other

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `legacy` | `=` | `legacy` | `C sqlite3_prepare_v2`, `C++ sqlite3_prepare_v2`, `C chmod`, `C chown`, `C libcurl setopt URL`, `C fw_setenv`, `C mtd`, `C system`, `C++ chmod`, `C get_user`, `Lua uci:set`, `Lua os.execute`, `Lua loadstring`, `Rust Command::new`, `Rust curl handle`, `Bash eval`, `Bash uci`, `Bash sed`, `Bash xargs`, `Bash sudo`, `Bash chmod`, `Bash sqlite3`, `Bash curl`, `Bash wget`, `Bash ip`, `probe: c control: system`, `probe: c sqlite3_prepare_v2`, `probe: c chmod`, `probe: c chown`, `probe: c libcurl setopt URL` |
| `legacy` | `=` | `legacy` | `probe: c get_user (kernel)`, `probe: lua uci:set`, `probe: lua os.execute`, `probe: rust Command::new`, `probe: bash eval`, `probe: bash uci` |
| `-` | `=` | `-` | `Rust variable named source`, `Rust variable named ip`, `C variable named find`, `Lua variable named sed`, `Java variable named source`, `C get_user_id`, `C get_user_pages (deliberate: not a sink)`, `probe: c get_user_pages` |
| `-` | `legacy` | `-` | `probe: java Runtime.exec` |

### Enumerator candidates

The 300 labels below are source-derived design cases, **unverified by execution**. Each suffix is local
to its named file. Terra round 1 uses `terra-NN`, later rounds use `terra-rK-NN`. Own uses `own-N`.
`no` predicts no marked `taint_sink`, `[]` predicts an admitted sink with empty rules. A nonempty
cell specifies the required references if the stated shape is admitted. Validate presence against the
matrix base before promoting a candidate. A different base result is an admission discrepancy to
record, never authority to add a finding. The open products themselves are governed by 5.2.

| Expected references on marked finding | File and label suffixes |
|---|---|
| `js-response@CWE-79` | own-r1 (1) · terra-r1 (16) · terra-r2 (08, 09) · terra-r4 (04, 07) · terra-r5 (02) · sol-r1 (01, 02, 04, 08, 09, 22, 33, 35, 50) · sol-r2 (45, 51, 58) · astra-r1 (23) |
| `js-eval@CWE-95;js-response@CWE-79;structured-code-eval@CWE-95` | own-r1 (2) · terra-r1 (11) |
| `js-dom-html@CWE-79` | own-r1 (3) · sol-r1 (03) · astra-r1 (44) · astra-r2 (38) |
| `go-exec-command@CWE-78;structured-os-command@CWE-78;taint-process-argument` | own-r1 (4) |
| `py-db-call@CWE-89;structured-sqli@CWE-89` | own-r1 (5) · terra-r1 (07) |
| `py-os-command@CWE-78` | own-r1 (6) · terra-r1 (01, 02, 06, 14) · terra-r2 (02, 20) · terra-r4 (01) · terra-r5 (10) · sol-r1 (39, 40) · sol-r2 (38, 52) |
| `legacy` | own-r1 (7) · terra-r1 (17) · terra-r2 (17, 18) · terra-r3 (16, 17, 18) · terra-r4 (08) · sol-r1 (59) · sol-r2 (13, 14, 15, 16, 17, 18) · astra-r1 (36, 37, 38, 39, 40, 41) · astra-r2 (41) |
| `configured-sink;taint-exec-name` | own-r1 (8) |
| `no` | terra-r1 (03, 12, 13, 15) · terra-r4 (05, 09) · terra-r5 (07) · sol-r1 (29, 34, 43, 46, 60) · sol-r2 (19, 20, 26, 30, 48, 53) · astra-r1 (09, 10, 12, 14, 30, 33, 34, 35, 42, 43, 48) · astra-r2 (05, 10, 25, 26, 27, 29, 30, 32, 33, 39, 40, 42, 43) · astra-r3 (05) |
| `taint-write-data` | terra-r1 (04) · sol-r2 (01, 02, 03, 04, 05, 06) |
| `go-file-op@CWE-22;structured-path-traversal@CWE-22` | terra-r1 (05) · astra-r1 (46) · astra-r2 (08) |
| `[]` | terra-r1 (08) · terra-r2 (10) · sol-r1 (51, 56, 57) · sol-r2 (59, 60) · astra-r1 (04, 18, 31) · astra-r2 (09, 12, 16, 23) |
| `structured-ssrf@CWE-918` | terra-r1 (09, 18) · terra-r2 (03, 04, 05, 06, 07, 15, 19) · terra-r3 (09, 11, 12, 14) · terra-r4 (06) · terra-r5 (03, 04) · sol-r1 (14, 15, 23, 38) · sol-r2 (41, 42, 43, 44, 46, 47) · astra-r1 (22) · astra-r2 (24, 28, 35, 36) · astra-r3 (04) |
| `structured-deserialization@CWE-502;structured-ssrf@CWE-918` | terra-r1 (10, 20) · astra-r2 (06) |
| `structured-path-traversal@CWE-22` | terra-r1 (19) · terra-r2 (16) · terra-r3 (03) · terra-r5 (05, 06) · sol-r1 (25, 30, 32, 36, 44) · astra-r2 (11, 15) |
| `py-os-command@CWE-78;structured-deserialization@CWE-502` | terra-r2 (01, 11) |
| `py-os-command@CWE-78;structured-ssrf@CWE-918` | terra-r2 (12) · terra-r3 (08, 13) · terra-r4 (10) · sol-r2 (37) |
| `go-format-string@CWE-134` | terra-r2 (13) · terra-r3 (06) · sol-r1 (16, 17, 18, 19, 20, 21, 41) · astra-r1 (28) · astra-r2 (22) |
| `structured-deserialization@CWE-502` | terra-r2 (14) · terra-r3 (10, 15) · sol-r1 (53) · sol-r2 (56) · astra-r1 (13, 15, 24) · astra-r2 (31) |
| `structured-os-command@CWE-78;structured-path-traversal@CWE-22` | terra-r3 (01, 02) · sol-r1 (52) · sol-r2 (54) · astra-r1 (06) · astra-r2 (07) |
| `go-exec-command@CWE-78;structured-os-command@CWE-78` | terra-r3 (04) · astra-r1 (26) |
| `taint-process-argument` | terra-r3 (05) · astra-r1 (05, 25, 27) |
| `go-format-string@CWE-134;go-html-output@CWE-79` | terra-r3 (07) |
| `js-dom-html@CWE-79;structured-xss@CWE-79` | terra-r5 (01) · sol-r1 (37) |
| `py-os-command@CWE-78;py-subprocess@CWE-78` | terra-r5 (08, 09) |
| `js-redirect@CWE-601` | sol-r1 (05, 06, 07, 10, 11, 12, 13, 27) · sol-r2 (50) |
| `js-fs@CWE-22;js-response@CWE-79;structured-path-traversal@CWE-22` | sol-r1 (24) |
| `taint-query-value!suggestion` | sol-r1 (26) · sol-r2 (07, 08, 11, 12, 24, 33, 35) |
| `js-redirect@CWE-601;taint-write-data` | sol-r1 (28) · sol-r2 (36) |
| `js-response@CWE-79;structured-ssrf@CWE-918` | sol-r1 (31) |
| `go-format-string@CWE-134;taint-query-value!suggestion` | sol-r1 (42) |
| `go-file-op@CWE-22;go-format-string@CWE-134` | sol-r1 (45) |
| `structured-path-traversal@CWE-22;structured-ssrf@CWE-918` | sol-r1 (47, 48) · sol-r2 (39) · astra-r2 (19) |
| `js-dom-html@CWE-79;structured-ssrf@CWE-918;structured-xss@CWE-79` | sol-r1 (49) |
| `structured-xss@CWE-79` | sol-r1 (54) · astra-r1 (11) · astra-r2 (14, 37) · astra-r3 (03) |
| `go-html-render!suggestion` | sol-r1 (55) |
| `format-wrapper` | sol-r1 (58) |
| `go-html-render` | sol-r2 (09, 10) |
| `taint-query-value` | sol-r2 (21, 22, 25, 28, 31, 32, 34) |
| `js-redirect@CWE-601;taint-query-value!suggestion` | sol-r2 (23, 27) |
| `structured-ssrf@CWE-918;taint-query-value!suggestion` | sol-r2 (29) · astra-r1 (03) |
| `js-fs@CWE-22;structured-path-traversal@CWE-22` | sol-r2 (40) |
| `js-response@CWE-79;taint-write-data` | sol-r2 (49) |
| `go-format-string@CWE-134;structured-path-traversal@CWE-22` | sol-r2 (55) |
| `structured-deserialization@CWE-502;structured-xss@CWE-79` | sol-r2 (57) |
| `py-module-template@CWE-1336` | astra-r1 (01) |
| `js-db-call@CWE-89;taint-query-value!suggestion` | astra-r1 (02) |
| `go-exec-command@CWE-78;structured-os-command@CWE-78;structured-path-traversal@CWE-22` | astra-r1 (07) |
| `structured-sqli@CWE-89` | astra-r1 (08) |
| `js-dom-html@CWE-79;js-exec-call@CWE-78` | astra-r1 (16) |
| `js-dom-html@CWE-79;js-location@CWE-601` | astra-r1 (17) |
| `js-dom-html@CWE-79;js-fs@CWE-22;structured-path-traversal@CWE-22;structured-xss@CWE-79` | astra-r1 (19) |
| `structured-deserialization@CWE-502;structured-path-traversal@CWE-22;structured-ssrf@CWE-918` | astra-r1 (20, 21) |
| `js-fs@CWE-22` | astra-r1 (29) |
| `configured-sink` | astra-r1 (32) |
| `configured-sink;format-wrapper` | astra-r1 (45) |
| `js-location-header` | astra-r1 (47) |
| `js-child-process@CWE-78;js-db-call@CWE-89;structured-os-command@CWE-78;structured-sqli@CWE-89` | astra-r2 (01, 02) |
| `py-db-call@CWE-89;taint-query-value!suggestion` | astra-r2 (03) |
| `py-db-call@CWE-89;structured-sqli@CWE-89;taint-query-value!suggestion` | astra-r2 (04) |
| `py-html@CWE-79;structured-xss@CWE-79` | astra-r2 (13) · astra-r3 (02) |
| `js-fs@CWE-22;js-redirect@CWE-601` | astra-r2 (17) |
| `py-module-command@CWE-78;py-subprocess@CWE-78` | astra-r2 (18) |
| `js-db-call@CWE-89;structured-sqli@CWE-89` | astra-r2 (20) |
| `go-html-render;taint-query-value` | astra-r2 (21) |
| `go-file-op@CWE-22` | astra-r2 (34) |
| `structured-ssrf@CWE-918;structured-xss@CWE-79` | astra-r3 (01) |

Some candidate truth lines describe desired detection rather than this lane's admitted line:

- Terra 12, 13, r4-05, r4-09 and r5-07 put the only flat callee or assignment target on a different
  line. Go cannot supply a structured match on an argument-only line. Terra 03 and 15 put only bytes
  in `writeFileSync`. The JS wrong-index range covers its callee, so these isolated fixtures lack an
  admitting file hit. If an independent operation admits that line, the proven contents role in A applies.
- Terra 01/02 use Python's recognized placeholder/tuple exclusion. The shell receives the only
  publishable role, unlike an unsuppressed bind-value occurrence. Terra 17 uses Bash `printf`, also a
  legacy pattern eligible for a Bash word. Verify the incoming edge before treating it as an
  `unquoted_expansion`-only example.
- Terra r3-01/02 are Go framework-source lines. Both structured roles survive where the origins
  reach them, but the flat-source filter still applies. Sol r1-51 and Astra r2-09, 12 and 16 exercise
  shape-only admission without an indexed inline witness. Astra r2-05 has a live second pattern but
  lacks an independent admission. Astra r3-01–05 distinguish an inline source, a separately anchored
  incoming edge, a source-line flat filter and a missing Flask seed.
- A sender receiving a JSON object/array, an argv element, a writer handle, a Gin template selector and
  a substitution in a format call each have a different operand contract from raw HTML, command text
  or a format program. Fulfilling a flat predicate on their line does not grant that CWE. Compare
  Sol r2-01–06, 36, 49, 60, Astra r1-05, 25, 27, and the writer-first examples Terra r3-06 and
  Astra r2-22. Own-4 is a two-origin seed sketch: the executable and independent process argument
  are different roles, duplicate ids still collapse. Own-8 supplies no binding for `exec`.

## Appendix F. Accepted losses, follow-ups and the measurement contract

These are the lane's local follow-up entries. Each **filed** disposition in A–D has an entry here.
The controller can link external issues at implementation close-out. No external issue number is
asserted for a new entry. SQLModel's existing #646 is addressed by the corrected SQL dispatch.

| Entry | Published without a CWE | Evidence needed for a narrower rule |
|---|---|---|
| F1. Ambiguous callee or receiver | taint-exec-name, taint-eval-name, taint-query-name, taint-remove-name, taint-html-name, structured-name-match | Resolve the actual receiver/method and reaching definition, including conditional rebinding and function-local helpers. An unrelated import or a shared suffix does not establish the interpreter. |
| F2. Output, navigation and client requests | taint-write-data, js-write, go-writer-output, go-html-render, js-location-header, browser-open, external-open, structured-client-request | Establish content type/escaping, downstream consumer, redirect status, handling of the target or a server-side request. Bytes at a fixed destination, a template name, a browser launch and a browser-only request do not establish those weaknesses. |
| F3. Query values and process arguments | taint-query-value, taint-process-argument | Determine whether the reached value can become SQL or shell syntax. Bound values, positional ORM objects and fixed-program argv do not by themselves select that syntax. Retain real weak emitters and their relative severity. |
| F4. Dynamic attributes and file mode | py-dynamic-attr, go-file-mode | Establish use of the selected attribute for dispatch, or the file's effective permissions and policy. A computed read and a permission mask alone do not establish unsafe reflection or path traversal. |
| F5. Name-only detectors | configured-sink, format-wrapper, legacy | A configured name has no declared weakness. The wrapper scan proves one-hop variadic-plus-printf co-occurrence, not the forwarded formal or slot. The legacy list has no operand model. Per-user ids would also make the registry open. |
| F6. Missing counterparts without a manual obligation | Every dash in D's 65-row pair table, individually named there | Separate mixed acquisitions/uses, scope-managed resources, intentional application lifetimes and infrastructure policy. The last column records what each pair actually establishes. |
| F7. Repeated counterparts | absence-repeated-counterpart | Establish two matching deallocations or unlocks, their common operand and the intervening lifetime before specializing. |

**Accepted detection and attribution losses**

- A first cleansed/excluded structured outcome can prevent an otherwise live finding. Go's start-line
  rule, flat-only continuation with no local admission, HCL cross-block flow (`ri27`), and absent
  framework/IPC source witnesses can also leave no finding. An existing finding admitted only by an
  unrelated sibling or by Go's no-path shape may have empty rules. Keep the Java probe as a separate
  absence control: its non-admission cause has not been established.
- Missing, coalesced or conflicting byte/binding evidence can omit a genuine occurrence. Syntactic
  containment can count a nested result even when its runtime value is independent. Current positional
  indexing can miss reordered/splatted arguments, comments in Go slots, interpolated Python strings
  and JS Function bodies outside the structured index. An existing matcher or exclusion can also miss
  a real call (including a second-hop wrapper or Flask inline template source without an incoming edge).
  Do not infer unobserved yields from an enumerated snippet.
- Full safe-call ranges can remove a nested flat match. Category-based/path-wide cleansing can
  suppress an operation whose public CWE differs from the sanitizer category. Python's start-line
  cleansing and span-based admission can disagree on continued calls. Use the per-call guard for
  attribution on admitted lines and record that discrepancy separately. Source-line flat exclusions
  and the two SARIF results from distinct established detectors are deliberate.
- The no-CWE rows lose weakness filtering in specific-mode SARIF. Neutral rows shared across flat and
  structured detectors lose that distinction. Scope-exit advisories and a generic repeated call can
  therefore produce accurate detector text without a CWE tag. Under #841, overlapping runs retain the
  baseline rule set, even if context found an additional rule.

**Measure before adopting a refusal.** Stage M reports two base runs of `9d1a7d06f653` on the same
35 inputs (pinned tree `6a5a242ad8e7`), 210 output pairs, 81 **all-category** taint findings,
6 primitives and 34 absence findings. These are denominators, not counts of sinks, lost identities
or untagged pairs. At S2 run both projections on the same admitted keys/paths from the pinned inputs
before locking the rules. Compare old line/any-argument/first-pattern candidates with the D result.
Count candidate **occurrences** by input, detector, role and reason, with distinct affected finding
keys and one worked location per nonzero reason. Count flat/structured/inline candidates separately.
Include old-only, D-only, kept, neutralized-CWE, wrong slot, ambiguous binding, cleansed/safe range,
framework filter, same-id collapse, weak override and admitted-empty. Count unadmitted candidate
keys by admission reason separately from admitted empties. For cross-run union compare overlap and
expanded-only keys. Preserve the base/head presence and line-severity multisets; use the M comparator
for all other bytes and fingerprints. An instrumented identity comparison is diagnostic only.

At S5 tabulate all six primitive categories and the 34 baseline absence findings by pair/category,
ordinary/repeated row, specific-CWE/neutral, and the reason for a generic repeat. Count emitted
findings with multiplicity (multiple pairs can hit one location). Compare with the same base and run
the explicit G-targets and both SARIF modes. Record zeros and absent classes. Where a pinned input
has no witness for a branch, report that and run the named positive/negative fixture separately.
No execution or post-change yields are claimed by this design.

**Change log, 2026-11-24.** Stage D fixed the publication tables, both open-grid rules, all 540 TSV
decisions and the 300 source-only candidate dispositions at design HEAD `457ea9ffa6de`. The matrix
checkout advanced from `a3986ce333f1` to `5e9e4d864cbb` during D. Stage M used a separate binary.

# Lane 7: rule identity on findings — design

Status: architecture final; the rule names and the non-taint rule list are fixed at stage D after the
matrix (PLAN stages T and E). No tracking issue exists yet; the plan files one. Research record:
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

Which lines become `taint_sink` findings, and their severity, category and description, do not change.
Rules are attributed only on lines the existing code already inserts, **per sink occurrence, not per line**:
a sink's rule is attached only where the tainted use (the edge's target `VarLocation`, with its bytes)
reaches that sink's operand as far as the existing code can tell. A sibling sink on the same line lends
neither its rule nor its strength.

- **Flat hits.** From each tainted use, walk up its enclosing calls (across lines, up to the enclosing
  statement), and to the assignment for the DOM and location sinks. An enclosing call whose callee
  identifier the classifier reports as a hit, with the use inside its argument list, contributes that hit's
  rule; an assignment sink contributes when the use is in its right side. The classifier has no per-rule
  argument index, so any argument counts. `res.send(eval(x))` therefore carries both rules, while in
  `cur.execute("SELECT 1"); os.system(x)` only the command's rule is attached. A call continued over several
  lines carries the same rules as the same call on one line (the walk starts from the use, not from the
  identifiers that start on the finding line). An enclosing call inside a range the cleansing code reports
  for that edge (`cleansed_structured_sink_call_ranges`) contributes nothing.
- **Caller-supplied and wrapper names** (`TaintConfig.extra_sinks`, detected variadic wrappers) are name
  matches: they contribute the rule stage D fixes for them only as an enclosing callee whose argument list
  holds the use.
- **Structured matches.** Every matching pattern is collected, not only the first: each language's outcome
  function becomes the first element of a function that returns all matches in the same order (special
  outcomes first), so presence is computed exactly as today, and every match that the cleansing code does
  not suppress for that path contributes its rule. The matcher ties a pattern's tainted argument to the path
  by variable name and line, not by bytes; that is as far as the code can tell, and two same-named uses on
  one line are not told apart.
- **Lines inserted outside the edge loop** (JS YAML inline sources, Python inline Flask sources, Go source
  lines): the patterns whose tainted argument holds the inline framework-source call or, for a Go source line
  with originating paths, the uncleansed matches of those paths. A Go source line with no originating path
  is matched without an argument check, so only patterns whose tainted argument holds the inline source call
  contribute.
- **When the code cannot tell.** A `legacy` hit (C, C++, Java, Rust, Lua, Terraform/HCL, Bash) has no operand
  model; `legacy` names the detector and asserts no weakness, so it is attached to every finding with a
  legacy hit. Any other hit that no tainted use reaches contributes nothing. A finding none of whose hits is
  reached (main admits the line through a sibling) keeps an empty list; S2 reports how many there are.
- **Per-rule severity.** The finding keeps today's line-level severity. A rule all of whose attached hits are
  weak (a structured SQL match takes the strength of the flat hit on its own call; other structured matches
  are strong) carries `severity: "suggestion"` when the finding is `warning`; otherwise the key is absent.

A structured match's rule comes from one function of the matched pattern and its call (`structured_rule`),
not of the category alone: the default is `structured-<category>` (`structured-xss`, `structured-sqli`,
`structured-ssrf`, `structured-deserialization`, `structured-os-command`, `structured-path-traversal`) with
the category's CWE (79, 89, 918, 502, 78, 22), and stage D lists the patterns whose operation differs (the
JS code-evaluation rows of the deserialization table, Mongoose `$where`, the `exec` pattern on a database
handle, where the flat classifier's verdict on the same call can decide). The
category, which drives cleansing, does not change. `taint_source` and `unquoted_expansion` findings carry
none.

**No known-wrong identity is published.** Stage D fixes, for every rule the registry will hold, the expected
identity from the operation and the operand, beside the value the code emits today. Where they differ, the
lane either corrects the value (a tag, a CWE, the structured mapping) when that changes no finding's presence
or severity, or files an issue and publishes the rule without a CWE until it is corrected. Three differences
are known before D: JS `eval`/`Function`/`vm` runners as structured deserialization (502), the structured
command pattern on SQLite `db.exec(sql)` (78), and `py-eval`/95 on SQLModel `session.exec` (#646).

PR #841 and lane 4's sink slice edit the same loop. Lane 4's sink reports `js-proto-write`/CWE-1321 through
this field, never through the description (lane 4 DESIGN decision 9 is to be amended accordingly). If this
lane's S2 merges first, lane 4's slice adds its rule to the line's rules and keeps the existing description of
a line that already held a sink; this lane's S3 adds the registry row if lane 4's sink has merged by then. If
lane 4's sink merges first, this lane's S2 owns the migration: it attaches `js-proto-write`/CWE-1321 from lane
4's side set to the rules of those lines, adds the identity lines for lane 4's test rows, leaves the
descriptions as lane 4 wrote them (this lane changes no description), and files the issue for lane 4 to drop
the tag from its description and read the field in its benchmark join.

### 5.3 The registry

`crates/prism-review/src/output/rule_registry.rs`: `RuleInfo { id, cwe, name, short, help, algorithms }`
rows, one named constant per rule, collected in one table sorted by id. Closure is by construction, not by scanning for literals: a built-in producer names a rule
only through its constant (`SinkHit` holds a `&'static RuleInfo`, `structured_rule` returns one, primitive
and absence use theirs), and a `RuleRef` is made only by `RuleInfo::rule_ref`, which copies the row's CWE,
so a computed id cannot compile and a tag cannot carry a CWE other than its row's. Tests: no production
source outside the registry builds a `RuleRef` by hand (a source scan, with a synthetic `format!`-built id
as its failing control); `check_emitted(algorithm, &RuleRef)` rejects an unknown id, a CWE that disagrees
with the row and an algorithm the row does not list (one failing control each), and every executed
positive row of the matrix passes its emitted references through it; ids are unique and CWE strings are
`CWE-<digits>`. External or unrecognised ids (an embedder's findings) keep their own serialisation path in
SARIF (5.5). `docs/rules.md` is generated from the table by a test-checked generator (the test fails when
the page is stale).

### 5.4 Other algorithms (second slice, list fixed at stage D)

- **primitive:** the rule id is the existing category; the registry adds its CWE where the fingerprint is a
  recognised weakness (weak hash, hash truncation, disabled certificate validation, hard-coded secret,
  shell interpolation).
- **absence:** one rule id per pair, a slug fixed at stage D and stable afterwards (not regenerated when the
  description is edited), with a CWE only for the standard resource classes (missing release, missing
  unlock; a lock released at scope exit is not one). `double_close` fires for any pair, so its id and CWE,
  and the mapping of the six primitive categories to weakness classes, are fixed at stage D. The pair table gains an id column; the
  description text does not change.
- Everything else keeps an empty list.

### 5.5 SARIF

- Always: each result gains `properties.rules` (the rule references) when the finding has any (decision 7).
  Rule ids, descriptors and fingerprints are unchanged, so alert history is unaffected.
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
- **Name-level structured attribution.** Structured matches tie the argument to the path by name and line,
  so two same-named uses on one line are not told apart (5.2).
- **Main defects found by the design review, filed and not fixed here:** the first structured match decides
  presence, so a line whose first match is cleansed and whose second is live gets no finding unless another
  sink admits it; Python call-level cleansing requires the call to start on the finding line while matching
  uses containment (a continued call's guard can be skipped); the identity differences stage D records.

Recorded, not pursued:

- Adding a field breaks a downstream full struct literal of `SliceFinding` at compile time; the API promises
  it only as a handle to read and pass back.
- Two identities of one SQL operation (flat and structured) give two `specific`-mode results at one location;
  stage D may give them one shared rule if it chooses.

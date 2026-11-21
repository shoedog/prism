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
fingerprints, and no change to SARIF rule ids unless the user asks for it.

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| `SliceFinding` has ten fields (algorithm, file, line, severity, description; and, skipped when empty: function name, related lines, related files, category, parse quality, diagrams). It is not `#[non_exhaustive]` and has no constructor or `Default`. There are 27 production struct literals (contract 12, absence 4, taint 3, callback dispatcher 2, one each in primitive, provenance, echo, symmetry, membrane, peer consistency) and about 15 in tests. The public API documents it as a handle to read and pass back. | `src/slice.rs:23-44`; `src/api/` module docs |
| The taint sink classifier returns a rule tag and a CWE per hit (about 38 tags: `js-dom-html`/79, `js-child-process`/78, `js-eval`/95, `py-template`/1336, `go-format-string`/134, …; other languages get `legacy` with no CWE). Nothing outside its own tests reads them: the sink loop turns the hit into a boolean and records only the line. Structured sink tables carry a category (six values: XSS, SQL injection, SSRF, deserialization, OS command, path traversal) and no CWE as data. A line can hold several hits. | `crates/prism-review/src/algorithms/taint/run.rs:258`; `taint_sinks/`; `src/frameworks/mod.rs` |
| Other algorithms: primitive already stores its rule name in `category`; absence's pair identity (66 pairs) and provenance's origin live only in the description text; contract kinds are a category suffix. | research section 2 |
| SARIF: `ruleId` is `prism/{algorithm}/{category}`; one rule descriptor per distinct id, with id, name, descriptions and `properties {algorithm, category}`. No tags, no help text, no taxonomies. The stable fingerprint hashes algorithm, category, file, function name, the description with digit runs masked, and the source line's text; the rule id is not an input. An earlier design note records "rule ids keep category strings verbatim; stability wins". | `crates/prism-review/src/output/sarif.rs:377`, `sarif_rules.rs` |
| Targets: the row id hashes category, description and severity among nine fields; the schema is closed, so a new row property needs a new contract minor. | `crates/prism-review/src/targets/mod.rs:279` |
| JSON and review output serialise the struct, so a new field that is skipped when empty appears automatically. The category strings `taint_source` and `taint_sink` are read by the review renderer, the confidence classifier and the targets mapping. | `crates/prism-review/src/output/review.rs:164` |
| Pinned by tests: exact SARIF rule ids and fingerprint presence; the targets id recipe and schema; review-compact key presence. | `sarif_test`, `sarif_shape_test`, `targets_mapping_test`, `review_compact_test` |

Inferred from GitHub's documented behaviour, not from this repository: code scanning takes CWE labels from a
rule descriptor's `properties.tags` (`external/cwe/cwe-79`), and identifies an alert by rule id together
with its own location hash, so changing a rule id starts a new alert history.

## 3. Goals, non-goals, anti-goals

Goals:

1. A finding carries zero or more rule references (rule id, optional CWE).
2. Taint findings carry the rules of every sink hit on their line, flat and structured.
3. SARIF output can present one descriptor per rule with CWE tags and help text, on request; by default it
   keeps today's rule ids and adds the rule references to each result's properties.
4. One registry lists every rule id with its CWE, name and description; a test proves that every rule the
   code can emit is in it; a generated page documents it.
5. Fingerprints, target ids, finding counts and default SARIF rule ids are unchanged, proven by comparison
   with main on fixed inputs.

Non-goals: new sinks or new rules; CWE labels invented for algorithms that have no weakness class (contract,
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
without a rule and skipped in serialisation when empty, so existing output stays byte-identical for those.
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
}
pub struct SliceFinding {
    // existing fields unchanged
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleRef>,                          // sorted by id, no duplicates
}
```

`SliceFinding` gains `Default` and stays constructible by struct literal; the 27 production literals and the
test literals add `rules: Vec::new()` (or `..Default::default()`), in a first commit that changes nothing
else. The field is additive under the API's handle promise.

### 5.2 Taint emission

The sink loop keeps what it already computes: for each sink line, the set of rule references from every
flat hit (`SinkHit.rule`, `SinkHit.cwe`) and the structured match of each edge (the matcher returns the
first matching pattern per edge; that one is kept, and reporting all of them is a follow-up). The loop also
adds sink lines that have no hit: YAML sinks, Python inline Flask sinks and Go source lines take the
structured rule of their category; lines added through caller-supplied or wrapper sink names get the rule
fixed at stage D. A structured match's rule id is
`structured-<category>` (`structured-xss`, `structured-sqli`, `structured-ssrf`, `structured-deserialization`,
`structured-os-command`, `structured-path-traversal`) with the CWE of its category (79, 89, 918, 502, 78,
22), from one function. A hit from a language without positional rules is `legacy`, no CWE. The emission
block attaches the line's set to the `taint_sink` finding. `taint_source` findings carry none. Severity,
category and description are untouched.

PR #841 and lane 4's sink slice edit the same loop. This lane should merge before lane 4's sink slice, which
then reports its rule through this field instead of the description (lane 4 DESIGN decision 9).

### 5.3 The registry

`crates/prism-review/src/output/rule_registry.rs`: one table of
`RuleInfo { id, cwe, name, short, help, algorithms }`, sorted by id. Tests: every rule tag literal
in `taint_sinks/` (the `js-…`, `py-…` and `go-…` strings, which reach `SinkHit` through local helper
functions and tuples, not only through `SinkHit::new`) matches a registry row, found by a source scan in the
test, so a new rule cannot be added without a row; every structured id and `legacy` are rows; every primitive category is a row; ids are unique
and CWE strings are `CWE-<digits>`. `docs/rules.md` is generated from the table by a test-checked generator
(the test fails when the page is stale).

### 5.4 Other algorithms (second slice, list fixed at stage D)

- **primitive:** the rule id is the existing category; the registry adds its CWE where the fingerprint is a
  recognised weakness (weak hash, hash truncation, disabled certificate validation, hard-coded secret,
  shell interpolation).
- **absence:** one rule id per pair, a slug of the pair's description, with a CWE only for the standard
  resource classes (missing release, missing unlock). `double_close` fires for any pair, so its id and CWE,
  and the mapping of the six primitive categories to weakness classes, are fixed at stage D. The pair table gains an id column; the
  description text does not change.
- Everything else keeps an empty list.

### 5.5 SARIF

- Always: each result gains `properties.rules` (the rule references) when the finding has any. Rule ids,
  descriptors and fingerprints are unchanged, so alert history is unaffected.
- `--sarif-rule-ids specific` (default `category`): a finding with rules is written as one result per rule,
  with `ruleId` `prism/{algorithm}/{rule id}`; descriptors come from the registry with `name`, short and
  full descriptions, `help`, and `properties.tags` holding `external/cwe/cwe-<n>` when the rule has a CWE.
  The fingerprint is unchanged (it does not include the rule id), so the two results of one multi-rule
  finding share it and differ by rule id. Findings without rules are written as today.
- The flag's help text says that switching modes starts new alert histories for the affected findings.

### 5.6 Review and JSON

`--format json` and `--format review` both serialise the finding, so the `rules` key appears in both and is
omitted when empty. No text output prints findings line by line, so there is no separate review rendering.
One byte-golden that holds a taint finding (`tests/fixtures/review_no_diagrams/golden/review_unflagged.json`)
changes in the taint slice and is re-recorded there.

### 5.7 Targets

No change: ids hash neither the new field nor anything this lane touches. A test compares target ids with
main's on the targets fixtures.

## 6. Gates

| Gate | Statement |
|---|---|
| G-additive | on a fixed fixture set, `--format json` and `--format review` output with the `rules` keys removed equals main's output byte for byte, after dropping the keys that differ between any two builds (the build identity, git revision and dirty flag) |
| G-fingerprint | the multiset of SARIF fingerprints in default mode equals main's; default-mode SARIF with `properties.rules` and the per-build run properties removed equals main's byte for byte |
| G-targets | target ids on the targets fixtures equal main's |
| G-closed | registry tests of 5.3 pass; in `specific` mode every result's rule id has a descriptor, and every descriptor with a CWE has the tag |
| G-coverage | every row of the existing sink-rule test tables (`tests/algo/taxonomy/taint_sink_rules_*`) asserts the finding's rule id and CWE, not only that a sink fired |
| G-sarif-valid | `specific`-mode output passes the repository's SARIF shape tests and, if a validator is available offline, the SARIF 2.1 schema |

## 7. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Default SARIF rule-id mode | `category` (today's ids; alert history preserved); `specific` is opt-in | flipping the default later is one line and a release note; every user of the default then gets new alert histories once | at any release |
| 2 | Several rules on one line | one finding with a list; SARIF `specific` mode writes one result per rule | — | before S1 |
| 3 | CWE for structured sinks | from the category, by one function | per-pattern CWEs would need a field on the pattern tables in the core crate | stage D |
| 4 | Non-taint algorithms in this lane | primitive and absence | others can be added one registry row at a time | stage D |
| 5 | `security-severity` in SARIF | not emitted (no scores are invented) | adding a fixed band per severity is a small change | — |
| 6 | Rule and CWE in the targets document | not added (closed schema; needs a contract minor) | a later minor can add it; ids would not change | when a consumer asks |

## 8. Risks and accepted limits

- **Twenty-seven struct literals change in one commit.** It is mechanical and reviewed as such, with no
  behaviour change in that commit.
- **`legacy` hits have no CWE.** Languages without positional sink rules get a rule id that says so.
- **Rule ids become a public surface.** Renaming one later is a breaking change for anyone filtering on it;
  the registry test and the generated page make renames visible in review.
- **Other output consumers.** Nothing in `eval/` parses finding JSON by key list, and the sibling
  fault-injection product reads targets only, which do not change.

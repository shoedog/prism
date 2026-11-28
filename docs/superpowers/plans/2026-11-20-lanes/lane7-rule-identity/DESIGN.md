# Lane 7: rule identity on findings — design

Status: D-7, 2026-11-25. The §8 review of D-6 at `45264869` returned B REJECT and A APPROVE.
This fold awaits owner disposition for dormant S2-W at `b3bfd5515ece` (PR #901).
S1 is present. Sections 5.2–5.5 and Appendices A–F
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
4. Built-in projections select named registry rows. Each emitted id and CWE must agree with its row,
   and the row must list the finding's algorithm. Section 5.3 defines how each condition is checked.
   A generated page documents the rows. External SARIF input follows 5.5.
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
// src/rule_registry.rs, re-exported at prism::slice::RuleRef
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleRef {
    id: String,                                       // e.g. "js-dom-html"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cwe: Option<String>,                              // e.g. "CWE-79"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    severity: Option<String>,                         // only when lower than the finding's (5.2)
}
// src/slice.rs
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

**Round 1 impact before adoption.** Nine SQL and two redirect identities change among 540 TSV keys.
One of 300 candidate identities changes (`sol-r2-50`). E records each old value. Flask and mux
change zero TSV cells: the sole template call reaches positional source and no fixture uses mux.Vars.
Both bound cross-spawn rows retain their ids. C covers six primitives and D revisits 24/65 ordinary
pairs outside the taint matrix. Stage M counted 81 all-category taint, six primitive and 34 absence
findings. Per-witness yields are unknown. Each producer measures its projection before activation (F).

**Round 2 impact against D-2 (`be906634a77c`).** The changes to Flask context and Python SQL
preserve all 540 matrix and 300 candidate expectations (E). The shell, TLS and credential predicates
affect three of six primitive categories. The cleanup predicate applies to all 22 conditional absence
pairs. Before publication measure how many of the six primitive and 34 absence findings move from
tagged to neutral, using F. Specific SARIF uses the fallback id for every empty normalized list,
including older and external inputs. This changes no taint expectation cell.

**Admission and attribution have different inputs.** Keep the existing `sink_lines`, first-outcome
priority (including special early exclusions), source/scope filters, evidence, order and line severity.
For a physical `(file, line)` that survives admission, collect rule evidence from **all** paths and inline
sources for that key. A path can supply a rule even when another path, processed later, admits the line.
Buffer candidates and intersect with the final set of admitted keys. An empty set on an admitted line
serializes with the `rules` key omitted. No identity creates, moves or splits a finding.

1. Identify the occurrence by file, AST span, callee/operator and operand span. Start a flat walk at
   each read proved under step 2, through enclosing operations up to its statement or callable boundary.
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
   the reached operand by the D-6 proof below. The path nominates a definition and a physical line.
   Representative order, syntactic proximity and equality of two `VarLocation` keys have no evidentiary
   weight. The same proof applies to the first read and to every sibling. A nested result inside
   a consuming operand counts under the existing syntactic expression-containment model.
3. Check cleansing on the **particular path, call and pattern**, using its original sanitizer category.
   A public reclassification does not change which guard admits a finding. Apply safe-call ranges to
   the flat callee/target identifier (a range may cover just the callee), and apply call-specific
   cleansing to each structured match. An outer safe-call range can exclude flat identifiers inside it.
   A reached structured inner call gets its own check, including when its outer SQL call binds a
   parameter. A live call elsewhere cannot uncleanse its sibling. A callee-only index miss excludes the
   corresponding indexed role. A distinct proven role, e.g. file contents, may still be named if the
   line is admitted. For Flask's own context exclusion, derive the projection decision per operand
   from B's bound source and scoped output use. A reached source qualifies for `py-template` and
   `structured-template`. A reached context key proved to render unescaped qualifies for `py-html`
   and `structured-xss`. Exclude an unproved context key independently. The admission helper's
   index-0 range can cover the whole call in a keyword permutation. Applying it to either proved
   role would lose the flat id. Preserve other cleansing, enclosing safe ranges and the independent
   framework-source flat filter. An inline witness alone supplies only structured evidence.
   A full parameterization, safe-loader, literal-binary or category guard continues to exclude the
   affected call's fallback. A line with only a cleansed first match and no other admission stays absent.
4. Preserve the framework flat filter: on a Go/Python framework-source line it excludes flat hits even
   from incoming edges. On JS-family source lines use the existing receiver, source/target byte ranges
   and tainted-argument/RHS checks. A Flask diff seed is not necessarily a detected framework-source
   line. For JS YAML, enumerate unsafe calls with an inline request in the document slot, keyed to the
   call's opening line. For Flask, enumerate all qualifying inline calls spanning the selected source line,
   binding source and context independently as in B for both path and inline evidence. For Go source lines union **all**
   uncleansed matching originating paths **and** inline witnesses even if a path already succeeded.
   For an originating path, carry the admitted `(file, source line)` where the call starts and the
   path's proved read span and line separately. With `x := c.Param("f"); os.Remove(` on the source
   line and `x,` on the next, an uncleansed path to that read in slot 0 selects
   `structured-path-traversal@CWE-22` on the opening line. Verify both operand checks on that same
   read in that same call. The path grants no admission on the continuation and no flat credit on
   the source line. A cleansed path or a read outside the selected operand contributes nothing.
   With no originating path, `go_sink_outcome(None)` can admit the line on shape alone. A rule then
   requires a descendant framework-source call in the selected operand and the existing source-line guard.
   For a method source resolve its receiver to the enclosing handler's framework parameter. For the
   free function `github.com/gorilla/mux.Vars` resolve its actual argument 0 to the incoming value
   of the matched `*http.Request` parameter, directly or through a unique alias. A reassigned formal,
   shadow, fresh request or unrelated binding fails. Apply this to inline contributions on **both**
   the primary and secondary routes. Keep independently sourced paths. A path seeded solely by the
   same unbound request is no corroboration. A binder-exists test accepts `mux.Vars(&http.Request{})`.
   Dropping all free-function witnesses would lose the bound `mux.Vars(r)` in `astra-r2-15`.
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
RHS/JSX values. Ordinary edges nominate reads on their own destination line.
Test the outer and inner slots and their guards separately. First/middle/last use, opening/closing
delimiter alone, split leaf, extra admitting sibling, clean literal/independent inner argument, weak SQL,
sandbox and Gin follow the same procedure. Two use lines have two independent decisions.
Cross-line credit goes to the existing insertion key on the specified inline routes and on Go's
structured originating-path route in step 4. For the latter the source key must equal the call's
opening key, and the path must prove the actual read in its eligible operand. The
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

The merged #841 union retains the baseline finding on overlap and the expanded finding on an expanded-only
key. Lane 4's side-set rule is `js-proto-write@CWE-1321` for a reached prototype write. If lane 4 lands
first, S2-JS migrates its side-set evidence, preserving descriptions, with its registry row. If it lands
second, lane 4 selects the named core row in the same PR that publishes it. Its #603 contract
requires every built-in ref to come from a core row. S2-R is a dependency for publication.
Lane 4 owns any description and benchmark-join migration.

#### Collector attribution options (design escalation 2)

**Independent options, before the pooled proposals and reviews.** The representative bytes
identify a token. They do not establish which definition the token reads. Operand meaning is a
second question even after reaching value has been proved.

1. Retain the present admission graph and produce a separate occurrence relation at data-flow
   construction. Key reads by file version, callable, binding, access path and half-open span,
   and associate each read with its reaching definition(s) *before* any line reduction.
   This preserves sibling and continued reads and gives the collector evidence it can audit.
   Copying every old line edge onto every read would preserve the original error. The producer
   needs a sound binding/transfer contract, versioned cache and a corpus compatibility run.
2. Ask each language for a projection-only proof on demand. Accept a closed grammar of reads,
   bindings and effects, with a proof from the path definition to the candidate read. An
   unclassified effect invalidates the proof. A language can implement a straight-line fragment
   early and add other constructs under separate tests. This avoids migrating the graph but
   duplicates flow reasoning and makes the unsupported-fixture census part of each activation.
3. Restrict the projection to explicit inline sources and uniquely attributable path uses.
   Suppress a coalesced group unless its binding and value at the particular read are proved,
   including for the representative itself. This is small and gives a meaningful lower bound,
   but would discard distinct live sibling roles and could weaken the id on a mixed-strength line.
4. Make the detector return a located call/target and its operand role on a read-only visit.
   Preserve the actual matcher index, binding, guard and selected line alongside that role.
   This handles language-specific call syntax and avoids a generic guess at a callee leaf.
   It composes with any of the first three choices. On its own it cannot distinguish two
   bindings or two values of one binding.

**Measured policy comparison.** At `b3bfd5515ece`, compare sorted (id, CWE, relative level)
lists at the marker in all nine TSV tables with the source bodies, and join all eleven enumerator
files by label to E's **design** value. Count a label once. Keep all line-admission decisions,
independent origins, actual inline roles, cleansing and the legacy line scan. Collapse ids and
recompute relative strength *after* removing witnesses. This is a static counterfactual on 540
keys and 300 specified shapes, conditional on their stated admission. It is not a corpus yield.
For the outline `own-8` use its two calls in the order written with one incoming local `x`.

| Policy applied to the same shapes | Changed TSV labels | Changed candidate labels |
|---|---:|---:|
| P3 strict physical-line uniqueness, withhold a repeated path group except when every read belongs to one and the same operand | 12 | 31 |
| P3 retaining the representative, or P6 keeping its own enclosing chain, taking the first source read as representative | 6 | 26 |
| P2 on once-bound simple locals, first exact use otherwise, with fields restricted to the retained read | 0 | 1 |
| D-6 single-definition proof for simple locals, exact independent witnesses and repeated-field limit | 0 | 1 |
| Full independently bound occurrence edges (own 1 / P4), including separately validated request reads | 0 required | 0 required |

P3 strict changes identity `ri02,07,15,21a,21b,22,23c` and hybrid_py `os.execvp`,
`os.spawnlp`, `from os import execv`, `probe: py os.execvp`, `probe: py os.spawnlp`. In those
five rows the same name occurs in distinct actual arguments. P6 retains their program selector
and one of the identical `ri07` calls. Its six changed identity rows retain respectively DOM HTML,
empty (guarded file), SSRF, deserialization, empty (cleansed HTML), and configured only.

| Candidate file | P3 strict changed suffixes | Of these, equal to E again with the first read under P6 |
|---|---|---|
| own-r1 | 8 | — |
| terra-r1 | 01, 02, 09, 14 | 02, 09, 14 |
| terra-r2 | 01, 13, 20 | 13 |
| terra-r3 / r4 / r5 | 07 / 10 / 02 | — |
| sol-r1 / r2 | 49, 50 / 45, 55 | r2-45 |
| astra-r1 | 01, 02, 03, 16, 20, 21, 24, 47 | — |
| astra-r2 | 01, 02, 03, 04, 17, 18, 19, 21 | — |
| astra-r3 | — | — |

Different representatives can change the P6 list. For instance, in `astra-r1-01` the sandboxed
first use alone leaves a suggestion override on the warning, while the second use restores the
strong id. In `sol-r2-55` the independent `raw` path survives either policy. Several siblings with
different names and one nested read with several enclosing roles do not imply coalescing. The two
open grids have no measured execution denominator. Use the same policy on every generated coordinate.

**Decision.** Adopt independent option 2 with the restricted single-assignment proof of P2,
positive facts from P5, and option 4 for dispatch. The shared collector intersects final keys,
keeps path/channel evidence separate, and reduces selected rows. It does no language-wide
rediscovery of reads, writes, slots or callees. The graph, cache and admission consumers stay
on their existing representation. Use stable-local, independently exact or keyed inline evidence
under the following conditions.

1. **Stable local.** On demand, per parse and exact callable span, start at the definition on
   the incoming path. Require its exact declaring token, canonical access path and an initializer
   or formal whose incoming value is the tainted value on that path. A synthetic seed needs its
   own uniquely located source/definition witness. At a copy or alias, match the reached
   RHS/source value to this target, or refuse that origin. Re-prove any coalesced hop used to
   justify this transfer. An unlocated edge alone proves nothing.
   Build a closed inventory over the owner's grammar, including nested/deferred bodies and
   binding scopes. Resolve each candidate at its own bytes to the same declaration token.
   The definition must dominate evaluation of each read (including order within a statement),
   with exactly one initialization per binding instance and no possible second write or storage
   escape over its lifetime. Pair Go parallel declarations to individual RHS values.
   A use before initialization, or a loop-carried value without a fresh dominating initialization,
   fails. Independent locals in a tuple or on the same line keep their own proofs.
2. **Proof boundaries.** Each language supplies a finite positive grammar for introduction,
   scope and reads, and a total classification of effects on the storage. An unclassified node
   that might introduce a name, write, expose a local or execute hidden source invalidates the
   proof. Default new grammar productions to unknown. Walking only existing def/use inventories
   and subtracting known bad spellings is insufficient. Check exact declaration sites including
   parameters, destructuring, catch and loop binders, anonymous callables and shadow scopes.
   Classify compound/update targets as read-before-write, simple targets as write-only, and
   keyword names, member keys, labels, type positions and declaration names as non-reads.
   Visit RHSs, receiver/index *expressions* and interpolation expressions in their value roles.
   Account for captured and class-field/static writes even when the write's source precedes
   the read; reject unproved address/reference/alias escape, dynamic scope/eval, mapped
   arguments, nonlocal/global effects, macro expansion and unresolved identifier normalization.
   In the positive fragment, traverse grammar fields in evaluation order. Record each normalized
   occurrence of the target binding as declaration, value read, write, storage exposure or
   unknown. Backslashes in inert string or regex text do not affect binding. Inspect the token kind
   before normalization. Refuse a proof that depends on an unresolved identifier escape. Traverse
   interpolation expressions and destructuring, skip comments and account separately for effects
   without a spelled target. An ordinary call can preserve
   an isolated local when the grammar passes its value and the slot is inaccessible to the
   callee. Passing by reference, deferred code with access to the slot, and dynamic environments
   need their own proof. Audit the default-unknown branch against the pinned grammar node types.
   `const` protects the local slot only. It says nothing about a field or pointee.
   Enumerate *every* proved value read on the path's nominated read line in the owner, including
   non-bare, nested, third-statement and continued-operand reads. Each must lie in a reachable
   evaluation region for that origin and pass the path's access-range and cleansing checks.
   Do not cross a deferred execution boundary on expression containment alone.
   An unrelated shadow may be excluded independently only
   with exact token-level scope proof.
3. **Exact point and inline source.** For a non-stable value accept an independently proved exact use only.
   Require the correct tree, owner, half-open read and def spans, canonical full access path,
   declaration identity, and a value proof at that program point. For example a direct,
   dominating assignment/parameter and a closed straight-line prefix with no possible killing
   effect proves the *old* local read in `audit(x += suffix)`. The transfer must carry that read
   into the argument. The new stored value gets a separate definition. A matching endpoint,
   a line RD label or absence of an inventoried kill cannot discharge that proof. If no such
   proof exists, withhold even the representative. Never expand a field, pointee or request-object
   alias by a stable-base proof. Require an exact field witness, or a uniquely bound recognized
   framework-source access in the operand under the applicable route. For path-based flat roles
   on a repeated identical request-field access on the same line, withhold the group on that
   receiver path, in either representative order. A line-wide source-range list cannot assign
   an incoming value to successive property evaluations. An independently tainted operand may
   still select the role. Inline sources use their existing language route, binding and guards,
   with a witness anchored at the real insertion key in the same parse, call and eligible operand.
   An inline-only witness supplies structured roles. Step 6's legacy token scan needs no read proof.

**Reuse and cost.** P5 provides building blocks. `src/cpg/binding_facts.rs` has explicit-byte
declaration lookup; even `relation_at == Same` can compare equal flat fallbacks.
`build/lexical_prune.rs` is removal-only. RD uses a line CFG, has unavailable outcomes and
keeps both line and exact-key labels. A retained pair or an `Exact` label on representative bytes
cannot certify an omitted read. Use positive per-byte facts via a small read-only, parse-scoped
query where available, only with complete language proof. Do not expose crate-private tables or
persist proof results. Use the parser's canonical access-path builder for TS assertions.

Use the following existing facts at the proof boundary. Each adapter still has to connect the
exact declaring token and the originating value to evaluation of this particular operand.

| Files | Facts to reuse | Obligation left to the adapter |
|---|---|---|
| `taint_sinks/js_binding.rs`, `js_scope.rs` | Unique binding candidates, lexical scope, receiver/import and shadow classification | Match declaration tokens and check evaluation-order dominance, including use before initialization. |
| `src/ast/js_write_census/` and its `alias_cert.rs` | Grammar-indexed declaration and write sites, destructuring, alias candidates and dynamic/deferred hazards | Use the positive facts within their certified fragment. Prove each read's order and value transfer, including any alias hop. |
| `taint_sinks/py_scope.rs::Sites` | Binding sites by spelling, imports and `global`/`nonlocal` declarations | Resolve the individual token in its scope, then check initialization, deferred writes and read order. |
| `taint_sinks/go_scope.rs` | Nearest declaration, formals, statement and init scopes | Pair each RHS, distinguish `:=` reuse, and check dominance and exposure of the local slot. |
| `taint_sinks/callee.rs` | JS-family, Python and Go call positions, receiver and argument nodes | Prove wrappers and semantic slots at exact spans and replay the original index check. The seven legacy grammars need their own extractors. |

- P1 leaves the open class intact. The first fold blocked the five destructuring writes yet still
  accepted the class initializer and enclosing catch span. More examples do not close that predicate.
- P2 with unconditional “exact otherwise” accepts the clean shadow in round 1. Restrict its
  fallback as above. All repeated *local* operands in the nonempty E comparison have the
  single-definition shape, including C `ri23c` (automatic pointer passed by value, no address
  of its storage). Under the D2 macro contract, census the callee spelling for an in-repository
  macro before treating the C call as a value copy. Refuse an in-repo macro or
  incomplete census. A callee with no in-repo macro definition is modeled as a function for this
  purpose, including undeclared `audit_payload` in `ri23c`. A macro defined exclusively outside
  the repository is a limit of that model. A pointer to the local slot still escapes it.
  The sole repeated-field delta is in E. The comparison concerns expected ids;
  implementation still has to exhibit each certificate.
- P3 and P6 lose the measured roles above. Uniqueness within each separate statement would
  also reuse one line edge across statements without proving binding or value.
- P4 or a persisted side relation can preserve the old projections if admission still runs on
  the exact old graph. Splitting `VarLocation` equality alone changes traversal/deduplication and
  does not repair the original binding. Either core data-flow alternative requires a dedicated
  core PR, next-free CPG/cache version, cold rebuilds and ORCHESTRATION §5.6 on the lane corpora:
  exact base and head release binaries, two graph builds per corpus, compare the full
  `nav dfg-stats --edges` multiset and **every** `nav call-stats` counter. Predict changes before
  running and account for every delta, including zero drift in unrelated counters. Also run the
  ordinary finding/byte gates and §5.5 for affected paths. This is additional host time and
  cross-lane cache coordination; elapsed hours have not been measured. D-6 incurs none of that
  migration. A need for changed core flow facts returns to design before any such edit.
- P5 alone lacks the missing occurrence and effect proof. A detector-only solution lacks the
  reaching-value proof; a graph-only solution lacks the semantic operand and old guard.

#### Evidence interface certificate (S2-W, positive tables)

**Scope (controller decision, 2026-11-25).** The certificate serves the languages whose sinks are
decided structurally today: JavaScript, TypeScript, TSX, Python and Go. Java, C, C++, Rust, Lua, Bash
and HCL keep legacy substring matching and have no adapter in this lane. Their rows below are a
starting point, **not in use, unpriced for adapters**: a future adapter for one of them re-prices and
re-reviews its own tables before relying on them. The census numbers and the `no_operation`
explanation are in the PRICE paragraph at the end of this subsection.

**Input contract (owner decision, 2026-11-27, option A).** The certificate is correct for **valid
source in a default build**: the parsed file is a program its language's compiler or interpreter
accepts, built without flags that remove or rewrite code (`python -O`, `-DNDEBUG`-style
switches, `tsc` options that erase code). Where the language runs a construct in that setting,
the certificate certifies it. A Python `assert` condition or message is a real call in a default
build and is certified; `python -O` is outside the contract. A shape that is not a valid,
compiling program is a **known limit**: the certificate does not judge validity, accepts what
its tables list, and the limit is pinned as a `known_limit` row beside a control (an accepted
row of the valid neighbour). The known limits are: a Go blank identifier read as a value or
used as a selector segment (`eval(_)`, `eval = _`, `b._`; compile error), a Python call inside
an argument list whose order is a SyntaxError (keyword before positional, repeated keyword,
`*` after `**`), and a TS/TSX `const enum` initializer that is not a constant expression
(TS2474). Their incidence is counted by the census (`price.rs`, `pending_shape` column): zero
on the lane corpora. The certificate states the **syntactic role** of an occurrence (for example the callee of a call
expression in valid source, default build). It does not claim that the call executes, so
statically dead code is outside the contract: a constant-false guard, and a short circuit on a
literal nullish base (`(void 0)?.eval(x)`, `null?.m(eval(x))`, `null?.[eval(x)]`). Those three inputs
are pinned as `known_limit` rows (r10-limit-js-*) beside controls (r10-control-js-*). The narrowed
cell stays as an extra refusal and is named as such: an optional call or member chain directly on a
literal `null` or `undefined` base is withheld (`op_special_form`), although the contract would not
require it. **Census (price.rs, five-language run `price-5l-fold14`, 2026-11-28).** What the census selects and
excludes is defined by the functions in `crates/prism-review/src/algorithms/taint/sink_rules/price.rs`
(`possibly_nullish_base`, `nullish_chain_base`, `under_nullish_chain`, `literal_only`, `literal_tests`,
`under_literal_test`, `peel`) and pinned by the regression tests
`superset_counts_every_possibly_nullish_base_and_constant_guard` and
`known_exclusions_of_the_census_are_not_counted`; this document does not restate it. Inputs that
reviewers constructed and that the census does not count (or counts) are recorded as rows in
`known_exclusions_of_the_census_are_not_counted`, by reference and not by description. The
certificate's contract (the syntactic role of an occurrence, not that the call executes) does not
depend on these counts.

**Census result, limited to that filter** (from `price-5l-fold14.tsv`; certified of all flat hits: Go 22,575 of 22,591,
JavaScript 2,116 of 2,172, TypeScript 708 of 735, TSX 21 of 36, Python 2,899 of 3,039; shares
unchanged). Possibly nullish chains in the corpora: 5 (TypeScript source 4, excalidraw 1), all live
bases: two conditionals with an `undefined` arm, one assignment expression, two `||` expressions;
none statically nullish. **0 certified hits under a possibly-nullish base as the filter defines
it. 21 literal-test rows listed** (Go 3, Python 15, TypeScript 3, JavaScript 0, TSX 0; listing
`shapes-list-fold14.log`), **1 unreachable**: 20 sit in constant-true loops (`while (true)`,
`while True`, `for ..; true; ..`), which are live, and the one unreachable row is a Go test file
(cockroach `pkg/server/debug/goroutineui/dump_test.go:58`,
`if false { _ = os.WriteFile("test.html", ..) }`, a debugging toggle). What the contract means for
that row: the certificate states that the occurrence is the callee of a call, which is true there;
it never claimed that the call executes, so a reader who needs reachability must not read it from
the certificate. The row is recorded as a **counted instance outside the contract, not as zero**.
This is not a count of the whole dead-code class.
Go `len`/`cap` of an array or pointer-to-array operand without calls or receives (the compiler
folds the operand; `len(p.a)` is the pinned row) is the same class, pinned as an adapter-obligation
`known_limit` beside a slice control. They are builtins, not flat sinks, so their incidence among
certified hits is zero by construction (row `go_len_cap_unevaluated_operand`, always 0). A comment-only JSX
body that overwrites nothing is refused as `attribute_overwritten`: an accepted over-refusal
(conservative, no code change; rows `r2-*-jsx-children-empty-expr`). The census tool counts each
limit once per hit; known limits of the census tool itself are that its Go blank classification
reads the grammar roles it lists (parameters, `var`/`const` specs, fields, `left` lists, receives)
and that Python keyword names are compared after NFKC. Runtime mutation, `-O`, preprocessors and
code rewriting are out of model, as for `Exact`. The fixtures are minimal and omit unrelated
declarations; a control is valid apart from names and types its snippet leaves undeclared, except
where a row says otherwise.

For the next fold, certify the operation, target and operand against three closed tables for the
parsed language. The claimed role selects the applicable rows. An omitted shape earns no receipt.
Count the first applicable refusal per candidate in table order (a), (b), (c), with specific rows
before each table's default. Keep the vocabulary for the dormant tables as documented below.
Report receipt refusals separately from the joined counts in F. Keep the current-tree,
owner, reached-read and admission-route checks. Reject parse recovery on the certificate path.

The interface checks the operation's syntactic form and context, the target's exact node position,
and the operand's kind, position and role. An `adapter_obligation` is a known-limit row beside
the controls. The adapter proves the callee's binding, the operand's reaching value, and that the
resolved operation evaluates the certified slot. This includes distinguishing `pkg.Eval` or `Eval`
the function from a type conversion, resolving constructors versus aggregates, resolving methods
with an explicit receiver, and proving forwarding, macro expansion, result-tuple mapping or spread
iterator behavior if a future certificate admits those routes. Such proof cannot add a row to these
tables. Under the D2 macro contract a C/C++ call-shaped spelling needs the adapter's macro census
before ordinary call attribution. An identified macro with unknown expansion or an incomplete
census earns nothing. Macros outside that census remain a limit of the D2 model.

`Language::all()` has twelve grammars. Python, JavaScript (including JSX), TypeScript, TSX and Go
have dedicated call-position sink rules in `taint_sinks/`. Java, C, C++, Rust, Lua, Bash and HCL
(`Terraform`) have no dedicated call-position rules there and use legacy flat matching. All twelve
can supply legacy or configured admissions as applicable. These rows confer no rule activation.
The kinds and fields below are for the Cargo.lock grammars, including vendored Go and TS/TSX.

**(a) Operation and argument container.** The second column supplies call forms for `Argument`,
Python `KeywordArgument` and `ReceiverValue` where (c) binds a receiver. The third column
supplies assignment and attribute forms. Table (c) checks the claimed role for each form.
`>` denotes a named child step when there is no field. C-family and Java assignments require `=`;
Go `assignment_statement` requires `=`. Bash uses only repeated `argument` fields, never redirects
or leading environment assignments as slots. Lua requires the parenthesized form of `arguments`.

| Language | Call and argument container | Other operation kinds |
|---|---|---|
| Python | `call.arguments:argument_list` | `assignment` with `right` |
| JavaScript | `call_expression.arguments:arguments`, `new_expression.arguments:arguments` | `assignment_expression`, `jsx_attribute` at `jsx_opening_element.attribute` or `jsx_self_closing_element.attribute` |
| TypeScript | As JavaScript, with no `type_arguments` on either operation | `assignment_expression` |
| TSX | As TypeScript | As JavaScript |
| Go | `call_expression.arguments:argument_list`, with no `type_arguments` and head other than `make` or `new` | `assignment_statement`, `short_var_declaration` |
| Java | `method_invocation.arguments:argument_list`, `object_creation_expression.arguments:argument_list` | `assignment_expression` |
| C | `call_expression.arguments:argument_list` | `assignment_expression` |
| C++ | As C, plus `new_expression.arguments:argument_list` with no `placement` or `declarator` | As C |
| Rust | `call_expression.arguments:arguments` | `assignment_expression` |
| Lua | `function_call.arguments:arguments` | `assignment_statement` |
| Bash | `command.argument` | `variable_assignment` |
| HCL | `function_call > function_arguments` | `attribute` |

The argument row requires the stated field and delimiters. This withholds Python's bare
`generator_expression`, JS-family tagged `template_string`, C++ braced `initializer_list`,
and Lua's unparenthesized `string` or `table_constructor`. Withhold imports, including
JS-family `call_expression.function:import`, and direct `this`/`super` calls. Java
`enum_constant` and `explicit_constructor_invocation` have no row. Withhold Java `super`-qualified
`method_invocation` and anonymous or explicit type-argument `object_creation_expression`.
Withhold casts and `type_conversion_expression`. For call-shaped conversions and constructors
apply the head restrictions in (b). In Go, `make` and `new` are aliased `identifier` heads.
Refuse those spellings, including their type-only operands (`slice_type`, `array_type`,
`type_identifier`). A call-shaped type alias under a different spelling requires binding proof.

Require a runtime production and placement as well as the listed operation kind. Where a grammar
shares a kind with a non-runtime production, use its pinned production and enclosing field path.
Withhold unproved placements, including erased TS/TSX `type_query`, `type_alias_declaration.value`
and `type_annotation` positions such as the aliased `call_expression` in `import("m").T`, and
Python `type_alias_statement.right`. TS/TSX also withhold operations within ambient declarations
or declaration files (`.d.ts`, `.d.mts`, `.d.cts`), interfaces and object types (including computed
keys), abstract or bodyless signatures, and `declare` or `abstract` class fields. Ordinary class
heritage and runtime computed member names remain eligible. For Go, refuse calls with terminal
name `Sizeof`, `Alignof` or `Offsetof` and every operation within their argument lists. This is a
conservative name-based refusal that covers aliases, unqualified heads and unrelated same-name
functions or methods, because the certificate does not resolve imports or bindings.
Withhold Java `annotation` arguments, Rust `attribute_item` and `inner_attribute_item`, and
C/C++ `attribute_specifier`, `attribute_declaration`, `attribute` and `alignas_qualifier`.
For C/C++ exclude `preproc_if.condition`, `preproc_elif.condition`, `preproc_def.value` and
`preproc_function_def.value` (opaque `preproc_arg`), as well as unevaluated `sizeof_expression`,
`alignof_expression`, C `generic_expression`, and C++ `decltype`, `noexcept`, `requires_expression`
and `requires_clause`. Do not admit `typeof` type uses or the operator spellings `noexcept`
and `typeid` merely because the grammar presents an `identifier` call head. Apply the query
restriction to calls inside those operands too. `preproc_call` and Rust `macro_invocation`
have no operation row. A call in a branch body still needs proof that the branch is active.

**(b) Call target.** Classify the entire declared callee and each required descent before
testing node equality. A plain terminal or whole classified member/qualified path may be the
target. Wrappers only descend, and computed keys certify only the complete literal node. A
receiver, namespace qualifier, string fragment or type argument cannot supply the terminal name.
For Java method calls, the head is already the exact `name` field. Non-call target binding is in
(c). `parenthesized_expression` and `non_null_expression` wrappers require exactly one named
expression, excluding comment extras, and no other syntax beyond the enclosing delimiters or `!`.

| Language | Head field and admitted target paths |
|---|---|
| Python | `call.function` to `identifier`, `attribute.attribute:identifier`, `subscript.subscript:string`, or a `parenthesized_expression` wrapper |
| JavaScript | `call_expression.function`, `new_expression.constructor` to `identifier`, `member_expression.property:property_identifier` or `private_property_identifier`, `subscript_expression.index:string` or substitution-free `template_string`, or a `parenthesized_expression` wrapper |
| TypeScript, TSX | As JavaScript, plus a `non_null_expression` wrapper |
| Go | `call_expression.function` to `identifier`, `selector_expression.field:field_identifier`, or a `parenthesized_expression` wrapper |
| Java | `method_invocation.name:identifier` (with or without `object`). `object_creation_expression.type` to `type_identifier` or `scoped_type_identifier` with exactly two unfielded named children, a `type_identifier` or recursive `scoped_type_identifier` prefix and a terminal `type_identifier` |
| C | `call_expression.function` to `identifier`, `field_expression.field:field_identifier` with `operator` `.` or `->`, or a `parenthesized_expression` wrapper |
| C++ | As C, plus `qualified_identifier.name` to `identifier`, or to `field_identifier` from a `field_expression`, with recursive `qualified_identifier` names and plain `namespace_identifier` scopes. `new_expression.type` to `type_identifier` or the same qualified path ending in `type_identifier` |
| Rust | `call_expression.function` to `identifier`, `scoped_identifier.name:identifier` with absent or plain identifier/`crate`/`self`/`super`/recursive `scoped_identifier` path, `field_expression.field:field_identifier`, or a `parenthesized_expression` wrapper |
| Lua | `function_call.name` to `identifier`, `dot_index_expression.field:identifier`, `method_index_expression.method:identifier`, or a `parenthesized_expression` wrapper |
| Bash | `command.name:command_name > word` with bare identifier spelling |
| HCL | `function_call > identifier`, preceding `function_arguments` |

Python requires exactly one `subscript` field and no comma outside the key. Its `string` may be
unprefixed, raw or Unicode, with no bytes prefix or `interpolation`. JS-family `index` must be
one complete `string` or a `template_string` without `template_substitution`. C and C++ have no
literal-key descent. For C++ the pinned `field_expression.operator` lists `.`, `->` and `.*`.
Only `.` and `->` have rows. `->*` occurs as an `operator_name` token, with no `field_expression`
operator row. Refuse its call-position spelling even under recovery. `primitive_type`, Java
`integral_type`/`floating_point_type`, Java `generic_type`, C++ `template_type` and all other
unlisted constructor heads are withheld. Go `F[int](x)` and Java `new Foo<T>(x)` are pinned
losses. Index and generic-function nodes have no descent, and an entire inner call, `new_expression`
or `macro_invocation` cannot pass on equality. Rust `scoped_type_identifier` has no reachable
call head in the pinned schema. Java's row covers the reachable use of that kind.

**(c) Operand and position.** An ordinary positional operand is the direct named slot child
and must match one value row below. Comments do not consume slots. Withhold a comment as
operand or reached token at any depth. The receipt also withholds a reached node whose subtree
contains only comments or extras. Labels and attribute tokens do not count as values.
Compute ordinals from positional children. Before binding any role on a Python call, reject
bare positional slots after keyword arguments or `dictionary_splat`, duplicate keyword names
under NFKC, and `list_splat` after `dictionary_splat` with `slot_mismatch`. Reject the grammar's
parenthesized list-splat alias with `operand_spread`, including for earlier claims. A legal
`list_splat` after a keyword remains subject to the expansion rule. Withhold expansion nodes and
any positional claim at or after an expansion that can change its slot. For Python keyword claims,
bind the exact direct `keyword_argument.value` by raw name spelling and withhold claims in the
presence of `dictionary_splat`. The adapter supplies NFKC identity for rule selection. Go requires
a `variadic_argument`, if present, to be the final slot. HCL and Bash
use the explicit wrapper/field exceptions below. A parenthesized identifier is withheld as an
operand, whether the claim names the wrapper or the identifier inside it.

| Language | Admitted direct values and keyword position |
|---|---|
| Python | `identifier`, `attribute` via identifier-rooted `object` and `attribute` fields, `integer`, `float`, `true`, `false`, `none`, and `string` with the plain-string restrictions in (b). Also `keyword_argument.value` with one of these values |
| JavaScript, TypeScript, TSX | `identifier`, `member_expression` via identifier-rooted `object` and listed `property`, `number`, `true`, `false`, `null`, `string`, substitution-free `template_string` |
| Go | `identifier`, `selector_expression` via identifier-rooted `operand` and `field`, `int_literal`, `float_literal`, `imaginary_literal`, `rune_literal`, `true`, `false`, `nil`, `interpreted_string_literal`, `raw_string_literal` |
| Java | `identifier`, `field_access` via identifier-rooted `object` and `field:identifier`, `decimal_integer_literal`, `binary_integer_literal`, `octal_integer_literal`, `hex_integer_literal`, `decimal_floating_point_literal`, `hex_floating_point_literal`, `character_literal`, `string_literal`, `true`, `false`, `null_literal` |
| C | `identifier`, `field_expression` via identifier-rooted `argument` and `field:field_identifier` with `.` or `->`, `number_literal`, `char_literal`, `string_literal` |
| C++ | As C, plus `raw_string_literal`, `true`, `false` |
| Rust | `identifier`, `field_expression` via identifier-rooted `value` and `field:field_identifier`, `integer_literal`, `float_literal`, `char_literal`, `string_literal`, `raw_string_literal`, `boolean_literal` |
| Lua | `identifier`, `dot_index_expression` via identifier-rooted `table` and `field`, `number`, `string`, `true`, `false`, `nil` |
| Bash | One `command.argument` as `word` of safe ASCII letters, digits or `_./:+-`, `raw_string`, literal-only `string`, or `string` containing exactly one quoted `simple_expansion > variable_name` and no other content. The operand is the whole word |
| HCL | One `function_arguments > expression` whose named payload is `variable_expr > identifier` followed by zero or more `get_attr > identifier`, or `literal_value` with exactly one `numeric_lit`, `bool_lit`, `null_lit` or `string_lit`. The operand is the whole `expression` |

Withhold `list_splat` (including its parenthesized alias), `dictionary_splat`, `spread_element`,
`parameter_pack_expansion`, `variadic_argument`, bare `vararg_expression`, HCL `ellipsis` and
`splat`, and Bash words whose expansion or glob can alter the count. In Rust, withhold an
argument if an `attribute_item` occurs at or before its slot. In C/C++ withhold
`argument_list > compound_statement`. No operand is an initializer label, including
`initializer_pair.designator`, Go `keyed_element.key`, Rust `field_initializer.field` and Lua
`field.name`. Nested calls, binary and conditional expressions, callable literals, casts and
other unlisted values are withheld. Selecting a descendant under an unlisted value does not
promote it to a slot. Receiver and assignment operands use the same value rows.

| Language | `ReceiverValue` | `AssignmentValue` |
|---|---|---|
| Python | `attribute.object` | `assignment.right` paired with `left` |
| JavaScript, TypeScript, TSX | `member_expression.object` or `subscript_expression.object` on an ordinary `call_expression` | `assignment_expression.right` paired with `left` |
| Go | `selector_expression.operand` | `assignment_statement.right` or `short_var_declaration.right`, sole `expression_list` child paired with sole child of `left`, excluding the blank target `_` |
| Java | `method_invocation.object` | `assignment_expression.right` paired with `left` |
| C, C++ | `field_expression.argument` | `assignment_expression.right` paired with `left` |
| Rust | `field_expression.value` | `assignment_expression.right` paired with `left` |
| Lua | `dot_index_expression.table`, `method_index_expression.table` | `assignment_statement > expression_list.value` paired with `variable_list.name`, one of each |
| Bash | — | `variable_assignment.value` paired with `name:variable_name` |
| HCL | — | `attribute > expression` paired with its preceding `identifier` |

Python literal-key callees and JS-family `new_expression` forms still admit their explicit
argument slots. Bind the receiver to the listed field of the classified callee in (b), including
through admitted callee wrappers. This permits receiver/target overlap only for a receiver
inside the submitted whole member path. An explicit receiver in the argument list has only its ordinary argument
slot. Bind an assignment target to the single identifier or admitted member path on the LHS,
or its terminal name, and the value to that assignment's exact RHS. Reject patterns, paired
lists, parallel assignment and other unlisted LHS forms before binding. For Bash and HCL use
the exact name positions in the table. All other roles are withheld except `AttributeValue` below.

| Language | `AttributeValue` |
|---|---|
| JavaScript, TSX | In the listed JSX tag's `attribute` fields, bind the target to `jsx_attribute`'s unfielded `property_identifier`. The next named child is the `string` value or a `jsx_expression` with exactly one admitted value. For `dangerouslySetInnerHTML`, require `jsx_expression > object > pair.value` with one `pair`, `key:property_identifier` spelled `__html`, and an admitted value. |
| All others | — |

The attribute must be the final writer of its name within that tag. Table (a) refuses a spread
as the operation. Withhold a candidate followed by a same-name `jsx_attribute` or any
`jsx_expression > spread_element` attribute. Earlier spreads cannot overwrite a later explicit
attribute. Also withhold `children` on a paired JSX tag with non-whitespace text, a character
reference, an expression with a value or an element in the body. Empty bodies, indentation-only
text and self-closing tags supply no body child in this check. A comment-only expression container
(`{/* c */}`) overwrites nothing at run time but is withheld like any body child (`attribute_overwritten`):
an accepted over-refusal, pinned by rows `r2-*-jsx-children-empty-expr`.
JSX attribute names and `pair.key` are labels, never value operands.

| Table | Counted reasons |
|---|---|
| (a) | `op_kind_unlisted` (operation omitted), `op_context_non_runtime` (type, unevaluated, attribute or preprocessing position), `op_special_form` (call-shaped special form or unsupported modifier), `op_arguments_unlisted` (container or delimiters), `op_type_arguments` (explicit type argument list on TS/TSX or Go call-shaped operation), `attribute_spread` (spread in place of `jsx_attribute`) |
| (b) | `callee_kind_unlisted` (head, wrapper or path omitted), `callee_target_unlisted` (claim names a different part of a listed path), `callee_type_unlisted` (unlisted constructor or conversion head), `subscript_key_not_plain_literal` (invalid key shape), `subscript_c_family` (no C/C++ key row), `member_pointer_operator` (C++ `.*`) |
| (c) | `operand_kind_unlisted` (value omitted), `operand_not_direct_child` (claim under a slot outside its exact path), `operand_spread` (expansion or unproved word count), `operand_label`, `operand_comment`, `operand_attribute` (argument metadata), `operand_macro_block`, `role_not_bound` (missing role or field relation), `slot_mismatch` (argument layout, ordinal or raw keyword spelling), `parallel_assignment`, `attribute_overwritten` |

`callee_type_unlisted`, `subscript_c_family`, `member_pointer_operator`, `operand_attribute`
and `operand_macro_block` are dormant-language reasons. They remain in the vocabulary for
the dormant tables and are unreachable on the five in-use languages.

`callee_equality_before_class` has no reason row. The order of checks is an invariant and
the rejected shape reports its callee reason.

**Accepted losses.** Pin valid generic instantiations and constructors, type assertions,
parenthesized operands, nested-call and callable-literal operands, bare generator arguments,
tagged templates, literal spreads, argument attributes and dynamically selected callees.
Carry these controls into the lane's step 2 census and F's joined comparison. Charge the
loss only after joining comparable admissions and reducing the surviving identities and strengths.
PRICE (2026-11-25, census over today's production flat sink hits on the lane corpora, release build, `price.rs`; a hit is certified when its enclosing operation, callee and at least one operand pass the three tables): Go 99.9% (22,575 of 22,591), JavaScript 97.4% (2,117 of 2,173), TypeScript 96.3% (708 of 735), Python 95.5% (3,027 of 3,168), Lua 94.7% (36 of 38), C++ 65.9% (890 of 1,350), TSX 58.3% (21 of 36), Bash 45.8% (1,422 of 3,104), C 43.9% (15,857 of 36,147), Rust 19.1% (417 of 2,181), Java 16.2% (278 of 1,714), HCL 1 of 9 (run 2026-11-25 over cockroach, kubernetes, vitess, django, mypy, TypeScript, excalidraw, express, redis, cpython-c, leveldb, secbench-node-r1, the prism source and the pinned inputs). Caveats: the share is an upper bound, because a hit counts if ANY operand of its operation certifies, which an adapter's single claim need not; the Java and Rust populations are thin (Java 1,714 hits, almost all from the prism and pinned trees; no Java corpus in the set) and HCL has 9 hits; TSX has 36 hits. TSX classification (all 15 misses read): 14 are a nested call as the operand value (13 of them the one idiom `dangerouslySetInnerHTML={trustIcon(icon)}` of a single repository, a JSX attribute whose value is a call that returns the html object, withheld like any nested call, row `tsx-L-census-jsx-call-value`; the 14th is `window.location.href = url.toString()`, assignment-style location rule, row `tsx-L-census-location-call-value`) and 1 is an `innerHTML` assignment whose target is a call chain (`role_not_bound`, owned by the assignment-style DOM rule); no JSX spread and no listed-shape miss, so no operand cell is added and the TSX share stays 21 of 36 (58.3%), 21 of 23 (91%) without that one idiom. In C, C++, Java, Rust and Bash most refusals are legacy substring hits that are not in call position (`no_operation`: C 18,018, Java 1,374, Rust 1,212, Bash 803, C++ 342), which no adapter could earn; counting only hits that have an operation, C 88.6%, C++ 89.0%, Java 84.2%, Bash 61.9%, Rust 43.0% (Rust: 188 `callee_target_unlisted` on scoped paths, 348 `operand_kind_unlisted`).

The CLOSED refusal controls rely on `op_kind_unlisted` (4, Java explicit constructor forms in 32),
`op_context_non_runtime` (28, N1, P3-04/05/07), `op_special_form` (P3-08/09/15),
`op_arguments_unlisted` (24/25), `callee_kind_unlisted` (1, 9–11, 13/15/18/19/22/26/27/33/37/38),
`callee_type_unlisted` (5/8/31, constructor form in 33), `subscript_key_not_plain_literal`
(2/13/14, N5), `subscript_c_family` (7/13), `callee_target_unlisted` (partial plain key in 14),
`member_pointer_operator` (3), `operand_spread` (N2/N3, P3-12/18), `role_not_bound` (N4),
`operand_not_direct_child` (N6a), `operand_attribute` (P3-01/02), `operand_macro_block` (P3-06),
`operand_label` (P3-10), `operand_comment` (P3-11), `parallel_assignment` (P3-13),
`attribute_overwritten` (P3-14) and `operand_kind_unlisted` (P3-19). Rows 40/41 and P3-20 get
positive positions. P3-03 retains the value position with keyword normalization at the adapter.

### 5.3 The registry

`prism::rule_registry` owns `RuleInfo`, `RuleRef`, the named constants, the sorted `rules()`
table, `lookup(id)` and `check_emitted(algorithm, rule)`. Each row declaration supplies both its
named constant and its table entry. `RuleInfo::rule_ref(&'static self)` copies the row's id and
optional CWE and sets severity to `None`. Built-in projections select a named `&'static RuleInfo`.
S2-W adds `prism::slice::attach_rules(&mut SliceFinding, rows)` as the sole writer of built-in
rule lists. Each projection passes all selected rows for one finding, pairing each row with whether
its occurrence is weak. The helper groups by id, takes the strongest occurrence, sorts by id and
replaces the vector once. It calls `row.rule_ref().with_severity(override_value)` per distinct row
using 5.2. Primitive and absence pass one row at the finding's own severity.

- **Compiler.** Both structs' fields are private to the core registry module and its descendants.
  Outside that tree, Rust rejects literals, struct updates and direct field assignments, including
  those inside expanded macros. Typed projection interfaces require a row. A string or deserialized
  ref cannot satisfy that signature. Public accessors borrow metadata. The maker takes a row and
  `with_severity(self, Option<String>) -> Self` takes only an override. The API supplies neither a raw-parts factory nor a
  deserializer for `RuleInfo`. Privacy grants access to all code inside the tree, including nested items.
- **Tests.** For every row, check that the maker copies its id/CWE and that applying or clearing a
  severity override preserves both. Separate compile-fail doctests guard against adding
  `Default` or `FromStr` for `RuleRef`. `check_emitted` rejects an
  unknown id, a missing, extra or different CWE, and an unlisted algorithm. Each producer slice
  applies it to every actual built-in ref on the matrix and pinned outputs, with independent
  expected identities for the exercised examples. This establishes validity on the paths run.
- **Review.** Trace each built-in projection through `attach_rules` back to its named rows. Check
  for writes or forwarded findings that bypass the helper. A computed
  string can still select a *valid* row through `lookup`. Public serde can materialize an arbitrary
  `RuleRef` or whole finding, and callers can clone or forward it into the public vector. Neither
  type checking nor output equality proves how that value was obtained. Review rules out these
  routes for built-in producers and inspects new factories, conversions and code inside the trusted
  module. External input is admitted through serde and normalized at the SARIF boundary (5.5).

S2-R moves the existing 64 A/B rows intact before the first producer. The source lives in
`src/rule_registry` (root `src/rule_registry.rs`). The review crate re-exports the module as
`prism_review::output::rule_registry`. Lane 8 I-9 (#881) imports `prism::rule_registry` directly
in `prism-taint` and removes the former review registry file from its move manifest and stop rule.
I-9 carries the classifier's `SinkHit` and uses its read accessors. The existing `prism::slice::RuleRef`
path and serde contract survive, including unknown ids, field order and omitted optionals.
Downstream struct literals, patterns accessing fields, and
direct field reads/writes are a source break. Readers use `id()`, `cwe()` and `severity()`.
Callers use named rows for built-ins and deserialize supplied wire data for external references.
Accept the field API change here to close direct identity writes before the first publication.
S3 adds the generated page, whole-table checks and the fixed C/D rows.

Use the exact id as the stable SARIF `name` (including uppercase primitive ids). Write `short` as one
sentence naming the observed operation and operand. Write `help` as one or two sentences explaining the
match and a specific inspection or mitigation. Use conditional language for nominal/weak matches and
name the missing evidence for an unresolved rule. Keep ids, CWEs and severity out of prose where they
would duplicate metadata. Generate the page and specific-mode descriptors from these same strings.
For `legacy` use `name = "legacy"`, `short = "A legacy sink name matched on a tainted line."` and
`help = "Inspect the matched name and trace the input to the operation. This detector does not identify a particular operand or weakness."`
Use `algorithms = ["taint"]` and `cwe = None`. A generated display name would change when copy changes.
Generic mitigation boilerplate would imply identical evidence for a format argument and a name match.

#### S2-R enforcement options (design escalation)

**Independent options, before pooling**

- Give producer helpers a closed input type in `prism-review`. They select an opaque registered
  row and pass it with the reduced severity to one writer. Rust can reject a string, a wire
  `RuleRef`, or a hand-built row at those interfaces. This preserves the core wire API and costs
  little before the first producer. Any producer with a mutable `SliceFinding` can still write
  its public vector directly. Review must account for each such writer and tests must inspect
  the references that actually leave it.
- Move the registry and maker into a small core module with private identity fields and public
  read accessors. Rust then checks direct literals and assignments even through aliases and
  macro expansion. Core can own the rows without a reverse dependency. Count external field
  uses first. This changes the Rust field API, moves review metadata into core and needs an
  explicit path for normalizing an imported unknown id. Public deserialization still allows a
  producer to synthesize an arbitrary wire value.
- Separate built-in and external findings up to the final conversion to the shared wire format.
  Only a registered row can enter the built-in rule list. This would make provenance a type
  property at every output that requires the built-in finding type. It also changes the finding
  types crossing algorithm, aggregation and output interfaces. Measure those interfaces before
  accepting the migration cost for an empty-producer slice.
- Validate every built-in rule at a common publication boundary. This can reject invalid values
  on untested paths and preserves external deserialization. It costs a lookup per reference,
  needs a defined failure policy and proves membership and algorithm eligibility only. Two
  equal refs have the same bytes regardless of how their ids were chosen. Establish that all
  built-in outputs pass the boundary before claiming coverage.
- Retain a source lint for a small set of recognizable spellings. It can point at an accidental
  literal in review. Macro expansion, type resolution and conditional compilation determine
  which Rust expression is actually built. Extending the current walker buys examples at an
  ongoing maintenance cost and leaves the exclusivity claim unproved.

**Pooled decision.** Adopt O4 and O6, plus O2's typed producer interfaces, output tests and
writer review. Remove the entire scanner. Apply Rust privacy to the independent `SinkHit`
literal restriction in its own classifier module (PLAN S2-R).

| Alternative | Disposition and cost |
|---|---|
| O1, harden the scanner | Reject. Following token trees, aliases, `cfg_attr` and mounted modules still leaves expansion and resolution to approximate. The implemented scanner and its controls already consume 321.5 weighted lines at `e53d01c8`. |
| O2, retain a small tripwire | Keep the typed interfaces, tests and review. Drop the tripwire even in combination with O4. Privacy checks direct construction for every spelling the compiler accepts. A syntax tripwire would still miss serde and valid-row lookup bypasses. |
| O3, non-exhaustive ref and cross-crate maker | Reject. `#[non_exhaustive]` blocks downstream literals but leaves public field mutation. Rust has no friend-crate visibility. A token sealed in core needs a public path to mint it in a dependent crate, and a hidden raw constructor is still callable. Co-locating the mint and rows yields O4. |
| O4, move rows and maker to core | Adopt. At the rejected product head `e53d01c8` the maker is the only in-tree production ref literal. Core takes ownership of the row strings. Embedders bear the field API change described above. |
| Keep id, CWE and algorithms in core and join review-side short/help by id | Reject. A two-way inventory test could detect missing or orphaned text, and core would carry fewer strings. Editing a rule would then require coordinating two tables and testing the join. One complete row supplies both the generated page and the SARIF descriptor. |
| O5, compiler-resolved lint | Reject. It can see resolved expressions but would add a toolchain and CI gate for restrictions privacy already checks. Proving serde provenance or ruling out forwarding would require additional analysis. |
| O6, private row fields | Adopt for *all* row fields. A seal alone with writable public fields would still permit changes to a local copy of a named constant. O6 alone leaves the S1 ref fields writable. |
| Separate built-in and external finding types | Defer. Enforcing that boundary throughout the public finding vector, aggregators and serializers would widen S2-R beyond the registry. |
| Validate at every built-in publication | Reject as a lane-wide runtime gate. It would require a common boundary and an error policy across formats, pay per ref, and still accept the wrong choice of a valid row. Keep `check_emitted` in the fixture gates and on canonical registered SARIF input in S4. |

### 5.4 Other algorithms

Select one immutable row per admitted finding. C requires its operation-specific witness, including
effective protection and credential-use checks where specified. D requires a manual obligation
proved outstanding at the exit on the reported path, or its two-site test for `double_close`.
Each supplies a named CWE-less fallback. A literal id survives edits to the description.
These refs inherit the finding's severity. Other algorithms emit an empty list.

### 5.5 SARIF

- After admission, normalize `SarifInputs` **once per finding, before either mode builds result
  properties or descriptors**. For a registered id on a listed algorithm take the registry's optional
  CWE, even if the supplied CWE is different, missing, or supplied for a neutral row. Run
  `check_emitted` on that canonical ref. Discard a registered id on an unlisted algorithm entirely.
  For an unknown external id copy the supplied id verbatim. Both surviving cases fill one private,
  serialize-only SARIF property type with the wire keys of 5.1. Give the unknown id the sole
  distinct well-formed CWE supplied for that finding, or none if absent or conflicting. Its specific descriptor uses the sorted union of
  all well-formed supplied CWEs for the same (algorithm, id) across admitted findings. This
  reduction operates on SARIF output fields and requires no core CWE setter or new `RuleRef`.
- Group by id within each finding and sort. A missing, unrecognized or higher override inherits the
  finding's severity. For a recognized finding take the strongest effective member in the order
  `concern > warning > suggestion > info`; serialize an override only if it is lower than the
  finding. An unknown finding severity inherits its existing SARIF level. Thus on a warning,
  `suggestion + warning` (or `suggestion + absent`) gives warning in either input order. Raw
  passthrough can contradict the descriptor. Dropping a valid id with a bad CWE loses its known
  identity. First-wins makes an external caller's ordering decide the alert level.
- Category mode keeps the category descriptor, level and fingerprint and includes the canonical
  `properties.rules` only when nonempty (decision 7). Specific mode emits one result per surviving
  id with its effective level, the **same full canonical list** on every copy, and
  `ruleId = prism/{encoded algorithm}/{rule id}`. An initially empty or all-discarded list yields one result
  at the finding's level, with no rules key. Category mode uses its ordinary id. In specific mode
  use `prism-fallback/{encoded algorithm}/{category}` (the usual category default if absent) with category
  descriptor text and no tags. In both specific forms encode the raw algorithm in one pass, writing
  `%25` for each `%`, `%2F` for each `/`, and copying every other character verbatim. The first slash
  after that segment unambiguously starts the verbatim rule id or category, which may contain slashes.
  Thus `(taint, vendor/check)` and `(taint/vendor, check)` have distinct descriptors, as does
  `(taint%2Fvendor, check)`. Use the original algorithm in properties and in the CWE union key.
  Category mode retains its existing id format. The distinct first segment separates the fallback
  from every registered or external specific id, including all six primitives whose verified id equals their category.
  Sharing one descriptor would either lend its CWE to the empty finding or erase the verified CWE.
  Compute evidence, locations and fingerprint once and assign `ruleIndex` after expansion and sorting.
  A registered descriptor uses the row's name, short, help and optional `external/cwe/cwe-<n>` tag.
  Unknown ids get their id as name and generic descriptions. A consumer distinguishes expanded
  results by fingerprint, rule id and primary location.
- The flag's help text and the README say that switching modes can start new code-scanning alert histories
  for the affected findings, and that GitHub's display of the CWE tags is unverified (section 2).

### 5.6 Review and JSON

`--format json` and `--format review` both serialise the finding, so the `rules` key appears in both, in
every copy of a finding (per result and in the aggregate list, single and multi run), and is omitted when
empty (decision 7). The text, paper, Mermaid and callers outputs print at most a finding's description
(Mermaid's diagram headings), so they show no rules and need no renderer change. One byte-golden holds a
taint finding (`tests/fixtures/review_no_diagrams/golden/review_unflagged.json`, two copies of one finding);
S2-PY re-records it once, with the reason in the commit.

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
| G-closed | The privacy compile-fail controls, serde round trips and row inventory in 5.3 pass. Every built-in ref on the matrix and pinned outputs passes `check_emitted`. Review traces each producer write. Every specific result indexes its own descriptor. Registered rules use the registry's optional tag, external ids follow 5.5, and an empty result indexes the distinct tagless fallback in either input order. |
| G-coverage | Check all 540 table/label keys both ways and presence at every marker. Activate identity assertions per PLAN partition (205/360/474/540 cumulative), including positive empty sets. On the final head every positive asserts its D identity and relative severity. |
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
| 7 | Default JSON, review and SARIF output | (**confirmed by the owner, 2026-11-21**; the optional per-rule `severity` key rides on the same answer unless the owner says otherwise) a finding with rules gains the additive `rules` key (SARIF: `results[].properties.rules`) by default, as other additive fields have shipped; the review golden is re-recorded once; additivity is "equal to main once the `rules` keys and per-build keys are removed" | the alternative is the key only behind an explicit option (every existing byte kept; default users get no rule identity). Moving to it after a release removes a key readers may rely on. S2-JS is the first producer and S2-PY owns the review golden. | before S2-JS merges |

Two collector decisions remain with the owner before product dispatch. For the single-mention
multiple-definition threshold, the proposed default is one unrescued admitted key on the 35 pins
(Appendix F). For review after a design decision, the proposed default is to carry #901's two
spent rounds into its successor proofs (PLAN Stage I). A fresh three-round cap per proof PR
requires an explicit owner decision.

## 8. Risks and accepted limits

- **Twenty-seven struct literals change in one commit.** It is mechanical and reviewed as such, with no
  behaviour change in that commit.
- **`legacy` hits have no CWE.** Languages without positional sink rules get a rule id that says so.
- **Rule ids become a public surface.** Renaming one later is a breaking change for anyone filtering on it;
  the registry test and the generated page make renames visible in review.
- **Other output consumers.** Nothing in `eval/` parses finding JSON by key list, and the sibling
  fault-injection product reads targets only, which do not change. A reader that rejects unknown keys sees
  the new key (decision 7).
- **Coalesced graph occurrences.** Section 5.2 requires a reaching-value and read-role proof even
  for the first span. Appendix F measures withheld occurrences and findings whose final list changes.
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

**Locate the operation before naming it.** Each grammar adapter returns the operation span,
exact callee leaf/operator, value-expression span and semantic role. Extract the leaf through
the classifier's proved wrappers. Cover JS constructors, static member/optional/generic calls,
parentheses and the final callable in a sequence such as `(0, eval)`, C++ qualification and
construction, Java creation types, Rust scoped generics, HCL function arguments and a static Bash
command word with repeated argument fields. Refuse computed `table[audit]`, a callee expression
without a proved terminal name, and dynamic command words. Only the verified leaf feeds configured
substring/`=name` and wrapper exact matching. A statically named callback such as `archive(x)`
in `astra-r1-32` can supply `configured-sink` with an unknown implementation. Test static
`(audit)(x)` and `ns::audit(x)` against the computed-index refusal.
Assignment and JSX adapters return the actual RHS or `__html` value and its matched target token.
A value read nested in a receiver/argument gets only the A roles that consume it.

**Closed nominal contract (flat and structured).** Resolve the callee by import, type, constructor,
unique alias or dominating assignment. A shadow, conflicting bindings or a known different operation
veto a spelling-based match. The conventions below use the named tables at `65a13c348b47`, their
specified operand slots and conditional registry help. Admission alone never extends this list.

| Family | Admitted nominal evidence for projection |
|---|---|
| JS SQL | Receiver leaf in `js.rs::DB_HANDLES` with a `SQL_NAMES` statement method, or `exec` on a DB handle. This admits untyped `db.query`, `pool.execute`, `knex.raw` and `prisma.$queryRawUnsafe`. Also `request.query` with constructed SQL text and the specific `createQueryBuilder(alias).where(clause)` chain. A bare import of `query` from a local module needs a resolved DB implementation. The path `./db` alone gives `taint-query-name`. |
| JS HTTP/storage | Use `js.rs::RESPONSES` as receiver root or exact member (`this.res` included), with direct calls or the `status/type/code` chain and the A/B body, header, redirect and file-serving methods. Default `reply.code(n).send(x)` is the data role. A proven explicit HTML type can change it. Also accept `ctx.redirect` and `ctx.response.redirect` on the Koa shape, and the bound `next/navigation.redirect` export. The storage convention is `disk.write(path, stream)`. Filesystem names use `js.rs::FS_LIKE` with `js_modules.rs::FS_API/TWO_PATH_API` at their path slots, or a bound `FS_MODULES` import. |
| Python SQL | DB-API `cur/cursor.execute/executemany/executescript`, `conn.exec_driver_sql`, `.objects.raw/filter` and a chain proven by the existing `sqla` grammar. On an unresolved receiver, only `query` may use a SQL-bearing literal in its actual statement operand (including f-string, concatenation, percent or format with a literal leading SELECT, INSERT, UPDATE, DELETE or WITH token, case-insensitive). A known different operation vetoes the exception. Formatting unknown variables or a lowercase identifier alone supplies no SQL witness. `python.rs::SQL_METHODS/PANDAS_SQL` give method vocabulary and `query_sql_arg` gives query slots. Neither identifies a receiver. SQLModel `Session.exec` requires the Session binding or an independently proven SQL statement. |
| Go SQL | Intersect `go.rs::SQL_NAMES` with the actual methods and slots of the resolved receiver. The import-free type spellings are sql.DB/Tx/Conn, sqlx.DB/Tx/Conn, pgx.Conn/Tx, pgxpool.Pool and gorm.DB, including pointers. `*sql.Conn` offers `ExecContext` only for execution. The field conventions are `.db.Exec(SQL, binds…)` with a SQL literal, `.db.WithContext(ctx).Exec(sql)` and `.pg.Exec(ctx, sql)` with a context argument. Method values retain the receiver. `Handle` supplies no DB type even with `QueryContext` or generic `Queryx[T]`. |
| Other named forms | The built-ins, DOM targets, module-qualified APIs, literal structured call paths and signature-specific forms explicitly listed in A/B are the remaining conventions, with their stated binding/semantic restrictions. Permit the unshadowed bare Flask `render_template_string(source, **context)`, `mark_safe` and `Markup` forms, and the qualified Go package spellings for A/B's file, format, exec, template and net/http methods. A generic method suffix in B still requires its stated operation. Python redirects require Flask/Werkzeug/Django binding (`flask.redirect`, `werkzeug.utils.redirect`, `django.shortcuts.redirect`, `django.http.HttpResponseRedirect`, including resolved imports). No language grants a redirect CWE to an unbound or injected callback. |

**SQL role order.** First bind the reached operand to an eligible slot. For a proved ORM comparison,
statement containing a bound value, or a nominal/known DB bind slot, return `taint-query-value`
for that value alone (also for an opaque ORM entity on a proved ORM query receiver). Otherwise an
established SQL execution/text slot gives its A/B SQL id. An unresolved query-like API with a
reached prospective statement/clause gives `taint-query-name`. Remaining positions give ∅.
Thus `session.query(x)` on an untyped parameter takes the third branch. A spelling-only exception
for `session` would confuse arbitrary clients with a Session. Treating any redirect callback as a
redirector would similarly misclassify `sol-r2-50`. Dropping nominal evidence wholesale would lose
the listed API conventions. Use source bindings and operands for dispatch.

| Classifier today | Operation and reached operand | Published identity | Disposition |
|---|---|---|---|
| js-eval/95 | Global/unshadowed or indirect JS eval, source text | js-eval/95 | same |
| js-function-ctor/95 | Function/new Function, dynamically parsed body or formal-parameter text | js-function-ctor/95 | same |
| js-child-process/78 | Bound child_process, execa, shelljs or cross-spawn launcher (`require("cross-spawn")` or its resolved default export), selected program or shell command. With shell enabled, shell-interpreted arguments qualify | js-child-process/78 | same |
| js-child-process/78 | Node fork selects the child **module filename** | js-child-module/73 | corrected |
| js-exec-call/78 | Proven remote command API, including node-ssh exec/execCommand, command text | js-exec-call/78 | same |
| js-child-process/78 or js-exec-call/78 | Bare unbound exec/spawn/fork, unknown/injected/shadowed receiver, or mixed regex/command binding | taint-exec-name | filed F1 |
| js-exec-call/78 | A regex variable is definitely reassigned to child_process before this call | js-child-process/78 | corrected |
| js-eval/95, js-function-ctor/95 or py-eval/95 | An eval-like spelling, no evidence the receiver evaluates code | taint-eval-name | filed F1 |
| js-db-call/89 | SQL text/fragment, including SQLite exec, raw constructors, query/execute and the reached SQL where-clause of a builder chain | js-db-call/89 | same |
| js-redirect/601 | Redirect destination, including a shifted URL after a status | js-redirect/601 | same |
| js-redirect/601, py-redirect/601 or go-redirect/601 | Reached prospective destination of an unbound or injected redirect callable | taint-redirect-name | filed F1 |
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
| py-db-call/89 | Established statement-text role under the closed contract above, including bound pandas read_sql family. Respect SQL/query keyword names and positional SQL slot | py-db-call/89 | same |
| py-eval/95 | SQLModel Session.exec with a SQL or raw SQL-bearing statement. Bind Session by its import/type/constructor or demonstrated statement API (#646) | py-db-call/89 | corrected |
| py-eval/95 | Built-in eval/exec or builtins alias, evaluated source | py-eval/95 | same |
| py-sql-clause/89 | Actual SQL syntax supplied to a clause or SQLAlchemy text constructor | py-sql-clause/89 | same |
| js-db-call/89, py-db-call/89, py-sql-clause/89, py-eval/95 (SQLModel), go-db-call/89 | Proven bind slot, proven ORM comparison or opaque entity operand on an established ORM receiver. A dynamic statement independent of a reached bind value gives the value role only | taint-query-value | filed F3 |
| js-db-call/89, py-db-call/89, py-sql-clause/89 or go-db-call/89 | Remaining ambiguous query/filter API with a reached prospective SQL/clause operand, including untyped session.query, q.filter/q.where, DataFrame filter or Cache.Query | taint-query-name | filed F1 |
| py-os-command/78 | os.system/popen or exec/spawn family, asyncio subprocess or pty: command text or program selector in the API's slot (spawn mode, context and argv have separate roles) | py-os-command/78 | same |
| py-subprocess/78 | subprocess run/call/Popen/check_*: shell text or selected executable (including the program element of argv) | py-subprocess/78 | same |
| py-module-command/78 | Proven paramiko, pexpect, fabric or invoke callable: command/program operand | py-module-command/78 | same |
| JS/Python/Go process tags /78 | Data passed only in fixed-program argv, options/environment, or after fixed shell command text, without evidence it is interpreted as code | taint-process-argument | filed F3 |
| py-template/1336 | Template constructor or Flask render_template_string: template source, including a reached source keyword | py-template/1336 | same |
| py-template/1336 | Flask literal template renders the reached context keyword unescaped under B's scoped-use test | py-html/79 | corrected |
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
| JS Sqli (89) | query, execute, raw, literal, $queryRawUnsafe, $executeRawUnsafe [0], statement/fragment slot of an API established under A. Keep the parameterization and Prisma checks | structured-sqli/89 | same |
| JS Sqli (89) | Mongoose $where[0] evaluates a JavaScript predicate supplied as code | structured-code-eval/95 | corrected |
| JS Ssrf (918) | fetch, get, post, axios, got [0], request [0,1]. Require the matcher-recognized client/global and a URL or destination option (host/hostname/port/protocol/path as it participates in a destination). These APIs can issue server-side requests. Body, method or unrelated header alone gives ∅ | structured-ssrf/918 | same |
| JS Ssrf (918) | A matched destination in an explicitly browser-only request. Require evidence of the browser-only binding or execution context. A TSX extension alone cannot establish it | structured-client-request | filed F2 |
| JS Deserialization (502) | unserialize, deserialize, yaml.load and bound js-yaml load [0], serialized document under accepted schema | structured-deserialization/502 | same |
| JS Deserialization (502) | eval, Function, vm.runInNewContext, vm.runInThisContext, vm.runInContext, vm.Script [0], JS source or executable formal text. A new-expression contributes only if the matcher actually returns it | structured-code-eval/95 | corrected |
| JS OsCommand (78) | exec, execSync [0], proven process/SSH command. spawn, spawnSync [0,1] with the existing shell:true check, program or shell-interpreted arguments, including the bound cross-spawn launcher in A when the spawn pattern matches | structured-os-command/78 | same |
| JS OsCommand (78) | exec[0] on SQLite/database handle executing SQL, including ri24 | structured-sqli/89 | corrected |
| JS PathTraversal (22) | readFile, readFileSync, writeFile, writeFileSync, createReadStream, createWriteStream, unlink, rm, sendFile, download [0], rename [0,1]. Only file paths | structured-path-traversal/22 | same |
| Python Xss (79) | mark_safe, Markup, markupsafe.Markup [0], trusted HTML. format_html[0], reached HTML format text, excluding escaped substitutions | structured-xss/79 | same |
| Python Xss (79) | render_template_string, reached value bound to the source formal by the signature below, overriding table [1] and the special matcher's syntactic index 0 | structured-template/1336 | corrected |
| Python Xss (79) | Same pattern, reached context keyword with a proved unescaped output of that key in the bound literal template. Otherwise ∅ for context | structured-xss/79 | same |
| Python Sqli (89) | execute, executemany, raw [0], reached statement slot established under A (nominal receivers only; an unresolved `execute`/`executemany` receiver gets no SQL-literal witness) | structured-sqli/89 | same |
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

**Two slot checks for ordinary indexed entries.** For each call × entry × channel, replay the
existing index test on that call/path (or inline source) with its original argument accessor.
Retain the particular old indexed argument spans that pass. Independently bind the semantic
formal to the actual value-expression span. Require the *same proved read* in both the permitted
old argument and its corresponding permitted semantic role. A passing sibling, another index on
the same call, or a corrected index alone cannot authorize that child. Exclude trivia from semantic
positions. Python `...` is a value; keyword labels are not values. Bind explicit keywords by
formal name independently of source order. JS tagged templates reserve slot 0 for template strings
and start substitutions at 1. For unknown-width spreads/splats refuse dependent positions or
ambiguous keyword binding. Independent earlier positions or a proven argument to a name-only
detector may still qualify. Cover comments both ways on `insertAdjacentHTML`, including an
independently admitting sibling. Its old callee exclusion still applies to the indexed flat role.
Apply the documented signature-specific Flask, JSX and A roles with their own guards.

These rows cover 40 JS-family entries, 48 Python entries, 17 ordinary Go entries and two Go framework
entries. Materially different slots in one occurrence can produce different rows. Framework-source and
semantic special cases in 5.2 decide if there is an occurrence at all. A rejected match contributes ∅.

**Flask edge and inline controls.** Bind the sole positional source or explicit `source=` keyword
in any order. Conflicting/missing source, extra positionals or an unresolved splat that could change
the binding supply no role dependent on it. For a context value require the actual keyword's reached
bytes (or inline request access there) and a use of that same binding in a literal output expression.
The bounded template proof accepts direct `{{ k | safe }}` or direct `{{ k }}` in an active literal
`autoescape false` region. Track nested true/false blocks and restoration on `endautoescape`.
Ignore comments and raw text. Uncertain expressions, filters, scopes or local rebinding give no
context proof. The admission helpers test index 0 and the presence of a block anywhere. Reusing them
would tag unused context. Withholding all context would also remove proven unescaped uses.

| Call shape on an admitted line, x reached | Eligible role before the flat-source filter |
|---|---|
| `render_template_string(name=x, source="fixed")` or its keyword permutation | ∅ |
| `render_template_string(name="fixed", source=x)` or its keyword permutation | template source (flat and structured on an eligible edge) |
| Literal source with an unused disabled block, or `{{ name }}` only after its end/inside nested true, `name=x` | ∅ |
| Literal source with `{{ name }}` inside active false, or a direct safe-filtered output of `name`, `name=x` | HTML context (flat and structured when eligible) |

For a literal `"{{ name | safe }}"` and reached `name=x`, place the literal first positionally,
first as `source=`, then last as `source=`. All three eligible edges project
`py-html@CWE-79;structured-xss@CWE-79` for that use. Recompute the call's own context exclusion
from this binding in each form. Retaining the old whole-call range suppresses `py-html` when
`name=x` precedes `source=`. On inline-only evidence expect just `structured-xss@CWE-79` on
admission. Use a real admitting sibling when needed and retain the framework-source flat exclusion.

## Appendix C. Primitive registry rows

Today all six categories have empty rules. Revising all six D-1 mappings affects zero taint TSV
cells and at most six Stage M primitive findings. The pinned proof split is unknown. Select the
category id and its fixed CWE on the specified witness, else its literal `<CATEGORY>_MATCH` row
without a CWE (F8). These twelve constants preserve category, description and proximity severity.
Bind the actual matched site and any helper definition. Ambiguous collapsed sites take the fallback.

| Current category / verified id | Positive projection witness | Tagged CWE | Admitted negative → corresponding _MATCH row |
|---|---|---|---|
| HASH_TRUNCATED_BELOW_128_BITS | Slice of a digest from bound Python hashlib (including a unique local hash-object alias). Actual hex width × 4 or byte width × 8 is below 128 for the identifier digest | CWE-328 | A non-hash object's `digest()[:8]` |
| HASH_TRUNCATION_VIA_CALL | Resolve this call to the defining helper. Its returned digest satisfies the first row, with the slice bound supplied by this actual argument to that formal | CWE-328 | Helper text mentions a digest slice in a string, or a different same-named helper returns ordinary data |
| WEAK_HASH_FOR_IDENTITY | Bound hashlib MD5/SHA-1 digest flows into the identified key/identity use in that assignment or its resolved consumer | CWE-328 | Shadowed `hashlib` on an identity-shaped assignment |
| SHELL_TRUE_WITH_INTERPOLATION | Bound subprocess run/Popen/call/check_call/check_output with a true shell option on this call, or bound os.system. At least one dynamic field of its command can affect shell syntax in its actual quoting context and shell dialect | CWE-78 | Fixed command beside unrelated interpolation, or every field protected as one data argument |
| CERT_VALIDATION_DISABLED | Disabling value in the peer or hostname verification slot of bound requests/httpx, Python ssl context, curl TLS option, Node TLS/HTTPS options or Go tls.Config, with that verification demonstrably missing at the TLS use | CWE-295 | Unrelated `helper(verify=False)`, or replacement verification intact or unresolved |
| HARDCODED_SECRET | The assigned literal reaches one of the credential slots listed below without rebinding | CWE-798 | Public display constant, unlisted client, or a literal used only as the username |

For the shell row, compare an unconstrained string parameter `x` used as the unquoted field in
`subprocess.run(f"printf '%s' {x}", shell=True)` on POSIX with the same field passed through
bound `shlex.quote(x)` as a whole data word. The first selects CWE-78 and the second
`SHELL_TRUE_WITH_INTERPOLATION_MATCH`. Check each field at its insertion site. If protection is
effective for all fields, or no field is proved syntax-capable, use the fallback. An interpolation
marker alone would tag the protected call. A function name alone cannot establish effective quoting.

For TLS, follow the effective configuration to a client handshake. Identify the missing peer-chain
or expected-hostname check, including any replacement verifier. For Go, a locally fixed
`tls.Config{InsecureSkipVerify: true}` with neither verifier callback nor other validation gives a
gap. A bound `VerifyConnection` that enforces both checks and propagates failure gives
`CERT_VALIDATION_DISABLED_MATCH`. An opaque callback or unresolved config change also gives that
fallback. A callback proved to accept every peer still leaves a gap if nothing else verifies it.
Using the flag alone would tag a verified connection.

The credential slots are a closed list for S5-P. On bound `requests` or `httpx` module calls
`request/get/post/put/patch/delete/head/options`, accept the password (second member) of the
`auth=(user, password)` pair, or the bearer token in the `Authorization` entry of the supplied
`headers=` mapping (case-insensitive header name). Also accept `password=` on the bound
`redis.Redis` constructor. Require the literal to supply the credential value at the use.
Other clients, slots and unresolved flows select `HARDCODED_SECRET_MATCH`. Inferring a credential
from arbitrary `password=` keywords would admit application helpers with different semantics.

A single truncation id loses the helper distinction. Digest width concerns collision resistance.
The credential rule spans passwords and keys. Unconditional tags mislabel the six controls above.
Dropping every tag loses the positive witnesses. Two constants keep each id's CWE immutable.

## Appendix D. Absence registry rows

In `default_pairs()` order, the first column is the exact current pair description. Today the algorithm
embeds that text in the three missing-counterpart findings, with no rule or CWE. The new id is a
literal constant independent of the description. D-1 tagged 24/65 ordinary pairs unconditionally
(4 × 401, 5 × 667, 7 × 772, 8 × 775). All 24 change their publication predicate, zero of the 540
taint rows do. Two lose their tag on every occurrence and 22 retain it only on proof. Stage M's 34
is an all-category absence total. The number reaching these pairs is unmeasured.

For `missing_counterpart`, `close_only_on_error_path` and `missing_close_on_error_path`, each
dash means **one** CWE-less id on all hits (43 pairs, filed F6). Each CWE means **two** fixed rows:
the id printed with its CWE for a verified **unmet** manual obligation on the reported path, and the literal id with `-match`
appended and no CWE for all other occurrences (F6). For example, `absence-go-file@CWE-775` and
`absence-go-file-match` are distinct constants. S3 registers 87 ordinary plus three repeat rows.
The producer selects exactly one. No runtime slug building or optional-CWE override.

For a tagged row recover the **full parsed acquisition** at the finding's open site, including
qualifier, language, binding and resource operand/result. Require the table's API and its local
manual-release obligation on success. An imported alias or typed receiver must resolve at that
use. For C/C++ require the corresponding library/kernel symbol (declaration/header context and no
shadow or replacement). Exclude managed prefixes `devm_`/`dmam_`, scope owners, ownership transfer
and unresolved lifetime. When the old line-level match has multiple possible acquisitions and cannot
identify one with that obligation, select the fallback. A substring, stripped method tail, neighboring
call or plain code-text match alone never establishes it.

For all 22 conditional pairs, carry the obligation for that resource along a feasible local path
from successful acquisition to the exit the finding concerns. For `missing_counterpart` identify a
specific reachable owner-scope exit. For `close_only_on_error_path` use the normal return named in
the description. For `missing_close_on_error_path` follow the reported goto through its label and
fall-through to exit, including cleanup between acquisition and goto. Use the emitter's sites
directly. One feasible path meeting the category's criteria with the obligation still outstanding
is a tagged witness. If none is proved, emit the single `-match` row. Checking the acquisition
and trusting the old counterpart scan would tag acquisitions already balanced on the reported path.

Keep this proof local to the selected path. Track the acquired value (or lock/reference), its
unique aliases and the release operation, including bound method values, function pointers and
cleanup scheduled to run at that exit. Match the release's resource slot. A close on a different
resource or a different path cannot discharge this obligation. A proved release on the selected
path discharges it. An opaque helper that may release it, an ambiguous alias, escape, or unresolved
control flow leaves the obligation unproved on that path. Restrict tagged witnesses to local
paths whose relevant effects can be accounted for. No interprocedural cleanup inference is required.
For Node, `const release = fs.closeSync` followed by `release(fd)` after `fs.openSync` discharges
that fd. For shell, parse both `exec 3<&-` and `exec 3>&-` as closes of fd 3 (likewise fd 4).
An admitted finding on either balanced sequence carries the corresponding CWE-less row.

| Observed pair description | Stable public id | CWE on path proof above | Required acquisition or reason for no tag |
|---|---|---|---|
| file open without close | `absence-file-handle` | — | Generic open/close spellings include scope-owned files. |
| lock without unlock | `absence-lock` | — | Generic lock/acquire includes scope guards. |
| connection opened without close | `absence-connection` | — | D-1 CWE-772 removed. Signal registration, reconnect and process-lifetime clients share this pair. |
| event subscription without unsubscribe | `absence-subscription` | — | Registration may live for the lifetime of the listener. |
| transaction begin without commit/rollback | `absence-transaction` | — | The intended commit or rollback boundary is unknown. |
| allocation without free | `absence-allocation` | — | Mixed malloc/alloc/new and release targets include managed storage. |
| timer set without clear | `absence-timer` | — | One-shot timers can finish by themselves. |
| item added without removal path | `absence-collection-item` | — | Persistent membership can be intentional. |
| span/timer started without end | `absence-span` | — | Instrumentation lifecycle. |
| resource acquisition without defer cleanup (Go) | `absence-go-defer` | — | The spelling of defer does not identify an acquired resource. |
| kernel allocation without free | `absence-kernel-allocation` | CWE-401 | Kernel kmalloc/kzalloc/vmalloc returning owned memory. |
| DMA allocation without free | `absence-dma-allocation` | CWE-401 | Kernel dma_alloc_coherent returning owned coherent memory. |
| IRQ registered without free | `absence-irq` | CWE-772 | Kernel request_irq/request_threaded_irq with a balancing unregister obligation. |
| spinlock without unlock | `absence-spinlock` | CWE-667 | Kernel spin_lock/spin_lock_irqsave on this lock. |
| clock enabled without disable | `absence-clock-enable` | CWE-772 | Kernel clk_prepare_enable taking a balancing clock reference. |
| platform driver registered without unregister | `absence-platform-driver` | — | Registration lifetime depends on driver ownership. |
| device tree node get without put | `absence-device-tree-node` | CWE-772 | Kernel of_node_get/of_find_node_by_name/of_find_node_by_type/of_find_compatible_node taking a node reference. |
| kernel mutex lock without unlock | `absence-kernel-mutex` | CWE-667 | Kernel mutex_lock on this mutex. |
| rtnl lock without unlock | `absence-rtnl-lock` | CWE-667 | Kernel rtnl_lock acquiring the networking lock. |
| kstrdup allocation without kfree | `absence-kernel-string` | CWE-401 | Kernel kstrdup returning an owned duplicate. |
| slab cache allocation without free | `absence-slab-object` | CWE-401 | Kernel kmem_cache_alloc returning an owned slab object. |
| RCU read lock without unlock | `absence-rcu-read` | — | RCU read-side nesting is a synchronization protocol, not a held mutex. |
| pthread mutex lock without unlock | `absence-pthread-mutex` | CWE-667 | POSIX pthread_mutex_lock on this mutex. |
| semaphore wait without post | `absence-semaphore` | — | A semaphore permit may intentionally be consumed or posted by another party. |
| mmap without munmap | `absence-memory-map` | CWE-772 | POSIX mmap yielding a manually owned mapping. Python mmap.mmap and NumPy memmap fail this contract. |
| POSIX file descriptor opened without close | `absence-posix-file-descriptor` | CWE-775 | POSIX open/openat/creat returning an owned descriptor. |
| directory opened without closedir | `absence-directory-stream` | CWE-775 | POSIX opendir/fdopendir returning the owned directory stream. |
| pthread rwlock without unlock | `absence-pthread-rwlock` | CWE-667 | POSIX pthread_rwlock_rdlock/wrlock on this rwlock. |
| Python threading lock without release | `absence-python-thread-lock` | — | Constructing a Lock/RLock does not acquire it. |
| Python multiprocessing pool without close/join | `absence-python-process-pool` | — | Scheduling work is mixed with creating the pool. |
| socket created without close | `absence-socket` | CWE-775 | POSIX socket returning an owned descriptor. Python scope-managed sockets and create_socket helpers use the fallback. |
| temporary file without cleanup | `absence-temporary-file` | — | mkstemp and auto-cleaning NamedTemporaryFile have different ownership. |
| Node.js stream without destroy/close/end | `absence-node-stream` | — | An auto-closing stream can finish normally. |
| server created without close | `absence-server` | — | A long-lived listener need not close in its creator. |
| database pool connection without release | `absence-database-pool-client` | — | pool.query can release internally, while pool.connect checks out a client. |
| fs.open without fs.close | `absence-node-file-descriptor` | CWE-775 | JS-family bound Node fs.open/openSync returning or delivering an owned descriptor. |
| lock/acquire without release/unlock | `absence-acquire-release` | — | Generic names identify multiple resource and scope-guard contracts. |
| Go sql.Open without db.Close | `absence-go-sql-db` | — | D-1 CWE-772 removed. A database/sql pool can live for the process, and other Open methods match. |
| Go file created without Close | `absence-go-file` | CWE-775 | Go os.Create/os.OpenFile returning an owned *os.File. GORM/client Create is a name hit only. |
| Go context without cancel (may leak goroutine) | `absence-go-context` | — | Cancellation, parent lifetime and deadline matter. |
| WaitGroup Add without Wait | `absence-go-wait-group` | — | A different goroutine can own the wait. |
| Go HTTP response body not closed | `absence-go-http-body` | CWE-772 | Go net/http Get/Post or (*http.Client).Get/Post/Do returning a response whose Body the caller owns. once.Do and cache.Get use the fallback. |
| Rust file opened without explicit flush/drop | `absence-rust-file-scope` | — | RAII closes the file. An explicit flush is a distinct operation. |
| advisory: Rust mutex lock held to end of scope (explicit drop() releases sooner) | `absence-rust-lock-scope` | — | Scope exit releases the guard. Earlier drop is advisory. |
| unsafe block without safety assertion or comment | `absence-rust-unsafe-documentation` | — | Presence of an assertion/comment is a review convention. |
| Rust TCP connection without shutdown/drop | `absence-rust-tcp-scope` | — | RAII drops the socket. |
| Rust Command created but never executed | `absence-rust-command-builder` | — | Creating a command need not execute it. |
| Lua file opened without close | `absence-lua-file` | CWE-775 | Lua's unshadowed io.open yielding an owned file. |
| Lua socket opened without close | `absence-lua-socket` | CWE-775 | Bound LuaSocket tcp/udp/connect returning an owned socket. |
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
| File descriptor opened but never closed | `absence-shell-file-descriptor` | CWE-775 | Parsed shell builtin exec with persistent fd 3/4 output/append redirection to a file and a local close obligation. Quoted text and command-local redirection use the fallback. |
| Lock file acquired but never released | `absence-shell-lock-file` | — | FD-scoped flock and persistent lock files differ. |
| Firmware flash write (mtd) without hash verification | `absence-firmware-verification` | — | A local checksum call alone does not establish an authenticity/integrity policy. |
| UCI config set without commit | `absence-uci-commit` | — | A caller may own the batch commit. |
| Kernel module loaded without unload in cleanup path | `absence-kernel-module` | — | Loaded modules may have system lifetime. |

This preserves weakness filtering on a verified descriptor or allocation. Dropping all 24 tags
would lose those uses. Keeping one tagged row per pair would label a local integer helper `open(1)`
as a descriptor acquisition. Changing the substring/method-only admission would move finding counts.
Specializing connection or SQL-pool lifetimes needs a separate ownership contract (F6).

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

**D-6 keyed delta from D-5, using the PR #901 fixtures at `b3bfd5515ece`.** All nine TSV
partitions retain their expectations (0/540). One of 300 candidate cells changes:

| Group and label | D-5 | D-6 | Reason |
|---|---|---|---|
| JS framework-source flat, `astra-r1-47` | `js-location-header` | `[]` on admission | The assignment and the Location value both evaluate `req.query.next`. Withhold the repeated field group. The response setter has no independent inline structured role. |

The other 47 Astra round-1 cells, all 43+5 later Astra cells, 120 Sol cells, 76 Terra cells
and eight own sketches retain their decisions. Thus the candidate populations become 43 predicted
absences, 15 admitted-empty and 242 nonempty conditional lists. Simple locals account for every
other repeated path-read comparison in §5.2. `ri02` requires RHS **and** command, `ri15` the
uncleansed format, `ri21a/b` both structured siblings, `ri22` the two live siblings and `ri23c`
both name origins. Nested and continued uses with one read retain all eligible enclosing roles.
These counts compare design cells. Neither a dormant writer nor the candidate sources establish
production yield. No registry, observation or presence cell changes.

**Round 1 keyed delta from D-1 (`65a13c348b47`).** This table preserves the superseded design value.
The tables below retain the separate TSV `rules` and `observed` history. D-3 keeps the D-2 cell values.

| Table | Exact label(s) | D-1 | D-2 |
|---|---|---|---|
| js | `bare redirect call` | `js-redirect@CWE-601` | `taint-redirect-name` |
| py | `redirect` | `py-redirect@CWE-601` | `taint-redirect-name` |
| hybrid_js | `bare imported query` | `js-db-call@CWE-89;structured-sqli@CWE-89` | `taint-query-name` |
| hybrid_py | `session.query(sql_var)` | `taint-query-value` | `taint-query-name` |
| hybrid_py | `r2: query lower var percent`, `r2: query CONST percent`, `r2: query CONST plus var` | `py-db-call@CWE-89` | `taint-query-name` |
| go | `QueryContext on any receiver`, `generic flat callee` | `go-db-call@CWE-89` | `taint-query-name` |
| hybrid_go | `dbConn.Exec`, `probe: go dbConn.Exec` | `go-db-call@CWE-89` | `taint-query-name` |
| candidate | `sol-r2-50` | `js-redirect@CWE-601` | `taint-redirect-name` |

The two JS cross-spawn labels retain `js-child-process@CWE-78;structured-os-command@CWE-78`
on the reached program with `shell: true`. The bound Next redirect and the response-method rows
keep their redirect id. The injected callback in the candidate has neither binding. The Go
`*sql.Conn` fixtures call `Exec` where that type offers `ExecContext`. The Python formatting
fixtures supply expressions but no SQL literal or resolved execution API. Presence and historical
classifier observations are independent of these eleven TSV and one candidate identity changes.

**Round 2 keyed delta from D-2 (`be906634a77c`).** Zero of 540 TSV cells and zero of 300
candidate cells changed. The committed Flask fixture reaches positional source. The new context
permutations are S2-PY controls. The SQL-literal witness is limited to `query` (review extension fold-back), so one candidate cell
(`astra-r2-04`) changes to `taint-query-name;taint-query-value!suggestion`. No existing TSV execute case gains an id. Primitive, absence and mixed
specific-mode descriptors are outside the taint matrix. The table above remains the delta from D-1.

| Source table | Rows | Previously `?` | Concrete expectations revised |
|---|---:|---:|---:|
| identity | 34 | 21 | 1 |
| js | 70 | 17 | 5 |
| hybrid_js | 127 | 48 | 0 |
| py | 46 | 5 | 5 |
| hybrid_py | 101 | 13 | 11 |
| go | 39 | 6 | 7 |
| hybrid_go | 70 | 15 | 6 |
| other | 8 | 0 | 0 |
| hybrid_other | 45 | 0 | 0 |
| **Total** | **540** | **125** | **35** |

The reviewed Stage T fixture has 415 concrete rows. D retains 380 and revises 35. Together with
125 unknowns, 160 cells need transcription. Ten concrete rows have an explicit observation: seven
differ from the expected value (`ri08`, `ri25`, `ri26`, the Java probe, the Go io.Writer case and
the Echo/Iris raw HTML cases). Three agree (`ri23e`, `ri23f` and `ri27`). The other 405 use `=`.
The observations describe the classifiers. The D column specifies the new publication behavior.

The brief's original fixture at `a3986ce333f1` has the same 540 keys and fixture bodies.
It had 124 unknowns, 39 revised concrete expectations and 377 retained. The intervening Stage T
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
| `js-redirect@CWE-601` | `=` | `js-redirect@CWE-601` | `res.redirect` |
| `js-redirect@CWE-601` | `=` | `taint-redirect-name` | `bare redirect call` |
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
| `?` | `js-db-call@CWE-89+structured:sqli` | `js-db-call@CWE-89;structured-sqli@CWE-89` | `mssql request.query`, `optional-chain query`, `TSX db.query`, `probe: mssql request.query` |
| `?` | `js-db-call@CWE-89+structured:sqli` | `taint-query-name` | `unknown receiver query`, `bare imported query` |
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
| `py-redirect@CWE-601` | `=` | `taint-redirect-name` | `redirect` |
| `py-file-open@CWE-22` | `=` | `py-file-open@CWE-22` | `open call` |
| `py-file-remove@CWE-22` | `=` | `py-file-remove@CWE-22` | `os.remove` |
| `-` | `=` | `-` | `get_user_id call (kernel pattern)`, `get_user call outside C`, `def get_user_id signature`, `getattr with a literal name`, `setattr with a literal name`, `compile`, `run name without subprocess binding`, `run call without subprocess binding`, `variable named source`, `variable named raw`, `revalidate (eval substring)`, `evaluate (eval substring)`, `render helper`, `data.find`, `socket send`, `text with a literal`, `text without sqlalchemy binding`, `execution_result variable`, `django filter with keyword lookups` |
| `-` | `=` | `-` | `filter_by with keyword lookups`, `lookup_where helper`, `redirect_url variable`, `freeze (free substring)` |

#### hybrid_py

| TSV rules | Observed | D | Exact fixture labels |
|---|---|---|---|
| `py-os-command@CWE-78` | `=` | `py-os-command@CWE-78` | `import os as o`, `from os import popen`, `from os import system as sh`, `os.execvp`, `os.spawnlp`, `from os import execv`, `asyncio.create_subprocess_exec`, `asyncio.create_subprocess_shell`, `pty.spawn`, `probe: py control: os.system`, `probe: py import os as o`, `probe: py os.execvp`, `probe: py os.spawnlp`, `probe: py asyncio.create_subprocess_exec`, `probe: py asyncio.create_subprocess_shell`, `probe: py from os import popen` |
| `py-db-call@CWE-89` | `=` | `py-db-call@CWE-89` | `cursor.executescript`, `conn.exec_driver_sql`, `client.query f-string`, `client.query concatenation`, `client.query percent format`, `client.query .format`, `pd.read_sql_query`, `pd.read_sql`, `probe: py cursor.executescript`, `probe: py conn.exec_driver_sql`, `probe: py bigquery client.query`, `probe: py pd.read_sql_query`, `r2: query keyword f-string`, `r2: query reordered keywords`, `r2: query sql= keyword percent` |
| `py-db-call@CWE-89` | `=` | `py-db-call@CWE-89` | `r2d: parenthesized implicit-concat SQL keyword`, `r2d: parenthesized concatenation SQL` |
| `py-db-call@CWE-89` | `=` | `taint-query-name` | `session.query(sql_var)`, `r2: query lower var percent`, `r2: query CONST percent`, `r2: query CONST plus var` |
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
| `go-db-call@CWE-89` | `=` | `taint-query-name` | `QueryContext on any receiver`, `generic flat callee` |
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
| `go-db-call@CWE-89` | `=` | `go-db-call@CWE-89` | `sqlx Queryx`, `sqlx MustExec`, `sqlx NamedExec`, `sqlx QueryRowx`, `handle *sql.DB Query`, `pgx r.pg.Exec`, `gorm chain Exec`, `method value query := db.Query`, `probe: go control: db.Query`, `probe: go sqlx db.Queryx`, `probe: go sqlx db.MustExec`, `probe: go gorm db.WithContext(ctx).Exec`, `probe: go pgx r.pg.Exec` |
| `go-db-call@CWE-89` | `=` | `taint-query-name` | `dbConn.Exec`, `probe: go dbConn.Exec` |
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
| `[]` | terra-r1 (08) · terra-r2 (10) · sol-r1 (51, 56, 57) · sol-r2 (59, 60) · astra-r1 (04, 18, 31, 47) · astra-r2 (09, 12, 16, 23) |
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
| `js-redirect@CWE-601` | sol-r1 (05, 06, 07, 10, 11, 12, 13, 27) |
| `taint-redirect-name` | sol-r2 (50) |
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
| `js-child-process@CWE-78;js-db-call@CWE-89;structured-os-command@CWE-78;structured-sqli@CWE-89` | astra-r2 (01, 02) |
| `py-db-call@CWE-89;taint-query-value!suggestion` | astra-r2 (03) |
| `taint-query-name;taint-query-value!suggestion` | astra-r2 (04), changed in the round 2 fold-back: an unresolved `execute` receiver has no SQL witness |
| `py-html@CWE-79;structured-xss@CWE-79` | astra-r2 (13) · astra-r3 (02) |
| `js-fs@CWE-22;js-redirect@CWE-601` | astra-r2 (17) |
| `py-module-command@CWE-78;py-subprocess@CWE-78` | astra-r2 (18) |
| `js-db-call@CWE-89;structured-sqli@CWE-89` | astra-r2 (20) |
| `go-html-render;taint-query-value` | astra-r2 (21) |
| `go-file-op@CWE-22` | astra-r2 (34) |
| `structured-ssrf@CWE-918;structured-xss@CWE-79` | astra-r3 (01) |

In `astra-r2-04` the reached ORM comparison in `session.execute(select(User).where(User.name == x))`
contributes the weak value role. The sibling `conn.execute("SELECT " + x)` has an unresolved receiver: an injected
client that only records text would receive CWE-89 from a literal-only witness (design review extension, reviewer B),
so A grants the SQL-literal witness to `query` alone and the candidate is `taint-query-name;taint-query-value!suggestion`.
The cost is that the common sqlite3 `conn.execute("SELECT " + x)` shape loses the CWE-89 distinction until a receiver
binding proves SQL execution.

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
| F1. Ambiguous callee or receiver | taint-exec-name, taint-eval-name, taint-query-name, taint-remove-name, taint-html-name, taint-redirect-name, structured-name-match | Resolve the actual receiver/method and reaching definition, including conditional rebinding, injected callbacks and imports of local wrappers. |
| F2. Output, navigation and client requests | taint-write-data, js-write, go-writer-output, go-html-render, js-location-header, browser-open, external-open, structured-client-request | Establish content type/escaping, downstream consumer, redirect status, handling of the target or a server-side request. Bytes at a fixed destination, a template name, a browser launch and a browser-only request do not establish those weaknesses. |
| F3. Query values and process arguments | taint-query-value, taint-process-argument | Determine whether the reached value can become SQL or shell syntax. Bound values, positional ORM objects and fixed-program argv do not by themselves select that syntax. Retain real weak emitters and their relative severity. |
| F4. Dynamic attributes and file mode | py-dynamic-attr, go-file-mode | Establish use of the selected attribute for dispatch, or the file's effective permissions and policy. A computed read and a permission mask alone do not establish unsafe reflection or path traversal. |
| F5. Name-only detectors | configured-sink, format-wrapper, legacy | A configured name has no declared weakness. The wrapper scan proves one-hop variadic-plus-printf co-occurrence, not the forwarded formal or slot. The legacy list has no operand model. Per-user ids would also make the registry open. |
| F6. Missing counterparts without a proved unmet obligation | The 43 dash ids and the 22 `-match` ids in D | Establish the acquisition, owner and obligation still outstanding at the exit on the reported path. Record discharged obligations separately from unresolved cleanup. Connection and SQL-pool lifetimes need their own rule before they can carry a CWE. |
| F7. Repeated counterparts | absence-repeated-counterpart | Establish two matching deallocations or unlocks, their common operand and the intervening lifetime before specializing. |
| F8. Primitive detector matches | Six `<CATEGORY>_MATCH` ids in C | Establish C's specific witness, including shell-syntax exposure, a TLS verification gap or a use on the closed credential list. Record effective and unresolved protections separately. |

**Open question for the owner (earlier Opus S5 on Node).** Should Node `fork(modulePath)` distinguish CWE-829 or
CWE-114 for code loading? The one `esm default` TSV row keeps `js-child-module@CWE-73`. It proves
control of a filename. The alternatives need evidence of untrusted functionality or process control.

**Accepted detection and attribution losses**

- D-6 withholds path-based flat credit for repeated identical request-field accesses on a
  receiver path (`astra-r1-47`). It also withholds individual reads with unresolved declaration,
  unknown effects/escapes, an ambiguous reaching value, missing source provenance or unproved
  field storage. A compound assignment may still supply its independently proved incoming read.
  Unknown callee, ambiguous formal and old-index/semantic-role disagreement withhold the affected
  role. These are per-witness losses. Preserve an independent path, inline source, name origin
  or legacy token when its own contract holds. No other labeled loss is accepted by D-6.
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

**Measure before enabling each producer.** Stage M ran `9d1a7d06f653` twice on 35 inputs (tree
`6a5a242ad8e7`, 210 output pairs). Count one analysis per input and report its actual `taint_sink`
denominator alongside the 81 all-category findings. Exclude output copies. Trace old line/first-pattern
hits and D on identical parse, paths and admission state. Label the private tag/category-default as
an old proposal. For round-1 refusals compare D-1 and D-2 on those witnesses. For round 2 compare
D-2 to D-3 on the affected Flask/SQL, primitive and absence witnesses in their pinned populations. Run G-additive
and G-fingerprint with ordinary uninstrumented binaries.

1. **Admission.** Key by pinned input/diff id, analysis variant (baseline/expanded for #841),
   relative file, physical line and category. Distinct unadmitted keys go in a separate population
   with the old admission branch/outcome. Compare finding and severity multisets with multiplicity.
   For #841 also report the overlap/expanded-only keys and the selected run after union.
2. **Join old hits to selected D occurrences.** Within an admitted key use detector family, private
   tag/pattern (language + table + entry, or name-detector origin) and half-open byte span of the
   callee leaf/operator. Instrument the exact old visit or selected call, including JS YAML,
   Flask path and inline, Go primary/secondary, flat identifiers, configured and wrapper visits.
   Use a stable table/entry ordinal (or equivalent unique key including its indices), so Go's two
   `exec.Command` entries at 0 and 2 remain distinct. A non-call identifier uses its token span.
   In #901's `function f(x) { audit(x); }` control the two visits lack destination spans, but both
   start at the exact tainted formal. Expect one located `audit` parent, two raw visits, two missing
   operand spans and **one proved child** at the argument, selecting `configured-sink` once.
   Repeat the two visits with injected origins at a different callable's formal `x`, no transfer to
   `f`'s formal and no independent source. Expect the same first three counts, **zero proved
   children**, and an admitted-empty finding. Only when the old
   *detector site itself* cannot be located record one unmatched group per finding/detector/entry/branch,
   with raw visit count and unknown occurrence multiplicity, separately from occurrence totals.
   Outer-join the located parents once per key. Keep path/inline channels as a set. Count old-only,
   D-only and common parents. For common parents show old → sorted D identities and flag every CWE
   neutralization. An old line hit has no old operand. Attach all D operand children to its one parent.
3. **Operand decisions before public-id reduction.** Child key = parent + argument/RHS byte span
   and prospective semantic slot (separate spans for two paths of rename). Enumerate each operand
   carrying a candidate use or inline witness, including wrong slots. Multiple paths count once.
   Any independently valid witness wins. For refusals take the first applicable reason:
   framework-source flat filter, wrong/unreached slot, unresolved binding/inline source, cleansed
   or safe range, then unmodeled role. Treat a neutral published role as selected, with its F reason.
   A located old-only parent takes its first child refusal in that order, or `unreached` if no child.
   Keep parent and child totals separate. Label overlapping channel views. Give distinct affected
   findings and one location per nonzero reason.
4. **Publication.** Group surviving (child, public id) contributions by finding and id. Count excess
   contributions as same-id collapses and count all-weak warning overrides after the strength reduction.
   Count each admitted key with no surviving id once as admitted-empty. Report these as separate
   stages, never as additive refusal reasons. Two paths to the same `res.send(x)` give one common
   parent, one selected child, one id and zero collapses. On an admitted `fmt.Sprintf(format, x)`
   with only the substitution reached, the old format hit joins to zero selected children: one
   old-only parent (wrong slot), one refused substitution, one admitted-empty finding. For the local
   `./db` query export two detector parents select one neutral id: two neutralizations, one collapse,
   one affected finding. These counts have different denominators.

**D-6 census before each activation.** Use Appendix E at D-5 as the keyed reference for labeled
losses and E at D-6 as the acceptance oracle, conditional on the recorded admission. For the
35 pins, use a **D-5 reference per admitted finding**. S2-WC records the old visits, paths,
guards and operands. For each prospective loss, independently reconstruct the D-5 rows and
strengths from that evidence and the D-5 A/B rules at `8273c09c`, across all applicable channels
on the same pinned input, variant and key. Record the witnesses and reviewed reference list.
Compare it with the D-6 list after both sides collapse ids and reduce strengths relative to the
same finding severity. Count missing ids, weakened retained ids and newly empty findings on
these comparable keys. Report reference coverage (compared / admitted) and refusals on keys
without a complete reference separately as unresolved. PR #901 published no rule list. Its
private detector tag alone cannot establish a prior list, strength or measured loss.
Record the proof route (stable local, exact point, inline or legacy), def/read spans and lines,
admitted key, old/semantic operand spans, role, strength and refusal detail under the first applicable F reason.
Split binding refusals by missing evidence, shadow, multiple writes, escape/unknown effects,
ambiguous field, and repeated request field. Split slot refusals by unproved callee, spread,
wrong role and old-index disagreement. Keep unknowns separate. Tabulate raw visits, unique
parents, selected/refused children, independently rescued roles, and losses *after* id/strength
reduction including newly empty lists. Exercise zero, one and two truly unlocated old visits,
and zero, one and two missing operands on a located site. Require the sole changed E control
in JS and unchanged positive controls in each language. Run the joined census on all 35 pins;
report observed denominators and examples, including zero support for any named loss. Report the
two grids' tested coordinates independently of the 840 labeled cases. A further changed E cell
or a wrong identity blocks activation pending correction or another design decision.

**Single mention with multiple reaching definitions (D-6 review S5, owner threshold open).**
Keep the D-6 proof routes. For example, a conditional `if not x: x = default` before
`os.system(x)` leaves a single read in the call but two possible definitions. Count the
case separately when old edge bytes locate the sole value read of that binding on the read
line, multiple definitions can reach it, and that ambiguity prevents proof of the incoming
value for an otherwise eligible, uncleansed operand. A second, non-reaching write alone does
not establish this bucket. S2-WC records the child,
its admitted key and cause. Each language replays the census with its adapter before activation.
Report children, distinct admitted keys, independent rescues at equal or greater strength for
the same id, reference-supported losses after reduction, and unresolved comparisons. We recommend
returning to design before activation at **one or more distinct admitted keys across the 35 pins**
with such a refusal and no independent rescue for that role at equal or greater strength.
This low threshold makes the first observed unrescued case available for an owner decision.
It applies even if the full D-5 list for that key is unresolved. Report unknown reachability
separately. The threshold selects cases for review and grants no additional route.

Separate before/after totals could hide a lost occurrence behind a new one. The joined yield is
diagnostic. A valid in-scope witness lost through an incorrect predicate
requires a bounded correction, keyed expectation and remeasurement before publication. Volume alone
cannot authorize an unsupported CWE. Record zeros, unknowns and uncovered cells separately.

At S5 compare all six primitive and 34 absence findings with multiplicity on the same input
and original category/severity/description. For each C row show matched site/operand and verified
versus `_MATCH`, with the effective shell quoting, verification gap/protection or credential slot
as applicable. For each D pair/category record language, **full matched callee spelling** (raw
text for a text-only hit), binding, resource, reported path and exit, and the tagged or CWE-less row.
Distinguish obligation outstanding, discharged, unresolved and no manual obligation.
Tabulate repeated calls by both release sites and fallback reason. Count a multi-pair line once per
emitted finding. Show zeros across the entire inventories. Exercise both sides on separate fixtures
where the pins have no witness. At S4 count specific-mode empty fallbacks by algorithm/category on
the pins and check their indexed descriptors for tags. Use the mixed synthetic pair to exercise
the external-input collision. Stage M has no external-input population.

**Change log, 2026-11-24.** Stage D fixed the publication tables, both open-grid rules, all 540 TSV
decisions and the 300 source-only candidate dispositions at design HEAD `457ea9ffa6de`. The matrix
checkout advanced from `a3986ce333f1` to `5e9e4d864cbb` during D. Stage M used a separate binary.

**Change log, 2026-11-24, D-2.** Folded round 1 at `65a13c348b47`. E changes eleven of 540 TSV
identities and one of 300 candidate decisions. A/B close the binding and operand rules, C/D split
tagged from neutral occurrences, 5.5 normalizes SARIF inputs, and F defines the census join.
PLAN puts the registry first and assigns each language a separately gated producer PR.

**Change log, 2026-11-24, D-3.** Design review round 2 fold against `be906634a77c`. Both reviews'
eight findings are folded into 5.2–5.5, A–F and the corresponding PLAN gates and slice controls.
E retains all 540 matrix and 300 candidate identities. The round 2 verdicts were REJECT and APPROVE.

**Change log, 2026-11-24, D-3b (review extension, orchestrator fold-back).** The focused verification of the D-3 folds by reviewer B (REJECT) found one new defect in the W-A fix: the SQL-literal witness on an unresolved `execute`/`executemany` receiver can attach CWE-89 to a non-SQL client (`executor.execute("SELECT ..." + x)`). Reviewer A had offered the alternative in its own finding W-A; it is applied: the witness is limited to `query`, and the candidate `astra-r2-04` changes to `taint-query-name;taint-query-value!suggestion` (one candidate cell; zero TSV cells). Edited in A, B, E, the keyed delta and PLAN S2-PY. All other round-2 folds were verified resolved by reviewer B.

**Change log, 2026-11-25, D-4 and D-5.** D-4 (`30216157813a`) restated goal 4. The earlier “only through”
claim becomes three obligations in §5.3. Rust privacy blocks struct literals and direct field writes
outside the registry, tests check emitted values on exercised paths, and producer review traces
how built-ins selected their rows. External serde and forwarding remain possible. D-5 folds the
single §8 review onto that decision. It specifies the common writer, lane 8's core import, the
S2-R controls and the algorithm encoding for both specific-mode descriptor namespaces (§5.5).

**Change log, 2026-11-25, D-6.** At the second collector rejection, §5.2 replaces generic
representative expansion with language-owned proofs and preserves separate semantic and old
matcher operands. Appendix E changes `astra-r1-47` only. F and PLAN assign the old-site census
and dormant language tests before publication. The 64 registry rows remain settled.

**Change log, 2026-11-25, D-7.** Folded the single review of `45264869`. Section 5.2 binds the
continued Go read to the originating path's admission key and documents existing proof helpers and
the C macro assumption. F distinguishes recovered spanless operands, proved losses and unresolved
refusals. D-7 changes zero D-6 expectation cells. Appendix E retains the D-5 delta of 0/540 TSV
and one candidate, `astra-r1-47` admitted-empty (43 absent, 15 admitted-empty, 242 nonempty).

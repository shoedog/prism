# Lane 10: subscripts in the data-flow graph — design

Status: the census design (section 6) is final. The identity and overlap rule (section 5.3) is fixed; the path
encoding and the remaining items of section 5 are tentative until the census, the executed test matrix and
enumeration (PLAN stages M, T, E). The plan files a tracking issue. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane10-keyed-subscripts.md`; design review (astra, round 1):
`~/prism-evidence/lanes-2026-11-20/reviews/astra-r1/lane10/review.md`.

## 1. Intent

Data held in dictionaries, maps and objects is read and written with subscripts: `payload["user"]`,
`m["k"] = v`, `o["x"]`. The lane was proposed to make such paths key-sensitive (`d["a"]` distinct from
`d["b"]`). The research found a larger defect underneath: **a value written through a subscript reaches
nothing**. `d["a"] = tainted; sink(d["a"])` is not reached in Python, JavaScript or Go, and neither is
`x = d["a"]; sink(x)` or `sink(d)`. Recall comes first; key sensitivity is what keeps the recall fix from
flooding.

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| An access path is a base name and a list of field strings. Every subscript becomes the one component `[]`, and the text parser cuts at the first `[`: `d["u"]["n"]` and `d["a"].c` are both `d.[]`. | `src/access_path.rs:33`, `from_expr` |
| A subscript write records one definition, `base.[]`, over the whole target; the key and anything after it are dropped. `del d["a"]` and JavaScript `delete d["a"]` record nothing. A subscript target in a JS `for (… of/in …)` head or as a direct leaf of an array pattern (`[o["a"]] = [t]`) records nothing at all (the header and pattern collectors skip it). | `src/ast/lvalues.rs` (`extract_for_in_lvalue_spans`, `extract_destructuring_def_spans`) |
| A plain subscript read records a use of the base variable (and of an index variable), not of a path. Only a read where a field access wraps a subscript (`d["a"].c`) records a use with the path `d.[]`. | `src/ast/rvalue_identifiers.rs:80-96` |
| Field sensitivity for dot fields comes from one index keyed by the path text of field-access nodes: a definition reaches only references whose path equals its own. Subscript nodes are not in that index. So the definition `d.[]` is an orphan: nothing but `d["a"].c`-shaped reads can reach it. | `src/ast/member_reference_index.rs:23-120` |
| Probes with `taint-reaches`, three languages. False negatives: `d["a"] = t; sink(d["a"])`; `…; sink(d)`; `…; x = d["a"]; sink(x)`; through an alias; augmented assignment; JavaScript `o["x"] = t; sink(o.x)` and `o.x = t; sink(o["x"])`. False positives: a literal `d = {"a": t, "b": "safe"}; sink(d["b"])` (the whole container is tainted); `d["u"].n = t; sink(d["u"].m)`. Also wrong: a write to another key is labelled as killing (`d["a"].c = t; d["b"].c = "x"; sink(d["a"].c)` is `killed`). Dot fields behave correctly. | research section 5 |
| Defects on main that the lane pins as known wrong rows and does not widen (review probes): Python `make()["a"] = "x"` records `Def(make)` and `Def(a)`, and JS `make()["b"].field = 1` records the whole text as one base; dynamic-only chains give `exact` across different runtime keys (`o[j].z = "safe"; o[i].z = t; sink(o[j].z)`, `o.[]` 35→36 `exact`, runtime safe); Python dot fields stay `exact` across a root rebinding (`o.x = t; o = a; sink(o.x)`) and across a write through an uncertified alias (`p = o; p.x = "safe"; sink(o.x)`); the JS object-literal binder gives `exact` from `t` into the callee's `q["b"]` for `put({a: t, b: "safe"})`. | review probes A, B, C |
| Today's output already contains a field string spelled `["x"]`: `(o)?.["x"].f` gives the fields `['["x"]', 'f']` (optional chaining is normalised, then the parenthesised tail is split on dots). A JS `$o.x` is recorded as the one base `$o.x` with no fields. | review probe C |
| The same gap exists for dot fields in one direction: `o.a = t; sink(o)` is not reached in Python and JavaScript. Go has a separate pass that connects field writes to later whole-object reads. | probes; `CLAUDE.md` ("Go eager aggregate reads") |
| Review taint is a different path model: it is not field-sensitive even for dot fields (`d.name = tainted; d.other = "safe"; sink(d.other)` is reported), and it matches request reads such as `req.body["x"]` by text. A subscript on the left of an assignment gives it no target in Python and Go. | research section 5 |
| Killed edges are still followed by taint; a correct kill changes a label, not a verdict. | research |
| Readers of path spellings: reaching definitions and kills (exact equality, except JavaScript, where `[]` is a wildcard in the kill relation in both directions by an earlier design decision; the same `affects` relation also drives D3, D2-c, the refusal classes, alias kills and the census trace), the JavaScript write census (any index is `[]`, pinned by tests), the object-literal binder (which already has a static-key policy), Step 5b argument binding (production, legacy twin, serial reference), Step 5c return flow and its reference, capture identities and capture-region placement, alias resolution (`aliases.rs` reparses the target string), member-only formal projection, the JS `for` initializer replay check, navigation output. The path is stored in the cached graph. Fixture counts are text matches, not behaviour pins: 116 of 176 census rows contain `[]` (14 site paths hold an index), the five MCP goldens print only `helper` with no fields, and the 105 alias lines belong to a call-resolution matrix. | research sections 3 and 8; review findings 4, 6, 18 |
| Corpus counts, by regular expression on non-test files: string-literal subscripts 3,984 (1,447 writes) in one Python corpus and 2,653 (760 writes) in another; 5 to 6% of their functions use two or more different literal keys on one base. Excalidraw 736 (32 writes). One Go corpus 200. Python's `.get("k")`, `.pop("k")`, `.setdefault("k")` are about as frequent as subscripts. | research section 6 |
| An earlier architecture note chose to ignore index values and to revisit "if false positives from array collapsing become a problem". | `docs/features/cpg/architecture.md:313` |

## 3. Goals, non-goals, anti-goals

Goals:

1. A value written through a subscript reaches later reads of that element, in Python, JavaScript/TypeScript
   and Go.
2. A static string key is part of the path, so a write to one key does not reach a read of another, and
   does not kill it.
3. In JavaScript and TypeScript, `o["x"]` and `o.x` are the same path when the key is an identifier-like
   string.
4. Dynamic keys keep today's meaning: a write with an unknown key may write any element; a read with an
   unknown key may read any.
5. Every rule is checked against executed truth on small programs, and against a labelled sample on real
   code.

Non-goals (this lane): integer indices and slices (arrays stay index-insensitive); dictionary methods
(`get`, `pop`, `setdefault`, `update`); `Map` and `Set` methods; keys held in constants or built by
expressions; containers passed to other functions (element paths across calls); making review taint
field-sensitive.

Anti-goals: connecting every element write to every element read of the same container without key
sensitivity (a recall fix that floods); changing the meaning of `[]` for dynamic keys; changing path
spellings for code that has no static-key subscript.

## 4. Approaches considered

**A. Recall only: let the orphan definition `d.[]` reach every subscript read of `d`.** Small, and fixes the
false negatives, but it makes `d["a"] = t; sink(d["b"])` reached, in the 5 to 6% of functions that use
several keys, and in every request-payload handler. Rejected as a first step.

**B. Keyed components for static string keys, then reads as path uses** (chosen). The path gains a keyed
component; reads of subscripts become uses of paths; the existing equality-based index then connects a
keyed write to reads of the same key, and a small wildcard rule connects dynamic keys.

**C. Treat a container as one abstract object** (every write taints the base variable, every read reads
it). This is what literal initialisers do today and is the source of the false positives. Rejected.

**D. Leave the graph alone and special-case subscripts in the taint engines.** It would fix taint only, in
two engines, and leave slicing and navigation wrong. Rejected.

## 5. Architecture — *encoding tentative; identity rule fixed*

### 5.1 Path components

A path component is one of: a field name (as today); the dynamic-index component `[]` (as today); a keyed
component for a subscript whose index is a string literal without interpolation or escapes. Its encoding in
`fields` and in `Display`, `Debug` and JSON is fixed at stage D under two conditions: it is injective with
field names, with `[]` and with every field string the text parser can already produce (today's output holds
a field spelled `["x"]`, section 2), and no existing output changes for code without a static-key subscript.
An internal tagged identity beside an unchanged external spelling is admissible.

| Language | `x["k"]` | `x.k` | Same path? |
|---|---|---|---|
| JavaScript, TypeScript | field `k` when `k` is an ASCII identifier-like string other than `__proto__` (the object-literal binder's existing static-key policy); otherwise keyed `["k"]` | field `k` | yes for identifier-like keys |
| Python | keyed `["k"]` | field `k` | no: items and attributes are different namespaces |
| Go | keyed `["k"]` for a map index that is a string literal | field `k` | no |

Integer literals, variables, constants and expressions give `[]`. A chain that holds at least one static
key is built from syntax nodes by one shared function, not from text, and keeps every component
(`d["u"]["n"]` is `d.["u"].["n"]`; `d["a"].c` is `d.["a"].c`). The builder answers one of three ways:
**unaffected** (no static key: the chain keeps today's text-derived path exactly, `d[k].c` stays `d.[]`, so code
without static-key subscripts does not change), **admitted** (the full path), or **refused** (a static key on a
chain the builder cannot place: a call, constructor, conditional or tagged root, a comment or escaped name
inside the chain, a root today's dot extraction spells differently, such as JS `$o` whose dot twin `$o.x` is
one base). A refused chain is never handed back to text extraction (section 5.2). The JavaScript
identifier-like test is the object-literal binder's rule, repeated in the new module; unifying the two is a
follow-up.

### 5.2 Definitions and uses

- Every write-target entry point (plain, augmented and logical assignment, update, JS `for … of/in` heads,
  array and object pattern leaves, Python and Go target lists) passes a subscript target to the builder. An
  admitted target records a definition of its full path; a refused one is consumed as a unit: no definition of
  any name in it, while its evaluated inputs (call arguments, receivers, key expressions) stay reads. A target
  shape no entry point can hand over is listed with a matrix row.
- Roles are separate: a plain assignment target writes and does not read the old element; augmented,
  logical and update targets read and write; a delete kills. Write-only targets are never registered as reads,
  in every language, not only on the fenced JS path.
- A subscript read records a use of its full path at the subscript's bytes, in addition to the base use it
  records today.
- `del d["k"]` and `delete o["k"]` record a kill-only definition of the path where the language's census
  already supports kill-only definitions (JavaScript), and are left as today elsewhere.

### 5.3 Identity, overlap and flow

Identity and overlap are different relations, and every rule of this lane uses them as defined here.

**Definite identity.** Two paths denote the same storage for certain only when every component of both is
static, the components are equal (JS: by the identifier rule, so `o["x"]` equals `o.x`), and the base is the
same binding by the predicate that language's field rules already use: Go, `BindingFacts`
(`src/cpg/binding_facts.rs`, the def and every use-line occurrence bind one declaration, as rule S checks);
Python, the rebinding census behind `python_prune::stable_receivers` (`strip_rebound`), applied to the base
name: bound once in the owner and never rebound, nested callables included; JavaScript, `js_prune::stable_bases`
with the writer eligibility of `src/cpg/reaching/affects_writers.rs`. A rebound receiver, an alias the
language does not certify (JS `alias_cert.rs` certifies single-step aliases; Python and Go certify none) or a
base no predicate proves fails identity. Only definite identity may give an `exact` edge or a kill.

**Possible overlap.** Everything else that may be the same storage: a dynamic component anywhere in either
path (`d[i]["a"]` and `d[j]["a"]` included, though their spellings are equal), or an unproven base. A possible
overlap gives an edge; the edge is not `exact`, and it kills nothing. An edge whose route crosses a possible
overwrite (a write with possible overlap with the def's path) is not `exact` either, when every route from the
def to the use crosses one (a route that avoids all of them keeps its label, as for the JS alias may-kill of
#783, `src/cpg/reaching/alias_kill.rs`). Two different static components at the same position never overlap.

| From | Reaches | Label |
|---|---|---|
| a definition of a lane-built path `p` | uses of `p`, and of paths that extend `p` | `exact` only under definite identity and no crossed possible overwrite; otherwise the overlap label |
| a definition and a use in possible overlap (a dynamic component on either side, an unproven base) | the use | never `exact`: `NameOnly` with the existing alias-unstable doubt, or a new doubt kind fixed at stage D |
| a definition of an element path | a later whole-container value read (`sink(d)`) | only if the census justifies the slice (section 6 and the paragraph below) |

Each admitted (read path, byte span) is its own candidate: two reads on one line, one matching exactly and one
by overlap, give two edges, deduplicated and labelled per edge pair. A dynamic subscript read gets a real use
only where a lane-built definition of the same base needs one (`d["a"] = t; f(d["a"], d[k])`); pure
dynamic-only code keeps today's nodes. Whether `d[i] = t; sink(d[j])` (no static key on either side) becomes a
promised may-flow or stays an accepted miss is a stage D item.

Whole-container reads (conditional slice). A consumer is a maximal value operand: an argument, a return
value, the right side of an assignment of the whole container or of an enclosing subcontainer
(`sink(d["u"])` after `d["u"]["n"] = t`). Evaluating a receiver, computing a key, membership, length,
identity, a method call and a nested projection are not consumers (the Go pass excludes the indexed storage
operand in the same way, `go_aggregate.rs`). The source contribution is killed by a write of its path or a
prefix, by a delete, and by a root rebinding; a descendant write weakens it. Whether JS identifier-key writes
(recorded as fields, `o["a"] = t`) are sources, which would also admit the deferred `o.a = t; sink(o)` lane, is
a stage D item.

### 5.4 Kills

Two predicates replace the one `affects` relation where kills are decided: **replaces(w, d)**, a write that
definitely overwrites `d` (`w`'s path is `d`'s path or a static prefix of it, under definite identity; a root
or field prefix counts: `d = {}` or `obj.items = …` kills `d.["a"]`, `obj.items.["a"]`), and **may_overlap(w, d)**
(section 5.3). A kill uses `replaces`. Consumers that test interference rather than overwrite (JS D3 order,
D2-c captured writes, the refusal classes, alias-kill refinement, the census trace: `d3.rs`, `d2c.rs`,
`refusal_classes.rs`, `alias_kill.rs`, `census_defs.rs`) use `may_overlap`, which differs from today's
wildcard `affects` only by excluding two different static keys at one position. Pinned behaviour that must
hold: `o.x = 1; o["x"] = 2; sink(o.x)` still kills (o27); a computed `o[k]++` still defeats `exact` (o28), as a
crossed possible overwrite rather than a kill, a named change of label. For JavaScript this narrows the earlier
wildcard decision for static keys and is listed for review as such.

For dynamic-only chains outside the lane's builder (no static key anywhere), main's equality kill and label
stay (`d[i] = ..` kills an earlier `d[j]` definition, and the dynamic-only `exact` of section 2 stays a pinned
known-wrong row): changing them is not this lane's work.

### 5.5 Initialisers (conditional slice)

A dictionary, map or object literal with static keys defines the keyed (or field) paths of its entries. An
entry is eligible only when its key, its whole value and nothing else of the literal share one line, the
literal's opening line holds no entry, and the key passes the key rule; no spread, computed key, method or
accessor. The entry definition takes its input from its value expression's span (the assignment hook sees
the target, so the initializer is located explicitly). Duplicate keys: the last entry wins. Whether the
container's own definition and its existing edges stay (they keep a base-to-sibling path on mixed layouts) is
a stage D item. The trace engine also carries taint from a use to every definition on the same line, so for a
literal written on one line the false positive survives in `taint-reaches` whatever the definitions are; that
propagation lives in a file other work owns (PR #841, lane 8), and this lane reports the case instead of
changing it.

### 5.6 Where the code goes

One new module builds paths from nodes (`src/ast/subscript_paths.rs`, with per-language key rules). A path
spelling's producers and every consumer that looks it up change in the same slice: the lvalue and rvalue
extractors (both `assignment_lvalue_paths_on_lines` and `assignment_lvalue_spans_on_lines`), the member
reference index and its oracle, Step 5b argument binding (three positions), the object-literal binder's read
chain (`object_literal_args.rs`, lane 3's file: one lookup, coordinated with lane 3), Step 5c and its
reference, capture identities and capture-region placement, alias resolution and member-only projection.
A consumer left on text is kept only with a test that proves its endpoint unaffected (the trace seeds compare
an argument's base; the JS `for` initializer replay check spells every bracket `[]` from syntax and stays a
conservative hazard). No slice loses an existing `exact` edge without listing it as a named change class.
Each slice that changes stored paths bumps the cache version and updates the pinned spellings it touches,
listed in the PR.

### 5.7 Out of model

What `CLAUDE.md` already puts out of model for `Exact` and for field paths in that language stays out for
keyed paths, each with a matrix row that pins the answer the contract gives (a wrong `exact` or a miss,
accepted and listed):

| Case | Contract it follows | Row |
|---|---|---|
| A callee mutates its argument (`mutate(d)` between write and read) | "a call that mutates the alias or another holder is out of model as for `Exact`" | O01 |
| Python `__setitem__`/`__getitem__`/`__delitem__` overrides, `dict` subclasses that redirect keys, ORM rows and data frames | Python: properties, descriptors and `__setattr__`/`__getattr__` are out of model | O02 |
| JS getters, setters, Proxies, array `length` truncation, inherited accessors; `__proto__` gets no static component (section 5.1) | JS field rules: getters, setters, Proxies and runtime mutation are out of model | O03 |
| Two distinct static paths naming one child (`d = {"a": v, "b": v}; d["a"]["x"] = t; sink(d["b"]["x"])`), a Go struct copy holding a shared map | field paths are compared as fixed storage (Go design §2.2) | O04 (a missed flow, never a wrong `exact`) |

Where a review fixture shows a wrong `exact` that the language's dot-field rules refuse (the JS certified
alias gives `alias_unstable`), keyed paths refuse too. Where the dot-field rules do not refuse (section 2),
keyed paths still need definite identity and the dot-field defect is filed.

## 6. The census (stage M)

On two Python corpora, Excalidraw, the TypeScript sources and two Go corpora, from Prism's own parser on the
current build (no product change). Counts are of feasible pairs. A write W and a read R of the same full base
chain (identifiers, dot members and every subscript before the one compared; JS dot reads count for
identifier keys, `o["x"] = t; sink(o.x)`) form a pair only when R follows W in execution order or both sit in
one loop, W and R are not in mutually exclusive arms (`if`/`elif`/`else`, `switch` cases, `try`/`except` arms),
no unconditional `return`/`raise`/`throw`/`break`/`continue` and no rebinding of the base or definite overwrite
of W's path lies between them on the straight-line part of the route, and both bind the same declaration where
the language records one.

| Id | Count | Decides |
|---|---|---|
| C1 | owners with a feasible W→R pair, by key relation (same literal key, different literal keys, either side dynamic) | the recall gain and the flood that approach A would cause |
| C2 | subscript reads and writes by index kind (string literal, identifier-like or not, integer, variable, other), role and language, over every grammar node kind that can hold a subscript (enumerated from the pinned grammars' node types; an unmatched kind is counted as `unknown`) | which keys become keyed components |
| C3 | owners where an element write is followed feasibly by a whole-container value read (section 5.3 consumer roles) | the container-read slice |
| C4 | eligible static-key literals (section 5.5 layout) whose name is later read feasibly by one of their keys | the initialiser slice |
| C5 | per concrete subscript write (by bytes, not spelling), whether its definition has an outgoing DataFlow edge in the same build; on the benchmark, cases whose recorded route ends at such a write | benchmark and real-diff effect |

Each count is validated in both directions on a labelled sample (sites the scan reports, and every subscript
node of sampled files, so an omitted grammar class shows up) and carries a golden fixture with exact per-owner
counts. Go or no-go (defaults): the keyed-path slices run when C1 finds at least 500 same-key feasible-pair
owners across the corpora, after scaling by the lower 95% bound of the sample's pair precision; the
container-read slice when C3 finds at least 300; the initialiser slice when C4 finds at least 300. Below the
first threshold the lane stops and reports.

## 7. Truth and gates

**Executed truth.** Each matrix row is a small program in Python, JavaScript or Go that marks one value and
records at its one sink call whether the marked value arrived; it is run to get the row's truth (`reaches` or
`does not`). Required behaviour is checked on paths: for `reaches`, a route exists from the marked definition
through the row's write W to the sink's use; for `does not`, no route made only of `exact` edges (DataFlow
edges, argument edges and the same-line use-to-definition transfers the trace engine applies) leads from the
marked definition to the sink's use.

| Gate | Threshold (default) |
|---|---|
| Matrix | every row agrees with its executed truth, or is a named accepted loss or a pinned known-wrong row |
| Differential | per slice, against the previous slice's binary (attribution: only its own classes of change, every removed or relabelled existing edge read and explained) and against the stage-M binary (cumulative impact, intervening main changes isolated); each gate names its comparison |
| No flood | edges added or relabelled between two occurrences whose full component sequences differ at a static position: zero. Endpoints are joined to occurrences by bytes, and keys compared by identity, never by line; JS `o["x"]` and `o.x` are one path. An endpoint that cannot be joined is counted as unclassifiable and sampled |
| Sample | of 200 added or relabelled edges, stratified by rule, language, label, fan-out and owner, at least 95% are real flows on a reader's check; unknowns and denominators reported |
| Engines | `taint-reaches` reaches every row whose truth is `reaches`; for a `does not` row no `exact`-only route exists as defined above (the engine follows killed edges, so a verdict alone cannot be required); review taint's verdicts are recorded for the same rows and are not required to change |
| Spellings | every changed fixture row and golden file is listed in the PR; paths of code without static-key subscripts are byte-identical, checked on the node inventory, not only the edge dump |

## 8. Follow-ups, not started by this lane

Dictionary and map methods as keyed reads and writes; integer indices; element paths across calls;
field-write to whole-object reads for Python and JavaScript (the dot-field half of the same gap);
field sensitivity in review taint; the main defects of section 2.

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Census thresholds | section 6 | none | at the census |
| 2 | JavaScript: identifier-like string keys unify with dot fields | yes | keeping them apart leaves `o["x"]` and `o.x` unconnected | stage D |
| 3 | Narrowing the JavaScript wildcard kill rule for static keys | yes, with the earlier decision's tests updated and listed | — | stage D |
| 4 | Label of flows that involve a dynamic key | not `exact` | — | stage D |
| 5 | Review taint | unchanged; its verdicts are only recorded | a separate lane | — |

## 10. Risks and accepted limits

- **This lane changes path identity**, which many proofs and pinned fixtures read. Slices are ordered so the
  first one is behaviour-preserving (the path builder, used only by a trace), and every later slice lists the
  spellings it changes.
- **Definite identity costs `exact` labels.** A container whose base is rebound or aliased gets non-`exact`
  keyed edges even where dot fields of the same object are `exact` today.
- **Dynamic keys dominate some code.** There the lane adds non-`exact` edges and no precision.
- **The verdict of review taint may not move**, because it does not use field paths. The benefit is in
  `taint-reaches`, slices, navigation and labels; the engines gate says so explicitly.
- **Recorded, not pursued:** a separate nav call-edge cache version bump (needed only for a nav contract
  change, review finding 18); re-blessing goldens wholesale (each changed assertion is named instead).

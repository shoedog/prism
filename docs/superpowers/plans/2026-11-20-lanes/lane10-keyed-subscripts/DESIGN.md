# Lane 10: subscripts in the data-flow graph — design

Status: the census design (section 6) is final. The path representation and the flow rules of section 5 are
tentative until the census, the executed test matrix and enumeration (PLAN stages M, T, E). The plan files
a tracking issue. Research record: `~/prism-evidence/lanes-2026-11-20/research/lane10-keyed-subscripts.md`.

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
| A subscript write records one definition, `base.[]`, over the whole target; the key and anything after it are dropped. `del d["a"]` and JavaScript `delete d["a"]` record nothing. | `src/ast/lvalues.rs` |
| A plain subscript read records a use of the base variable (and of an index variable), not of a path. Only a read where a field access wraps a subscript (`d["a"].c`) records a use with the path `d.[]`. | `src/ast/rvalue_identifiers.rs:80-96` |
| Field sensitivity for dot fields comes from one index keyed by the path text of field-access nodes: a definition reaches only references whose path equals its own. Subscript nodes are not in that index. So the definition `d.[]` is an orphan: nothing but `d["a"].c`-shaped reads can reach it. | `src/ast/member_reference_index.rs:23-120` |
| Probes with `taint-reaches`, three languages. False negatives: `d["a"] = t; sink(d["a"])`; `…; sink(d)`; `…; x = d["a"]; sink(x)`; through an alias; augmented assignment; JavaScript `o["x"] = t; sink(o.x)` and `o.x = t; sink(o["x"])`. False positives: a literal `d = {"a": t, "b": "safe"}; sink(d["b"])` (the whole container is tainted); `d["u"].n = t; sink(d["u"].m)`. Also wrong: a write to another key is labelled as killing (`d["a"].c = t; d["b"].c = "x"; sink(d["a"].c)` is `killed`). Dot fields behave correctly. | research section 5 |
| The same gap exists for dot fields in one direction: `o.a = t; sink(o)` is not reached in Python and JavaScript. Go has a separate pass that connects field writes to later whole-object reads. | probes; `CLAUDE.md` ("Go eager aggregate reads") |
| Review taint is a different path model: it is not field-sensitive even for dot fields (`d.name = tainted; d.other = "safe"; sink(d.other)` is reported), and it matches request reads such as `req.body["x"]` by text. A subscript on the left of an assignment gives it no target in Python and Go. | research section 5 |
| Killed edges are still followed by taint; a correct kill changes a label, not a verdict. | research |
| Readers of path spellings: reaching definitions and kills (exact equality, except JavaScript, where `[]` is a wildcard in the kill relation in both directions by an earlier design decision), the JavaScript write census (any index is `[]`, pinned by tests), the object-literal binder (which already has a static-key policy), argument binding, navigation output and five MCP golden files, 116 census fixture rows and 105 alias-matrix rows. The path is stored in the cached graph. | research sections 3 and 8 |
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

## 5. Architecture — *representation and flow rules tentative*

### 5.1 Path components

A path component is one of: a field name (as today); the dynamic-index component `[]` (as today); a keyed
component, written `["k"]` in the fields list, for a subscript whose index is a string literal without
interpolation or escapes.

| Language | `x["k"]` | `x.k` | Same path? |
|---|---|---|---|
| JavaScript, TypeScript | field `k` when `k` is an ASCII identifier-like string (the object-literal binder's existing static-key policy); otherwise keyed `["k"]` | field `k` | yes for identifier-like keys |
| Python | keyed `["k"]` | field `k` | no: items and attributes are different namespaces |
| Go | keyed `["k"]` for a map or slice index that is a string literal | field `k` | no |

Integer literals, variables, constants and expressions give `[]`. A chain that holds at least one static
key is built from syntax nodes by one shared function, not from text, and keeps every component
(`d["u"]["n"]` is `d.["u"].["n"]`; `d["a"].c` is `d.["a"].c`). A chain with no static key keeps today's
text-derived path exactly (`d[k].c` stays `d.[]`), so code without static-key subscripts does not change.
The JavaScript identifier-like test is the object-literal binder's rule; that function is private to a file
another lane owns, so the rule is repeated in the new module and unifying the two is a follow-up.

### 5.2 Definitions and uses

- A subscript write records a definition of its full path.
- A subscript read records a use of its full path, in addition to the base use it records today.
- `del d["k"]` and `delete o["k"]` record a kill-only definition of the path where the language's census
  already supports kill-only definitions (JavaScript), and are left as today elsewhere.

### 5.3 Flow

Within a function, on top of the existing equality rule (a definition reaches uses of the same path):

| From | Reaches | Label |
|---|---|---|
| a definition of a keyed or field path `p` | uses of `p`, and of paths that extend `p` | as today |
| a definition whose path contains `[]` where the use has a keyed component, and the reverse | the use (a dynamic key may be that key) | never `exact`: `NameOnly` with the existing alias-unstable doubt, or a new doubt kind fixed at stage D |
| a definition of an element path | a later read of the whole container (`sink(d)`) | only if the census justifies the slice (section 6); the Go pass that does this for fields is the model |

### 5.4 Kills

A keyed write kills earlier definitions of the same keyed path and of paths below it, and nothing else. A
dynamic-key write cannot be known to overwrite a keyed definition, so it does not kill one; an edge from a
keyed definition across such a write is not `exact`. Between two dynamic-key writes, today's code outside
JavaScript kills by path equality (`d[i] = ..` kills an earlier `d[j]` definition); whether that stays, as
goal 4 reads, or becomes a may-kill is fixed at stage D. For JavaScript, where `[]` is today a wildcard in the kill
relation in both directions, an identifier-like static key becomes an ordinary field write and takes the
field rules; the wildcard rule keeps applying to dynamic keys only. That narrows an earlier design decision
and is listed for review as such.

### 5.5 Initialisers

A dictionary, map or object literal with static keys defines the keyed (or field) paths of its entries
instead of tainting the whole container. This is a separate slice with its own census, because it changes
existing edges. The trace engine also carries taint from a use to every definition on the same line, so for
a literal written on one line the false positive survives in `taint-reaches` whatever the definitions are;
that propagation lives in a file other work owns (PR #841, lane 8), and this lane reports the case instead
of changing it.

### 5.6 Where the code goes

One new module builds paths from nodes (`src/ast/subscript_paths.rs`, with per-language key rules); the
places that build paths from text for subscripts call it (the research lists about eight: lvalue and rvalue
extraction, the member reference index, argument binding, capture, alias maps, member projection, the
JavaScript census). Each slice that changes stored paths bumps the cache version and updates the pinned
spellings it touches, listed in the PR.

## 6. The census (stage M)

On two Python corpora, Excalidraw, the TypeScript sources and two Go corpora, from the current build's
nodes (no product change):

| Id | Count | Decides |
|---|---|---|
| C1 | functions where a subscript write is followed, on some path, by a subscript read of the same base: by key relation (same literal key, different literal keys, either side dynamic) | the recall gain and the flood that approach A would cause |
| C2 | subscript reads and writes by index kind (string literal, identifier-like or not, integer, variable, other) and by language | which keys become keyed components |
| C3 | functions where an element write is followed by a whole-container read | the container-read slice |
| C4 | container literals with static keys whose elements are later read by key | the initialiser slice |
| C5 | on the benchmark and on `taint-reaches` runs over the 120-diff population: flows that stop at a subscript write today | benchmark and real-diff effect |

Go or no-go (defaults): the keyed-path slices run when C1 finds at least 500 same-key write-then-read
functions across the corpora; the container-read slice when C3 finds at least 300; the initialiser slice
when C4 finds at least 300. Below the first threshold the lane stops and reports.

## 7. Truth and gates

**Executed truth.** Each matrix row is a small program in Python, JavaScript or Go that marks one value and
records at a sink call whether the marked value arrived; it is run to get the row's truth (`reaches` or
`does not`). Required behaviour: an edge path exists when the truth is `reaches`; for `does not`, either no
path or a path whose label is not `exact`.

| Gate | Threshold (default) |
|---|---|
| Matrix | every row agrees with its executed truth, or is a named accepted loss |
| Differential | per slice, only its own classes of change; every removed or relabelled existing edge is read and explained |
| No flood | edges added between different literal keys of one base: zero |
| Sample | of 200 added edges sampled from the corpora, at least 95% are real flows on a reader's check |
| Engines | `taint-reaches` reaches every row whose truth is `reaches`; for a `does not` row no path made only of `exact` edges exists (the engine follows killed edges, so a verdict alone cannot be required); review taint's verdicts are recorded for the same rows and are not required to change |
| Spellings | every changed fixture row and golden file is listed in the PR; paths of code without static-key subscripts are byte-identical. Keyed definitions and the reference index that connects them change in the same slice, because keyed definitions alone would lose edges that exist today |

## 8. Follow-ups, not started by this lane

Dictionary and map methods as keyed reads and writes; integer indices; element paths across calls;
field-write to whole-object reads for Python and JavaScript (the dot-field half of the same gap);
field sensitivity in review taint.

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
  first one is behaviour-preserving (the path builder, used only by the census), and every later slice
  lists the spellings it changes.
- **Overloaded subscripts** (`__getitem__` on data frames, arrays, ORM rows) are treated like dictionaries.
  Flows are may-flows; the sample gate measures how often that is wrong.
- **Dynamic keys dominate some code.** There the lane adds non-exact edges and no precision.
- **The verdict of review taint may not move**, because it does not use field paths. The benefit is in
  `taint-reaches`, slices, navigation and labels; the engines gate says so explicitly.

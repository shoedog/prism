# Lane 4: prototype-pollution tracing — design

Status: the experiment design (section 6) is final. The detection rules of section 5 are *tentative* until
the experiment, the executable test matrix and enumeration close (PLAN stages M, T, E). Tracking issue:
#603 (parent #604). Research record: `~/prism-evidence/lanes-2026-11-20/research/lane4-prototype-pollution.md`.

## 1. Intent

Report the JavaScript and TypeScript defect in which attacker-controlled property names reach a write that
can land on `Object.prototype`: recursive merges and path setters such as

```js
function set(obj, path, value) {
  const keys = path.split('.'); let o = obj;
  for (let i = 0; i < keys.length - 1; i++) { o = o[keys[i]] = o[keys[i]] || {}; }
  o[keys[keys.length - 1]] = value;            // path "__proto__.polluted" pollutes every object
}
```

The lane is funded in two steps. First a short experiment on the existing benchmark shows how many known
cases the planned sink would report and how often it would fire on ordinary code. Implementation starts only
if that number is meaningful.

## 2. What exists today (verified on `origin/main` at 1179055f)

The issue's premise is out of date, which changes what has to be built.

| Fact | Reference |
|---|---|
| `target[key] = value` **is** recorded as a definition, of path `target.[]`, and `key` is a use on that line. This has been so since April. | `src/ast/lvalues.rs:628-634`; probe |
| The key is not part of the path: `o[k]` and `o["x"]` are both `o.[]`; `o[k].y` is recorded as `o.[]`. No data-flow edge starts or ends at a `[]` path (0 of 73 edges in the probe): reads of `o[k]` collapse to `o`. | `src/access_path.rs:152-163`; probe |
| Value and key taint already reach the write line. `taint-reaches` from an Express handler's `req.body` reports the `key` use at `target[key] = …` as Reached in the recursive merge, and the path array at the path setter's final write. | probe 7.3 |
| `for (const key in source)` carries taint from `source` to `key` (same-line propagation). `Object.keys(source).forEach(key => …)` does not: callback parameters are not bound (#434). | probe; `src/cpg/query.rs:731-767` |
| There is no prototype-pollution sink and no guard recognition. A merge guarded by `if (key === '__proto__' \|\| key === 'constructor') continue;` is still Reached. Guard machinery accepts only `return` as a rejecting branch. | `crates/prism-review/src/algorithms/taint/cleansing.rs:271-273` |
| Review sinks are matched per identifier on a tainted line; the only assignment sinks (`innerHTML`, `location`) are decided by a property name. No sink depends on which operand (key, value, receiver) is tainted. A flow path's end does carry the variable's access path and byte span. | `crates/prism-review/src/algorithms/taint/run.rs:213-292`, `taint_sinks/js.rs:449-493`, `src/data_flow/types.rs:7` |
| A finding has no field for a rule tag or a CWE. The sink classifier's tag and CWE are read only by its unit tests; `taint_sink` findings are emitted per line with a fixed description, and SARIF rules are keyed by category. Severity comes from a per-line strength grade that can be Weak when the line also holds a weak SQL call. | `src/slice.rs:24`, `taint/run.rs:456-590`, `taint_sinks/strength.rs:38`, `crates/prism-review/src/output/sarif_rules.rs` |
| The benchmark harness measures reach, not findings: `nav taint-reaches --source <API formal> --sink <ground-truth write line>`, crediting any reached occurrence on that line. It has no guarded or fixed-version negatives. | `eval/secbench/run.py:290-346` |
| Benchmark numbers (baseline of 2026-10-05, an older build): 192 prototype-pollution entries, 190 acquired, ground truth derivable for 95; of those 41 traced, 26 reach the function only, 8 partial, 3 not reached, 17 errors. | `docs/superpowers/plans/2026-10-04-secbench-ground-truth/R1.md` |
| Shape of the cases: 145 of 148 recorded sinks are subscript assignments; none is `Object.assign` or a spread. By exploit payload: path setters 52 (31 traced), object merges 46 (5 traced), flatten/unflatten 7 (4 traced). Merge cases fail before the write: rest parameters and `arguments` (#595), callbacks (#434), `dist` directories the loader skips. | research section 3.5 |
| Draft PR #841 (cross-file taint context) edits `taint/run.rs`, `taint.rs`, `taint/seeds.rs` and `src/cpg/{trace,query,cfg_queries}.rs`. It does not touch `taint_sinks/`, `src/ast/` or `src/sanitizers/`. | PR #841 |

Consequences:

- "Dynamic-key definitions" need not be built for detection. The write is already a definition and the key
  already arrives. What is missing is knowing **which writes can pollute**, **which operand is the key**,
  **which guards make the write safe**, and a **sink** that uses those facts.
- The largest loss on merge-shaped cases is upstream of this lane (parameter binding and callbacks). The lane
  reports those cases as potential, and does not fix them.
- Key identity in paths (`o["x"]` versus `o[k]`) and edges through `[]` paths are data-flow precision work
  with their own measurement. They are follow-ups (section 9), not prerequisites.

## 3. Goals, non-goals, anti-goals

Goals:

1. An experiment that answers, on the current build: how many benchmark cases would the planned sink
   report; and how many writes would it single out in ordinary code.
2. A fact, computed from syntax, for each dynamic-key write that can reach a prototype object: its class,
   its key operands, and the state of any guard.
3. A review-taint sink that fires when taint reaches a **key operand** of such a write, with CWE-1321, and
   stays silent when a guard is proven sufficient.
4. Truth labels for the rules obtained by execution: each matrix row is a small function that is run under
   Node against the standard payloads to see whether a fresh object's prototype was polluted.
5. Path-walking writes and recursive merges delivered as separate slices.

Non-goals (this lane): `Object.assign`, spread and library merge calls as sinks; gadget tracing (where the
polluted property is later read); key flow through callback parameters (#434) and rest or `arguments`
formals (#595); loader admission of `dist` directories; computed-key path identity in the data-flow graph.

Anti-goals: a sink on every `obj[key] = value` with a tainted value; treating any mention of `__proto__` as
a sufficient guard; changing the data-flow producer or the cache format for this lane.

## 4. Approaches considered

**A. Data-flow first** (the issue's sketch): give dynamic-key writes richer definitions, add key provenance
to paths, then a sink on the definition. The definition already exists and key taint already reaches the
line, so the producer work would not move detection, while it would touch reaching definitions, the write
census and the cache. Rejected for this lane.

**B. Sink on any tainted dynamic-key write.** Small, and matches the benchmark's "reach" credit. It fires on
`cache[id] = value`, on matrix writes and on one-level copies, which cannot pollute a prototype. Rejected.

**C. Syntactic facts for pollution-capable writes, plus an operand-aware sink** (chosen). A per-function
analysis classifies writes that can step onto a prototype object (section 5.1), names their key operands,
and evaluates guards. The sink fires only when the tainted occurrence on the line is a key operand of such
a write. Facts live in the core crate so that a CLI listing can expose them to the benchmark harness; the
sink lives in the review crate.

**D. Function summaries used as derived sinks** (mark `set` and `merge` as sinks at their call sites, by
argument position). Attractive for reporting at the caller, and it would work where taint does not enter
the helper. It needs a summary mechanism the taint code does not have (extra sinks are not reachable from
the CLI), and the helper-internal sink of C already reports through interprocedural reach. Kept as a
possible later presentation layer over the same facts.

## 5. Architecture — *rules tentative*

```
ParsedFile ──► proto_writes facts (core, per file, on demand)
                 │                                   │
                 ├─► prism nav proto-writes (CLI)    └─► review taint: js-proto-write sink (key operand tainted,
                 │      joined by the harness               no sufficient guard) ─► taint_sink finding, CWE-1321
                 │      with taint-reaches verdicts
```

### 5.1 Classes of pollution-capable write

A dynamic-key write is an assignment (plain, compound or logical) whose target is a subscript with a
non-literal index. It is pollution-capable when one of these holds:

| Class | Shape | Why it can pollute |
|---|---|---|
| `nested_keys` | the target has two or more non-literal index levels (`a[k1][k2] = v`) | `k1 = "__proto__"` selects the prototype, `k2` names the polluted property |
| `path_walk` | the target's base is a cursor: (a) a variable that the same function reassigns from a dynamic read (`o = o[k]`, `o = o[k] = o[k] \|\| {}`, `o = o[k] ?? …`), in a loop, an array-method callback, or recursion; or (b) the result of a `reduce` call whose callback returns a dynamic read of, or a dynamic-key assignment through, its accumulator | an earlier step can move the cursor onto the prototype |
| `recursive_merge` | the target's base is a parameter (or a single alias of one) of a function that calls itself, directly or through one same-file helper, and the recursive call passes a dynamic read of that base (`target[key]`) at the base's parameter position; the other arguments are not examined | the recursive call makes the callee's base the prototype |

Not in any class, and therefore never a finding: a one-level copy (`for (k in s) t[k] = s[k]`) with no
walk and no recursion; a write whose every index is a numeric literal or a numeric loop counter; a write to
a `Map` or `Set` through methods. A *numeric loop counter* is a variable declared in the initialiser of a
`for (init; test; update)` statement and changed nowhere except by `++`, `--`, or `+=`/`-=` of a numeric
literal in that statement's update.

Each fact names its **key operands**: the identifiers and member chains inside the index expressions of the
write, excluding numeric loop counters. A walk step that is itself a dynamic-key write
(`o = o[k] = o[k] || {}`) is a fact of its own with its own operands.

### 5.2 Guards

Guard state is `none`, `partial` or `sufficient`. *Which syntactic forms are sufficient is decided by the
executable matrix, not by this table*; the table is the starting hypothesis.

| Form | Hypothesis |
|---|---|
| a comparison of the key (or of each path segment) with `"__proto__"` and with `"constructor"` or `"prototype"`, whose true branch leaves (`continue`, `return`, `throw`, `break`) or whose false branch contains the write; the same through `includes` on a literal array or a `switch` | sufficient |
| the same for `"__proto__"` only | partial: `constructor.prototype` still pollutes |
| `Object.prototype.hasOwnProperty.call(o, k)` (or `Object.hasOwn`) tested before each walk step so that a missing key is never followed | to be decided by execution |
| the root created by `Object.create(null)` | not sufficient for a path walk (an intermediate `{}` has a prototype) |
| a key validated by a regular expression or an allow-list | none, unless a row proves a specific form |

Placement, also a hypothesis for the matrix: a guard counts when it is in the same function as the write,
tests an expression that is the write's key expression or a `const` initialised once from it, and lies on
every path from where that key is bound (the loop iteration's start, the callback's entry, the function's
entry) to the write.

A `sufficient` guard means no finding. A `partial` guard keeps the finding and adds a note to its
description. This follows the taint sinks' existing policy: only proven non-sinks are suppressed.

### 5.3 The sink

In review taint, for JavaScript, TypeScript and TSX: when a tainted flow path's end occurrence lies inside
a key operand of a pollution-capable write (matched by byte span; the path's end carries one), and that
write's guard is not `sufficient`, the line becomes a sink line.

- **Where the check sits.** Once per flow-path edge in the sink-matching loop of `taint/run.rs`, outside the
  per-identifier loop, and before the skip that loop applies to framework-source lines (a key such as
  `req.body[k]` sits on such a line). It records the line in a side set that the emission block reads.
- **What is emitted.** A finding of the existing category `taint_sink`, severity `warning`, with the
  description "tainted key from {source} reaches a prototype-capable write at line N (CWE-1321,
  js-proto-write)" and " (partial key guard)" appended when the guard is partial. The tag and the CWE travel
  in the description because a finding has no field for them (decision 9 in section 10 is the alternative).
- **Strength.** A line with such a hit is Strong whatever else is on the line; the per-line grade must not
  turn it into a `suggestion`.
- A tainted value or receiver alone does not make the line a sink.

PR #841 edits the same loop, so that slice is rebased over #841 or lands after it. Review taint keeps
findings in the diff's file today (#602, PR #841); a helper in another file is reported once that lands, and
`taint-reaches` plus the listing cover it meanwhile.

### 5.4 The listing

`prism nav proto-writes --repo <dir> [--format json|text]` (CLI only, like `dfg-stats`; no MCP tool) prints
every fact: file, line, function, class, key operand names, guard state. The benchmark harness joins it
with its `taint-reaches` verdicts to compute a **finding-equivalent**: a case counts when the ground-truth
write line holds a fact that is not `sufficient`ly guarded and a key operand occurrence is Reached. This
measures the sink's benchmark yield without synthesising diffs for review taint.

### 5.5 Where the code goes

```
src/ast/js_ts.rs                         module lines (the directory's module file)
src/ast/js_ts/proto_writes.rs            facts and the public entry point (ParsedFile::js_ts_proto_writes)
src/ast/js_ts/proto_writes/classes.rs    nested_keys, path_walk, recursive_merge
src/ast/js_ts/proto_writes/guards.rs     guard state
src/navigation/proto_writes.rs           the listing (queries.rs is over the size ceiling: no logic there)
crates/prism-review/src/cli.rs, main.rs  NavQuery::ProtoWrites and its arm
crates/prism-review/src/algorithms/taint_sinks/proto_write.rs   the operand-aware sink check
crates/prism-review/src/algorithms/taint/run.rs                 the per-edge check, the side set, the emission changes
tests/fixtures/proto_writes/             matrix fixtures (one function per file) and the Node truth runner
```

No cache version change: facts are computed from parsed files on demand and nothing new is stored.

## 6. The experiment (stage M)

Four measurements on the current build, then a memo.

| Id | Question | Method |
|---|---|---|
| E1 | How many prototype-pollution cases trace today? | re-run the harness for that class on a main build (the harness gains a class filter and recorded pin overrides; eval-only change) |
| E2 | In traced cases, is the reached occurrence the key? | from each row's `trace_detail.parameter_results` (a verdict per occurrence on the sink line) and a parse of the sink file: is a Reached occurrence's byte span inside an index expression of the write at the ground-truth line and column |
| E3 | How many ground-truth writes fall in a class of 5.1, and how are they guarded? | a prototype classifier script over the ground-truth sink functions (syntax only), judged by 5.1 as written; a write the script cannot place is counted as no class |
| E4 | How often do the classes occur in ordinary code? | the same script on Excalidraw, the TypeScript compiler sources, `express`, `directus-directus/api` and `calcom-cal.com/apps/api` from `bench-repos`; a reader labels up to 40 hits, sampled after sorting by file and line: can pollute if the key is attacker-controlled / cannot / guarded |

**Conversions** C = cases that are traced (E1) with the key reached (E2) and a class assigned (E3).
**Potential** P = cases that reach the function only or partially and have a class: what upstream fixes
(#595, #434, loader admission) would add. P is reported and not gated.

Gate, pre-stated:

| Result | Action |
|---|---|
| C ≥ 25 | continue to stages T and E without waiting |
| 15 ≤ C < 25 | post the memo and ask the owner |
| C < 15 | stop the lane; post the memo on #603 |

E4 does not gate. If fewer than 70% of the sampled class hits are judged able to pollute, the misjudged
shapes become matrix rows and the class definitions are tightened in stage D before any slice.

The memo (`EXPERIMENT.md` in this directory) holds E1 to E4, C and P by shape (path setter, merge, flatten,
other), the list of converted case ids, and the classes' hit counts per corpus.

## 7. Truth by execution (stages T and E)

Every matrix row is a file exporting one function, plus a call description (which argument is attacker
data). A runner starts a fresh Node process per row and per payload, calls the function, and checks whether
`({}).polluted` is defined afterwards. Payload families: an object parsed from
`{"__proto__":{"polluted":1}}`; from `{"constructor":{"prototype":{"polluted":1}}}`; the paths
`"__proto__.polluted"` and `"constructor.prototype.polluted"` as strings and as arrays. The row's truth is
`pollutes` when any payload does, else `safe`. Required behaviour: a fact that is not `sufficient` for a
`pollutes` row; no fact, or a `sufficient` guard, for a `safe` row.

Only fixtures written for this lane are executed. Benchmark packages are never executed.

## 8. Gates

| Gate | Threshold (default) |
|---|---|
| Experiment | section 6 |
| Matrix | every `pollutes` row has a non-sufficient fact; every `safe` row has none or a sufficient guard; exceptions are named accepted losses |
| Benchmark yield | finding-equivalents (5.4) ≥ 0.9 × C on the same build |
| Ordinary code | on the E4 corpora, every fact outside test directories is listed in the slice's PR; the E4 sample re-labelled against the final rules shows at least 80% able to pollute |
| Sink | the sink rows: fires on key taint, silent on value-only and receiver-only taint, silent under a sufficient guard, `warning` severity even beside a weak call on the same line |
| No regression | review-taint output on the existing taint fixtures is unchanged except for new `js-proto-write` findings; no change to any data-flow statistic |

## 9. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Key enumeration through callbacks (`Object.keys(x).forEach(k => …)`, `_.each`) | P cases whose first break is the callback; belongs to #434 |
| Rest and `arguments` formals | P cases with those first breaks; #595 |
| Loader admission of `dist` and build directories | the 10 cases skipped today |
| Call-site (summary) reporting for helpers | reader feedback that the helper-internal finding is hard to act on |
| `Object.assign`, spread, library merge calls | a benchmark or corpus count; none of the 148 recorded sinks has this form |
| String-literal key identity and `[]`-path edges in the data-flow graph | a consumer that needs them (lane 5's field reads is a candidate) |

## 10. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Experiment thresholds | 25 / 15 as in section 6 | none | at the memo |
| 2 | One-level copies | not reported | reporting them as a weak finding is one class and one strength rule | stage D |
| 3 | A sufficient guard | no finding | a weak (`suggestion`) finding instead is a strength rule | stage D |
| 4 | A partial guard | finding stays, with a note | — | stage D |
| 5 | Where facts live | core crate, with a CLI-only listing | moving them into the review crate removes the listing and the harness join | before S1 |
| 6 | Executing lane-written fixtures under Node for truth labels | yes: child process per row, 5 s limit, no network | without it, labels come from reading | before stage T |
| 7 | Fixed versions of benchmark packages as negatives | downloaded with `npm pack` (archive only, never installed or run), at most 20, if the network allows | skipping it leaves guards tested only on lane fixtures | stage M |
| 8 | The sink slice and PR #841 | proceed and rebase; if #841 is still open when the sink slice is ready, do the merge-class slice first | — | at S2 |
| 9 | How a finding says it is this rule | category `taint_sink`, with the CWE and the tag in the description | a category of its own (for example `prototype_pollution`) needs a SARIF rule entry and a targets mapping arm, and changes what consumers of `taint_sink` see | before S2 |

## 11. Risks and accepted limits

- **The classes are syntactic.** A walk hidden behind a helper call, a cursor stored in an object field, or
  recursion through three functions is missed. Misses are counted against the benchmark, never guessed at.
- **Guard sufficiency is subtle.** That is why each guard form needs an executed row before it suppresses
  anything; unknown guards never suppress.
- **Benchmark ground truth is noisy** (one "vulnerable" coordinate sits in already-guarded code). A case the
  final rules call guarded is listed for a human check, not counted against the lane.
- **Merge-shaped cases mostly fail upstream.** The lane's benchmark yield is therefore dominated by path
  setters until #595 and #434 land.
- **Security-benchmark content and model filters.** Briefs for non-Claude seats use neutral static-analysis
  wording; product implementers work from the lane's fixtures and never open the benchmark tree.

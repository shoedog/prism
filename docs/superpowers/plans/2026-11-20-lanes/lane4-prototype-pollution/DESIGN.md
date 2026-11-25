# Lane 4: prototype-pollution tracing — design

Status: the experiment design (section 6) is final. The detection rules of section 5 are *tentative* until
the experiment, the executable test matrix and enumeration close (PLAN stages M, T, E). Tracking issue:
#603 (parent #604). Research record: `~/prism-evidence/lanes-2026-11-20/research/lane4-prototype-pollution.md`.
Revised after design review round 1 (2026-11-21).

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

## 2. What exists today (verified on `origin/main` at 1179055f; key placement and receiver propagation on a0f2c427)

The issue's premise is out of date, which changes what has to be built.

| Fact | Reference |
|---|---|
| `target[key] = value` **is** recorded as a definition, of path `target.[]`, and `key` is a use on that line. This has been so since April. | `src/ast/lvalues.rs:628-634`; probe |
| The key is not part of the path: `o[k]` and `o["x"]` are both `o.[]`; `o[k].y` is recorded as `o.[]`. No data-flow edge starts or ends at a `[]` path (0 of 73 edges in the probe): reads of `o[k]` collapse to `o`. | `src/access_path.rs:152-163`; probe |
| The key's Use on the write line rarely has the key's own byte span. In a named owner, an identifier inside the index of a plain-statement write gets a zero-width Use at the line start (`o[k] = v`, `a[k1][k2] = v`, `o[keys[keys.length - 1]] = v`, `if (!t[key]) t[key] = {}`), or shares the Use of the same name's right-hand occurrence (`target[key] = source[key]`; `t[key] = f(t[key] \|\| {}, v)`, the occurrence inside the call argument). Exact spans exist for walk steps (`o = o[k] = o[k] \|\| {}`), compound and logical writes (`o[k] ??= {}`, `o[k] += v`), member-chain keys (`o[opts.key][name]`: `opts.key` exact, `name` zero width) and anonymous owners. In the baseline's 41 traced cases the key's Reached occurrence is zero width in 32, a right-hand occurrence in 3 and inside the index span in 1. | probe (`~/prism-evidence/lanes-2026-11-20/reviews/lane4-fable-r1-probes/spans-a.js`) |
| Value and key taint already reach the write line. `taint-reaches` from an Express handler's `req.body` reports the `key` use on the line of `target[key] = …` as Reached in the recursive merge, and the path array at the path setter's final write. | probe 7.3 |
| `for (const key in source)` carries taint from `source` to `key` (same-line propagation). `Object.keys(source).forEach(key => …)` does not: callback parameters are not bound (#434). | probe; `src/cpg/query.rs:731-767` |
| A tainted receiver reaches the key of a path walk that aliases its receiver parameter. For a function whose formals share one line and which begins `let o = obj` and walks `o = o[k]`, a tainted `obj` taints `path` and `value`: the alias mints twin definitions of `obj` at `let o = obj` and at `o = o[k]`, they have edges to a Use of `obj` on the signature line, and same-line propagation (a Use reaches every Def on its line in its function) taints the sibling formals. Without the alias (`obj[k1][k2] = v`, `const u = obj[k1]; u[k2] = v`), or with one formal per line, it does not. | probe (same directory: `receiver-decoy-e.js`, `receiver-controls-f.js`); `src/cpg/query.rs` |
| There is no prototype-pollution sink and no guard recognition. A merge guarded by `if (key === '__proto__' \|\| key === 'constructor') continue;` is still Reached. Guard machinery accepts only `return` as a rejecting branch. | `crates/prism-review/src/algorithms/taint/cleansing.rs:271-273` |
| Review sinks are matched per identifier on a tainted line; the only assignment sinks (`innerHTML`, `location`) are decided by a property name. No sink depends on which operand (key, value, receiver) is tainted. A flow path's end is a `VarLocation`: its kind (Def or Use), function, function start line, line, access path and byte span. | `crates/prism-review/src/algorithms/taint/run.rs:213-292`, `taint_sinks/js.rs:449-493`, `src/data_flow/types.rs:7` |
| A key that is itself a framework source expression (`store[req.params.kind][id] = 1`) is never an edge end: a source root starts edges and does not end one. `taint/run.rs` already computes the framework source-access byte ranges of each source line. | `taint/run.rs:115-118`, `taint/js_ts/source_access.rs` |
| A finding has no field for a rule tag or a CWE (lane 7 of this plan set adds one, a `rules` list; see "What is emitted" in 5.3 for what changes if it merges first). The sink classifier's tag and CWE are read only by its unit tests; `taint_sink` findings are emitted per line with a fixed description, and SARIF rules are keyed by category. Severity comes from a per-line strength grade that can be Weak when the line also holds a weak SQL call. | `src/slice.rs:24`, `taint/run.rs:456-590`, `taint_sinks/strength.rs:38`, `crates/prism-review/src/output/sarif_rules.rs` |
| The benchmark harness measures reach, not findings: `nav taint-reaches --source <API formal> --sink <ground-truth write line>`, crediting a reached occurrence on that line. Per case it saves the raw `taint-reaches` output (`raw/<class>/<entry>/witness.json.gz`: every Use on the ground-truth line, with a verdict per source root) and, as a comparison channel, review taint on a synthetic diff of the source parameter lines (`raw/<class>/<entry>/taint.json.gz`, `--algorithm taint --taint-return-flow --taint-source <formal>`). The row's `trace_detail.parameter_results` keeps only occurrences that pass `sink_value`: an exact start byte, or a zero-width Use whose name occurs once on the line; a key named two or more times on the line (`ret[keyPath[keyPath.length - 1]] = value`) is absent from it. It has no guarded or fixed-version negatives. | `eval/secbench/run.py:64-72`, `:134-153`, `:290-330` |
| Benchmark numbers (baseline of 2026-10-05, an older build): 192 prototype-pollution entries, 190 acquired, ground truth derivable for 95; of those 41 traced, 26 reach the function only, 8 partial, 3 not reached, 17 errors. All 41 traced cases have source and sink in one file. | `docs/superpowers/plans/2026-10-04-secbench-ground-truth/R1.md` |
| Shape of the cases: 145 of 148 recorded sinks are subscript assignments; none is `Object.assign` or a spread. By exploit payload: path setters 52 (31 traced), object merges 46 (5 traced), flatten/unflatten 7 (4 traced). Merge cases fail before the write: rest parameters and `arguments` (#595), callbacks (#434), `dist` directories the loader skips. | research section 3.5 |
| Draft PR #841 (cross-file taint context) edits `taint/run.rs`, `taint.rs`, `taint/seeds.rs` and `src/cpg/{trace,query,cfg_queries}.rs`. It does not touch `taint_sinks/`, `src/ast/` or `src/sanitizers/`. | PR #841 |

Consequences:

- "Dynamic-key definitions" need not be built for detection. The write is already a definition and the key
  already arrives on its line. What is missing is knowing **which writes can pollute**, **which operand
  selects the prototype**, **which guards make the write safe**, and a **sink** that decides by name which
  operand is tainted: the key arrives as a line-level Use of its name, not as an occurrence inside the index.
- The sink rests on two producer behaviours that nothing pins today: the Use an index identifier gets on the
  write line (pinned by a core test in stage T), and the receiver propagation above, which makes one sink
  row a known false positive (5.3, decision 10).
- The largest loss on merge-shaped cases is upstream of this lane (parameter binding and callbacks). The lane
  reports those cases as potential, and does not fix them.
- Key identity in paths (`o["x"]` versus `o[k]`) and edges through `[]` paths are data-flow precision work
  with their own measurement. They are follow-ups (section 9), not prerequisites.

## 3. Goals, non-goals, anti-goals

Goals:

1. An experiment that answers, on the current build: how many benchmark cases would the planned sink
   report; and how many writes would it single out in ordinary code.
2. A fact, computed from syntax, for each dynamic-key write that can reach a prototype object: its class,
   its selector and property key operands (5.1), and the state of any guard.
3. A review-taint sink that fires when taint reaches a **selector key operand** of such a write, with
   CWE-1321, and stays silent when a guard is proven sufficient.
4. Truth labels for the rules obtained by execution: each matrix row is a small function that is run under
   Node, with payloads expanded from the row's own template, on several root kinds and in both module modes,
   against a positive control, to see whether a shared prototype object changed (section 7).
5. Path-walking writes and recursive merges delivered as separate slices.

Non-goals (this lane): `Object.assign`, spread and library merge calls as sinks; gadget tracing (where the
polluted property is later read); key flow through callback parameters (#434) and rest or `arguments`
formals (#595); loader admission of `dist` directories; computed-key path identity in the data-flow graph;
the receiver propagation of section 2 (decision 10).

Anti-goals: a sink on every `obj[key] = value` with a tainted value; a sink on taint of the property-naming
key alone (`sessions[room][userId] = socket`); treating any mention of `__proto__` as a sufficient guard;
changing the data-flow producer or the cache format for this lane.

## 4. Approaches considered

**A. Data-flow first** (the issue's sketch): give dynamic-key writes richer definitions, add key provenance
to paths, then a sink on the definition. The definition already exists and key taint already reaches the
line, so the producer work would not move detection, while it would touch reaching definitions, the write
census and the cache. Rejected for this lane. What survives of "key provenance" is the sink's by-name
decision of which operand is tainted (5.3).

**B. Sink on any tainted dynamic-key write.** Small, and matches the benchmark's "reach" credit. It fires on
`cache[id] = value`, on matrix writes and on one-level copies, which cannot pollute a prototype. Rejected.

**C. Syntactic facts for pollution-capable writes, plus an operand-aware sink** (chosen). A per-function
analysis classifies writes that can step onto a prototype object (section 5.1), names their selector and
property operands, and evaluates guards. The sink fires only when a tainted Use on the write's line names a
selector operand of such a write. Facts live in the core crate so that a CLI listing can expose them to the
benchmark harness; the sink lives in the review crate.

**D. Function summaries used as derived sinks** (mark `set` and `merge` as sinks at their call sites, by
argument position). Attractive for reporting at the caller, and it would work where taint does not enter
the helper. It needs a summary mechanism the taint code does not have (extra sinks are not reachable from
the CLI), and the helper-internal sink of C already reports through interprocedural reach. Kept as a
possible later presentation layer over the same facts.

## 5. Architecture — *rules tentative*

```
ParsedFile ──► proto_writes facts (core, per file, on demand)
                 │                                   │
                 ├─► prism nav proto-writes (CLI)    └─► review taint: js-proto-write sink (selector operand
                 │      joined by the harness               tainted by name, no sufficient guard)
                 │      with taint-reaches output           ─► taint_sink finding, CWE-1321
                 │      (diagnostic)
```

### 5.1 Classes of pollution-capable write

A dynamic-key write is an assignment (plain, compound or logical) whose target is a subscript with a
non-literal index. It is pollution-capable when one of these holds:

| Class | Shape | Why it can pollute |
|---|---|---|
| `nested_keys` | the target has two or more non-literal index levels (`a[k1][k2] = v`) | `k1 = "__proto__"` selects the prototype, `k2` names the polluted property |
| `path_walk` | the target's base is a cursor: (a) a variable that the same function reassigns from a dynamic read (`o = o[k]`, `o = o[k] = o[k] \|\| {}`, `o = o[k] ?? …`), in a loop, an array-method callback, or recursion; or (b) the result of a `reduce` call whose callback returns a dynamic read of, or a dynamic-key assignment through, its accumulator | an earlier step can move the cursor onto the prototype |
| `recursive_merge` | the target's base is a parameter (or a single alias of one) of a function that calls itself, directly or through one same-file helper, and the recursive call passes a dynamic read of that base (`target[key]`) at the base's parameter position; the other arguments are not examined | the recursive call makes the callee's base the prototype |

Open for stage D, each with matrix rows it must satisfy:

- Where a **recursive path setter** belongs (row pw05: the cursor is a parameter of a function that calls
  itself with a dynamic read of it, or with an assignment whose value is one): `path_walk` or
  `recursive_merge`. Stage D decides this before slice S1a is sized. The slice gates are stated by class
  (section 8), so the decision moves cases between slices, never between pass and fail.
- Whether a **read-only walk followed by a fixed-name write** is a fact
  (`for (const s of path.split('.')) o = o[s]; o.enabled = true;`, or `o[KEY] = v` with a module constant).
  It pollutes; under the classes above it has no dynamic-key write and so no fact.
- How the classes are read where the wording leaves room: whether an alias of a dynamic read
  (`const u = users[id]`) makes `u` a cursor; whether a cursor may be assigned through an intermediate
  variable or a ternary; whether a call through a property (`exports.merge(…)`, `utils.set(…)`,
  `this.merge(…)`) is "calls itself"; whether an argument that wraps or aliases the read
  (`target[key] || {}`, `clone = target[name]`) is "passes a dynamic read". Until stage D, the *strict
  reading* answers no to each, and the experiment reports the strict and the liberal count (section 6).

Not in any class, and therefore never a finding: a one-level copy (`for (k in s) t[k] = s[k]`) with no
walk and no recursion; a write whose selector operands (below) are all numeric literals or numeric
counters, the cursor-moving reads included (`const row = rows[i]; row[col] = v` with `i` a counter); a
write to a `Map` or `Set` through methods. A *numeric counter* is a variable whose every write in the
function is a numeric literal, `++`, `--`, or `+=`/`-=` of a numeric literal, wherever it is declared (a
`for` initialiser, or an earlier `var i, j;`). A TypeScript `number` annotation is never evidence of a
counter: values that arrive as JSON strings can be any key (`grid[y][x] = v` with tainted `y` pollutes).
Other shapes that fall in a class but cannot pollute (a last-element index `rows[rows.length - 1][k]`,
indices from `forEach((x, i) => …)`, `entries()`, `indexOf`, `parseInt`, unary `+` or `| 0`, a key with a
constant prefix or suffix that cannot form one of the three names, a conditional of literals, enum and
constant member keys, a walk over the characters of a string) are matrix rows (PLAN T-1, rows nk05 to nk15
and pw12) that stage D must satisfy; so are their polluting neighbours (`'__' + k + '__'`, a walk over an
array of strings).

Each fact names its **key operands**. Each operand has its name as the data-flow graph spells it (an
identifier or a member chain: `keys`, `opts.key`), its base (the first identifier), its byte span, and a
**role**:

- **selector** operands, the keys that can move the target onto a prototype: for `nested_keys`, every index
  level of the write but the last; for `path_walk`, the indices of the reads and writes that move the
  cursor, plus every index level of the write but the last; for `recursive_merge`, the key of the dynamic
  read that the recursive call passes at the base's parameter position;
- the **property** operand: the identifiers and member chains in the write's last index.

Numeric counters are neither. An operand that qualifies for both roles is a selector. A selector may lie on
another line than the write (a walk key, a descent read); the property operand is always inside the write's
target. A walk step that is itself a dynamic-key write (`o = o[k] = o[k] || {}`) is a fact of its own; its
index moves the cursor, so it is a selector of that fact. Whether taint of the property operand alone is
silent (the tentative rule) or a `suggestion` is decided by stage D against rows.

### 5.2 Guards

Guard state is `none`, `partial` or `sufficient`. *Which syntactic forms are sufficient is decided by the
executable matrix, not by this table*; the table is the starting hypothesis. A guard is always judged
against the fact's **selector** operands.

| Form | Hypothesis |
|---|---|
| a strict-equality comparison of a selector (or of each path segment) with `"__proto__"` and with `"constructor"` or `"prototype"` (`===`/`!==`; `includes` or `indexOf` on a literal array or on a module-level array; `has` on a module-level `Set`; a `switch`), whose rejecting branch leaves (`continue`, `return`, `throw`, `break`) or whose accepting branch contains the step (`if (k !== '__proto__' && k !== 'constructor') { … }`), **on a key that is provably a string**: an element of a `split` result, a `for…in`, `Object.keys` or `Object.entries` key, or a variable tested by `typeof k === 'string'` on the same path | sufficient |
| the same forms on a key that can be an array (a setter that accepts `Array.isArray(path) ? path : path.split('.')`) | not sufficient: `[["__proto__"], "polluted"]` passes `===` and `includes`, and `o[["__proto__"]]` coerces to the string |
| a loose `==`/`!=` comparison of the same shape | to be decided by execution (an array operand is coerced before comparing) |
| the same comparisons for `"__proto__"` only | partial: `constructor.prototype` still pollutes. On the first level of a two-level `nested_keys` write it is in fact sufficient (`a.constructor[k2] = v` cannot reach `Object.prototype`); calling it partial errs toward a finding and is an accepted-loss candidate |
| `Object.prototype.hasOwnProperty.call(o, k)` (or `Object.hasOwn`) tested before each walk step so that a missing key is never followed; `hasOwnProperty.call(target, key)` before a merge's recursive descent | to be decided by execution |
| other forms seen in real fixes: `k in Object.prototype` or `hasOwnProperty.call(Object.prototype, k)`; a test of the whole path before it is split; a `filter` pass over the segments; a sanitiser that renames the key; a composite own-and-enumerable predicate on target and key; `Object.defineProperty` in place of assignment; redirecting to a scratch object when the name is `__proto__`; a cursor identity check (`o === Object.prototype`); null-prototype intermediates | to be decided by execution |
| the root created by `Object.create(null)` | not sufficient for a path walk (an intermediate `{}` has a prototype), nor for a recursive merge into a null-prototype target (it descends into an ordinary child) |
| a key validated by a regular expression or an allow-list | none, unless a row proves a specific form |

Placement, also a hypothesis for the matrix: a guard counts when it is in the same function as the write,
tests a selector operand (or a `const` initialised once from it), and lies on every path from where that
selector is bound (the loop iteration's start, the callback's entry, the function's entry) to the point
where it selects: the cursor-moving step for `path_walk`, the write for `nested_keys`, the recursive call
for `recursive_merge`. Consequences of stating it per selector:

- A guard on the property operand only does not count. A path walk guarded on its final key only still
  pollutes with `"__proto__.polluted"`; one guarded on its walk keys only is safe even with the final key
  free.
- For `nested_keys`, every selector level needs its own guard.
- A key that is rewritten between the guard and the step (`toLowerCase`, `trim`, any other transformation)
  is not guarded: `"__PROTO__"` and `" __proto__ "` pass the guard and pollute after the rewrite.

A `sufficient` guard means no finding, and only for a form backed by executed `safe` rows. Because a
sufficient guard suppresses findings, the fixed-version harvest of section 6 (E5, decision 7) is required
before any guard form suppresses, and the guard forms it finds are matrix rows. If the harvest cannot be
made, guard states are still computed and listed, but `sufficient` is treated as `partial` by the sink. A
`partial` guard keeps the finding and adds a note to its description. This follows the taint sinks'
existing policy: only proven non-sinks are suppressed.

### 5.3 The sink

In review taint, for JavaScript, TypeScript and TSX, a flow-path edge whose end (`edge.to`) is a **key hit**
makes its line a sink line. An edge end is a key hit for a fact when all of these hold:

1. it is a Use. A Def inside an index (`o[k = f(v)][j]`) is reached by same-line propagation from any
   tainted Use on the line and never counts;
2. it is in the fact's function (same function start line) and on a line the write's target spans;
3. its access path has the same base as one of the fact's **selector** operands and, where both carry
   fields, one path is a prefix of the other;
4. the fact's guard is not `sufficient` (5.2).

The match is by name, not by byte span, because the key's Use on the write line is usually a zero-width
anchor at the line start or the Use of the same name's right-hand occurrence (section 2). Byte spans are
used only to tell two writes on one line apart: a Use whose span lies inside one write's target is matched
to that write alone; a Use outside every target on the line (the zero-width anchor, a right-hand
occurrence) is matched to every write on the line that has a selector with its base. Two writes on one line
that share a key name are therefore judged together: the line fires when any of them is not sufficiently
guarded.

A selector that is itself a framework source expression has no edge ending at it. The check therefore also
counts as a key hit a selector whose byte span overlaps one of its line's framework source-access ranges,
which `taint/run.rs` already computes per line (`store[req.params.kind][id] = 1`). A write whose own line is
the only diff line, with the seed on that line, stays silent, as every existing sink does.

- **Where the check sits.** Once per flow-path edge in the sink-matching loop of `taint/run.rs`, outside the
  per-identifier loop, and before the skip that loop applies to framework-source lines (a key such as
  `req.body[k]` sits on such a line); the framework-source-access check runs once per JS-family source line.
  A hit records the line in a side set that the emission block reads.
- **What is emitted.** A finding of the existing category `taint_sink`, severity `warning`, with the
  description "tainted key from {source} reaches a prototype-capable write at line N (CWE-1321,
  js-proto-write)" and " (partial key guard)" appended when the guard is partial. A line keeps one finding:
  when the line also holds another taint sink, the description above is used, with " (the line also holds
  another taint sink)" appended. The tag and the CWE travel in the description because a finding has no
  field for them (decision 9 in section 10 is the alternative).
- **If lane 7's taint slice has merged before this lane's sink slice** (the preferred order; that lane
  posts the field's contract on #603 first): the rule is written to the finding's `rules` list as
  `js-proto-write` with CWE-1321, the description carries no tag text, a line that already holds another
  sink keeps its existing description and gains the rule with no suffix, and every reader in this lane (the
  benchmark join's `sink_finding`, the real-diff filter, the matrix row that asserts the tag) tests
  membership of `js-proto-write` in `rules`. This lane then adds its rows to that lane's expected-rules
  table. **If this lane's sink slice merges first**, the text above stands, its readers accept either form,
  and lane 7's taint slice migrates the producer and the readers.
- **Strength.** A line with such a hit is Strong whatever else is on the line; the per-line grade must not
  turn it into a `suggestion`.
- **Value, property and receiver taint.** A tainted value, a tainted property operand alone (tentative, 5.1)
  or a tainted receiver alone does not make the line a sink. One known exception lies outside this lane: a
  receiver-only seed reaches the key of a path walk that aliases its receiver parameter to the cursor when
  the formals share a line (section 2), so such a line fires today (`set(req.session, 'cart.items', v)`
  with a constant path reports a tainted key). It is a recorded known false positive, asserted as such in
  sink row `sk03b` so a later fix shows up; a data-flow issue is filed for it; its frequency is measured on
  the benchmark and the ordinary-code corpora (section 6 E3 and E4, PLAN S4). Whether to fix it is
  decision 10.

PR #841 edits the same loop, so that slice is rebased over #841 or lands after it. Review taint keeps
findings in the diff's file today (#602, PR #841); a helper in another file is reported once that lands, and
`taint-reaches` plus the listing cover it meanwhile.

### 5.4 The listing

`prism nav proto-writes --repo <dir> [--format json|text]` (CLI only, like `dfg-stats`; no MCP tool;
documented as unstable, so its output may change without notice) prints every fact: file, line, function,
class, key operands with their roles, guard state. It serves the PR fact lists, E4, and a **diagnostic**
for the benchmark: the harness joins it with each case's raw `taint-reaches` output to compute a
**finding-equivalent** (a case counts when the ground-truth write line holds a fact that is not
`sufficient`ly guarded and a selector operand is reached by name under the rule of section 6, E2) and to say,
for each case that does not count, which condition failed (no fact, guard, key not reached).

Once the sink exists (slice S2), the benchmark-yield gate counts the sink itself: `js-proto-write` findings
at the ground-truth line in the review-taint output the harness already saves per case (`taint.json.gz`,
section 2). All baseline traced cases have source and sink in one file, so the limit that review taint keeps
findings in the diff's file does not apply to them; a traced case whose source and sink are in different
files is listed and not counted until PR #841 lands. Before S2 (slice S1a), finding-equivalents stand in
for findings.

### 5.5 Where the code goes

```
src/ast/js_ts.rs                         module lines (the directory's module file)
src/ast/js_ts/proto_writes.rs            facts and the public entry point (ParsedFile::js_ts_proto_writes)
src/ast/js_ts/proto_writes/classes.rs    nested_keys, path_walk, recursive_merge, operand roles
src/ast/js_ts/proto_writes/guards.rs     guard state
src/navigation/proto_writes.rs           the listing (queries.rs is over the size ceiling: no logic there)
crates/prism-review/src/cli.rs, main.rs  NavQuery::ProtoWrites and its arm
crates/prism-review/src/algorithms/taint_sinks/proto_write.rs   the operand-aware sink check
crates/prism-review/src/algorithms/taint/run.rs                 the per-edge check, the side set, the emission changes
tests/ast/proto_key_use_test.rs          core test pinning the key's Use on the write line (stage T)
tests/fixtures/proto_writes/             matrix fixtures (one function per file) and the Node truth runner
```

No cache version change: facts are computed from parsed files on demand and nothing new is stored.

## 6. The experiment (stage M)

Five measurements on the current build, then a memo.

| Id | Question | Method |
|---|---|---|
| E1 | How many prototype-pollution cases trace today? | re-run the harness for that class on a main build (the harness gains a class filter and recorded pin overrides; eval-only change). Its raw outputs are kept for E2 and the yield gate |
| E2 | In traced cases, does taint reach the key? | from each case's raw `taint-reaches` output (`witness.json.gz`: every Use on the ground-truth line, with a verdict per source root) and a parse of the sink file (read, never run). **By name:** the key is reached when a Use on the ground-truth line has a Reached verdict from a data root (the harness's own `data_root` test) and its path base equals the base of an identifier or member chain inside an index expression of the write at the ground-truth line and column. **Strict:** the same, and the Use's byte span lies inside that index expression. Both counts are reported |
| E3 | How many ground-truth writes fall in a class of 5.1, and how are they guarded? | a reader labels each traced ground-truth function (read, never run) with a strict class, a shape label and the guard form (labels below). A prototype classifier script runs on the same functions as a cross-check, for the guard-token report and for the aliased-receiver exposure (5.3) |
| E4 | How often do the classes occur in ordinary code? | the classifier script on Excalidraw, the TypeScript compiler sources, `express`, `directus-directus/api` and `calcom-cal.com/apps/api` from `bench-repos`; when they give fewer than 20 class hits, `apache-superset/superset-frontend`, then `apache-answer/ui` are added, in that order, until 20 or the list ends. A reader labels up to 40 hits, sampled after sorting by file and line: can pollute if the key is attacker-controlled / cannot / guarded. The script also counts aliased-receiver exposure |
| E5 | Which guard forms do real fixes use? | the fixed versions of up to 20 converted packages, fetched as archives (decision 7, never installed or run); the reader records each fix's guard form and placement. Required whenever any guard form is to suppress a finding (5.2) |

E2 does not separate selector from property operands: in the benchmark the keys of a write derive from one
string or array, and the sink-based yield gate (section 8) applies the operand roles later. Name matching can
credit a case by coincidence (a callback parameter spelled like the outer formal); the reader marks such
cases and the memo lists them.

E3 labels. The **strict class** is `nested_keys`, `path_walk`, `recursive_merge` or `none`, judged by 5.1 in
its strict reading. The **shape label** is one of: the three class names; `helper_walk` (a walk in a helper,
the write through its result: `const parent = walk(obj, keys); parent[last] = v`); `alias_two_step` (an
alias of a dynamic read written through, no loop: `const u = users[id]; u[field] = v`,
`p = out[section] = out[section] || {}; p[key] = v`); `indirect_cursor` (a cursor moved through an
intermediate variable, a ternary, or the parent of the cursor); `property_recursion` (recursion through a
property: `exports.merge(…)`, `utils.set(…)`, `this.merge(…)`); `alias_arg_recursion` (the recursive call
passes an alias or wrapper of the read: `clone = target[name]; extend(deep, clone, copy)`,
`merge(target[key] || {}, v)`); `return_recursion` (`obj[key] = setPath(obj[key], rest, v)`);
`field_cursor` (a cursor held in an object field); `no_polluting_write` (no write on the ground-truth line
can pollute); `other` (with a note). The **liberal reading** also counts `alias_two_step` and
`indirect_cursor` as `path_walk`, and `property_recursion` and `alias_arg_recursion` as `recursive_merge`.

**Conversions** C = cases that are traced (E1) with the key reached by name (E2) and a class assigned (E3).
C is reported twice: **strict** (strict class not `none`) and **liberal** (strict class not `none`, or a
shape label the liberal reading counts). **Potential** P = cases that reach the function only or partially
and have a class: what upstream fixes (#595, #434, loader admission) would add. P is reported and not gated.

Before the gate is applied, the memo lists every traced case that does not convert, with its reason: key not
reached by name; no polluting write on the line; or no class, with its shape label. If most of them fail
because of how the key's arrival is matched (the key's Use is absent from the raw output, or present under
another name) rather than because the key is not reached or the write is in no class, the result contradicts
a premise of this design: ORCHESTRATION section 8, not a stop.

Gate, pre-stated, applied to C under the strict reading (the liberal reading is shown beside it):

| Result | Action |
|---|---|
| C ≥ 25 | continue to stages T and E without waiting |
| 15 ≤ C < 25 | post the memo and ask the owner |
| C < 15 | stop the lane; post the memo on #603 |

E4 does not gate. Its percentage is stated only on 20 or more class hits. If fewer than 70% of the sampled
class hits are judged able to pollute, the misjudged shapes become matrix rows and the class definitions are
tightened in stage D before any slice.

The memo (`EXPERIMENT.md` in this directory) holds E1 to E5; the per-case list of non-conversions; E2 by
name and strict; C strict and liberal, and P, by payload shape (path setter, merge, flatten, other) and by
shape label; the list of converted case ids; the classes' hit counts per corpus with the reader's tally; the
aliased-receiver exposure counts on the benchmark functions and the corpora; and the guard forms of the
fixed versions.

## 7. Truth by execution (stages T and E)

Every matrix row is a file exporting one function, plus a call description. A runner starts a fresh Node
process per row, payload, root kind and module mode, calls the function, and compares the prototype objects
before and after. Only fixtures written for this lane are executed. Benchmark packages are never executed.

1. **Payloads come from the row's template.** The call description says which argument takes attacker data
   and how a list of key segments is written into it: a delimited path string (the row gives the delimiter
   and syntax: `.`, `/` with a leading `/` as in a JSON pointer, brackets, `:`), an array path, an object
   with flat delimited keys (`{"__proto__.polluted": 1}`), a nested object built by `JSON.parse`, or a text
   syntax the row names (argument vectors, ini text). The runner expands a fixed family set through the
   template: direct (`__proto__`, `polluted`); one benign prefix segment (`a`, `__proto__`, `polluted`);
   doubled (`__proto__`, `__proto__`, `polluted`); `constructor`, `prototype`, `polluted`; and, for
   templates that accept arrays, each of these with every segment wrapped in a one-element array. The
   payload-family enumeration (PLAN T-1b) closes the set, which is stored as data in
   `tests/fixtures/proto_writes/truth/families.json`; this section does not list later additions.
2. **Root kinds.** Where the function takes a root, the call description lists the root kinds it is run
   with: a plain object, `Object.create(null)`, a class instance, an array.
3. **Module modes.** Every row runs as sloppy CommonJS and as a strict ES module, both wrappers generated
   from the one case text. Truth is `pollutes` if either does; both results are recorded.
4. **Positive control.** Each row has a benign payload through the same template (by default segments `a`,
   `b` and value 1) and its expected effect (for example `root.a.b === 1`). It must run without throwing and
   have that effect in each mode and root kind it is used with. A row whose control does not behave is
   INADMISSIBLE: never `safe`, and fixed before stage D. A row whose function throws on every pollution
   payload is `safe` only when its control passed.
5. **Detection.** Before and after the call, the runner snapshots the own property keys (strings and
   symbols) of `Object.prototype`, `Array.prototype` and `Function.prototype`, with the identity of each
   value; any difference is `pollutes`.
6. **Record.** `truth.jsonl` records the Node version and, per row, the result of every payload, root kind
   and mode, and the control's result.
7. **Containment.** The child runs under Node's permission model (`--permission`, with `--allow-fs-read`
   limited to the case file and the driver, and no other allowance), in an empty temporary working
   directory, with the environment cleared except `PATH`, under a 5 second limit. Node 24's permission
   model does not cover the network, so the runner refuses a case file that names `net`, `http`, `https`,
   `http2`, `dgram`, `dns`, `tls`, `child_process` or `worker_threads`, or calls `fetch`; and the
   orchestrator reads every new row before its first run.
8. **TypeScript rows** use only erasable type syntax and run with Node's built-in type stripping.

Each row also names its **attacker names**: the names, as spelled at the row's write, whose values derive
from the attacker argument. Required behaviour: for a `pollutes` row, a fact at the write line whose guard
is not `sufficient` and that has a selector operand whose base is an attacker name; for a `safe` row, no
fact, a `sufficient` guard, or no selector operand whose base is an attacker name. No row may stay
INADMISSIBLE.

## 8. Gates

| Gate | Threshold (default) |
|---|---|
| Experiment | section 6 |
| Matrix | every row meets the required behaviour of section 7; no row is INADMISSIBLE; exceptions are named accepted losses |
| Benchmark yield, by class | over the converted cases (C, strict reading) whose E3 class is one the slice has enabled: before the sink (slice S1a), finding-equivalents (5.4) ≥ 0.9 × that count; from slice S2 on, and at acceptance, `js-proto-write` findings at the ground-truth line in the saved review-taint output (5.4) ≥ 0.9 × that count; both on the same build. Cases the final rules call guarded are listed for a human check, not counted against the lane |
| Ordinary code | on the E4 corpora, every fact outside test directories is listed in the slice's PR; the E4 sample re-labelled against the final rules shows at least 80% able to pollute (on 20 or more class hits) |
| Sink on real diffs | after slice S2: review taint with the sink over a fixed set of at least 60 real JS/TS diffs from the ordinary-code corpora (PLAN S4); every `js-proto-write` finding is listed and a reader labels up to 40. Reported with its tally, including the aliased-receiver false positives, and not gated; a percentage is stated only on 20 or more findings |
| Sink | the sink rows: fires on selector-key taint; silent on value-only taint, on property-only taint (tentative, 5.1), on receiver-only taint where the write has no cursor alias, and under a sufficient guard; the aliased-cursor receiver row is asserted as the known false positive; `warning` severity even beside a weak call on the same line |
| No regression | review-taint output on the existing taint fixtures is unchanged except for new `js-proto-write` findings; no change to any data-flow statistic; the core test that pins the key's Use on the write line stays green |

## 9. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Key enumeration through callbacks (`Object.keys(x).forEach(k => …)`, `_.each`) | P cases whose first break is the callback; belongs to #434 |
| Rest and `arguments` formals | P cases with those first breaks; #595 |
| An object-literal argument reaching the callee's key (`merge(cfg, { user: req.body })` stops at the call) | P cases with that first break; beside #595 and #434 |
| Loader admission of `dist` and build directories | the 10 cases skipped today |
| A one-hop walker summary (a same-file function that returns a value reached by dynamic reads from its parameter), so a write through a helper walk's result is a fact | the E3 `helper_walk` count (the largest no-class group in the baseline reading); it is the mechanism `recursive_merge` already needs for its helper hop |
| Classes for the other no-class shapes (`alias_two_step`, `indirect_cursor`, `property_recursion`, `alias_arg_recursion`, `field_cursor`) | the E3 count per shape label |
| The receiver propagation of section 2 (a Use on a parameter list tainting sibling formal definitions) | decision 10, and the measured false-positive rate |
| Call-site (summary) reporting for helpers | reader feedback that the helper-internal finding is hard to act on |
| `Object.assign`, spread, library merge calls | a benchmark or corpus count; none of the 148 recorded sinks has this form |
| String-literal key identity and `[]`-path edges in the data-flow graph | a consumer that needs them (lane 5's field reads is a candidate) |

## 10. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Experiment thresholds | **Confirmed by the owner, 2026-11-20:** 25 / 15 as in section 6, applied to C under the strict reading | none | at the memo |
| 2 | One-level copies | not reported | reporting them as a weak finding is one class and one strength rule | stage D |
| 3 | A sufficient guard | no finding, once the fixed-version harvest has been made (5.2) | a weak (`suggestion`) finding instead is a strength rule | stage D |
| 4 | A partial guard | finding stays, with a note | — | stage D |
| 5 | Where facts live | core crate, with a CLI-only listing | moving them into the review crate removes the listing and the harness join | before S1a |
| 6 | Executing lane-written fixtures under Node for truth labels | yes: child process per row, payload, root kind and mode, 5 s limit, contained as in section 7 | without it, labels come from reading | before stage T |
| 7 | Fixed versions of benchmark packages as negatives | required before any guard form suppresses: downloaded with `npm pack` (archive only, never installed or run), at most 20. If the network does not allow it, no guard suppresses (`sufficient` is treated as `partial`) | without it, guards are tested only on lane fixtures | stage M |
| 8 | The sink slice and PR #841 | proceed and rebase; if #841 is still open when the sink slice is ready, do the merge-class slice first | — | at S2 |
| 9 | How a finding says it is this rule | category `taint_sink`, with the CWE and the tag in the description | a category of its own (for example `prototype_pollution`) needs a SARIF rule entry and a targets mapping arm, and changes what consumers of `taint_sink` see | before S2 |
| 10 | Fixing the receiver false positive (alias-twin definitions plus same-line propagation onto sibling formals, section 2) | not fixed in this lane: it is outside the lane's constraints (no producer change). A data-flow issue is filed, the rate is measured (E3, E4, PLAN S4) and reported. The owner or the design seat decides; the orchestrator never does | a fix is its own PR with a corpus differential and touches files PR #841 edits; until then sink row `sk03b` asserts the false positive | stage D |

## 11. Risks and accepted limits

- **The classes are syntactic.** A walk hidden behind a helper call (the largest no-class group in the
  baseline reading), a cursor stored in an object field, a cursor moved through an intermediate variable, or
  recursion through a property or through three functions is missed. Misses are counted against the
  benchmark by shape label, never guessed at.
- **Guard sufficiency is subtle.** That is why each guard form needs an executed row, and the fixed-version
  harvest must have been made, before it suppresses anything; unknown guards never suppress.
- **The sink rests on producer behaviour.** Detection needs an index identifier to have a Use on the write
  line; today that is a zero-width anchor or a shared right-hand Use, which other lanes (#818, the alias-twin
  slices, rule U) are reshaping. The core test of stage T turns red first if a later slice removes it.
- **A known false positive.** Receiver-only taint fires on path walks that alias their receiver with formals
  on one line (5.3, decision 10).
- **Name matching inherits coincidental credits** (a callback parameter spelled like the outer formal). They
  are few and are listed in the memo.
- **Benchmark ground truth is noisy** (one "vulnerable" coordinate sits in already-guarded code). A case the
  final rules call guarded is listed for a human check, not counted against the lane.
- **Merge-shaped cases mostly fail upstream.** The lane's benchmark yield is therefore dominated by path
  setters until #595 and #434 land.
- **Security-benchmark content and model filters.** Briefs for non-Claude seats use neutral static-analysis
  wording; product implementers work from the lane's fixtures and never open the benchmark tree.

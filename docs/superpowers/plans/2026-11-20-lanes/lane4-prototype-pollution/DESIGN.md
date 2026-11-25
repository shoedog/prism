# Lane 4: prototype-pollution tracing — design

Stage D design of record, pending the two design-review approvals. Tracking issue #603, parent #604. The
historical measurement protocol is in section 6 and its results are in [EXPERIMENT.md](EXPERIMENT.md).

## 1. Intent

A JavaScript or TypeScript write can change a shared built-in prototype when an externally controlled key
selects its receiver. The review rule identifies the selecting key and attaches a warning to the dependent
write. Ordinary path setters and recursive destination updates are the two common shapes.

Stage M converted 29 benchmark cases under its strict reading and passed its threshold of 25. Stage I will
measure how often the product emits the warning on those same cases and on ordinary code.

## 2. What exists today (verified on `origin/main` at 1179055f; key placement and receiver propagation on a0f2c427)

The issue's premise is out of date, which changes what has to be built.

| Fact | Reference |
|---|---|
| `target[key] = value` **is** recorded as a definition, of path `target.[]`, and `key` is a use on that line. This has been so since April. | `src/ast/lvalues.rs:628-634`; probe |
| The key is not part of the path: `o[k]` and `o["x"]` are both `o.[]`; `o[k].y` is recorded as `o.[]`. No data-flow edge starts or ends at a `[]` path (0 of 73 edges in the probe): reads of `o[k]` collapse to `o`. | `src/access_path.rs:152-163`; probe |
| The key's Use on the write line rarely has the key's own byte span. In a named owner, an identifier inside the index of a plain-statement write gets a zero-width Use at the line start (`o[k] = v`, `a[k1][k2] = v`, `o[keys[keys.length - 1]] = v`, `if (!t[key]) t[key] = {}`), or shares the Use of the same name's right-hand occurrence (`target[key] = source[key]`; `t[key] = f(t[key] \|\| {}, v)`, the occurrence inside the call argument). Exact spans exist for walk steps (`o = o[k] = o[k] \|\| {}`), compound and logical writes (`o[k] ??= {}`, `o[k] += v`), member-chain keys (`o[opts.key][name]`: `opts.key` exact, `name` zero width) and anonymous owners. In the baseline's 41 traced cases the key's Reached occurrence is zero width in 32, a right-hand occurrence in 3 and inside the index span in 1. | probe (`~/prism-evidence/lanes-2026-11-20/reviews/lane4-fable-r1-probes/spans-a.js`) |
| On the baseline binary the orchestrator saw an exact key Use inside the index at the read step in a loop body (`o = o[s]`) and a zero-width Use of `id` at the declarator line anchor in `const u = users[id]`. Both retain the key by name. | Orchestrator's Stage D probe, recorded under A-F2. Route 2 in 5.3 depends on both forms. |
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
| At the historical baseline PR #841 (cross-file taint context) edited `taint/run.rs`, `taint.rs`, `taint/seeds.rs` and `src/cpg/{trace,query,cfg_queries}.rs`. The design checkout includes it with `--taint-context` defaulting to `off`. | PR #841, `TaintContextArgs` in `crates/prism-review/src/cli.rs` |

Consequences:

- "Dynamic-key definitions" need not be built for detection. The write is already a definition and the key
  already arrives on its line. What is missing is knowing **which writes can pollute**, **which operand
  selects the prototype**, **which guards make the write safe**, and a **sink** that decides by name which
  operand is tainted: the key arrives as a line-level Use of its name, not as an occurrence inside the index.
- The Stage T core test pins the key Use on the write line. S1a-1 extends it at the read step and declarator.
  The seeded receiver case pins the separate propagation behaviour in decision 10.

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
property operands, and evaluates guards. A reached selector provides evidence for the dependent write under 5.3.
Facts live in the core crate so that a CLI listing can expose them to the
benchmark harness; the sink lives in the review crate.

**D. Function summaries used as derived sinks** (mark `set` and `merge` as sinks at their call sites, by
argument position). Attractive for reporting at the caller, and it would work where taint does not enter
the helper. It needs a summary mechanism the taint code does not have (extra sinks are not reachable from
the CLI), and the helper-internal sink of C already reports through interprocedural reach. Kept as a
possible later presentation layer over the same facts.

## 5. Architecture and detection rules

```
ParsedFile ──► proto_writes facts (core, per file, on demand)
                 │                                   │
                 ├─► prism nav proto-writes (CLI)    └─► review taint: js-proto-write sink (selector operand
                 │      joined by the harness               tainted by name, no sufficient guard)
                 │      with taint-reaches output           ─► taint_sink finding, CWE-1321
                 │      (diagnostic)
```

### 5.1 Fact classes and operands

**Recursive path setters belong to `recursive_merge`.** The test is whether a selected destination returns at a
recursive formal position. A shrinking path and a recursively enumerated source use the same rule. S1a handles
the two nonrecursive classes. Of the 29 strict conversions in stage M, 19 walks and two nested writes belong to
S1a/S2. All eight recursive writes belong to S3, including five path setters. Liberal-only cases add coverage
without changing those denominators.

Start at the recorded write target. Include ordinary, compound and logical assignments, prefix/postfix updates,
assignment targets inside array/object patterns (including rest and defaults), and member targets bound by a
`for…of` or `for…in`. Peel parentheses and erasable TypeScript wrappers. A computed key on the *source* side of
a destructuring pattern supplies a value (`t2-157`). A store on the original root with no preceding selection,
an own class-field definition, and a method call supply no target for this rule. Another assignment on the same
line is evaluated independently. A fixed leaf such as `cursor.enabled` or `cursor[KEY]` **is** a write through
an earlier selection (`pw10`, `pw11`). The run-time function body counts in methods, expressions, callbacks and
generators.

| Class | Required relation at the write |
|---|---|
| `nested_keys` | In the written member chain, at least one computed receiver lookup has a qualifying selector. The last member names the property. A fixed initial base such as `this.store` is fine. A fixed intervening field of an unknown child starts a new receiver chain. Carry selection across that boundary only for an explicit shared-base link (`constructor`, `prototype`, `__proto__`). Thus `a[k].fields[p]` alone supplies no relation from `k` to the final receiver. |
| `path_walk` | A live local receiver may be the result of a dynamic child read, even once and without a loop (`at01`, `at02`, `t2-041`). Follow same-function identifier copies, temporaries, a saved parent, optional subscripts, the two-argument native `Reflect.get`, and direct computed or positional destructuring of the read (`ic01–ic03`, `dc01`, `t2-007`, `t2-085`). An assignment expression takes its stored value. Follow an arm of a ternary, logical fallback or logical assignment that can retain the selected child. A step that stores and advances through that slot is itself a fact (`pw08`). Include loops in any clause, inline iteration callbacks mutating the captured cursor, and `reduce`/`reduceRight` with an explicit initial cursor and a callback returning the selected accumulator child. |
| `recursive_merge` | The writer stores through a destination formal or its local alias. Its call passes a selected child of that destination back into the destination position, directly or through **one** same-file helper. Track actual-to-formal positions on each edge, after erasing TS-only formals. A direct child, the result of assigning that child, a local alias, or a fallback/conditional with an arm retaining it qualifies (`pw05`, `ra01`, `ra02`, `rr01`, `t2-009`, `t2-038`). Return-style assignment qualifies. A bare call, a statically bound export/object property or a method on the same uniquely bound receiver qualifies (`rp01`, `rp02`). Require a same-file definition and stable binding, including for `utils.set`. Also unwrap a single constant `fn.bind(null)` with no bound positional arguments and native `Reflect.apply(fn, receiver, [actuals…])` with a literal argument list (`t2-065`, `t2-087`). An unresolved property spelling, a destination supplied only as `this`, or recursion into the source while passing the original destination yields no recursive fact. |

Use one fact per target. Give an explicit qualifying nested target priority, followed by recursive destination,
then walked receiver. Collect selectors from all qualifying relations at that target, including the slot in a
cursor-moving store. Scope local bindings by lexical identity. Join alternative reaching assignments by union.
An unconditional replacement with a fresh value, a copy of the selected object, or restoration of the entry
cursor on all exits kills the old relation (`t2-079`, `t2-108`, `t2-109`, `t2-165`). A deferred method using
another function’s captured *selected receiver* needs a summary. The inline iteration case above has a lexical
cursor and a visible step. Ordinary locals holding keys may come from parameters, captured input or callbacks.
The recursive forwarding hop and the single pure computation, predicate or assertion in 5.2 exhaust the admitted
call summaries. The numeric field proof below inspects writes within a closed lexical collection.

**Which keys can select.** Evaluate the key value used for the lookup. Suppress a selector when its entire value
domain excludes the three shared-base link names on ordinary data roots. If a local data literal or stable slot
visibly holds a watched prototype, keep a dynamic selector capable of selecting that slot even under a different
name. A finite table mapping keys only to benign string values can establish a key domain (`t2-093`, `t2-125`).
A Map method returning a receiver is outside the write classes (`nk15`, `t2-043`).

- Numeric sources include numeric literals, numeric arithmetic and bitwise coercions, unary `+`, the unshadowed
  `Number`, `parseInt` and `Math.floor`, and a counter with a numeric initial value and exclusively numeric
  updates. For `+=` require numeric operands on every reaching assignment. Length arithmetic, a native regexp
  match offset, typed-array element values and own index names of a fresh typed array qualify. Native array
  `forEach` positions, `entries()` positions and array/string `indexOf` results qualify with an identified
  built-in receiver/operation. Use a local literal/allocation, a dominating runtime test, or an explicit
  intrinsic call for that proof. The untyped receivers in `nk08a–c` have none. A runtime `Number.isInteger`
  check on the actual key and `String` of a proven number preserve the exclusion (`t2-018`, `t2-057`, `t2-098`).
- In a closed lexical collection of records, a field is numeric if all element constructors initialise it
  numerically and every reachable field store preserves that domain. Inspect local builders and captured arrays.
  An unknown writer or external element invalidates the proof. A `number` annotation, relational comparison, or
  the numeric index used to *load* a key never proves the loaded key numeric (`nk14`, `t2-044`, `t2-045`,
  `t2-069`). A nonnumeric alternative after a counter or search result invalidates its exclusion (`t2-017`,
  `t2-058`).
- Evaluate literal alternatives, immutable local/module data members, finite unmodified lookup tables, booleans,
  symbols, and the value-producing tail of a comma expression. For concatenations and templates, use the fixed
  prefix and suffix to test compatibility with the three names.
  This includes a compatible interpolation or a conditional with a reserved-name arm (`nk11`, `t2-042`). An
  incompatible fixed prefix or suffix excludes the selector (`nk10`). Retain interior-only templates: both the
  colon and underscore fixtures ran safe, so that pair does not support a new infix-only cut (8.1). Guard
  narrowing is
  recorded on an otherwise eligible fact in 5.2. Runtime key sources such as captures and enumeration of unknown
  object keys remain eligible. The keys of a literal regexp’s named capture groups are fixed, even when its
  captured values vary (`t2-055`, `t2-158`).
- A proven primitive string yields single characters under value iteration and position names under `for…in`. An
  untyped iterable can yield whole names. `pw12a` and `pw12b` are byte-identical and require the same walked
  fact. `for…in` over an array can enumerate named properties (`t2-056`). An erased annotation, a formal’s
  spelling, a row’s call template and the contents of `attacker_names` are never inputs to classification.

**Destination and ordinary-code exclusions.** A write-back is local to the enumerated destination when the same
stable slot is read and rewritten, and the only recursion transforms that slot’s current value. Check object
*and* key identity, including nested `Object.keys`/`Object.entries` over existing own data slots, or a recursive
destination enumerating its own keys (`t2-147`). Require a closed plain-data receiver with only private
traversed own children, proved from local allocations and stores (including a locally bound helper called on
that closed literal in `t2-147`). An opaque formal or library result lacks that proof even under native own-key
enumeration. A bare one-level rewrite with no recursive destination relation has no fact at entry. A
different object supplying the keys, an independently selected destination, or a recursive path argument
preserves the fact (`rm01`, `rr01`). A fresh base for a one-level store, an unconditionally installed fresh own
slot (`t2-078`), and the fresh buckets of native `Object.groupBy` give no route to a shared receiver. A fresh
ordinary root by itself still admits inherited reads (`pw07`, `t2-015`, `t2-105`).

For a null-prototype table, trace its actual allocation across local statements. Discharge a receiver lookup
only when every candidate child on that lookup is a newly allocated private data object/array. A null table
filled on a missing-key branch with only fresh literals qualifies. A walk built exclusively from a fresh null
root and fresh null children qualifies by the same invariant (`t2-097`). A known null root alone leaves other
receiver steps eligible (`t2-013`). For this exclusion, private means a locally allocated data child with no
unknown writer, escape, or use as another object’s prototype before the write. Treat opaque or shared children,
retained views, proxies and unknown stores as unproved. Initialisation conditional on the truthiness of an
inherited read leaves that read eligible. Distinct targets in one destructuring assignment receive independent
facts. The fresh-slot proof uses an earlier completed installation of the same stable own slot (the reused
property key has proven primitive identity as in 5.2) that dominates
the target as a separate statement. A replacement on the right-hand side occurs after capture of the assignment
receiver (`t2-110`). The bounded proof also leaves earlier sibling pattern targets unresolved (`t2-127`).

**Operands.** A selector is a value-bearing identifier or member chain used to choose a receiver at a retained
step. Record its spelling, base identifier, source span and file-local AST owner byte interval, plus lexical
declaration and reaching definition spans for the binding/version test in 5.3. Keep the write owner separately.
Include the key read that feeds a
cursor or recursive call and qualifying outer indices of the final target. Include the local key’s direct source
when it is a stable alias, an element/destructuring read from the path (including native `shift`, `pop` and the
element of a sliced path in an inline iterator), or a segment sliced from that path (`pw05`, `t2-009`,
`t2-050`). An array index used solely as a numeric offset has no role. For an unknown key-producing call the
value arguments inside the selector expression can contribute (e.g. `x` in `nk08c`). Ignore discarded
expressions, types and the destination object as sources of selector identity.

The final member’s value-bearing key operands have role `property`. Omit constant names and proven numeric
offsets from the operand list. A fixed or numeric leaf still has a fact when its receiver was selected.
The same value in both roles keeps `selector`. A selected *object* has the receiver role in the expression.
Property-only seeds give silence in `nk04`, `pw09`, `sk12` and `sk13`. Each enabled fact retains the selector
occurrences through which that particular write is reachable, for the consumer in 5.3.

### 5.2 Guard states

Classify the protection of the selector values and of the actual receiver immediately before the relevant
effect. Use `none` when no fact exists. `sufficient` silences a fact. `partial` leaves the warning and adds the
partial-guard note. E5 obtained 20 fixed versions, 14 in the strict-converted population. Their source
identifies forms worth testing. Suppression requires a matching executed safe fixture and the conditions below.

**Placement.** For each selector on each path to the write, match the same binding and value version. A per-step
test must cover the actual cursor advance or recursive destination argument on *every* iteration/descent. For a
direct chain, cover every qualifying receiver lookup. A stable key may also be checked after a side-effect-free
selection if every path to the dependent store then requires that check (`t2-121`). A finite identity check can
apply to the selected receiver itself. A guard for the final property alone has state `none` (`g31`). An
unguarded final property is compatible with full selector protection. In `g32` the selector test itself lacks string proof.

Compute the actual control-flow destination of `return`, `throw`, `break` and `continue`, including labels,
`catch`, `finally` and callback scope. A branch can reject the write, restrict its accepting branch, or skip
just the selected step. A skip suffices only if subsequent selection and the final write cannot reuse an
unchecked pending key or cursor. A switch-local break, an absorbed throw, a return followed by a store in
`finally`, and a return from an earlier callback give no protection (`t2-133`, `t2-134`, `t2-080`, `t2-081`). A
check on only some paths, iterations or selector levels is `partial` (`t2-026`, `t2-049`, `t2-063`). In `t2-062`
the catch requires a nullish root and the write requires a non-nullish root. No execution of the write benefits
from the catch check, so the state is `none`. A
changed binding, second getter read, changed input across `await`, or unchecked decode/normalisation invalidates
the old proof (`g34a/b`, `t2-027`, `t2-061`, `t2-074`, `t2-082`).

**Key-domain proof.** The core fact computes the guard state from evidence at the actual selection. The by-name
sink reads its guard state. A fixture argument, TS annotation, method spelling or successful call on a formal
supplies no runtime domain. For a native split require a proved primitive string receiver (literal, dominating
`typeof`, stored native `String` result or finite positive string equality), unmodified native operation and
ordinary literal separator. An explicit native string split with a literal string delimiter also establishes a
dense array of strings on normal return. For subsequent array operations require the same native-operation and
stable sequence proofs. A receiver with a custom `split`, iterator or filter can produce different values. A
regexp's coercion of its argument proves nothing about a later lookup with that argument.

For own-key tests and fresh-slot installation, the checked and used keys must have the same property-key value
across uses. A stored primitive string or symbol (including the result of native `Reflect.ownKeys`) supplies
that identity. Repeated conversions of an unknown object do not. Check receiver and key versions on each path.
For a deny-list comparison require a proved primitive string at the comparison and selection. Native `for…in`,
`Object.keys`/entry names and matched string capture groups provide strings. A finite positive test against
literal strings can establish its own domain on the accepting branch.

| Sufficient form | Executed safe witnesses with the proof |
|---|---|
| Reject both `__proto__` and `constructor` at every selection on proved strings, optionally also `prototype`. | g01 (enumeration), g26 (runtime type check), t2-048 (one stored string conversion) |
| Allow only exact safe literal strings for the selector or entire stable path. A bound same-file assertion must actually exit on rejection. Native set membership in a closed, unmodified set of safe strings proves the accepted key's type. | t2-073, t2-083, t2-121, t2-146 |
| Recompute the same pure normalisation on the same immutable string after checking its normalised result against a finite allow-list. | t2-152 |
| Follow destination-owned ordinary-data children or install the fresh fallback at each step, with stable property keys and no known shared child. Admit the composite own-and-enumerable test. A closed local table or cloned snapshot with private children admits an own-only lookup on an unchanged key. | g24, g25, t2-120, t2-166, t2-170 |
| Redirect the one string header selection to a fresh scratch object on the reserved name, with no second selector before the store. | g28 (the name is a literal regexp's captured string) |
| Admit the selected receiver by identity in a closed collection of fresh objects or by its local private brand. | t2-118, t2-153 |

A guard form without an executed safe witness **meeting its premises** stays at most partial. This includes the
current split-only deny-lists, deny-list helpers, switch, regexp checks, filtered/mapped segments and
whole-sequence prechecks. The split receivers in g04, g06–g07, g10, g15–g19, g21–g23, g32, t2-025, t2-029,
t2-072, t2-167, t2-168, t3-013, t3-024, t3-056 and seeded sk04 lack the native/string proof. In t2-141 an own
check and the subsequent reads repeat an unproved key. All have a partial guard. t2-167 and t2-168 remain silent
given their supplied names. sk04 requires a warning with the partial-guard note. The assertion on a finite
allowed string in t2-146 has an independent proof. Loose equality stays partial. The two-name combination
`__proto__` plus `prototype` stays partial in t3-023: the executed safe row also restricts the cursor to objects
and its split receiver is untyped.

`partial` covers a relevant but incomplete name set, comparison on a potentially non-string segment, a
remaining-path prefix or whole-parent check covering only some selections, or exclusion of just one possible
receiver identity (`g02`, `g11–g13`, `g20`, `g29`, `g33`, `t2-028`, `t2-050`, `t2-075`). An own check on a known
shared-value registry (`t2-114`, `t2-139`), a sparse-array precheck that skips elements the walk consumes
(`t2-131`), and a stateful selector regexp (`t2-132`) are partial. Unrecognised predicates, type annotations,
value/presence tests, tests on the source or a parallel schema cursor, and a null root or fallback without the
private-child invariant give `none` (`g03`, `g05`, `g08`, `g09`, `g30`, `t2-145`, `t2-151`). A step whose only
guard is in a different activation has `none` unless it uses the explicit predicate/assertion summary above.

An obvious reference to a watched prototype under an otherwise allowed key invalidates a name-only suppression.
An accepting equality with a fixed string proves that primitive value on the accepting branch. A deny-list on
unknown values always requires separate string evidence.

**Coercion boundary.** Class a loose `==`/`!=` name check as `partial`. `g14` is `safe` for the JSON-fed run,
including array segments. Its general array-accepting API supplies no stable string proof. A relational range
test on the original key (`t2-045`) supplies neither a numeric key nor a name guard. It is `none`. Both rows
require a warning and share loss `R-COERCE`. The supplied family enumeration records a `pollutes` neighbour for
each if a non-JSON element supplies conversion methods. The four-root and extended-root fields keep their
recorded values. In particular `g33` is already `pollutes` under JSON inputs and requires an ordinary
partial-guard warning.

### 5.3 The sink

For JS, TS and TSX, match reached **Uses** to the retained selector of a fact with guard `none` or `partial`. A
Def, a property-only Use, or a receiver-only Use supplies no match. Require the same base and compatible member
paths (one is a prefix where both carry fields). Report at the write target. Two observations can establish the
match.

1. **At the target.** Match an actual selector occurrence on a target line. Preserve its binding, value version,
   full member-path compatibility and lexical AST owner. A final-property occurrence alone supplies no selector
   evidence. The target and the selector may share a stable value with another target (`sk16a/b`).
2. **At an earlier selection.** Index the selector occurrences that can still supply that particular write's
   receiver, including callback and recursive steps in their own owners. The producer premise in section 2
   covers both an exact loop-body Use and a zero-width declarator Use. Rebinding the cursor to an unrelated
   value on all paths kills the link.

Both routes use the same fallback. A producer Use may represent several same-path occurrences on a line even
when its retained span is nonempty. Resolve candidate reads of that name/path under the producer's file, line
and available function identity to their AST owners, lexical declarations and reaching value versions. Use the
exact in-operand span directly when the producer occurrence is unambiguous. For a zero-width anchor, a shared
RHS Use, or a retained span outside the earlier operand, admit by name only when **every indistinguishable
read** denotes one owner, one binding and one version compatible with the selector. Include reads outside
eligible targets, guarded siblings and reads across a same-line assignment in this test. A retained
representative byte offset does not resolve an otherwise conflated pair. An unambiguous match can support each
live dependent write using that same value. Check each write's guard separately before line deduplication.

A different owner on the same line, shadowing, an intervening redefinition, an unresolved member write or
multiple reaching versions defeats fallback unless syntax uniquely resolves the read. An ambiguous Use supplies
no credit. Record `M-AMBIG` once per affected case if no independent selector witness succeeds, list both
candidate spans and count the miss against yield. The row intersection test in section 7 supplies identified
selector Uses and does not simulate this producer loss.

A directly selected framework source also qualifies at the selector’s own span under the existing framework
source-access overlap test. The ordinary diff, file, source and same-line seed exclusions apply to the target
finding. A read-only step supplies evidence, never a finding at the read. A cursor-moving assignment can
independently be a target (`sk01`).

- **Where the check sits.** Check each flow-path edge end once outside the per-identifier loop, before the
  framework-source-line skip. Check source-access ranges once per JS-family source line. Record the dependent
  fact’s source, owner and write location for emission, including when no existing sink or tainted Use occupies
  that target line. One selector occurrence may support several writes. Deduplicate by target line under the
  existing finding contract.
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
- **Value, property and receiver taint.** A value-only, property-only or isolated receiver-only seed is silent
  (sk02, sk03a, sk03c, sk12, sk13). An aliased receiver with same-line formals can also reach the selector under
  the existing producer. Keep sk03b as the known warning, measure it at S4, and track the producer behaviour in
  #854 (decision 10).

PR #841 is present on the design checkout. Cross-file taint context is opt-in (`--taint-context` defaults to
`off`). Use default same-file review for the benchmark and real-diff measures. Report any opt-in experiment
separately.

### 5.4 The listing and benchmark join

The unstable CLI-only `prism nav proto-writes --repo <dir> [--format json|text]` lists facts with file, owner,
target span, class, role-tagged operands and guard. Join the listing at the designated target (file, line and
column) of each frozen strict conversion. Record the structural class and selector, including its actual site
when earlier. The pre-sink finding-equivalent is a **diagnostic**: join the saved ground-truth-line Uses with a
Reached verdict from a data root to the target's eligible selector under 5.3. Record missing facts, guards,
missing earlier-site evidence, ambiguous binding/version, and property-only reaches separately. There is no
selector-site recapture in this policy.

Six nonrecursive cases need evidence outside the frozen line: confucious, eivindfjeldstad-dot,
grunt-util-property, json-pointer 0.6.0 and 0.6.1, and just-safe-set. The write-line ceiling is 15/21 even by
spelling, at most 14/21 after removing minimist's cross-binding coincidence. In grunt-util-property the final
property reuses a key name. In recursive doc-path 2.0.0 the line carries `keyPath` while the recursive selection
uses `currentKey`. Keep all these cases in their original class denominators.

S2 starts the yield gate on actual `js-proto-write` findings at the designated file and line in the same build's
review-taint output (`taint.json.gz`). Review can see the earlier selector through 5.3. Before S2, measure
structural coverage and show the frozen finding-equivalent diagnostic alongside it. Section 8 specifies
thresholds and guard review.

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

The five measurements below are the historical stage-M protocol. Its strict and liberal readings and conversion
populations are frozen. Section 5 defines the product rules.

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

E3 labels. The **strict class** is `nested_keys`, `path_walk`, `recursive_merge` or `none`, judged by the
pre-stage-D strict reading used in the memo. The **shape label** is one of: the three class names; `helper_walk`
(a walk in a helper,
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

Use the recorded `truth` for each call. Project its exercised root kinds onto the four ordinary kinds and
include calls that construct their own root (`root: none` in the execution record). The watched objects are
`Object.prototype`, `Array.prototype` and `Function.prototype`. All 326 IDs, files and four-root labels agree
with the saved Node v24.15.0 execution. There are 153 `pollutes`, 173 `safe` and zero inadmissible rows. Seven
extended-root neighbours change from safe to pollutes (g01, g04, g18, g19, g24, g25, g28). g29 already pollutes.
Use the four-root result for the gate and disclose the extended results independently of the current guard
decision.

[ROW_REQUIREMENTS.md](ROW_REQUIREMENTS.md) specifies each designated write. With the row's named selector Uses
supplied as reached, require `fires` exactly when a fact has a selector base in `attacker_names` and its guard
is `none` or `partial`. A witness in the table names such a base. All other decisions are `silent`. Resolve
names at the selector's binding. A source-seeded test separately measures actual propagation and direct
framework-source overlap (`rm02` versus `sk11`). In `t3-051` the supplied name is the selected receiver `entry`.
The selector's base is `req`. The named-input test is silent (M-ROW-SELECTOR). A separately supplied source
range on `req.params.section` must reach the dependent write in S2.

The 326 decisions comprise 202 fires and 124 silences. Of the silences, 96 have no fact, 16 have a sufficient
guard and 12 have a fact without a matching selector and without sufficient protection. The 293 in-scope
decisions comprise 136 detections, 17 misses, 96 safe silences and 44 safe reports. The remaining 33 scope rows
require 22 fires and 11 silences.

## 8. Gates

| Gate | Threshold (default) |
|---|---|
| Experiment | section 6 |
| Matrix | every row meets the required behaviour of section 7; no row is INADMISSIBLE; exceptions are named accepted losses |
| Benchmark before S2 | S1a-1 and S1a-2 require structural coverage of at least 19/21 nonrecursive cases: an enabled fact with the required class and a selector at the designated write or its live receiver step. S1b preserves this coverage. If S3 precedes S2, apply the same structural metric to all classes (at least 27/29). Record frozen finding-equivalents and ambiguity reasons per class as diagnostics. |
| Benchmark yield from S2 | On the enabled strict conversions, gate once on the **aggregate** actual findings. Report numerator and denominator for each class. With no exclusions, require 19/21 (19 walks + two nested writes) or 27/29 with the eight recursive cases enabled. S4 uses all 29. |
| Guard exclusions | For yield only, enumerate sufficient guards for independent human checking of the actual selector, domain and placement. Remove only explicitly approved guarded cases from both numerator and denominator. With raw size N and G approvals require at least ceil(0.9 × (N − G)) distinct unexcluded cases with findings. Publish raw and adjusted counts by class, IDs and approvals. Pending/rejected guards and ambiguous reaches stay in the denominator. If all cases have approved guards, report zero eligible cases and omit the yield percentage. |
| Ordinary code | Apply 8.1 to the final fact population. Require ≥80% capable at 20 or more sampled final facts. Below 20 after all reserves, record insufficient evidence and the counts without a percentage. That recorded outcome satisfies the reporting condition for acceptance. |
| Sink on real diffs | after slice S2: review taint with the sink over a fixed set of at least 60 real JS/TS diffs from the ordinary-code corpora (PLAN S4); every `js-proto-write` finding is listed and a reader labels up to 40. Reported with its tally, including the aliased-receiver false positives, and not gated; a percentage is stated only on 20 or more findings |
| Sink | the sink rows: fires on selector-key taint; silent on value-only taint, on property-only taint (5.1), on receiver-only taint where the write has no cursor alias, and under a sufficient guard; the aliased-cursor receiver row is asserted as the known false positive; `warning` severity even beside a weak call on the same line |
| No regression | review-taint output on the existing taint fixtures is unchanged except for new `js-proto-write` findings; no change to any data-flow statistic; the core test that pins the key's Use on the write line stays green |

### 8.1 Re-labelling the stage-M ordinary-code sample

This is a design re-labelling of the 32 saved coordinates, using the frozen `m/e4_hits.json` and
`m/e4_labels.jsonl`. Count structural facts before any source-reach filter. The reader labelled six capable,
three guarded and 23 incapable under the measuring script. In particular a numeric **leaf** can still be a real
write. Declaring upstream callers trusted would erase that distinction.

| E4 hit IDs | Required disposition | Basis |
|---|---|---|
| 5, 6, 9, 10, 21, 27 | Keep six `nested_keys` facts | Ordinary table with a dynamic receiver selection. A truthiness check, a check of an existing value’s shape, a DB convention or a numeric property operand cannot establish a private receiver. None has a confirmed attacker-reachable selector in the reader labels. |
| 1–4, 25–26 | No fact | Closed graph-node records initialise `level` numerically and write it from a numeric accumulator. Subtraction yields a number. The point offsets in 25–26 use arithmetic, with a fixed numeric suffix even on the string arm of `+ 1`. Array and TS types alone earn no exclusion. |
| 7–8 | No fact | A separately allocated null-prototype table receives fresh arrays/records on missing keys. The final store targets one of these local children. The query origin of the key does not remove this proof. |
| 11 | Keep the nested fact | Library-produced metadata passes through own-key enumeration. The selected children have no local plain/private-data proof. |
| 12–20 | Keep nine recursive facts | `filter` and `deep` are opaque destinations. Their recursive calls descend through a dynamic child and `for…in` includes inherited slots. Hits 14 and 19 put the separator on the **written property**. The recursive selector is still `key`. |
| 22–23 | No fact | The receiver key always carries a fixed path prefix. |
| 24, 28–32 | No fact | 24 crosses a fixed interior field, with a prior dependent access that the reader found cannot complete for the shared receiver. The binding-table stores address an intervening table or a just-installed value. A fixed leaf alone is never an exclusion. At 30 the reader also identifies a destination-own check with an effective throwing exit. Crediting a guard independently would also require a stable property key, ordinary method identity and a verified exit. |

The reader’s reason for hit 29 describes the adjacent fixed-property store at line 1672 of `woff2-bindings.ts`.
The saved coordinate is line 1668, whose receiver crosses `.overloadTable`. The disposition above uses the
recorded coordinate. At 30 the fixed-interior-field rule suffices independently of the proposed throw-helper
guard.

Of 32 original coordinates, the design retains 16 and removes 16. Six of the retained hits had the reader's
capable label and ten had the incapable label. These are design dispositions. Product cut frequencies and the
final listing remain to be measured. An executed example supports a bounded rule only where the corpus site
independently meets its premises.

| Proposed E4 cut | Recorded safe / pollutes controls and decision (t3 suffixes unless named) |
|---|---|
| Own-slot write-back (11–20) | 001/025 and 002/026 distinguish the supplied calls. Opaque inputs defeat the plain-data proof at 001/002 and hit 11. 027 and 038 are safe one-level writes with no recursive destination; 028 pollutes with recursive descent. Neither one-level control licenses removing hits 12–20. Keep the narrower closed-literal rule (`t2-147` / 026). |
| Numeric selector (1–4, 25–27) | 003 safe versus 029/031 polluting for a replaced or externally initialised field. 004/032, 005/033 and 006/034 distinguish native operations. 035/036 distinguish arithmetic from unrestricted input. 030 pollutes with a numeric leaf. Apply the proof only to a receiver selector. |
| Template fragments (14, 19, 22–23) | 007 and 037 both ran safe, including 037’s row-specific input. Withdraw the proposed interior-only cut. Keep qualifying facts for both rows. 038 puts the fragment on a one-level leaf. The older safe fixed-prefix `nk10` and polluting compatible concatenation `nk11` support the anchored prefix cut at 22–23. |
| Private children (7–8) | 008/009 safe; 039–041 pollute with a shared slot, ordinary root or opaque child. Require a null table and exclusively private possible children. |
| Fixed interior or installed private receiver (24, 28–32) | 010 safe versus 043/044 polluting on a direct receiver. The six coordinates have fixed interior fields. The narrower installation proof has safe `t2-078` (one string conversion) and polluting 045 (conditional installation). 011/012 ran safe but reuse an unproved split key. Keep their facts. |
| Effective exit | 013 safe; 042/046 pollute with a returning helper or a property-only check. 013 proves the helper exits but its repeated key lacks primitive identity. Keep its partial guard. The fixed interior at hit 30 accounts for its exclusion. |

On the final build, list all non-test facts and per-cut counts (candidates, excluded, retained), including new
fact shapes. Use the five M-4 corpora first. With fewer than 20 final class hits add
`apache-superset/superset-frontend`, then `apache-answer/ui`, then `screwdriver` from the local bench-repos
checkout, in that fixed order, stopping after the first corpus that reaches 20 or after the list ends. Pin every
revision. The historical reserve counts of 18 and zero are candidate counts. Sort by corpus, file, line and
target span and take all hits up to 40, otherwise sample 40 with seed 20261120. Keep sufficient facts in the
denominator and label capability independently of source reach. At 20 or more sampled facts require at least 80%
capable. Below 20 report capable/guarded/incapable counts and **insufficient evidence**. Acceptance can proceed
with that recorded result. A measured ratio below threshold at 20 or more does not pass.

## 9. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Key enumeration through callbacks (`Object.keys(x).forEach(k => …)`, `_.each`) | P cases whose first break is the callback; belongs to #434 |
| Rest and `arguments` formals | P cases with those first breaks; #595 |
| An object-literal argument reaching the callee's key (`merge(cfg, { user: req.body })` stops at the call) | P cases with that first break; beside #595 and #434 |
| Loader admission of `dist` and build directories | the 10 cases skipped today |
| A one-hop walker summary (a same-file function that returns a value reached by dynamic reads from its parameter), so a write through a helper walk's result is a fact | the E3 `helper_walk` count (the largest no-class group in the baseline reading); it is the mechanism `recursive_merge` already needs for its helper hop |
| Remaining storage, lookup and callee-identity cases beyond 5.1 | M-STORAGE, M-LOOKUP and the E3 residual shapes after measuring product facts |
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
| 8 | The sink slice and PR #841 | integrate the landed #841 loop; default same-file measurement, cross-file context opt-in | — | at S2 |
| 9 | How a finding says it is this rule | category `taint_sink`, with the CWE and the tag in the description | a category of its own (for example `prototype_pollution`) needs a SARIF rule entry and a targets mapping arm, and changes what consumers of `taint_sink` see | before S2 |
| 10 | Fixing the receiver false positive (alias-twin definitions plus same-line propagation onto sibling formals, section 2) | not fixed in this lane: it is outside the lane's constraints (no producer change). Issue #854 tracks it, the rate is measured (E3, E4, PLAN S4) and reported. The owner or the design seat decides; the orchestrator never does | a fix is its own PR with a corpus differential and touches files PR #841 edits; until then sink row `sk03b` asserts the false positive | stage D |

## 11. Accepted losses and scope

Every in-scope mismatch between the named-input requirement and recorded truth has an `accepted_loss` ID. Each
row keeps its testable class, guard and sink behaviour. The seeded and ambiguous-reach losses below are tracked
separately from the 326 named-input decisions.

| ID | Direction and reason | Rows |
|---|---|---|
| M-HELPER | Missed. A separate helper returns a walked object or moves a captured cursor through a named call. | hw01, t2-008 |
| M-STORAGE | Missed. A field, stack of destination/source pairs, or record-valued reducer carries the cursor. | fc01, im01, t2-086 |
| M-LOOKUP | Missed. A getter, map lookup, or awaited call supplies the selected object. Each needs a receiver relation outside 5.1. | t2-031, t2-043, t2-054 |
| M-CALL-WRITE | Missed at the designated write. A reflective setter, property-definition call or bulk-copy call performs the leaf store. An assignment elsewhere in the function cannot substitute. | g27, oa01, rs01, rs02, t2-023, t2-024, t2-070, t2-071 |
| R-ITER | Reported although safe. The body does not prove character/position iteration, native array callback/index semantics, or the type of an instance-method result. | pw12b, nk08a, nk08b, nk08c, t2-174 |
| R-COERCE | Reported although JSON-safe. Loose equality or a relational test leaves the property key unproved at an unrestricted API. | g14, t2-045, t3-022 |
| R-TARGET | Reported although safe. Frozen/shared candidates, proxy effects, a wrapper-built namespace, sibling-pattern effects, logical stores and typed-array views exceed the local receiver proof. Own enumeration of an opaque input lacks a private-child proof even when only that destination is transformed. | t2-111, t2-119, t2-124, t2-127, t2-130, t2-156, t2-172, t3-001, t3-002 |
| R-SCHEMA | Reported although safe. The checked parallel schema cursor cannot prove ownership of the corresponding destination child. | t2-151 |
| R-KEY-PROOF | Reported although safe in the run. The receiver of a split, the resulting sequence or the property key reused for a check or installation lacks a local native/primitive proof. Two further partial rows, t2-167/168, lack a matching supplied selector and remain silent. | g04, g06, g07, g10, g15–g19, g21–g23, g32, t2-025, t2-029, t2-072, t2-141, t3-011, t3-012, t3-013, t3-024, t3-056. Also sk04 outside the matrix. |
| R-TEMPLATE | Reported although safe. One interior-only template has no executed polluting control for the proposed cut. The compatible template remains eligible even though its recorded families ran safe. | t3-007, t3-037 |
| R-PAIR | Reported although safe. The two-name strict pair omits the constructor check and relies on an additional object-only step and unproved split domain. | t3-023 |
| M-ROW-SELECTOR | Missed with the supplied receiver name alone. The selector requires the base of the request member or independent framework-source evidence. | t3-051 |
| R-ASYNC | Reported although safe. Validation and the selected value meet across fulfilled asynchronous results. | t2-169 |
| M-BIND | Seeded sink miss. A recursive fact exists, but upstream propagation from the source into a callback key is missing (#434). | sk11 (same fact shape as rm02) |
| R-RECEIVER | Seeded sink false positive. An aliased receiver with same-line formals propagates to the selector (#854, decision 10). | sk03b |
| M-AMBIG | Potential producer miss. A collapsed Use cannot be attributed to a unique AST owner, binding and value version. Suppress that evidence and count every otherwise missed converted case against yield, with the competing spans. | No isolated frozen row. S2 exercises rebinding and same-line owners; count observed seeded/benchmark misses separately. |

`O-APP` marks 32 application-owned or user-class rows. Within the `t2-` series these are 014, 102–104, 106–107,
110, 112, 114, 122–123, 126, 128–129, 131–140, 142–145, 148–149, 154 and 162. `O-STRING` marks `t2-096`, where
the evidence names `String.prototype` outside the three watched objects. The row table marks every affected ID
and gives its static output. A `safe` observation here says only that no watched prototype changed. These 33
rows contribute to neither direction of the in-scope loss tally. Fixture substitutions remain visible in the
case source. `t2-034` uses an as-const object, `t2-036` an explicit constructor assignment, `t2-124` a frozen
namespace-shaped record, and `t2-125` a frozen numeric reverse table. All four have ordinary row decisions.

### Enumeration close and slice additions

Stage E is **CLOSED by the controller under the OpenAI allowance**. The t2 tiers reached their declared caps:
terra four rounds (36, 14, 13, 12), sol three (30, 21, 20), astra two (17, 15). Their last rounds still added
variants of existing classes and no new class. This closure uses the controller decision. Stage D freezes the
original 92, the 178 t2 rows and the 56 executed t3 rows (326 total). No materialisation remains for the eight
round-one requests.

Section 8.1 records the E4 pairs. The other t3 controls cover live earlier selectors and decoy seeds (014–018
and 047–051), recursive method identity and selected versus copied arguments (019–021 and 052–053), and stable,
stale or incomplete guard checks (022–024 and 054–056). Their recorded truths and every individual requirement
appear in the row table. A safe execution without the local proof can still require a warning.

The open class for slice generators is variants of **control flow and selector provenance**. Every later
discovery becomes an executed matrix row, with its neighbour and named requirement, **inside the slice that
fixes the gap**. Add seeded or producer probes there when the distinction concerns reach rather than execution
truth. Refresh changed base RED/GREEN receipts before enabling the row. In particular S2 tests same-line version
changes, shadowed owners and stable shared RHS reads. A broader interior-template cut or split-dependent
suppression needs a discriminating executed pair satisfying its actual premises before it enters a slice.

## 12. Change log

**2026-11-24 · Stage D, first draft.** Replaced the tentative class and guard readings with sections 5.1–5.3, preserved the
stage-M populations, and reconciled all 270 fixture expectations and the slice briefs. The row table records 110
in-scope required detections, 16 accepted misses, 98 in-scope safe silences, 13 accepted safe reports and 33
scope-tagged rows. The ordinary-code re-labelling is in 8.1. Product verification and the two design-review
approvals remain open.

**2026-11-24 · Stage D, round 2 of 2.** Targeted fold of both first-round reviews on the existing design. The
next joint design review is the last under the cap.

| Finding | Decision |
|---|---|
| A-F1, B-1 | Start yield at S2. Before S2 gate structural coverage and record the frozen write-line witness as a diagnostic. Keep all six earlier-selector cases and the minimist binding coincidence visible. |
| B-2 | One aggregate threshold with per-class counts. From S2 remove only human-approved sufficient guards from both sides of the yield calculation. Ambiguous reaches remain in the denominator. |
| A-F7 | S3 requires 27/29 aggregate structural matches if it precedes S2, otherwise actual findings under the adjusted yield rule. Record doc-path's earlier recursive selector. |
| A-F2 | Record the orchestrator's exact loop Use and zero-width declarator Use. Route 2 uses both. The orchestrator pins both in the first S1a-1 commit. |
| A-F3 | Add the selector to t2-119/124/131/144/151. All five fire, two under O-APP. Extend R-TARGET and introduce R-SCHEMA. |
| B-3 | Require local native/string or stable property-key proof. Enumerate all 16 surviving sufficient rows in 5.2. sk04 fires with a partial note. Section 11 names the R-KEY-PROOF reports. |
| B-4 | Store file-local AST owner spans for operands and writes. Allow name fallback only for one owner, binding and version. Disclose and count unresolved reaches as M-AMBIG. |
| B-5 | Retain E4 hits 11–20, including both written-property templates. Section 8.1 gives the proof and the safe/polluting controls for each proposed cut. Drop the unsupported interior-only cut. |
| A-F4 | Reconcile all 56 executed t3 controls. Where a pair failed to distinguish a cut or the source lacks its premises, retain the fact. Record the separate named-input miss for t3-051. |
| A-F5 | At fewer than 20 final facts after Superset, Answer and Screwdriver, accept the recorded insufficient-evidence outcome without a percentage. At 20 or more retain the 80% threshold. |
| B-6 | t2-062 has guard none. The catch cannot protect an execution reaching the write. |
| B-7 | #841 is present. Context is opt-in. Measure benchmark yield and real diffs on default same-file review. |
| B-8 | Record the controller's Stage E closure, tier caps and open generator class. Freeze 326 rows and admit future discoveries within their fixing slice. |
| A-F6 | Split S1a into separately reviewed structure and exclusion slices. S1b builds a bounded structured AST walker and shares lexical value versions with those slices. |
| A-F8 | Keep the four-root gate and disclose all seven extended-root flips, including those whose guards are now partial. |

The new contract has 136 in-scope detections, 17 misses, 96 safe silences, 44 safe reports and 33 scope rows.
The product gates and final design approvals remain pending.

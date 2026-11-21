# Lane 3: React value flow across JSX props — design

Status: architecture final; every rule marked *tentative* is fixed only after the census, the test matrix
and enumeration (PLAN stages M, T, E). Tracking issue: #596 (parent #604). Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane3-jsx-props.md`.

## 1. Intent

In `<Html html={text} />` the value `text` should reach the place where `Html` uses its `html` prop, so that
provenance, impact and taint questions cross component boundaries the way they already cross ordinary
calls. The first slice covers statically resolved components, named attributes and destructured props.
Spread, overwrite order, wrappers and unresolved components are handled explicitly: each either binds by a
stated rule or is counted as refused with a reason. State and context propagation are not in this lane.

Success is measured by correct end-to-end paths and by review examples a reader finds useful, not by the
number of edges added.

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| A JSX opening or self-closing element is an ordinary call site with `jsx_element = true`. Its span covers the whole opening element. Lowercase tags are dropped as intrinsic. Capitalised tags resolve like calls: same-file `local_def`, imports `import_member`, namespace members `import_qualified`, all Exact in the probes. | `src/queries.rs:150-166`, `src/call_graph.rs:176`, `src/resolution.rs:2158` |
| `memo(fn)` and `forwardRef(fn)` resolve to the inner function when the wrapper is a named import of `react` around a function expression in a `const`. `React.memo(Identifier)`, class components and `lazy` from another file do not resolve. A same-file `React.lazy(() => import(..))` binds the element to the loader arrow (a wrong target with no formals). | `src/ast/js_wrapped_export.rs` |
| Attributes are not arguments. A JSX site has an empty argument list, and the argument-binding loop skips it. No edge leaves any attribute value. Children are not represented. | `src/cpg/build.rs:1603` |
| **Destructured formals have no definition.** `function C({ html })`, arrows, `React.FC` arrows and `memo(({html}) => ..)` give `html` no Def and no positional slot; the body's use of `html` has no incoming edge. Plain `props` formals have a Def, and a member-only formal has projection edges `props → props.html`. One exception exists: a pattern whose default is another formal (`({ a } = other)`, `[u] = o`) already gets Defs at its identifiers from the parameter-default alias code. | `src/parameter_slots.rs:75-77`, `:481-520`; `src/ast/js_ts/param_alias.rs:184`; probes |
| Parameter occurrences and positional slots have many readers besides the data-flow producer: `FunctionInfo.param_names`, the call graph, the `peer_consistency`, `primitive` and `quantum` algorithms, and the reasoning layer's parameter seeds. JavaScript reference search is already fenced by binding scope, and that fence already understands object patterns. | `src/ast/tree_access.rs:226`, `src/call_graph.rs:1872`, `src/reasoning/seeds.rs:267`, `src/ast/parameters.rs:12`; `js_ts_binding_pattern_may_bind` |
| Object-literal arguments already bind field by field (#594): `f({ k: v })` adds `Use(v) → Use(p.k read)` for plain formals of an Exact-singleton callee. Destructured, rest and bare-arrow formals are refused. The pass runs inside the argument-binding loop and is mirrored in two test-only twins. | `src/cpg/object_literal_args.rs`, `src/cpg/callee_legacy.rs`, `src/cpg/step5b_reference.rs` |
| Review taint (default) follows every data-flow edge. The trace engine behind `taint_reaches` and `--taint-return-flow` crosses a function boundary only when the edge **ends at a Def**, exactly one call site matches the source occurrence, and that site resolves Exact. The #594 edges end at a Use, so the trace engine stops at them today. | `src/cpg/trace.rs:712-800`; probes |
| The `dangerouslySetInnerHTML` sink needs the taint path to end at a use named like the identifier on the sink line. | `crates/prism-review/src/algorithms/taint/js_ts/sinks.rs:129-175` |
| The attribute name itself (`html`) and tag names are recorded as variable uses in the caller. | probe `nodes-at` |

Size, by a regular-expression count on Excalidraw (not a census): 2,061 `name={…}` attributes on
capitalised elements, 20 spread attributes, 892 capitalised elements of which 209 are member elements;
component-like definitions with a destructured first parameter 194, with a plain identifier 36.

## 3. Goals, non-goals, anti-goals

Goals:

1. A definition for each binding element of an object-pattern formal, so destructured props exist in the
   data-flow graph.
2. One binder that connects a "props source" (an object-literal argument, or a JSX element's attributes) to
   a "props target" (a plain formal's member reads, or a pattern formal's binding elements), field by field.
3. Edges into pattern formals end at the element's Def, so both taint engines cross them.
4. Explicit outcomes for everything else: counters by reason for unresolved components, spreads,
   non-data values, keys the callee does not read.
5. Acceptance by a type-checker oracle (no wrong edge), by end-to-end path counts, and by rated review
   examples on Excalidraw and on the private corpus.

Non-goals (this lane): `useState`, `useContext` and effect propagation; `children`; spread pass-through
(`<Inner {...props} />` binding the spread's source); new sinks (`href`); class components; resolution of
`React.memo(Identifier)`, `lazy`, compound components built from object members; removing the spurious
attribute-name uses. Each is listed in section 9 with the census number that would justify a follow-up.

Anti-goals: a whole-object edge from an attribute value into `props` (it would taint every sibling prop);
edges from a component that is not an Exact singleton; a second copy of the object-literal key rules.

## 4. Approaches considered

**A. Generalise the #594 binder.** A JSX element is, for data flow, a call whose single argument is the
object literal of its attributes. Split the existing pass into a source side and a target side; add a JSX
source and a pattern target. One set of key, spread and overwrite rules serves calls and elements. **Chosen.**

**B. A separate JSX pass** with its own pairing logic (the sketch in #604). It would duplicate the key,
duplicate-key, spread and write-suppression rules and drift from them. Rejected.

**C. Rewrite JSX into a synthetic call with an object-literal argument before analysis.** It would change
byte spans, call-site records and every consumer keyed on them. Rejected.

For the callee side, two options were weighed. **Element Defs through the existing formal machinery**
(chosen): binding elements become parameter occurrences, so classification (unused, bare, member-only with
projection), refusals (escaped names, duplicates, `eval`, `arguments`) and intra-function edges come from
code that already handles formals. **A side table without Defs** (reads of pattern names found
syntactically) would avoid touching the producer but would leave the body's uses without any definition,
which is the defect itself.

## 5. Architecture

```
            props source                              props target
object-literal argument ─┐                     ┌─ plain formal: its member reads  (edge ends at a Use)
                         ├─► entries ─► bind ──┤
JSX element attributes ──┘   (ordered)         └─ pattern formal: binding elements (edge ends at a Def)
```

### 5.1 Pattern formals (callee side) — *rules tentative*

For JavaScript, TypeScript and TSX, an object pattern in a formal position contributes one entry per
binding element: the key path it reads and the local it binds.

| Pattern element | Key path | Local |
|---|---|---|
| `{ a }` | `a` | `a` |
| `{ a: b }` | `a` | `b` |
| `{ a = 1 }`, `{ a: b = 1 }` | `a` | `a` / `b`, marked defaulted |
| `{ a: { c } }` | `a.c` | `c` |
| `{ 'aria-label': l }` | `aria-label` | `l` |
| `{ ...rest }` | every key no sibling names | `rest`, marked rest |
| `{ [k]: v }` | unknown | `v` gets a Def and no incoming binding |
| an array pattern anywhere inside | — | that subtree gets nothing |

**A new accessor, not a wider one.** Pattern elements come from a new function
(`ParsedFile::js_ts_pattern_params`). The existing parameter occurrences, parameter names and positional
slots are not changed, so every other reader of them (section 2) behaves exactly as today, and a formal
after a pattern stays positionally unbound, as today. The only consumer in the first slice is the data-flow
producer, which registers a Def for each admitted pattern local in the same loop, and with the same
classification, as an identifier formal: at the function's first line with the binding identifier's bytes;
no Def when the local is unused; a base Def with projection edges when it is only read through members, as
in `({ user }) => user.name`.

Admission of an element (each rule has matrix rows):

- the local is a plain identifier, spelled without escapes, and no other formal or element of the list
  binds the same name;
- it has no default, or an inert default by the existing test (`is_inert_default_value`);
- the pattern formal itself has no default or the default `{}`. A pattern whose default is another formal
  is already handled by the parameter-default alias code; this lane adds nothing for locals that code
  defines, so nothing is registered twice;
- the owner-level refusals identifier formals get (`eval`, `arguments`, recovery nodes in the parameter
  list) apply unchanged.

Sibling identifier formals keep exactly what they get today. The pattern's position is its index in the
syntactic parameter list (a TypeScript `this` parameter not counted).

For binding (section 5.3) the facts per callee are the positional targets, unchanged, plus a pattern map.
It is generic over the target representation because the argument-binding step has a production path
(cached `ParamTarget`s) and two test-only twins that work on graph node indices:

```rust
pub(super) struct PatternEntry<T> { pub key: Vec<String>, pub key_known: bool, pub target: Option<T>,
                                    pub defaulted: bool, pub rest: bool }
pub(super) struct PatternFormal<T> { pub position: usize, pub entries: Vec<PatternEntry<T>> }
```

A wrapped component whose inner function is anonymous (`const C = memo(({ html }) => …)`) resolves by span,
while today's parameter facts are looked up by name and line range. The required behaviour is that it binds
like the named form; if the lookup cannot be made by span in this lane, it is an accepted loss with its
census count.

### 5.2 Props sources — *rules tentative where marked*

An ordered list of entries:

```rust
pub(super) enum PropsEntry {
    Key { keys: Vec<String>, value: PropsValue },   // value: Leaf(span) | Constant | Opaque
    Spread,                                          // {...x}: may write any key
    Wildcard,                                        // computed key
}
```

- **Object literal:** exactly today's rules, moved behind this type (shorthand, nested literals flattened,
  accessors and `__proto__` never bound, escaped and non-ASCII keys as wildcards).
- **JSX attributes,** in source order:
  - `name={expr}`: a Leaf when `expr`, after parentheses and TypeScript assertions are removed, is an
    identifier or a member path; nested Key entries when it is an object literal (`style={{ color: c }}`
    gives `style.color`); otherwise Opaque (calls, conditionals, templates, functions, elements).
  - `name="text"` and a bare `name`: Constant.
  - `{...x}`: Spread. The spread's own source is not bound in this lane.
  - `key` and `ref`: skipped. They are not props, bind nothing and overwrite nothing.
  - A namespaced attribute name: Opaque with that key.
  - Children do not produce an entry; a `children` key in the callee receives nothing.

Overwrite order is the object-literal rule: a Spread or Wildcard refuses every key written before it; keys
written after it bind; of duplicate keys the last one decides.

### 5.3 Binding

The site must resolve to exactly one Exact callee in the JavaScript family, and the target is the callee's
first formal (position 0). For object-literal arguments the target is the formal at the argument's position, as
today.

- **Plain formal:** unchanged from #594. `Use(value) → Use(p.<read>)` for each read whose member chain is a
  prefix of, or extends, the entry's key path, with the callee's flow-insensitive write suppression. Label:
  the worse of Exact and the read's definition-edge label. This includes #594's rule that a *bare* read of
  the formal (`const { a } = props`, `fn(props)`, `{...props}`) receives every key: the props object does
  hold every attribute, but everything after that read is no longer field-selective. Those edges are
  counted separately (`edges_to_bare_use` in the counters of 5.4) so their share is visible; follow-up F3 is
  the refinement.
- **Pattern formal:** `Use(value) → Def(local)` for each pattern entry whose key path is a prefix of, or
  extends, the entry's key path; a rest entry receives every key no sibling names; an entry with an unknown
  key receives nothing. Label: Exact (the call is an Exact singleton and the key match is syntactic).
- Dedupe by (from, to). The source occurrence is found by byte span, as for arguments, so the attribute
  name's own use node is never taken for the value.
- The binder keeps the existing generic shape: it takes the targets in any representation together with a
  function that resolves one to a graph node.

`Exact` here states a static binding, as everywhere in prism: this attribute name is this prop key of this
component. It does not say when React renders the component.

### 5.4 Counters

One new field on the existing statistics record, `dfg_props: BTreeMap<String, usize>` (the same shape as
`dfg_loop_plan`), printed by `nav dfg-stats` and under `call-stats.dfg_labels`. Keys: `sites_jsx`,
`sites_literal`, `edges_to_def`, `edges_to_use`, `edges_to_bare_use`, and refusals by reason:
`refused_not_exact_singleton`, `refused_no_props_formal`, `refused_value_not_data`, `refused_overwritten`,
`refused_key_not_read`, `refused_key_unknown`, `spread_sources_unbound`. They are computed where the
argument-binding step runs (at graph assembly, on every build: cold, full cache hit, partial hit), returned
beside the edges per caller and merged in a fixed order. They are the "handled explicitly" record for
unresolved components, wrappers and spreads.

### 5.5 Where the code goes

```
src/ast/js_ts/pattern_params.rs    pattern elements of a formal (declared in src/ast/js_ts.rs)
src/ast/parameters.rs              unchanged accessors; the JavaScript path gains nothing here
src/data_flow/build/file.rs        registers Defs for admitted pattern locals beside identifier formals
src/cpg/callee_params.rs           compute_pattern_formal_nodes and the cached ParamTarget form
src/cpg/props_binding.rs           PropsEntry, PropsValue, the generic binder, the counters
src/cpg/object_literal_args.rs     becomes the object-literal source; pairing moves to props_binding.rs
src/cpg/jsx_props.rs               the JSX attribute source
src/cpg/build.rs                   module lines (`#[path]`, like callee_params and object_literal_args), the
                                   call for JSX sites, the counters' merge, the `dfg_props` field. The file
                                   is at 1,777 weighted lines: wiring only.
src/cpg/callee_legacy.rs, src/cpg/step5b_reference.rs   the same calls at the same position, so the parity
                                   tests keep agreeing
src/cpg.rs                         `#[cfg(test)] mod` lines for the new test files
```

Each slice that changes the graph or the statistics record bumps `CACHE_VERSION` (189 on main today) and
adds its line to the list in `src/cpg_cache.rs`.

### 5.6 Effect on the taint engines

- Default review taint follows both edge kinds.
- The trace engine crosses pattern-formal edges (they end at a Def; the JSX site contains the source
  occurrence by byte range even when the element spans several lines). It still stops at plain-formal
  edges, as it does today for #594. Closing that is follow-up F5 (section 9), after PR #841, which is
  editing the same file.
- Limit: two elements of the same component on one source line are two matching call sites, and the trace
  engine refuses to choose between them. Counted in the measurement, not fixed here.

## 6. Measurement

**Census (stage M, before any design rule is fixed).** A Node script with the pinned TypeScript 5.9.3
joins `prism nav call-stats --dump-sites` records with the syntax tree and reports, for Excalidraw and one
more public React corpus: JSX sites by resolution outcome; attributes by value shape and by position
relative to a spread; the callee's first-formal shape (object pattern: flat, alias, default, rest, nested,
computed; plain identifier: member-only, mixed, destructured in the body; none); and the predicted number
of bindable (attribute, key) pairs. The model for the script is `~/prism-evidence/js-args/census.cjs`.

**Oracle.** The corpora are checked without their dependencies installed, so the oracle uses only what the
TypeScript checker can decide from the project's own files. For every cross-function edge the lane adds:
(1) component identity: the symbol of the element's tag name, with aliases followed, has a declaration that
is the edge's target function, or a `const` whose initialiser is an admitted wrapper call around it;
(2) key match, from syntax: the attribute's name (or the literal's key path) equals the property name of the
binding element the edge ends at, or is comparable with the member chain of the read it ends at. CORRECT
when both hold; WRONG when the tag resolves to another declaration or the keys do not match; UNDECIDED when
the tag's symbol cannot be resolved; INADMISSIBLE when the probe failed. A sample of UNDECIDED rows is read
by hand.

**Differential.** Base and head `nav dfg-stats --edges` and `nav call-stats` on each corpus, every change
attributed to a named class, exit non-zero on an unattributed change. The comparator model is
`~/prism-evidence/jsts-lane/783/scripts/compare761.py`.

**End-to-end paths.** From the head graph: chains that cross at least one props edge, by number of
component hops, split into chains ending at a sink line, chains ending at another element's attribute, and
others; each hop must be CORRECT.

**Review examples.** Thirty commits whose diff changes a line inside a JSX attribute's value expression on a
capitalised element, in a non-test file (decided with the TypeScript parser on the commit's version of the
file). They come from a corpus with full history: the second corpus has it; the pinned Excalidraw copy has
no history and the local clone is shallow, so Excalidraw contributes only if a full clone can be fetched.
For each commit, run `leftflow` and `fullflow` with the base and the head binary and show a reader
(Opus-5.5, high) the context that only the head binary adds. The reader labels each example useful, neutral
or noise against one question: would a reviewer of this change need to look at this code?

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Census go | predicted bindable pairs ≥ 1,000 across Excalidraw and the second corpus; otherwise stop and report |
| Slice S1 | differential: only additions (new Defs for pattern locals and their intra-function edges). No removal is expected, because reference search is already fenced by scope; any removal or label move on an existing edge is read and explained one by one. No counter outside the data-flow statistics moves. A Go or Python corpus shows no change at all |
| Oracle | 0 WRONG among added props edges; UNDECIDED reported with its hand-read sample |
| Volume (#604) | ≥ 1,000 props edges ending at JSX attribute values on the public corpora |
| Taint | the issue's example produces the `dangerouslySetInnerHTML` finding in default review taint and a Reached verdict from `taint-reaches` |
| Review examples | of 30, at least 15 gain context; of those, at least 60% useful and at most 10% noise |
| Private corpus | the same measurements run locally; aggregate numbers only |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Second public React corpus | the one of `calcom-cal.com`, `apache-superset` (front end), `apache-answer` (ui) with the most Exact-resolved JSX sites in the census | none | at the census |
| 2 | `key` and `ref` | never bound | binding `ref` for React 19 is a one-row change | before S3 merges |
| 3 | Review-example thresholds | section 7 | none | at stage G |
| 4 | Follow-ups F1 to F6 | none starts without the stated census number being met; then the orchestrator proposes it to the owner | — | after S4 |
| 5 | Attribute-name uses in the caller | left as they are; an issue is filed | removing them later is its own differential | — |
| 6 | Positional slots continuing past a pattern formal, and pattern locals as parameter names or seeds | not changed: both have readers outside this lane (two review algorithms, the reasoning layer's seeds) | follow-ups F7 and F8 | — |

## 9. Follow-ups, each with the number that would justify it

| Id | Follow-up | Justified when the census shows |
|---|---|---|
| F1 | Whole-object sources: a spread `{...x}` in an element or literal, and an identifier passed where the callee has a pattern formal (`D(obj)`) | spread attributes on ≥ 5% of JSX sites with attributes, on either corpus |
| F2 | `children` as a prop | ≥ 100 components that read `children` and are given an identifier or member expression as a child |
| F3 | Plain `props` destructured in the body (`const { a } = props`) | ≥ 10% of bound attributes target such components (the zero-width-read limit of #594) |
| F4 | Resolution of `React.memo(Identifier)`, `export default memo(..)`, compound components | ≥ 10% of capitalised JSX sites unresolved for these reasons |
| F5 | Trace-engine descent across edges that end at a formal's member read | plain-formal targets ≥ 10% of bound attributes; and PR #841 merged |
| F6 | `href` and `src` sinks; `useState` and `useContext` provenance | after this lane's review examples show reader value |
| F7 | Positional binding of identifier formals that follow a pattern formal | ≥ 100 such call sites on a corpus |
| F8 | Pattern locals as parameter seeds for `taint_reaches` and as parameter names for the algorithms that read them | a user request, or a seed-based measurement that needs it |

## 10. Risks and accepted limits

- **The producer change (S1) sits in heavily proven code** (reaching definitions, the write census, alias
  twins, pruning rules, parameter refusals). The differential's prediction is written before each run, and
  any label movement on existing edges is a finding to explain, not to re-baseline. Known interactions: a
  one-line owner's edges are labelled by the single-node rules, so a pattern local there may get `sameline`
  or `cfg_incomplete`, not `exact`; an owner that gains definitions can change its reaching-definition
  availability counters. The test-only legacy producer reads the unchanged accessors and is not updated;
  tests that compare the live producer with it are adjusted only by excluding owners with pattern formals,
  each exclusion named in the PR.
- **A wrong component target gives wrong edges.** Edges require an Exact singleton, and the oracle checks
  the resolved signature. The known wrong target (`React.lazy` loader arrow) has no formals and receives
  nothing.
- **Opaque values dominate some attributes** (callbacks, inline objects from calls). They are counted, not
  bound; the census decides whether a later rule is worth it.
- **Out of model:** props added at run time (`cloneElement`, higher-order components other than the
  admitted wrappers), `defaultProps`, and render timing.
- **Review taint across files** is the subject of #602 and PR #841. This lane changes no taint code;
  end-to-end checks therefore use `taint-reaches` as well as review taint.

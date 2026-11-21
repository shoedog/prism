# Lane 3: React value flow across JSX props — design

Status: architecture final; every rule marked *tentative* is fixed only after the census, the test matrix
and enumeration (PLAN stages M, T, E). Design review round 1 (2026-11-21) is folded into this text.
Tracking issue: #596 (parent #604). Research record: `~/prism-evidence/lanes-2026-11-20/research/lane3-jsx-props.md`.

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
| A JSX opening or self-closing element is an ordinary call site with `jsx_element = true`. Its span covers the whole opening element. Lowercase tags are dropped as intrinsic. Capitalised tags resolve like calls: same-file `local_def`, imports `import_member`, Exact in the probes. A member tag `<Q.M>` resolves `import_qualified`, Exact, whenever `Q` is any imported binding: through the namespace proof when `Q` is a namespace import (the site's `local_binding` is `JsLocalBinding::NamespaceImport`), otherwise by the module's file stem and the member's spelling. So `<Ctx.Provider>` with `import { Ctx }`, and `<Menu.Item>` with `import { Menu }` (named or default) where the module holds `const Menu = { Item: Impl }`, bind to whatever function of that module is spelled `Provider` or `Item`, a wrong target; the same rung gives `api.send(a)` a wrong Exact argument edge. A local `const` object, `props.X` or `this.X` qualifier gives `UnknownName`. | `src/queries.rs:150-166`, `src/call_graph.rs:176`, `src/resolution.rs:2158`, `:2372-2420`; probes `res2`, `res3` of the review |
| `memo(fn)` and `forwardRef(fn)` resolve to the inner function when the wrapper is React's (a named import of exactly `"react"`, or a member of its default or namespace import, `React.memo`) around a function expression that is the value of a `const` (`ParsedFile::js_ts_wrapped_export`, rules R2 to R12). Such a target is recorded as a wrapped callable (`JsTerminal.wrapped` in the file, `JsExportTarget::SpannedLocal` across files), and a non-JSX site to it drops `WrappedExportNonJsx`. `React.memo(Identifier)`, class components and `lazy` from another file do not resolve. Any other call around an inline function (`observer(fn)`, `styled(fn)`, `withRouter(fn)`, `useMemo(() => fn)`, a same-file `React.lazy(() => import(..))`) is a may-call binding and falls back to name search, which binds the element `local_def`, Exact, to the inner function under the declarator's name; for `lazy` that is the loader arrow, a wrong target with no formals. The resolution result carries no wrapper flag: both routes give the same resolution kind. | `src/ast/js_wrapped_export.rs`, `src/ast/js_binding_values.rs` (`js_ts_classify_call`, `js_ts_call_may_call`), `src/ast/js_binding.rs:28`, `src/resolution.rs:3428`, `:3583`; `docs/features/language-coverage/js-ts-known-gaps.md` §1.1 |
| Attributes are not arguments. A JSX site has an empty argument list, and the argument-binding loop skips it. No edge leaves any attribute value. Children are not represented. | `src/cpg/build.rs:1603` |
| A JSX attribute value has a Use node with its own bytes only when the element sits under a `return`, the right side of an assignment, or a call's arguments. Elsewhere (a declarator, an object-literal value, a conditional, a JSX child outside those) the line has one zero-width Use at its start, and a reference on a function's signature line (a one-line expression-bodied arrow) has none. A value captured from an enclosing function inside a callback has its def-use edge end at a node owned by the enclosing function. | `src/ast/rvalues.rs:291-372`, `src/data_flow/build/file.rs:567-569`; review probes `jsx`, `jsx2`, `jsx3` |
| **Destructured formals have no definition.** `function C({ html })`, arrows, `React.FC` arrows and `memo(({html}) => ..)` give `html` no Def and no positional slot; the body's use of `html` has no incoming edge. Plain `props` formals have a Def, and a member-only formal has projection edges `props → props.html`. One exception exists: a pattern whose default is a plain identifier that no formal at or after it binds (`({ a } = other)` with `other` a formal, `({ a } = outer)` with an outer variable, `[u] = o`) already gets Defs at its identifiers from the parameter-default alias code. | `src/parameter_slots.rs:75-77`, `:481-520`; `src/ast/js_ts/param_alias.rs:42-62` (`param_alias_of_entry`), `:184`; probes |
| Parameter occurrences and positional slots have many readers besides the data-flow producer: `FunctionInfo.param_names`, the call graph, the `peer_consistency`, `primitive` and `quantum` algorithms, and the reasoning layer's parameter seeds. JavaScript reference search is already fenced by binding scope, and that fence already understands object patterns. | `src/ast/tree_access.rs:226`, `src/call_graph.rs:1872`, `src/reasoning/seeds.rs:267`, `src/ast/parameters.rs:12`; `js_ts_binding_pattern_may_bind` |
| Object-literal arguments already bind field by field (#594): `f({ k: v })` adds `Use(v) → Use(p.k read)` for plain formals of an Exact-singleton callee. Destructured, rest and bare-arrow formals are refused. A bare read of the formal with its own bytes (`wrap(p)`) receives every key; a bare read in a declarator initialiser (`const { a } = p`), a condition, a `for-in` head or an `in` test has a zero-width Use and receives nothing. A string key that is not an identifier spelling (`'aria-label'`) is a definite key no static member read can name. The leaf's source node is the Variable node whose bytes lie inside the leaf's span. The pass runs inside the argument-binding loop and is mirrored in two test-only twins. | `src/cpg/object_literal_args.rs` (`literal_key`, `:134-151`), `src/cpg/build.rs:1711-1766` (`argument_var_node_in_span_with_exact`), `src/cpg/callee_legacy.rs`, `src/cpg/step5b_reference.rs`; review probe `l16` |
| Review taint (default) follows every data-flow edge. The trace engine behind `taint_reaches` and `--taint-return-flow` crosses a function boundary only when the edge **ends at a Def**, exactly one call site matches the source occurrence, and that site resolves Exact. The #594 edges end at a Use, so the trace engine stops at them today. It finds the call site in `call_graph.callers`, which is keyed by the site's spelled callee name, using the **target function's** name, so it does not descend when the two differ (a renamed import, a default import under another local name, a wrapper whose inner function has another name). A site matches the source when it has the same caller and either the same line or a span containing the source's bytes. | `src/cpg/trace.rs:712-820` (`descend_target` region, `call_site_matches_source_occurrence`), `src/call_graph.rs:1205`; review probe `alias` |
| Both taint engines propagate from a reached Use to every Def on the same line of the same function, so in a one-line function a tainted formal reaches its sibling formals. | `src/cpg/query.rs:767-799`, mirrored in `src/cpg/trace.rs`; review probe `oneline` |
| The `dangerouslySetInnerHTML` sink needs the taint path to end at a use named like the identifier on the sink line. | `crates/prism-review/src/algorithms/taint/js_ts/sinks.rs:129-175` |
| The attribute name itself (`html`) and tag names are recorded as variable uses in the caller. When an attribute's name equals its value's name (`title={title}`), the line has two byte-distinct Use nodes of that path. The def-use edges of a line reach one Use node per (function, line, path): the node the variable index returns for that key (`VarIndex`, first wins). When the name token comes first, the value's own node has no incoming edge. | `src/cpg/var_index.rs:188`, `src/cpg/query.rs:85` (`var_node`), `src/cpg/build.rs:897-935`; probe `nodes-at`, review probe `jsx4` |

Size, by a regular-expression count on Excalidraw (not a census): 2,061 `name={…}` attributes on
capitalised elements, 20 spread attributes, 892 capitalised elements of which 209 are member elements;
component-like definitions with a destructured first parameter 194, with a plain identifier 36. How many
member elements have a namespace-import qualifier, and how many targets sit behind a wrapper other than
React's, are census numbers.

## 3. Goals, non-goals, anti-goals

Goals:

1. A definition for each binding element of an object-pattern formal, so destructured props exist in the
   data-flow graph.
2. One binder that connects a "props source" (an object-literal argument, or a JSX element's attributes) to
   a "props target" (a plain formal's member reads, or a pattern formal's binding elements), field by field.
3. Edges into pattern formals end at the element's Def, so both taint engines cross them.
4. Explicit outcomes for everything else: counters by reason for unresolved components, spreads,
   non-data values, keys the callee does not read, values with no source node, member qualifiers and
   wrappers the lane does not admit.
5. Acceptance by a type-checker oracle (no wrong edge), by end-to-end path counts, and by rated review
   examples on Excalidraw, the second public corpus and the private corpus.

Non-goals (this lane): `useState`, `useContext` and effect propagation; JSX children as the `children` prop;
spread pass-through (`<Inner {...props} />` binding the spread's source); new sinks (`href`); class
components; resolution of `React.memo(Identifier)` and `lazy`; member elements whose qualifier is not a
namespace import (compound components such as `Menu.Item` or `Ctx.Provider`: today they resolve to a
same-spelled function of the module, so 5.3 refuses them, and the resolver defect is filed as its own
issue); components behind a wrapper other than React's `memo` and `forwardRef`; removing the spurious
attribute-name uses; bare reads of a plain formal with a zero-width Use. Each is listed in section 9 with
the census number that would justify a follow-up.

Anti-goals: a whole-object edge from an attribute value into `props` (it would taint every sibling prop);
edges from a component that is not an Exact singleton; edges into a target the lane cannot show is the
rendered component (a non-namespace member qualifier, an unadmitted wrapper); a second copy of the
object-literal key rules.

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

For the caller side's source node, three options were weighed. **The line-keyed Use node** (chosen, 5.3):
the node the line's def-use edges already end at, with the value's span used only as proof. **Exact-span
nodes** (the call path's rule) disconnect every value whose spelling occurs earlier on its line, which JSX
makes the normal case (`title={title}`), and find nothing where the element is not under a `return`, an
assignment or a call. **Def-use edges to every occurrence on a line** would change the producer's
one-use-per-line model and every label proof; not for this lane.

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
- every pattern on the path from the formal to the local has no default or an inert one: `{ a: { c } = {} }`
  admits `c`; `{ a: { c } = make() }` and `{ a: { c } = other }` do not, because the default runs code or
  reads another binding before `c` is bound;
- the pattern formal itself has no default or an inert one, tested after removing TypeScript assertion
  wrappers around the default (`{} as P`, `<P>{}`, `{} satisfies P`, `{}!`), which run no code;
- the pattern formal is not one the parameter-default alias code defines. That code's own predicate
  (`param_alias_of_entry`, `src/ast/js_ts/param_alias.rs`: the entry's default is a plain identifier that no
  formal at or after the entry binds, whatever the identifier names, an earlier formal or an outer variable)
  decides; such a formal is skipped whole, so `({ a } = other)` and `({ a } = outer)` keep exactly the Defs
  main gives them and nothing is registered twice. A pattern whose identifier default names a later formal
  fails the previous rule;
- the owner-level refusals identifier formals get (`eval`, `arguments`, recovery nodes in the parameter
  list) apply unchanged.

Sibling identifier formals keep exactly what they get today. The pattern's position is its index in the
syntactic parameter list (a TypeScript `this` parameter not counted).

For binding (section 5.3) the facts per callee are the positional targets, unchanged, plus a pattern map.
A formal skipped by the alias rule is recorded in the map as skipped, with no entries, so the binder can
count it (5.4). The map is generic over the target representation because the argument-binding step has a
production path (cached `ParamTarget`s) and two test-only twins that work on graph node indices:

```rust
pub(super) struct PatternEntry<T> { pub key: Vec<String>, pub key_known: bool, pub target: Option<T>,
                                    pub defaulted: bool, pub rest: bool }
pub(super) struct PatternFormal<T> { pub position: usize, pub alias_skipped: bool,
                                     pub entries: Vec<PatternEntry<T>> }
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

- **Object literal:** today's rules, moved behind this type (shorthand, nested literals flattened,
  accessors and `__proto__` never bound, escaped and non-ASCII keys as wildcards), with one change of
  representation: a string key that is not an identifier spelling (`'aria-label'`) keeps its content as the
  key instead of being dropped. No static member read can spell such a key, so every plain-formal target
  binds exactly as today; a pattern entry with the same string key (`{ 'aria-label': a }`) binds, for literals
  and JSX alike. These are the one set of key rules for both sources.
- **JSX attributes,** in source order:
  - `name={expr}`: a Leaf when `expr`, after parentheses and TypeScript assertions are removed, is an
    identifier or a member path; nested Key entries when it is an object literal (`style={{ color: c }}`
    gives `style.color`); otherwise Opaque (calls, conditionals, templates, functions, elements).
  - `name="text"` and a bare `name`: Constant.
  - `{...x}`: Spread. The spread's own source is not bound in this lane.
  - `key` and `ref`: skipped. They are not props, bind nothing and overwrite nothing.
  - A namespaced attribute name, and `__proto__`: Opaque with that key.
  - `children={expr}`: an ordinary Key on a self-closing element or an element with no child content. On an
    element with child content, the children replace it at run time, so the entry is overwritten (counted
    `refused_overwritten`).
  - Child content produces no entry; a `children` key in the callee receives nothing from it.

Overwrite order is the object-literal rule: a Spread or Wildcard refuses every key written before it; keys
written after it bind; of duplicate keys the last one decides.

### 5.3 Binding

**Target admission.** Checked in this order; the first check that fails is the site's one refusal (5.4):

1. The site resolves to exactly one Exact callee in the JavaScript family (`refused_not_exact_singleton`).
2. Member qualifier: a JSX site whose tag is a member expression (`<Q.M>`), and a call site whose callee is
   a member expression when the target is a pattern formal, binds only when the site's qualifier is a
   namespace import, that is when the site's `local_binding` is `JsLocalBinding::NamespaceImport` (set by
   `ParsedFile::js_ts_namespace_binding`). Every other qualifier (a named or default import of a value, a
   re-export of one, a type-only import) is refused (`refused_member_qualifier`). The call half exists
   because a pattern target is new in this lane and the same resolver rung would give it the wrong callee
   (`api.send({ a: x })` into an unrelated `export function send({ a })`); a plain-formal target of a member
   call keeps #594's behaviour unchanged.
3. Wrapper (JSX sites only; a call keeps today's behaviour, where a `debounce`-like wrapper is right): let
   `F` be the callee's function node. When `F`'s parent, after `parenthesized_expression`
   nodes, is the `arguments` of a `call_expression`, the component is reached through that call. It binds
   only when the call is an admitted React wrapper: the call is the value of a declarator in a `const`
   declaration that is a statement of the program (directly or under an `export` statement), and
   `ParsedFile::js_ts_wrapped_export` returns `Ok` for that declaration with the line span of `F` (the same
   predicate, R2 to R12, that records `memo` and `forwardRef` targets for resolution; `useCallback` is not
   admitted). Any other call (`observer(fn)`, `styled(fn)`, `withRouter(fn)`, a nested `memo(observer(fn))`,
   `export default observer(function X() {…})`, `useMemo(() => fn)`, the `lazy` loader arrow, a wrapper
   declared inside a function) is refused (`refused_unknown_wrapper`). Resolution gives both routes the same
   kind, so this syntactic test on the target is how the binder tells them apart. The test lives in a
   `ParsedFile` accessor beside the pattern accessor (5.5).
4. A props formal exists at the target position. For JSX sites the position is 0; for object-literal
   arguments it is the argument's position, as today. No formal there: `refused_no_props_formal`. A pattern
   formal the alias rule skips (5.1): `refused_alias_pattern`.

**Entries into targets.**

- **Plain formal:** unchanged from #594. `Use(value) → Use(p.<read>)` for each read whose member chain is a
  prefix of, or extends, the entry's key path, with the callee's flow-insensitive write suppression. Label:
  the worse of Exact and the read's definition-edge label. A bare read of the formal with its own bytes
  (`fn(props)`, `<Inner {...props} />`, `return props`) receives every key: the props object does hold every
  attribute, but everything after that read is no longer field-selective. Those edges are counted
  separately (`edges_to_bare_use`) so their share is visible. A bare read with a zero-width Use (a
  declarator initialiser such as `const { a } = props`, a condition, a `for-in` head, an `in` test)
  receives nothing, as in #594 today and in this lane (`refused_zero_width_read`); follow-up F3 is the
  refinement.
- **Pattern formal:** `Use(value) → Def(local)` for each pattern entry whose key path is a prefix of, or
  extends, the entry's key path; a rest entry receives every key no sibling names (counted `edges_to_rest`:
  a rest local is not field-selective); an entry with an unknown key receives nothing. Label: Exact (the
  call is an Exact singleton and the key match is syntactic).
- Dedupe by (from, to).
- The binder keeps the existing generic shape: it takes the targets in any representation together with a
  function that resolves one to a graph node.

**Source node.**

- **Object-literal leaves:** as #594, the Variable node whose bytes lie inside the leaf's span
  (`argument_var_node_in_span_with_exact`), unchanged. (Calls keep main's limit: a value whose spelling
  also occurs earlier on the line, `g(x) + h(x)`, binds from a node the line's def-use edge does not reach.)
- **JSX values:** for each path `P` a Leaf value contributes (the full member path and its base, as for
  arguments), the source is the Use node that the variable index returns for the key (caller file, the
  site's caller function and its start line, `L`, `P`, Use), where `L` is the line of the value's first
  byte (for a value spanning lines, its lines in order, the first that has a node). This is the node the
  line's def-use edges end at and the node both taint engines start from. The value's byte span is used
  only as proof that this attribute's value spells `P` (the Leaf test of 5.2); the node's own bytes are
  never compared with it, and may be the attribute name's token (`title={title}`), another attribute's
  value (`<D a={x} b={x} />`, `<Row key={it.id} item={it} />`) or a zero-width anchor at the line's start
  (an element in a declarator). The `path` and `trace_reaches` assertions of the matrix check, on every
  binding row, that the source is reached from the value's definition. When the site's caller has no node
  under the key, the entry binds nothing and is counted: `refused_source_in_enclosing_owner` when a function
  enclosing the caller has a node under the same key with its own name and start line (looked up outward),
  else `refused_source_not_found`.

Two members of the second class are stage D decisions, with their rows and census counts: a reference on
a signature line, which has no node (`const One = ({ title }) => <D title={title} />;`: mint the use or
accept the loss), and a value captured from an enclosing function inside a callback (bind from the
enclosing owner's node or accept the loss; for a block-bodied callback the caller's own node exists but has
no incoming edge). The element-context enumeration (PLAN Task T-3) may add members; each goes to stage D the
same way.

`Exact` here states a static binding, as everywhere in prism: this attribute name is this prop key of this
component. It does not say when React renders the component.

### 5.4 Counters

One new field on the existing statistics record, `dfg_props: BTreeMap<String, usize>` (the same shape as
`dfg_loop_plan`), printed by `nav dfg-stats` and under `call-stats.dfg_labels`. They are computed where the
argument-binding step runs (at graph assembly, on every build: cold, full cache hit, partial hit), returned
beside the edges per caller and merged in a fixed order. They are the "handled explicitly" record for
unresolved components, wrappers, qualifiers, missing sources and spreads. A site-unit refusal is counted
once per site, by the first admission check of 5.3 that fails; entry-unit keys count each entry of an
admitted site.

| Key | Unit | Counts |
|---|---|---|
| `sites_jsx` | site | JSX element sites with at least one attribute in a JavaScript-family caller, before admission |
| `sites_literal` | site | call sites with at least one object-literal argument, before admission |
| `edges_to_def` | edge | edges into a pattern local that is not a rest local |
| `edges_to_rest` | edge | edges into a rest local |
| `edges_to_use` | edge | edges into a plain formal's member read |
| `edges_to_bare_use` | edge | edges into a bare read of a plain formal with its own bytes |
| `refused_not_exact_singleton` | site | admission check 1 |
| `refused_member_qualifier` | site | admission check 2 |
| `refused_unknown_wrapper` | site | admission check 3 |
| `refused_no_props_formal` | site | admission check 4, no formal |
| `refused_alias_pattern` | site | admission check 4, alias-skipped pattern formal |
| `refused_zero_width_read` | site | admitted sites whose plain target formal has a zero-width bare read that its Def reaches |
| `refused_value_not_data` | entry | Constant or Opaque values |
| `refused_overwritten` | entry | entries a later Spread, Wildcard, duplicate key or child content overwrites |
| `refused_key_not_read` | entry | Leaf entries no read or pattern entry of the target matches |
| `refused_key_unknown` | entry | pattern entries with an unknown key, once per admitted site |
| `refused_source_not_found` | entry | Leaf JSX entries with no source node in the caller or an enclosing function |
| `refused_source_in_enclosing_owner` | entry | Leaf JSX entries whose source node is in an enclosing function only |
| `spread_sources_unbound` | entry | Spread entries |

### 5.5 Where the code goes

```
src/ast/js_ts/pattern_params.rs    pattern elements of a formal (declared in src/ast/js_ts.rs)
src/ast/js_ts/component_route.rs   admission check 3: direct, admitted wrapper, or other wrapper
src/ast/parameters.rs              unchanged accessors; the JavaScript path gains nothing here
src/data_flow/build/file.rs        registers Defs for admitted pattern locals beside identifier formals
src/cpg/callee_params.rs           compute_pattern_formal_nodes and the cached ParamTarget form
src/cpg/props_binding.rs           PropsEntry, PropsValue, the generic binder, admission, the counters
src/cpg/object_literal_args.rs     becomes the object-literal source; pairing moves to props_binding.rs
src/cpg/jsx_props.rs               the JSX attribute source and its line-keyed source lookup
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
- The trace engine crosses pattern-formal edges: they end at a Def, and the line-keyed source of 5.3 matches
  its element by line, or by byte containment in a multi-line opening element. It still stops at
  plain-formal edges, as it does today for #594.
- Limit: the trace engine does not descend when the tag's spelling differs from the target function's own
  name (`import { Html as RichText }`, `import View from` a file whose default export is `function Html`,
  `const Button = memo(function ButtonInner(…) {…})`), because it finds the site through the name-keyed
  caller index. The edge exists and default review taint follows it; `taint-reaches` reports the boundary.
  Counted in the measurement.
- Limit: two elements of the same component on one source line are two matching call sites, and the trace
  engine refuses to choose between them. Counted in the measurement, not fixed here.
- Limit (inherited): in a one-line component both engines go from a reached Use to every Def on the same
  line, and pattern Defs sit on the function's first line, so a tainted prop reaches its sibling props
  (`function Card({ title, body }) { return <div title={title} dangerouslySetInnerHTML={{ __html: body }} />; }`
  reports a finding for a tainted `title`). Accepted, pinned by a matrix row, counted by the census
  (one-line components with two or more props).
- Closing the plain-formal stop and the name-keyed descent is follow-up F5 (section 9), after PR #841, which
  is editing the same file.

## 6. Measurement

**Census (stage M, before any design rule is fixed).** A Node script with the pinned TypeScript 5.9.3
joins `prism nav call-stats --dump-sites` records with the syntax tree and reports, for Excalidraw and one
more public React corpus: JSX sites by resolution outcome; member elements by qualifier kind (namespace
import, named import, default import, other); targets by route (direct, admitted wrapper, other wrapper);
sites whose tag spelling differs from the target function's name; attributes by value shape and by position
relative to a spread; elements by statement context and owner kind (the axis of PLAN Task T-3); the callee's
first-formal shape (object pattern: flat, alias, default, rest, nested, computed; plain identifier:
member-only, mixed, destructured in the body, other zero-width bare read; none); one-line components with
two or more props; the source-node outcome per Leaf value under the 5.3 rule (connected: the caller has the
node and a def-use edge ends at it; found but unconnected; in an enclosing function only; not found); and
the predicted number of connected bindable (attribute, key) pairs. The model for the script is
`~/prism-evidence/js-args/census.cjs`.

**Oracle.** The corpora are checked without their dependencies installed, so the oracle uses only what the
TypeScript checker and the TypeScript syntax tree (independent of tree-sitter) can decide from the project's
own files. For every cross-function edge the lane adds:

1. Component identity: the symbol of the element's tag name (or of the call's callee), with aliases
   followed, has a declaration that is the edge's target function, or a `const` whose initialiser is an
   admitted wrapper call around it. When the declaration is a `const` initialiser, an object-literal
   property or an expando assignment (`Tabs.Tab = X`) whose right side is an identifier or a function
   expression, the oracle follows that one step before comparing. A member tag whose qualifier is not a
   namespace import is WRONG unless that step reaches the target.
2. Key match, from syntax: the attribute's name (or the literal's key path) equals the property name of the
   binding element the edge ends at, or is comparable with the member chain of the read it ends at.
3. Attribution, from the TypeScript tree: some attribute of an element on the source's line (or of a
   multi-line opening element containing it) whose tag resolves to the target has that key and a value that
   spells the source's path; that attribute is the key's last writer (no later attribute of the same name,
   no later spread); it is not `key` or `ref`; a `children` attribute is on an element with no child
   content. For a literal the same checks apply to the argument's properties.

CORRECT when all three hold; WRONG when the tag resolves to another declaration or a check fails;
UNDECIDED when the tag's symbol cannot be resolved; INADMISSIBLE when the probe failed. UNDECIDED rows are
bounded (section 7) and sampled by hand: a seeded random sample of 50 per corpus (all when fewer) is read by
a Claude seat against the same three checks, and a hand-read WRONG counts as an oracle WRONG.

For slice S1, which adds intra-function edges only, a binding-identity oracle: for each added edge from a
pattern Def, the checker's symbol of the binding element at the Def's bytes equals the symbol of an
identifier spelled like the path's base on the use line (CORRECT if one does, WRONG if none does, UNDECIDED
if the line has none). A hand-read sample of 30 added `exact` edges whose local has a write reference in its
function completes it.

**Differential.** Base and head `nav dfg-stats --edges` and `nav call-stats` on each corpus, every change
attributed to a named class, exit non-zero on an unattributed change. The comparator model is
`~/prism-evidence/jsts-lane/783/scripts/compare761.py`.

**End-to-end paths.** From the head graph: chains that cross at least one props edge, by number of
component hops, split into chains ending at a sink line, chains ending at another element's attribute, and
others; each hop must be CORRECT. Also the connected share: among bound JSX props edges whose value's root
the checker binds to a parameter or local of a function, the share whose source node has an incoming
data-flow edge.

**Review examples.** Two directions, each with its own display, because `leftflow` and `fullflow` keep
forward results only in the diff's own function:

- *Forward:* commits whose diff changes a line inside a JSX attribute's value expression on a capitalised
  element. Display: the frontier of `prism nav taint-reaches --source <file>:<changed line>` (it descends
  up to two calls) with the base and the head binary; the reader sees the lines only the head reaches.
- *Backward:* commits whose diff changes a line in a component body that reads one of its props (a pattern
  local of its first formal, or a member read of a plain first formal). Display: `fullflow` (it keeps
  definitions in another file) with the base and the head binary; the reader sees the lines only the head
  adds.

Both in non-test files, decided with the TypeScript parser on the commit's version of the file, 15 per
direction. They come from a corpus with full history: the second corpus has it; the pinned Excalidraw copy
has no history and the local clone is shallow, so Excalidraw contributes only if a full clone can be
fetched. `private-corpus` contributes up to 10 per direction if it has history, run locally under the
private-code rules, with only counts in public text. Two readers (Opus-5.5 high and Sonnet-5.5 high, in
separate sessions) label each example useful, neutral or noise against one question: would a reviewer of
this change need to look at this code? An example is useful when both say useful, noise when either says
noise, otherwise neutral; agreement (share of equal labels and Cohen's kappa) and every disagreement are
reported.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Census go | predicted connected bindable pairs ≥ 1,000 across Excalidraw and the second corpus, and on each of the two the rate (connected bindable pairs over identifier- or member-valued attributes on Exact-resolved capitalised elements) ≥ 30%; otherwise stop and report |
| Slice S1 | differential: only additions (new Defs for pattern locals and their intra-function edges). No removal is expected, because reference search is already fenced by scope; any removal or label move on an existing edge is read and explained one by one. No counter outside the data-flow statistics moves. A Go or Python corpus shows no change at all. Binding-identity oracle: 0 WRONG; hand-read sample: 0 wrong |
| Oracle | 0 WRONG among added props edges, the attribution checks included; UNDECIDED ≤ 25% of added props edges on each corpus; the hand-read UNDECIDED sample has 0 WRONG |
| Volume (#604) | ≥ 1,000 CORRECT props edges starting at JSX attribute values on the public corpora |
| End-to-end | connected share ≥ 90% on each public corpus; ≥ 25 chains with two or more component hops, every hop CORRECT, on the public corpora |
| Taint | the issue's example produces the `dangerouslySetInnerHTML` finding in default review taint and a Reached verdict from `taint-reaches` |
| Review examples | per direction: of 15, at least 8 gain context; of those, at least 60% useful and at most 10% noise |
| Private corpus | the same measurements, review examples included, run locally; aggregate numbers only |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Second public React corpus | the one of `calcom-cal.com`, `apache-superset` (front end), `apache-answer` (ui) with the most predicted connected bindable pairs in the census | none | at the census |
| 2 | `key` and `ref` | never bound | binding `ref` for React 19 is a one-row change | before S3 merges |
| 3 | Gate and review-example thresholds | section 7 | none | at stage G |
| 4 | Follow-ups F1 to F9 | none starts without the stated census number being met; then the orchestrator proposes it to the owner | — | after S4 |
| 5 | Attribute-name uses in the caller | left as they are (the 5.3 source rule does not depend on them); an issue is filed | removing them later is its own differential and moves edge endpoints in every TSX file | — |
| 6 | Positional slots continuing past a pattern formal, and pattern locals as parameter names or seeds | not changed: both have readers outside this lane (two review algorithms, the reasoning layer's seeds) | follow-ups F7 and F8 | — |
| 7 | Props through wrappers other than React's `memo` and `forwardRef` (state-management or styling wrappers that pass props through, such as `observer(fn)`, `styled(fn)`) | refuse and count (`refused_unknown_wrapper`, 5.3 check 3) | an allow-list of pass-through wrappers proven by import identity is one table and its rows (follow-up F9); revisit with the census count | after the census; any time before F9 |

## 9. Follow-ups, each with the number that would justify it

| Id | Follow-up | Justified when the census shows |
|---|---|---|
| F1 | Whole-object sources: a spread `{...x}` in an element or literal, and an identifier passed where the callee has a pattern formal (`D(obj)`) | spread attributes on ≥ 5% of JSX sites with attributes, on either corpus |
| F2 | `children` as a prop | ≥ 100 components that read `children` and are given an identifier or member expression as a child |
| F3 | Bare reads of a plain formal with a zero-width Use (`const { a } = props` and the other #594 zero-width positions), which needs a line-keyed node rule on the callee side | `refused_zero_width_read` sites ≥ 10% of admitted sites with a plain target |
| F4 | Resolution of `React.memo(Identifier)`, `export default memo(..)`, compound components | ≥ 10% of capitalised JSX sites unresolved or refused for these reasons |
| F5 | Trace-engine descent across edges that end at a formal's member read, and descent where the tag's spelling differs from the target's name (match the site through the caller's own sites and `resolve_call_site`, not through the name-keyed index) | plain-formal targets ≥ 10% of bound attributes, or bound JSX sites whose tag spelling differs from the target's name ≥ 10% of bound JSX sites; and PR #841 merged |
| F6 | `href` and `src` sinks; `useState` and `useContext` provenance | after this lane's review examples show reader value |
| F7 | Positional binding of identifier formals that follow a pattern formal | ≥ 100 such call sites on a corpus |
| F8 | Pattern locals as parameter seeds for `taint_reaches` and as parameter names for the algorithms that read them | a user request, or a seed-based measurement that needs it |
| F9 | Props through pass-through wrappers on an allow-list proven by import identity (owner decision 7) | `refused_unknown_wrapper` ≥ 10% of Exact-singleton JSX sites with attributes, on either corpus |

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
  the resolved signature. The known wrong targets are refused before binding: member elements whose
  qualifier is not a namespace import (check 2), and targets reached through any call other than an
  admitted React wrapper, the `React.lazy` loader arrow among them (check 3). The resolver defect behind
  check 2 (a wrong Exact target for member calls and member elements through an imported value) is filed as
  its own issue at close-out; fixing it belongs to a resolution slice with Tier-A.
- **A source node that the caller's definitions do not reach gives a disconnected edge.** The 5.3 rule takes
  the node the line's def-use edges end at; the `path` rows and the end-to-end connected share check it.
- **Opaque values dominate some attributes** (callbacks, inline objects from calls). They are counted, not
  bound; the census decides whether a later rule is worth it.
- **Accepted limits, each counted:** a bare read of a plain formal with a zero-width Use receives nothing
  (F3); name-keyed trace descent (5.6, F5); two sites of one component on a line (5.6); same-line
  propagation in one-line components (5.6); the element-context members stage D accepts as losses (5.3).
- **Out of model:** props added at run time (`cloneElement`), what an unadmitted wrapper does to props (it
  is refused, not modelled), `defaultProps`, and render timing.
- **Review taint across files** is the subject of #602 and PR #841. This lane changes no taint code;
  end-to-end checks therefore use `taint-reaches` as well as review taint.

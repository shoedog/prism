# Lane 3: React value flow across JSX props — design

Status: Stage D design of record for the 311-row matrix at `dfe75ad226de`. Implementation and corpus acceptance are pending. The Stage D decisions and evidence are in sections 5.1–5.9.
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
| A JSX opening or self-closing element is an ordinary call site with `jsx_element = true`. Its span covers the whole opening element. Lowercase tags are dropped as intrinsic. Capitalised tags resolve like calls: same-file `local_def`, imports `import_member`, Exact in the probes. A member tag `<Q.M>` resolves `import_qualified`, Exact, whenever `Q` is any imported binding: through the namespace proof when `Q` is a namespace import (the site's `local_binding` is `JsLocalBinding::NamespaceImport`), otherwise by the module's file stem and the member's spelling. So `<Ctx.Provider>` with `import { Ctx }`, and `<Menu.Item>` with `import { Menu }` (named or default) where the module holds `const Menu = { Item: Impl }`, bind to whatever function of that module is spelled `Provider` or `Item`, a wrong target; the same rung gives `api.send(a)` a wrong Exact argument edge. A namespace import is not enough either: the namespace proof matches the target against the module's export of that name (`CallGraph::js_ts_namespace_exports`, built by `resolve_js_namespace_exports` in `src/ast/js_ts/exports/`), and when the module has no such export the proof returns nothing and the stem-and-spelling match above decides, so `<UI.Item>` with `import * as UI from "./table"` binds Exact to an object member `Other.Item` or an unrelated `Item` of that module. A local `const` object, `props.X` or `this.X` qualifier gives `UnknownName`. | `src/queries.rs:150-166`, `src/call_graph.rs:176`, `src/resolution.rs:2158`, `:2372-2420`, `src/resolution/js_ts/namespace.rs` (`js_ts_namespace_proof`); probes `res2`, `res3`, `k1` of the reviews |
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
component-like definitions with a destructured first parameter 194, with a plain identifier 36. The subsequent X census proved zero member exports matching the target on its pinned build. It classified three non-test target routes as other wrappers. F9 counts only otherwise eligible sites with attributes (5.7).

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
rendered component (a non-namespace member qualifier, a namespace member that is not the module's export,
an unadmitted wrapper); a second copy of the
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

For the caller-side JSX source, use the first-wins line index with an incoming-edge and binding-identity proof (5.3.4). An exact-span-only lookup can pick the disconnected value token in `title={title}`. Changing the producer to link every occurrence would change reaching-definition labels outside this lane. The binder can retain a connected line anchor without either change.

## 5. Architecture

```
            props source                              props target
object-literal argument ─┐                     ┌─ plain formal: its member reads  (edge ends at a Use)
                         ├─► entries ─► bind ──┤
JSX element attributes ──┘   (ordered)         └─ pattern formal: binding elements (edge ends at a Def)
```

### 5.1 Pattern formals

**5.1.1 — Syntactic positions and admission.** Walk each JS-family callable's own formal list, in order, ignoring comments, types and TypeScript's pseudo-parameter `this`. Record an object pattern at its runtime argument position even if it is empty or has no eligible leaves. A root array or a rest *formal* destructures a different value and supplies no object-pattern target. An ordinary identifier after a pattern keeps its existing positional hole. Python and Go use their existing producers. The new `js_ts_pattern_params` accessor is private to data flow and pattern-target discovery. Other consumers of parameter names, occurrences and slots keep their current inputs. Examples include p07, p11, p18–p20, sol-1-01, sol-1-03, sol-1-04, astra-2-04 and c-positional-stop.

At each object level, descend through shorthand, renaming, defaults, nested objects and object rest. The property path is a vector of keys; the local's identity is its binding identifier token and byte interval. A computed key can introduce a local with an unknown path. An array subtree introduces no new local here. For a binding to qualify, its unescaped identifier must be unique among **bound names** of the entire formal list, including excluded subtrees. Property names in renames do not compete. Recovered lists supply no new facts. An escape that may alias a bound name precludes a uniqueness proof. Preserve the producer's owner and scope guards.

Run the existing `js_ts_param_pattern_alias_target` predicate on the original syntax first. A whole-pattern plain-identifier default that it owns is present but `alias_skipped`, with no new elements or Defs. Preserve the existing local and alias twin, including their byte identity. Otherwise every default from the formal through the leaf must pass `is_inert_default_value` after peeling parentheses and erased `as`, angle assertion (where legal), `satisfies` and non-null wrappers from the **value**. The allowlist is number, string, boolean, null and genuinely empty object/array literals (comments allowed, holes excluded). A failure excludes that subtree. It does not exclude siblings or make its property available to rest. Do not reclassify a wrapped identifier as a legacy alias. See p03–p04, p12–p13, p28–p31, p34, terra-4-04, sol-1-02, sol-2-03, astra-1-03.

**5.1.2 — Definitions and reference identity.** Register one used, eligible local at the callable's start line with the local's actual token bytes, even for multiline formals. Use data-flow owner identity, including synthetic callbacks. Find *value references to this binding* in its parameter and body environments. Include reads in computed names/defaults, callable tag names, constructors and captures. Exclude type positions, labels, property-name tokens, `new.target`, and reads resolved to shadowing declarations (including hoisted declarations, switch/loop scopes, catch bindings and private function-expression names). Pattern-specific filtering at admission must not change ordinary identifiers' output. A write to an admitted formal uses the existing identifier write/kill machinery. A local used only through members gets the base Def and its existing member projections. A truly unused local gets no Def. This does not require removing existing attribute-name Use nodes from the CPG.

Use the identifier twin's reaching definitions, labels, member writes, `var` handling, direct/indirect `eval`, and ownership of `arguments`. Use a non-simple identifier list for a control that depends on mapped arguments. Preserve the distinction between an absent edge, a killed edge and an edge from a *later* Def with the same name. p15's unconditional assignment replaces the formal. In the recorded TSX twins p23's entry edge is `cfg_incomplete` and p27's is `killed` (the captured write supplies the `cfg_incomplete` edge). p35 retains its documented one-node sibling relabel. A Def alone is not proof of a Use on its line (p21, p33, j31). See also p14–p17, p22–p27, astra-1-01/02, astra-3-01/02/03, astra-5-01/02 and sol-4-02/03.

**5.1.3 — Consumed keys and rest.** At each pattern level record *all* definite sibling source keys and whether any sibling key is unknown, independently of Def admission. A nested pattern, an empty object/array, a rename, an array subtree, an unused local and a refused default each consume their entire named property once. Repeated keys may bind multiple different locals. A rest entry carries its parent path and the level's exclusion set. It accepts a leaf strictly below that parent whose **first remaining segment** is outside the exclusion set. A parent-only value does not prove any individual residual key. An unknown sibling disables rest only at that level. An unknown ancestor makes descendant entries unknown. Known siblings still bind; a nested unknown does not disable an outer rest whose sibling's name is known. See l23, terra-2-06/08, terra-4-02, terra-5-01/02/03/04, astra-1-17/18, astra-2-04/05, sol-1-14.

The AST returns per-position state, per-level consumed keys and source order, plus local name/bytes, path or unknown, residual predicate if any, default flag and admission for each element. The CPG substitutes an optional graph target for the local, keeping those predicates even when the target is absent. Resolve an admitted callable to its unique syntax node and binding Def by file, callable span and local token bytes. Use identical coordinates in cached and direct discovery. This includes the anonymous first argument of an admitted React wrapper (j05). A same-named function, or another Def on the same line, cannot substitute.

### 5.2 Props sources

**5.2.1 — Literal keys and values.** Parse the direct object argument after the transparent wrappers of 5.1.1. A source is an ordered **tree of writes**, retained until overwrite decisions at that level are complete. Each definite key is one segment: unescaped ASCII identifier/shorthand spelling or the entire contents of an unescaped ASCII quoted key. Empty string, `'a.b'`, `'0'` and `'aria-label'` each occupy one segment. Compare vectors, never strings joined by dots. On the pattern side a bare nonnegative safe decimal integer with canonical digits also names its decimal string. Other numeric pattern spellings are unknown. A numeric literal **source** is never a leaf. Treat its write as opaque at a known canonical safe-decimal key, otherwise as a possible overwrite of earlier numeric keys (conservatively, any segment starting with an ASCII digit, or `Infinity`). Thus `1e0` can replace `'1'`, while an ordinary following field survives. A computed, escaped or non-ASCII source key is a wildcard, including on methods; the corresponding pattern key is unknown. No escape decoding or computed-constant evaluation. See l20, sol-2-06, sol-3-03, terra-2-07, terra-4-05 and astra-1-18.

A shorthand or a statically spelled identifier/dotted member value is a leaf with its operand bytes. Peel transparent wrappers where they wrap an operand, including within a member chain. Optional dot segments keep their path for the evaluated branch; preserve private `#` and receiver identity. Subscripts (including quoted indexes), calls, function/method/accessor values, arrays, conditionals and other expressions contribute no leaf. Literal object values recurse. Accessors, methods and `__proto__` (even shorthand) can determine the final writer but cannot supply a bound value. Reject recovered literal subtrees conservatively; on a recovered ordinary literal preserve #594's whole-literal refusal. Inspect nested call/element sites independently of whether their value supplies a leaf to a parent. Incoming edges describe syntactic initializer provenance; they do not simulate getters mutating siblings, generator resumption, heap mutation or abrupt completion during binding (astra-3-14, astra-4-03/04).

**5.2.2 — JSX entries.** Read only the attributes of this opening or self-closing tag, in byte order. A named expression uses 5.2.1's value grammar, including object literals. A quoted JSX value or bare boolean is constant. An element-valued attribute (braced or unbraced) is opaque; its inner site is independent. A namespaced attribute and `__proto__` are opaque named writes; a non-ASCII attribute name uses the wildcard rule. Top-level, case-sensitive `key` and `ref` are skipped, without suppressing evaluation of their expressions or nested sites. These spellings in a literal, including a nested literal in JSX, are ordinary keys. Spreads are unknown writers even when their operand is an inline object.

An explicit `children` is an ordinary attribute unless the paired element has an emitted child. Use JSX text's line trimming/joining semantics: a same-line space survives, an indentation-only multiline node does not. Empty/comment-only expression containers contribute no child. A nonempty expression (including one that evaluates to null), text surviving normalization, element, fragment or spread child supplies a final overwrite of `children`. A spread child conservatively counts as content regardless of emit mode or runtime arity. Child content itself supplies no props source. See j08–j14, j17, j20, j38, sol-1-13/15/24, sol-4-09/11/12, astra-1-26 and astra-4-11.

**5.2.3 — Last writer.** Decide a parent's winning write before visiting its leaves. A later write of the same key replaces the whole earlier subtree, including when the later value is empty, constant, opaque or an accessor. A spread/wildcard masks preceding candidate keys at its own level; later named entries survive. A nested wildcard cannot mask another top-level prop. A spread never supplies its own source. No traversal of an overwritten value on behalf of its parent may suppress a separately recorded call/JSX site inside that value. See l07–l09, j06–j07, j40, sol-1-09/12, sol-3-04, sol-4-04, terra-4-01 and astra-2-21.

**Compatibility projection.** For ordinary calls into plain formals retain #594's original writer/leaf visibility and exact-span source algorithm, including for a bare read. Simply exposing a newly representable `'aria-label'` leaf to a bare `wrap(p)` would change existing ordinary-call edges. Carry legacy key eligibility on each segment/writer or use an equivalent projection of the same tree. Partial or full cache hits and both test twins must use it too. The richer key domain serves pattern targets and the new JSX source.

### 5.3 Binding

**5.3.1 — Site and target admission (numbered checks for 5.4).**

1. Start from a recorded JS-family source site, with at least one JSX attribute or a direct object-literal argument, and the whole existing resolution result. Require exactly one Exact JS-family callable (`refused_not_exact_singleton`). No record means no site and no counter. Preserve intrinsic drops for unqualified lowercase, hyphenated and colon tags. Qualified lowercase members and `_`/`$`/non-ASCII callable names use the ordinary resolver. TS overloads bind to a body, erased declarations supply no function. A type assertion on a component value cannot select the asserted signature's declaration.
2. For member JSX and for *pattern positions* of member literal calls require a single simple qualifier that is a value namespace import (`refused_member_qualifier`), then prove that module's export under the spelled member equals the resolved file, local name and function span (`refused_member_not_export`). Reuse `js_ts_namespace_proof` and its module lookup read-only. A re-export must meet the same comparison. Matching the export is an additional test on the singleton already returned. It cannot manufacture a candidate the resolver's name and module filter excluded. Plain positions of ordinary member calls keep #594.
3. For JSX locate the unique resolved callable syntax node. If the function is an argument of a call (through parentheses), admit only the first function operand of a program-scope `const` initializer (possibly exported) that `js_ts_wrapped_export` proves is React's `memo` or `forwardRef`. Compare the function node/bytes as well as its registered identity; a comparator cannot receive these props. Other call parents are `refused_unknown_wrapper`, including a local wrapper and hooks. No candidate at check 1 takes precedence even when the syntax looks like a wrapper (j36). Use the span-selected anonymous function's first formal for j05. For a direct function require the site's selected lexical value to identify that function, using the callable/import/alias identity and matching file and syntax span. Compare the value namespace in the site's evaluation environment (including parameter defaults). A same-block reference before a const initializer, an erased import, a reassigned or parameter-shadowed spelling with no unique callable proof, a missing body or ambiguous syntax identity fails `refused_target_identity` if check 1 has passed. A hoisted declaration can pass. Direct member sites already have the export identity proof from check 2. This check can only veto the selected target.
4. JSX selects position zero. A literal selects its safely positional argument (5.3.3). An absent/unsupported formal at that position is `refused_no_props_formal`, a whole alias-skipped object pattern is `refused_alias_pattern`. A present empty or fully leaf-refused object pattern is still a formal.

j03–j05, j15–j16, j34–j36, j39, l19, l21–l22 and astra-1-22 cover these distinctions. Apply checks 2–4 independently to eligible positions at a literal call. A refused pattern position must not erase the existing plain position or another admitted pattern position. Resolve a site only once. See 5.4 for mixed-site accounting.

**5.3.2 — Matching and graph identity.** For a surviving literal/JSX leaf, one key path matches an ordinary pattern entry when either vector is a prefix of the other. Emit `Use(value) → Def(local)`, Exact, to each matched, uniquely identified Def. Rest uses only 5.1.3. An unknown path or missing Def receives nothing. A deeper source feeding a shorter ordinary pattern path enters a container. Projection from that Def or a rest Def can carry a sibling field downstream. Section 6 excludes such mismatched hops from credited chains.

For a plain target, take reached reads and flow-insensitive write suppression from #594, including a computed write to the formal and prefix/deeper member-write suppression. Use the worse of Exact and the callee's definition-edge label, retaining kill information. A member read must have a comparable path. A reached bare read with proven operand bytes takes every visible key (a bracket read can be bare); a zero-width bare *target* takes none. Member reads can still bind at that site. No edge goes to the plain formal's Def. Dedupe final emitted props edges by concrete `(from NodeIndex, to NodeIndex)`, including multiple sites of the same caller, in stable site/argument/source order. Different byte-specific nodes with the same displayed spelling remain different. Recursive calls can bind within the same owner; classify by site witness and endpoints, not inequality of owner names. See l01–l06, l16–l18, l24, j02, j12, j19, astra-1-07/08, astra-2-08/09 and astra-5-04.

**5.3.3 — Literal sources and position.** Use #594's byte-contained Variable lookup for the full path and its base. It can select a disconnected exact occurrence (sol-1-08); this contract is specific to ordinary calls. Only direct literal arguments qualify, including through the transparent wrappers. A preceding *argument* spread makes the literal's position unsafe, regardless of the spread operand or whether a formal exists at the syntactic index. Refuse this literal (`refused_argument_after_spread`); earlier safe arguments remain eligible. An object *property* spread instead uses 5.2.3. Do not unwrap `call`/`apply`, setters, tagged templates, conditional arguments or whole-object identifiers into new calls/positions. See l13–l14, sol-1-04/05/06, sol-4-05, astra-3-04/05 and astra-5-03.

**5.3.4 — JSX source proof, for every statement context × owner.** Take the graph after S1's local Defs/projections and ordinary intra-owner flow have been assembled, before Step 5b adds props or ordinary argument edges. Prove a path expression at the surviving leaf's operand bytes. Identify the source site's **execution owner** from its recorded caller and syntax; callable bodies and parameters, enclosing class-definition expressions and private names must keep their own binding environments. A function that happens to contain the line is insufficient. Apply the following decision to each leaf independently.

1. Use the full structured path first, then its base for a member. For each path search the operand's value-token lines in source order, looking up the first-wins Use under (file, caller name, caller start line, value line, path, Use). Consider only the actual operand, excluding surrounding type syntax. A candidate must be the concrete destination of an existing intra-owner DataFlow definition/projection edge for this value's binding. Incoming label need not be Exact. Any path used as a fallback must pass the same proof. Emit from the **first proven** candidate only. A full projection beats a base read; a disconnected projection does not hide a connected base (j10, sol-3-10, sol-4-08/09).
2. A real value occurrence, an earlier representative token and a zero-width line anchor can all qualify. For a representative other than the leaf's own occurrence, prove the line represents the same binding and value version: collect reads of that path/root in the owner's evaluation region on the line using the scope walk, ignoring JSX attribute-name/property tokens as reads. Require one binding identity and no intervening write/update to the root or overlapping member path between the represented read and the leaf (for an anchor, across the possible represented reads). For every candidate, including one at the operand bytes, require the contributing Defs to denote the operand's lexical binding. A lower-confidence edge from a different binding still contaminates a raw graph path. A proved conflict makes the representative ambiguous. Distinct possible versions from control flow on earlier lines keep the producer's labels. A name-token representative can stand for the proved read in `title={title}`. A read in another arm with the same binding and no intervening write can stand for it as well. If owner identity collides, bindings differ, or an intervening write makes the line representative stale, refuse the leaf. Do not try another byte occurrence or a coarser base to escape a failed identity proof. See j21, j25–j28, terra-1-08, astra-5-11, sol-2-14 and sol-3-08/09.
3. If there is no proven candidate, emit no edge. Count **once per leaf**, in order: a detected identity/version conflict (`refused_source_ambiguous`), any caller candidate with no qualifying incoming edge (`refused_source_unconnected`), a Use for the same binding and value-line path only in a lexically enclosing callable (`refused_source_in_enclosing_owner`), otherwise `refused_source_not_found`. The enclosing lookup is diagnostic. Never mint a signature-line Use, jump from the Def in its place or use another owner's node as the edge source. A disconnected callback Use takes precedence over a connected enclosing Use. No incoming props edge may bootstrap the proof. A local field/parameter may gain its ordinary Def-to-Use edge in S1, so the base census is not the acceptance oracle.

The required outcomes for mixed and lost context rows are in 5.7. All other positive context rows require the connected caller binding by this same rule, including zero-width declarators, independent nested JSX, callback-owned formals, accessors, generators, switch arms, loops and `try`/`finally`. No dispatch by statement kind. The open-class generator in 5.8 exercises both successful and failing source evidence in each class.

**5.3.5 — Graph paths and trace.** A binding assertion joins the caller's definition to the emitted source by concrete node identity. For pattern targets assert descent where the existing trace can select this one Exact site by caller, callee lookup name and line or containing span. For a name mismatch or two matching sites require the graph path and trace boundary. A plain-target edge ends at a Use, where trace stops. Preserve inherited same-line sibling propagation and separate syntactic origin from execution order. See j01, j18–j19, j35, j37 and l12. Keep the trace engine's existing traversal.

### 5.4 Counters

`dfg_props: BTreeMap<String, usize>` is assembled with Step 5b on cold, partial and full cache hits. Production and both test twins count against the same immutable input graph, return per-caller facts and merge them in deterministic order. Site identity includes the caller and site bytes. Count site inventory before iterating resolution targets, including a recorded site resolving to none.

| Keys | Unit and counting rule |
|---|---|
| `sites_jsx`, `sites_literal` | Once per recorded JS-family JSX site with attributes or ordinary site with at least one syntactically direct literal argument. A later unsafe argument spread decision does not change the inventory. |
| `refused_not_exact_singleton`, `refused_member_qualifier`, `refused_member_not_export`, `refused_target_identity`, `refused_unknown_wrapper`, `refused_no_props_formal`, `refused_alias_pattern` | One admission reason at most per site. Check 1 short-circuits the entire site. At a mixed literal site, bind every admitted safe position. If any position is refused by checks 2–4, count the earliest failed check among those positions once, breaking ties by argument index. A qualifier refusal on a pattern position can therefore coexist with an emitted plain edge (astra-1-22). |
| `refused_argument_after_spread` | Per direct literal position following an argument spread at an Exact site. Such a position does not participate in formal admission. |
| `refused_zero_width_read` | Once per admitted site with any reached zero-width bare plain-formal read, independently of successful member/other reads. |
| `spread_sources_unbound`, `refused_overwritten`, `refused_value_not_data` | Per admitted source position/JSX opening. Count each spread evaluated by the source tree once. Count each displaced leaf/terminal once, including the leaves under a replaced parent (j40); never count one leaf again for a second masking write. Count each surviving constant/opaque named terminal once. An empty object is a parent write with zero leaves. Skip reserved attributes before entry counts. Unknown writers mask without supplying a named leaf. |
| `refused_key_unknown`, `refused_key_not_read` | Count each unknown pattern entry (including an indeterminate rest) once per admitted position, irrespective of the number of leaves. For each surviving data leaf with no eligible matching destination, count one `refused_key_not_read`. This includes a present empty pattern. |
| `refused_source_ambiguous`, `refused_source_unconnected`, `refused_source_in_enclosing_owner`, `refused_source_not_found` | Per surviving JSX leaf with a matching destination, one reason in 5.3.4 order. No multiplication by candidate paths, lines, reads or locals. A failed literal byte lookup emits nothing, as in #594. |
| `edges_to_def`, `edges_to_rest`, `edges_to_use`, `edges_to_bare_use` | Disjoint counts of distinct emitted props edges to non-rest pattern Def, rest Def, member Use or bare Use. Count once even with several witnesses. `edges_to_container` is the subset of `edges_to_def` with at least one deeper-leaf witness. In measurement, reconstruct the distinct syntax witnesses/restrictions even when two keys share endpoints. |

### 5.5 Where the code goes

```
src/ast/js_ts/pattern_params.rs    pattern levels, consumed keys and binding spans (declared in src/ast/js_ts.rs)
src/ast/js_ts/component_route.rs   admission check 3: direct, admitted wrapper, or other wrapper
src/resolution/js_ts/namespace.rs  admission check 2's export proof: a read-only query reusing
                                   js_ts_namespace_proof, reached through CpgCallResolver; no outcome changes
src/ast/parameters.rs              unchanged accessors; the JavaScript path gains nothing here
src/data_flow/build/file.rs        registers Defs for admitted pattern locals beside identifier formals
src/cpg/callee_params.rs           compute_pattern_formal_nodes and the cached ParamTarget form
src/cpg/props_binding.rs           ordered writer tree, shared binder, admission and counters
src/cpg/object_literal_args.rs     becomes the object-literal source; pairing moves to props_binding.rs
src/cpg/jsx_props.rs               JSX extraction, connected caller lookup and ambiguity/refusal classification
src/cpg/build.rs                   module lines (`#[path]`, like callee_params and object_literal_args), the
                                   call for JSX sites, the counters' merge, the `dfg_props` field. The file
                                   is at 1,777 weighted lines: wiring only.
src/cpg/callee_legacy.rs, src/cpg/step5b_reference.rs   the same calls at the same position, so the parity
                                   tests keep agreeing
src/cpg.rs                         `#[cfg(test)] mod` lines for the new test files
```

Each slice that changes the graph or the statistics record bumps `CACHE_VERSION` (189 on main today) and
adds its line to the list in `src/cpg_cache.rs`.

The four implementation slices have separate boundaries. S1 supplies pattern facts and registers local Defs in the existing producer. S2a moves the existing #594 binder without changing its behavior. S2b consumes the S1 facts, models ordered literal writes, and queries the existing resolver proof. S3 reuses that binder and the pre-step-5b graph to select JSX sources. The generators in 5.8 exercise combinations at those boundaries. They require no separate resolver or data-flow implementation slice.

### 5.6 Effect on the taint engines

- Default review taint follows both edge kinds.
- The trace engine can cross a pattern-formal edge when its Def and caller Use satisfy 5.3.5. Match the site's own line or source bytes contained in the opening span, including on a continuation line. It still stops at
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

### 5.7 Accepted losses and their census units

All public numbers below are non-test, in the order **Excalidraw / cal.com / apache-superset / apache-answer**. M used Prism `9d1a7d06f653` and TypeScript 5.9.3, before S1. A leaf's first failed prediction rule, all candidate leaves of a source class, sites and distinct components are different units. The figures below are baseline observations and predictions. S1 can give a previously disconnected pattern-bound caller a Def and a reached Use; measure the residual on the S1 graph and again on the implementation head.

| Lost flow or precision | Row decision and count |
|---|---|
| No caller Use, including signature-line reads | **j31 loses** the props edge; p33 still defines `title`. sol-2-13 and sol-5-05 bind their split forms and lose the compact signature. sol-5-06 loses the iteration-local default and binds the outer-formal control. astra-5-08 loses both catch defaults and binds the body-local control. sol-1-22 accepts no edge for module initialization, non-callable field and static block in its three fixtures. `not_found` all leaves **82 / 275 / 1,127 / 2**; first failures **65 / 70 / 199 / 0**. The explicit signature-miss cell has **0 / 7 / 14 / 0** leaves, **0 / 3 / 7 / 0** first failures. The exact loop/catch/initializer fixtures are not measured separately. Recover those AST regions at the attribute bytes and group `source_miss` by region. |
| Enclosing-owner-only value, or disconnected capture with its own caller Use | **j33 expression loses** (enclosing only); **j33 block loses** (caller Use unconnected). Do not use the connected enclosing Use for either. astra-1-20, astra-4-07 and astra-4-09 lose the disconnected capture only; their locals and the method control bind. In astra-2-10, astra-3-06 and astra-5-05 the callback's local leaf binds, the enclosing capture loses. The last row establishes no capture path without requiring a second, disconnected node to exist. All enclosing-only leaves **12 / 123 / 588 / 1**; first failures **7 / 26 / 137 / 0**. A capture-specific split of disconnected block Uses is not measured. Join the operand's lexical declaration owner to the caller and group by `owner_kind` and leaf outcome. |
| Caller Use exists but has no incoming definition/projection | sol-1-23 loses the catch-value leaf; the shadowed outer formal supplies nothing. astra-5-06 loses both receiver getter leaves and binds `note`. Full-path and base candidates compete per leaf (j10, sol-4-09). All disconnected leaves **664 / 1,871 / 2,534 / 474**; first failures **439 / 687 / 365 / 48**. Of the private-corpus first failures, **655** were source-unconnected. The catch and receiver subtypes are not measured. astra-1-14, astra-2-11 and astra-2-20 are positive post-S1 obligations for the explicit pattern-bound value, not permanent losses inferred from M. The spread relay in astra-2-20 still stops. |
| Ambiguous line representative or colliding owner identity | sol-2-14 and sol-3-08 lose the compact shadow/write case and bind the split control. sol-3-09 loses both sites: the call graph skips identical named owner IDs before emitting sites; byte-distinct local edges remain. Measure identity conflicts by comparing candidate and operand binding spans/versions in the S3 matcher; measure missing sites by syntax-to-site join. Neither exact count was measured. Earlier-spelling *connected* leaves **251 / 1,180 / 1,409 / 185** include safe cases like j25; blanket refusal would discard good paths. |
| Unresolved component, imported-value member, export or wrapper proof missing | First failures: not Exact **514 / 3,809 / 7,230 / 1,319**, member qualifier **1 / 10 / 18 / 0**, unknown wrapper **0 / 52 / 12 / 0**, no formal **1 / 5 / 0 / 0**. The private-corpus not-Exact cell is **913**. Namespace qualifiers lacking a matching target export include **0 / 113 / 0 / 0** sites in M's qualifier inventory. sol-2-16, sol-4-13, astra-2-15 and astra-3-08 lose the default/renamed/barrel positive variants. sol-4-10 loses the specialized alias only. Their exact subtype counts are not measured. Join the syntax's export/alias declaration to the recorded target pool and compare terminal identities. Capitalised elements lacking a joined site: **0 / 49 / 13 / 0**. **j05 binds** by callable span: the anonymous-in-admitted-wrapper target inventory is **132 / 67 / 55 / 0**, with **101 / 6 / 36 / 0** predicted pairs from those targets. |
| Plain target destructured in its body or other zero-width bare target read (F3) | l16, j24, astra-1-08. First-failed leaves **8 / 148 / 52 / 17**. F3's otherwise-eligible site counts are **4/73, 53/306, 34/101, 15/16**. Private-corpus first failures: **1,997** zero-width reads, the dominant refusal. The source-side zero-width rule does not repair this callee loss. |
| Spread, overwrite, child content, reserved or opaque values (F1/F2) | Spread sites **37 / 845 / 463 / 22**. First failures for later spread/wildcard **1 / 94 / 38 / 0**, reserved attributes **25 / 71 / 40 / 3**. Distinct components reading a path-valued child **12 / 28 / 8 / 0**. Counts by `value_shapes` include call-valued attributes **449 / 2,048 / 1,734 / 212** and function-valued **388 / 1,315 / 1,051 / 246**. An opaque or overwritten parent does not cost a nested recorded site. Separate counts for spread-child syntax, getter effects, inherited keys and bracket-valued attributes are not measured. Subdivide the source syntax at the attribute/argument bytes. Runtime spread arity has no static census cell. |
| Excluded pattern or literal shapes and lost positions | Unknown keys/rest, non-inert or alias defaults, array patterns, rest formals, unused/refused locals, unsafe spread positions, opaque arrays, whole-object calls and unread keys use 5.1–5.3. First-failed nonmatching keys **6 / 20 / 18 / 0**. F7 counts ordinary calls supplying an identifier formal after a pattern: **50 / 10 / 9 / 1**. Alias-default and complex-default formals have overlapping shape inventories in M; loss per rejected binding or ordinary call is not measured. Use the pattern facts and S2 position/leaf counters on the corpus. Ordinary exact-span literal sources can be disconnected; this JSX census did not measure them. |
| Container/rest or bare-read coarsening, inherited graph/trace limits | l24, sol-3-07 and astra-4-03/04 pin static projection/initializer limits. Bare whole-object and container/rest edges can reach a different field; use `paths.py` witness restrictions. Flow-insensitive member writes can also lose pre-write reads. Those precise path losses are not measured. F5 plain-target bound attributes **36 / 125 / 35 / 2** and differently **spelled** bound sites **6 / 3 / 9 / 13** are proxies, not observed trace boundaries. j19's multiple matching sites and wrong-key downstream paths have no measured count. j37's one-line sites with two known keys: **0 / 0 / 0 / 0**. Sibling flow, conditional evaluation, exceptions and inherited killed/uncertain CFG labels remain visible in raw outputs; no claim of render-time execution follows. |

For the callback rows, M also has narrower context cells. The following counts filter non-test capitalised tags in `attributes.jsonl`, use nearest `context`, and pool ordinary and JSX-hosted callbacks of the indicated body kind. They include every resolution level. Each cell is **unconnected / enclosing-only / missing**. They give context for a row; they do not identify the row's exact runtime subtype.

| Context and representative losses | X | cal.com | Superset | Answer |
|---|---:|---:|---:|---:|
| Block return (j33 block, astra-1-20) | 32/0/0 | 133/0/7 | 238/0/0 | 24/0/0 |
| Block JSX child (astra-4-09) | 68/1/0 | 93/0/0 | 218/1/13 | 58/0/0 |
| Expression-body nested attribute (astra-2-10) | 0/0/2 | 0/9/0 | 0/6/4 | 0/0/0 |
| Expression-body guard (astra-3-06) | 0/1/0 | 0/2/9 | 0/54/37 | 0/0/0 |
| Expression-body JSX child (astra-5-05) | 0/5/11 | 0/40/88 | 0/127/119 | 0/0/0 |
| One-line expression-body callback (j33 expression) | 0/0/0 | 0/11/7 | 0/0/12 | 0/0/0 |

The remaining statement/owner examples require a binding under 5.3.4, including these groups:

- j21–j22, j25–j30 and j32; terra-1-01 through terra-1-08, terra-2-01 through terra-2-04, terra-3-01 through terra-3-05 and terra-4-03. A switch, loop, yield, throw, template, declarator or return does not by itself approve or refuse a leaf.
- sol-1-21, sol-2-07 through sol-2-12, sol-3-10 through sol-3-14, sol-4-06 through sol-4-08. A class heritage expression, computed method name or decorator runs in its defining environment. A callback or field arrow's body has its own owner. Do not borrow the constructor parameter or a same-named outer formal.
- astra-1-07, astra-1-13 through astra-1-16, astra-1-19, astra-1-21, astra-1-24/25, astra-2-11/12/13/17/18, astra-3-07/12, astra-4-08/10/12, astra-5-07/10/11. A represented source in unreachable code or on an uncertain edge still gives a static prop binding. The normal CFG labels remain visible. For mixed rows only the values specified above and in the row can supply a source.

### 5.8 Mechanical coverage of the two open classes

E stopped with new rows in every fifth round. The fixtures are witnesses to rules, not an exhaustive syntax inventory. The following Python generator runs from the repository root. The first family enumerates grammar routes upward from an element through every named parent slot, crossing execution owner with the evidence that 5.3.4 inspects. It includes anonymous and nested owners, class fields and defaults. Supertype transitions are expanded without consuming a syntax level. Increase `depth` to enumerate longer routes, including recursive occurrences of the same production. Syntax context has no voting weight in the decision. Invalid combinations are not positive program fixtures.

```python
from collections import defaultdict
from itertools import product
from pathlib import Path


def parent_slots(language):
    grammar = Path("src/ast/js_write_census/data/signatures") / (language + ".sig")
    supers, parents = defaultdict(set), defaultdict(set)
    for line in grammar.read_text().splitlines():
        parts = line.split("\t")
        if parts[0] == "SUPER":
            supers[parts[2]].add(parts[1])
        if parts[0] == "ALT" and parts[-1] == "named":
            parents[parts[3]].add((parts[1], parts[2]))
    def slots(kind):
        seen, todo = set(), [kind]
        while todo:
            child = todo.pop()
            if child in seen:
                continue
            seen.add(child)
            todo.extend(supers[child])
        return sorted({slot for child in seen for slot in parents[child]})
    return slots


def routes(slots, child, depth):
    if depth:
        for parent, field in slots(child):
            frame = (parent, field, child)
            yield (frame,)
            for rest in routes(slots, parent, depth - 1):
                yield (frame,) + rest


owners = ("module function const_arrow property_arrow named_callback anonymous_callback "
          "jsx_callback class_method object_method getter setter constructor field_arrow "
          "field_initializer static_block generator async_generator").split()
# Evidence and expected result, independent of the statement's syntactic name.
evidence = {
    "connected_full": "full", "full_dead_base_connected": "base",
    "connected_zero_width": "bind", "earlier_same_binding_version": "bind",
    "both_full_and_base_connected": "full", "after_S1_local_projection": "full",
    "shadow_collision": "ambiguous", "intervening_write": "ambiguous",
    "ambiguous_index_owner": "ambiguous", "caller_dead_outer_live": "unconnected",
    "caller_dead": "unconnected", "outer_only": "enclosing_owner",
    "missing_including_signature": "not_found", "no_site_record": "no_site",
}


def context_cases(language, depth):
    slots = parent_slots(language)
    for kind in ("jsx_element", "jsx_self_closing_element"):
        for route in routes(slots, kind, depth):
            for owner, (state, expected) in product(owners, evidence.items()):
                yield route, owner, state, expected


def shape_cases(language):
    # Tokens are source spellings, including the quotes around string keys.
    keys = ["a", "'a'", "'a.b'", "''", "'aria-label'", "'0'", "0", "1e0",
            r"'\u0061'", "café", "[k]", "['a']", "__proto__"]
    patterns = ["K: v", "K: v = 0", "K: v = make()", "K: {}", "K: []",
                "K: [v]", "K: { a: v, ...inner }", "K: { a: v } = {}",
                "K: { a: v } = make()", "K: v, K: w"]
    sources = ["K: x", "K: [x]", "K: {}", "K: { a: x, b: y }",
               "get K() { return x }", "K() { return x }"]
    wrappers = ["V", "(V)"]
    if language != "javascript":
        wrappers += ["V as P", "V satisfies P", "(V)!"]
    if language == "typescript":
        wrappers += ["<P>V"]
    for key, pattern, source, wrap, order in product(
            keys, patterns, sources, wrappers, ("before", "after", "duplicate")):
        entry = source.replace("K", key)
        writes = {"before": "...o, " + entry, "after": entry + ", ...o",
                  "duplicate": entry + ", " + key + ": {}"}
        # Each case supplies an actual formal and an expression for the argument.
        yield ("{" + pattern.replace("K", key) + ", ...rest}",
               wrap.replace("V", "({" + writes[order] + ", keep: y})"))


def pattern_tail_cases(language, depth):
    # Each terminal supplies a real formal fragment. Recursive steps wrap the
    # current subtree and test exclusions at both the inner and outer levels.
    wrappers = ["V", "(V)"]
    if language != "javascript":
        wrappers += ["V as P", "V satisfies P", "(V)!"]
    if language == "typescript":
        wrappers += ["<P>V"]
    def defaults():
        yield ""
        for value, wrapper in product(("{}", "[]", "0", "null", "[,]", "other",
                                       "make()"), wrappers):
            yield " = " + wrapper.replace("V", value)
    def patterns(level):
        yield from ("{}", "[]", "[,v,...tail]", "{a}", "{a: v, b: v}",
                    "{a: v, a: w}", "{...rest}", r"{a: \u0076}")
        if level:
            for child in patterns(level - 1):
                for default, sibling in product(defaults(),
                        ("", ", unused: u", ", [k]: dynamic", ", ...outer",
                         ", skip: [item], ...outer", ", empty: {}, ...outer",
                         ", [k]: dynamic, ...outer")):
                    yield "{box: " + child + default + sibling + "}"
    for pattern in patterns(depth):
        for formal_default in defaults():
            yield pattern + formal_default

```

Run the generated decision vectors against the lookup/matcher units. Use the checked-in fixtures for the full CPG path. Increasing depth enumerates additional grammar routes. For the source proof, generate the structural input graph independently of the lookup: include a wrong-owner node, two byte-distinct same-spelling Uses, a disconnected full path and a connected base. Check exact node identity or the single refusal reason against `evidence`. Pair with the real CPG matrix to exercise site discovery and syntax-to-owner mapping. For syntax tests, generate the formal/argument pairs above and mirror each applicable argument object as a nested JSX attribute value. `pattern_tail_cases` enumerates empty/array terminals, rest siblings, escaped and repeated bindings, plus own and intermediate defaults with wrapper variants. Each recursion adds an object level. Include legacy alias/TDZ and later-formal controls from the matrix when materializing the formal in a callable. The reference test first materializes each level's final writer map, then removes every sibling key for rest. Compare paths as segment vectors, including the empty-string segment, and compare refusal and edge counts. Do not calculate expected outcomes with the production extractor. Syntax that a particular grammar cannot parse exercises recovery, or is excluded from the valid-source run with the grammar reason.

### 5.9 Stage D change log

| Input | Decision and consequence |
|---|---|
| Frozen X prediction 500–850 vs 308, including 439 post-admission unconnected leaves. The endpoint audit found 33 connected zero-width path Uses in X. | A caller Use's existence is insufficient. Require a connected caller-owned source; retain zero-width representatives with a binding proof. Distinguish missing, disconnected, enclosing and ambiguous. Measure again after S1. Compare both graphs under the same first-failure ordering as well as reporting the new counters, since a key miss can precede a source miss in S3. |
| X 308/854 (36.07%); candidate non-test pairs 710, 630, 47. | Select cal.com at 710/1,852 (38.34%). The public census gate passed at **1,018** vs 1,000. M counted predicted pairs. Several can share one emitted edge. The 1,000-edge, 90%-connected and multi-hop implementation gates retain their original thresholds. Earlier-spelling occurrences account for 154 / 388 / 273 / 9 predicted pairs. Recheck ambiguity and full/base selection on the head. |
| Public first formals favor patterns on X (525 object, 73 identifier, 33 absent) and cal.com (831, 313, 160). Private-corpus has 1,518 identifier, 157 object pattern, 115 absent. | Limit the public reach premise to the sampled public sites. The private prediction is **373/3,321 (11.2%)**, with 1,997 zero-width-read and 913 not-Exact first failures. This does not apply a public census threshold to the private corpus. Record F3 volume without adding a callee-side zero-width binder. |
| Anonymous admitted wrappers contribute 101 X and 6 cal.com predicted pairs. | j05 is a required binding. Use the wrapper's first-argument function span to discover pattern facts. Count unproved identity separately; do not equate an anonymous inner function with a missing formal. |
| p15, p23, p27 vs the T1 identifier-twin receipts; signature/callback and lexical-identity rows. | Pin the actual Def and label, distinguish definition registration from use materialization, filter new locals by runtime binding identity, and retain non-simple-formal controls. |
| Empty, repeated, numeric, dotted, escaped, non-ASCII, default and rest shapes; l23, sol-1-09, sol-2-06, astra-1-17/18, astra-2-04/05, terra-5-01–04. | Carry per-level exclusions and parent writes across the interfaces. Expand erased wrappers at every eligible default. Preserve legacy plain-literal visibility, including bare reads. |
| JSX trivia, nested/overwritten sites and literal call positions; sol-1-13/15/24, astra-2-17/21, astra-4-11, astra-5-03. | Define child-content overwrite at JSX emit granularity and preserve evaluation of independent sites. A preceding argument spread has no safe literal slot. |
| j36, sol-2-16, sol-4-10/13, astra-2-15, astra-3-08 and mixed-position astra-1-22. | Admission uses the existing resolution result first. Export proof can only filter, and a refusal on a pattern position preserves the plain position. Count site/position failures in a fixed order. |
| j10, j31, j33, sol-1-23, sol-2-14, sol-3-08/09, astra-1-20, astra-4-07/09 and astra-5-06. | Select one connected, binding-correct representative per JSX leaf with full-path priority. Do not mint a Use, borrow an enclosing owner, or count syntax absent from the site inventory. |
| astra-1-07, astra-2-08/09, astra-5-04 and j19. | Identify recursive props by a site witness even when both endpoints have the same owner; count distinct concrete edge endpoints even when presentation spellings coincide. |
| E stopped after new-row counts 10/8/5/5/4 (Terra), 27/19/17/13/9 (Sol), 26/21/14/12/10 (Astra). | Neither class has a dry-round receipt. Section 5.8 replaces a presumed finite syntax list with generators. The four implementation PRs remain S1, S2a (pure move), S2b and S3. |

Each row has one primary deciding rule in fixture order. The ledger changes 21 behavioral expectations and corrects seven base-color seeds only. The other 283 rows keep their assertions and seeds. The 311-row behavioral inventory reconciles to 25, 44, 16 rows under 5.1.1–3; 16, 22, 16 under 5.2.1–3; and 45, 27, 11, 83, 6 under 5.3.1–5. [Row dispositions](ROW-DISPOSITIONS.md)

## 6. Measurement

**Census (stage M, on the pre-S1 product).** A Node script with the pinned TypeScript 5.9.3
joins `prism nav call-stats --dump-sites` records with the syntax tree and reports, for Excalidraw and one
more public React corpus: JSX sites by resolution outcome; member elements by qualifier kind (namespace
import whose module exports the member, namespace import without that export, named import, default
import, other); targets by route (direct, admitted wrapper, other wrapper);
sites whose tag spelling differs from the target function's name; attributes by value shape and by position
relative to a spread; elements by statement context and owner kind (the axis of PLAN Task T-3); the callee's
first-formal shape (object pattern: flat, alias, default, rest, nested, computed; plain identifier:
member-only, mixed, destructured in the body, other zero-width bare read; none); one-line components with
two or more props; the source-node outcome per Leaf value under M's original line-index lookup (connected: the caller has the
node and a def-use edge ends at it; found but unconnected; in an enclosing function only; not found); and
the predicted number of connected bindable (attribute, key) pairs. The model for the script is
`~/prism-evidence/js-args/census.cjs`.

**Oracle.** The corpora are checked without their dependencies installed, so the oracle uses only what the
TypeScript checker and the TypeScript syntax tree (independent of tree-sitter) can decide from the project's
own files. Check every new props edge, including recursive same-owner hops, against an independently recovered site:

1. Component identity: the symbol of the element's tag name (or of the call's callee), with aliases
   followed, has a declaration that is the edge's target function, or a `const` whose initialiser is an
   admitted wrapper call around it. When the declaration is a `const` initialiser, an object-literal
   property or an expando assignment (`Tabs.Tab = X`) whose right side is an identifier or a function
   expression, the oracle follows that one step before comparing. A member tag whose qualifier is not a
   namespace import is WRONG unless that step reaches the target; a namespace member that the module's
   exports (read by the checker from the module's own file) do not contain is WRONG, not UNDECIDED.
2. Key match, from syntax: the attribute's name (or the literal's key path) equals the property name of the
   binding element the edge ends at, or is comparable with the member chain of the read it ends at.
3. Attribution, from the TypeScript tree: some attribute of an element on the source's line (or of a
   multi-line opening element containing it) whose tag resolves to the target has that key and a value that
   spells the source's path; that attribute is the key's last writer (no later attribute of the same name,
   no later spread); it is not `key` or `ref`; a `children` attribute is on an element with no child
   content. For a literal the same checks apply to the argument's properties. For JSX also verify incoming intra-owner definition evidence for the operand's binding and the absence of a binding/version conflict in any earlier representative (5.3.4). Use TypeScript binding identity and incoming endpoints with bytes, including for a zero-width representative. If identity cannot be decided, record UNDECIDED. Retain the distinct key restrictions of multiple valid witnesses. Verify recursive edges by the site and the formal's binding bytes.

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
attributed to a named class (check literal/JSX site witnesses for recursive edges before classifying a same-owner edge as intra-function), exit non-zero on an unattributed change. The comparator model is
`~/prism-evidence/jsts-lane/783/scripts/compare761.py`.

**End-to-end paths.** From the head graph: chains that cross at least one props edge, by number of
component hops, split into chains ending at a sink line, chains ending at another element's attribute, and
others; each hop must be CORRECT, and a chain that enters a container or rest local by one key and leaves
it through a member read of another key is not counted (5.3). Also the connected share: among bound JSX props edges whose value's root
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

An example gains context only through a line that the head reaches across a props edge (forward: the
head's witness from the changed line to it crosses a props edge; backward: the head's slice reaches it
through one); a head-only line reached without one (for example through an S1 definition alone) is
listed apart and not rated. Both in non-test files, decided with the TypeScript parser on the commit's version of the file, 15 per
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
| Slice S1 | differential: only additions (new Defs for pattern locals and their intra-function edges), plus one expected change class: in a one-node owner, an existing sibling edge moves from `exact` to `cfg_incomplete` when the new local is written or updated in the owner (section 10). No removal is expected, because reference search is already fenced by scope; any removal, and any other label move on an existing edge, is read and explained one by one. No counter outside the data-flow statistics moves. A Go or Python corpus shows no change at all. Binding-identity oracle: 0 WRONG; hand-read sample: 0 wrong |
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
| 8 | One read-only export query added under `src/resolution/` for admission check 2 (5.3 check 2, 5.5: `src/resolution/js_ts/namespace.rs`, reusing `js_ts_namespace_proof`, reached through `CpgCallResolver`; no resolution outcome changes) | yes (**confirmed, owner 2026-11-24**) | the export proof would otherwise be a copy of the namespace comparison outside the resolver | before S3 |

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
  or `cfg_incomplete`, not `exact`; in such an owner, a new local that the owner writes or updates
  (`function C({ p }, safe) {` / `return report(++p, safe, p);` / `}`) makes the write census complete its
  base, the completed bases repeat on the single node, and the single-node admission refuses the owner, so
  the unrelated sibling edge `safe → safe` moves from `exact` to `cfg_incomplete`, exactly as it does today
  for the identifier twin `(p, safe)` and the alias-defaulted `({ p } = fallback, safe)` (review probe
  `k3`); this is the S1 gate's expected change class, pinned by row p35; an owner that gains definitions can change its reaching-definition
  availability counters. The test-only legacy producer reads the unchanged accessors and is not updated;
  tests that compare the live producer with it are adjusted only by excluding owners with pattern formals,
  each exclusion named in the PR.
- **A wrong component target gives wrong edges.** Edges require an Exact singleton, and the oracle checks
  the resolved signature. The known wrong targets are refused before binding: member elements whose
  qualifier is not a namespace import, or whose namespace member is not the module's export (check 2), and
  targets reached through any call other than an
  admitted React wrapper, the `React.lazy` loader arrow among them (check 3). The resolver defect behind
  check 2 (a wrong Exact target for member calls and member elements through an imported value, or through
a namespace import when the module has no export of the member's name) is filed as
  its own issue at close-out; fixing it belongs to a resolution slice with Tier-A.
- **Caller-source identity.** A disconnected or ambiguous JSX source is a refused leaf under 5.3.4. Confirm the shared node on each positive path and measure the share independently on the head.
- **Opaque values dominate some attributes** (callbacks, inline objects from calls). They are counted, not
  bound; the census decides whether a later rule is worth it.
- **Accepted limits (units and unmeasured subtypes in 5.7):** a bare read of a plain formal with a zero-width Use receives nothing
  (F3); name-keyed trace descent (5.6, F5); two sites of one component on a line (5.6); same-line
  propagation in one-line components (5.6); the element-context losses in 5.7.
- **Out of model:** props added at run time (`cloneElement`), what an unadmitted wrapper does to props (it
  is refused, not modelled), `defaultProps`, and render timing.
- **Review taint across files** is the subject of #602 and PR #841. This lane changes no taint code;
  end-to-end checks therefore use `taint-reaches` as well as review taint.

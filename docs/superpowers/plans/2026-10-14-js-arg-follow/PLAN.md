# JS/TS argument following: variant matrix, census and plan

**Status:** plan only, no product code. Branch `plan/js-arg-follow`, base main `b593ad18`.
**Decision it serves:** the owner's JS/TS order is "local aliases, then args, then imports". Aliases (#372) and returned values (#382) are merged. This plan decides what argument following is, whether it is worth a slice, and how to split it.

## 1. Recommendation

1. **Build A1, named targets only, NameOnly, as two small slices** (facts, then the route). On the TypeScript compiler (T) it gives NameOnly targets at about **232 product call sites** that are dropped today (cap 8 targets per site; 460 edges). On Excalidraw (X) it gives only **13 sites**. The X yield is small for a structural reason (finding 2), not because X calls its parameters rarely.
2. **Take finding 2 to the owner before A2.** prism has no call-graph node for an anonymous function (an inline arrow or function expression passed as an argument or JSX prop). So:
   - an inline arrow cannot be a target. On X product code, 74 of the 87 sites where following finds anything reach *only* inline arrows;
   - **no call inside an anonymous function is recorded at all**. On X product code that is 3,275 call nodes; on T, 3,306. In tests it is 21,062 on X and 15,387 on T (`it(() => …)`, `describe`). That includes the 230 X and 246 T parameter calls inside callbacks (`useEffect(() => { onDone() })`).

   That gap is larger than argument following itself, and it is a separate, structural lane: a new kind of `FunctionId` and new records everywhere. If the owner opens it, A2 (anonymous targets) adds about 70 X and 125 T sites on top of A1.
3. **Park `.call`/`.apply`/`.bind`, `props.x()` member callees, rest parameters and `arguments`.** The census finds at most 5 such sites for each form on X and T (`cb.call`/`cb.apply` sites: X 4, T 3; props-member sites: X 3, T 0; `args[i]`/`arguments[i]` sites: 0 and 0). When the caller calls the holder through `.call`/`.apply`/`.bind`, the census counts that as an escape.

The yield is not too small for one slice on T. On X (the React shape) A1 alone is marginal, and the React value depends on finding 2.

## 2. Census (X = Excalidraw `0642e72c`, T = TypeScript `src` at `7964e22f`)

Release build of main `b593ad18` (`nav --no-cache call-stats --dump-sites`). Census: `measure/census.cjs`, sha256 `c54ea500…`, with the pinned TypeScript 5.9.3 checker, static only. Each prism `call_site` record is joined to its syntax node by UTF-8 byte span. Unmatched records: X 6 of 19,140 (0.03%), T 223 of 61,705 (0.36%). Evidence: `~/prism-evidence/js-args/{X,T}/` (`aggregate.json`, per-site `param-sites.jsonl`, `sites.jsonl`).

A **parameter site** is a recorded call whose callee is an identifier bound to a parameter of an enclosing function, the **holder**. The binding may be plain, defaulted, destructured (`({ onClick }) =>`), or an in-body destructure of a parameter (`const { onClick } = props`). `cb.call(…)`/`cb.apply(…)` are included. "Product" excludes `tests/`, `__tests__`, `*.test.*`, `*.spec.*`, fixtures and (T) `harness/` and `testRunner/`.

| | X | T |
|---|---:|---:|
| Parameter sites (all) | 349 | 1,161 |
| … dropped today, `UnknownName` | 287 | 802 |
| … dropped today, `LocalBindingUnproven` | 62 | 213 |
| … resolved today by name search (`free_single` Exact 143, `free_multi` 18 edges) | 0 | 146 |
| … destructured props (destructured, holder rendered as JSX) | 147 | 0 |
| … in-body destructure of a parameter | 5 | 148 |
| Holder exported | 203 | 468 |
| Holder escapes (passed, stored or returned as a value) | 35 | 203 |
| Exported or escaping | 237 | 496 |
| **Closed**: every in-repo call of the holder passes callables, holder not exported and not escaping | 17 | 206 |
| … and prism resolves every one of those calls to the holder | 14 | 154 |
| … and every target is a named function (what prism can target today) | 2 | 64 |
| Parameter calls with no prism record (inside an anonymous function) | 230 | 246 |

**Expected NameOnly yield** comes from the holder's callers that prism already resolves to it (reverse edges), product code, sites dropped today:

| | X | T |
|---|---:|---:|
| A1, named targets, no cap | 13 sites (19 edges) | 247 sites (937 edges) |
| A1, cap 8 targets per site | **13 sites (19 edges)** | **232 sites (460 edges)** |
| A1 target-set size: median / p90 / max | 1 / 4 / 4 | 1 / 6 / 54 |
| A1 + A2 (anonymous targets), cap 8 | 84 sites (137 edges) | 357 sites (801 edges) |
| A1 + A2 set size: median / p90 / max | 1 / 4 / 15 | 2 / 8 / 121 |

The large sets are generic helpers: `core.ts` `map(array, f)` would get 112 targets. That is why the rule has a cap (§3).

The 146 T sites resolved today by name: the census finds no followable value at 143 of them and a disjoint one at 3, all with open callers. The A1 rule never touches a site that resolves today (§3, rule 1). Those name-search edges belong to the refusal slice (#376 item 8).

**Census self-check.** `measure/selfcheck.cjs` runs the census on every `step: "arg"` fixture of the matrix and compares it with the authored truth. 105 of 121 fixtures with a record agree. All 16 disagreements are on the conservative side: the census calls the site open where the truth is closed, through escapes into `.apply`/`.bind`/a local table, a parameter reassigned in the holder, a spread at the caller, or an object-literal method whose caller it cannot see. The census never calls a site closed with targets outside the truth. A reassigned parameter yields no targets. So the closed counts above are a lower bound and the yield is not inflated.

## 3. The rule (A1)

At a JS/TS call or JSX site whose callee identifier is bound, at the site, to a parameter `p` of holder `H`:

1. **Fallback only.** The route runs only where today's resolver drops the site (`UnknownName` or `LocalBindingUnproven`). A site that resolves today keeps its answer byte for byte. So no `CORRECT` or `NONE-OK` matrix row can regress, and the 146 name-search sites on T are untouched.
2. **Never when `p` is written** anywhere in `H` (any assignment, update or `for … of p`). The binder's existing write index (M2) decides.
3. **Callers.** Use every Source call site (a call or JSX element) whose resolution is exactly one **Exact** target equal to `H`. Use no NameOnly reverse edges, so NameOnly is never chained, and no synthetic or indirect records.
4. **Value at the position.** For position `i` (a TypeScript `this` parameter takes no position) and path `π` (empty, an object key, or an array index; one level), take the argument at `i`, or the JSX attribute named by `π`'s key when the caller is a JSX element and `i = 0`, then step into an object or array literal along `π`. The value contributes:
   - a callable through the existing terminal machinery (`JsFollow`: an in-file callable by name and span, an import local through the import-member route, a namespace member, or a `const` alias chain of #372). Only Exact terminal outcomes count;
   - both branches of `c ? a : b`, both sides of `a || b`, `a ?? b`, and the right side of `a && b`;
   - `useCallback(fn, deps)` → `fn`, as the binder already does;
   - nothing for `undefined`, `null`, a literal, a missing argument, or a missing key (a missing argument or key, or an explicit `undefined`, takes the parameter's or binding element's default value instead);
   - the caller's own parameter: **forwarding**, followed recursively up to depth 2 with a visited set;
   - anything else (a call result, a member of an object, `this.m`, a spread argument before or at `i`, a JSX spread before the attribute, a `let`/`var`, an anonymous function in A1) contributes no target and is counted by reason.
5. **Grade.** Every target is `NameOnly`, of a new `ResolutionKind::ArgumentFollow` (`argument_follow`). This holds even when the set is closed and has one element: a closed set is a fact about visible code, and an exported holder's invisible callers make it a "may".
6. **Cap.** More than 8 distinct targets: drop as `ArgFanout`, a new `DropReason` counted in `call-stats`. Then `map(arr, f)` stays dropped instead of growing 112 edges.
7. **Empty.** No target: the site keeps today's drop reason (counted as `arg_follow.empty` by reason).

Exported or escaping holders are **not** refused: NameOnly is exactly the "may" grade, and the matrix's `may` rows want `MAY-NAMEONLY`. `call-stats` counts them separately (`arg_follow.holder_exported`, `holder_escapes`), so the owner can tighten this later without a re-measure.

## 4. How it fits the existing machinery

- **Call graph and reverse edges.** `CallGraph::callers` is keyed by callee *name* (raw sites), so "the Exact callers of `H`" means `callers[H.name]` filtered by resolution. That must not run inside `resolve_call_site_full`: it would resolve recursively, and the cost would be per site. A1 computes a whole-program **derived index** once, after the build, in the same post-pass position as `compute_indirect_call_sites`: `js_ts_arg_targets: BTreeMap<ParamSlot, ArgOutcome>`, with `ParamSlot = (holder FunctionId, position, path)` and `ArgOutcome = Targets(BTreeSet<FunctionId>) | Fanout | Empty(reason)`. Forwarding iterates this index to a fixed point bounded by depth 2. The resolver then reads only the index: one map lookup at the drop point.
- **Precedent.** Go already has argument following, Level 3 in `compute_indirect_call_sites` (`go_callback.rs`): bare free functions passed to proven higher-order functions become synthetic `IndirectResolution` records with `pre_resolved_target`. A1 deliberately does **not** reuse the synthetic-record shape. JS/TS answers belong to the Source record's own outcome (as with #372 and #382), so `call-stats`, the callers index and the matrix see one record per site.
- **#372's single predicate (`via_alias_route`) and stand-down.** These are unchanged. The argument route never sets `via_alias_route`, and it cannot produce "exactly one Exact target", so `js_ts_alias_route_hit` stays false at every parameter site. The textual Level-1 path (`resolve_fptr_assignment`) only fires where the body assigns the name. Rule 2 refuses exactly those parameters, so the two routes never answer the same site. The rule test (`js_alias_rule_test.rs`) gains one assertion: no `ArgumentFollow` outcome has `via_alias_route`.
- **#382 (returned values).** Independent. A `Returned` binding is a `const`, never a parameter. One documented interaction: #382's `param-value` row (`useThing(cb)` returns `{ run: cb }`) needs argument following *inside* a returned fact. That is left out of A1, and the row stays as it is.
- **Callers index / nav.** NameOnly targets flow through the normal resolved-outcome path. `nav callers` scores them 0.6 with `Reason::Resolution`. `--format callers` and `output/review.rs` credit only Exact alias outcomes and are unchanged. `call-stats` gains `js_arg_follow` (sites by outcome, by parameter kind, holder exported or escaping, fan-out, forward depth).
- **Taint.** The P14 descent is Exact-gated, so it does not descend through argument edges. `evidence_rank` grades a path with a NameOnly call hop below an all-Exact path. Findings that cross an argument edge stay `nameonly`/`candidate`. No taint code changes; one test pins that a taint path through an `ArgumentFollow` edge is not `asserted`.
- **Binder.** It already proves "the nearest binding of this name is a parameter" (`Pos::Param` in `js_binding_walk.rs`), and `Unproven("not_callable")` is what parameter sites carry today. A1 adds the **slot** (holder span, position, path, default) next to it, as a new `CallSite` field (`js_param_slot: Option<JsParamSlot>`, `serde(default)`, excluded from `cmp_key`). It does not change `local_binding`, so the existing `call-stats` reason counters stay byte-identical.

## 5. Persisted facts and cache impact

| Fact | Where | Per-file or derived |
|---|---|---|
| `CallSite.js_param_slot` (holder span, position, key-or-index path, default value as `JsArgValue`) | every JS/TS Source site bound to a parameter | per file |
| `CallSite.js_args: Vec<JsArgValue>`, plus for JSX `js_jsx_attrs: BTreeMap<String, JsArgValue>` and a spread mark; `JsArgValue = Follow(JsFollow) \| Param{slot} \| Union(Vec) \| Object{key→value, spread} \| Array{…} \| Nothing \| Opaque(reason)`, one level of literal | every JS/TS Source call and JSX site with arguments | per file |
| `CallGraph.js_ts_arg_targets` | whole program | derived after merge, never cached on its own |

- CPG cache: `CACHE_VERSION` 117 → 118 (new `CallSite` fields). Nav call-edge cache: 65 → 66 (resolved edges change in A1b; bump in A1a as well, since the `CallSite` serialization changes).
- **Incremental builds.** `js_ts_arg_targets` must be recomputed on the merged graph (the P5/P9 pattern noted in `build_incremental_with_scope_graph_inputs`), because a change in file A changes what B's holder receives. The A1b test: a partial cache hit, where only the caller's file changed, gives the same index as a cold build.
- **Size.** `js_args` is stored for every call with arguments, which is most calls. Keep `Opaque` compact (a `u8` reason). The A1a gate measures cache size on T and X before and after.

## 6. Matrix rows each part flips

The new `step: "arg"` rows were added in this plan, 135 of them: TS and JS, with JSX and TSX for props. Bless changed no existing row: `git diff` shows 135 added lines in `expected.jsonl` and 0 removed. Today they are MISSING 103, MAY-NONE 18, NO-RECORD 8, NONE-OK 6. Every recorded row drops with `UnknownName`, and no new row has a wrong edge today.

| Part | Rows (both languages unless noted) | Expected change |
|---|---|---|
| A1a (facts only) | none | `expected.jsonl` byte-identical. This is the A1a gate |
| A1b, one or set → DEMOTED | `named-fn-same-file`, `const-arrow-same-file`, `imported-fn`, `const-alias-of-import`, `namespace-member`, `second-position`, `param-shadows-import-name`, `two-callers-same-fn`, `two-callers-two-fns`, `conditional-arg`, `default-unused`, `default-overridden`, `default-mixed`, `default-undefined-arg`, `inner-block-shadow`, `closure-in-holder`, `optional-call`, `guarded-call`, `destructured-object-param`, `destructured-object-shorthand-arg`, `destructured-array-param`, `forwarded-two-levels`, `forwarded-two-levels-two-roots`, `holder-passed-itself`, `escapes-to-local-table` (partial: f only), `class-method-this-call`, `props-destructured`, `props-destructured-arrow-component`, `props-two-elements`, `props-local-handler`, `props-missing-at-one-caller`, `props-forwarded-two-levels`, `props-component-slot` (JSX site), `props-name-trap`; also on the existing `param-args` rows: `plain-one-caller`, `plain-two-callers`, `plain-member-arg`, `forwarded-twice`, `default-value`, `destructured-props*`, `closure-param`, `props-destructure-in-body` (all namings) | MISSING → DEMOTED |
| A1b, may → MAY-NAMEONLY | `exported-holder`, `exported-via-list`, `default-exported-holder`, `escapes-to-external`, `callers-mixed-external`, `caller-passes-own-param`, `exported-class-method`, `props-exported-component`, and the existing `param-args` `plain-exported` | MAY-NONE → MAY-NAMEONLY |
| A1b, must not move (negative controls) | `non-callable-arg`, `external-arg`, `no-callers` (NONE-OK); `props-spread-caller`, `param-of-callback-to-external` (may with no targets); `reassigned-before-call`, `reassigned-conditionally` (rule 2 refuses; stay MISSING); every `param-lookup` row | unchanged |
| A1b, conditional on prism resolving the holder Exact | `object-literal-method` (`api.run(imp)`), `props-memo-wrapped` (holder is the arrow inside `memo()`) | DEMOTED if the reverse edge is Exact; otherwise unchanged and recorded as a known gap. Not a blocker |
| A2 (anonymous callables) | `inline-arrow`, `inline-function-expression`, `props-inline-arrow`, `param-args` `inline-arrow-arg` → DEMOTED; `props-in-effect-closure`, `param-of-callback-to-local-hof` → NO-RECORD becomes a record | A2 only |
| Parked | `holder-via-call`/`-apply`/`-bind`, `ts-this-param`, `site-is-param-dot-call`, `props-member`, `rest-index`, `rest-destructured`, `arguments-object`, `spread-at-caller`, `destructured-arg-is-variable`, `new-through-param` (NO-RECORD: prism does not record `new` through a parameter) | unchanged |

The A1b implementer pins the actual diff and explains every row that differs from this table. A row in the "must not move" group that moves is a defect, not a re-bless.

## 7. Slice split (each review sized to converge in 2 rounds)

- **A1a: facts and counters, no resolution change.** `JsParamSlot`, `JsArgValue`, the two `CallSite` fields, the binder code that fills them (reusing `js_ts_follow_alias` for argument identifiers), cache bumps, and a `call-stats` `js_arg_facts` histogram. Gates: matrix `expected.jsonl` byte-identical; X and T `call-stats --dump-sites` byte-identical apart from the new counters; unit tests per `JsArgValue` shape, including the parameter's own position, the `this` parameter, defaults, and a JSX attribute after a spread; cache size measured; RAYON_NUM_THREADS=1 and 4. About 500–700 lines with tests. It has no behavior to dispute, only facts, so 2 rounds is realistic.
- **A1b: the route.** The derived index and fixed point, rules 1–7, `ResolutionKind::ArgumentFollow`, `DropReason::ArgFanout`, counters, the rule-test assertion, the incremental-equivalence test, the matrix bless (§6), and a CLAUDE.md ladder note. Corpus gate: X and T versus main. Added NameOnly edges should be about X 13 / T 232 sites. Every added target is adjudicated against the census's `param-sites.jsonl` (the same target set, by file, name and line), and there must be 0 changes at sites that resolve today. A full authoritative mutation gate before merge.
- **A2 (owner decision first): anonymous callables.** Not argument following. It is a call-graph identity change: a `FunctionId` for each inline arrow or function expression, records for the calls inside them, and how `callers`, nav and the review output show them. It needs its own spec and census (X 3,275 and T 3,306 unrecorded product calls). After it, argument following to anonymous targets is a short follow-up (A2b: `JsArgValue::Inline(span)` already exists from A1a).

## 8. Risks

| Risk | Mitigation |
|---|---|
| Recursion or cost from resolving callers inside the resolver | Derived index computed once per build; the resolver does one lookup (§4) |
| Incremental or partial cache hits leave a stale index | Recompute on the merged graph; cold-versus-partial equivalence test (§5) |
| Fan-out noise from generic helpers (`map`, `forEach`) | Cap of 8 → `ArgFanout`; the census shows the cap costs 15 T sites and 0 X sites |
| Overclaiming a closed set when the holder escapes (`.apply`, a table, a returned value) | NameOnly only, never Exact; escapes counted. The matrix rows `escapes-to-local-table` and `holder-passed-itself` pin the partial answer |
| Reassigned parameter gives a wrong target | Rule 2 refusal; `reassigned-*` rows are negative controls |
| Position errors (`this` parameter, spread arguments, `.call` offset) | `this` skipped; spread at or before `i` gives no target; `.call` parked; rows `second-position` (trap) and `ts-this-param` pin these |
| Name traps (a parameter named like an import or a same-file function) | The binder's site binding already decides the parameter; rows `param-shadows-import-name` and `props-name-trap` |
| Cache growth from `js_args` on every call | Compact encoding; size measured in A1a; drop the per-site store for arguments that are `Opaque` at every position |
| The X (React) value is small without A2 | Stated up front (§1); A2 is an owner decision |
| Census proxy: "prism resolves the argument" is approximated by the checker's binding kind (import, `const`, function declaration) | The A1b corpus gate adjudicates every added edge; the self-check shows the census errs only on the conservative side |

## 9. Reproduce

- Product binary: `cargo build --release` at `b593ad18` (`target/release/prism`, reports `slicing 3.1.2 (b593ad183e9f)`).
- Dumps: `prism nav --no-cache call-stats --repo <root> --dump-sites` → `~/prism-evidence/js-args/{X,T}/sites.jsonl` (sha256 in `~/prism-evidence/js-args/SHA256SUMS`).
- Census: `node --max-old-space-size=12288 measure/census.cjs <root> <sites.jsonl> <out>`; checker `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.
- Self-check: `node measure/selfcheck.cjs tests/fixtures/js_alias_matrix/fixtures.jsonl target/release/prism <scratch>`.
- Private corpus: `sh CONTROLLER-census.sh <root>` prints `aggregate.json` only (verified on X: its output equals X's `aggregate.json`).

## Controller addendum (2026-10-14)

- **Unrecorded calls inside anonymous callables: reproduced by the controller** on a synthetic file with the #382 binary (`V4-prism`). `foo()` inside `[1].map(() => foo())` and inside `setTimeout(function () { foo(); })` has no call-site record, and `nav callers foo` lists only the calls in `A`, `top` and `B`.
- **Census on private corpus F**, aggregates only:
  - parameter sites: 344;
  - named-target yield: 4 sites (any target: 21);
  - closed sites: 0;
  - product call nodes inside anonymous callables with no record: 2,470 of 15,860 (15.6%).

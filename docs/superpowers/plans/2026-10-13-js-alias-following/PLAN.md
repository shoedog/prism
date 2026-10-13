# JS/TS alias-following — bounded implementation plan

**Decision proposal, not implementation authority.** Build positive local binding proofs, then parameter lookup, then import improvements; finally replace unsupported JS/TS bare-name repo fallback. Keep E5 parked until that release contract and its accepted losses are approved. Do not resume its exhausted review loop with another syntax exemption.

All corpus numbers below come from [ANALYSIS.md](ANALYSIS.md) and `/Users/wesleyjinks/prism-evidence/js-alias/step0b/{X,T}/decision-summary.json`: `slice_opportunities`, `policy_exposure`, `part1b`, `part1d`, and `part1a.parameters`. They are **static site-level opportunity ceilings**, with registered target identities, not measured implementation gains. An implementation must measure actual full target sets. `P / Tst` separates product/test sites. No private sources were opened.

## Decision and accepted boundaries, before dispatch

1. **Nearest declaration controls.** Find the nearest language binding first, then dispatch its category in the owner’s implementation order: local → parameter → import. A nearer parameter shadows an outer local or import. This makes the order implementable without sacrificing lexical correctness.
2. **Exact requires a singleton proven callable identity.** Carry file, declaration/span identity, ordinary-call versus JSX mode, and proof provenance. Do not resolve to a same-spelled function elsewhere. Direct existing local/import/export proofs remain available. Type signatures and arbitrary factory results do not supply value identity.
3. **Drop unsupported repo-name candidates; do not demote them.** A NameOnly label on an unrelated function still corrupts topology. Only optional argument-following P2 has a positive reason to emit NameOnly candidates. Removal applies to JS/TS **Source bare-identifier call/JSX** fallthrough at R5, including `free_multi` if the owner accepts its quantified uncertainty cost. It does not silently broaden to indirect/member/constructor/`super` records.
4. **Bounded follow.** First delivery follows at most eight local/value-alias hops and the existing two re-export hops, with a visited key `(canonical file, declaration byte span, member key, call/JSX mode)`. Exhaustion, cycle, parse/binding ambiguity, unsupported value, type-only route or noncallable terminal → no new target and, after final R5 refusal, drop. Bounds are separate: checker-alias collapsing is not a product depth proof. Local proved census traces are ≤4; the known additional export-depth opportunity is 40 T sites at three hops.
5. **Immutable subset.** New alias links initially require `const`, a unique declaration and no explicit write to that binding anywhere in its modeled scope, including nested closures. Existing direct-callable behavior is retained. `let`/`var` aliases, assignment-built aliases, nested/array/default/rest destructuring, conditional targets, unknown factory returns, object getters/spreads/computed keys and escaped object values are accepted no-new-edge limits. This is a conservative static model; it does not promise runtime monkey-patching analysis. `const a=f` is not stable if `f` is an explicitly reassigned binding. Value recursion checks every hop.
6. **No caller/terminal identity expansion.** A site with no Prism record and an anonymous callback with only a DFG synthetic owner remain outside this work. X’s 6 and T’s 72 known missing import endpoints without a registered target remain missing. The separate 41 T same-file misses are not credited to alias work.
7. **Valid script globals need an owner choice.** Literal removal loses known synthetic correct script-global examples, despite zero such proved corpus edges. Recommended exception G is declaration/scope-proved script globals, not repo-wide callable-name uniqueness. If the owner insists on no cross-file global environment at all, record those examples as accepted losses before implementing final refusal.
8. **`super` is a separate construct.** The round-2 imported-object heritage witness is outside the bare-identifier census. Do not land E5’s broad refusal over it. Keep baseline `super` behavior outside this replacement until separate heritage slice H is approved; if “no repo-wide name search” must cover every JS/TS call kind, H or explicit accepted loss is a release prerequisite.

A positive slice can land independently while the old unsupported R5 behavior remains. Each such PR must say that it **adds binding proof but does not yet fulfill the no-by-name objective**. The final R5 slice is a separately reviewed semantic cut. This avoids concealing interim behavior or selling one positive slice as E5 completion.

## Slice sequence and measured ceiling

| Dispatch order | Slice | X gain P / Tst | T gain P / Tst | Wrong corrected/removed | Known correct-loss exposure / limit |
|---|---|---:|---:|---|---|
| 1 | L1: immutable identifier chains in a file | 1 / 0 | 0 / 0 | 0 measured corrections | New positive proof only; realized losses unmeasured |
| 2 | L2: local namespace-member aliases and flat destructures | 0 / 0 | 0 / 0 | 0 measured corrections | Preserves namespace round-2 witnesses; do not count synthetic examples as corpus gains |
| 3 | L3: React wrapper identifier argument | 1 / 0 | 0 / 0 | 0 measured corrections | Wrapper mode/arity/provenance remains mandatory |
| 4 | P1: parameter lookup/refusal state | 0 / 0 | 0 / 0 | 0 current parameter wrong edges | All 330 X / 1,004 T sites currently have no targets |
| 5, optional | P2a/P2b: observed argument candidates | ≤20 / 0 sites | ≤116 / 5 sites | No measured correction claim | NameOnly only; counts use current Exact incoming callers and ≤3 candidates; not Exact gains |
| 6 | I1: bounded relative module paths | 7 / 0 | 14,064 / 1,103 | 0 corrections | Mostly `.js`→`.ts`, not a 15k-site alias engine |
| 7 | I2a: exported const values / aliases | 1 / 0 | 2 / 0 | 0 corrections | Preserve conflict/type-only/cycle barriers |
| 8 | I2b: exported React wrapper values | 3 / 5 | 0 / 0 | 0 corrections | One X case also requires I1; allocated here once |
| 9 | I3: import-equals namespace alias | 0 / 0 | 10 / 0 | T 3 corrected sites: 2 Exact + 1 NameOnly | 7 T missing + 3 corrected; old NameOnly site has 3 wrong candidates |
| 10, optional | I4: re-export cap 2→4 | 0 / 0 | 40 / 0 | 0 corrections | These 40 remain missing if cap stays 2; one also needs I2a |
| Separate owner-selected prerequisite | G: proven script-global binding | 0 observed | 0 observed | Unmeasured | Needed to preserve known correct global synthetic shapes under final refusal |
| Separate heritage lane if required | H: imported-object-member `super` | Outside denominator | Outside denominator | Unmeasured | Needed before claiming no-by-name for keyword constructor calls |
| Final | R: remove unsupported bare-name R5 fallback | 0 positive gains | 0 positive gains | Preserve removal of X 19 / T 201 old E5 WRONG targets; two T sites get I3 replacements | X 12 / T 2 unknown original Exact targets; X 3 / T 59 unknown NameOnly sites; script-global witnesses require G or accepted loss |

Disjoint alias/import maximum is **X 18 / T 15,219**; without optional I4 it is X 18 / T 15,179. P2 is a separate candidate-edge objective and must not be added to Exact gains. Optional P2 may be declined after P1 so imports proceed in the requested order. Every private slice receipt must use the supplied controller census first, then compare implementation outputs and adjudicate all additions, replacements and removals; a static ceiling is not an acceptance threshold.

The common verification/review contract below applies to **every** slice, including G/H/R. Each slice also lists its behavior-specific RED tests, negative controls and mutants. Estimates are changed production LOC plus focused tests/fixtures; they are planning estimates, not quotas. No slice gets more than two review rounds without the convergence classification required below.

## L1 — immutable local identifier aliases

**Accepted input/result limits:** `const a=f; a()` or `<A/>` may reach an existing same-file callable; `let a=f`, an assigned target binding, duplicate declaration, cyclic chain, depth-nine chain, conditional initializer, default destructure or factory result gets no new edge. After R, unsupported repo fallback drops. Existing direct function/arrow/class paths are controls, not newly claimed gains.

**Rule and contract:** Extend the binder’s Alias state with declaration identity; follow `const` identifier links within the selected lexical environment to a direct registered function/arrow/class terminal, retaining its existing call-versus-JSX eligibility. Return Exact only for one file/span target. New proof returns no NameOnly. During L1 alone, an unproved chain records its refusal reason but retains the prior resolver’s out-of-slice behavior; R later makes that refusal terminal at repo-name fallback. Reuse scope/write/parse checks rather than making a second name map. `CLAUDE.md` sentence: “A JS/TS local identifier alias resolves Exact only when a bounded chain of unique, unwritten const bindings reaches one registered callable in the modeled lexical environment.”

**Termination:** Eight value hops, same-file identities, cycle set; transparent TS assertion/parenthesis wrappers cost no semantic alias hop. Count aliases consistently in diagnostics and tests. `const a=f` where `f` is a parameter remains a parameter terminal, never a guessed function.

**Yield/cost:** X 1 product site (`flush`→the actual local callable), T 0; no measured corrected site. New-positive-only target sets should lose zero existing CORRECT targets, but this is a required implementation gate, not a measured fact. Private: same-file alias positives and unknown refusals separately, full target identity and grade.

**RED/negative/mutants:** Same-name distractor in another file must fail on main and resolve to the alias target after change; two-hop and renamed variants; JSX/direct-call modes; nearer parameter/block/catch binding; later and nested write; assigned terminal; duplicate const; declaration cycles; cap-eight success/cap-nine refusal; same-line declarations distinguished by byte span. Mutants remove each const/unique/scope/write/cycle/depth/span/parse guard. Negative cases retain/drop precisely according to the staged contract, never accidentally assert that final R already landed.

**Size/STOP:** 200–350 production + 250–400 test LOC. Split provenance storage from traversal before dispatch if lifecycle wiring pushes production past roughly 500 LOC. Common gates apply; STOP any wrong target, unrelated binding override or new serialization path lacking an executable test.

## L2 — local aliases of namespace members

**Accepted limits:** `import * as N from './m'; const f=N.f; f()` and `const {f:g}=N; g()` are admitted when N’s nearest declaration is that value namespace import and the export proof is unique. Nested/default/rest/array/computed destructuring, generic object escapes/getters/mutations, namespace lookalikes, and unmodeled member chains get no new edge. Static object-literal member support is deliberately deferred despite the census’s conservative diagnostic tracer supporting some cases; no public opportunity is assigned to that extra capability.

**Rule/contract:** Reuse `js_ts_namespace_binding` and export resolution as a terminal of a local alias; carry cross-file identity through `JsTerminal`/resolution instead of matching the local alias spelling. Flat object destructuring selects the explicit property key, including a renamed binding. Exact singleton only; no NameOnly. Unresolved terminal records bound-but-unproved and eventually drops at R. `CLAUDE.md`: “A const local alias of a statically proven namespace export retains the export’s callable identity and call/JSX restrictions; member spelling alone is not binding evidence.”

**Termination:** Same eight-value/two-export bounds and visited key; namespace import must be unique, value-bearing, nearest and unwritten. Preserve export ambiguity and type-only erasure. Do not infer namespace identity from object type or same repo file stem.

**Yield/cost:** X 0 / T 0 added corpus sites; it is still necessary to close the known ordinary-call and JSX namespace-alias loss shapes before R. Synthetic preservation is not a yield estimate. Private: count actual member/destructure chains and replacements; retain all unsupported routes.

**RED/negative/mutants:** Round-2 namespace alias ordinary/JSX witnesses with a differently named true terminal and a misleading unique same-name function; flat rename destructure; inner parameter named N; type-only namespace, later write to local alias, conflicted export, missing property, default/rest/computed destructure, cycle and over-budget route. Mutants remove qualifier binding identity, type-only check, property-key selection, JSX mode or conflict evidence. Full/hydrated/subset graph tests must agree on cross-file target identity.

**Size/STOP:** 250–450 production + 300–500 tests. Keep generic object/value-flow admission out; split cross-file terminal storage as a prerequisite if needed. Common gates plus STOP if an import-spelling exemption substitutes for proof.

## L3 — identifier argument to a proven React wrapper

**Accepted limits:** `const C=memo(Base); <C/>` reaches Base only if the wrapper is an admitted value import from React and Base follows the L1/L2 proof. `memo(Base)()` as an ordinary call, shadowed `memo`, extra/spread arguments, reassigned Base or an unknown factory remain unproved. Preserve current inline-wrapper behavior. A callback type is not an implementation target.

**Rule/contract:** Extend the existing wrapper-value classifier’s first-argument proof from inline callable to a bounded identifier terminal. Reuse exact import provenance and the existing admitted arities for useCallback/memo/forwardRef. Ordinary useCallback and JSX-only memo/forwardRef modes remain distinct. Exact only for one target; no NameOnly; unsupported route eventually drops at R. `CLAUDE.md`: “An admitted React wrapper may retain a proven identifier argument’s callable identity, with the existing wrapper provenance, arity and JSX-mode restrictions.”

**Termination:** Shared value bound and cycle set include the wrapper argument; wrapper classification must not recursively manufacture a new alias budget. Caller and export summaries use the same proof result.

**Yield/cost:** X 1 product JSX site, T 0. Existing wrapper-correct edges must be unchanged; actual loss count unmeasured. Private: separate inline controls, identifier additions, ordinary-call refusals and wrapper-spelling-only unknowns.

**RED/negative/mutants:** `memo(Base)` with alias name different from Base (RED on main), analogous useCallback/forwardRef synthetic cases if the shared branch admits them; ordinary-call twin; shadowed/non-React import, wrong arity/spread, overwritten argument, chain cycle, anonymous unregistered target. Mutants remove each import/module/arity/spread/mode/target-identity gate. Tests assert actual target and confidence, not merely count.

**Size/STOP:** 150–250 production + 200–350 tests. Common gates; STOP if wrapper type/return signature is used as value proof or distinct exported-wrapper behavior is bundled here.

## P1 — parameter lookup only

**Accepted limits:** `function run(cb){cb()}` and `{cb}`/default formal variants are bound to parameters; their defaults do not imply that every call uses that default. They receive no positive call edge. This phase does not chase callers, props or runtime invocation order.

**Rule/contract:** Make parameter identity an explicit nearest-binding outcome before file-import fallback. Preserve parameter-expression versus body scope distinctions and the existing position/parse refusal machinery. No Exact or NameOnly is added. An unresolved formal reaches a parameter-specific refusal at R rather than repo name lookup. `CLAUDE.md`: “A JS/TS callee bound to a parameter is not resolved by same-named repository functions; a default value alone does not establish the actual argument.”

**Termination:** Lookup stops at the parameter; aliases ending there retain its identity, with no recursion into callers. Destructure/default/rest shapes are classified for diagnostics but do not gain terminal targets.

**Yield/cost:** 330 X / 1,004 T recorded parameter sites, all currently without targets: 0 additions, 0 current wrong removals, 0 known correct losses. This is essential policy infrastructure, not a claimed accuracy gain. Private: measure any currently resolved parameter collisions and their exact removed targets before adopting R.

**RED/negative/mutants:** Main already has function-local/import shadow guards; do not invent a failing-main parameter collision when the current guard already drops it. P1 is a contract/coverage checkpoint if P2 is deferred, with no standalone product change needed. If parameter provenance is added for P2, its RED asserts the newly exposed stable enclosing-callable/formal identity, preserved across alias and serialization paths; all existing call targets stay identical. Negative controls cover parameter shadow of an import/outer local, destructured prop, alias of formal, omitted/default actual, rest, nested scope and body redeclaration versus parameter initializer. Mutants skip parameter lookup, treat default as always used, search an outer scope first, or lose parameter identity through an alias. Keep D12 fixtures active. Any claimed new refusal behavior needs a real same-environment RED before it enters scope.

**Size/STOP:** 0 production if the existing parameter guards suffice for lookup-only; otherwise 100–200 production plus 150–300 focused tests for the P2 provenance prerequisite. Common gates apply to any code change; STOP any positive edge inferred from a default alone or any silent removal of PR-A’s outgoing DataFlow firewall.

## P2 — optional actual-argument following, separate decision

**Accepted limits:** Open/exported/escaped functions may have invisible callers. Observing one argument does not prove exclusivity. Anonymous inline actuals without a registered callable remain unavailable; the PR-B `<cb@...>` DFG name cannot be promoted. Spreads, rest mapping, dynamic props, computed/destructured expressions beyond the accepted mapping, defaults with ambiguous supply, `.call/.apply`, recursive candidate cycles and >3 distinct candidates are no-new-edge limits. Omitted/explicit-undefined default substitution is modeled only at that specific observed caller, never globally.

**Rule/contract:** Use only existing Exact incoming call records with positive language-binding provenance to the enclosing callable and syntax-proven actual-to-formal positions, then follow actual values through the bounded local/import machinery. Unsupported `free_single` is not an eligible incoming proof merely because its confidence says Exact. Emit **NameOnly**, with observed caller/slot/target provenance, for 1–3 distinct registered candidates; drop the candidate set if it exceeds three, never truncate arbitrarily. No Exact promotion even for a syntax-closed singleton. Preserve any unknown/invisible-caller flag. `CLAUDE.md`: “Parameter-call candidates derived from observed arguments are NameOnly, retain actual-call provenance, and do not claim that all callers or possible values are visible.”

**Termination:** One incoming-argument expansion in the first delivery, no recursive parameter-to-parameter propagation; local/value/export budgets remain shared. Candidate identity deduplicates by FunctionId, not name. Recompute deterministically when caller/argument/target changes.

**Measured ceilings:** Current Exact incoming edges provide candidate sites X 22 (37 edges), T 138 (286 edges); ≤3 cap leaves X 20, T 121 (116 product/5 test). These are upper bounds before the additional incoming-provenance and first-delivery mapping restrictions, not promised realized counts. The broader checker-visible ceiling is X 24/T 279 registered-target sites (43/1,433 edges), not an honest current-graph yield. Visibility has X 182/T 678 unique parameters: exported 111/226, syntactically closed 22/245, other-open 49/207. Those heuristic classes never license Exact. Actual implementation losses/precision/performance unmeasured; private must report open/closed, candidates, cap losses and unmatched caller records.

**Relation to merged work:** PR-A D12 refuses outgoing new-formal DataFlow paths into cross-file Exact free_single; it remains. PR-B B-D1/B-D5 supplies DFG ownership only. P2 must not create anonymous FunctionIds or start inferring interprocedural argument edges through those synthetic owners. The incoming bare-formal DataFlow hole is a distinct seam; call candidates do not silently repair it. Tests bind CallGraph and DFG expectations separately.

**Split before dispatch:** P2a (200–350 production + 250–450 tests) handles plain positional formals and registered actual values. P2b (200–350 + 250–450) handles flat literal JSX props/destructured formals and per-caller default selection. Publish the aggregate ceiling as shared, not as a measured allocation between these slices; that split is unmeasured until row mapping/implementation. Both have two-round review caps. If one needs a new caller-discovery engine, park P2 and proceed to imports after the owner’s decision.

**RED/negative/mutants:** Multiple named actuals to one formal, exported caller with unknown external use, true inline FunctionId versus DFG-only callback, two equal-spelled targets, correct positional index, omitted versus supplied default, explicit undefined, literal prop rename, spread override, four-target cap, call-edge grade change and target removal. Mutants promote singleton to Exact, accept NameOnly incoming calls, ignore export/escape metadata, confuse slot/property keys, keep stale candidates, truncate overflow, or mint synthetic callback identities. Common gates plus a targeted argument/return DataFlow full-value comparison; STOP any unexplained new interprocedural path.

## I1 — relative module path proof

**Accepted limits:** Admit canonical in-root relative `.`/`..` directory routes and redundant separators, and TypeScript’s explicit extension substitution for supported JS-family suffixes to corresponding TS-family sources. Do not infer arbitrary npm package entry points, execute package metadata hooks, weaken tsconfig project matching, or resolve an ambiguous candidate by filesystem order. Keep the existing two re-export cap. Unknown external imports stay unknown.

**Rule/contract:** Repair candidate construction/normalization before applying existing export/member proof, preserving unique module identity, project/path eligibility and value/type distinctions. Exact is justified by the whole import/export route, never the suffix alone. No NameOnly introduced; failed routes eventually drop at R. `CLAUDE.md`: “A JS/TS relative import may use the modeled TypeScript extension-substitution and directory rules only when they identify one eligible module and a unique callable export.”

**Termination:** Finite explicit suffix/candidate list, canonical repo-bound paths, existing two-export traversal bound and cycle/conflict checks. For a literal relative `.js` specifier, the new substitution precedence is `.ts`, `.tsx`, `.d.ts`, `.js`, `.jsx`, as confirmed with the pinned checker and an in-memory host (`step0b/module-candidate-order.json`). Select the first eligible existing module under that policy, then prove its value export; a declaration-only first choice is not permission to skip to a lower-priority runtime file and invent an edge. Other explicit suffix substitutions (`.jsx`, `.mjs`/`.mts`, `.cjs`/`.cts`) retain existing behavior in this slice; new support is deferred. Dot/directory normalization retains the existing finite index-file rules. Keep package exports and full NodeNext semantics out.

**Yield/cost:** X 7 product; T 15,167 (14,064 product/1,103 test), ceiling after existing export proof. Controls show frozen base’s direct `.js` import drops UnknownName while an extensionless same-target import is Exact import_member; this is same-environment evidence. No implementation loss count. Private: route-class histogram, conflict/ambiguity count, all old Exact targets, CJS/tsconfig controls and wall/RSS cost.

**RED/negative/mutants:** Direct `.js`→`.ts` same-target pair; `.` directory index; redundant separators; `.js` and `.ts` coexistence with the explicitly chosen TypeScript-compatible precedence; duplicate/case/symlink/canonical-path identities under existing platform rules; nonrelative alias from wrong project; missing/erased export; cycles and depth boundaries; no-outside-root candidate. Mutants skip explicit-suffix substitution, directory index, canonicalization, project/containment check, precedence or ambiguity barrier. Do not broaden this to an arbitrary resolver rewrite.

**Size/STOP:** 200–350 production + 300–500 tests; pre-split suffix and directory rules into I1a/I1b if >500 production LOC or two distinct proof mechanisms need separate review. Preserve the published combined ceiling; suballocation X7/T15,167 is only credited where row proofs establish it. Common gates plus STOP if the large measured opportunity is used to justify unadjudicated target additions.

## I2a — exported local values and aliases

**Accepted limits:** Exported const-arrow/function values and bounded immutable alias chains can forward their registered callable identity; arbitrary return values, conditionals, setters/getters, unproven mutable values and ambiguous star/named export claims remain unproved. Existing CJS producer mutation/conflict barriers remain authoritative.

**Rule/contract:** Extend export summaries/forwarding from direct function declarations to verified local/value identities supplied by L1/L2. Preserve `UnprovenLocal` and conflicting export claims rather than dropping the inconvenient candidate. Exact singleton only; no NameOnly; unsupported export eventually drops at R. `CLAUDE.md`: “A value export may forward a bounded, proven local callable identity; aliases do not erase type-only, conflict, cycle or producer-barrier evidence.”

**Termination:** Shared eight-value/two-export bounds, file/declaration keys, explicit type-only and CJS producer checks. A namespace export alias follows the actual exported binding, not a same-named member elsewhere.

**Yield/cost:** X 1 product (forwarded const-arrow), T 2 product namespace aliases within two export hops. One additional related T alias is allocated to I4. Losses unmeasured; private must separate ESM exported values, import-forward aliases and existing CJS control routes.

**RED/negative/mutants:** Export list of a const arrow; renamed alias of namespace export; default named expression if admitted; named export versus conflicting star, unproven competing value, type-only, assigned source binding, cycle/cap, CJS exports overwrite/reset barriers. Mutants remove any uniqueness/conflict/producer check, conflate local/exported name, or match a target merely by line/name. Common gates.

**Size/STOP:** 250–450 production + 300–500 tests; split ESM summary admission from CJS changes if latter are needed. Recommended first slice changes no CJS semantics. STOP loss of negative export claims or an unbounded export graph.

## I2b — exported wrapper identifiers

**Accepted limits:** The L3 proven wrapper result may be exported/imported while retaining its mode and exact target span. Unknown wrappers, type-only exports, mutated aliases, extra/spread arguments and ordinary calls to JSX-only memo/forwardRef values remain unproved. Anonymous target identities are not invented.

**Rule/contract:** Reuse the single L3 proof in `js_ts_wrapped_export`/export summaries; persist the underlying callable and wrapper-mode constraint across module routes. Exact only for singleton eligible target; no NameOnly. `CLAUDE.md`: “Exporting an admitted wrapper preserves its proven target identity and wrapper call-mode restrictions across imports.”

**Termination:** Shared budgets, visited set and export conflict evidence; never reset depth or discard wrapped mode while traversing a renamed/default export.

**Yield/cost:** X 8 sites (3 product/5 test), T 0. One X test also needs I1; no double count. No implementation loss claim. Private: imported wrapper additions, default/named/renamed splits and ordinary-call negative controls.

**RED/negative/mutants:** `export const C=memo(Base)` and `export default memo(Base)` with named registered Base; import rename/default then JSX; ordinary-call twin, shadowed wrapper, conflicting export, unregistered anonymous Base and mode loss through re-export. Mutants erase wrapped marker, misbind default/renamed identity or skip shared guards. Common gates.

**Size/STOP:** 150–300 production + 200–350 tests. STOP duplicate wrapper classifiers that can disagree between local and exported routes.

## I3 — import-equals aliases through exported namespace values

**Accepted limits:** `import f=N.renamed` follows a proven value namespace/export alias to its real callable; type-only import-equals, arbitrary dotted objects, ambient signatures without an in-repo value, ambiguous exports and unsupported external `require` forms receive no new edge. Existing proven CJS require routes are preserved, not generalized.

**Rule/contract:** Treat import-equals as binding evidence to follow, not an exemption from refusal and not a request to find a repo callable named f. Use I2a/L2 module and value proofs. Exact singleton only; no NameOnly. `CLAUDE.md`: “A value import-equals alias resolves to the proven callable exported by its bound namespace route, never to an unrelated callable sharing the alias’s spelling.”

**Termination:** Shared budgets/cycle keys; follow the import-equals declaration’s namespace identity and exported property, preserving type-only and value-proof barriers.

**Yield/cost:** T 10 product sites: 7 previously missing, 2 WRONG Exact replacements, 1 WRONG NameOnly site replacement (removes its 3 wrong candidates). X 0. Two corrected Exact sites preserve E5’s original target-removal result while adding the right target. Private: measure equivalents and ensure the old wrong FunctionIds are absent, not merely that a right target was added.

**RED/negative/mutants:** Exact round-2 renamed export witness with a unique same-spelled distractor; namespace-local imported alias; type-only, shadow namespace, exported noncallable, ambiguity, depth/cycle and external-module-reference control. Mutants keep alias spelling instead of export key, skip namespace binding or type-only checks, and retain old candidates alongside the proven target. Common gates.

**Size/STOP:** 200–350 production + 250–400 tests. STOP any treatment of syntactic import presence as a positive callable proof.

## I4 — optional bounded re-export depth increase

**Accepted limits:** Keep cap two unless the owner accepts this separate extension. If adopted, raise to four with the same per-route visited/collision rules; five-hop, cyclic or ambiguous routes remain dropped. These limits are published input/result behavior, not review defects.

**Rule/contract:** Change only the explicit re-export traversal budget and its consistently applied discovery/lookup surfaces; do not use a higher cap to bypass unresolved export proof. Exact remains the unique proven terminal; no NameOnly. `CLAUDE.md`: “JS/TS export following is bounded to four re-export hops and refuses cycles, ambiguity and routes exceeding that bound.”

**Yield/cost:** T 40 product sites at three hops, X 0; one needs I2a. Keeping cap two leaves those already-missing sites unchanged. Performance/losses unmeasured; private must publish the depth histogram, graph fan-out, cache size, wall time and RSS.

**RED/negative/mutants:** Three/four-hop renamed and star routes, five-hop refusal, reachable cycle, duplicate terminal versus conflicting terminal, unresolved competing branch. Mutants omit any depth check in discovery/lookup or reset visited on a renamed route. Common gates.

**Size/STOP:** 50–150 production + 200–350 tests. STOP a need for generic unbounded graph traversal or new conflict semantics; replan rather than expand this small slice.

## G — owner-selected script-global exception

**Accepted limits:** Recommended preservation is limited to a unique callable declaration in the language’s script global environment, with no nearer local/parameter/import shadow and no conflicting global declaration. Exported declarations in external modules are not script globals. Position-unknown scope is not proof of absence. Ambient `Array` must not bind an unrelated repository function. If this rule is declined, the merged-namespace/enum/decorated-class witnesses become explicit known-correct synthetic losses under R.

**Rule/contract:** Construct a declaration-based global environment from admitted script files and prove lookup through the actual caller scope, including the currently refused Position cases. This may index global declarations across files; it is an explicit exception to an interpretation forbidding any cross-file global lookup. It must not reuse “one function in the repo has this name.” Exact singleton only, otherwise drop; no NameOnly. Proposed `CLAUDE.md`: “A cross-file script-global call is Exact only when the modeled global declaration environment and the caller’s lexical scope prove that unique callable binding.”

**Yield/cost:** 0 proved public corpus residual globals; preservation of known synthetic inputs, private yield unmeasured. Do not price this as an accuracy gain from X/T. The census’s 1 X/2 T unresolved residuals are not automatically global candidates.

**Termination/tests/mutants:** Finite admitted global declarations plus the shared alias bounds; no package/runtime global discovery. RED on each merged namespace, enum and decorated-class Position witness; ordinary script-global control; external-module same-name export, ambient library name, local/parameter shadow, duplicated scripts, parse/position ambiguity and declaration removal. Mutants confuse script/module, ignore scope/refusal, accept ambient type-only declaration or fallback to unique repo callable. Common gates. Estimate 250–450 production + 350–500 tests; pre-split scope-proof support from global-environment lookup if >500 production LOC. STOP open-class Position exceptions; a one-name or one-node exemption is not a design.

## H — separate constructor/heritage obligation

The keyword `super` does not enter the bare-identifier table. Baseline behavior remains outside R’s scope. If the owner requires all JS/TS repo-name fallback gone, approve a separate heritage proof first: resolve `extends N.Base` or an imported object member to one constructible registered class/constructor, then route `super` by that class relation, not by searching for a callable named `super`. Object value provenance, property mutation/construction barriers, class-versus-callable eligibility and inheritance cycles are separate proof obligations; ordinary function signature compatibility is insufficient.

**Accepted limits/contract:** Unknown object/member heritage, ambiguous/nonconstructible target or cycle → no new Exact, no NameOnly, and drop only under the separately approved all-call-kind refusal. `CLAUDE.md`: “A super constructor edge follows a proven constructible heritage target rather than repository-wide keyword spelling.” Local/alias bound eight and heritage visited identities bound eight; explicit longer-chain refusal is part of this separate contract. Public yield/cost **unmeasured**, as is private; do not invent a number from this census. Preserve the exact round-2 baseline target as a regression control, then assert the declared heritage target after the new proof.

RED: imported-object-member heritage whose real constructor has a different name from the keyword; controls for shadowed import/object, getter/nonconstructor, ambiguous property, class removal, inheritance cycle and depth. Mutants bypass constructibility, heritage binding, member provenance, shadow/cycle/depth checks. Common gates. Estimate 300–500 production + 350–550 tests; split object-member value proof and constructor routing before dispatch if necessary. This is not silently bundled into L2 or R. A completed alias plan is not a claim that H has been implemented or that the broad original E5 refusal is now safe.

## R — final refusal and E5 replacement

**Accepted losses, requiring owner acknowledgment before implementation:** X 12 unknown original free_single targets (11 undecided, 1 inadmissible), T 2 inadmissible original free_single targets. Literal removal of all repo-name fallback additionally removes X 3 unknown NameOnly sites/9 edges and T 59 unknown NameOnly sites/360 edges. T’s remaining one NameOnly site is corrected by I3. Script-global synthetic correct edges require G or an explicit accepted-loss decision. Unsupported chains listed above receive no edge; absence of a proved endpoint does not mean the value is noncallable. Existing earlier positive local/import rungs remain outside this negative cut.

**Rule/contract:** After existing and new positive local/parameter/import routes, refuse remaining JS/TS Source bare-identifier `free_single` and `free_multi` results. A selected binding whose modeled value is unsupported does not grant a name-search fallback. Preserve explicit G if approved. Constructors/member calls/IndirectResolution/`super` remain outside this bounded cut. Exact only through positive proof; NameOnly only through separately admitted P2; otherwise drop with a stable reason distinguishing bound-unknown, unresolved-global and proof limit. `CLAUDE.md`: “JS/TS Source bare-identifier calls and JSX elements do not resolve through repository-wide callable-name uniqueness; unsupported bindings are dropped after the admitted positive binding rules.”

**Yield/cost:** Baseline 31 X/203 T free_single sites disappear or resolve positively, removing the original 19 X/201 T WRONG targets; I3 replaces two of T’s wrong targets. Of those E5 WRONG rows, 19 X/195 T final values remain unknown, so the claim is old wrong-target removal, not proof that no callable exists. T’s four ambient collisions correctly have no in-repo edge. Static public known-correct loss is 0 but actual implementation loss remains to be measured. Private original 109 WRONG/0 classified CORRECT lost is inherited; the controller must verify those exact original targets remain absent and separately adjudicate new removals/additions. No planner guarantee of private zero loss.

**Avoid overreach:** Dropping every checker-unknown local target would also affect earlier local/same-file routes: X 124 sites (19 W/105 U), T 430 (195 W/235 U). That larger rewrite is not included. The unchanged checker’s unresolved nonrelative imports cannot veto 3,112 existing X Exact import sites. Keep existing affirmative product proofs unless a constructible mismatch and bounded repair justify a separate slice.

**Tests/mutants:** Reuse all E5 old-wrong targets plus positive alias controls; zero surviving forbidden R5 target sets, not merely reduced counts; NameOnly multi-candidate refusal; preserved G or explicit global-loss assertions; Source/Indirect pair; bare JSX/member JSX pair; `super` exclusion until H; non-JS language parity. Mutants omit family/origin/bare/JSX fences, allow unique-name fallback through a bound unknown, keep one wrong candidate beside a replacement, demote instead of remove, conflate Position with unbound, or bypass exact target lifecycle invalidation. D12 must retain its real `free_single` provenance checks for out-of-scope fixtures; do not make its regression pass merely by renaming a ResolutionKind. Common gates plus exact old-target removal set and all-loss adjudication.

**Size/STOP:** 150–300 production + 350–550 tests. If replacing E5 facts requires broad persisted-field changes, split provenance migration from semantic refusal with independently verified behavior. STOP on any unaccepted correct loss, unmapped private removal, new wrong target, Source/Indirect scope expansion or another open class of positive-binding loss.

## Common gates, custody and two-round cap

Every slice starts from controller-bound main/product base, not this parked-candidate checkout by assumption. Record checkout, HEAD/base/diff, dirty/owned paths, source hashes, binary hashes and allowed actions. No implementation is authorized by this document alone. The controller commits; this planning step makes no Git writes.

Before implementation, write the slice’s exact input→target/grade/drop table and its accepted limits. Obtain same-environment RED on main for every new behavior; a failing setup, zero selected tests or exit code alone is inadmissible. Also state an alternative mechanism for each diagnostic and the output that separates it. Pin all conjuncts with meaningful behavioral mutants, including each serialized/recomputed path rather than tests that simply mirror the new helper.

For each implemented slice:

- Run the project’s **full** Rust suite and doctests with the repository’s prescribed environment; the E5 lane’s recorded commands are `cargo nextest run --offline --all-targets --features mcp --no-fail-fast` and `cargo test --doc --offline --features mcp` (serialize the test run with the lane’s thread configuration). Record actual passed/failed/ignored totals. The baseline comparison for any failure uses the same environment. Out-of-scope failures are reported, never silently fixed or rebaselined. If host facilities prevent the full suite, run the largest runnable subset and enumerate every exclusion; no full-green claim follows.
- Run fmt/check/clippy gates required by the repository. Because this work touches call resolution, build release immediately in the same worktree, then `cd eval && uv run tier-a --matrix-only --allow-stale-sut`; run `uv run tier-a --quick --allow-stale-sut` before review with the freshly built binary. Missing rust-analyzer/host facilities is an explicit exclusion. Full `--corpus all` is human-triggered only. Report regressions and flip candidates, never rebaseline to erase them.
- Compare X then T and controller-owned private output on source-bound manifests and fresh caches. Report full target identities, confidence/rung/drop reason, wrong removals/corrections, known-correct losses, unknown additions/removals, source/indirect populations and per-test partitions. Keep static ceilings separate from realized gain. Re-run the reverse join gate; >3% is STOP. Never execute corpus packages.
- Verify full build, cold/warm CPG, nav cache hit/miss, serialization roundtrip, skeleton/hydrated/subset graph reconstruction, add/remove/merge/rebuild, rename and target deletion. Assert full values, exact counters and cache versions (planning snapshot CPG 115 / nav 62). Bump the relevant persisted schema/outcome versions and exact test/registry pins; a helper-only test is not lifecycle coverage.
- Keep non-JS output byte-identical. Verify CallGraph/nav and DFG separately. Alias proof may legitimately affect argument/return dataflow, so enumerate those changes; it is not enough that call-edge counts improve. Preserve PR-A D12 and PR-B anonymous-owner isolation unless separately authorized.
- Measure wall/RSS and cardinality against the same-environment base. Proposed STOP thresholds are >25% median wall or RSS regression on a controlled three-run comparison, any unbounded graph/candidate growth, any new WRONG or unaccepted CORRECT loss. The owner can set stricter budgets before dispatch; no performance approval is inferred from this static run.
- Refresh the lane handoff from the installed template at each stable point and snapshot claim-bearing evidence. Source/revision/binary binding belongs in every review packet. A new durable conclusion replaces stale claims in the lane documents in the same turn.

**Review cap: two rounds per independently bounded slice, declared before dispatch.** WRONG requires a constructible input/state and incorrect result; report it before SMELL. Do not downgrade an inherited WRONG merely because this census ends at unknown. Closed enumerable findings receive targeted fixes on the existing artifact. At the cap, classify convergence: only shrinking, non-repeating bounded findings justify a disclosed extension; new members of the same open class mean park and revise the design. Do not restart the artifact or re-roll E5 without explicit owner approval and a reason it cannot be salvaged. If a gate prints only its first error, enumerate the entire defect population before the next state-changing retry. No reviews were dispatched in this planning turn.

## E5 disposition options

| Option | Effect on measured E5 result | Round-2 shape consequences | Judgment |
|---|---|---|---|
| Positive slices, then unchanged E5 | Aims to retain removal of X19/T201/private109 old wrong targets, with T import-equals replacements; realized totals require controller measurement | L2 handles namespace aliases, I3 handles exported import-equals; unchanged broad refusal still loses script-global Position and imported-object `super` shapes without G/H | Better sequencing, but **do not land unchanged E5** merely because alias tests pass |
| Fold refusal into ordered lookup / final R5 policy | Preserves the 19/201 old wrong-target objective; T2 become positive corrections. Private109 remains an exact-target controller gate, not a fresh result | L2/I3 prove alias cases; G or explicit loss handles globals; H or explicit bounded `super` exclusion handles heritage | **Recommended**, with stated bare-Source scope and owner decisions. Reuse negative facts/tests; retire broad E5 exemption logic |
| Drop E5’s second arm, retain target-referenceability arm | Keeps X0/T53/private0 WRONG removals; abandons X19/T148/private109 of the original result. T53 comes from `both` rows, not just the four ambient calls | Round-2 alias/global/super examples were second-arm losses and generally regain baseline behavior; no positive alias target proof, target-side arm still needs its own controls | A quantified precision retreat, not completion of owner’s direction |
| Delete JS/TS bare R5 by-name fallthrough immediately | Removes the public 31/203 free_single targets, including W19/W201 and unknown12/2; additionally affects free_multi as quantified above. Private total unknown until census | Drops correct alias witnesses unless positive rules precede it; globals require G/loss. `super` must be explicitly scoped out or separately proved | Final policy can be this narrow cut, but **not an unsequenced switch** or a claim of zero semantic cost |

Sources: public arm counts in `step0b/history-XT-summary.json`; private arm/result counts are inherited from the required E5 packet and brief, not re-verified. None of these options upgrades old zero-classified-CORRECT-loss corpus observations into safety for all inputs. The two-round E5 cap remains exhausted; this is a new positive-proof design with bounded slices, not permission for round three of exemptions.

## Owner decisions and completion status

1. **“Args”:** recommended P1 lookup-only in the initial delivery; opt into P2a/P2b NameOnly observed candidates separately, or defer both after P1. The decision changes scope/grade, not just scheduling.
2. **Script globals:** recommended explicit G declaration/scope exception. Otherwise accept the demonstrated global synthetic losses before R. Zero observed public positives does not settle this language-policy choice.
3. **No-by-name breadth:** recommended remove both free_single and free_multi for Source bare identifiers, accepting the listed unknown exposures; keep indirect/member/`super` scope explicit. If “all calls” is required, H and the other out-of-denominator kinds need measured scope before release.
4. **Import depth:** keep two initially or approve separate I4 to recover the 40 T three-hop sites. No silent budget increase.
5. **Private acceptance:** controller runs `CONTROLLER-census.sh`, then implementation full-target deltas, including the inherited 109 WRONG target set. Static private gain/loss is currently unmeasured; do not dispatch a no-loss adoption claim from public data.

[ANALYSIS.md](ANALYSIS.md), this plan and [CONTROLLER-census.sh](CONTROLLER-census.sh) complete the requested planning deliverables. Controller X aggregates match the public X table bytes exactly; measurement checks pass 38/38. No current brief STOP remains. Product implementation, private execution, full product tests, mutation/Tier-A/performance gates, G/H proofs and actual slice yields were **not** performed. The former blocked documents are superseded; the original STOP/ruling remains in ANALYSIS history and the untouched step0 snapshot.

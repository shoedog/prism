# SPEC: lane js-param-defs, PR-A "binding shapes" (gaps 3 + 4)

**Status:** R2 targeted fold of the existing artifact. Review round 2/2 classified as converging; the owner brief authorizes this disclosed one-time cap extension for closed N1/N2 and S2–S4. No third review round or restart is authorized.
**Design input:** `DESIGN-INPUT-fable-evaluation.md` §3.1, §3.2 "Gap 3+4", §3.3, §3.5.
**Base:** `c8de720b36c24ae8a7ab274ec10c994f258eb336`; historical planner `006573d9` is source-equivalent.
**Prototype / dispatch:** committed `bd4c30ffbe0a843391ca5c44e68c69cd8ae3b4d5` (`origin/proto/js-param-defs`) plus `repair-r2/R2-src.patch`. This commit already contains R1; do not re-apply R1-src.patch. R2 docs dispatch from `a0e8504e87e383536f6be7e1a90ee463e93d76c7` plus `repair-r2/R2-docs.patch`. Historical `1b2dfdc9` remains the pre-R1 prototype for gain-cost comparisons.
**Evidence:** `MEASUREMENTS-r2.md`, `HANDOFF-repair-r2.md`, `~/prism-evidence/js-param-defs/repair-r2/`; `CENSUS.md`, `PROBE-LOG.md` and `MEASUREMENTS-prA.md` retain historical evidence.

## Goal (decision served)
The owner's 2026-10-05 decision is to ship JS/TS parameter Defs for the gaps SecBench demonstrated. This PR covers two of them, with Defs registered only for named callables:
- the unparenthesised arrow formal (`x => …`);
- the identifier rest formal (`...xs`).

Success means:
- SecBench's 6 rest conversions and the 3 arrow rows gain their formal Def on the **unmodified** packages. Measured: 6/6 rest and 1/3 arrow trace; the other 2 arrows stop at downstream barriers. The eligible sweep has 0 regressions;
- every changed DFG row on X/Xi/T/SecBench is adjudicated at byte identity; E7 and unreachable-flow base parity are disclosed separately;
- non-JS output is byte-identical.

Exact remains a static-binding grade. Every ambiguity resolves toward no edge.

## §0 Decisions
| # | Decision | Why (evidence) |
|---|---|---|
| D1 | Scope is gaps 3 + 4, in one PR with one cache transition | The two share code paths (`function_parameter_occurrences`, `typescript_parameter_bindings`). Splitting them would cost a second transition (EVALUATION §3.6). |
| D2 | **Deviation from EVALUATION §3.2:** `find_parameters_node` is **not** changed. New helpers are added instead: `js_ts_bare_arrow_parameter`, `js_ts_rest_identifier` and `parameter_binding_region`. They are used at exactly 3 consumers: `function_parameter_occurrences`, `typescript_parameter_bindings` and `scope.rs::declaration_seed`. The `exact_read` arm uses the bare-arrow helper. | There are 16 callers. 12 iterate the list's children, which would see an empty identifier "list". `collect_js_ts_parameter_bindings` has its own `parameter` fallback that a `Some` would bypass, losing the S1b F3 binding. `js_ts_parameter_receiver_binding` would change receiver evidence. PROBE-LOG P8. The measured result: call-site rows are byte-identical on X/Xi/T/SecBench. |
| D3 | **Correction to EVALUATION §3.2:** TS rest is `required_parameter` with `pattern: rest_pattern(identifier)`, admitted under the existing child allowlist. The Def bytes are the inner identifier. | The pinned grammar has no `rest_parameter` node (PROBE-LOG P2). |
| D4 | `exact_read_is_plain_required_parameter` treats the bare arrow formal as plain-required for JS **and** TS/TSX. Rest stays excluded. | EVALUATION lists the JS arm only. The TS bare formal has no `required_parameter` wrapper, so the TS arm needs it too. Rest's parent is `rest_pattern`, so it is excluded and the byte-distinct caller-read expansion never covers rest. |
| D5 | The RD declaration seed uses `parameter_binding_region` (`scope.rs::declaration_seed`). `introduction_is_classified` is unchanged. | Without the seed, bare-arrow labels diverge from the identical `(x) =>` control. 9 T rows would be Exact where the control is NameOnly(Killed) (PROBE-LOG P5). The second site is equivalent: the root callable's own fields are never visited there (P6). |
| D6 | A rest formal is admitted only as the **last** parameter with no trailing comma after it (`(...xs, y)` is an early error the grammar accepts without recovery; found by the pre-existing test `reviewer_optional_inert_complete_allowlist_and_old_path_controls`). A JS rest formal is admitted only when the whole list is clean (no recovery, no duplicate binding). Pre-existing JS plain-formal behaviour on duplicate lists is byte-identical. TS keeps its existing whole-list refusal (duplicates, escaped spelling). A TS bare formal spelled with an escape is refused. | A duplicate is an early error once a rest element makes the list non-simple, so the ambiguity resolves to no Def. Changing sloppy-mode plain duplicates is out of scope and would break byte identity. |
| D7 | Positional slots are unchanged. Rest still stops `slots()`, because it is variadic, so Step 5b never binds an argument to it. | — |
| D8 | `CACHE_VERSION` goes 106 → 107. `NAV_CALL_EDGE_CACHE_VERSION` is unchanged at 62. | DFG rows and RD seeds change. Call-site rows are byte-identical on every measured corpus. One transition (lesson 10). |
| D9 | The has-bare-reference (member-only) guard is unchanged for the new shapes. | Relaxing it is PR-C and needs an owner accepted-cost decision. Census: on T, 1,041 named bare-arrow formals and 9 rest formals stay refused because they are member-only. |
| D10 | Anonymous callables still get no DFG pass. | That is PR-B. The census puts 1,128 bare-arrow formals (T) and 894 (SB) in anonymous owners. |
| D11 | **Any competing binding guard, new shapes only:** refuse the formal if any binding in the owner's body may bind its name. Cover nested formals; catch patterns; for-in/of heads (`const`, `let`, `var`, and conservative assignment heads); variable binding patterns; named function/generator/class expressions and declarations; TS class `type_identifier` names, abstract classes, runtime enums and namespaces/modules. Any escaped binding spelling fails closed without a raw-spelling equality assumption. | R1 sol W1 / Opus F1: the old guard omitted these binders and minted false Exact and NameOnly edges. The whole-owner refusal sacrifices new gains. Unrelated ordinary names and captured reads remain admitted. PR-B may retire this guard only after it proves containment for every binding kind and escaped ambiguity. R1 measurements price the refusal. |
| D12 | **Incoming bare-formal hole remains. Outgoing E5 refusal added:** after assembling intra/interprocedural argument and return DataFlow edges, reverse-walk from assembled argument Uses whose call resolution is cross-file Exact `free_single`, independent of callee syntax. Step 5b retains this predicate per emitted argument edge, so imported or NameOnly calls to the same target do not seed refusal. If a new bare/rest parameter Def reaches such an edge, remove only that new Def's outgoing DataFlow edges; retain its static Def and all existing call/argument edges. | sol W2 refutes the prior isolation claim: caller `input => Array(input)` newly reaches an unrelated `{Array: (s) => sink(s)}` across files. Reverse reachability includes locals and transitive flow; direct-formal-read-only refusal is insufficient. Raw DFG stays intact for incremental reconstruction when the target changes. R2 N1 covers object-literal shorthand methods (`w2m`) and unimported declared exports shadowing globals (`w2d`), alongside W2. Imported callees, same-file calls and NameOnly candidates keep their source rows; S2 recovers all four T gains forfeited by the old syntax guard, confirmed CORRECT by the unchanged byte checker (`MEASUREMENTS-r2.md`). Parenthesised incoming behavior remains asymmetric (Opus F5); the E5 lane should gate Step 5b on referenceable names for both spellings and reject pair/member/call-argument inference. |
| D13 | New rows carry prism's **existing** Exact semantics: a Def that reaches the Use on an unflagged CFG route. The parity tests pin it: bare vs parenthesised, and rest vs plain, give byte-identical dumps. Under EVALUATION §3.5 step 2's stricter rule (any prior write ⇒ not Exact), a hit counts against PR-A only if the same-shape plain-parameter control differs. | SB had 2 strict hits (`postcss` 20→24, `js-data` 1403→1449). R1 full-package plain-parameter controls carry identical byte rows and Exact labels on base (`repair-r1/postcss-E6-control.json`, `js-data-E6-control.json`); historical PROBE-LOG P14 is superseded by these current controls. Tightening Exact for every formal is a separate, owner-level semantics change (E6). |

## Non-goals (disclosed)
- Callback identity for anonymous callables (PR-B).
- The member-only base Def and projection edges (PR-C).
- Destructured rest (`...[a, b]`, `...{a}`). It stays refused and is pinned by tests.
- Destructured parameters generally (the ws5 React-props item).
- The legacy `arguments` object.
- TS `this` pseudo-parameters. They are not bindings; `pattern: (this)` stays excluded.
- Optional or defaulted rest. Both are syntax errors, and the allowlist refuses them.
- Anonymous `function*` expressions.
- Positional binding of arguments to rest (variadic, D7) or to the bare formal (D12; follow-up once E5 is fixed in the call ladder).
- Fixing the pre-existing defects in §6.
- `function_parameter_names` for **JS** keeps its string extractor and does not list rest or bare formals. TS names are derived from occurrences and now do list them. Its other consumer, `quantum_slice`, therefore changes output for TS/TSX only. This is a SMELL, recorded and not pursued.

## §1 Behaviour (JS, TS, TSX)
| Shape | Before | After |
|---|---|---|
| Named callable `x => …` / `async x => …`, `x` used bare | no Def | Def at the identifier bytes, pinned to the callable start line, with Def→Use rows. Labels equal those of the `(x) => …` control. |
| Caller `f(v)` of a named bare arrow | no arg→param edge (slot hole) | unchanged: still a hole (D12) |
| Bare or rest formal whose owner contains any competing or ambiguous binding of its name | none | none (D11) |
| Named callable `function f(a, ...xs)`, `xs` used bare | `a` only | `a` and `xs`. Labels equal those of the plain `function f(a, xs)` control. No Step 5b edge ever targets `xs`. |
| TS `(...xs: T[])`, `(this: T, ...xs: T[])` | none | `xs` |
| `...[a, b]`, `...{a}` | none | none |
| JS `(m, ...m)` (duplicate) | `m` (first) | `m` (first) only |
| TS `(m, ...m)`; TS `\u0061 => a` | none | none |
| Curried `x => y => …` | none | outer named arrow gets `x`; inner anonymous `y` gets no Def (PR-B) |
| Formal used only as `p.x` (any shape) | none | none (PR-C) |
| Anonymous owner (`list.map(x => …)`, `f(function(...a){})`) | none | none (PR-B) |
| Any non-JS/TS language | — | byte-identical (helpers are language-gated; measured on Python, Go and Rust) |

## §2 Implementation
The reference implementation is the prototype diff: `src/ast.rs`, `src/parameter_slots.rs`, `src/cpg/reaching/scope.rs` and `src/cpg_cache.rs`, with tests in `src/ast_js_param_defs_tests.rs`, `src/cpg/js_param_defs_tests.rs` and `src/cpg/required_parameter_tests.rs`. Per-site reasons are in `IMPLEMENTOR-prA.md`.

## §3 Tests (each fails on pre-change code; negatives per new path)
- **AST layer:**
  - bare / async / `get` / curried occurrences;
  - rest in JS/TS/TSX, with a `this` pseudo-parameter and a comment;
  - destructured, duplicate and escaped refusals;
  - slots unchanged;
  - the `exact_read` arm;
  - helpers inert for Rust (whose `..` is also `rest_pattern`) and Python lambdas.
- **CPG layer:**
  - Def bytes;
  - Step 5b never binds an argument to rest or to the bare formal (D12), while the parenthesised control keeps its edge;
  - the nested-formal refusal (D11), with an unrelated-name control;
  - label parity: bare vs parenthesised arrow and rest vs plain formal give **byte-identical `dfg_edge_dump`**;
  - curried inner arrow gets no Def;
  - refused shapes;
  - TS whole-list duplicate refusal;
  - R2 w2m/w2d in JS/TS/TSX, direct and local/rest paths, imported/same-file/NameOnly controls, and E7 JSX-attribute/pair-key plain-formal parity.
- **Updated pins of the old gap (intended changes):** `unsupported_parameters_are_not_introduced_and_do_not_compress_slots` (`rests` Def), `required_parameter_unsupported_forms_do_not_supply_definitions` (`...a` moves to supported; `...[a]` stays unsupported), `reviewer_optional_inert_complete_allowlist_and_old_path_controls` (`value?, ...rest` → `value, rest`), and `nested_execution_owner_eager_and_own_callable_controls` (`p=>sink(p)` → `p`).
- **Mutants:** `mutants/js-param-defs.json` registers PD-01…PD-29 (PD-19…28 are R1 guard mutants; PD-29 disables the R2 resolution predicate):
  - helper gate, TS fast path and escape guard;
  - JS and TS rest admission, the duplicate guard, pattern admission and binding bytes;
  - the `exact_read` arm, RD seed and cache pin;
  - rest-last (PD-12);
  - the nested-formal guard: helper plus 4 call sites (PD-13…17);
  - the Step-5b hole (PD-18);
  - catch, loop-head, variable, function/class, escaped-name and trailing-comma refusal, transitive E5 exposure, TS type-identifier and runtime declaration guards (PD-19…28).

  `mutants/lane-p-tsconfig-paths.json` `P2-M11` is rebound to the 107 anchor, with an `intent_revisions` entry.

## §4 Measurement contract (EVALUATION §3.5, run on X, Xi, T, SecBench; F by the controller)
1. `nav --cache-dir ~/prism-evidence/js-param-defs/cache dfg-stats --edges` base vs head, multiset diff (`probes/rowdiff.py`) into ADDED / LOST / RELABELLED, using the byte dumper alongside the historical wire projection. Owners and endpoint spans are part of identity; an owner/span replacement is visible as LOST plus ADDED, never silently treated as the same row. Call rows use `call-stats --dump-sites` byte comparison.
2. Step 1, binding by the TS 5.9.3 checker (`probes/adjudicate.cjs`) over `probes/byte_dump.rs`:
   - export each endpoint's file, callable identity, byte span and parameter-Def classification;
   - map UTF-16 positions to physical UTF-8 bytes, preserving astral characters and the UTF-8 BOM removed by the compiler; a different physical/compiler text is UNDECIDED; call `getSymbolAtLocation` on the exact Use identifier bytes (shorthand Uses use the checker's value symbol), compare the declaration **name** span with the Def identifier span, and compare its callable byte identity;
   - missing or ambiguous owner records remain UNDECIDED; zero-width Uses require unanimous declaration-span identity for every same-name occurrence inside the recorded owner on the represented line; mixed/absent occurrences remain UNDECIDED;
   - ADDED WRONG is a STOP except the expressly disclosed pre-existing E7 row; LOST checker-correct base rows are a STOP. E1 base-only declarator rows are checker-WRONG, not lost correct parameters.
3. Step 2: Exact is the owner's static-binding grade and retains D13's route semantics. Independently report prior writes and syntax-proved unreachable Uses; do not call binding CORRECT a flow proof. R1 rest-after-return is identical on same-environment plain-formal base/head controls; the repair brief permits this parity disclosure (E8). Complete independent CFG reachability over corpora remains unverified.
4. Step 3: on the same harness `measure()`, base and head over the target and eligible rows; then the payload-specific BFS (r2-opus-A F5). A package that traced on base and not on head is a STOP.
5. Non-JS controls: Python (black), Go (caddy) and Rust (pinned prism snapshot) must be byte-identical for DFG and call-site rows.

## §5 Gates
- Touched tests.
- One `cargo nextest run --features mcp`.
- Advisory scoped mutgate, plus the authoritative lane run.
- fmt and clippy CI form `--all-targets --features mcp -- -W clippy::all`, compared to same-environment base; no new warning in touched code. `-D warnings` is not this gate.
- Fresh release build, then Tier-A `--matrix-only`.
- Tier-A quick (VALID; compare against r2).
- The §4 controls.
- F, run by the controller with `CONTROLLER-pd.sh diff` and a new `PRIVATE_EVIDENCE_ROOT`.

## §6 Pre-existing defects surfaced (recorded, not fixed here)
- **E1 (WRONG at byte level, line-level rows mask it):** a named arrow's pass owns every Def on its start line, including the declarator that names it: `const file = forEach(xs, file => …)`. Base therefore labels `Def file@L → Use file@L+1` Exact, although the Use binds the arrow formal. PR-A's formal Def collapses that line, and the row becomes NameOnly(SameLine): 4 T rows, safe direction (PROBE-LOG P7). PR-B's callable-span containment is the natural fix.
- **E2 (SMELL):** a destructuring read `const {a} = p` is labelled a kill of `p` for later Uses. It is conservative (NameOnly), and the parenthesised control behaves identically.
- **E3 (WRONG, PR-B scope):** the enclosing function's reference walk does not fence a nested callable's formals. EVALUATION §3.2 Gap-1 containment.
- **E4 (SMELL):** an assignment's own LHS identifier is recorded as a Use reached by the prior Def. This is parity with plain formals.
- **E5 (WRONG, call-resolution lane):** cross-file unqualified calls resolve Exact through `free_single` without a proven lexical/import binding. This includes declared exported functions in unimported ES modules that shadow globals (`export function escape(s)`), object-literal shorthand methods (`{ Array(s) {…} }`), and callables whose name is only *inferred* (`{ Array: t => … }`, `o.readFile = f => …`, `const x = f(() => …)`). Base example: `debug.ts:1057 Array(height)` resolves Exact to `inferFromUsage.ts:551`. Name inference is not referenceability (EVALUATION doctrine 4). D12 now refuses outgoing paths from new sources into every cross-file Exact `free_single` argument edge, using call-resolution provenance rather than callee syntax; the original incoming-hole-only claim was refuted by sol W2; PR-B must not give synthetic names to this ladder either. For parenthesised arrows the false Exact arg→formal rows already exist on base.
- **E6 (SMELL, semantics question for the owner):** Exact means "reaches on an unflagged route", not "the only reaching definition". A conditionally reassigned formal keeps an Exact edge to a later Use, for plain formals on base and for the new shapes alike (D13).

## §7 Risks
- R-A1: the RD seed change (D5) affects any JS/TS Def inside a bare formal's span. Only the formal itself is there.
- R-A2: T gains about 1.1 k rows, so the dfg-stats parity tests and `parallel_equality` must stay green (they are in nextest).
- R-A3: arrow conversions 1/3 trace; 2/3 move past source binding to a downstream barrier (B-plain-argument, D-cjs). These are residual gaps per §3.5 step 3, not WRONG.

## R1/R2 disclosures and carry-forward
- **E7 — WRONG, pre-existing classifier defect (Opus F2, R2 N2):** non-reference positions (pair keys **and JSX attribute names**) are counted as Uses on the formal Def line, identical to the plain-formal base. The R2 TSX control `x => q(<C x={1} />, function () { return x; })` and `(x) =>` plain base both carry a checker-WRONG attribute-name row. `anyTokenExcept(...tokens)` in T's `services/formatting/rules.ts` counts the non-computed `{tokens: ...}` key as a bare Use. The byte checker refutes its edge although a second `tokens.some` on the same line binds correctly. A same-environment plain-formal base control carries the same key edge. The byte oracle also finds the same E7 mechanism in SecBench `marsdb_0.6.11/lib/Cursor.js`, `_addPipeline(...args)`, key `args` at line 128; replacing the rest dots with spaces in an owned copy preserves byte spans and gives identical checker-WRONG rows on plain base and rest head. PR-C's shared Bare/MemberOnly/Unused classifier must exclude non-reference positions using the byte checker's `isReferencePosition` list, including pair keys and JSX attribute names; retain shorthand `{tokens}`, computed keys and JSX attribute values as reads. Local export-specifier property names remain references (R2 S1 caveat); only remote re-export names are non-reference positions. Its field projections must use the same binding proof.
- **E8 — WRONG flow, inherited parity (sol W4):** a rest source reaches a Use after unconditional `return`, identically to a plain formal on base/head. The repair brief explicitly permits proving and disclosing this parity. The binding checker reports static identity only; no complete flow-soundness claim is made. **Dynamic-scope base parity (R2 S4):** JS `x => { with (obj) { sink(x); } }` has the same NameOnly row as the plain-formal base. Static binding is not provable inside a `with` body; this remains disclosed parity, next to E8, and is illegal in strict/module/TS code.
- **Opus F5:** `x =>` remains an incoming Step-5b hole while `(x) =>` keeps existing bindings, including inherited E5 false Exacts. This forfeits some valid incoming bindings; E5 follow-up must settle referenceability before filling the bare slot.
- **Opus F6:** P2-M11 and PD-11 intentionally duplicate the 107→106 cache-pin mutation. Keep both population counts; every future cache bump must reconcile both registries. This is cross-lane coupling, not independent migration coverage.
- **Opus F8 / PR-B:** containment must fence catch/loop/name/block/escaped binders, preserve captures and use non-referenceable anonymous identities. Retiring D11 requires that complete proof.
- **Opus F8 / PR-C:** census member-only counts are upper bounds; pair keys and nested-name confusion can inflate prism's apparent bare-use population. Projection edges must exclude non-reference tokens and preserve path isolation.
- **SecBench F4:** exclude a package from both row aggregates if either side's required producer fails, including partial output. Record side-specific status and time. `clean-css_4.1.10` and `natural_5.1.0` are explicitly `prism_error` on both eligible sweep sides when their artifacts confirm it; they receive no regression credit.
- **Mutants:** R1 adds PD-19…PD-27 for catch, loop head, variable, function/class self-name, escaped spelling, comma, outgoing E5 refusal and TS class names; PD-13 is rebound to the widened nested-formal predicate.

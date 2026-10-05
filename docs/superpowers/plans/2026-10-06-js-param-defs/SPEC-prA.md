# SPEC: lane js-param-defs, PR-A "binding shapes" (gaps 3 + 4)

**Status:** planner draft for spec review (Opus-5.5 ∥ gpt-6.1-sol, cap 2).
**Design input:** `DESIGN-INPUT-fable-evaluation.md` §3.1, §3.2 "Gap 3+4", §3.3, §3.5.
**Base:** `006573d9`.
**Prototype:** uncommitted in the `~/code/prism-pd-plan` working tree; the final binary is `prism-head-prA3`. It is the reference implementation.
**Evidence:** `CENSUS.md`, `PROBE-LOG.md`, `MEASUREMENTS-prA.md`, `~/prism-evidence/js-param-defs/`.

## Goal (decision served)
The owner's 2026-10-05 decision is to ship JS/TS parameter Defs for the gaps SecBench demonstrated. This PR covers two of them, with Defs registered only for named callables:
- the unparenthesised arrow formal (`x => …`);
- the identifier rest formal (`...xs`).

Success means:
- SecBench's 6 rest conversions and the 3 arrow rows gain their formal Def on the **unmodified** packages. Measured: 6/6 rest and 1/3 arrow trace; the other 2 arrows stop at downstream barriers. The eligible sweep has 0 regressions;
- every changed DFG row on X/T/SecBench is adjudicated correct or safe-direction;
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
| D6 | A rest formal is admitted only as the **last** parameter (`(...xs, y)` is an early error the grammar accepts without recovery; found by the pre-existing test `reviewer_optional_inert_complete_allowlist_and_old_path_controls`). A JS rest formal is admitted only when the whole list is clean (no recovery, no duplicate binding). Pre-existing JS plain-formal behaviour on duplicate lists is byte-identical. TS keeps its existing whole-list refusal (duplicates, escaped spelling). A TS bare formal spelled with an escape is refused. | A duplicate is an early error once a rest element makes the list non-simple, so the ambiguity resolves to no Def. Changing sloppy-mode plain duplicates is out of scope and would break byte identity. |
| D7 | Positional slots are unchanged. Rest still stops `slots()`, because it is variadic, so Step 5b never binds an argument to it. | — |
| D8 | `CACHE_VERSION` goes 106 → 107. `NAV_CALL_EDGE_CACHE_VERSION` is unchanged at 62. | DFG rows and RD seeds change. Call-site rows are byte-identical on every measured corpus. One transition (lesson 10). |
| D9 | The has-bare-reference (member-only) guard is unchanged for the new shapes. | Relaxing it is PR-C and needs an owner accepted-cost decision. Census: on T, 1,041 named bare-arrow formals and 9 rest formals stay refused because they are member-only. |
| D10 | Anonymous callables still get no DFG pass. | That is PR-B. The census puts 1,128 bare-arrow formals (T) and 894 (SB) in anonymous owners. |
| D11 | **Nested-formal guard (new shapes only):** a bare or rest formal gets no Def when any callable nested in its owner binds the same name as one of its own formals (`js_ts_nested_callable_binds_formal`). Existing plain formals are unchanged. | The legacy reference walk (`is_shadowed_at`) fences block declarations but not a nested callable's formals (E3). The first T measurement added 5 checker-refuted rows from 2 such formals, 1 of them **Exact**. Example: `.map(name => { …forEach((entry, name) => …name…) })`. The safe direction is no Def. PR-B's containment fence retires this guard. Measured cost: MEASUREMENTS §2. |
| D12 | **Step-5b hole for the bare formal:** the bare formal has a Def, but `compute_param_def_nodes` does not bind call arguments to it. The parenthesised `(x) =>` behaviour is unchanged. | The first T measurement filled the slot and added 57 arg→formal rows. 26 were **Exact**, but the TS checker confirmed only 1 of those 26. The others bind calls that the ladder's `free_single` resolves to *name-inferred* callables (pair keys, assignment targets), for example `Array(height)` → `{ Array: t => … }` in another file, or `readFile(f)` → `o.readFile = f => …`. This is pre-existing call-resolution defect E5: the same Exacts already exist for parenthesised arrows on base. PR-A must not extend it. The SecBench targets seed the formal directly, so they lose nothing (MEASUREMENTS §4). |
| D13 | New rows carry prism's **existing** Exact semantics: a Def that reaches the Use on an unflagged CFG route. The parity tests pin it: bare vs parenthesised, and rest vs plain, give byte-identical dumps. Under EVALUATION §3.5 step 2's stricter rule (any prior write ⇒ not Exact), a hit counts against PR-A only if the same-shape plain-parameter control differs. | SB had 2 strict hits (`postcss` 20→24, `js-data` 1403→1449). The plain-parameter controls carry the identical Exact on base (PROBE-LOG P14). Tightening Exact for every formal is a separate, owner-level semantics change (E6). |

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
| Bare or rest formal whose name a nested callable re-binds as a formal | none | none (D11) |
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
  - TS whole-list duplicate refusal.
- **Updated pins of the old gap (intended changes):** `unsupported_parameters_are_not_introduced_and_do_not_compress_slots` (`rests` Def), `required_parameter_unsupported_forms_do_not_supply_definitions` (`...a` moves to supported; `...[a]` stays unsupported), `reviewer_optional_inert_complete_allowlist_and_old_path_controls` (`value?, ...rest` → `value, rest`), and `nested_execution_owner_eager_and_own_callable_controls` (`p=>sink(p)` → `p`).
- **Mutants:** `mutants/js-param-defs.json` registers PD-01…PD-18:
  - helper gate, TS fast path and escape guard;
  - JS and TS rest admission, the duplicate guard, pattern admission and binding bytes;
  - the `exact_read` arm, RD seed and cache pin;
  - rest-last (PD-12);
  - the nested-formal guard: helper plus 4 call sites (PD-13…17);
  - the Step-5b hole (PD-18).

  `mutants/lane-p-tsconfig-paths.json` `P2-M11` is rebound to the 107 anchor, with an `intent_revisions` entry.

## §4 Measurement contract (EVALUATION §3.5, run on X, Xi, T, SecBench; F by the controller)
1. `nav --no-cache dfg-stats --edges` base vs head, multiset diff (`probes/rowdiff.py`) into ADDED / LOST / RELABELLED. RE-OWNED is not observable on DFG rows, which carry no owner. Call rows use `call-stats --dump-sites` byte comparison.
2. Step 1, binding by the TS 5.9.3 checker (`probes/adjudicate.cjs`):
   - every ADDED def→use row must bind the Use to a formal of a callable starting on the Def line;
   - every use→def row must resolve the call to that callable and position;
   - WRONG is a STOP;
   - for a LOST row, a checker-correct base row is a STOP.
3. Step 2, Exact labels: a write to the formal before the Use (outside the Use's own assignment right-hand side), or in a loop that also contains it, is reported as `EXACT_PRIOR_WRITE`. Each hit gets the same-shape plain-parameter control. It is WRONG only if the control differs (D13). CFG reachability is not independently checked (disclosed).
4. Step 3: on the same harness `measure()`, base and head over the target and eligible rows; then the payload-specific BFS (r2-opus-A F5). A package that traced on base and not on head is a STOP.
5. Non-JS controls: Python (black), Go (caddy) and Rust (pinned prism snapshot) must be byte-identical for DFG and call-site rows.

## §5 Gates
- Touched tests.
- One `cargo nextest run --features mcp`.
- Advisory scoped mutgate, plus the authoritative lane run.
- fmt and clippy.
- Fresh release build, then Tier-A `--matrix-only`.
- Tier-A quick (VALID; compare against r2).
- The §4 controls.
- F, run by the controller with `CONTROLLER-pd.sh diff` and a new `PRIVATE_EVIDENCE_ROOT`.

## §6 Pre-existing defects surfaced (recorded, not fixed here)
- **E1 (WRONG at byte level, line-level rows mask it):** a named arrow's pass owns every Def on its start line, including the declarator that names it: `const file = forEach(xs, file => …)`. Base therefore labels `Def file@L → Use file@L+1` Exact, although the Use binds the arrow formal. PR-A's formal Def collapses that line, and the row becomes NameOnly(SameLine): 4 T rows, safe direction (PROBE-LOG P7). PR-B's callable-span containment is the natural fix.
- **E2 (SMELL):** a destructuring read `const {a} = p` is labelled a kill of `p` for later Uses. It is conservative (NameOnly), and the parenthesised control behaves identically.
- **E3 (WRONG, PR-B scope):** the enclosing function's reference walk does not fence a nested callable's formals. EVALUATION §3.2 Gap-1 containment.
- **E4 (SMELL):** an assignment's own LHS identifier is recorded as a Use reached by the prior Def. This is parity with plain formals.
- **E5 (WRONG, call-resolution lane):** unqualified calls resolve Exact through `free_single` to callables whose name is only *inferred* (`{ Array: t => … }`, `o.readFile = f => …`, `const x = f(() => …)`). Base example: `debug.ts:1057 Array(height)` resolves Exact to `inferFromUsage.ts:551`. Name inference is not referenceability (EVALUATION doctrine 4). PR-A avoids amplifying it (D12); PR-B must not give synthetic names to this ladder either. For parenthesised arrows the false Exact arg→formal rows already exist on base.
- **E6 (SMELL, semantics question for the owner):** Exact means "reaches on an unflagged route", not "the only reaching definition". A conditionally reassigned formal keeps an Exact edge to a later Use, for plain formals on base and for the new shapes alike (D13).

## §7 Risks
- R-A1: the RD seed change (D5) affects any JS/TS Def inside a bare formal's span. Only the formal itself is there.
- R-A2: T gains about 1.1 k rows, so the dfg-stats parity tests and `parallel_equality` must stay green (they are in nextest).
- R-A3: arrow conversions 1/3 trace; 2/3 move past source binding to a downstream barrier (B-plain-argument, D-cjs). These are residual gaps per §3.5 step 3, not WRONG.

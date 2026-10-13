# E5 "call referenceability": confirmation review, round 2 of 2 (Opus-5.5)

Reviewed: worktree `/Users/wesleyjinks/code/prism-e5-plan` at `c08ea98b` (source `2783899d`) against round-1 head `ce6de397` and main `bab0dfbd`; the fold diff and the whole diff for `src tests mutants`; both round-1 reports; rulings R1 to R7 and the step-2b brief; the six packet files named in the brief; public evidence in `step2/` and `step2b/`.

**Verdict: FIX.** Every round-1 finding is closed as ruled. One bounded item remains (N1): the unbound exception that ruling R2 added keys on one of the two binder states that mean "no own-file binding was found", so a module caller's call to a script global is still dropped when the binder grades the site `Position(...)`. It is a one-pattern change with zero predicted effect on every measured row. The controller can fold and verify it without another review. Nothing else blocks.

How I checked:
- Read the fold diff and the code around it. Did not run cargo.
- Confirmed the frozen step-2b binary is the reviewed source: `head-prism` SHA-256 `a2b4841d…` equals `step2b/binary-binding.json`, and all 315 product source hashes in that manifest equal the worktree files.
- Ran the frozen binaries (base, step-1 head, step-2b head) on all 48 round-1 Opus fixtures, 11 of sol's round-1 fixtures and 78 new synthetic fixtures, 137 in all. One fresh cache directory per invocation, except one deliberate shared-cache lifecycle probe.
- Ran the pinned TypeScript 5.9.3 checker on the synthetic fixtures where a verdict depends on what the name denotes.
- Read the mutation gate's per-mutant kill logs (`step2b/mutants/summary.json`) and recomputed the measured tables from `step2/union-target-adjudications.jsonl`.

Scripts, in `~/prism-evidence/js-e5/review-r2/opus-scratch/`:
- `probe.sh <fixture-dir> <symbol>` prints the callees of one symbol on base, step-1 head and step-2b head.
- `callers.sh <fixture-dir> <symbol>` prints the caller set of one target on base and step-2b head.
- Round-1 fixtures are in `fx/`, sol's in `solfx/`, new ones in `n/`. Transcripts: `r1-fixtures-rerun.txt`, `sol-fixtures-rerun.txt`, `new-finding-n1.txt`, `q2-evidence.txt`, `r2-exception.txt`, `q3-jsx.txt`, `q3-super.txt`, `q4-limits.txt`, `require-forms.txt`.

---

## Findings

### N1 — WRONG, MATERIAL: a module caller's call to a script global is dropped when the binder grades the site `Position(...)` instead of `Unproven("unbound")`

**Input and result (run).** Target in every row: `globals.ts` containing `function f(x: number) { return x; }` (a script file, top-level declaration, outside the arm (a) refusal set).

| Fixture | Caller (`caller.ts`, a module) | main | step-2b head | Pinned checker |
|---|---|---|---|---|
| `n/w1_enum_ns` | `export enum Color { Red = 1 }` then `export namespace Color { export function caller(x: number) { return f(x); } }` | Exact free_single `globals.ts:f@1` | dropped, LocalBindingUnproven | `globals.ts:1 FunctionDeclaration` |
| `n/w2_ns_ns` | `export namespace A { export const a = 1; }` then `export namespace A { export function caller(x: number) { return f(x); } }` | Exact free_single | dropped | `globals.ts:1 FunctionDeclaration` |
| `n/w7_nonexported_merged_plus_import` | an unrelated import, then two non-exported `namespace A` blocks, the call in the second | Exact free_single | dropped | `globals.ts:1 FunctionDeclaration` |
| `n/w8_decorated` | `export function caller(x: any) { const K = @f() class Named {}; return K; }` | Exact free_single | dropped | `globals.ts:1 FunctionDeclaration` |

Controls (run):
- `n/w3_single_ns`: the same call inside a single, unmerged namespace is kept by the head.
- `n/w4_script_merged` and `n/w8_decorated_script`: the script-caller twins are kept by the head.
- `n/w5_merged_module_target`: target `export function f` is refused by the head; the checker says UNRESOLVED, so that refusal is right.
- `n/w6_merged_alias`: a local alias `const f = o.cb` inside the merged namespace is refused; the checker resolves to the local declaration, so that refusal is right.

**Cause.**
- The binder reports "the walk found no own-file binding" in two ways. `Walk::Unbound` becomes `Unproven("unbound")`. `Walk::Unchecked(reason)` becomes `JsLocalBinding::Position(reason)` (`src/ast/js_binding_site.rs:36-37`, `src/call_graph.rs:5761-5769`).
- The reasons reachable for a name with no local declaration in a module file are `namespace_leave` (the call is in a namespace or enum body that has a merge partner in the same file, `src/ast/js_binding_walk.rs:201-213, 245-268`) and `decorated_class_expression` (`js_binding_walk.rs:182-184`). `unproven_position` is reachable only from type positions.
- The exception in arm (b) tests `Unproven("unbound")` only (`src/resolution.rs:3672`). A `Position` site therefore gets no exception, the calling file has no import evidence for the name, and arm (b) drops it.
- The binder's own contract for `Position` is "an unproven position keeps base behavior" (`src/call_graph.rs:527`). Arm (b) does not honour it.

**Hypothesis log (per the debugging discipline).**

| Step | Expectation | Result |
|---|---|---|
| Predicted from reading, before the run | A merged-namespace body in a module gives `Position("namespace_leave")`, so base keeps and head drops; a single namespace gives `unbound`, so head keeps | Got exactly that (`w2` versus `w3`) |
| Alternative cause: an earlier lexical guard or one of the older R5 drop reasons | Base would drop too | Refuted: base keeps; the head's drop reason is LocalBindingUnproven with `dropped_local_binding_unproven 1` |
| Alternative cause: arm (a) | Would need `Unproven("unbound")` and a negative-set target | Refuted: the target is a script top-level declaration, outside the set |
| Direct observation of the binder state | `call-stats` shows a position counter | `local_binding_unchecked_position.namespace_leave: 1` for `w2`, absent for `w3` |

**Scale, stated plainly.**
- No measured row has this shape. Of the 758 removed targets on X, T and SecBench, the product binder state is `Unproven("unbound")`, `MayCall("alias")`, `Unproven("import")` or `Unchecked` on every row; none is `Position` (recomputed from `step2/union-target-adjudications.jsonl`, field `product_call_facts.local_binding`).
- The shape needs a module file, a merged namespace or enum body (or a decorated named class expression), and an unimported script global.
- It is the round-1 class (sol W1, Opus F2) in a position the R2 wording did not cover. The R2 condition copies my own round-1 recommendation, which was too narrow.
- This item was not introduced by the fold: the step-1 head drops the same fixtures.

**Recommended fix.** In the arm (b) exception only, treat `Position(_)` like `Unproven("unbound")`:

```rust
let no_own_binding = matches!(&site.local_binding,
        JsLocalBinding::Unproven(reason) if reason == "unbound")
    || matches!(&site.local_binding, JsLocalBinding::Position(_));
// arm (b): ... && !(no_own_binding
//     && is_js_ts_import_member_file(&target.file)
//     && !self.js_ts_free_single_refusals.contains(target))
```

Arm (a) stays as it is. With this change `w1`, `w2`, `w7` and `w8` are kept; `w5` stays refused by arm (b) because its target is in the refusal set; `w6` stays refused because its binding is `MayCall("alias")`.

**Predicted measured change:** none on X, T or SecBench, because no removed row has a `Position` binding, so the removed set cannot change. The controller should confirm the same on F.

**Verification without another review.**
- Add `w1`, `w2`, `w8` as kept-Exact controls and `w5`, `w6` as refused twins.
- Register one mutant that removes the `Position` alternative; `w1` or `w2` kills it.
- Either run the head-to-head identity against the step-2b head (expected zero differing rows), or count removed rows with a `Position` binding (expected zero).
- Adjust the drafted `CLAUDE.md` sentence and SPEC line 17 from "unbound" to "no own-file binding found (unbound or position-unproven)".

**Options.**
- (A) `Unproven("unbound") | Position(_)` (recommended). One pattern; restores the binder's stated contract for `Position` inside arm (b); zero measured cost. It keeps main's edge when a merge partner in the same file exports a non-function value of that name, which is main's behaviour today.
- (B) `Position("namespace_leave")` only. Narrower; leaves `w8` dropped. `decorated_class_expression` is rare enough that this is defensible, but it leaves a second special case in the predicate.
- (C) Leave the code and list it as a limit. Costs nothing now. It is the one stated limit that would be a checker-confirmed correct resolution the head drops and main keeps, so the spec sentence "keeps module-to-script globals as main does" would have to be qualified.

**When this would not apply.** If the owner rules that `Position` sites in module callers are outside the contract, or withdraws "Position keeps base behavior" for E5. Then option C is the fold and the spec must say so.

**Why I read this as converging, not open-class.** This round I enumerated the binder's whole state space for a Source-origin unqualified site that reaches R5, not just examples:
- `Unproven("unbound")`: exception applies.
- `Position(*)`: this finding.
- `Unchecked`: only non-identifier callees. I found `super(...)` and nothing else in 72 probed call shapes (`n/pos_mod_ts`, `n/pos2_mod_tsx`, `n/pos3_mod_js`, each with a script-caller twin).
- `MayCall(*)`, `Unproven("import")`, `Unproven("with" | "parse_recovery")`: an own-file binding exists; the alias policy or the evidence set decides.
- `not_callable`, `duplicate_declaration`, `import_parse_recovery`: dropped before E5 on main as well.

The set of "no own-file binding" states is therefore closed at two, and the fix covers both.

---

### N2 — SMELL, IMMATERIAL: module-caller JSX to a script-global component declared through a wrapper call is dropped while its script-caller twin is kept

- `n/q3a_mod_memo`: `export function caller() { return <F />; }` with script `const F = memo(() => null);`. main keeps Exact free_single; the head drops. `n/q3a_script_memo` (script caller) is kept by both. `n/q3e_mod_forwardref` behaves the same.
- Mechanism: the wrapper-initialised arrow is in the arm (a) refusal set, so the R2 exception does not apply, and arm (b) covers JSX while arm (a) does not.
- Why this is not WRONG: the name `F` is declared as the result of a call, not as the function. The slice already refuses the plain-call form of the same name and target for every caller (`n/q3f_mod_call_memo` and `n/q3f_script_call_memo`, both dropped by arm (a)). Arm (a)'s JSX exclusion was a scoping choice at step 0, not a finding that JSX to a wrapper is a static binding. The checker resolves `F` to the `VariableDeclaration`, not to the arrow.
- No measured row has this shape: the five unbound JSX removals on SecBench have module-file targets.
- Recommended: no code change. Add one sentence to the spec. If the owner later wants symmetry, extend arm (a) to JSX rather than widening the exception. Does not apply if the owner decides JSX to a `memo`/`forwardRef` wrapper is in the contract; that would be a new rule with its own measurement.

### N3 — SMELL, IMMATERIAL: the binder change alters one outcome outside E5, and the packet does not say so

- `n/e_dup_comment`: a script file with `import /* l */ f = N.g;` and a same-file `function f`. main and the step-1 head resolve `f(x)` as LocalDef; the step-2b head drops it as a duplicate declaration.
- This is correct. It equals the uncommented form on all three binaries (`n/e_dup_plain`), and TypeScript rejects the program.
- It is a behaviour change at R4, outside the E5 rung. The 581-root identity shows no corpus instance. State it in one line of the pull-request description.

### N4 — SMELL, IMMATERIAL: two "unrelated name must not exempt" negative rows are decided by arm (a), so they cannot fail from over-collection

- In `e5_import_equals_and_awaited_import_evidence_preserve_fallbacks`, the rows `import g = require('./callee');` and `const { g } = await import('./callee');` call an unbound `f` whose target is a module export. Arm (a) refuses them whatever the evidence set holds. The same is true of the `require_unrelated_trivia_*` and `await_unrelated_trivia_*` rows.
- The property is pinned elsewhere: the direct assertion that the evidence set contains `renamed` and not `f` (lines 406-417), and the alias rows (`await acquire(...)`, `import f = N.g`).
- Recommended: relabel those rows, or give one of them a local alias so arm (b) alone decides. Not a blocker.

### N5 — SMELL, IMMATERIAL: the stated dynamic-import limits are "head drops, main keeps", and the limit list is incomplete

- Run, closure shape, target `export function f`: `(await import(...))`, `await (import(...))`, `await import(...) as any` and `.then(m => m)` are kept by main and dropped by the head (`n/l1_outer_parens`, `l1_inner_parens`, `l1_as`, `l1_then`).
- By source the first three bind the same export as the plain form R1 exempts. The pinned checker resolves all of them, and the exempted plain form, to the caller's own binding element. The packet's oracle would therefore call them UNDECIDED, not CORRECT, and `CLAUDE.md` puts `import()` re-acquisition outside the modelled contract. Under the standard both reviewers used in round 1 this is not WRONG.
- Not listed in the spec, same behaviour, present since round 1:
  - assignment-form require (`let f; f = require("./callee")`, `n/rq_assign`);
  - `require(...) as T` (`n/rq_ts_as`);
  - parenthesised require destructuring (`n/l1_require_parens`);
  - `const f = (await import(...)).f` (`n/l1_member`);
  - assignment destructuring (`n/l1_assign`).
  The checker stops at the local declaration for each, so they fall under the owner's alias policy.
- Recommended: add these to the limit list, and word the list as "dropped by E5, kept by main, undecided by the oracle" so nobody reads it as "keeps main". No code change in this slice; the step-2b brief says not to extend R1. Would not apply if the owner wants parity for parenthesised and asserted forms; that is one peel loop on each side of `await`.

### N6 — SMELL, IMMATERIAL: small residue

- `syntax_child` skips `comment` but not `html_comment`. HTML-like comments are not legal in module code, so no reachable loss.
- In `js_ts_import_value_names`, `find(identifier)` could select the entity identifier if error recovery removed the local name. That needs a parse error inside the alias; `import_parse_recovery` handling sits in front of it.
- Arm (a)'s `site.origin == Source` conjunct has no registered mutant. It is probably equivalent: indirect records carry `local_binding: Unchecked` (visible in the E5-13 kill log), and arm (a) needs `Unproven("unbound")`.
- `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/SPEC.md:959` still names `b5_nested_declarators_keep_base`. Historical document; no action needed.

---

## Question 1: closure of the round-1 findings

| Finding | Status | Evidence |
|---|---|---|
| sol W1 / Opus F2: module caller to script global | Closed as ruled (R2). One residual shape of the same class is N1 | Kept on the step-2b head (run): sol `module_global_js`, `module_global_ts`; Opus `t01`, `t02`, `t03`, `t05`, `t08`, `t22`, `t27`, `t28`, `t32`, `t33`. Test `e5_unbound_module_to_script_global_stays_exact_with_negative_twins` with alias, parameter, module-target and C-target twins. Mutants E5-12, E5-16, E5-17, E5-18 killed, each on the expected assertion |
| sol W2 / Opus F1: import-equals | Closed | Kept (run): sol `import_equals`, `import_equals_only`; Opus `t04`, `t04b`, `t04c`, `t23`, `t24`. Renamed twins stay refused (run): `n/e_export_alias_renamed`, `n/e_exp_req_renamed`, `n/e_term_twin`, `n/e_local_trivia_twin`. The two T `format` rows are still removed: T b_only has 2 `Unproven("import")` WRONG rows. Mutants E5-09, E5-10, E5-14, E5-15 killed |
| Opus F1, related: awaited dynamic-import destructuring | Closed (R1 parity) | `t29` and sol `dynamic` kept; `acquire` and unrelated-name twins refused; mutant E5-11 killed; F's four UNDECIDED targets now kept |
| sol W3 / Opus F3: indirect records | Closed | Kept (run): Opus `t18`, `t20`; sol `ordinary_indirect`, `jsx_indirect_renamed`. Source-origin JSX alias still refused (unit test, and X's 5 `MayCall alias` JSX WRONG rows are still removed). Mutant E5-13 killed; its log shows the site as `origin: IndirectResolution` |
| sol S1 / Opus F6: controls and mutants | Closed | `e5_arm_a_binding_and_jsx_controls_hold_negative_target_constant` asserts file, name, span, kind and confidence; fixtures `m1`, `m2`, `m3` and sol `arm-a-alias-control` kept (run). Merge is tested in both directions. Mutants E5-05 to E5-08 killed. Earlier-rung controls are labelled. All 19 registry patterns occur exactly once in their source file and every selector names an existing test (checked by script) |
| Opus F4: D12 tests vacuous, PD-29 would survive | Closed | Six `r2_d12_*_script_*` tests. PD-29 is killed in all six at `js_param_defs_tests.rs:748`, the D12 assertion itself. The early-return branch now asserts that a script-global fixture cannot take it. `d12s` stays Exact on the head (run) |
| Opus F5: dead guard, always-true conjuncts | Closed | Guard removed; `graph_target_resolution` is reached only from `rust_scope_graph_resolution`, which is called only under the Rust check (`src/resolution.rs:2345-2406`). The two conjuncts are gone, with a comment. The spec sentence is corrected |
| Opus F7: SecBench double count | Closed | Recomputed from the step-2 rows: 170 / 354 per root; the two `total.js_3.4.6` roots hold 36 W / 61 I each, so 134 / 293 distinct; 11 contributing roots, 10 distinct sources. The spec states all of it |
| Opus F8: b5 table against main | Closed (by reading) | The spec lists ten values per grammar, four from arm (a) and six from arm (b); the test is renamed |

Measured totals recomputed from `union-target-adjudications.jsonl`: X 19 / 0 / 11 / 1, T 201 / 0 / 0 / 2, SB 170 / 0 / 0 / 354. They equal the packet and the round-1 prediction.

## Question 2: is the code the fold added sound?

- **Unbound exception.** Sound for `Unproven("unbound")` sites; not complete, see N1. Boundaries (run, `r2-exception.txt`):
  - Nested declarations in a script file, CommonJS top-level functions and `.cjs` targets keep main's edge, as the spec states.
  - C and Python targets are refused from a module caller and kept from a script caller.
  - Method, object-pair and member-assignment targets are refused.
  - Parameters, catch bindings, class names, type-only imports and ambient declarations never reach the exception.
  - The extension test on the target and the language test in the fact builder cover the same set (`js, jsx, mjs, cjs, ts, tsx`).
  - Warm-cache lifecycle on the head (`n/life2`, one shared cache by design), with a cold control: script target, module target, back, alias caller, import-equals, renamed import-equals, back, unrelated edit. Every step gave the expected answer.
- **Import evidence.** Sound.
  - It only adds exemptions, so an over-match can only keep main's behaviour.
  - Renamed aliases, `require;` with a real semicolon, non-string and two-argument forms stay refused.
  - Comments at every position are handled (run: `n/e_*`).
  - Evidence is file-wide by name, the same breadth as the existing `require` evidence.
  - `import type f = require(...)` followed by a value call is dropped by arm (a) (`n/e_importtype_req`). TypeScript rejects that call, so the drop is right.
- **Binder change in `js_ts_import_value_names`.**
  - Scope: the grammar rule is `'import' identifier '=' (identifier | nested_identifier) ';'`, so the local identifier is the first named child unless an extra precedes it. The change can only affect an `import_alias` whose first named child is not the local identifier: a comment in practice; an `html_comment` or an `ERROR` node in principle.
  - It is right: the commented spelling now binds the same name as the uncommented one.
  - It does reach beyond E5 for such an alias (N3). The function has one caller, the scope index.
  - It is pinned by the trivia table rows, not by a registered mutant.
- **Dead guard.** Removal is correct. One site mints JS/TS `free_single`, and it is guarded.

## Question 3: arm (b) boundary, Source-origin JSX and `super(...)`

No concrete WRONG found. I agree with the origin-only cut.

- **`super(...)`.**
  - `function super` is a syntax error (checked with `node --check`), so any callable named `super` is an inferred property or method name and can never be what `super(...)` binds.
  - `n/sup_mod` and `n/sup_mod_pair`: main resolves `super(x)` Exact to `api.super = function…`; the head drops it. The script twin `n/sup_script` keeps main's wrong edge, which is the recorded follow-up.
- **Source-origin JSX.**
  - Unbound JSX to a script-global function declaration, direct arrow binding, function expression or self-named wrapper argument is kept (`t28`, `n/q3b`, `q3c`, `q3h`; `<F></F>`, `<F>text</F>`, attributes and fragments in `n/pos2_mod_tsx`).
  - Unbound JSX to a module export or an object property is refused, and the checker agrees it is unresolved (`n/q3g_mod_pair`).
  - JSX through a local alias is refused under the alias policy.
  - The only asymmetry I could construct is N2, which is not a static binding under the slice's own definition.
  - JSX inside a merged namespace body would fall under N1.
- sol's narrower cut would give up X's five Source-origin JSX WRONG removals and re-admit the `super` rows for no correct resolution gained. I would not take it.

## Question 4: stated limits

- Module to CommonJS top-level function, and module to a nested declaration in a script file: these keep main's behaviour (run: `n/r2_cjs_top`, `n/r2_nested_script`). Not a loss.
- Exported-require recovery with nested parentheses or an argument assertion: TypeScript rejects those programs. Not a loss.
- Renamed namespace aliases: correct refusals.
- Parenthesised or asserted awaited imports and `.then`: the head drops what main keeps. By source three of the four are the same binding as the exempted form; by the packet's oracle they are UNDECIDED. Not WRONG under the round-1 standard; see N5 for the wording and the unlisted neighbours.
- The limit that is a checker-confirmed correct resolution lost is not on the list: N1.

## Question 5: anything new and material from the fold

Nothing material was introduced by the fold. N1 is a residue of the round-1 class that the fold's condition did not cover. N3 is the one behaviour change the fold introduced outside E5; it is correct and needs disclosure only.

## Checked and found sound

- All 48 round-1 Opus fixtures and 11 sol fixtures on three binaries. My results equal the packet's four-way table for every Opus fixture. Every fixture that demonstrated a round-1 WRONG finding is kept by the step-2b head.
- The frozen step-2b binary is bound to the reviewed source.
- All 19 mutants are reported killed and admissible, each with a real assertion failure in its log (no compile errors, no timeouts).
- Head-to-head identity summary: 581 roots, 0 removed and 0 added call rows and graph rows.
- Cache-version comment, shared `is_module` helper, relabelled earlier-rung controls, drafted `CLAUDE.md` sentence.

## Not checked

- Cargo, the test suite and the mutation gate were not run. Test and mutant results are the packet's receipts, which I read.
- Private corpus F evidence was not opened; F numbers are the controller's. No `f-e5-*` or `frontend-portal` directory was opened.
- The 758 target and 1,938 graph adjudications were not re-adjudicated. I recomputed totals and binder-state splits from the rows only.
- The 581-root head-to-head identity, the non-JS byte-identity claim, inventories, navigation and module-dependency deltas, the Tier-A matrix and quick runs were not re-derived.
- Of the packet's 162 four-way fixtures, I reran only the Opus and sol round-1 ones, plus my own versions of about a dozen trivia shapes.
- The edge-cache sidecar was not exercised: the frozen head binary reports dirty build inputs and skips the warm sidecar load. MCP refresh was not executed.
- `CONTROLLER-e5.sh`, the census probes and `step2b/ast-probe.rs` were not reviewed.
- F8's ten-value b5 table was checked by reading, not rerun.
- Files with parse errors, Flow syntax and `.mts`/`.cts` were not probed.
- sol's round-2 report and scratch directory were not opened.

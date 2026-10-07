# FABLE-DESIGN-R4 — PR-B "callback identity": the parameter-environment seam

**Written:** 2026-10-06 · **By:** Fable 5.1 (independent designer, read-only) · **Answers:** `FABLE-ESCALATION.md` (R4 loop 1/3)
**Read:** REPORT.md, STOP.json, HANDOFF.md, PROBE-LOG.md, default-invocation-losses.json (18 rows), matrix-cell-records.jsonl (763 cells, re-tabulated below), MATRIX-exceptions.md, classify_matrix.py, prB-repair-r4-brief.md; SPEC-prB.md (B-D1…B-D14, §8, R2/R2b folds), MEASUREMENTS-prB-R2b/R3/R4.md, reviews prB-spec-r2-{opus,sol}.md and prB-fold-confirm-opus.md (C1); HANDOFF-controller-2026-10-05.md; my DESIGN-INPUT §3. Source read at `origin/wip/js-param-defs-prB-r3` = `310de8b5` (R4 WIP; `1af4301f` + R4-src.patch) via `git show`; main `da0604b3`. No git writes, no corpus runs, no frontend-portal.
**Mechanism claims cite file:line in `310de8b5`.** Claims I did not probe are tagged `[STATIC]`.

---

## 0. Recommendation (three lines)

1. **Choose (b): scope PR-B to fail closed on the whole parameter-expression seam** — byte-identical to main for named callables, no formal rows for synthetic callables — delete the R2b/R3/R4 copy machinery, ship the measured value (f8c768b3-class: 0 LOST-correct / 0 ADDED-WRONG on X/T/SecBench, F +9,923/−891 modulo one function), and certify with a mechanical per-callable row-equality proof in **one** review round.
2. **Do not implement (a).** The question "invoked vs created" is undecidable syntactically (class static blocks, getters, a later default calling the closure, closures passed to unknown callees); the only sound in-model answer is a *lexical may-source with a NameOnly(CfgIncomplete) label*, and that answer makes most of R3/R4's conditional-evaluation machinery unnecessary. That is a new model, not a repair — it belongs in its own slice (§5), where the design is one page and provably closed.
3. **Owner decisions needed now:** (i) accept (b) and defer the S1 CLAUDE.md sentence to the follow-up slice (under (b) there are no copy rows for it to grade); (ii) accept that the one measured seam instance (X extra-asciinema, 4 rows) reverts to main's rows (1 correct, 3 inherited false-Exact, disclosed as E12); (iii) authorise the matrix parity predicate to become per-binding.

---

## 1. What the evidence says

**The seam, precisely.** Every WRONG since the last complete measured candidate `f8c768b3` (R3-SOURCE-1, R4-DESIGN-1, Opus C1's nine fixtures) lives in callables where **(S)** `js_ts_parameters_have_expressions` (`ast_callback_identity.rs:342`) **and** the body function-scope rebinds a formal name (`js_ts_function_scope_binds`, `:712`, or a body-level `function n(){}`). That is the exact boolean `separate_body_binding` in `js_ts_binding_scope_at` (`:321–327`): when true the formal and the body `var` are two binding environments, and the only thing re-admitting formal→body rows is the "copy" model: `js_ts_parameter_copy_binding` (`:368`) → `parameter_copy_jobs` (`data_flow.rs:693–748`) → `copy_edges` (`:1082–1100`) → a second RD solve over an `implicit_entry` Def (`:1205–1281`, `reaching.rs:81/232/406`) → `parameter_reaching_sources` (`reaching.rs:39–72`), `js_ts_parameter_write_is_conditional` (`:428–517`), `js_ts_statement_write_dominates` (`:519`), `js_ts_write_end` (`:405`), `grade_parameter_environment_copy` (`data_flow.rs:215`). Roughly 450 LOC across four files, all conditional on (S).

**Anatomy of the 18 losses.** Main's "correct" row is `h:f@22(d)→use NameOnly(SameLine)`: main has one flat binding per name and line-inventories the nested arrow's `f=5` as an ordinary Def of `h`. Head's named pass still registers that Def, but under (S) it is a Def of the *parameter* environment, and the copy's source filter requires `js_ts_span_in_own_scope` (`data_flow.rs:711`) — so the write inside the arrow is never a copy source. The deferred-closure control (`E-deferred-default-write-*`) loses the identical main row and is classified OK, because there Node reads 1. So the open question is not "which rows" but "which lexical writes count as sources of the copy, and with what label".

**Anatomy of the 272 parity failures.** `classify_matrix.py:85` tests `main == head` over the *whole callable*. Re-tabulating `matrix-cell-records.jsonl` by what actually differs:

| Family | Owner | n | What differs (js) | Verdict |
|---|---|---:|---|---|
| destructured | callback | 159 | head ADDS rows for the *plain* sibling formal `f` only; `[d]` has no rows either side | not a fail-closed violation; predicate is per-callable where per-binding was meant |
| destructured | named | 45 | head LOSES one zero-width line-level Use on the `function f`/`catch(f)` declaration line; the `P-simple-*` sibling (no destructuring) loses the same row and is classified **OK** | authorised checker-WRONG removal, unrelated to destructuring |
| destructured | named | 42+2 | the (S) seam (`{d=(f=5)}={}` + `var f`): copy rows vs main's flat rows | **is** the seam; resolved by §4 |
| direct eval | both | 12 | named: (S) seam; callback: ADDED Exact rows where `eval("f=5")` can mutate `f` | needs a real fail-closed rule (§2 Q3) |
| legacy `arguments` | both | 12 | named: (S) seam or parity; callback: ADDED Exact `f@18→use` while sloppy mapped `arguments[0]=9` aliases `f` (Node reads 9) | needs a real fail-closed rule (§2 Q3) |

So 204 of 272 are a tooling defect; 44 are the seam; 24 need a per-binding refusal rule for synthetic owners. None of the 272 is a LOST-correct row beyond the 18 already counted.

**A SMELL in the R4 model worth recording.** `E-two-writes-named-*` head emits three **Exact** rows into one Use (`f@11`, `f@17`, `f@26 → f@47`). Exact is the single-reaching-definition grade (CLAUDE.md); three Exact sources for one read is the multi-target-Exact shape this project already treats as a defect (owner-key lane). Under (b) it is moot; under §5 it is fixed by construction.

**Measured incidence.** One function on all public corpora: X extra-asciinema (`MEASUREMENTS-prB-R2b.md:19`, 4 rows: 1 CORRECT, 3 false-Exact on main, which the copy model dropped). F: unmeasured for this seam specifically, but the three R2b–R4 loops surfaced zero corpus rows outside synthetic fixtures. Shapes like `function(f, d=0){ var f … }` are `no-redeclare` lint errors; the one real instance is bundled/minified code.

---

## 2. The four design questions

### Q1 — Invoked captured writes vs closure creation
**Do not distinguish them; the distinction is not syntactically decidable and the sound model does not need it.** Counterexamples to any "IIFE detector": `d=class{static{f=5}}` (runs at definition; `class_static_block` is a callable boundary for the own-scope walk), `d=({get x(){f=5}}).x` (getter), `get=()=>{f=5}, d=get()` (invoked by a *later* default), `d=run(get)` (invoked by an unknown callee, or not), `d=c&&get()` (conditional), `d=(()=>{throw 0})()` (body never runs). A lemma from the spec closes the class instead: **a parameter-environment binding `f` can be written during FunctionDeclarationInstantiation only by (i) an assignment lexically inside the parameter list, (ii) code lexically inside a callable/class defined in the parameter list, or (iii) direct `eval` inside the parameter list.** (`arguments` cannot alias it: with parameter expressions the object is unmapped; property effects never write the base; exceptions only prevent the copy.) Hence the source set of the copy is a *lexical* set — every lvalue write to base `f` whose span lies inside `parameter_binding_region(c)` — and (ii) contributes **may-sources of unknown timing**, labelled `NameOnly(CfgIncomplete)` exactly as the B8 rule already labels captured accesses in nested callables. No "invocation" concept enters the model; B-D5 (no interprocedural edges into or out of synthetic owners) is untouched because nothing here is a call edge.

### Q2 — Source identity and timing model (coherent with B-D2 and B-D5)
If the copy model is kept (§5), replace R3/R4's ordered must/may transfer with: `sources(c,f) = {formal Def} ∪ W`, where `W` = lexical param-region writes to `f` (own-scope **and** nested, W4 with-bodies excluded → cell is OUT-OF-MODEL). Labels: a copy row is **Exact iff `W = ∅`** and the body-side RD proves no kill (this is S1's case, the only one where "the single reaching definition" is literally true); if `W ≠ ∅` **every** copy row (formal and each write) is `NameOnly(CfgIncomplete)` — the CFG does not model conditional parameter evaluation, and that is the honest doubt word. This deletes `parameter_reaching_sources`, `js_ts_parameter_write_is_conditional`, write-end ordering and literal-branch pruning: they buy Exact labels in constructible cases at the price of an open class. **B-D2 amendment:** "writes inside nested callables are kill-only" stays for *body* nesting; writes inside nested callables **in the parameter region** are emitted only as copy-row sources (never as Defs reaching reads of a different binding). **Adjudicator amendment** (`adjudicate.cjs` rule 2): a synthetic-owned Def whose innermost container is a callable *inside the owner's parameter region* is admissible iff it is a copy-row source. B-D14 unchanged. **B-D5 unchanged.**

### Q3 — A real fail-closed main-row-retention mechanism for OUT-OF-MODEL cells
"Fail closed" must be stated **per binding**, not per callable, and it has two shapes because main has rows only for named owners:

| Binding condition (JS/TS/TSX, per formal name `n` of callable `c`) | Named pass | Synthetic pass |
|---|---|---|
| **SEAM(c,n)**: `js_ts_parameters_have_expressions(c)` ∧ (`js_ts_function_scope_binds(body(c), n)` ∨ body-level `function n`) | flat scope for `n` (= main) | refuse `n` (no Def, no walk) |
| **EVAL(c)**: a direct `eval(` call lexically anywhere inside `c` (any nesting) | main rows (flat path, no change) | refuse **all** formals of `c` |
| **ARGS(c)**: non-strict `c` with simple parameters and an `arguments` reference anywhere in `c` outside nested non-arrow callables | main rows (no change) | refuse all formals of `c` |
| destructured formal | already refused (PR-A D11) | already refused |
| `function*` expressions | not a pass (non-goal) | not a pass |

Named passes keep main's rows because the flat path *is* main's mechanism; synthetic passes refuse because main had nothing to retain. Refusal of a binding = skip the `param_occurrences` entry at `data_flow.rs:654–748`; body-local Defs of the same name proceed on the ordinary own-scope path (in-model, already measured). Strictness for ARGS reuses R4's contextual-strictness classifier. The parity check in `classify_matrix.py` becomes: for each binding `n`, rows whose Def is `n`'s formal or whose endpoint lies in `n`'s parameter region are equal main↔head (named) or empty (synthetic).

### Q4 — Certification
Bounded slice, cap and counterexamples are in §4.3–4.5 for (b) and §5.3 for the follow-up.

---

## 3. Decision and proportionality

| | (a) implement Q1/Q2 inside PR-B | **(b) fail closed on the seam, ship** | (c) keep R4 machinery + IIFE patch |
|---|---|---|---|
| Goal-level result it serves | 3 false-Exact rows removed on X; constructible-only gains | PR-B's measured value (SecBench joint 90/97 potential, F +9.9k) ships now | same as (a) with worse convergence |
| Loops consumed | design change + B-D2/adjudicator amendments + re-review of the copy block: realistically both remaining loops, review open-class risk high (3 loops of same-kind findings = open class per convergence discipline) | 1 loop, 1 review round; the proof is mechanical | reviewer finds static-block/getter/`?.` next round |
| Risk to measured value | medium (new labels across 3 corpora + F) | none outside one function | medium |
| Code | +~60 / −~250 LOC on a reviewed-thrice block | −~450 LOC, +~60 | +~40 |

**Decision: (b).** Under global steering (proportionality; convergence: open-class findings at cap → park and escalate to design), continuing to patch the copy model inside PR-B is not justified by a population of one function. The deleted machinery is preserved in git (`310de8b5`) and its matrix (763 cells + Node oracle) is the ready-made test bed for the follow-up slice.

---

## 4. PR-B under (b): LLD for the repair engineer

### 4.1 Source changes (`310de8b5` base)
| File | Change |
|---|---|
| `src/ast_callback_identity.rs:321–327` | Delete `separate_body_binding`; the `if` becomes `if !params_bind && !self_bind { region = body?; }`. Formal and body `var` share one scope id again (main). Add `pub(crate) fn js_ts_seam_binding(&self, c, n) -> bool` = SEAM(c,n) (reuse `js_ts_parameters_have_expressions`, `js_ts_function_scope_binds`, the body-`function` check now in `js_ts_parameter_copy_binding:383–394`). Add `js_ts_direct_eval_anywhere(c)` and `js_ts_mapped_arguments_possible(c)` (strict classifier from R4). |
| `src/ast_callback_identity.rs` | Delete `js_ts_parameter_copy_binding` (368–401), `js_ts_parameter_write_is_conditional` (428–517), `js_ts_statement_write_dominates` (519–~575) and `js_ts_write_end` (405) if no remaining caller. Keep R4's early-error/BoundNames/comment logic (not in the seam). |
| `src/data_flow.rs` | Delete `parameter_copy_jobs` (645, 693–748), `copy_edges` (1082–1100), the copy solve (1205–1281), `grade_parameter_environment_copy` (215). In the `param_occurrences` loop (654) add: `if synthetic && (seam || eval || args) { continue; }` before Def registration; compute `eval`/`args` once per callable. Named passes: no new branch. |
| `src/cpg/reaching.rs`, `src/cpg.rs:64` | Delete `parameter_reaching_sources` (39–72) and `DefSite.implicit_entry` (81, 232, 406) plus its three test initialisers. |
| `src/cpg_cache.rs` | one `CACHE_VERSION` step (rows change vs 111); re-pin PD-11 / P2-M11 once. |
| `CLAUDE.md`, `SPEC-prB §0` | **Remove the S1 sentence** (no copy rows exist); add §0 decision B-D15 "parameter-expression seam fails closed (named = main, synthetic = refused); model deferred to slice js-param-env". Non-goals: add eval, mapped `arguments`, seam. |

### 4.2 Tests (each must fail on `310de8b5`)
- Replace `r2b_copy_same_line_and_default_assignment_negatives`, `r3_c1_rows_use_the_standard_rd_labels`, `r3_diagnostic_skipped_defaults_preserve_each_reaching_source`, `r4_one_default_retains_only_its_normal_completion_sources` with **golden-row tests**: for the nine Opus-C1 shapes, R3's `d=(f=5),e=(f=7)`, R4's IIFE and deferred closure, and `E-eval-default`, `E-default-arguments` — named owner: head rows for `f` equal the main rows recorded in `matrix-cell-records.jsonl` (bytes + labels, all three grammars); synthetic owner: zero rows with a formal-`f` Def or a parameter-region endpoint, body `var f=2 → use(f)` rows unchanged.
- Negative controls per new path: `(f, d=0) => { use(f) }` (no body rebinding → unchanged Exact rows, no refusal); `function h(f){ var f; use(f) }` (no parameter expressions → unchanged); strict-mode `arguments[0]=9` callback (unmapped → **not** refused); `eval` as a property (`o.eval(...)`, indirect) → not refused; nested non-arrow function using `arguments` → outer not refused.
- Mechanism pin: `js_ts_binding_scope_at(formal_byte, "f") == js_ts_binding_scope_at(body_var_byte, "f")` for a SEAM fixture; unequal for a nested-callable formal (E3 fence still live).
- Keep R4 early-error groups and `refused_shapes_gain_no_def` as they are.

### 4.3 Proof of byte-identity to main (the certificate)
1. **SEAM census probe** (`probes/seam_census.rs`, same pattern as `census_b.rs`): emit `(file, owner start..end, owner name|<cb@>, n, kind ∈ {SEAM, EVAL, ARGS})` for every JS/TS/TSX callable on X, Xi, T, SecBench (controller: F).
2. **Row equality:** `rowdiff.py` new head vs `f8c768b3` dumps. *Every* differing row must have its Def owner in the census. For named census entries: `rows(head)[owner, n] == rows(main)[owner, n]` byte-for-byte including labels (main dump = `prB/bin/prism-base-da0604b3-bytes`). For synthetic census entries: no head row with a formal-`n` Def or a parameter-region endpoint. Any row failing both tests is a STOP.
3. **Early-error exception:** a differing row in a callable the census does not list is admissible only if `node-oracle.cjs` proves that callable text a SyntaxError (R4 refusal); report the count.
4. **Matrix rerun** with the per-binding parity predicate: expected 0 DESIGN-CHANGE, 0 LOST-correct, all SEAM/EVAL/ARGS cells PASS per binding; publish the new counts.
5. Non-JS/call-site/nav identity, nextest, mutants, fmt, clippy, Tier-A matrix + quick, perf, F — as SPEC §5. Expected F delta vs `f8c768b3`: only census functions.

### 4.4 STOP conditions
Any differing row outside the census or the SyntaxError exception; any named census row not equal to main; any synthetic census row with a formal-`n` endpoint; any non-JS byte change; F delta touching a non-census function; a second review-round finding that is a new *family* (then park PR-B as-is — the artifact is preserved, never restarted).

### 4.5 Review cap
One round, Opus ∥ sol, scoped to: the three predicates (SEAM/EVAL/ARGS — ask each reviewer for one constructible callable where a predicate is false but main and head differ for a formal), the deletion's completeness, and the certificate's admissibility. Convergence class expected: closed-enumerable. If round 1 is clean → merge on green CI.

---

## 5. Follow-up slice `js-param-env` (the real model)

### 5.1 Design (one page, closed by the Q1 lemma)
- Restore `separate_body_binding` (binding-correct separation, R2) and a copy row set per §2 Q2: `sources = {formal} ∪ W_lexical`; Exact iff `W=∅` ∧ body RD proves; else all rows `NameOnly(CfgIncomplete)`.
- Body side: keep Opus-C1 rule 1 (drop a copy row only on a *statement-level unconditional* dominating write; otherwise emit with the RD label) — the one piece of R3 worth keeping.
- OUT-OF-MODEL stays fail-closed as §2 Q3 (eval, with-body in the parameter region, mapped `arguments`).
- Synthetic owners: copy-row sources inside parameter-region nested callables are admissible (B-D2 + adjudicator amendments, §2 Q2).
- S1 sentence lands here, reworded: "…the copy is Exact only when no write to the formal exists in the parameter list".

### 5.2 Why it is closed
The lemma enumerates the write mechanisms; the model admits all of (i)+(ii) lexically and refuses (iii). There is no "invocation" to get wrong, so the R3/R4 open class (ordering, literal branches, sequence expressions, optional chaining, static blocks, getters, later-default invocation, unknown callees) collapses into one rule. The only remaining precision question is *labels*, and the chosen label is the project's existing doubt word for exactly this situation.

### 5.3 Certification
Reuse the R4 matrix (763 cells, Node oracle, `classify_matrix.py` per-binding) plus a counterexample set that must be in the fixtures before dispatch: IIFE; deferred closure never called; closure called by a later default; closure passed to an unknown callee; `c && get()`; class static block; getter; `o?.m(f=5)` with `o` nullish; `(f=5, f=7)` sequence; default that throws; nested `with` (OOM); `eval` (OOM); each × named/callback × sloppy/strict/module × JS/TS/TSX. Expected: 0 LOST-correct vs main, 0 Exact rows whose Node-proven value differs from the Def's. Review cap 2 (Opus ∥ sol). Measure: X extra-asciinema reverts to 1 correct Exact row + 0 WRONG; everything else byte-identical to the (b) head. Size: ~150 LOC, one cache step.

---

## 6. Owner decisions requested
1. (b) over (a): ship PR-B fail-closed; defer the model to `js-param-env`.
2. Withdraw the S1 CLAUDE.md sentence from PR-B (re-land with §5).
3. Accept the one measured reversion (X extra-asciinema: 3 inherited false-Exact rows retained, disclosed as E12 pre-existing, not ADDED).
4. Per-binding parity predicate in the matrix tooling.
5. New non-goals for PR-B: direct eval, mapped `arguments`, the parameter-expression seam — each with its refusal rule stated.

*What I did not verify:* I did not build or run either binary; line numbers are from `git show 310de8b5` and the R4 patch. The "flat path reproduces main" claim rests on `js_ts_parameters_have_expressions` having exactly two callers in `src/` — `js_ts_binding_scope_at:323` (the fence) and `js_ts_parameter_copy_binding:376` (the copy) — which `git grep` on `310de8b5` confirms; whether the flat walk's RD/labels are otherwise byte-identical to main for seam names is what the §4.3 certificate proves, not something I assert `[STATIC]`.

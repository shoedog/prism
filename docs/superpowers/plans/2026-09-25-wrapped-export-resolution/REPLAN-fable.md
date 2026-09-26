# Re-plan after the spec review cap: wrapped-export S1 (Fable, 2026-09-25)

**Status:** planning only; builds on the reviewed r2 packet (`1d4320b1`). Nothing under `src/`, `tests/`,
`eval/` or `Cargo.*` changes. The owner picks a branch in §2; until then SPEC §0 D2/D6 and §3.2 are **pending
owner choice** and this file is the design of record for the disagreement.

**Evidence:** `~/prism-evidence/wrapped-export/replan/` (`EXPECTATIONS-pre-run.md` written before every run,
`RESULTS.md`, `probes/`, `MANIFEST.sha256` `06203def…`). The controller's precedent probe is at
`~/prism-evidence/wrapped-export/spec-reviews/precedent-probe/`. Corpus F appears here only as aggregates.

## 0. Summary

- The r1→r2 finding sequence **is** open-class under sol's standard, and it stays open-class under sol's own
  proposed fix (strengthened D6(b)): the React object can be acquired in-file with no `require`/`import`
  spelling at all (`arguments[1]('react')`, `module.require('react')`; Q9a–c pass r2 custody as Exact in both
  modes). No finite denylist closes it, because JavaScript has ambient authority. Closure is only ever
  **relative to a declared analysis model**.
- Under the Exact contract prism applies everywhere else, every WRONG in r1 and r2 is **out of model**. On
  `main`, `import_member` (and `import_qualified`, and Python) emits Exact through same-file direct writes,
  `require` re-acquisition, `Object.defineProperty`, `__defineGetter__` and `eval` (Q1–Q6, plus the controller's
  probe). Two of those are sol's r2 mechanisms verbatim. The only same-file mutation prism models is the CJS
  producer barrier over the file's **own** `module.exports` (Q8), where mutation is the construction idiom.
- The packet's mistake was r2 SPEC §3.2's sentence "Within the file, the model is closed." That claim invited
  a class of counterexamples the project never set out to defend. Sol's r1 self-critique already conceded the
  way out: W1 "does not apply under a stated analysis model that … excludes local mutation. The packet states
  neither."
- **(a) already contains (b).** The 19 mode-(b) rows are a strict subset of the 107 mode-(a) rows (|b−a| = 0),
  so "both" is 107, not 126. And under sol's *strengthened* (b), which refuses any file with a default or
  namespace React binding, 12 of those 19 fall away (`FilledButton.tsx`, `LibraryMenuSection.tsx` both use
  `import React, { forwardRef, … }`): **7 on X, 0 on F**.
- **Recommendation: Branch P** (project-standard Exact, custody dropped, model stated project-wide), 107 / 4
  edges, about 325 src lines, one more review round. §2.4 gives the reasoning; §5 gives the reviewer brief.

## 1. Diagnosis

### 1.1 Under sol's standard the class is open, and strengthened (b) does not close it

Sol's standard, stated across r1 and r2: an Exact edge must be sound against **any same-file runtime mutation
or acquisition** of the React module object. Each round found new forms (r1: direct member write; r2: `require`
re-acquisition with a named-only import, computed specifier, inherited `__defineGetter__`). Sol's proposed exit
is strengthened D6(b): named ESM imports only, no default/namespace React binding, and "fail-closed treatment
of every other same-file React acquisition".

That last clause is not a decidable syntactic set. Under the bundler/CJS-interop execution model that sol's own
W1–W3 depend on (M15), the module body runs inside `function (exports, require, module, __filename,
__dirname)`, so `arguments[1]` **is** `require` and `module.require` is a second handle. MEASURED on the r2
prototype (`RESULTS.md` Q9):

| Input (lib.tsx) | Mode | Result |
|---|---|---|
| `import React`; `arguments[1]('react').forwardRef = fake`; `React.forwardRef(fn)` | a | Exact `Island@6-8` |
| `import { forwardRef }`; `module.require('react').forwardRef = fake`; `forwardRef(fn)` | **b** | Exact `Island@6-8` |
| same input | a | Exact `Island@6-8` |

After those, the next round would bring `globalThis.require`, `new Function("return require")()`,
`process.mainModule.require`, `import.meta`, `Reflect.get(...)`. Each is a one-line denylist addition, and each
new round finds the next one. A denylist over an open vocabulary is the definition of open-class. The only way
to make "closed" true is to declare what is in model. Once that is done, D6(a) and D6(b) differ in *where the
line is drawn*, not in whether a line exists.

### 1.2 Under prism's existing Exact semantics the sequence is out of model

Prism has no doc comment defining `ResolutionConfidence::Exact` (`src/resolution.rs:26-29`). The contract is
established by (i) what the ladder does, (ii) `CLAUDE.md`'s confidence paragraph, and (iii) the P4 lineage.

**(i) What the ladder does.** Base binary `prism-base-12ca6e8e`, expectations written first, all admissible
(`RESULTS.md`):

| Probe | Same-file input on `main` | Result |
|---|---|---|
| controller | `import { f }`; `require("./lib").f = g`; `f()` | `exact import_member` |
| Q1 | producer `export function f(){}` … `f = g` | `exact import_member` |
| Q3 | `import * as L`; `L.f = g`; `L.f()` | `exact import_qualified` |
| Q4a | `Object.defineProperty(require("./lib"), "f", {value: g})`; `f()` | `exact import_member` |
| Q4b | `require("./lib").__defineGetter__("f", () => g)`; `f()` (sol's r2 W3 mechanism) | `exact import_member` |
| Q5 | `eval("0")` in the importing file; `f()` | `exact import_member` |
| Q6 | Python `from lib import f; import lib; lib.f = g; f()` | `exact import_member` |
| Q8 | CJS producer `module.exports.f = f; Object.defineProperty(module.exports, "f", …)` | **drop** (barrier) |
| Q8c | Q8 without the reflective line | `exact import_member` |

So Exact `import_member` is a **static-binding** claim: the site's name resolves, by the language's import and
export syntax as prism models it, to exactly one declaration. It does not assert what the heap holds at call
time. This is uniform across the JS rungs and across languages. The single exception (Q8) is principled: CJS
exports are *built* by mutating `module.exports`, so a producer barrier over that object is modeling the idiom,
not defending against monkey-patching. Sol's r2 claim that "the CJS producer precedent … does not cover these
findings, because every mutation or acquisition occurs in the analyzed file" is answered by Q4a/Q4b: the
`import_member` precedent covers same-file mutation directly, and it is the apt precedent, because the producer
file of a wrapped export is a *consumer* of `"react"`.

**(ii) `CLAUDE.md`:** "`asserted` grades the evidence path, not the heuristic's truth." Confidence is a grade of
the resolution evidence, not a runtime guarantee. The P4 doctrine ruling (codex, recorded in
`docs/analysis/prism-llm-and-accuracy-plan.md:15`) is the same: "barrel-resolved single-target `import_member`
remains grounded → Exact is correct."

**(iii) The F4 guard was static.** P4 item 1c admitted `export const f = (...) => …` and
`export const f = function … {}` only; the F4 fix (commit `2b58aa5d`) refused other initializers because "if
any same-file FunctionId named `f` existed elsewhere (e.g. a nested function), R4c could bind the export to the
wrong function." That is a **static misbinding** hazard: the fact is a name, and the name might denote another
callable. S1's span verification closes exactly that hazard (C07, C09–C11, T-N11). Runtime monkey-patching was
never part of F4, and the only other base-side "written" checks (`js_ts_module_value_written`, the P4 const-only
destructured `require`) are about rebinding a **name** in the file, not mutating an object.

The precise line, which every branch below uses: **S1 proves what a name denotes; it does not prove what the
heap holds.** Static provenance (R5, R6 P1–P5, R4 parse recovery, the span filter, R11) is in model. Runtime
mutation of a module object (r2 R7 K1–K8, in every form) is out of model, as it is for `import_member` today.

### 1.3 Verdict on the disagreement

- Sol is right that r2 §3.2, **as written**, claimed more than it proved, and right that no K-list closes it.
- The controller is right that the standard sol applied is stricter than the Exact contract prism honors on
  every other rung, in every language, on `main` today.
- Neither is settled here. It is an owner decision about the confidence contract (D2), and the branches below
  are the two coherent ways to close it. What is **not** coherent is a third way: keep custody, keep claiming
  same-file closure, and add K9–K12. That is the path that did not converge.

## 2. The owner fork

| Branch | Rule for the React object | Yield X / F (new Exact) | Budget est. src / tests / combined (§4) | Residual risk | Convergence outlook |
|---|---|---|---|---|---|
| **P** project-standard Exact (recommended) | out of model, same as `import_member`; custody K1–K8 dropped; model stated project-wide | **107 / 4** (all forms) | ~325 / ~570 / ~895 → caps **350 / 600 / 950** | a real same-file monkey-patch of `React.forwardRef`/`memo` yields Exact to the render function. MEASURED prevalence: r2 custody, which refuses far more than monkey-patches, fired on **0** of the 72 admitted declarators' files on X+F (P18) | 1 round likely: every mechanism item (JSX gate, span, barrel key, parse recovery, comparator) is already CLOSED in r2; the only new material is the stated model and the folds. Reviewer judges against the model (§5) |
| P-guard (option inside P) | as P, plus refuse a **syntactic write to the selected member** (`React.forwardRef = …`, `delete`, `++`, `||=`, destructuring: w1–w3 on `O.forwardRef`/`O.memo` only) | 107 / 4 | ~380 / ~625 / ~1,005 → caps 400 / 650 / 1,050 | same as P minus the one r1-W1 shape; it is stricter than base (Q3 shows `L.f = g` is not refused today) and is **not** a proof, so it must be labelled best-effort or it re-opens the class | as P, plus a small risk that a reviewer reads the guard as a closure claim. Mitigated by the label |
| **S** sol-standard, strengthened D6(b) | named ESM callee only; any default/namespace/`default as` React binding refuses the file; fence F1–F6 (§2.2) refuses `require`, `import()`, `eval`, `with`, `module`, `arguments` anywhere; host globals declared out of model | **7 / 0** (source read of the 19 mode-(b) rows; re-measure) | ~325 / ~595 / ~920 → caps 350 / 620 / 970 | refuses the most common React import idiom (`import React, { forwardRef }`) and 93% of the measured value; still needs a declared model (Q9), so the "sound against any acquisition" promise is not actually made | 1–2 rounds: the fence vocabulary is the residual surface (`globalThis`, `Function`, `import.meta`, `process`…). Converges only if the brief states host/ambient authority out of model, which is the same kind of statement P makes |
| Hybrid (S + member forms at NameOnly) | as S for Exact; every other admitted declarator (member forms, and named forms in files with a default React binding) binds **NameOnly** `import_member`, span-verified | 7 Exact + 100 NameOnly / 0 + 4 NameOnly | ~372 / ~670 / ~1,040 → caps 400 / 700 / 1,100 | NameOnly edges score 0.6, never `asserted`, and drop under `--min-confidence exact`; one syntactic proof yields two confidences depending only on import spelling, which is hard to explain and sets a precedent for "spelling-graded" confidence | as S for the Exact half; the NameOnly half is uncontroversial (R4c already demotes to NameOnly on multi-match, `src/resolution.rs:3428`) but adds plumbing to review |

Not on the table: NameOnly for everything (owner chose Exact; and it would leave `import_member` inconsistent
with itself), and any further K-list extension.

### 2.1 Branch P: project-standard Exact

**Analysis model (replaces r2 SPEC §3.2 in full).**

> S1 binds by static provenance. The callee must be a named or default/namespace **ESM** import binding of
> `"react"` that is unique, value-typed, unwritten as a binding, and uncompeted at module scope (R5, R6
> P1–P5); the export target is the inner function's exact span; only JSX element sites bind. Runtime mutation
> of the React module object, by any means and from any module, is **out of model**, exactly as it is for every
> `import_member` and `import_qualified` edge prism emits today (replan probes Q1–Q6). Prism grades the
> evidence path, not the heap.

**What changes in the r2 design.**

- Drop R7 entirely (K1–K8, write positions w1–w3, the occurrence walk). The reason table becomes R1–R6, R8–R11
  (R12 unreachable, unchanged). Renumber or leave a gap; leave a gap, so r2 cross-references stay valid.
- Keep R6 P1–P5. They are static-binding facts, cheap, and consistent with `js_ts_forwarded_import`
  (`src/ast/js_module_forwarding.rs:12-18`), which refuses written, type-only and competed locals today.
- Keep the hand-rolled React import parser (r2 prototype `react_imports()`, 64 lines). It is ESM-only by
  construction, which is what closes r2 SMELL 2 (§3). Retract SPEC §9's "reuse `extract_import_bindings()`
  saves 50 lines": that reuse is what let destructured `require("react")` through. If the implementer prefers
  the reuse, the origin check is "the binding's local is declared by a top-level `import_statement`", and
  T-N18 must pin it.
- Replace T-R7-K1…K8 with three **model-boundary characterization tests** (MB1–MB3) that assert the *current*
  result on the three sol inputs (r1 W1 direct write; r2 W1 `require("react").forwardRef =` with a named
  import; r2 W3 `__defineGetter__`) is **Exact**, with a comment naming SPEC §3.2. Their purpose is the same as
  C06's pinned false Exact: a future slice that changes the model does so on purpose and flips a named test,
  not by accident. Add one Tier-A fixture `react_default_member_write_out_of_model/` with `exact = true`,
  replacing r2's `react_default_member_write_refused/`.
- T-S1 loses the `React.forwardRef = fake` epoch (it is no longer a state change) and gains nothing; six epochs
  remain.

**Custody machinery: keep or drop?** Drop it (K1–K8 and the occurrence walk). Reasons: (1) it is 101 src lines
whose only measured effect on two corpora is zero refusals; (2) a partial guard with no closure claim is what
seeded "why not this form too" in r1→r2; (3) base does not refuse the same writes for any other rung (Q3), so
keeping it makes wrapped exports the one JS edge with a runtime-mutation check and invites the next slice to
ask why. **P-guard** (w1–w3 on the selected member only, ~56 lines) is offered because it is closed as a
syntactic set (the grammar's property-write positions are finite) and it pre-empts the single realistic
misuse. If the owner takes it, the SPEC must label it "best-effort refusal of a visible direct write; not a
custody proof" and MB1 flips to a drop.

**Project-wide statement: yes, needed.** Without it the next JS slice (S1b, S2, `paths`) re-litigates the same
question. Proposed `CLAUDE.md` addition, right after "`asserted` grades the evidence path, not the heuristic's
truth", pending owner wording:

> `Exact` is a static-binding grade: the site's name resolves, by the language's declaration, scope, import
> and export syntax as prism models it, to exactly one callable. It does not assert runtime truth. Runtime
> mutation of module objects (monkey-patching, reflective writes, `eval`, host globals, re-acquisition through
> `require`/`import()`), from the same file or another, is out of model for every rung and every language
> (replan probes, 2026-09-25). Producer-side mutation is modeled only where mutation is the construction
> idiom (CJS `module.exports`, `src/ast/js_cjs_export_barriers.rs`). A slice that wants to model runtime
> mutation proposes it as a new contract, not as a precision fix to an existing rung.

Also record it in `docs/superpowers/pipeline-lessons.md` as a doctrine (a one-paragraph lesson: "state the
analysis model before the reviewer does; a closure claim over an open vocabulary is a planning defect").

### 2.2 Branch S: sol-standard, strengthened D6(b)

**The closed fail-closed rule, stated as precisely as it can be.** A declarator is admitted only if all hold:

- **S-R5.** The callee is an identifier bound by a top-level ESM `import_statement` of exactly `"react"`,
  whose **imported** name is `forwardRef` or `memo` (aliases allowed). Member forms (`React.forwardRef`)
  refuse as `callee_not_admitted`.
- **F1.** No top-level ESM value import of `"react"` in the file binds the module object: no default clause,
  no `* as`, and no named specifier whose imported name is `default`. (`import type` is exempt.)
- **F2.** No `call_expression` anywhere in the file has a callee spelled `require` or `import` (any argument;
  no substring test, so sol W2 is closed by construction).
- **F3.** No identifier spelled `eval`, `module`, `arguments`; no `with_statement`. (`module` and `arguments`
  are the CJS-wrapper handles; Q9.)
- **F4.** R6 P1–P5 for the callee local, and R4 import parse recovery, as in r2.
- **F5.** No identifier or property spelled with an escape (`\`) anywhere in the file (r2 K3, widened).
- **F6 (the model line).** Host and ambient globals (`globalThis`, `window`, `self`, `global`, `Function`,
  `process`, `import.meta`) and cross-module mutation are **out of model**. They cannot be refused without
  refusing most React files (`window.addEventListener` is routine).

**Can it be closed?** Only relative to F6. F1–F5 close every acquisition through a *declared* vocabulary
(import syntax, `require`, dynamic import, the two CJS-wrapper handles, `eval`/`with`). The reviewer can always
name an ambient path (`new Function("return require")()`), and F6 is where the brief must say "out of model,
argue it against D2". So Branch S makes the same *kind* of statement Branch P makes, with a larger in-model set,
for 7 edges. If the owner does not want to make any model statement, the honest fallback is NameOnly for all
forms, which the owner has declined.

**Deferred under S (planned, not refused):** member forms and default-binding files. The follow-up is not "a
closed rule later", because §1.1 shows none exists under this standard; it is either the owner adopting the P
model for those forms or delivering them NameOnly (the hybrid). Say so in the SPEC rather than promising a
closure that cannot be built.

### 2.3 Hybrid

Branch S for Exact, plus: `SpannedLocal` gains `exact: bool`; `ResolvedJsExport.span` becomes
`Option<(usize, usize, bool)>` or a sibling field; `js_ts_import_member_candidates` returns the single
span-verified match, and the R4c arm emits `exact(...)` or `demoted(...)` by that flag. Member forms and
named forms in F1-refused files set `exact = false`. Everything is still span-verified, JSX-gated, and still
refuses the F4 hazards, so the NameOnly edges are the same 100 targets P would emit Exact, at score 0.6. Worth
doing only if the owner takes S and wants the 100 edges delivered rather than deferred. It is not better than P
on any axis except conformance to sol's standard, and it costs the most.

### 2.4 Recommendation: Branch P, plain (no guard)

From the long-term-health seat:

1. **Consistency of the confidence contract is worth more than 88 edges either way.** Prism's Exact is one
   thing on every rung and in every language (Q1–Q6). Holding one JS export shape to a different, stricter
   contract makes the label mean less, not more, because consumers (nav `score`, `asserted`, `--min-confidence`)
   cannot see which contract an edge was graded under. If the owner ever wants a runtime-mutation-aware grade,
   it should be a new label or reason across all rungs, designed once.
2. **The 107/4 yield is audited correct and the refusal set is measured empty.** Branch S keeps 7 and refuses
   the `import React, { forwardRef }` idiom, which is the shape most React code in the wild uses. That is not a
   precision gain on any measured corpus; it is a recall loss bought to defend against inputs with zero measured
   prevalence.
3. **Convergence.** Branch P has one new thing for a reviewer to judge, the model, and the owner sets it. Every
   mechanism finding is already CLOSED in r2. Branch S has an open vocabulary (F6) and the same reviewer will,
   correctly, keep probing it unless the brief draws the line for them, which is the P move anyway.
4. **Sol's own r1 self-critique** pre-committed that a stated model excluding local mutation dissolves W1. Take
   sol at their word; state the model.

Why plain rather than P-guard: a partial refusal with no closure claim is what the r1→r2 dynamic fed on, and
base does not make the same check for any other rung. If the owner wants the guard for the optics of the r1
input, take it with the "best-effort, not custody" label and accept ~56 lines.

## 3. Fold plan for the remaining r2 findings

| r2 finding | Branch P | Branch S / hybrid |
|---|---|---|
| **W1** named-only import + `require("react").forwardRef = fake` → Exact | **Out of model** (§2.1 model; base Q4a/Q4b/controller probe). Pinned as MB2, asserting Exact, comment cites §3.2. SPEC §4 row reads "documented out of model", not "refused" | **Folded**: F2 refuses any `require` call. Control C36-named becomes a drop row |
| **W2** `require("re"+"act")` computed specifier → Exact | Out of model; covered by MB2's class, no extra test (no substring rule exists to regress) | Folded by F2 (any `require`, no argument test). Add the control as a drop row |
| **W3** `React.__defineGetter__("forwardRef", …)` → Exact | Out of model (Q4b shows base does the same for `import_member`). Pinned as MB3 asserting Exact | Moot for Exact (F1 refuses the default binding). Under the hybrid the input yields NameOnly, pinned as a NameOnly delivery row |
| **SMELL 1** T-R6-P2 does not exercise P2 | **Folded** as sol proposes: `import { memo } from "react"; import type { T as memo } from "./types";` → `callee_provenance`; assert the reason. Keep the old type-only row as a second R5 row | Same |
| **SMELL 2** `extract_import_bindings()` reuse admits destructured `require("react")` | **Folded**: keep the hand parser (ESM by construction), retract the §9 reuse saving, add T-N18 `const { forwardRef } = require("react")` → `callee_not_admitted`. If reuse is kept, the origin check in §2.1 is mandatory and T-N18 pins it | Same; under S, T-N18 also drops by F2 (`require` anywhere), so add a second row where the destructured require is the *only* CommonJS spelling to prove S-R5 refuses it independently |

Also carried from r2 without change: D2(i) JSX-only, W2 parse recovery, W3 comparator, W4 generator, W5 barrel
span key, S2 census metadata (all CLOSED in r2).

## 4. Budget and slicing

Honest lines: after `rustfmt`, non-blank, non-`//`, `#[cfg(test)]` and `tests/**` counted as tests
(`probes/honest_lines.py`). The r2 prototype is **MEASURED 431 src** (P17, sol-reproduced), of which
`js_wrapped_export.rs` is 303 by the §9 function table (predicate 103, import parsing 64, write position 41,
module-scope competitor 35, custody walk 34, occurrence accounting 26) and the other files 128. Everything
below is an **estimate** from that table; the implementer recounts and the cap governs.

| Bucket | P (plain) | P-guard | S | Hybrid |
|---|---|---|---|---|
| `js_wrapped_export.rs` | 303 − 34 − 26 − 41 − 5 (R7 call, mode switch) ≈ **197** | 197 + 41 + ~15 (targeted member walk) ≈ 253 | 103 − 22 (member branch) − 6 + 64 − 15 + 8 (F1) + 35 + ~30 (F2/F3/F5 fence) ≈ **197** | S + 22 (member branch back) ≈ 219 |
| other files | 128 | 128 | 128 | 128 + ~25 (`exact` flag through `js_exports.rs`, `resolution.rs`, `ast.rs`) ≈ 153 |
| **src estimate** | **~325** | ~381 | **~325** | ~372 |
| **src cap / early stop** | **350 / 325** | 400 / 380 | 350 / 325 | 400 / 370 |
| test rows (≈9 lines each) | 11 P + 4 J + 17 N (incl. T-N18) + 5 R6 + 3 MB + 3 O ≈ 43 → ~390 | + 6 guard rows → ~445 | 8 P + 4 J + 18 N + 5 R6 + 8 fence + 3 O ≈ 46 → ~415 | S + ~8 NameOnly delivery rows → ~490 |
| helpers + T-S1 + cache/nav | ~180 | ~180 | ~180 | ~180 |
| **tests estimate** | **~570** | ~625 | ~595 | ~670 |
| **tests cap / early stop** | **600 / 560** | 650 / 610 | 620 / 580 | 700 / 660 |
| **combined cap** | **950** | 1,050 | 970 | 1,100 |
| Tier-A fixture dirs | 3 (`forwardref_named_export`, `forwardref_ident_arg_nested_samename_refused`, `react_default_member_write_out_of_model`) | 3 (third = `…_refused`) | 3 (third = `react_default_binding_refused`) | 3 |

Two honest notes. Branch S is **not** cheaper in src than P: the fence replaces custody line for line. And the
optional `extract_import_bindings()` reuse would take ~40 lines off any branch, but it is what caused SMELL 2;
the caps above assume the hand parser.

**Slicing.** One slice under P or S (single mechanism, as r2 argued; S1-i/S1-ii split still not recommended
because S1-i cannot be yield-tested). The hybrid should be two slices: S first, then the NameOnly delivery as
S1-n, because the second changes the R4c emission path and deserves its own RED and its own review.

**Acceptance yields** (SPEC §8.2) per branch: P: X exactly the 107 rows of `P6-excalidraw-expected-rowdiff.json`,
F 4, R/T byte-identical. S: X the 7 rows whose producer is `TextField.tsx` or `ButtonIcon.tsx` (re-derive the
expected row-diff from `X-b-rowdiff.json` filtered by producer file, and re-measure; do not trust the source
read), F 0. Hybrid: S's 7 Exact plus the remaining 100 X rows as NameOnly, F 4 NameOnly.

## 5. Round-3 authority and the reviewer brief addendum

The 2-round spec cap is spent. A third round needs the owner's explicit approval, and it should be **one round,
on the chosen branch only**, with the analysis model fixed by the owner before dispatch, so the reviewer judges
the packet against the owner's standard rather than their own. Under Branch P, an APPROVE requires:

1. SPEC §3.2 replaced by the §2.1 model text; the words "closed" and "custody" gone from the Exact claim.
2. R7/K1–K8 removed from §3.1, §4, §5, §7, the mutant table (M10 dropped) and the budget.
3. W1–W3 dispositioned as "documented out of model" with MB1–MB3 and the Tier-A fixture flipped to `exact = true`.
4. SMELL 1 and SMELL 2 folded (§3), T-N18 present.
5. The replan probes cited as the precedent evidence, with paths.
6. Budget re-capped to 350 / 600 / 950 (plain P).
7. The `CLAUDE.md` statement drafted, marked pending owner wording.

A mutation-class input in round 3 is then argued against D2 (an owner decision), and is a WRONG only if it
demonstrates a **static** misbinding: the name denotes a different callable than the span-verified one.

**Addendum to `REVIEWER.md` for round 3 (Branch P wording; swap §3.2 text for S):**

> ## Round 3 (owner-approved, final): the analysis model is fixed
>
> The owner has set the confidence contract for this slice. Judge the packet **against this model**, not
> against a stronger one:
>
> > Exact is a static-binding grade. S1 proves what a name denotes: the callee is a unique, unwritten,
> > value-typed ESM import binding of `"react"`; the export target is the inner function's exact span; only
> > JSX element sites bind. Runtime mutation of the React module object, by any means, from this file or any
> > other, is out of model, as it is for every `import_member` edge prism emits on `main`
> > (`~/prism-evidence/wrapped-export/replan/RESULTS.md`, Q1–Q6, Q9).
>
> **What is a WRONG under this model.** A concrete input where the span-verified target is a callable that the
> name does not statically denote (a nested or shadowed same-name function, a comparator arrow, a Pattern-3
> over-named argument, an impostor or non-`"react"` `forwardRef`, a parse-recovered import, a barrel conflict,
> an incremental-rebuild epoch that disagrees with the full build), or an Exact edge from a non-JSX site, or a
> counted-twice or uncounted refusal, or a cap breach. Give the input, the incorrect target, the mechanism and
> a file:line.
>
> **What is not a WRONG.** Any input whose only defect is that the React object is mutated or re-acquired at
> runtime (`React.forwardRef = …`, `require("react")…`, `Object.defineProperty`, `__defineGetter__`, `eval`,
> `arguments[1]`, host globals). These are pinned as model-boundary tests MB1–MB3 asserting Exact. If you think
> the model itself is wrong, say so under **Convergence view** as an argument against D2, with the cost you
> would pay in edges, and do not count it as a finding.
>
> **Per finding**, keep the shape: tag (WRONG / SMELL); a recommended fix that is bounded and specific; at
> least one alternative with its cost, risk and maintenance tradeoff, including "accept and document as out of
> scope" where it is reasonable; and a self-critique that names the assumptions the finding rests on and says
> when it would not apply. Report WRONG items first; a finding without a concrete failure scenario is a SMELL.
>
> **Prior-round closure.** For r2 W1–W3 and SMELL 1–2, state CLOSED, NOT CLOSED, or OUT OF MODEL (with the
> pinned test that documents it).
>
> Review as a senior or principal engineer whose goal is the project's long-term health: rigorous, and never
> inventing issues. A clean result is a valid result.

## 6. Not settled here

- **The fork itself.** Which contract Exact means for this slice is the owner's call. §2.4 is a recommendation.
- **Branch S yield is a source read (7), not a run.** A Branch-S implementation must re-measure; `FilledButton`
  and `LibraryMenuSection` are refused by F1 on inspection, but F2–F5 were not run on the two surviving files
  beyond a grep for `require`, `import(`, `eval` and `with` (none found).
- **`CLAUDE.md` wording** is drafted, not applied; the owner owns that text.
- **P-guard vs plain P.** Recommended plain; the guard is an owner option with a labelling condition.
- **Line counts** are estimates from the P17 function table, not a recount of an implementation.
- **Whether sol will accept a stated model** is not knowable in advance; the brief is written so that the
  reviewer's disagreement with the model lands in "Convergence view" rather than as findings, which is where an
  owner-decided contract belongs.

## 7. What changed in the packet

- This file. `SPEC.md` and `README.md` gain a one-line pointer marking D2/D6/§3.2 as pending owner choice;
  their r2 text is left intact so the reviewed artifact is preserved. Once the owner picks, the controller
  applies §2.1 or §2.2 to `SPEC.md`, `IMPLEMENTOR.md` and `REVIEWER.md` in one commit.

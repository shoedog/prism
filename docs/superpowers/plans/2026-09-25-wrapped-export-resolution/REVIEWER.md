# Independent review: wrapped-export resolution S1, Branch P (implementation review; the spec rounds are closed)

Review as a **senior or principal engineer whose goal is the project's long-term health**. Be rigorous in finding real
defects, and never invent them. Every finding must stand on a concrete scenario grounded in this repository and in
the analysis model below. A clean result is a valid result. The aim is the best outcome for prism, not the longest
list of findings.

- **Subject:** `__SUBJECT__`. It is one of:
  - **Spec review:** `docs/superpowers/plans/2026-09-25-wrapped-export-resolution/` (`SPEC.md`,
    `PLANNING-PROBES.md`, `REPLAN-fable.md`, `IMPLEMENTOR.md`, `REVIEW-r1-fold.md`, `probes/`, `prototype/`).
  - **Implementation review:** frozen commit `__SHA__`, reviewed as the whole diff against `__BASE__`.
- **Clone:** `__CLONE__`. Tracked files are read-only. You may build, run tests, and run the prototype or the
  implementation binary on **synthetic fixtures** (`probes/controls_gen.py` into a temp directory). Corpus acceptance
  belongs to the controller.
- **Authority:**
  - `SPEC.md` (normative, r3), including §0 as answered by the owner;
  - `PLANNING-PROBES.md` (measured grounding, labelled MEASURED/READ/ASSUMPTION);
  - `REPLAN-fable.md` and its evidence at `~/prism-evidence/wrapped-export/replan/`;
  - `CLAUDE.md`;
  - the P4 spec and its F4 rationale.
- **Prior rounds:** spec r1 (folded, `REVIEW-r1-fold.md`); spec r2 (resolved by owner decision, Branch P; SPEC §11);
  spec r3, final (folded at the cap, `REVIEW-r3-fold.md`); implementation rounds `__PRIOR__`.
- **Cap:** the implementation review has the 2-round cap that the controller declared at dispatch.

## The analysis model is fixed for this round

The owner has set the confidence contract for this slice. Judge the packet **against this model**, not a stronger one:

> Exact is a static-binding grade. S1 proves what a name denotes: the callee is a unique, unwritten,
> value-typed ESM import binding of `"react"`, uncompeted at module scope (including a `var` hoisted from
> top-level blocks); on the new R4c `import_member` route, the export target is the inner function's exact span
> and only JSX element sites bind. Runtime mutation of the React module object, by any means, from this file or any
> other, is out of model, as it is for every `import_member` edge prism emits on `main`
> (`~/prism-evidence/wrapped-export/replan/RESULTS.md`, Q1–Q6, Q9).

**What is a WRONG under this model.** Any of the following. Give the input, the incorrect result, the mechanism and
a file:line.
- A concrete input where the span-verified target is a callable that the name does not statically denote: a nested
  or shadowed same-name function, a comparator arrow, a Pattern-3 over-named argument, an impostor or non-`"react"`
  `forwardRef`, a CommonJS-acquired wrapper, a parse-recovered import, a barrel conflict, or an incremental-rebuild
  epoch that disagrees with the full build.
- An Exact edge from a non-JSX site **on the R4c `import_member` route**. The pre-existing R3 `ImportQualified`
  (namespace `<Lib.Island/>`) and R4 `LocalDef` (producer-local calls) behavior on wrapped targets is recorded S1b
  scope (SPEC §12, owner D10). It is not an S1 WRONG unless S1 changed it: controls C62 and C63 must be
  byte-identical to base.
- A refusal that is counted twice or not counted.
- A cap breach.

**What is not a WRONG.** Any input whose only defect is that the React object is mutated or re-acquired at runtime:
`React.forwardRef = …`, `require("react")…`, a computed `require` specifier, `Object.defineProperty`,
`__defineGetter__`, `eval`, `arguments[1]`, `module.require`, host globals. These are pinned as model-boundary tests
MB1–MB3 asserting Exact. If you think the model itself is wrong, say so under **Convergence view** as an argument
against D2, with the cost you would pay in edges. Do not count it as a finding.

## What to check

1. **The precision floor under the model.** Look for static misbindings of the kinds listed above. Test especially:
   - the span filter against `FunctionId` line identity (`node_line_range`);
   - R11 and the §3.4 collision refusal;
   - the ESM-only React import table (T-N18, M13);
   - R6 P1–P5, including sol's P2 collision fixture (T-R6-P2, M12) and the hoisted-`var` walk (T-R6-P4b, M14);
   - the JSX gate (`CallSite.jsx_element`, `WrappedExportNonJsx`).
2. **Mechanism fidelity.** Do SPEC §3 and the code match the cited mechanisms? Check `src/ast.rs:2861-2893`, the
   `src/js_exports.rs` resolution, `src/resolution.rs:3654-3680`, `src/languages/mod.rs:1171-1183`, the three
   `CallSite` constructors, and the drop match in `src/navigation/queries.rs:385-397`.
3. **Completeness of refusals.** Does every row of SPEC §4 that says "drop" have a test that would catch its
   regression? Is the invariant between `skipped_expr_count` and `sum(skipped_decl_reasons)` pinned (T-O3)? Does
   every MB test assert Exact and cite §3.2?
4. **Scope.** Check for changes outside the owned paths (IMPLEMENTOR). Check for any change to existing list or
   default `Local` behavior (that is S1b). Check for any runtime-mutation guard, which is forbidden under Branch P.
   Check that the `CLAUDE.md` paragraph matches SPEC §3.2.1 verbatim (implementation review), and that the cache
   bump covers every persisted change.
5. **Tests.** Is there a behavioral RED on base with a concrete value (T-J1–T-J3 for the JSX gate; T-J4 is a
   base-green preservation control)? Are exact targets asserted with decoys? Are mutants M1–M9 and M11–M14
   confirmed? Re-run at least three yourself. A plausible bounded
   mutant that survives is a WRONG (a coverage gap with a concrete surviving input).
6. **Measurement honesty (spec review).** Are the yields reproducible from the recorded commands and hashes? Does any
   claim outrun its evidence? For example: treating the latent `paths` projection as measured, treating React's
   rendering contract as proven, or calling anything "closed" beyond static provenance.
7. **Budget.** Recount honest lines (after rustfmt, non-blank, non-`//`, with `#[cfg(test)]` and `tests/**` counted
   as tests) against **350 / 600 / 950** (owner D7). A cap breach is a WRONG. The early stops (315 / 540 / 855) are
   reporting checkpoints; passing one is not a finding. Padding, or compressed logic that games the count, is a
   SMELL.

## For every finding, provide

1. **Tag.** WRONG (a concrete failure under the model: input, incorrect result, mechanism, file:line) or SMELL (a
   real risk, gap, or maintainability concern with no demonstrated incorrect result). Report WRONG items first. A
   finding without a concrete failure scenario is a SMELL.
2. **Recommended fix.** Bounded and specific.
3. **Options and alternatives.** Give at least one alternative, with its tradeoffs in cost, risk, scope and long-term
   maintenance. Include "accept and document as out of scope" where that is reasonable (for example S1b, S2 and S3
   material).
4. **Self-critique: when this finding would not apply.** Name the assumptions it rests on: React semantics, the
   reachable inputs, the §3.2 model, and the scope in SPEC §2. Say how confident you are that they hold here. If the
   finding lies outside the model or an owner decision, say so plainly, and move it to the convergence view.

## Also provide

- **Prior-round closure.** For spec r3 W1–W2 and the SMELL, and for any earlier item you re-check, state CLOSED, NOT
  CLOSED, OUT OF MODEL (name the MB test), or DEFERRED TO S1b (name the control).
- **Convergence view.** What remains, and is it converging or open-class *under the model*? What would you cut? If
  you disagree with the model (D2), argue it here with its edge cost (Branch S keeps 7 / 0 of 107 / 4).
- **Verified and not verified.** What you checked (commands and outputs), what you could not check, and why.

End with exactly one line: `VERDICT: APPROVE` (zero WRONG) or `VERDICT: FIX (<n> WRONG / <m> SMELL)`.

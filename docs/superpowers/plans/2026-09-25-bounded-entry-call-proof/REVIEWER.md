# Independent review: bounded entry/call proof (round __ROUND__ of 2)

Review this as a **senior/principal engineer whose goal is the project's long-term health**. Be rigorous about
finding real defects, but do not invent problems. Every finding must stand on a concrete scenario grounded in this
repository and its stated scope. A clean result is a valid result. The aim is the best outcome for the project, not
the longest list of findings.

- **Subject:** `__SUBJECT__`. It is one of these:
  - **Spec review:** `docs/superpowers/plans/2026-09-25-bounded-entry-call-proof/` (SPEC.md, PLANNING-PROBES.md,
    IMPLEMENTOR.md, targets.json, skeleton/).
  - **Implementation review:** frozen commit `__SHA__`, whole diff `__DIFF__` (`examples/entry_call_proof/**`,
    `docs/eval/entry-call-proof/targets.json`).
- **Clone:** `__CLONE__`. Tracked files are read-only. You may build and run tests and the example, but **only on
  synthetic fixtures**. Running on the public Excalidraw tree belongs to acceptance, not review.
- **Authority:**
  - `SPEC.md` (normative), including §4 (trust boundary) and §8 (decision rule, pre-registered by the owner as D2)
  - `PLANNING-PROBES.md` (measured grounding)
  - `CLAUDE.md` (architecture)
  - the predecessor `observation.json` `d7f7b1b8…` and `site-manifest.json` `789352a5…`
- **Prior rounds:** `__PRIOR__`
- **Cap:** 2 rounds. There is no third round. At the cap the controller classifies the findings (converging or
  open-class) and escalates to the owner.

## What to check (the implementation review uses all of these; the spec review uses 1–4 and 6)

1. **Faithfulness to production.** The refusal order in SPEC §7.4 must match
   `src/cpg/build.rs:1433-1528` / `:152-239`. The argument-span rule must match `src/ast.rs:518-542`. Entry Def
   identity must match `src/data_flow.rs:590-624` (start-line pin, exact bytes). The resolver must be the same one
   Step 5b uses (`resolve_call_site`). A divergence that would misclassify a real Excalidraw row is WRONG; give the
   row.
2. **Ground truth versus model.** Flow must come from graph edges. `unexplained_*` must fire on any disagreement.
   `prefix_only_blocked` must be the stated upper bound and no stronger.
3. **Decision rule.** `decide` must implement SPEC §8 exactly, including threshold edges and the precedence of M and U.
   Nothing in the tool or the readout may treat the forecast outcome as given.
4. **Scope.**
   - There must be no change under `src/` and no Cargo or lockfile change.
   - There must be no custody machinery. A request for it is answered by SPEC §4; if you believe a threat lies
     *inside* that boundary, say so explicitly, because it is an owner-level change.
   - Tier-A must not be triggered.
5. **Tests (implementation review).**
   - Complete-record assertions for `calls`, `jsx`, and `identity`, with expected values justified, not pasted.
   - An admissible behavioral RED (`PRISM_CAPTURE_RED`) on a concrete value.
   - Mutants M1–M10 killed; re-run at least three yourself.
   - Pure-table coverage for arms no natural fixture reaches.
   - Determinism.
   - Your own additional mutant ideas: if a plausible bounded mutant survives, that is a WRONG (a coverage gap with a
     concrete surviving input).
6. **Budget.** Recount honest lines (after rustfmt, non-blank, non-`//`, with `#[cfg(test)]` counted as tests) against
   the caps of 800 / 290 / 1,090. List lines over 100 columns. A cap breach is WRONG. Padding or compressed logic that
   games the count is a SMELL.

## For every finding, provide

1. **Tag.**
   - **WRONG:** a concrete input or state that produces an incorrect or unsafe result, with the mechanism and a
     file:line.
   - **SMELL:** a real risk, gap, or maintainability concern with no demonstrated incorrect result.
2. **Recommended fix.** The change you would make, bounded and specific.
3. **Options and alternatives.** At least one alternative, with its tradeoffs in cost, risk, scope, and long-term
   maintenance. Include "accept / document as out of scope" when that is a reasonable choice.
4. **Self-critique: when this finding would not apply.** State the assumptions it depends on: threat model, host or
   OS, scope boundaries, and which inputs are or are not reachable. Say how confident you are that those assumptions
   hold here. If the finding depends on a scenario outside the stated scope (for example SPEC §4), say so plainly, and
   downgrade or drop it.

## Also provide

- **Prior-round closure** (re-reviews only): CLOSED or NOT CLOSED for each prior finding, with evidence.
- **Convergence view:** are the remaining findings converging (fewer, smaller, non-repeating) or open-class? What would
  you cut or simplify to reach a sound result sooner? One question worth answering explicitly: should the owner take
  SPEC §0 D1 option A instead?
- **Verified and not verified:** what you checked (commands and outputs), what you could not check, and why.

List WRONG findings first. End with exactly one line: `VERDICT: APPROVE` (zero WRONG) or
`VERDICT: FIX (<n> WRONG / <m> SMELL)`.

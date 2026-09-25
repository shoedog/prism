# Independent review: wrapped-export resolution S1 (round __ROUND__ of 2)

Review this as a **senior or principal engineer whose goal is the project's long-term health**. Be rigorous in
finding real defects, and never invent them. Every finding must stand on a concrete scenario grounded in this
repository and the stated scope. A clean result is a valid result. The aim is the best outcome for prism, not the
longest list of findings.

- **Subject:** `__SUBJECT__`. It is one of:
  - **Spec review:** `docs/superpowers/plans/2026-09-25-wrapped-export-resolution/` (`SPEC.md`,
    `PLANNING-PROBES.md`, `IMPLEMENTOR.md`, `probes/`, `prototype/`).
  - **Implementation review:** frozen commit `__SHA__`, reviewed as the whole diff against `__BASE__`.
- **Clone:** `__CLONE__`. Tracked files are read-only. You may build and run tests, and you may run the prototype or
  the implementation binary on **synthetic fixtures** (`probes/controls_gen.py` into a temp directory). Corpus
  acceptance belongs to the controller.
- **Authority:**
  - `SPEC.md` (normative), including §0 as answered by the owner
  - `PLANNING-PROBES.md` (measured grounding; each claim is labelled MEASURED/READ/ASSUMPTION)
  - `CLAUDE.md`
  - the P4 spec, `docs/superpowers/specs/2026-07-03-prism-p4-js-export-modeling-spec.md`, and its F4 rationale
- **Prior rounds:** `__PRIOR__`
- **Cap:** 2 rounds. There is no third round. At the cap, the controller classifies the remaining findings as
  converging or open-class and escalates to the owner.

## What to check

1. **The precision floor (highest priority).** Can any input make S1 produce an Exact `import_member` edge to a
   function that rendering or calling the export does not invoke? Start from the F4 hazard (M2): a nested or shadowed
   same-name function, a same-line nested function, a comparator arrow, a Pattern-3 over-named argument (M4), an
   impostor or aliased `forwardRef`, a rebound `React`, re-export chains and barrels, conflicting claims, and
   incremental rebuilds (T-S1). A WRONG must name the input source and the incorrect target.
2. **Mechanism fidelity.** Do SPEC §3 and the code match the cited mechanisms: `src/ast.rs:2861-2893`,
   `src/js_exports.rs` resolution, `src/resolution.rs:3654-3680`, and `src/languages/mod.rs:1171-1183`? Check
   especially the span's line basis against `FunctionId` (`node_line_range`), and the default/namespace eligibility
   point (M10).
3. **Fail-closed completeness.** Does each row of SPEC §4 have a test that would catch its regression? Are there
   refusal gaps in the §3.1 reason set, meaning a skip that is not counted or is counted twice? Is the invariant
   between `skipped_expr_count` and the reason counts pinned?
4. **Scope.** Is anything changed outside the owned paths (IMPLEMENTOR)? Has any existing list or default `Local`
   behavior changed (that is S1b's)? Has `function_name`, ownership or the DFG been touched? Does the cache bump
   cover every persisted change?
5. **Tests (implementation review).** Is there a behavioral RED on base, with a concrete value? Are exact targets
   asserted with decoys? Are the mutant kills M1–M7 confirmed? Re-run at least three yourself. A plausible bounded
   mutant that survives is a WRONG (a coverage gap with a concrete surviving input).
6. **Measurement honesty (spec review).** Are the yields reproducible from the recorded commands and hashes? Does any
   claim outrun its evidence, for example treating the latent `paths` projection as measured, or treating the React
   semantics as proven?
7. **Budget.** Recount honest lines (after rustfmt, non-blank, non-`//`, with `#[cfg(test)]` and `tests/**` counted
   as tests) against **200 / 450 / 650**. A cap breach is a WRONG. Padding, or compressed logic that games the
   count, is a SMELL.

## For every finding, provide

1. **Tag.**
   - **WRONG:** a concrete input or state that produces an incorrect or unsafe result. Give the input, the incorrect
     result, the mechanism, and a file:line.
   - **SMELL:** a real risk, gap, or maintainability concern with no demonstrated incorrect result.
2. **Recommended fix.** Bounded and specific.
3. **Options and alternatives.** Give at least one alternative, with its tradeoffs in cost, risk, scope and long-term
   maintenance. Include "accept and document as out of scope" where that is reasonable (for example S1b, S2 and S3
   material).
4. **Self-critique: when this finding would not apply.** State the assumptions it depends on: React semantics, the
   reachable inputs, and the scope boundaries in SPEC §2. Say how confident you are that they hold here. If the
   finding lies outside the stated scope or an owner decision, say so plainly, and downgrade or drop it.

## Also provide

- **Prior-round closure** (re-reviews only): CLOSED or NOT CLOSED for each prior finding, with evidence.
- **Convergence view:** are the remaining findings converging (fewer, smaller, non-repeating) or open-class? What
  would you cut to reach a sound result sooner? Answer explicitly: is D2 = Exact justified, or should the owner take
  NameOnly?
- **Verified and not verified:** what you checked (commands and outputs), what you could not check, and why.

List WRONG findings first. End with exactly one line: `VERDICT: APPROVE` (zero WRONG) or
`VERDICT: FIX (<n> WRONG / <m> SMELL)`.

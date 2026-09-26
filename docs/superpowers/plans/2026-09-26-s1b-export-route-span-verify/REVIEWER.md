# Review brief: S1b, span-verified JS/TS export and local routes (one brief for sol and Opus)

Review as a **senior or principal engineer whose goal is prism's long-term health**. Be rigorous in finding real
defects and never invent them. Every finding must stand on a concrete scenario grounded in this repository and in
the analysis model below. A clean result is a valid result. The aim is the best outcome for prism, not the longest
list of findings.

- **Subject:** `__SUBJECT__`. One of:
  - **Spec review:** `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/` (`SPEC.md`,
    `PLANNING-PROBES.md`, `IMPLEMENTOR.md`, `probes/`, `prototype/`).
  - **Implementation review of sub-slice `__SLICE__` (S1b-a or S1b-b):** frozen commit `__SHA__`, reviewed as the
    whole diff against `__BASE__`.
- **Clone:** `__CLONE__`. Tracked files are read-only. You may build, run tests, and run the base, prototype or
  implementation binaries on **synthetic fixtures** (`probes/controls_gen.py` into a temp directory, then
  `probes/controls_summarize.py` and `probes/controls_diff.py`). Corpus acceptance belongs to the controller.
- **Never open** the private corpus (F) or `~/prism-evidence/wrapped-export/planning/CORPORA-PRIVATE.txt`. Corpus F
  appears in the packet as aggregates only.
- **Authority, in order:** `SPEC.md` (normative), including §0 as answered by the owner; `PLANNING-PROBES.md`
  (labelled MEASURED / READ / ASSUMPTION); `CLAUDE.md`; the S1 packet
  (`docs/superpowers/plans/2026-09-25-wrapped-export-resolution/`), whose §12 recorded this scope.
- **Prior rounds:** `__PRIOR__`.
- **Cap:** 2 rounds per sub-slice, declared by the controller at dispatch. At the cap, the controller classifies the
  remaining findings as converging or open-class (CLAUDE.md convergence discipline).

## The analysis model is fixed

> **Exact is a static-binding grade** (`CLAUDE.md`, "Exact is a static-binding grade"). A JS/TS name denotes a
> callable when the nearest scope that declares it holds exactly one declaration, that declaration's value is a
> function (or an admitted React wrapper of one), the scope parses cleanly, and no write in that scope targets the
> binding (SPEC §3.1). An export denotes what its module-scope binding denotes (§3.2); a namespace member denotes the
> resolved module's export (§3.4); a lowercase, dashed or namespaced JSX tag denotes no binding at all (§3.5).
> **Runtime mutation of module objects, `eval`, host globals and `require`/`import()` re-acquisition are out of
> model for every rung.** A `with` statement is static syntax and is in model: it makes every name ambiguous.

**What is a WRONG under this model.** Give the input, the incorrect result, the mechanism and a file:line.
- An **Exact edge to a callable the name does not statically denote**, on any route S1b owns (the D4 list,
  default-identifier, function-declaration and declarator export routes; R3 namespace qualifiers; R4 `LocalDef` in
  S1b-b; any rung for a JSX intrinsic tag). Typical sources: a binding form the declaration collector misses
  (a shadowing parameter, catch parameter, `for` head, class name, `enum`, `namespace`, import, parameter-default
  scope, Annex B block function, `with`), a parse-recovery shape that hides a declaration, a same-line collision, a
  write the write scan misses, a Pattern-2/3/5 over-named candidate, or an incremental rebuild that disagrees with a
  full build.
- A **removed or changed edge that was right**, where the SPEC does not record it as an owner-accepted cost. The
  accepted costs are listed in SPEC §0 and §8 (for example the parse-recovery refusals and the may-call class).
- A refusal counted twice or not counted; a counter that disagrees with the row-diff.
- A cap breach (SPEC §9).

**What is not a WRONG.** Runtime mutation (`Lib.f = g`, `Object.defineProperty`, `eval`, CJS wrapper handles,
host globals); these are pinned as model-boundary tests asserting Exact. A route S1b explicitly leaves unchanged
(SPEC §2 non-goals, each pinned by a test): named- and default-import qualifiers (S2), CommonJS `Local`, synthetic
and `IndirectResolution` sites. If you think the model or an owner decision is wrong, say so under **Convergence
view** with the edge cost, not as a finding.

## What to check

1. **Closure of the declaration collector (SPEC §3.1).** Is every ECMAScript and TypeScript binding form covered,
   either as a declaration or by the parse-recovery rule? Try each reviewer probe class in **both** the JSX (`.jsx`)
   and TSX (`.tsx`) grammars: parse recovery; quote and string tokens (compare raw tokens, no quote trimming);
   lowercase intrinsic tags, opening versus self-closing elements, and type-only imports; hoisted `var` in nested
   blocks and same-line collisions; barrel and star re-export conflicts; incremental versus full rebuild.
2. **Mechanism fidelity.** Do SPEC §3 and the code match the cited mechanisms (PLANNING-PROBES M1–M14)?
3. **Refusal completeness.** Every SPEC §4 row that says "drop" has a test that fails if the refusal regresses.
4. **Scope.** No change outside the owned paths (IMPLEMENTOR). No change to the non-goal routes. The cache bumps
   cover every persisted change.
5. **Tests.** Behavioral RED on base with a concrete value for every new path; exact `(file, name, start, end)`
   targets with decoys; the mutants in SPEC §7 confirmed. Re-run at least three yourself. A plausible bounded
   mutant that survives is a WRONG (a coverage gap with a concrete surviving input).
6. **Measurement honesty (spec review).** Are the row-diffs reproducible from the recorded commands and hashes? Does
   any claim outrun its evidence (for example, calling the auditor independent where it shares the planner's model;
   PLANNING-PROBES says where it does)?
7. **Budget.** Recount honest lines (after `rustfmt`, non-blank, non-`//`; `#[cfg(test)]` and `tests/**` are tests)
   against the sub-slice caps in SPEC §9. A cap breach is a WRONG. Early stops are checkpoints. Padding, or logic
   compressed to game the count, is a SMELL.

## For every finding, provide

1. **Tag.** WRONG (a concrete failure under the model: input, incorrect result, mechanism, file:line) or SMELL (a
   real risk, gap or maintainability concern with no demonstrated incorrect result). Report WRONG items first. A
   finding without a concrete failure scenario is a SMELL.
2. **Recommended fix.** Bounded and specific.
3. **Options and alternatives.** At least one alternative with its tradeoffs in cost, risk, scope and long-term
   maintenance. Include "accept and document" where reasonable.
4. **When this finding would not apply.** Name the assumptions it rests on (the model, the reachable inputs, the
   scope in SPEC §2, an owner decision in §0) and how confident you are that they hold. If the finding lies outside
   the model or an owner decision, say so plainly and move it to the convergence view.

## Also provide

- **Prior-round closure.** For each earlier finding you re-check: CLOSED, NOT CLOSED, OUT OF MODEL (name the test),
  or DEFERRED (name the owner decision).
- **Convergence view.** What remains, and is it converging or open-class under the model? What would you cut?
- **Verified and not verified.** What you ran (commands and outputs), what you could not check, and why.

End with exactly one line: `VERDICT: APPROVE` (zero WRONG) or `VERDICT: FIX (<n> WRONG / <m> SMELL)`.

## Controller notes

<!-- The controller fills this in at dispatch: subject, SHA/base, clone, cap, prior rounds, and any owner rulings
     made since the packet was written. -->
__CONTROLLER_NOTES__

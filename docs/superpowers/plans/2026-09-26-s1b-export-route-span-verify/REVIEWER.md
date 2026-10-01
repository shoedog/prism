# Review brief: S1b, span-verified JS/TS export and local routes (one brief for sol and Opus)

Review as a **senior or principal engineer whose goal is prism's long-term health**. Be rigorous in finding real
defects and never invent them. Every finding must stand on a concrete scenario grounded in this repository and in
the analysis model below. A clean result is a valid result. The aim is the best outcome for prism, not the longest
list of findings.

- **Subject:** `__SUBJECT__`. One of:
  - **Spec review:** `docs/superpowers/plans/2026-09-26-s1b-export-route-span-verify/` (`SPEC.md`,
    `PLANNING-PROBES.md`, `IMPLEMENTOR.md`, `probes/`, `prototype/`).
  - **Implementation review of sub-slice `__SLICE__` (S1b-1, S1b-2, S1b-3 or S1b-4):** frozen commit `__SHA__`, reviewed as the
    whole diff against `__BASE__`.
- **Clone:** `__CLONE__`. Tracked files are read-only. You may build, run tests, and run the base, prototype or
  implementation binaries on **synthetic fixtures** (`probes/controls_gen.py` into a temp directory, then
  `probes/controls_summarize.py` and `probes/controls_diff.py`). Corpus acceptance belongs to the controller.
- **Never open** the private corpus (F) or `~/prism-evidence/wrapped-export/planning/CORPORA-PRIVATE.txt`. Corpus F
  appears in the packet as aggregates only.
- **Authority, in order:** `SPEC.md` (normative), including §0 as answered by the owner; `PLANNING-PROBES.md`
  (labelled MEASURED / READ / ASSUMPTION); `REVIEW-r1-fold.md`; `CLAUDE.md`; the S1 packet
  (`docs/superpowers/plans/2026-09-25-wrapped-export-resolution/`), whose §12 recorded this scope.
- **Prior rounds:** `__PRIOR__`.
- **Cap:** 2 rounds per sub-slice, declared by the controller at dispatch. At the cap, the controller classifies the
  remaining findings as converging or open-class (CLAUDE.md convergence discipline).

## The analysis model is fixed

> **Exact is a static-binding grade** (`CLAUDE.md`, "Exact is a static-binding grade"). A JS/TS name denotes a
> callable when the nearest scope that declares it (SPEC §3.1's enumerated table) holds exactly one declaration whose
> value is a function (or an admitted React wrapper of one), the scope passes the fail-safe and the parse-recovery
> rule, and the binding is not in the **may-call class** (a written binding, or a call-wrapped declarator with a
> direct function argument), which **keeps base behavior by owner decision E5**. An export denotes what its
> module-scope binding denotes (§3.2); a namespace member denotes the resolved module's export (§3.4); a lowercase,
> dashed or namespaced JSX tag denotes no binding (§3.5). **Runtime mutation of module objects, `eval`, host globals
> and `require`/`import()` re-acquisition are out of model for every rung.** A `with` statement is in model: it makes
> every name ambiguous.

**What is a WRONG under this model.** Give the input, the incorrect result, the mechanism and a file:line.
- An **Exact edge to a callable the name does not statically denote**, on any route S1b owns (the D4 list,
  default-identifier, function-declaration and declarator export routes; R3 namespace qualifiers; R4 `LocalDef`;
  any rung for a JSX intrinsic tag). Typical sources: a binding or write form the §3.1 table misses or misclassifies,
  a grammar kind the closure probe misclassifies as inert, a parse-recovery shape that escapes B1, a same-line
  collision, an over-named candidate, or an incremental rebuild that disagrees with a full build.
- A **removed or changed edge that was right**, where the SPEC does not record it as an owner-accepted cost (the
  accepted costs are in SPEC §0 and §8, for example E6's parse-recovery refusals).
- **Any changed row in the may-call class** (E5 keeps it at base), or a may-call rule that is not a closed syntactic
  predicate.
- A refusal counted twice or not counted; a counter that disagrees with the row-diff.
- A cap breach (SPEC §9).

**What is not a WRONG.** Runtime mutation (`Lib.f = g`, `Object.defineProperty`, `eval`, CJS wrapper handles,
host globals); these are pinned as model-boundary tests asserting Exact. A route S1b explicitly leaves unchanged
(SPEC §2 non-goals, each pinned by a test): named- and default-import qualifiers (S2), CommonJS `Local`, synthetic
and `IndirectResolution` sites. If you think the model or an owner decision is wrong, say so under **Convergence
view** with the edge cost, not as a finding.

## What to check

1. **Closure of the binding core (SPEC §3.1).** Does the enumerated table cover every ECMAScript and TypeScript
   binding and write form it claims to, keyed to the cited static semantics? Does `probes/grammar_closure.py` derive
   the suspect kinds soundly, and is every one classified correctly (not just listed)? Is B1's closure argument
   sound, and is its stated ASSUMPTION the only one? Try each reviewer probe class in **both** the JSX (`.jsx`)
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

## Round 3 addendum (pending owner approval; the cap is spent)

The round-3 brief is `REPLAN-fable.md` §7, verbatim, appended here at dispatch once the owner approves the round
and answers OQ1–OQ7. Until then the r2 brief above is the record. In one line: the evaluation-context model
(SPEC §3.1a) is fixed by the owner; a WRONG needs a *listed* position mis-cited, a creator missing from Σ′, a D-row
miss, a leave-predicate hole, a B1 escape, a right edge removed outside the accepted costs, a may-call change, a
counter defect or a cap breach; a base row left at base because its position is unlisted is a SMELL with the row to
add, not a WRONG.

## S1b-3 addendum (r4, against the landed code)

Subject: the S1b-3 sub-slice (or 3a / 3b, OQ10) implementing SPEC §3.8 on the landed S1b-2a/2b collector. Beyond
the checklist above:
- **The write resolver is the review surface.** 2a's predicates (a) and (b) are deleted; a WRONG is a write that
  reaches a binding but the resolver misses (a `MayCall` lost: name the input and the edge that changes), or a write
  that does not reach the binding but is counted (a `Callable` lost). The mangled-write rule (identifiers that are a
  child or sibling of an `ERROR`) is deliberately over-inclusive toward `MayCall`; an over-count is a SMELL unless it
  removes a right edge the SPEC keeps.
- **The owner's M2 ruling is fixed:** every written binding keeps base, whatever its kind. A refusal of a written
  class, `for (var …)` head or parameter is a WRONG.
- **What drops (SPEC §3.8 (11), owner 2026-09-29 rounds 1 and 2).** A drop (R4 or R5) is right for a binding that
  is not statically bound (a parameter or catch parameter: the owner-accepted value-flow cost, §0 (3), is not a
  finding unless the measured count in §8 is wrong) or that provably holds no function (`for` head, class, enum,
  namespace, marker, duplicate, recovered import, or a declarator with no value or a NoFn value **and no default in
  its pattern**). A drop of a declarator whose value is outside NoFn, or whose pattern holds a default, is a WRONG
  (name the row). A drop at R5 for `unbound`, `import`, `with`, `parse_recovery`, B0 or `MayCall` is a WRONG. A
  lexical auditor cannot certify an alias removal: `audit_s1b3.py`'s `removed_alias` (now including default-bearing
  patterns) must be 0, or every such row hand-audited.
- **The NoFn class is closed by the owner's conservative cut:** a finding that asks for more precision (for example
  property-sensitive destructuring) is out of scope; a row the cut makes base is not a WRONG.
- **P1:** the shared `collect_js_ts_binding_pattern_names` trusts parser-confirmed identifiers (non-ASCII, ZWNJ/ZWJ,
  `$`); a changed row elsewhere that this causes is reported with the preservation test that pins it.
- **Recovered imports:** only a top-level `ERROR` whose first two non-comment tokens are `import` and something other
  than `.` or `(` marks names; poisoning a name that no such broken import spells is a WRONG; poisoning every word of one
  (including a path segment) is the accepted over-approximation; a Unicode alias split into fragments is a WRONG.
- **Budget:** owner-approved caps (3a src 700 / tests 740; 3b src 220 / tests 1,320, report point 1,190). Tests are expected near the
  row estimate (≈ 5 lines per unit row, ≈ 30 per end-to-end scenario); a large overrun is a SMELL naming the batches.
- **Evidence:** the controller's audited row-diff must equal SPEC §8's r4 row (3a: 0 on every corpus);
  `maycall_changed` 0.

## S1b-4 addendum — 2026-10-01

READ: review cap is **2 rounds**, and numeric LOC caps are abolished by the
owner. Historical cap-breach WRONG/checkpoint rules in this brief do not apply.
Review the dated SPEC §3.4 amendment and S1b-4 dispatch against landed 915fca43;
verify the independent value-flow inventory/hand audit, not a lexical
not-callable annotation. C220 is a constructible right alias-export edge on
the base. OQ-S1b4-1 is resolved by the controller: namespace-only Alias opacity
keeps R3 base while D4 remains unchanged. The prototype, r2 expected rows,
411-control r4 reference (397 old sections byte-identical to r3) and 53-row hand audit are present; the expanded
mutant receipts and explicit equivalent variants are in S1b-4-MEASUREMENTS.md.
Verify the source/binary hashes in BUILD-MANIFEST and the controller's eventual
commit binding before dispatch. Private F and bounded quick exclusions are
explicit in the measurements; neither is implicit approval.
Both Opus spec rounds are consumed; this final fold precedes implementation.
The implementation review surface is the finite qualifier outcome/export/E7
matrix and its bounded private-barrel proof. Opaque binding-cell file/name must never stand in for callable origin;
renamed and cyclic alias guards have same-environment base and Node evidence.

## Controller notes

<!-- The controller fills this in at dispatch: subject, SHA/base, clone, cap, prior rounds, and any owner rulings
     made since the packet was written. -->
__CONTROLLER_NOTES__

READ current S1b-4 review subject: plan922f00df plus the final r2 fold; source
base915fca43, prototype beec4a23 plus final owned fold. Both plan rounds are
consumed; the controller classified convergence and authorized this final fold.
W5/W6/S6/S7 dispositions are in SPEC and r4 measurements. E5 has no exception: may-call
and written export terminals preserve full base rows on every route, whatever kind.
X1 must be killed; X2 is redundant and X6 is the disclosed S7 survivor.
Implementation starts from the controller's single cumulative squash commit on
proto/s1b-4-final; its SHA is filled in before dispatch. No owner question remains.
Private F is controller-only; no historical F aggregate certifies r4.

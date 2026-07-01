# S2 Spec — Round-3 Dual Review (architecture / tradeoff lens)

**Date:** 2026-06-13 · **Spec under review:** rev 3 (commit `2b8e86b`) →
**outcome: rev 4.** Prior rounds: `s2-spec-review-2026-06-13.md` (r1, byte-as-key trap),
`s2-spec-review-r2-2026-06-13.md` (r2, `(file,start_line)` regression).

**Charter (owner, verbatim):** "thorough evaluating tradeoffs preferring that anything
deferred isn't more expensive later which would lead to a costly refactor, that seams are
strong, and detailed architecture is solid, well composed, meets goals, has well defined
failure modes, on items deferred ensures we leave openings for enhancement, risks are
thoroughly evaluated, and other architectural concerns are well vetted and reasoned."

**Reviewers:**
- **codex (gpt-5.5, xhigh)** via a2a-bridge (`a2a-bridge.s2-arch-eval-codex.toml`,
  prompt `prompts/s2-arch-eval.md`). Read-only; verified the spec against live code shape
  (`func_index`, `var_index`, `FunctionId`, cache v4, `SymbolRef`/`shape.rs`,
  type-provider `FunctionId` synthesis) before judging. Raw: `/tmp/s2-arch-eval-codex.md`.
- **claude (opus, max effort)** run as an **operator subagent** (Agent tool, model:opus)
  — the bridge claude model-override defect (forces sonnet) can't guarantee opus, so the
  subagent path is the verifiable-model workaround (same as r2).

## Verdicts

| Reviewer | Verdict | One-line |
|---|---|---|
| codex | **tighten** | Core sound; the deferral/additivity claims around occurrence-splitting, `FunctionId.start_byte`, call-site spans, and **witness-wire identity** need correction before implementation. |
| opus | **sound to plan (5 tightenings)** | Lock-in mostly unfounded (witness reserves `ordinal`, carries name+line); two regret items (vacuous nav ordinal; buried same-line admission semantics) + 3 lower-priority tightenings. |

**Both lenses agree on everything structural:** byte-additive identity (never key/Ord),
`(file,name,start_line)` func key fixing `build.rs:211`, the three `trace.rs` NodeIndex
order points (:242/:342/:379) as the byte-sort retirement targets for Plan B, and the
named failure modes. No reviewer challenged the architecture.

## The crux (apparent disagreement → reconciliation)

The two verdicts diverge only on **deferral lock-in severity**, and the divergence is
framing, not fact:

- **codex:** three deferrals overclaim "additive." Occurrence-splitting changes **DFG
  set/map semantics** (`VarLocation` *is* the DFG key, not just `var_index`);
  `FunctionId.start_byte` is a broad call-graph/cache/provider/nav migration; `CallSite`
  is already line-only-keyed (`call_graph.rs:1176`) so same-line duplicate calls collapse.
  Fix: put bytes on the witness wire (`SymbolRef`/`GraphNode`/`CallSite`) **now**, or
  explicitly fence Plan B's guarantee to line-only.
- **opus:** lock-in mostly unfounded — the witness reserves `ordinal` and carries
  name+line, so identity is extensible without a breaking change, and the
  `FunctionId.start_byte` migration costs the same before/after Plan B. But (1) the nav
  `ordinal` as specified is **always 0** after the `var_index` dedup (its disambiguation
  domain collapses) → vacuous; (2) S2 quietly changes same-line admission from Plan B's
  conservative *any-after-any* to *first-occurrence byte compare* — buried, must surface
  for the Plan B re-plan + fixture re-validation.

**Reconciliation (owner-facing):** they don't contradict on facts — only on whether to
treat the line-only wire as a blocker. **Plan B isn't built, so we control the wire.** The
decisive code fact (verified): `var_index` is keyed `(file, function, line, path, access)`
— **the CPG already has one node per occurrence at line granularity**, and `node_of`
(`shape.rs:206`) reads each node's own byte via `to_var_location`. So a byte **range** on
the wire is *meaningful now* (the exact span for the ~99% one-occurrence-per-line case),
**not** vacuous. The vacuous-now thing is only the **ordinal** (its domain
`(file,line,path,access)` collapses under the dedup → always 0 until occurrence-splitting).

So the synthesis satisfies both: **ship the byte range on the wire now** (codex's "make it
byte-ready"; honest because occurrences already exist) and **reserve the ordinal**
(opus's vacuousness finding; the genuinely-deferred discriminator). The earlier
"adding bytes now is vacuous" objection conflated *byte range* with *ordinal* — corrected.

## Owner decision (witness wire)

Owner leaned "put byte range / byte-derived ordinal on `SymbolRef`/`GraphNode`/`Location`
now so the wire is byte-ready," and asked whether the risk is just cost. Answer landed:
for the byte **range** it is essentially cost (low; no correctness risk — byte & line
share one `to_var_location` source); for the **ordinal** it is *not* just cost (semantic
overclaim — always 0 until occurrence-splitting). **`CallSite`** is the one place "now"
fixes a current bug (same-line duplicate-call collapse), not merely pre-wires.

**Decision folded into rev 4:** byte **range** added now to `Location`, `SymbolRef`,
`GraphNode`, and `CallSite`; `ordinal` stays `0` (reserved, documented); occurrence-
splitting + `FunctionId.start_byte` deferred with named additive seams.

## Findings → disposition in rev 4

| # | Finding | Reviewer | Disposition |
|---|---|---|---|
| 1 | Witness identity line-only despite byte-bearing nodes; add bytes to wire or fence Plan B | codex #1 | **Folded** — §1 wire-decision row; §5 `node_of` emits byte range on `Location`/`SymbolRef`; §0 additive-extensibility contract. |
| 2 | nav `ordinal` always 0 after var_index dedup (vacuous) | opus #1 | **Folded** — `ordinal` reserved-0 (not populated from byte rank); §5/§9; §7 asserts `==0`. |
| 3 | Same-line admission semantics change (any-after-any → byte compare) is buried | opus #2 | **Folded** — §5 Plan B bullet surfaces it for the re-plan + round-6 fixture re-validation. |
| 4 | Occurrence-splitting not cleanly additive (DFG set/map = `VarLocation`-keyed) | codex #2 | **Folded** — §4 names the `Ord≡Eq≡Hash` hand-written agreement (byte excluded) + §7.6 invariant test; §9 seam states splitting = relax dedup + populate ordinal (additive). |
| 5 | `FunctionId.start_byte` deferral is a broad migration, not localized | codex #3 | **Folded** — §9 seam documents the call-graph/provider/nav/cache breadth + that it's no more expensive after Plan B (wire carries node byte, not a `FunctionId`). |
| 6 | `CallSite` line-only → same-line duplicate calls collapse | codex #4 | **Folded as in-scope** — §4 adds `CallSite` byte + `cmp_key` extension; §7.8 de-collapse test; removed from §9 deferral (only the call-site *ordinal* remains deferred). |
| 7 | Replace tuple span plumbing with a named span-bearing record + deterministic tie-breakers | codex #5 | **Folded** — §4 helpers return `VarOccurrence`/`StatementSpan` (not widened tuples); §3 total tie-break (`end_byte`, `access`, `NodeIndex`). |
| 8 | Add explicit function-candidate API; don't hide overload ambiguity | codex #6 | **Folded** — §5 `function_candidates(file,name) -> Vec<NodeIndex>`; `function_node` documented unique/first. |
| 9 | `.function` audit must include `cfg_queries`, taint — not just trace | codex risk | **Folded** — §5 names `node_file_fn` (trace.rs:173, primary), `cfg_queries.rs:237`, `taint.rs:4875`. |
| 10 | `FunctionId` `Ord/Eq` includes `end_line` while S2 identity excludes it | codex risk | **Folded** — §8 risk: deliberate; Step-1 join matches `(name,start_line)`, ignores `end_line` (node stable under body edits). |
| 11 | Equal `VarLocation` key, differing byte → silent map-insertion winner | codex risk | **Folded** — §8 risk: byte non-identity; deterministic insertion + total tie-break; no consumer keys on the winner. |
| 12 | Failure modes: macro-generated def/call; multi-target call resolution; augmented-assign LHS = Def+Use | codex #4 (failure gaps) | **Folded** — §6 adds the three rows (augmented-assign now explicit Def+Use). |
| 13 | Cache: deterministic byte-sorted `location_index` rebuild under ties | codex #7 | **Folded** — §5 cache bullet + §3 tie-break applied in `reconstruct_cpg`/`from_parts`. |
| 14 | `node_file_fn` (trace.rs:173) named the primary `.function` edit | opus | **Folded** — §5. |
| 15 | Enumerate merged `shape.rs` same-line tests in the expected-flip list | opus | **Folded** — §7.7 (wire byte adds fields to their serialized output). |
| 16 | `VarLocation` `Ord≡Eq` invariant test; document `trace.rs:211` | opus | **Folded** — §7.6 invariant; §5 documents `:211` stays NodeIndex-sorted (not a same-line concern). |

**Nothing rejected; nothing requires redesign.** All 16 are completeness/scoping
tightenings — consistent with both verdicts (codex "tighten" = correct the deferral text;
opus "sound to plan" = proceed with the 5 named items folded). rev 4 is the result; next
gate is the owner spec-review, then writing-plans.

# S1b planning packet: span-verify every JS/TS export and local route (r4, 2026-09-27)

**Base:** `origin/main` `a6d853f5` (S1 merged). Planning only: nothing under `src/`, `tests/`, `eval/`, `Cargo.*` or
`CLAUDE.md` changes on this branch. **Status:** spec round 2 (the cap) found the collector open-class; S1b-1 ships;
S1b-2..4 are **re-planned in `REPLAN-fable.md`** (evaluation-context table, positional fail-safe, leave predicate);
the owner took Option K and five sub-slices (OQ1–OQ7, SPEC §0). Spec round 3 (the last) was **folded at the cap**
(`REVIEW-r3-fold.md`; owner: "targeted fold, then implement"). S1b-1, S1b-1b and S1b-2a are on main (`761541c2`);
S1b-2b is approved (`3961cc21`). **r4 (2026-09-27): S1b-3 re-planned against the landed code** (SPEC §3.8, prototype
v10, PLANNING-PROBES Q49–Q61). **Open for the owner:** OQ10 (split S1b-3 into 3a/3b), OQ11 (M1 for destructuring
declarators), OQ12 (R5 reach of carry-forward 4). Re-plan evidence:
`~/prism-evidence/s1b/replan/`; r3 evidence: `~/prism-evidence/s1b/r3fold/` and `~/prism-evidence/s1b/planning/`.

| File | Purpose |
|---|---|
| `SPEC.md` | normative design r2: §0 owner answers and open questions; §3.1 the binding core as an enumerated table keyed to ECMA-262 and TS declaration spaces, with the fail-safe and the narrower parse rule; per-route semantics; controls; counters; cache; per-sub-slice tests, mutants and Tier-A fixtures; acceptance as exact audited row-diffs per sub-slice; budget |
| `REVIEW-r3-fold.md` | the at-cap round-3 fold: finding → disposition → location → RED rows and mutants → measured |
| `PLANNING-PROBES.md` | mechanisms M1–M17 and probes Q0–Q61 (r3: Q36–Q48; r4: Q49–Q61) (command, pre-run expectation, result, output path); r2 results per corpus and route |
| `REVIEW-r1-fold.md` | every spec round 1 finding → disposition → location → evidence, plus fold findings and disagreements |
| `REPLAN-fable.md` | re-plan after the round-2 cap: diagnosis (the unit of proof is the position, not the kind), options K / A / B / D with measured yields and budgets, the evaluation-context table, the fold plan for the bounded r2 items, owner questions OQ1–OQ7, the round-3 review brief |
| `IMPLEMENTOR.md` | dispatch brief for one sub-slice, with per-batch budget checkpoints, and the **S1b-2a dispatch** section |
| `REVIEWER.md` | one brief for sol and Opus |
| `probes/` | tools (auditor `jsscope.py`, `grammar_closure.py` with the kind and `E(kind, field)` tables and the runtime equality check, census, row-diff audit, taxonomy, 240-scenario control generator with JSX/TSX twins, `replan/` RP generator and replay), pre-run expectations (addenda 1–7), reference control summaries per sub-slice, `expected/` row-diffs for X, R and T |
| `prototype/s1b3-prototype-v10.diff.txt` | the S1b-3 prototype on the landed code (r4; scratch, never built into this branch) |
| `prototype/s1b-prototype-v9.diff.txt` | the feasibility prototype, r3 (never built into this branch) |

Evidence: `~/prism-evidence/s1b/planning/` with a `MANIFEST.sha256`.

## r4 measured facts (prototype v10 on `3961cc21`)

- **Row-diffs, audited:** X 236 (134 re-targeted, 102 wrong removed), F 476 (1 re-targeted, 471 wrong removed, 4 E6
  right lost), R 0, T 1,125 (635 re-targeted, 439 wrong removed, 10 right added, 41 E6 right lost). r3's R4 counts
  hold exactly; the rest is carry-forward 4's R5 class (OQ12). `maycall_changed` 0.
- **3a alone** (the collector, no call-site wiring): 0 rows on all four corpora.
- **Suites:** v10e 4,619 / 6 / 1 (6 by design; head 4,625 / 0 / 1); Tier-A 165 / 165 with the S1b-3 fixture (RED on
  head); closure probe exit 0.
- **Budget:** 664 src (3a 490, 3b 168); proposed caps SPEC §9 r4 (3a 590 / 620 tests, 3b 220 / 800).

## r3 measured facts (prototype v9)

- **The r3 fold's corpus cost: 0 rows** against v8 on X, F, R and T (Q41, Q42), including sol W1's Annex-B
  predicate, which restores C111's right edge. Counters identical to v8; `local_binding_unchecked_position` `{}` on
  all four (Q43).
- **F4 (OQ8):** X 91, F 16, R 0, T 5 rows, 112 / 112 audited right (Q45).
- **Suites:** prototype 4,574 / 9 / 1 (the same 9 pins); Tier-A 166 / 166 (Q47, Q48).
- **Budget (Q46):** 1,448 src; 2a 557 (cap 510), 2b 123 (140), 3 533 (480), 4 177 (185). SPEC §9, OQ9.

## Key measured facts (prototype v8)

- **Row-diffs (base → v8):** X 1,019, F 536, R 117, T 870, every changed row audited.
- **Right edges removed:** only by the parse-recovery rule (E6): F 4, T 41, all in files with tree-sitter grammar
  gaps. The r1 packet's "0 right edges removed outside parse recovery" was false (Opus W9, 21 X rows); v8 fixes it.
- **Right edges added:** T 10 (an import shadowed by an enclosing function declaration).
- **May-call rows (E5):** X 40, F 56, T 163 kept at base; 0 changed.
- **Wrong edges removed:** JSX intrinsic tags X 62 rows / F 103; R4 X 153 rows (134 re-targeted to the one right
  callable, 19 removed), F 31, T 817 (635 re-targeted, removing 1,799 extra targets); shared-collector fixes F 7, T 2.
- **Fail-safe cost:** 0 rows on all four corpora. **Scoped vs fixed base module scan:** 0 rows.
- **E6 narrower rule:** keeps F 7 and T 12 right rows the broad rule refused, risks 0. E6b would recover T's 41 more.
- **E7:** T's 24 namespace rows stay Exact; F's 4 bare-specifier rows become NameOnly (edges kept).
- **Suites:** base 4,583 / 0 / 1; prototype 4,574 / 9 / 1 (9 by-design pins). Tier-A base 162 / 162 (+4 new fixtures
  RED), prototype 166 / 166. Performance on T neutral (r1, Q19).
- **Budget:** about 1,050 src lines of prototype design code; SPEC §9 proposes four sub-slices with caps
  60 / 620 / 330 / 185 src (E13, open).

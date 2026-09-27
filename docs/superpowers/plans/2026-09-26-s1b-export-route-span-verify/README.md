# S1b planning packet: span-verify every JS/TS export and local route (r2, 2026-09-26)

**Base:** `origin/main` `a6d853f5` (S1 merged). Planning only: nothing under `src/`, `tests/`, `eval/`, `Cargo.*` or
`CLAUDE.md` changes on this branch. **Status:** spec round 1 folded (`REVIEW-r1-fold.md`); owner answers E1–E12
recorded in SPEC §0; three questions open (E5b, E6b, E13); spec round 2 next.

| File | Purpose |
|---|---|
| `SPEC.md` | normative design r2: §0 owner answers and open questions; §3.1 the binding core as an enumerated table keyed to ECMA-262 and TS declaration spaces, with the fail-safe and the narrower parse rule; per-route semantics; controls; counters; cache; per-sub-slice tests, mutants and Tier-A fixtures; acceptance as exact audited row-diffs per sub-slice; budget |
| `PLANNING-PROBES.md` | mechanisms M1–M17 and probes Q0–Q35 (command, pre-run expectation, result, output path); r2 results per corpus and route |
| `REVIEW-r1-fold.md` | every spec round 1 finding → disposition → location → evidence, plus fold findings and disagreements |
| `IMPLEMENTOR.md` | dispatch brief for one sub-slice, with per-batch budget checkpoints |
| `REVIEWER.md` | one brief for sol and Opus |
| `probes/` | tools (auditor `jsscope.py`, `grammar_closure.py`, census, row-diff audit, taxonomy, 218-scenario control generator with JSX/TSX twins), pre-run expectations (addenda 1–6), reference control summaries per sub-slice, `expected/` row-diffs for X, R and T |
| `prototype/s1b-prototype-v8.diff.txt` | the feasibility prototype (never built into this branch) |

Evidence: `~/prism-evidence/s1b/planning/` with a `MANIFEST.sha256`.

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

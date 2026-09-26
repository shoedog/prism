# S1b planning packet: span-verify every JS/TS export and local route (2026-09-26, r1)

**Base:** `origin/main` `a6d853f5` (S1 merged). Planning only: nothing under `src/`, `tests/`, `eval/`, `Cargo.*` or
`CLAUDE.md` changes on this branch. **Status:** awaiting the owner's §0 answers and the spec review.

| File | Purpose |
|---|---|
| `SPEC.md` | the normative design: §0 owner decisions E1–E12 with options and recommendations; closed binding rules (§3.1); per-route semantics; controls; counters; cache; per-sub-slice tests, mutants and Tier-A fixtures; acceptance as exact audited row-diffs; budget |
| `PLANNING-PROBES.md` | mechanisms M1–M14 (READ, with file:line) and probes Q0–Q23 (command, pre-run expectation, result, output path); per-corpus, per-route results |
| `IMPLEMENTOR.md` | dispatch brief for one sub-slice, with mandatory per-batch budget checkpoints |
| `REVIEWER.md` | the single brief for sol and Opus (model, WRONG vs SMELL, fix + options + "when this would not apply", controller-notes slot) |
| `probes/` | the S1 tools (extended) plus the independent auditor (`jsscope.py`), census, row-diff audit, edge taxonomy, control generator (153 scenarios, JSX and TSX twins), pre-run expectations, reference control summaries, and `expected/` row-diffs for the public corpora |
| `prototype/s1b-prototype-v7.diff.txt` | the feasibility prototype (never built into this branch) |

The evidence root is `~/prism-evidence/s1b/planning/` with a `MANIFEST.sha256`.

## Key measured facts

- **Where the wrong edges are.** The export routes (D4) and R3 namespace qualifiers are latent: 0 wrong
  `import_member` rows on four corpora, 0 namespace decoys. R4 `LocalDef` and JSX intrinsic tags are not. The
  prototype changes X 1,086 rows, F 682, R 118 and T 1,033, all audited:
  - JSX intrinsic tags bound on every rung (`<input>` → a test helper, `<label>` → action arrows): X 62 rows, F 103
    removed; plus relabels X 804, F 387, R 117.
  - R4 multi-target rows re-targeted to the one callable the name denotes: X 134, F 1, T 635 (T removes 1,799 extra
    target edges).
  - R4 rows removed: X 86, F 191, R 1, T 398. Static-binding verdicts: statically wrong, may-call (throttle,
    memoize, HOCs, rewritten bindings), pass-through (`useCallback`, owner decision E4), and right-but-refused by
    parse recovery (F 7, T 53, both caused by tree-sitter grammar gaps; E6).
- **0 right edges removed outside parse recovery**, and 0 additions on the corpora.
- **DFG:** X loses 27 argument → parameter edges, each tied to a removed `local_def` edge.
- **Controls:** 153 scenarios as pre-registered (4 recorded deviations). Tier-A 162 / 162 on base and prototype;
  the S1b fixtures are RED on base and green on the prototype.
- **Suite:** base 4,583 / 0 / 1. The prototype fails only by-design pins (fact representation, a drop reason, a
  closed audit gap).
- **Performance:** neutral on T (270.8 s base, 259.0 s prototype), with a per-file memo that is required.
- **Budget:** about 667 src lines of prototype design code against S1's 342, so the SPEC recommends three
  sub-slices (E1): S1b-1 core + D4 + intrinsic (cap 300 src), S1b-2 lexical `LocalDef` (340), S1b-3 R3 namespace
  (120).

## Owner decisions (SPEC §0)

E1 slicing; E2 intrinsic tags (recommend: include, on every rung); E3 a new drop reason; E4 admit `useCallback`;
E5 drop the may-call class; E6 whole-scope parse-recovery refusal; E7 keep an export-filtered stem lookup for
unresolvable namespaces; E8 leave non-namespace R3 to S2; E9 imported-local parity; E10 leave indirect sites;
E11 caps; E12 review process.

## Not measured

The may-call and `useCallback` semantics are library contracts (reported, not decided); sloppy-mode detection is
avoided by construction; `tier-a --quick`, `--features mcp`, clippy and the Node gate were not run on the prototype;
incremental rebuild parity is argued and pinned by tests, not measured on the corpora.

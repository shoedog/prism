# Fold-confirmation review: lane js-param-defs PR-B, the R9 fix delta (PR #351)

## Your role
You are a senior/principal engineer reviewing for the project's long-term outcome. Be rigorous about finding real defects, and never invent them. Read-only: no git writes or PR comments; delete `target/` and scratch afterward. Never open any directory named `frontend-portal`. This is static analysis; never execute corpus packages. Node may run small synthetic fixtures.

## The decision this review serves
Whether the plan PR #351 merges and source head `96817370` lands on main as PR-B. This is one narrow round on the fix delta only. The design, the owner decisions in SPEC-prB §8 and everything reviewed before `2431cb10` are settled; report against them only if the delta implements one wrongly.

Contract (unchanged): zero false Defs or edges; no LOST correct rows against main; Exact stays a static-binding grade (repo `CLAUDE.md`); call sites and non-JS output byte-identical; symbol, call and module navigation byte-identical. Owner decision E13 puts Flow / type-annotated JavaScript out of scope; the owner will re-affirm or revise it on the corrected numbers below, so its extent must be stated honestly.

## What changed
The final review round (`~/prism-evidence/js-param-defs/review/prB-final-{opus,sol}.md`; read your own, skim the other) returned FIX from both reviewers. R9 folded those findings:

| Finding | Claimed fix |
|---|---|
| Opus F2: early-error refusal fired on parse-recovered parameter lists and dropped whole named functions in type-annotated JS | no early error when the parameter list has parse errors |
| Opus F3: tables treated any Node `SyntaxError` as proof main's row was wrong | evidence rule: allowlist of genuine early errors; everything else INADMISSIBLE; type-annotated JS identified by the TypeScript parser, not the `@flow` pragma; an independent TS-AST census |
| Opus F1: JSX tag names as Uses | fixed |
| Opus F5 / sol W3: `eval` reached through TS wrappers (`as`, `!`, parentheses, satisfies) | fixed |
| sol W1: class receiver writes | fixed |
| sol W2: class self-name binding | fixed |
| Opus F6–F9 (F9 = type-predicate parameter names are erased, not runtime Uses) | fixed or recorded |

- **Source delta:** in this clone, `git diff 2431cb10 origin/wip/js-param-defs-prB-r3` (head `96817370`; 6 files, about 470 added lines, most of them tests). Main file: `src/ast_callback_identity.rs`.
- **Docs:** this branch (`review-r9` = origin `plan/js-param-defs-prB` at `5ddbf1d6`), packet `docs/superpowers/plans/2026-10-06-js-param-defs/`.
- **Worker report:** `~/prism-evidence/js-param-defs/prB/repair-r9/REPORT.md` and `HANDOFF.md`; frozen binaries in `repair-r9/bin/`; main-base binaries in `~/prism-evidence/js-param-defs/prB/bin/`.
- **Corpora:** X `~/prism-evidence/inputs/excalidraw-0642e72c/source`; T `~/code/bench-repos/TypeScript/src`; SecBench `~/prism-evidence/inputs/secbench-pkgs`. Oracle: `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.

## Reported results to check, not to trust (578 admitted roots)
- **Outside type-annotated JavaScript:** X ADDED 15,510 CORRECT, LOST 3,415 WRONG, 2 UNDECIDED; T ADDED 19,434 CORRECT, LOST 374,016 WRONG; SecBench ADDED 278,199 CORRECT and 0 WRONG, LOST 654,296 WRONG, 2 UNDECIDED, 0 CORRECT.
- **Inside type-annotated JavaScript (SecBench only):** ADDED 1,079 CORRECT, 84 WRONG; LOST 6 CORRECT, 426 WRONG, 26 UNDECIDED, 5 INADMISSIBLE. Before the F2 fix: 601 raw oracle-CORRECT losses; the fix restored 590 (87 Aurelia, 503 React Native).
- **Gates:** nextest 5,241 passed; mutants 103/103; clippy parity; Tier-A matrix 178/178.
- **Known open bookkeeping, already being closed by a continuation worker; do not report it as a finding:** 62 certificate rows against `f8c768b3` had no class because the R9 brief listed none for the F9 fix (34 removed type-predicate endpoints, 28 added runtime reads); a controller-side probe helper mis-read the escaped string `"use\x20strict"` as a directive; one root (`redos/cejs_2.0.20170212`) timed out under machine load and is being re-measured. Do report it if you find any of the 62 rows is *not* an F9 change.

## Priorities
1. **Each fix closes its finding and nothing adjacent regresses.** Build the head. For every row of the table, re-run the original reproducer, then vary it: the neighbouring syntax forms, both named and synthetic owners, JS, TS and TSX.
2. **The F2 guard does not over-reach.** "No early error when the parameter list has parse errors" must not readmit a callable that is genuinely an early error in plain JavaScript, and must not create false rows in a parse-recovered region outside type-annotated JS.
3. **New false Exact or LOST-correct rows outside type-annotated JavaScript** introduced by this delta specifically (class receiver writes, class self-name, type predicates, JSX tags, wrapped `eval`).
4. **The evidence rules are sound.** Is the early-error allowlist complete enough that a genuine early error is not marked INADMISSIBLE, and strict enough that a parser limitation is never counted as proof? Is "type-annotated JavaScript" decided independently of the product? Is the inside/outside split honest: does any inside-class row sit in a file with no type annotations, and on a sample are the 84 ADDED WRONG and 6 LOST CORRECT rows truly the misparse class E13 describes?
5. **Tests and mutants.** Does each fix have a test that fails on `2431cb10`, with a negative case? Are the ten new mutants meaningful?

## Output
Write the review to `REVIEW_OUT`. For each finding:
- **Tag:** WRONG (name the input or state and the incorrect result) or SMELL, plus MATERIAL or IMMATERIAL to the contract above.
- **Evidence:** file:line and a minimal fixture, labelled reproduced or static.
- **Fix:** a recommended fix plus alternatives.
- **Self-critique:** when the finding would not apply.
- **Classification:** NEW FAMILY or CLOSED INSTANCE of a named earlier family.

For each row of the fix table, state CLOSED, PARTIAL or NOT CLOSED with your evidence. End with a verdict — APPROVE, FIX (closed, enumerable fixes) or REJECT — and list what you did not check.

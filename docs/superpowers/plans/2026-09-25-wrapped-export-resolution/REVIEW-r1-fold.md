# Spec round 1 (sol, FIX 5 WRONG / 2 SMELL): fold record

> **Superseded in part (r3, Branch P).** The W1 row's D6 = (a) custody design was refuted by sol's round 2 and
> replaced by the owner's Branch-P model (SPEC §0 D2/D6, §3.2; `REPLAN-fable.md`). All other rows stand. The
> r2 → r3 disposition is recorded in SPEC §11.

The controller directed that D2 = Exact be kept, and that it meet sol's two stated conditions: (i) JSX-only binding
and (ii) closed provenance of the mutable React object. Every finding below is folded. Each fix was re-measured with
the r2 prototype on all four corpora and all 52 controls (PLANNING-PROBES P16–P24).

| # | Finding (sol) | Disposition | Where in the packet | Evidence |
|---|---|---|---|---|
| D2 (i) | A direct `Island(props)` on a `forwardRef`/`memo` object would get an Exact Call edge | **Folded.** There was no JSX-vs-call distinction (M13), so S1 adds `CallSite.jsx_element`. A `SpannedLocal` target binds only JSX element sites; other sites drop with the new `DropReason::WrappedExportNonJsx`, counted as `dropped_wrapped_export_non_jsx`. `new X()` is not a call site in JS/TS (M13) | SPEC §2, §3.4, §4, §5, T-J1–T-J4, M9 | C31: `WrappedExportNonJsx`; C32: no `new` row, JSX Exact; X/F `non_jsx` = 0 (P18) |
| W1 | `React.forwardRef = fake` gave a false Exact | **Folded as D6 = (a).** Closed occurrence custody over every default or namespace `"react"` binding (K1–K8, with write positions w1–w3). The analysis model is stated and cited to the CJS producer barrier (M14). Custody also covers named-form wrappers (M15, C37). Option (b) was measured and is not recommended | SPEC §0 D6, §3.2, R7, §4, T-R7-K1…K8, M10; fixture `react_default_member_write_refused/` | C28, C33–C37, C40–C43 and C45–C52 all refused; C38 benign uses Exact. (a) X 107 / F 4; (b) X 19 / F 0; 0 wrong on both (P18) |
| W2 | A parse-recovered import supplied provenance | **Folded.** R4 `import_parse_recovery`: any top-level `import_statement` with ERROR or MISSING refuses. Per-statement, not whole-file | SPEC §3.1 R4, T-N17 (with a twin where the error is outside the imports), M11 | C29 refused (P18c) |
| W3 | One-line `memo(render, cmp)` contradicted T-P4 | **Folded.** R11 `comparator_line_collision` is refused and counted. It is declared a non-goal (a formatting-dependent false negative). T-P4 is reworded to a comparator on a **different line** | SPEC §2, §3.1 R11, T-P4, T-N15 | C30 refused, reason counted. 0 such declarators on X and F (P18) |
| W4 | The generator could not produce C26/C27 | **Folded.** `controls_gen.py` now generates C01–C52. `SCENARIOS.json` is regenerated and copied as `probes/controls_SCENARIOS.json`. The expectations are extended (`P3-expectations-pre-run.md` C26/C27, `P18-expectations-pre-run.md`). A clean-directory reproduction gives 107 identical files and an identical r2 summary | `probes/controls_gen.py`, `probes/controls_SCENARIOS.json`, PLANNING-PROBES P22, IMPLEMENTOR smoke | P22 |
| W5 | No test pinned the span-sensitive star-barrel conflict | **Folded.** Integration test T-N16 (C39) and mutant M8. The mutant was run: it gives NameOnly to both spans instead of a drop, so the test kills it by asserting no `ImportMember` | SPEC §3.3, T-N16, M8 | C39: drop with `barrel_conflicts` 1 (P18c); M8 on C39: NameOnly ×2 (P21) |
| S1 | The provenance reason and test contract was not dispatch-exact | **Folded.** R1–R12 is a total order, with the first failing check naming the reason. The provenance subclauses P1–P5 each have a test (T-R6-P1…P5). The custody subclauses K1–K8 each have a row (T-R7-*). R12 `inner_unnamed` is marked unreachable. The sum invariant is pinned by a mixed-declarator fixture (T-O3) | SPEC §3.1, §5, T-O1, T-O3 | prototype reason counters (P18) |
| S2 | The TypeScript P2 artifact had stale `indexed` metadata | **Folded.** It is regenerated with the recorded inventory (173/176 indexed, matching sol). The r1 artifact is kept and labelled as empty-inventory | PLANNING-PROBES P24 | `census/P2-tssrc-census.json` |

## Consequential changes

- **Budget.** Re-capped from 200/450/650 to **420/650/1,070**. The measured r2 prototype is 431 src lines, with a
  demonstrable saving to about 380 (SPEC §9, P17). A split fallback is described there but not recommended.
- **Cache note.** v98 now also covers the `CallSite.jsx_element` field.
- **Tier-A.** The third fixture is now the member-write refusal (it replaces the optional `memo_named_export`).
- **Acceptance numbers.** Unchanged: 107 / 4 / 0 / 0. The r2 X and F row-diffs are byte-identical to r1's.

## Where the planner and sol differ

- **Convergence view: "the Exact case is open-class around mutable default-import objects."** The planner's position
  is that the **same-file** case is closed. JS property writes have a finite syntax (w1–w3). Every reflective or heap
  write needs the object as a bare value, and custody refuses every bare use (K5–K7). The residue is cross-module or
  host-global mutation. prism already excludes that for CJS module objects (M14), and SPEC §3.2 names it as the
  analysis model. If sol still sees an open class, the useful counterexample is a **same-file** input that passes
  K1–K8 and mutates `React.forwardRef` or `React.memo`.
- **"Longer term, JSX-to-render should be a distinct React/framework edge."** Agreed as direction, but out of scope.
  `jsx_element` is the seam for it (SPEC §10).

# Bounded entry/call proof: planning packet (2026-09-25)

**Question.** For the 12 native positional-gap sites, three things are unknown:

1. whether Prism's current CPG has an entry Def for the out-of-prefix parameter;
2. whether it has resolved callers;
3. whether it has argument→parameter flow at that ordinal.

The packet decides between two outcomes: a targeted production repair is warranted, or the work defers with a reason.

**State.** This is planning only: no code has been committed and nothing is authorized. Owner decisions D1–D3 in
SPEC §0 are open.

| File | Purpose |
|---|---|
| `SPEC.md` | Normative plan for option B, the byte-exact Rust observer. It covers seams (§3), trust boundary (§4), inputs (§5), schema and taxonomy (§7), the pre-registered decision rule (§8), RED, controls, mutants, and budget (§9), acceptance (§10), and review/STOP (§11). Appendix A is option A, the probe-receipt readout with 0 new lines. |
| `PLANNING-PROBES.md` | Read-only measurements with existing `prism nav` commands on the pinned input, recorded as hypothesis, expected, and observed. It also records synthetic separating fixtures and the skeleton's measured size. |
| `IMPLEMENTOR.md` | Implementer prompt: TDD order, RED capture, mutant table, budget stop rule, and handback. |
| `REVIEWER.md` | Reviewer prompt in the owner's v2 brief shape, for the spec review and the implementation review. Each has a 2-round cap. |
| `targets.json` | The 12 pinned targets (SHA-256 `b584bb4a…1ffd`), projected from `observation.json` `d7f7b1b8…` by the jq in SPEC §5. |
| `skeleton/main.rs.txt`, `skeleton/fixtures/` | The compiled planning skeleton (`bfff8c48…`, 618 helper / 158 test lines after rustfmt; unreviewed, not normative) and its synthetic fixtures. |
| `probe-extracts/` | Durable filtered extracts of the probe outputs; the full-output hashes are in PLANNING-PROBES. |

## Key measured facts

- **Entry Defs.** Present, with exact bytes, for 11 of 12 sites. `getStateForZoom.appState` has none, because a
  parameter used only through field access is skipped by design (`src/data_flow.rs:602`).
- **Resolved callers.** Only the two helpers have any: 5 and 3, all Exact `import_member`. The ten `forwardRef`
  components have none, for three distinct reasons (SPEC §2): wrapped named exports, whose imported JSX uses drop
  `UnknownName` because wrapped `const` exports are deliberately not recorded (`src/ast.rs:2874-2890`);
  `RowStack`/`ColStack`, reached only through the default-object member aliases `Row`/`Col` (`Stack.tsx:59`); and
  `SidebarInner`, whose JSX use inside a nested wrapper has no call-site record (the unowned lane).
- **Flow.** There are zero edges at every selected ordinal. The control shows the probe can see such edges: all three
  in-prefix edges into `elements`@0 exist.
- **Prediction.** At most one caller tuple is blocked only by the positional prefix. The predicted outcome is
  `defer / prefix_only_yield_below_threshold`.

## Status (2026-09-25): executed via option A

- Owner decisions: D1 = A (probe readout, 0 new lines); D2 = Y ≥ 3 ∧ S ≥ 2, locked.
- Spec review: r1 FIX (1 WRONG / 1 SMELL), r2 FIX (0 WRONG / 1 SMELL). Both were folded, converging within the
  2-round cap.
- Result: `defer / prefix_only_yield_below_threshold`, Y = 1, S = 1. Sol reconciliation: RECONCILED. See
  `docs/eval/entry-call-proof/readout.md` and `receipt.json`.
- Option B (`IMPLEMENTOR.md`, `skeleton/`) was not dispatched. It is kept as the plan of record if a byte-exact
  observer is ever wanted, for example on a second corpus.
- The larger gap surfaced is wrapped-export resolution (80 `UnknownName` drops across 7 components), not positional
  holes.

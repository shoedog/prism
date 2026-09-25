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
  components have none. Their JSX uses drop `UnknownName`, because wrapped `const` exports are deliberately not
  recorded (`src/ast.rs:2884-2890`).
- **Flow.** There are zero edges at every selected ordinal. The control shows the probe can see such edges: all three
  in-prefix edges into `elements`@0 exist.
- **Prediction.** At most one caller tuple is blocked only by the positional prefix. The predicted outcome is
  `defer / prefix_only_yield_below_threshold`.

## Next steps

1. The owner decides D1 (A or B), D2 (thresholds), and D3 (gate coverage).
2. Spec review with `REVIEWER.md` (2 rounds; sol gates).
3. Then either option A (controller procedure) or option B (dispatch with `IMPLEMENTOR.md`, then implementation
   review, then SPEC §10 acceptance).
4. The controller commits this packet, and refreshes the lane handoff with it, at the next stable point.

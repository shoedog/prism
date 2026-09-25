# Bounded entry/call proof — readout (2026-09-25)

**Scope.** This readout covers the 12 native positional-gap sites pinned in `targets.json` (SHA-256 `b584bb4a…1ffd`).
They were projected from the predecessor observation `d7f7b1b8…` (`docs/eval/native-positional-gap/`). For each
site it asks one question: does a resolved Exact caller pass a flow-carrying argument at the selected later ordinal
that is blocked **only** by the positional-slot prefix?

It is a measurement, not a repair. It was produced by SPEC Appendix A (owner D1 = A: probe readout, 0 new lines),
with the owner-locked D2 threshold **Y ≥ 3 ∧ S ≥ 2**. Y counts caller-occurrence rows; S counts distinct target
sites.

## Decision

**`defer / prefix_only_yield_below_threshold`**: **Y = 1, S = 1.**

The conservative bound comes first. The only row that can qualify is `ImageExportDialog.tsx:108 →
prepareElementsForExport`, so **Y ≤ 1 and S ≤ 1**, which already falsifies the threshold. The exact row:
`ImageExportDialog.tsx:108` passes `exportSelectionOnly` at ordinal 2. Its Def (destructuring, line 80) reaches this
Use through a raw DFG edge. The entry Def exists at exact bytes 1338–1357. It has no inbound argument edge.

No positional-hole repair is recommended on this evidence.

## Per-target result

| Target / binding@ordinal | Entry Def (E3) | Resolved callers (E1) → argument at ordinal (E5) | Inbound (E4) | Counts toward Y |
|---|---|---|---|---|
| Picker, FilledButton, Island, PropertiesPopover, QuickSearch, SidebarInner, RowStack, ColStack, TextField, ToolButton / `ref`@1 | present (10/10) | none | 0 | 0 |
| prepareElementsForExport / `exportSelectionOnly`@2 | present | actionClipboard:136 `true`; actionClipboard:209 `true`; **ImageExportDialog:108 `exportSelectionOnly`** | 0 | **1** |
| getStateForZoom / `appState`@1 | **absent**: field-only parameter, no Def by design (`src/data_flow.rs:602`) | actionCanvas:144/185/226 `appState`; App:4327 `this.state`; App:6930 `state` | 0 | 0 |

All 8 resolved callers are Exact `import_member`, score 1.0.

**Control.** The ordinal-0 in-prefix control has exactly 3 Exact edges into `data/index.ts:47 elements`, at
actionClipboard 137/210 and ImageExportDialog 109. The probe does see argument edges wherever the prefix admits them.

## Why the ten components contribute nothing (E2 ledger; P2)

The refusals come through three distinct lanes, and none of them is the positional prefix:

| Lane | Targets (call occurrences) | Mechanism |
|---|---|---|
| Wrapped named export → `drop:UnknownName` | ToolButton 40, FilledButton 13, Island 11, TextField 7, PropertiesPopover 6, QuickSearch 2, Picker 1 | `export const X = forwardRef(arrow)` records no export (`src/ast.rs:2874-2890`, deliberate) |
| Default-object member alias → `drop:UnknownName:alias` | ColStack 6 (`Col`), RowStack 1 (`Row`) | `export default { Row: RowStack, Col: ColStack }` (`Stack.tsx:59`) |
| Unowned use (no call-site row) | SidebarInner | JSX use inside a nested wrapper callback |
| `resolved_elsewhere` | Picker 1 (`IconPicker.tsx:319`) | Exact `local_def` to a different local `Picker` (correct) |

Even if these callers resolved, `ref` is a JSX prop that React routes. It is never a positional call argument, and
JSX has no argument list (`src/languages/mod.rs:835-850`). **So the positional prefix is not what limits this
population.** The larger, separable gap is **wrapped-export resolution**: 80 dropped `UnknownName` occurrences across
7 components. Any follow-up should be planned on that seam, not on positional holes.

**Sensitivity.** The only unresolved helper call is `data/resave.ts:38 → prepareElementsForExport`. It drops
`UnknownName` because of `import … from "."`, and it passes the literal `false` at ordinal 2. Even counted, it adds 0
to Y.

## Custody (full detail in `receipt.json`)

- **Production.** Commit `7e115934`. Its diff against `origin/main` `7ecccd99` (PR #321) is empty for `src`,
  `Cargo.toml`, `Cargo.lock`, and `build.rs`. The release `prism` binary SHA-256 is `263f71ad…f3b1`.
- **Input.** Excalidraw `0642e72c`, recovered durable tree: `INPUT MANIFEST OK: 1229 files, symlinks=0`.
- **Runs.** `probe-runner.py` ran twice, with `--no-cache` on each of 26 commands (`call-stats --dump-sites`,
  `dfg-stats --edges`, and 12 × `nodes-at` plus 12 × `callers`). All exited 0 with empty stderr. The two runs are
  byte-identical: both `MANIFEST.json` files hash to `20164e85…5ee3`.
- **Extracts.** `extract.py` (plan folder, `9efdf7ba…c533`) was applied to both runs, and the results are
  byte-identical. They were re-derived again at commit time and still match `extracts/`.
- **Reconciliation.** gpt-5.6-sol at xhigh re-derived E1/E3/E4/E5 for all 12 targets and Y/S statically from the
  hashed outputs and source, without trusting the extractor. It spot-checked E2 and recomputed six output hashes
  against both manifests. Verdict: **RECONCILED**.

## Review history

- **Spec r1.** FIX, 1 WRONG / 1 SMELL. The WRONG was that the jq-filter prose was not executable; it was replaced by
  `extract.py`. The SMELL was compressed refusal lanes.
- **Spec r2.** FIX, 0 WRONG / 1 SMELL: two stale summaries in README and P5. Folded; converging within the 2-round
  cap.
- **Controller-found defect.** Before any use, the first `extract.py` version counted the literal `true` as an
  identifier, which gave Y = 3 with S = 1 (still below the threshold). This was fixed and disclosed to the reviewer.
  The reviewer found no remaining similar misclassification.

## Not done

- No new code, no build beyond the release `prism` used for probing, and no Tier-A run. Production is unchanged.
- The reviewer did not re-run the full 1,229-file input verification; the controller did, before the runs.
- Measurement covers one corpus. A second corpus would be needed before claiming the positional gap is irrelevant in
  general. Option B, the byte-exact observer, remains available if that is ever wanted.

# Native positional-gap characterization — readout (2026-09-24)

**Scope.** This readout is a research observation over exactly 12 Excalidraw ArrowFunction sites (11 files, 37,040
bytes; manifest `site-manifest.json`, SHA-256 `789352a5…326b`). It measures what Prism's existing native AST APIs
report for callables that have an object-pattern parameter followed by a later ordinary required identifier.

It is not parameter support, CPG entry, callee resolution, or runtime demand. The observation packet itself states
`authorizes_runtime_edge:false`, `native_entry_measured:false`, and `callee_resolution_measured:false`.

## Conclusion (SPEC §8 option (a))

**Bounded next proof required.** All 12 sites are clean, uniquely named native candidates, and each is `eligible`: a
named native binding sits at a selected later-required source ordinal, and the non-null native positional-slot
prefix has no slot at that ordinal.

The packet disposition is `next_action: bounded_entry_and_call_proof`, with reason
`named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof`. This authorizes only a separately
planned CPG-entry and resolved-call proof over these site identities. No implementation is recommended here.

| Site (path : bytes) | Callable | Raw parameters (ordinal: shape) | Native slots | Binding outside the prefix |
|---|---|---|---|---|
| `…/ColorPicker/Picker.tsx:1379–5671` | Picker | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/FilledButton.tsx:902–2925` | FilledButton | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/Island.tsx:273–480` | Island | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/PropertiesPopover.tsx:802–3130` | PropertiesPopover | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/QuickSearch.tsx:318–707` | QuickSearch | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/Sidebar/Sidebar.tsx:1483–4701` | SidebarInner | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/Stack.tsx:371–784` | RowStack | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/Stack.tsx:821–1234` | ColStack | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/TextField.tsx:781–2943` | TextField | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/ToolButton.tsx:1572–5662` | ToolButton | 0: object, 1: identifier | `[]` | `ref` @1 |
| `…/data/index.ts:1228–2535` | prepareElementsForExport | 0: identifier, 1: object, 2: identifier | `[elements@0]` | `exportSelectionOnly` @2 |
| `…/scene/zoom.ts:95–953` | getStateForZoom | 0: object, 1: identifier | `[]` | `appState` @1 |

Status partition: `unique_named` 12, `unique_unnamed` 0, `missing` 0, `ambiguous` 0, `recovery_quarantined` 0.
The total is 12, so the full denominator is preserved.

**Observed pattern.** Ten of the twelve are React `forwardRef`-style components, `({…props}, ref)`, where the second
positional argument `ref` is invisible to the positional-slot prefix. The native prefix stops at the first
object-pattern parameter. The other two are ordinary helpers with the same shape. This is the systematic gap that a
later entry/call proof would have to address. It is a characterization only, not an admission.

## Custody

- **Source.** Commit `700c1519`, which the branch later rebased onto `main` with no code change. Production is
  equivalent to the PR #319 merge `30e13053`: `src`, `Cargo.toml`, `Cargo.lock`, and `build.rs` have an empty diff.
- **Native binary.** `parameter_slot_characterization`, SHA-256 `964b2076…2f84`, from an offline, locked build.
- **Input.** The recovered durable Excalidraw `0642e72c` tree. Its authentication is recorded in
  `input-recovery.md`.
- **Runs.** Two cold runs of `request.mjs … | <native binary>`. Both exited 0, and the outputs are byte-identical:
  SHA-256 `d7f7b1b8c8b107f3e666169c61dec507061db1e79c171a1b95cdb99163ff9355`, 10,565 bytes, stored as
  `observation.json`. The exact commands and hashes are in `receipt.json`.
- **Independent reconciliation (SPEC §7 step 3).** gpt-5.6-sol, xhigh, static. It re-derived every row's status and
  eligibility from the recorded fields and the source text. Verdict: **RECONCILED, 0 WRONG / 0 SMELL**.

## Review history

- r1–r5 were the original launcher design. It was parked twice with non-converging custody findings.
- The owner re-planned to (b), following the Fable advisor: keep the reviewed worker and replace the launcher with a
  request builder.
- The (b) spec was approved in round 2. The implementation took 2 rounds plus a disclosed narrow confirmation, which
  approved.
- The owner decided REPLAN-B §9: the worker owns request-schema validation.
- Details: `docs/superpowers/plans/2026-09-19-post319-native-positional-gap/`.

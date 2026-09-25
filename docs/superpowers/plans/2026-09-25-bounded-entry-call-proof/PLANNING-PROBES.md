# Planning probes: bounded entry/call proof (2026-09-24)

This file records read-only measurements taken with **existing** Prism commands on the pinned Excalidraw input, before
any design. Each probe lists the hypothesis, what it predicts, what would refute it, and what was observed. No new tool
was run on the public input. The planning skeleton (P9) ran on synthetic fixtures only.

## Environment and custody

| Item | Value |
|---|---|
| Planning clone | `/Users/wesleyjinks/code/prism-plan-entry-call`, branch `plan/bounded-entry-call-proof`, base `origin/main` `7ecccd99` |
| Binary | `cargo build --release --offline --locked --bin prism` (27.8 s). `target/release/prism` SHA-256 `263f71adf43286f70861d9ac649e5aae2fcf5fb7b2f285540b9ba86533acf3b1` |
| Input root | `/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source` |
| Input check | `cd <root> && shasum -a 256 -c ../source-file-manifest.sha256 --quiet` gives `VERIFY_OK`. The manifest lists 1,229 files and has SHA-256 `1a6c4397…68f0`. The `find` set equals the manifest set, and there are 0 symlinks |
| Nav cache | `--cache-dir <scratch>/navcache`. The first query was a cold miss and rebuilt in 34 s. Later queries reused that cache. `--no-cache` was used for the synthetic repos |
| Predecessor packet | `target/planner-context/observation.json` SHA-256 `d7f7b1b8…9355`, and `site-manifest.json` `789352a5…326b`. Both match the predecessor receipt |

Full outputs live in the session scratchpad, which is not durable, so their hashes are recorded here:

- `call-stats --dump-sites`: `849f389a…393f58e` (19,219 records)
- `dfg-stats --edges`: `0653ceb9…b0818` (60,163 edges)

Compact filtered extracts are durable in `probe-extracts/`.

## P1. Do resolved callers exist for the 12 callables?

- **Hypothesis H1a.** The two plain helpers (`prepareElementsForExport`, `getStateForZoom`) have Exact callers, and
  the ten `forwardRef` components have JSX callers.
- **Refuted if** any callable shows zero callers even though grep finds uses.
- **Command:** `prism nav --cache-dir <c> callers --repo <root> --symbol <name> --file <path> --format json`, once per
  site.

**Observed:**

| Callable | `nav callers` items | Kind / score |
|---|---:|---|
| getStateForZoom | 5 | `import_member`, 1.0 (actionCanvas.tsx 144/185/226; App.tsx 4327, 6930) |
| prepareElementsForExport | 3 | `import_member`, 1.0 (actionClipboard.tsx 136/209; ImageExportDialog.tsx 108) |
| Island, Picker, FilledButton, PropertiesPopover, QuickSearch, SidebarInner, RowStack, ColStack, TextField, ToolButton | **0 each** | — |

H1a holds for the helpers. **It is refuted for all ten components:** they have zero resolved callers even though JSX
uses exist (for example, `<Island` appears in six files).

## P2. Why do the components have no callers? (Separating probe)

- **H2 (wrapper).** `export const X = React.forwardRef(arrow)` is not recorded as a callable export, so an imported
  `<X/>` drops.
- **Alternative H2′ (JSX).** Cross-file JSX call sites never resolve.
- **Discriminator.** Under H2, a plain-arrow component used via JSX resolves while a wrapped one drops. Under H2′, both
  drop.
- **Commands:**
  - `nav call-stats --repo <root> --dump-sites` on the public tree, filtered by `callee_text`.
  - Synthetic `fx1` (`a.tsx`: `Island = React.forwardRef(...)`, `Plain = arrow`, `Wrapped = React.memo(...)`,
    `helper = arrow`; `b.tsx` uses all four) with `--no-cache`.

**Observed on the public tree** (extract `probe-extracts/call-sites-selected.jsonl`):

| Callee text | Sites | Disposition |
|---|---:|---|
| Island | 11 | `UnknownName` 11 |
| ToolButton | 40 | `UnknownName` 40 |
| FilledButton | 13 | `UnknownName` 13 |
| TextField | 7 | `UnknownName` 7 |
| PropertiesPopover | 6 | `UnknownName` 6 |
| QuickSearch | 2 | `UnknownName` 2 |
| Picker | 2 | ColorPicker.tsx:151 `UnknownName`; IconPicker.tsx:319 resolves to a different, local `Picker` (`local_def`) |
| Row / Col (`<Stack.Row>`, `<Stack.Col>`) | 7 | `UnknownName` 7. The callee text is the member property, not `RowStack`/`ColStack` |
| SidebarInner | 0 records | Its one JSX use (Sidebar.tsx:214) sits inside `Object.assign(forwardRef((props, ref) => …))`, so no call-site record exists |
| getStateForZoom | 5 | Resolved. Two syntactic calls (App.tsx:5678, 12817) have **no record** (inside `withBatchedUpdates(...)` / `setState` callback arrows) |
| prepareElementsForExport | 4 | 3 resolved; resave.ts:38 `UnknownName` (`from "."`). The test file's calls (export.test.ts:446/491/550, inside `it(...)` arrows) have no record |

**Observed on `fx1`:**

- `Plain` via JSX: `Plain@5:exact:import_member`.
- `Island` (forwardRef) and `Wrapped` (memo): `UnknownName`.
- `helper(...)` call: Exact.

**Result:** H2 is confirmed and H2′ is refuted. The mechanism is at `src/ast.rs:2874-2890`: an exported
`variable_declarator` records an export only when its initializer is `arrow_function` or `function_expression`. A
call initializer such as `forwardRef(...)` or `memo(...)` is `skipped_expr_count += 1`. This is a deliberate precision
rule (review-fix F4), so the wrapped component's JSX sites drop with `UnknownName`.

## P3. Does the CPG have an entry Def for each out-of-prefix binding?

- **H3.** Every selected binding has exactly one `Variable{Def}` at the callee's start line, with the binding's exact
  bytes.
- **Refuted if** any count is not 1.
- **Why it could fail.** `src/data_flow.rs:597-624` registers parameter Defs from `function_parameter_occurrences`, but
  `:602` skips a parameter that has no bare reference (field-only use).
- **Command:** `nav nodes-at --repo <root> --location <path>:<start_line> --format json`, filtered by
  `function==<callable>`, `access==Def`, and exact `start_byte`/`end_byte` from `observation.json` (extract
  `probe-extracts/entry-defs-nodes-at.txt`).

**Observed:** entry Def is exactly 1 for 11 sites (all ten `ref`@1, and `exportSelectionOnly`@2 at bytes
1338–1357). It is **0 for `getStateForZoom` `appState`@1**. The `nodes-at` output for zoom.ts:3 has no parameter
Variable at all, and the body uses only `appState.offsetLeft`, `appState.zoom.value`, and similar field accesses. So H3
holds for 11 sites and is refuted for one, with a named mechanism: field-only use skips the Def by design (field
isolation).

## P4. Does arg→param flow exist at the selected ordinal?

- **H4.** No inbound DataFlow edge reaches any selected entry Def, because the positional prefix
  (`src/parameter_slots.rs:437-449` for JS and `:451-479` for TS) stops at the object pattern, so Step 5b
  (`src/cpg/build.rs:1476-1478`) never reaches that ordinal.
- **Control.** Where the prefix *does* cover an ordinal (`prepareElementsForExport` `elements`@0), edges must exist.
- **Command:** `nav dfg-stats --repo <root> --edges`, filtered to `to == (path, start_line, base, def)` (extract
  `probe-extracts/edges-into-entry-lines.jsonl`).

**Observed:**

- **0** inbound edges into each of the 12 selected entry keys.
- The control holds: **3** Exact edges into `data/index.ts:47 elements`, one from each resolved caller
  (actionClipboard.tsx:137 and :210, ImageExportDialog.tsx:109).
- Sibling ordinal 2 (`exportSelectionOnly`) receives none.

H4 holds, and the in-situ control shows the probe can see this kind of edge.

**Limitation (why an observer adds value).** The `dfg-stats` edge rows have no function identity and no byte spans.
Absence claims are robust: zero edges into a `(file, line, base, def)` key means zero edges into the exact binding. A
*presence* claim, however, could be confounded by a same-line alias twin; see P8, where `bare`'s `st` has a twin Def
on its line.

## P5. Which blocked callers would a positional repair actually unblock?

This probe uses manual source reading of the resolved callers.

| Callee / ordinal | Resolved caller | Argument at the ordinal | Would a positional hole emit an edge? |
|---|---|---|---|
| prepareElementsForExport / 2 | actionClipboard.tsx:136 | literal `true` | No (no variable occurrence) |
| prepareElementsForExport / 2 | actionClipboard.tsx:209 | literal `true` | No |
| prepareElementsForExport / 2 | ImageExportDialog.tsx:108 | identifier `exportSelectionOnly` | **Yes, plausibly (the only one)** |
| getStateForZoom / 1 | 5 callers: actionCanvas.tsx:144/185/226, App.tsx:4327, App.tsx:6930 | `appState` ×3, member expression `this.state`, identifier `state` | No. Entry Def absent (P3) |
| 10 components / 1 | none resolved | JSX has no argument list (`src/languages/mod.rs:835-850`; `src/cpg/build.rs:1463`) | No. Even if the callers resolved, `ref` is a JSX prop that React routes; it is never a positional argument |

**Predicted counterfactual yield:** 1 caller tuple at 1 site.

## P6. Separating fixture for every refusal class (synthetic `fx2`, `--no-cache`)

`lib.ts` defines `both(a, later)`, `mid(a, {pad}, later)`, `fieldOnly({pad}, st)` (field-only `st`), and
`bare({pad}, st)` (`const s = st`). `use.ts` calls each.

**Observed cross-file edges:**

- `both`: `n→a`, `later→later`
- `mid`: `n→a` only, at both call sites

**Observed entry Defs:**

- `both.a`, `both.later`, `mid.a`, `mid.later`, `bare.st` (286–288) are present.
- `fieldOnly.st` is absent.
- An extra `st` Def at 316–317 (the alias twin from `const s = st`) sits on `bare`'s start line.

These confirm each stage class in isolation and supply the same-line twin used as the RED input below.

## P7. Is every public API an observer needs actually public?

- **Hypothesis.** A `cargo` example can read everything through `pub` items with no `src/` change.
- **Refuted if** any item is `pub(crate)`.
- **Checked:**
  - `repo_loader::load_repo` (`src/repo_loader.rs:64`)
  - `CpgContext::build_with_scope_graph_inputs` (`src/cpg/context.rs:67`), which is the nav cold path
    (`src/navigation/cache.rs:80+`)
  - `CodePropertyGraph.graph` / `.call_graph` (`pub`)
  - `CpgNode` / `CpgEdge` / `VarAccess` (`src/cpg.rs:65`)
  - `CallGraph.calls` (`src/call_graph.rs:708`) and `CallSite` fields (`:420-440`)
  - `resolve_call_site` / `resolve_call_site_full` (`src/resolution.rs:3683`, `:2294`), and `ResolutionOutcome.drop`
    (`:1080-1085`)
  - `ParsedFile::{all_functions, function_parameter_slot_occurrences, has_bare_references, node_line_range}`, `.tree`
    (`pub`)
  - `Language::{is_call_node, call_function_name, call_arguments, function_name}`

  `call_argument_texts_and_spans_at` (`src/ast.rs:11520`) is `pub(crate)`, so the observer re-derives argument spans
  with the same rule: named, non-comment children of `call_arguments`, as in `src/ast.rs:518-542`.
- **Observed:** all needed items are public. The P9 skeleton compiled against the clone unchanged, and clippy
  `-W clippy::all` was clean.

## P8. RED input: does a name-keyed entry match err on a concrete value?

- **H8.** Keying the entry Def by `(file, function, line, name)` without exact bytes reports `bare.st` as ambiguous,
  because the same-line alias twin also matches. Exact bytes report it as present.
- **Observed** (skeleton adapter switch `exact_entry=false` on the synthetic `calls` fixture):

  ```
  exact=true  bare.entry={"status":"present"}   bare.prefix_only_blocked=true  exact_prefix_only_blocked=2
  exact=false bare.entry={"status":"ambiguous"} bare.prefix_only_blocked=false exact_prefix_only_blocked=1
  ```

  This is a behavioral difference on a concrete value, so the RED is admissible.

## P9. Skeleton feasibility and honest size (synthetic fixtures only)

**What was built.** `skeleton/main.rs.txt` (SHA-256 `bfff8c48…c3e3`). It is a scratch cargo project that depends on
the planning clone by path. It compiles, is clippy-clean, and ran on `skeleton/fixtures/{calls,jsx,identity}`.

**Line counts.** After `rustfmt --edition 2021`, non-blank and non-`//` lines are: **helper 618**, **`#[cfg(test)]`
158**. Four test lines exceed 100 columns (raw JSON strings; these must be split).

**Synthetic results:**

- **`calls`.** `both` has flow present (both ordinals). `mid` (variable) and `bare` (×2) are `prefix_only_blocked`. The
  literal `mid(…, true)` is not blocked, and neither is `fieldOnly` (entry absent). The comment-argument call
  `both(/* c */ n, later)` gives flow present, with no mismatch. Decision: `targeted_repair_warranted` (Y=3, S=2).
- **`jsx`.**
  - `Island` (forwardRef): 0 callers, 1 `dropped UnknownName`.
  - `PlainRef` JSX site: Exact, `arg_count:null`, `no_positional_arguments`.
  - `PlainRef({…}, r)` call: `prefix_only_blocked`.
  - `setState` callback in a class method: owned; its argument has no caller-identity variable.
  - Wrapped class-field arrow `onChange = wrap((e) => PlainRef(…, e))`: `unowned`, which reproduces the App.tsx:5678
    shape.
- **`identity`.** Wrong bytes, wrong name, and a missing file are each `missing`.
- **Repeat.** Two runs are byte-identical (`41a984ca…`, pre-comment-row version).

**Skeleton caveat.** The skeleton predates two SPEC additions: the `callable_not_unique → inconclusive` rule, and the
`resolved_targets_at_site` field. The forecast in SPEC §9 includes them.

## Summary of measured facts

1. **Entry Defs.** Present for 11 of 12 sites. Absent for `getStateForZoom.appState`, because field-only use skips the
   Def by design.
2. **Resolved callers.** Exist only for the two helpers (5 and 3, all Exact `import_member`). The ten components have
   none. Their JSX uses drop as `UnknownName` because wrapped exports are unrecorded by design
   (`src/ast.rs:2884-2890`). Separately, some call sites are unowned (nested callbacks) or use member aliases
   (`Stack.Row`).
3. **Arg→param flow.** Zero edges into any of the 12 selected ordinals. The in-prefix control edges exist (3 of 3 into
   `elements`@0).
4. **Counterfactual.** A positional-hole repair would unblock at most one tuple: ImageExportDialog.tsx:108 →
   `exportSelectionOnly`@2. The components' `ref` is framework-routed; JSX supplies no positional argument 1.
5. **Prediction for the bounded observer:** `defer`, with reason `prefix_only_yield_below_threshold`, Y=1, S=1. This
   is falsifiable by the run.

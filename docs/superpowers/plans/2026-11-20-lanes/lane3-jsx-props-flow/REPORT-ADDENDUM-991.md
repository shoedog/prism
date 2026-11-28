# Lane 3 report addendum: issue #991 diagnosis and the review-example re-measure (2026-11-28)

Product: main f720d717 (the later commits on main are docs and targets). Evidence: `~/prism-evidence/jsx-props/f991/` (`DIAGNOSIS.md` with the 53 rows, `ledger-3.txt`, `ws-counts.json`) and `~/prism-evidence/jsx-props/g/examples/f991/` (records, packs, reader labels). Tools: `tools/seed_diag.py`, `seed_ledger.sh`, `jsx_sites.cjs`, `examples.py` (fixed).

## 1. Why 2 of 15 forward cal.com commits gained context: the 53 seeds, in counts

| Class | Seeds | Cause |
|---|---|---|
| Started on an added props edge, frontier followed | 3 | none |
| Started on an added props edge, frontier refused at depth 0 | 3 | deliberate under DESIGN 5.6: 2 `target_not_binding` (the edge ends at a member-read Use `props.x`; `descend_target` accepts a Def), 1 `no_matching_site` (the line's only Use is the `{banners &&` condition, outside the JSX opening) |
| Query error | 12 | all `SymbolNotFound`: the line holds no variable node (`<>`, `src="..."`, `return;`, `}`, zero-width `t("...")`). The earlier run read stderr, so no reason was recorded. Product correct; tool defect, fixed |
| No added edge | 35 | 16 component imported through a workspace package specifier (`@calcom/ui/...`), no props edge at the JSX site; 7 attribute with no value variable; 6 changed line is not a props value; 3 dynamic component (`getMDXComponents().wrapper`); 2 npm package component; 1 intrinsic tag |

The walk is not the cause. Fixing every depth-0 refusal would raise forward gain from 2 to at most 3 of 15 (gate 8). The controller decided no walk fix.

### The workspace-package lever
cal.com at f972feef, non-test `.tsx`/`.jsx`, JSX elements with a capitalised tag and at least one non-literal attribute value, by where the component's import comes from, and how many have a cross-file data-flow edge from a line of the element:

| Import form | Sites | With a cross-file edge |
|---|---|---|
| workspace package (`@calcom/*`, `@coss/*`) | 2,084 | 32 (1.5%) |
| relative | 551 | 263 (47.7%) |
| npm package | 628 | 18 |
| same file or unbound | 412 | 5 |

Five of the 16 workspace seeds, from source: `EmptyScreen` (`@calcom/ui/components/empty-screen`, a named re-export of `export function EmptyScreen({..})`), `VerticalTabItem` (`@calcom/ui/components/navigation`, a `default as` re-export of an arrow component), `Button` (`@calcom/ui/components/button`, a named re-export of a `forwardRef` component), `Form` (`@calcom/ui/components/form`, a named re-export from `./inputs/Form`), `Tooltip` (`@calcom/ui/components/tooltip`, `default as` re-export). All five are defined in `packages/ui` of the same repository and reached through the package's `exports` subpath map (`"./components/empty-screen": "./components/empty-screen/index.ts"`) and a barrel `index.ts`. The `callers` query confirms the sites do not resolve: `EmptyScreen` has 1 caller, `Tooltip` has 9 callers, all inside `packages/ui` through relative imports. Prism resolves bare specifiers only through tsconfig `paths`; resolving package `exports` entries is the parked package-resolution lane, issue #352 (nothing new to file). This, not the walk, is where review examples on a monorepo would gain context. Not measured: how many of the 2,084 sites would get an edge after resolution (the callee must also be a pattern-formal or projectable component).

## 2. Re-measure (fresh seeded draw, seed 20261127, pool of 60 qualifying commits per direction, R2 sample excluded)
Same method as report section 5. cal.com f972feef; Excalidraw from a full clone (`~/code/prism-jsx-census-corpora/excalidraw`, HEAD f4e5226a, history walked from the pinned 0642e72c). Readers: Opus-5.5 high and Sonnet-5.5 high, separate sessions, blind to each other, on the 11 examples that gained context (combined label: useful if both, noise if either, else neutral).

| Row | cal.com R2 sample (old) | cal.com fresh | Excalidraw fresh |
|---|---|---|---|
| Gate as defined: forward, all seeds | 2 / 15 | 3 / 15 | 5 / 15 |
| Gate as defined: backward | 1 / 15 | 2 / 15 | 1 / 15 |
| Forward restricted to commits with at least one seed that names a variable | 2 / 14 | 3 / 15 | 5 / 14 |
| Seeds with no variable (`SymbolNotFound`) / all seeds | 12 / 53 | 13 / 57 | 10 / 73 |

The second row group is a reading aid; the gate is the first. Backward (`fullflow` on the patch) has no seed errors, so it has no restricted row. Gate: at least 8 of 15 gain context, of those at least 60% useful and at most 10% noise.

Readers on the fresh sample: cal.com gained 5: 2 useful, 1 neutral, 2 noise; Excalidraw gained 6: 3 useful, 2 neutral, 1 noise. Equal labels 6 of 11. Disagreements (not averaged): cc-bwd-02 (Opus noise, Sonnet neutral), cc-bwd-13 (noise / neutral), cc-fwd-09 (useful / neutral), ex-fwd-05 (neutral / useful), ex-fwd-10 (useful / neutral); a reason per label is in `g/examples/f991/readers/`.

**The review-example gate is still not met on either corpus.** Forward gains are 3 and 5 of 15 against 8; the useful share of gained examples is 2 of 5 and 3 of 6 against 60%, and noise is 2 of 5 and 1 of 6 against 10%. The threshold is unchanged. #596 stays open for the owner.

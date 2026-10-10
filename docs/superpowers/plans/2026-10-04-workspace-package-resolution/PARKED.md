# PARKED — lane PKG: JS/TS workspace package-entry resolution

**Status:** parked by the owner on 2026-10-04, together with lane S2. Tracked in the GitHub issue linked from PR #344.

## Branches (all on `origin`)

| Branch | Head | What it holds |
|---|---|---|
| `plan/workspace-package-resolution` | this branch | The plan packet: census, SPEC with the three-way result contract (`Bound` / `ProvenUnresolved` / `Unsupported`), the TypeScript 5.9.3 differential gate (`probes/`), `R1-REPORT.md`, `OQ.md` (park note), `reviews/`, `briefs/`. Draft PR #344. |
| `proto/workspace-package-resolution` | `03fa9c29` | The prototype implementation, two commits off main `4e592daa`. |

### Commits on `proto/workspace-package-resolution`

| Commit | State |
|---|---|
| `92c1d0bc` | Bounded package-entry resolver prototype. |
| `03fa9c29` | R1: folds spec review round 1. The differential gate passes 5,096 cases with 0 wrong bindings; public call streams are unchanged. |

## Why it is parked
The lane adds no call edges on its own; its value was unblocking lane S2. Spec review round 2 found about eight more closed divergences from TypeScript's resolver (JSON winners, trailing-slash `exports` keys, colon paths for ESM writers, secondary-pass priority, `@types` siblings), and found that the gate's axes cannot catch most false `ProvenUnresolved` results. Those findings are **not folded**; they are listed in `reviews/spec-r2-opus.md` and `reviews/spec-r2-sol.md`.

## To resume
- Fold the round-2 findings first, and widen the differential gate's axes as the reviews describe, before any further review round.
- Rebase onto current main. Main's `CACHE_VERSION` has moved (107 after PR #350), and the version is pinned in **two** mutant registries: `mutants/js-param-defs.json` (PD-11) and `mutants/lane-p-tsconfig-paths.json` (P2-M11). Re-pin both.

## Reusable assets
- The differential TypeScript-oracle gate harness (`probes/`).
- The three-way resolution result contract.
- The round-1 and round-2 review findings: a list of every known divergence from TypeScript's resolver.

## Related
- Lane S2 (consumer): branches `plan/s2-import-qualifiers` and `proto/s2-import-qualifiers`, draft PR #343.
- On main: `docs/features/language-coverage/jsx-tsx-react-plan.md`, section "Parked JS/TS lanes (2026-10-04)".

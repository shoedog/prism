# PARKED — lane S2: JS/TS import-qualifier Exact edges

**Status:** parked by the owner on 2026-10-04, together with lane PKG. Tracked in the GitHub issue linked from PR #343.

## Branches (all on `origin`)

| Branch | Head | What it holds |
|---|---|---|
| `plan/s2-import-qualifiers` | this branch | The plan packet: SPEC (with the §2a escape-channel table), `OQ-s2.md` (owner decisions S2-O6 to S2-O9 and the park note), measurements, `reviews/`, `briefs/`. Draft PR #343. |
| `proto/s2-import-qualifiers` | `06fdb649` | The prototype implementation, four commits off main `4e592daa`. |

### Commits on `proto/s2-import-qualifiers`

| Commit | State |
|---|---|
| `c35719e1` | Fail-closed whitelist-of-uses prototype (X +132 CORRECT). |
| `75a35a5e` | R2: static-binding guards under owner decision S2-O7. **X +132, F 0 changed. The last state with positive yield.** |
| `61641bdb` | R3b: precise refusal joins (X 0). |
| `06fdb649` | R4: rebased on lane PKG `03fa9c29`; only `ProvenUnresolved` writer imports are out of model (S2-O9). X 0. Contains lane PKG's resolver code. |

## Why it is parked
Sound refusal joins need prism to resolve every writer import exactly as TypeScript does. One unresolved or unsupported import revokes every gain in that package, because the refusal target cannot be narrowed. Yield fell to 0 six times. The prize was X +132 class-static edges on one public corpus; the private target corpus gains nothing.

## To resume
- Start from `75a35a5e` (the S2-O7 static-binding state) with an explicit owner ruling on unresolvable writer imports, or resume after lane PKG reaches full TypeScript resolution parity.
- Rebase onto current main. Main's `CACHE_VERSION` has moved (107 after PR #350), and the version is pinned in **two** mutant registries: `mutants/js-param-defs.json` (PD-11) and `mutants/lane-p-tsconfig-paths.json` (P2-M11). Re-pin both.
- The resume binaries for `CONTROLLER-s2.sh` are not on the remote; rebuild them from the commits above.

## Related
- Lane PKG (prerequisite): branches `plan/workspace-package-resolution` and `proto/workspace-package-resolution`, draft PR #344.
- On main: `docs/features/language-coverage/jsx-tsx-react-plan.md`, section "Parked JS/TS lanes (2026-10-04)".

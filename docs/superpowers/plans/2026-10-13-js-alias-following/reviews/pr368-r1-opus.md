# Review of PR #368, slice I1 (relative imports: `.js` suffix, bare dots, redundant segments)

Reviewer: Opus. Round 1 of at most 2. Head reviewed: `aa74eae0` on `origin/main` `e2701e89`.

**Verdict: FIX.** One thing must change before merge (three mutation-registry anchors orphaned by the cache-version bumps; the pre-merge mutation gate exits 1). One decision must be recorded: the goal's second clause, "nothing that resolves today changes", holds for the candidate list but not for edges. I found three consumers where a call that resolves on main comes out differently on this branch. I recommend accepting that behaviour and saying so, not changing product code.

The candidate function itself is sound. Every wrong or changed edge below comes from what a consumer does once a module is found, or from a model limit that already exists on main for the canonical spelling.

## How I tested

- Behaviour: the two prebuilt binaries on synthetic fixtures, each run with a fresh cache directory. Branch `i1b-prism` reports `aa74eae0`. Main `base-prism` reports `40ad8390`, which is the pre-squash PR-B commit; its `src` diff against the merge base `e2701e89` is the MCP code plus provenance accessors in `cpg_cache.rs` and `navigation/`, so it is a valid same-environment control for call resolution.
- Every difference attributed to the branch below has the main binary's output on the same fixture beside it. Where I say "same as the canonical spelling", I ran the canonical twin on main.
- Fixtures are under `opus-scratch/fx/<name>`; outputs are `opus-scratch/out-<name>.base.txt` and `.i1.txt`; `opus-scratch/bin/cmp.sh <fixture>` reruns one.
- Mutants: a verbatim copy of `src/call_graph.rs` lines 6686-6853 plus the PR's unit test file in a scratch crate (`opus-scratch/mut`), 31 mutants, unit tests only.
- The worktree was not modified (`git status` empty before and after).

## Findings

### W1. WRONG, MATERIAL. The cache-version bumps orphan three mutation-registry anchors; the authoritative mutation gate exits 1

- `mutants/js-param-defs.json` lines 96-101 (`PD-11-cache-version-not-bumped`) and `mutants/lane-p-tsconfig-paths.json` lines 708-719 (`P2-M11-cpg-cache-version`, `P2-M12-nav-cache-version`) anchor on the literal text `const CACHE_VERSION: u32 = 114;` and `const NAV_CALL_EDGE_CACHE_VERSION: u32 = 62;`.
- The PR changes those lines to 115 and 63 (`src/cpg_cache.rs:254`, `src/navigation/call_edge_cache.rs:108`) and does not touch the registries.
- Probe: `python3 scripts/mutgate/mutgate.py --plan-only --out <scratch>` on the branch exits 1 and reports all three as `anchor-missing` (`opus-scratch/mutgate-plan/schemata-plan.json`). Control: each anchor string occurs exactly once on `origin/main` and zero times on the branch.
- In a real run `anchor-missing` becomes `admissible: False`, verdict INADMISSIBLE, exit 1 (`scripts/mutgate/mutgate.py:142-145, 614-617, 684-685`). The README says registered anchors are selected even outside the diff, so the scoped `--since main` run fails too.
- CI does not run the gate (`.github/workflows` has no reference to it), so the green CI on this PR does not show the break.
- I tag this MATERIAL because `CLAUDE.md` names the gate as the authoritative pre-merge check and these three mutants are the ones that prove the version pin tests work. It does not affect resolved edges.
- **Fix:** in both files change the anchor to the new value and the replacement to the old one (115 to 114 twice, 63 to 62 once), then run the gate for those three ids. PR-B did the same ("both registry anchors advance together").
- **Alternative:** make the anchors version-independent. Cost: mutgate only supports literal anchors, so this is a tool change, larger than the slice.
- **Would not apply** if the two lane files were retired. They are not: mutgate loads every `mutants/*.json` on each run.

### W2. WRONG, MATERIAL. A correct NameOnly edge on main is dropped when an imported props type is reached through a `.js` specifier

Fixture `p3a`:

```ts
// client.ts
export class Base { m() { return 1; } }
export default class Client extends Base { other() { return 2; } }
// props.ts
import type Client from "./client";
export type Props = { client: Client };
// app.ts
import type { Props } from "./props.js";
export function run({ client }: Props) { client.m(); }
```

| | `client.m()` |
|---|---|
| main | `client.ts:m@1 name_only/r6_single_owner` (this is `Base.m`, the real target) |
| branch | `DROP ExternalReceiver` |
| main, canonical `./props` (`p3ac`) | `DROP ExternalReceiver` |

- Mechanism: `js_ts_props::collect` creates a proof only when the props module has exactly one candidate (`src/js_ts_props.rs:171-175`). On main `./props.js` has none, so there is no proof and the site falls through to the unknown-receiver rung. On the branch the proof exists, and a site with a proof either hits a direct method of the proven class or returns `dropped(ExternalReceiver)` (`src/resolution.rs:2700-2722`). An inherited method is a miss.
- This is the case question 1 asks about: a call that used to fall through to a later rung now stops.
- The same path also turns a wrong NameOnly edge into a drop (`p3c`: props class imported from a package, an unrelated in-repo class has `m`) and a NameOnly edge into the correct Exact one (`p3b`).
- Incidence: zero in X and T. My own recount of the stored site dumps finds no resolved-to-dropped record.
- I tag this MATERIAL because it is a direct counterexample to "nothing that resolves today changes", not because of its frequency.
- **Fix (recommended):** accept it. The branch makes the `.js` spelling behave exactly like the canonical spelling, and that is a better invariant than strict additivity. Say so in the PR description and pin it with a twin test (props through `./props.js`: direct-method hit, inherited-method drop).
- **Alternative A:** on a miss, try the direct same-file base as the Python arm already does (`resolution.rs:2948-2958`). This recovers `Base.m` for every spelling. Cost: it changes rows that resolve on main today, so it needs its own slice and measurement.
- **Alternative B:** mark proofs that exist only because of the I1 fallback and fall through on a miss. Cost: a provenance bit in a cached struct, and one import behaving differently by spelling, which is the inconsistency this slice removes.
- **Would not apply** when the class defines the method directly, or when the props import already resolved on main.

### W3. WRONG, MATERIAL. A redundant-segment namespace import whose module lacks the member is upgraded from NameOnly to Exact on an unrelated file

Fixture `p4`:

```ts
// pkg/x.ts
export function other() { return 1; }
// other/x.ts
export function f() { return 2; }
// pkg/app.ts
import * as N from "./a/../x";
export function g() { N.f(); }
```

| | `N.f()` |
|---|---|
| main | `other/x.ts:f@1 name_only/import_qualified` |
| branch | `other/x.ts:f@1 exact/import_qualified` |
| main, canonical `./x` (`p4c`) | `other/x.ts:f@1 exact/import_qualified` |

- `N` is `pkg/x.ts`, which has no `f`. The target is wrong on both; the branch grades it Exact.
- Mechanism: the qualifier rung preselects by file stem (`src/resolution.rs:2584-2603`). When the module is not found, `js_ts_namespace_outcome` returns the NameOnly grading (`src/resolution_js_namespace.rs:48-58`). When it is found but the member is absent, line 20 returns `None` and the caller falls back to `exact(matched, ImportQualified)` (`resolution.rs:2617-2620`).
- The canonical behaviour is pinned on main by `d5_authoritative_missing_member_and_fallback` in `tests/integration/js_binding_namespace_test.rs` (the `("lib", "export {};"), ("other/lib", LIB)` row expects Exact on `other/lib`).
- Only redundant-segment spellings are newly affected. For a plain `./x.js` the namespace route already found `x.ts` on main through `namespace_sibling`, so nothing changes there.
- Incidence: zero in X and T (no `import_qualified` record changes at all).
- Same tag reasoning as W2: a call that resolves on main resolves differently. This is also the one answer to question 5 that is not clean.
- **Fix (recommended):** accept and disclose, with a twin test.
- **Alternative:** when the module file is resolved, its namespace proof is complete and the member is absent, drop instead of returning `None`. This fixes the canonical spelling too. Cost: it changes a row pinned on main, so it is an S1b-4 follow-up, not this slice.
- **Would not apply** when the member is provable in the resolved module: then the branch returns the single correct Exact target and removes the decoy (`p4p`, `p22`: main two NameOnly targets, branch one Exact).

### W4. WRONG, IMMATERIAL. Bare `.` and `..` resolve to `index.*` even when the directory's `package.json` points `main` elsewhere

Fixture `p16`: `pkg/package.json` has `"main": "./lib/main.js"`; `pkg/lib/main.js` and `pkg/index.js` both define `f`; `pkg/test/t.js` has `const { f } = require("..")` and `pkg/test/t2.ts` has `import { f } from ".."`.

| | `f()` in both callers |
|---|---|
| main | `DROP UnknownName` |
| branch | `pkg/index.js:f@1 exact/import_member` |
| main, canonical `../../pkg` (`p16c`) | `pkg/index.js:f@1 exact/import_member` |

- Node resolves a directory through `package.json` `main` before `index`, so the real target is `pkg/lib/main.js`. I ran Node 24.15 on this fixture: `require("..")` from `pkg/test` resolves to `pkg/lib/main.js`. TypeScript does the same with `types` and `main` (from my knowledge of its resolver, not run).
- Mechanism: the relative lane's directory step is the index list only (`src/call_graph.rs:6770-6780`, and the same in the literal function). The tsconfig-paths lane already refuses a directory that has a `package.json` (`src/js_paths.rs:514-516`).
- I tag this IMMATERIAL because it does not contradict either clause of the goal as prism models a directory import: main does the same for `./dir`. It needs `main` to differ from the index and an index file to exist with the same exported name. Bare `..` from a test directory to a package root is the spelling most likely to meet a `package.json`, which is why I ran it.
- None of the six bare-dot sites in X (three directories) targets a directory with a `package.json`; I checked each against the pinned source. The seventh changed X site is a redundant-segment spelling (`../..//shortcut`).
- **Fix (recommended):** add "`package.json` `main`/`types` for a directory specifier" to the PR's not-modeled list and pin the row.
- **Alternative:** refuse a directory-index candidate from the new path when the directory has a `package.json`. Cost: the candidate function sees only `indexed_files`; the snapshot that knows about `package.json` is passed transiently to `apply_js_paths`, so this needs plumbing and cache handling, and without parsing `main` it would also refuse the common harmless case.
- **Would not apply** when the directory has no `package.json`, or its `main` is the index file.

### S1. SMELL, MATERIAL. An Exact edge on main is removed when a newly found `export *` branch exposes a real conflict

Fixture `p1`: `pkg/index.ts` is `export * from "./a.js"; export * from "./b";`, both `a.ts` and `b.ts` export `f`, and `app.ts` imports `f` from `./index`.

| | `f()` |
|---|---|
| main | `pkg/b.ts:f@1 exact/import_member` |
| branch | `DROP UnknownName` |
| main, canonical `./a` (`p1c`) | `DROP UnknownName` |

- Mechanism: an unresolved star branch is skipped (`src/js_exports.rs:646-648`); two distinct candidates are a conflict (`:685-688`). `p1d` reaches the same path through `export * from "."`.
- The branch is right: the name is ambiguous under ES module semantics, so main's edge was a false Exact. Node 24.15 rejects the equivalent `.mjs` program with "contains conflicting star exports for name 'f'" (`opus-scratch/node-esm`). That is why this is a SMELL, not a WRONG.
- It is MATERIAL only in that the measured row "an existing Exact target changed or removed: 0" is a corpus measurement, not an invariant. It belongs in the same recorded decision as W2 and W3.
- **Fix:** disclose and pin with a test. **Alternative:** keep newly found star branches out of conflict detection; cost: it preserves a false Exact, so I would not.
- **Would not apply** when both branches reach the same declaration, or the barrel has a direct named export.

### S2. SMELL, IMMATERIAL. Three import forms through bare dots still resolve nothing, so twin parity is not universal

| Fixture | Form | Bare-dot spelling (main and branch) | Canonical twin (main and branch) |
|---|---|---|---|
| `p5`, `p5c` | `import * as N from "."`, `N.f()` | `DROP ImportExternal` | `exact/import_qualified` |
| `p23` | `import idx from ".."`, `idx.f()` | `DROP ImportExternal` | `exact/import_qualified` |
| `p21` | `import { f } from ".."; export { f };` then imported elsewhere | `DROP UnknownName` | `exact/import_member` |

- Causes: the qualifier rung's stem prefilter computes an empty stem from `.` and `..` (`src/resolution.rs:2584-2606`), and `raw_relative_literal` (`src/js_exports.rs:230-232`) does not accept bare dots, so the forward is a blocked claim.
- All three fail closed: no wrong edge, no lost edge.
- Named, renamed and default-call imports, destructured `require`, `./`, `./..`, `../.` and `export ... from ".."` all work (`p9`, `p21`).
- **Fix:** list these as not covered. **Alternative (follow-up):** let the prefilter compare against the resolved candidate file, and let `raw_relative_literal` accept `.` and `..`; cost: both touch rungs that resolve on main.

### S3. SMELL, IMMATERIAL. The tsconfig-paths lane gives a different answer for the same barrel

Fixture `p17`: `barrel.ts` is `export { real } from "./lib/real.js"; export * from "./lib/star.js";`, imported as `@lib` and as `./barrel`.

- Through `@lib`: dropped on main and branch. Through `./barrel`: dropped on main, Exact on branch. `p17c` confirms the alias works with canonical hop spellings.
- Cause: `Resolver::relative` requires the literal indexed JS file for a `.js` spelling (`src/js_paths.rs:481-488`).
- Leaving the lane unchanged is safe. In `p17b` (two stars, one spelled `.js`, both defining `f`) the alias route refuses through `via_unresolved_star`.
- It needs a follow-up for recall: path alias plus `.js` hops is the normal TypeScript ESM monorepo shape. Not for this slice.

### S4. SMELL, IMMATERIAL. Test gaps: three behaviour-changing mutants survive, and two consumer paths have no test

Of 31 mutants, 24 are killed and 7 survive.

| Mutant | Result | Comment |
|---|---|---|
| Drop the `..` arm of `directory_only` | survives | `..` from `pkg/sub/app.ts` would return `pkg.ts`. No test has a same-stem sibling of the parent directory. The branch itself is right (`p18`, `p18b`). |
| Drop `!directory_only` from the substitution guard | survives | `./x.js/` would return `x.ts`. No test spells `.js/`. The branch itself is right (`p18`). |
| Remove `trim()` in the normalized function | survives | `" ./x.js "` resolves Exact on the branch (`p15`); not pinned either way. |
| Never consult the literal list first | survives, equivalent | The normalized function returns the same list for every canonical spelling. |
| Return the normalized result even when empty | survives, equivalent for consumers | `None` versus `Some([])` is not observable in any consumer I read. |
| Drop `!target.is_empty()` on the file loop | survives, equivalent | Redundant with `!directory_only`. |
| Put substituted candidates after index candidates | survives, equivalent | Substitution only runs when there are no index candidates. |

- The first two also survive the integration tests by reading (no `pkg.ts` sibling, no `.js/` spelling); I did not run the integration tests against mutants.
- Suggested unit rows: `list("..", "pkg/sub/app.ts", &["pkg.ts"])` is empty and with `"pkg/index.ts"` added equals `["pkg/index.ts"]`; `list("./x.js/", APP, &["pkg/x.ts"])` is empty; one row that fixes the whitespace behaviour.
- The 97 receiver-typed changes in T go through `js_ts_direct_exported_class_owner` and `js_ts_props`. Neither has a test through a new spelling in this PR. `p8` and `p3b` show they work; a twin row for each would hold them.
- The new functions are in no mutation lane. A small lane file with the mutants above would cost little and replace the manual run.

### S5. SMELL, IMMATERIAL. Wording and numbers that do not match what I measured

- `CLAUDE.md` line 212, "JS/TS relative specifiers share one candidate list", is too broad. The tsconfig-paths hop resolver, the qualifier rung's stem prefilter, `raw_relative_literal` and `namespace_sibling` do not use it (S2, S3). Suggested: "Member imports, re-exports and class-identity lookups share...".
- "`.d.ts` is never a substitution candidate" has one corner: `./x.d.js` substitutes to `x.d.ts` (`p6`). TypeScript's suffix replacement lands on the same file as far as I know (not run), and a real declaration file has no body (`h` in `p6` stays dropped), so no edge results.
- The T column mixes units. 61,712 is records; 15,341 and 15,244 are unique site keys (61,001 of them); 15,419 in the checker sentence is records. By record I count 15,419 changed: 15,322 dropped to Exact `import_member` plus 97 receiver-typed. The conclusion is unaffected.
- The stored X adjudication (`measure/X/i1b.adjudicated.txt`) reads 6 `agree` plus 1 `checker_names_a_signature_without_body`, not 7 `agree`; it predates the adjudicator change visible in the T files. I checked all 7 X edges against the pinned source and all are the declaration the import names.

### S6. SMELL, IMMATERIAL. Two substitution tables

- `namespace_sibling` (`src/resolution_js_namespace.rs:62-85`) keeps its own table. Its `.js` arm can no longer find anything the shared function does not.
- Its `.jsx` arm makes `import * as N from "./x.jsx"` Exact while `import { f } from "./x.jsx"` drops, on main and branch (`p19`).
- Fix when `.jsx` is modeled: move the table into the shared function and delete `namespace_sibling`. Not for this slice.

## Answers to the eight questions

1. **Additivity.** It holds for the candidate list, by construction: a non-empty literal list is returned untouched. It does not hold for edges, in three consumers: imported props (W2), namespace qualifier (W3) and star barrels (S1). The exactly-one-candidate consumers are additive: zero to one accepts, zero to two refuses (`p10`), and a miss in the receiver-type rung still falls through as on main (`p8`). Routing `.` and `..` to the relative lane removes nothing, because the paths lane never stores a specifier starting with `.` (`src/js_paths.rs:328`).
2. **Precedence.** Keep literal-first. With `x.js` and `x.ts` both indexed, `./x.js` gives `x.js` on main and branch (`p14`), and main's extensionless `./x` also picks `x.js`. TypeScript's order would move an existing Exact edge from `x.js` to `x.ts` and make the two spellings disagree. With `x.js` and `x.d.ts`, TypeScript's order would select a file with no bodies and lose the edge. What literal-first loses: when compiled output sits beside its source, edges land in the output. If the project wants TypeScript's order, change both spellings in one measured slice.
3. **`.d.ts`, `.jsx`, `.mjs`, `.cjs`.** Right to leave out. `.d.ts` has no callable body. `.mts` and `.cts` are not indexed languages (`src/languages/mod.rs:27-29`), so `.mjs` and `.cjs` have nothing to find. `.jsx` to `.tsx` is a real recall gap with an existing asymmetry (S6); no wrong or lost edge.
4. **New wrong edges.** Two: W3 and W4. A whitespace-padded specifier now resolves (`p15`), inherited from main's `trim()`. Sound: root escapes, a directory and file sharing a stem, `x.js.ts`, a directory named `x.js`, case, dotfiles, trailing slashes, `..` chains, mixed normalization and substitution, query strings and fragments (`p13`, `p14`, `p15`, `p18`, `p20`).
5. **Negative controls.** Missing export, both type-only import forms, and a same-named function in an unrelated file produce no edge for member imports through `.js`, `.` and redundant segments (`p7`). The exception is the namespace form in W3.
6. **Other relative-specifier logic.** Safe to leave unchanged; every inconsistent pair I found fails closed (S2, S3). Both need a follow-up for recall.
7. **Tests.** See S4: three non-equivalent mutants survive, and two consumer paths are untested through the new spellings.
8. **Persisted state and docs.** Both version bumps are needed and present. The nav sidecar is validated only against its own version, package version, grammar and skip policy (`call_edge_cache.rs:250-254`), so its bump is not redundant. Pin tests pass. The mutation registries were missed (W1). The `CLAUDE.md` sentence is too broad (S5).

## Before merge

1. **Required:** update the three registry anchors (W1) and run the gate for them.
2. **Decision to record:** the edge-level contract. My recommendation is that the new spellings behave as the canonical spelling does, which includes W2, W3 and S1: two sentences in the PR description, the `CLAUDE.md` wording from S5, and a twin test for each. If the owner wants strict edge-level additivity instead, W2 and W3 each have an alternative with its cost.

Everything else is recorded and does not block.

## Checked and found sound

- Candidate-level additivity, and the gate change in `resolve_js_ts_member_module`.
- The three exactly-one-candidate consumers.
- The member-import rung: a found module without the export drops on both binaries.
- Literal-first precedence and the `.ts`-then-`.tsx` order. That order matches TypeScript's for a `.js` spelling as I know its resolver; I did not run `tsc`.
- The path shapes in question 4, on both binaries.
- My own recount of the stored X and T site dumps (`opus-scratch/bin/sitediff.py`): no record added or removed, no existing Exact target lost, no resolved-to-dropped; every change ends Exact. All 7 X edges hand-checked against the pinned source.
- The PR's tests in the worktree: 29 unit and 20 integration pass, both version pin tests pass, `cargo fmt --check` is clean.
- CI on head `aa74eae0`: all checks succeeded, including the four test-suite shards, so the full suite has run on the final commit.

## Not checked

- The private corpus F (not opened); the 1,011-fixture matrix and its 17 rows (separate PR).
- The TypeScript-checker adjudication of T (15,419 edges): I recounted the site diff but did not rerun or sample the checker comparison.
- The full test suite and clippy locally (read from CI only); `tier-a --quick` (not reported by the PR; the changed functions are reached only from JS/TS paths).
- The full mutation gate (only `--plan-only`); integration tests against mutants.
- The `executable_owner` lane beyond reading `map_member`: it is additive by the same cardinality argument, not exercised.
- Build time and memory of the branch binary; no timing was recorded for it.
- Symlinked directories, case-insensitive filesystems, and incremental partial-hit cache builds with the new spellings.
- The planning document for this slice (not in the worktree); I judged the two deliberate differences from the brief's description.

**Verdict: FIX** (W1 required; record the edge-level contract for W2, W3 and S1).

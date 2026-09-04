# Handoff — Python imported typed-receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex  
**Workspace:** `/private/tmp/slicing-py-imported-receiver` · `py-imported-receiver-owner`  
**Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9`  
**RED/design checkpoint:** `0fe4e46`  
**Focused-green implementation checkpoint:** `d896430`
**Full-gate handoff checkpoint:** `476631d`
**Parked incremental RED checkpoint:** `5d233e8`

## 0. Current verdict

**PARKED at the declared two-round review cap; do not open an MR.** The full-build implementation is focused-green, but review round 2 proved an incremental-parity WRONG for an unchanged caller when the imported class proof changes. Current HEAD intentionally contains one failing regression. A bounded proof-set mismatch successor is designed but requires explicit owner authorization.

The steering carrier names an adjacent installed `handoff-template.md`, but no template is present in this checkout. This handoff follows the established lane shape.

## 1. Scope and authority

The Exact authority chain is:

1. one eligible structured member import for the local type spelling;
2. one indexed Python file matching the import module before member filtering;
3. one occurrence-clean module-scope class matching the original imported member;
4. one non-ambiguous direct method in that exact class span.

Classifier and resolver share `python_imported_class_route`. Full and direct-subset construction build structured imports, indexed-file identity, and clean class facts before call-site classification. Only already-proven imported type names alter persisted receiver recovery. A query-time recheck guards incremental/cached graph state.

Out of scope: `module.Class`, function/class-local imports as authority, wildcard/duplicate/rebound imports, imported inheritance, re-exports, JS/TS, and a general Python scope graph. A failed proof remains materialized residue and never enters the global bare-owner Exact fallback.

## 2. RED/GREEN evidence

- Exact-base RED at `c220525c`: focused target selected 24 tests; `23 passed, 1 failed`. The sole failure was the new positive, `receiver_type None` instead of `ImportedClient`. Ambiguity, external collision, function-local import, and inherited-only controls passed.
- First GREEN exposed an existing-invariant regression: external imported `Bar` carried `Some("Bar")`. Classification was narrowed to the derived proven-class set rather than weakening the assertion.
- Review-round RED: a proven module import shadowed by function-local `from other import Client` incorrectly carried `Some("Client")`. The same focused test failed `0/1` at the new assertion. The existing whole-function binding census now blocks imported constructor/local-annotation recovery for that state.
- Final focused result: `25 passed, 0 failed`.
- Complete Python target: `71 passed, 0 failed`.
- Import-binding integration selection: `56 passed, 0 failed`; two pre-existing warnings.
- `cargo check --all-targets`: pass; pre-existing test warnings only.
- Full default suite at `476631d`: `3,547 passed, 0 failed, 1 ignored` across 28 result summaries.
- Full `mcp` suite at `476631d`: `3,733 passed, 0 failed, 1 ignored` across 30 result summaries.
- `cargo fmt --all -- --check`, `git diff --check`, and the configured Clippy command passed. Clippy emitted 243 repository warnings; none identify the new helper/classifier logic.
- Tier-A matrix-only after an immediate release rebuild: all 104 rows `ok`.
- Tier-A quick after an immediate release rebuild: oracle error `0.0`, SUT error `0.0`, all 104 matrix rows `ok`; command exit 2 solely because the generated corpus/SUT SHA `476631d3d78b` differed from pinned `20c8490591a3`. This is a baseline-validity refusal, not a behavioral regression, and no baseline was updated.
- Parked round-2 RED at `5d233e8`: `0 passed, 1 failed`; incremental `receiver_type` was `None` while a fresh build returned `Some("Client")` and one Exact `TypedParam` target.

Invalid probes discarded without belief updates: one orchestration quoting failure, one Cargo command using nonexistent target `python`, and the first sandboxed Tier-A invocation that could not access the `uv` cache.

## 3. Review and convergence

Declared cap: two rounds.

- Round 1: one confirmed WRONG (function-local constructor import could be rebound to the module-level imported class); bounded RED added and fixed. One provisional cache WRONG was refuted and withdrawn: CPG and navigation caches include the source-content `cache_build_identity`, so changed `src` bytes force invalidation without a format-version bump.
- Round 2: one confirmed WRONG. With unchanged `app.py`, changing only `pkg/models.py` from no clean `Client` to clean `Client.send` makes a fresh build recover `Some("Client")` and Exact resolution, while incremental construction retains `None`. The existing path rebuilds changed-file facts and selected whole-program indexes but does not reclassify unchanged Python call sites after imported-class proof changes.
- Cap classification: two rounds produced two new proof-lifecycle defects rather than a declining closed list. The review loop is not converged, so the slice is parked. The successor design is `docs/superpowers/specs/2026-09-04-python-imported-receiver-incremental-parity-PARKED.md`.

## 4. Remaining work

1. Obtain explicit owner authorization for one bounded successor round; do not push or open an MR from the parked state.
2. On this existing branch/artifact, add the inverse proof-present-to-absent RED and implement the specified old/new imported-proof-set mismatch guard with full-build fallback.
3. Rerun focused parity tests, the full default and `mcp` suites, formatting, Clippy, release build, Tier-A matrix-only, and Tier-A quick. Do not rebaseline corpus drift.
4. Run one explicitly declared review round limited to incremental proof-set parity. If it is clean and gates are acceptable, refresh custody before seeking publication authorization.

## 5. Hypothesis-probe-result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| Existing import metadata can prove the narrow owner without a new scope graph. | Eligible member import + module cardinality + clean class span identify one owner file/class. | Import facts retain only text and cannot distinguish defining modules. | Supported; the structured binding carries local/module/original member and existing indexes prove the rest. |
| Relaxing the flat import guard alone is safe. | External/function-local imported types remain suppressed downstream. | Any such type carries recovered state or enters bare-owner Exact. | Falsified; external `Bar` changed persisted state, so recovery is now limited to pre-proven imported classes. |
| Function-local imports cannot perturb a proven module import. | Constructor/local annotation still names the module-level class. | A local import binding of the same type name changes constructor ownership. | Falsified by RED; whole-function local binding census now blocks that constructor/local-annotation recovery. |
| Cache versions must bump for the semantic change. | Unchanged version can serve stale call sites/edges after binary upgrade. | Cache fingerprint includes changed binary-input contents. | Falsified; both caches compare `cache_build_identity`, which hashes `src` inputs. |
| Incremental construction reclassifies unchanged Python callers after imported-class proof changes. | Fresh and incremental builds agree after only the defining module gains a clean imported class. | A whole-program rematerialization or rebuild guard restores parity. | Falsified by admissible RED: fresh returned `Some("Client")`, incremental retained `None`; source inspection confirmed no Python rematerialization or proof-set mismatch guard. |

## 6. Custody

- Branch/worktree: `py-imported-receiver-owner` at `/private/tmp/slicing-py-imported-receiver`.
- Durable commits: `0fe4e46` (design + intentional RED), `d896430` (focused-green implementation), `476631d` (full-gate handoff checkpoint), and `5d233e8` (parked incremental RED + bounded successor design).
- Root checkout remains `/Users/wesleyjinks/code/slicing` on `main` at `c220525c`; its pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` were not touched.
- Generated Tier-A reports/run/snapshot from the baseline-invalid quick run were removed after their verdict was recorded; they were not committed or used to rebaseline.
- No baseline, remote branch, MR, or external system has been changed.

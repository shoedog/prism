# Handoff — Python imported typed-receiver ownership

**Refreshed:** 2026-09-04 · **By:** Codex `/root` · **Provider:** codex  
**Workspace:** `/private/tmp/slicing-py-imported-receiver` · `py-imported-receiver-owner`  
**Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9`  
**RED/design checkpoint:** `0fe4e46`  
**Focused-green implementation checkpoint:** `d896430`

## 0. Current verdict

**IMPLEMENTED; full acceptance and publication remain open.** The bounded slice resolves Python receivers typed by an eligible module-scope `from module import Class [as Alias]` to one exact in-repo direct method only when module-file cardinality and clean class identity are both unique.

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

Invalid probes discarded without belief updates: one orchestration quoting failure and one Cargo command using nonexistent target `python`.

## 3. Review and convergence

Declared cap: two rounds.

- Round 1: one confirmed WRONG (function-local constructor import could be rebound to the module-level imported class); bounded RED added and fixed. One provisional cache WRONG was refuted and withdrawn: CPG and navigation caches include the source-content `cache_build_identity`, so changed `src` bytes force invalidation without a format-version bump.
- Round 2: pending after full gates and final diff review.

## 4. Remaining work

1. Run `cargo fmt --all -- --check` and the configured Clippy gate.
2. Run the project’s full test suite and report totals without rebaselining unrelated failures.
3. Run `cargo build --release`, then Tier-A matrix-only and Tier-A quick with `--allow-stale-sut` only against that immediately rebuilt binary.
4. Perform review round 2. Classify findings as WRONG or SMELL; park if the same proof-completeness class recurs at the cap.
5. Refresh this handoff with final evidence, commit explicit paths, then push/open the MR only after gates are acceptable.

## 5. Hypothesis-probe-result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| Existing import metadata can prove the narrow owner without a new scope graph. | Eligible member import + module cardinality + clean class span identify one owner file/class. | Import facts retain only text and cannot distinguish defining modules. | Supported; the structured binding carries local/module/original member and existing indexes prove the rest. |
| Relaxing the flat import guard alone is safe. | External/function-local imported types remain suppressed downstream. | Any such type carries recovered state or enters bare-owner Exact. | Falsified; external `Bar` changed persisted state, so recovery is now limited to pre-proven imported classes. |
| Function-local imports cannot perturb a proven module import. | Constructor/local annotation still names the module-level class. | A local import binding of the same type name changes constructor ownership. | Falsified by RED; whole-function local binding census now blocks that constructor/local-annotation recovery. |
| Cache versions must bump for the semantic change. | Unchanged version can serve stale call sites/edges after binary upgrade. | Cache fingerprint includes changed binary-input contents. | Falsified; both caches compare `cache_build_identity`, which hashes `src` inputs. |

## 6. Custody

- Branch/worktree: `py-imported-receiver-owner` at `/private/tmp/slicing-py-imported-receiver`.
- Durable commits: `0fe4e46` (design + intentional RED) and `d896430` (focused-green implementation).
- Root checkout remains `/Users/wesleyjinks/code/slicing` on `main` at `c220525c`; its pre-existing untracked `.superpowers/` and `eval/snapshots/prism-fb81481dafa7.json` were not touched.
- No baseline, generated eval report, remote branch, MR, or external system has been changed yet.

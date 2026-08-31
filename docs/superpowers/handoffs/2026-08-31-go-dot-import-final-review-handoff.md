# Handoff — #4b Go dot-import final redesign review

**Recorded:** 2026-08-31T03:14:44Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-4b-go-dot-import-v7` · `p4b-go-dot-import-v7`
**Exact base:** `[MEASURED]` `e96d9c5011e2f979dbb1d4c6eeaa5c29629c5c5e` (`origin/main`, PR #218 merge)

## 0. Verdict and authority

Roadmap #4b is **PARKED at the declared final redesign-review cap**. Do not implement v6 and do not describe it as settled. Fresh-cap round 1 had two `WRONG`s. Final round 2 found two new constructible `WRONG`s in the repeated namespace/poison completeness class, so the loop is not converging and cannot be silently extended. The authoritative details are in `docs/superpowers/specs/2026-08-23-go-dot-import-redesign.md` §7; the roadmap row is reconciled to this state.

The retained measured value is historical: four zap `observer.New` Tier-A sites were adjudicated `prism_fn`. This review did not rerun zap, corpora, the oracle, Tier-A, or the full suite and makes no new executable-result claim.

The steering carrier refers to an installed `handoff-template.md`, but a filesystem search found no such file on this machine. This handoff follows the established adjacent lane shape instead; that absence is not evidence about the design.

## 1. Rebound compiled reality

| Item | Current fact |
|---|---|
| Base | `origin/main` = `e96d9c5011e2f979dbb1d4c6eeaa5c29629c5c5e` after #16 and #18 closeout |
| Unqualified Go ladder | `resolution.rs:3276-3389`; local, same-package, then R5 `FreeSingle`/`FreeMulti` |
| Dot-import state | `ParsedFile::go_has_dot_import` + serialized `CallGraph::go_dot_import_files`; no path map or call-resolution rung |
| Module identity | exact `@go-import:<path>` keys on `go_package_basenames`; paths derive from manifest topology/directory, not package-clause parse quality |
| Go profiles | missing package clause becomes `String::new()` |
| Existing binding helpers | conservative whole-function value set; receiver-specific occurrence walker; neither is the specified complete bare-callee proof |
| Build purpose | no shared review-vs-navigation mode in CPG/CallGraph builders; review may build full, scoped, or scoped-to-full |
| Cache versions | CPG `54`; navigation sidecar `23` |

The stale standalone clone `/Users/wesleyjinks/code/slicing-p4b-go-dot-import` remains historical custody at `main` `36b2796`; it was not modified or used as current authority.

## 2. Final findings

### WRONG 1 — function-only import and partial shadow census can mint the wrong Exact

Concrete state A: package `q` exports `type New int`; the caller dot-imports `q` and calls `New(x)` as a conversion; another repository package has the only free function named `New`. V6 says the name is absent from dot-imported packages because it enumerates only free functions, then preserves R5. R5 emits an Exact edge to the unrelated function.

Concrete state B: a caller dot-imports a package exporting `func New`, then declares `func f() (New func()) { New() }`. The named result variable is the call target. The cited receiver occurrence pattern checks input parameters and selected locals but not named results; a literal implementation records no shadow and emits the imported function Exact. Select receives (`case New := <-ch: New()`), method receivers, local const/type declarations, ordinary import names, and function-literal results are the same finite scope obligation.

Bounded fix for a future fresh design: build an all-exported-declaration namespace index with declaration kinds; terminally drop on any non-function/collision; add one occurrence-aware scope classifier covering every Go binding form and parse-recovery unknown; mint only one proven exported function with no competing declaration of any kind. Require red fixtures per form and before/after/sibling-scope controls.

Confidence increases with the Go specification's file-block and single-namespace rules plus the current R5/source walker census. It decreases if an earlier production gate type-resolves and terminally filters conversions/func-value calls before the ladder. It collapses if those shapes are proven unreachable at every resolver and manifest consumer.

### WRONG 2 — empty package clauses do not poison the candidate directory

Concrete state: an in-repo imported directory contains one ordinary `package q` file exporting `New` and another ordinary parsed Go file whose package clause is missing/recovered as empty. The module graph still proves both files' import paths from directory topology. V6 discards empty clauses, observes exactly `{q}`, and may emit `q.New` Exact from an incompletely proven directory namespace.

Bounded fix for a future fresh design: require every relevant non-test Go file to have a nonempty package clause and exact-admissible parse/build profile; empty clause, parse recovery, or build uncertainty poisons the directory. Pin the mixed valid-plus-empty-clause fixture red-first.

Confidence increases with the directly read extraction and module-identity implementations. It decreases if a separate candidate gate rejects all empty profiles before clause qualification. It collapses if module identity makes the entire directory unproven whenever any ordinary file lacks a package clause.

### SMELLs

- Consumer purpose must be explicit across full/scoped/fallback/incremental/cache/navigation/MCP paths; `ctx.scope` cannot encode review-disabled semantics.
- Replace or canonically extend `go_dot_import_files`; do not add an independent mutable map.
- Re-cut exact base, source locations, cache transitions, controls, and corpus/oracle baselines in any future lane.

## 3. Hypothesis–probe–result log

| Hypothesis | Expected if true | Falsifier / alternative | Result |
|---|---|---|---|
| H1: the cited binding machinery is a complete per-call Go shadow oracle. | It covers result parameters and communication-clause receive declarations with lexical scope. | The whole-function helper over-suppresses scope while the receiver walker omits those forms. | **Falsified.** Named results and select receives are not covered by the cited occurrence pattern. |
| H2: “no dot-import function candidate” proves the imported name is absent. | Dot imports expose functions only, or a prior gate rejects non-function names. | Go imports all exported package-block declarations and R5 still sees a same-named free function elsewhere. | **Falsified.** A type conversion can be resolved Exact to an unrelated function. |
| H3: module proof rejects an ordinary file with an empty package clause. | `resolve_files` consults `GoBuildProfile` or parse quality. | It derives import path solely from module boundary and directory. | **Falsified.** Empty-clause files remain path-proven. |
| H4: review-disabled behavior can be inferred from scoped graph state. | Every review build is scoped and never falls back to full. | Default review is full and scoped builds can return a full build. | **Falsified.** A purpose flag is required. |

Alternative mechanisms considered: terminal type/conversion filtering before the ladder (not present), module-graph package-clause validation (not present), and one existing complete lexical helper (the available helpers split conservatism and occurrence scope; neither satisfies both).

## 4. Verification and exclusions

This is a docs-only review disposition. Source, spec, roadmap, Go language authority, current branch/base, and cache constants were read. No source or test behavior changed. `cargo fmt --all -- --check`, `cargo check`, and `git diff --check` each exited `0`; the explicit trailing-whitespace scan found no matches. A full suite, Tier-A, corpora, oracle, and cache battery are excluded because there is no implementation claim. Any failed or zero-selected probe remains inadmissible.

## 5. Next queue

Proceed to roadmap #13, whose Go-first typed-`func` callback population measurement gate was previously met, but rebind its exact current design, consumers, and acceptance evidence before authorizing implementation. #4b may resume only through an explicit owner decision commissioning a fresh design cap from §2's bounded prerequisites—not by folding v7 in this exhausted review loop.

## 6. Identifiers

| Item | Value |
|---|---|
| Worktree | `/Users/wesleyjinks/code/slicing-4b-go-dot-import-v7` |
| Branch | `p4b-go-dot-import-v7` |
| Exact base | `e96d9c5011e2f979dbb1d4c6eeaa5c29629c5c5e` |
| Spec | `docs/superpowers/specs/2026-08-23-go-dot-import-redesign.md` §7 |
| Review cap | `2`; exhausted, non-converging |
| Runtime changes | none |

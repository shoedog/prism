# Handoff — #13a Go Level-3 bare-function callback design

**Recorded:** 2026-08-31 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-13a-go-level3-design` · `p13a-go-level3-design`
**Exact base:** `[MEASURED]` `a3768a9d40903c32251346d196107d48af25eb47` (`origin/main`, PR #219 merge)

## 0. Verdict and authority

Roadmap #13a's design is **SETTLED for a bounded implementation**. The authority is `docs/superpowers/specs/2026-08-31-go-level3-bare-function-callback-design.md`. It authorizes only Go B1 callback values: a bare identifier proven to name one in-repo non-test free function, passed to a proven in-repo free-function HOF at a callable parameter slot. Implementation remains gated on watched REDs, same-environment base controls, the full suite, Tier-A, five-corpus reconciliation, and all cache/navigation lifecycle paths.

The steering carrier names an installed `handoff-template.md`, but a filesystem search found no such file on this machine. This handoff follows the established adjacent lane shape. The absence of the template is not evidence about the design.

## 1. Measured value and custody

Retained packet: `/Users/wesleyjinks/code/prism-lane-artifacts/2026-08-23-next10/13-hof-sweep/`

| Artifact | SHA-256 |
|---|---|
| `hof-sweep-results.json` | `24d8607567a8b0c8d28cd39d675b19593bb356217668b3bdbdde3f091d64e05c` |
| `hof-sweep-samples.md` | `2abfa54f82e53d156b25362c4591d2d611bc585df8f6a3ee603f934d9277328d` |
| `hof-sweep-slots.tsv` | `748af6e2da085bf4c58f3ca1b62f4ed2675cc1618119a478aab3a184b9a7f665` |
| `hof-sweep.go` | `c7f975a9c298956429c44e8e96f1f19605ef0059b5969accc31e4029b4606416` |

The five corpora reported zero parse errors and zero files skipped for size. The strict unambiguous non-test B1 floor is Caddy `49`, Prometheus `6`, etcd `3`, Hugo `9`, zap `0`: total `67`. This is a syntactic census, not a type-checking/build-tag/shadow oracle. Acceptance is a 67-row site-level classification, not a requirement to emit 67 edges. Caddy's two primary positive controls use named function types, so literal-`func`-only extraction is insufficient.

## 2. Rebound compiled substrate

| Item | Current fact at exact base |
|---|---|
| Level 3 producer | disabled; returns zero sites and zero count in `compute_indirect_call_sites` |
| Exact target carrier | serde-defaulted `CallSite::pre_resolved_target`; included in ordering and navigation key |
| Exact callback resolution | a live pre-resolved target yields one `Exact` `ParameterCallback` outcome |
| Argument identity | indexed by exact `(start_byte, source callee text)` with positional spans |
| Parameter slots | prefix-preserving Go grouped-name semantics; variadic/unrepresentable tails stop |
| Callable substrate | import-aware `GoTypeProvider`; boolean named-function-type proof exists but no retained canonical callable signature |
| Bare value substrate | same-package free-function resolver exists, but does not prove the identifier occurrence against every Go namespace binding |
| Package poison | canonical `go_dot_import_files` exists; clean-package/empty-clause/parse/profile proof must be added |
| Cache versions | CPG `54`; navigation resolved-edge sidecar `23` |

Prism structural navigation was not used as authority because its available repository snapshot was stale. The LSP navigation capability was unavailable. Current source, exact checkout state, retained artifacts, and compiled data structures were read directly.

## 3. Settled implementation boundary

The design requires:

- two-phase import-aware callable-signature extraction for free functions and literal/in-repo named function types;
- callback-strict signature identity, never the existing permissive bare-name equality;
- one exhaustive, conservative occurrence proof for callback argument identifiers and unqualified HOF callees;
- exact in-repo import identity for qualified HOF calls and default import names;
- reusable clean-package/profile proof across caller, HOF, target, and named callable types;
- HOF-local direct callback invocation with no nested callable, rebinding, shadow, or address escape;
- synthetic sites carrying exact target identity plus a separate source-callee name;
- one effective-source-name helper used by both Step-5b lookups and all three reasoning source-expression roles;
- stable conservation telemetry and a site dump joinable to all 67 measured rows; and
- CPG `54 -> 55` plus navigation sidecar `23 -> 24`, with full, round-trip, incremental, and sidecar parity.

Excluded: B2 qualified/method values; method or interface HOFs; B3 locals/package vars; B4 literals; B5 returns/fields/composites; assignments; variadic or generic callback slots; external named types; tests; nested-literal callback invocations; JS/TS and every other language.

## 4. Review and convergence record

Declared design-review cap: `2`. One Round-3 extension was disclosed under the convergence rule after findings decreased from three to one distinct, smaller downstream class.

| Round | Result |
|---|---|
| 1 | **FIX — 3 WRONG.** Unknown default-import package names could be guessed from path text; a local HOF callee could shadow the package HOF; mutation through `&cb` could invalidate a later direct callback invocation. Folded strict import proof, HOF occurrence proof, reusable package poison, free-HOF-only scope, and address-escape rejection. |
| 2 | **FIX — 1 WRONG.** A synthetic `cb(x)` site whose `callee_name` is the resolved target could resolve its Call edge but lose Step-5b/reasoning arguments keyed by source text. Folded separate serde-defaulted source-callee identity and one helper. |
| 3 | **SETTLED.** An exhaustive `site.callee_name` consumer census completed the same Round-2 population: two Step-5b lookups and three reasoning source-expression roles. CPG Call/Return attachment and navigation target topology separately use the resolved `FunctionId` and retain target identity. No open `WRONG` remains in the controller audit. No independent implementation review has occurred. |

Confidence increases if the red-first fixtures prove every enumerated namespace and source-identity case and the 67-row join conserves exactly. It decreases if compiled reality requires an unenumerated Go binding form or a second signature authority. It collapses if any accepted synthetic site lacks a uniquely proven target/signature/occurrence or any cache path retains a stale edge.

## 5. Verification and exclusions

The design PR was documentation-only: design, roadmap, and handoff. No runtime behavior, cache schema, CLI grammar, or test fixture changed. Source consumers, cache constants, branch/base, and retained packet hashes were rebound. `cargo fmt --all -- --check`, `cargo check`, staged `git diff --check`, and the explicit trailing-whitespace scan passed. The four retained packet files were rehashed and matched §1. PR #220 passed Format Check, Clippy Lint, Test Suite including MCP tests, and Language Coverage Matrix, then squash-merged at `8c40bfc344b263e06fd21419dc133a4aed92ae98`. Coverage was pending and intentionally not awaited by owner direction. The merge command's nonzero exit was local cleanup only: a direct server query proved `state=MERGED` and returned that exact merge SHA.

Not run for this design artifact: full tests, Tier-A, corpora, oracle, cache battery, or release build. Those are implementation acceptance gates and no executable change is claimed. A local Go compiler probe for the default-import collision needed an explicit writable `GOCACHE`; the initial cache-refused probe was inadmissible, while the corrected probe confirmed the same-file collision is invalid Go. Prism's parse-clean edit-state operating model still requires fail-closed static import proof.

## 6. Next exact steps

Completed: the docs-only gate merged as PR #220, and implementation plus detached control worktrees were cut at its exact merge SHA.

1. Add callable signature/index tests with production minting disabled.
2. Add exhaustive occurrence, namespace, profile, mutation, and address-escape proof tests.
3. Add watched positive Level-3 tests; compile them and retain RED on exact base before enabling production minting.
4. Implement B1 minting, source-callee consumption, telemetry/dump, cache bumps, and lifecycle parity in the spec's sequence.
5. Run focused checks, full suite with totals, immediate release build plus Tier-A matrix/quick, and five-corpus same-base reconciliation of all 67 rows.
6. Run a bounded independent implementation review before publication; classify every finding `WRONG` or `SMELL` and use same-environment controls before attribution.

## 7. Identifiers

| Item | Value |
|---|---|
| Design worktree | `/Users/wesleyjinks/code/slicing-13a-go-level3-design` |
| Design branch | `p13a-go-level3-design` |
| Design base | `a3768a9d40903c32251346d196107d48af25eb47` |
| Design publication | PR #220 · merge `8c40bfc344b263e06fd21419dc133a4aed92ae98` |
| Implementation worktree | `/Users/wesleyjinks/code/slicing-13a-go-level3-impl` |
| Implementation branch | `p13a-go-level3-impl` |
| Detached base control | `/private/tmp/slicing-p13a-base-8c40bfc` |
| Implementation exact base | `8c40bfc344b263e06fd21419dc133a4aed92ae98` |
| Design | `docs/superpowers/specs/2026-08-31-go-level3-bare-function-callback-design.md` |
| Roadmap | `docs/analysis/prism-post-plan-roadmap.md` row `13` |
| Review cap | `2 + 1 disclosed convergence extension`; settled |
| Runtime changes | none |

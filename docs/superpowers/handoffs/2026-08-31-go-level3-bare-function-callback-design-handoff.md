# Handoff — #13a Go Level-3 bare-function callback implementation

**Refreshed:** 2026-08-31 · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/private/tmp/slicing-p13a-impl-8c40bfc` · `p13a-go-level3-impl`
**Exact base:** `[MEASURED]` `8c40bfc344b263e06fd21419dc133a4aed92ae98` (PR #220 merge)

## 0. Verdict and authority

Roadmap #13a's bounded Go B1 implementation is **CLOSED AND MERGED** in PR #221 at `492160590ca76388d4385e2bc2099fb84294f791`. The authority remains `docs/superpowers/specs/2026-08-31-go-level3-bare-function-callback-design.md`: only a bare identifier proven to name one in-repo non-test free function, passed to a proven in-repo free-function HOF at a strict callable parameter slot, may mint an edge.

Post-merge custody was rebound against GitHub's live PR and commit state. PR #221 is merged; its squash commit has exact base `8c40bfc344b263e06fd21419dc133a4aed92ae98` as parent, and its tree `e45e6f73cd07f663d39993adf5bfe63fb1a6f19a` is byte-identical to local implementation head `e0ebca86b0606f2e1f63c474d6c20d0f1be49162`. The managed executor refused `git fetch` before execution, so no local branch is claimed to contain the squash commit. Format, Clippy, Test Suite, and Language Coverage Matrix were green before merge; coverage was not awaited by owner direction.

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

The successor real-corpus screen is now **CLOSED**. Thirteen no-cache production-path runs conserved `981 = 3 accepted + 978 drops`. Two accepted Delve rows were debugger fixtures and were not used to close the real-source signal. Kubernetes module `k8s.io/pod-security-admission` supplied one non-fixture accepted edge from `addCheck(CheckProcMountRestricted)` to the direct `f()` invocation and exact target; `go test ./policy` passed. See `2026-08-31-go-level3-b1-real-corpus-measurement.md`.

## 2. Implemented compiled substrate

| Item | Current implementation fact |
|---|---|
| Level 3 producer | `compute_indirect_call_sites` performs a telemetry-only name join, proves each occurrence, and mints only accepted Go B1 sites |
| Exact target carrier | serde-defaulted `CallSite::pre_resolved_target`; included in ordering, navigation key, cache, and resolution |
| Source identity | serde-defaulted `source_callee_name`; one helper feeds both Step-5b lookups and reasoning's three source-expression roles |
| Callable signatures | strict import-path-aware free-function and callback-parameter indices; in-repo named types/aliases are unique and cycle-safe |
| Occurrence proof | exact AST identifier span plus whole-function binding/mutation census and package/file collision proof |
| HOF proof | free-function-only, exact import identity, singleton `Exact` inbound resolution, direct non-nested parameter invocation, no shadow/mutation/address escape |
| Telemetry | aggregate conservation plus stable per-candidate accepted/drop records with inbound, HOF, target, slot, parameter, and invocation spans |
| Cache versions | CPG `55`; navigation resolved-edge sidecar `24` |

Prism structural navigation was not used as authority because its available repository snapshot was stale. The LSP navigation capability was unavailable. Current source, exact checkout state, retained artifacts, and compiled data structures were read directly.

## 3. Preserved implementation boundary

The implementation provides:

- two-phase import-aware callable-signature extraction for free functions and literal/in-repo named function types;
- callback-strict signature identity, never the existing permissive bare-name equality;
- one exhaustive, conservative occurrence proof for callback argument identifiers and unqualified HOF callees;
- exact in-repo import identity for qualified HOF calls and default import names;
- reusable clean-package/profile proof across caller, HOF, target, and named callable types;
- HOF-local direct callback invocation with no nested callable, rebinding, shadow, or address escape;
- synthetic sites carrying exact target identity plus a separate source-callee name;
- one effective-source-name helper used by both Step-5b lookups and all three reasoning source-expression roles;
- stable conservation telemetry and a site dump joinable to every production candidate, with named manual reconciliation for pre-candidate exclusions; and
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

Declared implementation self-review cap: `3`. Rounds 1–2 found no `WRONG`; Round 2 rechecked whole-program lifecycle placement and confirmed that the complete branch Clippy log contains no diagnostic for the new callback module or minting block, while the expanded `CallSite::cmp_key` complexity lint is already present on exact base. At implementation close, one `SMELL` remained: the five retained corpora contained no row accepted by the full authority. The successor Kubernetes measurement resolved that evidence gap with one non-fixture row that survived every proof and compiled under the module's own package test. This does not authorize B2–B5 or weaken any B1 authority. No independent implementation review has occurred.

## 5. Verification and exclusions

Watched positives compiled and failed on the implementation branch before minting (`0/2`, exit `101`) because the exact edges were absent. The same self-contained tests failed for the same reason in `/private/tmp/slicing-p13a-base-8c40bfc`; negative fixtures remained fail-closed. After implementation, signature/proof units passed `12/12`, consolidated Level-3 graph/lifecycle/consumer tests passed `22/22`, `cargo check --all-targets --all-features` passed, `cargo fmt --all -- --check` passed, and `git diff --check` passed.

The full required suite completed with captured exit `0`: `3728` passed, `0` failed, `1` ignored across `29` test binaries. `cargo clippy --all-targets --all-features -- -D warnings` remains red with `129` library and `168` test errors; the exact base fails with the same counts and leading diagnostics under the same Rust `1.94.0` toolchain, so this is baseline debt rather than a branch regression.

An immediate branch `cargo build --release` passed. Tier-A matrix passed every fixture. Tier-A quick produced oracle/SUT error rates `0.000`, no matrix failure, and exit `2` solely because `16dd35b974fd != pinned 20c8490591a3`; the exact-base quick control likewise produced `0.000`/`0.000`, no matrix failure, and exit `2` solely because `8c40bfc344b2 != pinned 20c8490591a3`. Generated baseline documents and snapshots were removed without rebaselining; the branch run JSON remains under ignored `eval/runs/` custody.

Five-corpus branch/base `call-stats` runs all exited `0`. Caddy/Prometheus/etcd/Hugo/zap produced `2226` production candidates, `11` exact inbound sites, `0` accepted sites, and `0` Level-3 edges; every aggregate conserved. Removing only the six new telemetry keys leaves branch and exact-base stats byte-identical for all five corpora, proving no unrelated resolution-kind drift.

All `67` retained floor rows were adjudicated. The stable production dump directly classifies `57`: Caddy `45` strict-import-name unavailable plus `4` nested-only invocations; Prometheus `2` strict-import-name unavailable; Hugo `6` missing slot/argument identity. The remaining `10` are named pre-candidate exclusions: Prometheus `4` qualified/method HOF rows; etcd `1` method HOF row plus `2` callbacks with external named types; Hugo `1` method-HOF/local-alias row plus `2` qualified function arguments. These exclusions are outside the authority-defined prospective `(source span, known free-HOF callback slot)` key and therefore are not falsely counted as conserved production candidates.

## 6. Next exact steps

1. Preserve PR #221 / `492160590ca76388d4385e2bc2099fb84294f791`, the retained implementation/base evidence, and the successor measurement packet; do not restart this closed lane.
2. Preserve the owner-authorized full-corpus Tier-A adjudication and PR #223 repair; do not rebaseline the invalid Prism anchor.
3. Keep B2–B5 outside the shipped authority. A future expansion requires its own measured population, RED, design, and review cap.
4. Advance the ranked roadmap to return-flow taint.

## 7. Identifiers

| Item | Value |
|---|---|
| Design publication | PR #220 · merge `8c40bfc344b263e06fd21419dc133a4aed92ae98` |
| Implementation publication | PR #221 · merge `492160590ca76388d4385e2bc2099fb84294f791` |
| Implementation branch head | `e0ebca86b0606f2e1f63c474d6c20d0f1be49162` · tree `e45e6f73cd07f663d39993adf5bfe63fb1a6f19a` |
| Implementation worktree | `/private/tmp/slicing-p13a-impl-8c40bfc` |
| Implementation branch | `p13a-go-level3-impl` |
| Detached base control | `/private/tmp/slicing-p13a-base-8c40bfc` |
| Implementation exact base | `8c40bfc344b263e06fd21419dc133a4aed92ae98` |
| Design | `docs/superpowers/specs/2026-08-31-go-level3-bare-function-callback-design.md` |
| Roadmap | `docs/analysis/prism-post-plan-roadmap.md` row `13` |
| Successor measurement | `docs/superpowers/handoffs/2026-08-31-go-level3-b1-real-corpus-measurement.md` |
| Design review cap | `2 + 1 disclosed convergence extension`; settled |
| Implementation self-review cap | `3`; Rounds 1–2 complete, no `WRONG`; corpus-positive `SMELL` resolved by successor measurement |
| Runtime changes | bounded Go B1 Level-3 minting plus telemetry/source identity/cache lifecycle |

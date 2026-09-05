# Rust recall repair and direct default-class receivers

Base: PR #241 merged at `18869079452d1ec7f92809875c5d35bc486bcbff`.
Authority: owner approved the next increment and explicitly the Rust recall repair.
Implementation and verification complete within the three-round cap. Published
as `e9e153e` in [PR #242](https://github.com/shoedog/prism/pull/242), now merged
at `854d53f` (verified by fetch/log in the indirect-default continuation).

## Result

- The real `src/main.rs:183` call to
  `prism::navigation::module_graph::module_deps` changes from UnknownName to an
  Exact `qualified_owner` edge to `src/navigation/module_graph.rs:196`.
  Cold Exact callers and callees both contain that same call-site line.
- On one fixed 88,716-site source universe, Exact edges change **20,966 → 21,038**:
  **72 additions, zero removed targets**, one changed evidence label. All changes
  are in `src/main.rs` and `src/bin/prism-mcp.rs`. The complete identity delta and
  bidirectional evidence are in `2026-09-04-rust-default-verification.json`.
- Direct named default-class declarations now support JS/TS/TSX default imports
  for constructor and typed-parameter receivers. TS default type-only imports
  support typed parameters only. Anonymous/indirect exports and reexports remain
  excluded; erased imports never acquire runtime constructor authority.
- Python source is unchanged. The prior fixed samples are byte-identical before
  and after: Black **400 sites / 79 Exact**, Excalidraw **136 / 26**, JavaScript
  **12 / 4**. This preserves #240's measured gains; it is not a new Python gain
  or a whole-corpus recall estimate.

The initially reported Rust removal was a comparison-label bug: the script counted
any changed existing record as removed. `aggregate_diagram_warnings` at main.rs:750
retains its exact target at slice.rs:394; its evidence kind improves from
`qualified_owner` to `constructor_local` once the imported `MultiSliceResult`
identity is available. The corrected comparison matches target identities before
counting losses. The local source constructs `MultiSliceResult` at main.rs:738.

## Root cause, architecture and review

Cargo's implicit binary-to-own-library relationship was absent from graph inputs.
Capture binary path -> (library crate name, exact library root) from the manifest,
then populate the existing consuming-root extern-prelude map. Do not strip path
prefixes or guess package identity from directory names. Custom library paths are
per package and do not rely on the legacy flattened `lib_path` field.

Negative tests exposed two bounded prerequisites: target-edition overrides must
not receive automatic bindings, and module-rooted path lookup must not bypass
lexical type bindings. The latter needed block type/trait/extern-crate dispatch
and opaque function/impl/trait generic barriers. Visibility and unsupported paths
remain conservative. CPG cache **62 → 63**, navigation sidecar **31 → 32**.

Review cap: three self-review rounds. Round 1 closed target-edition handling;
round 2 enumerated and closed lexical binding omissions; round 3 is the final
matrix, complete suites, cold/source-paired delta and persistence checks. This is
**SELF-PASS (NOT INDEPENDENT)**, not an independent review.

The old named-import negative matrix had one explicit direct-default exclusion.
It was replaced by an indirect-default negative, with new positive coverage for
the admitted direct shape. No unrelated test was re-baselined. Inadmissible
compile probes and RED/GREEN results are recorded in the implementation spec.

## Verification

| Check | Observed result |
|---|---|
| Full default `cargo test --no-fail-fast` | 3,740 passed, 0 failed, 1 ignored |
| Full MCP `cargo test --features mcp --no-fail-fast` | 3,930 passed, 0 failed, 1 ignored |
| Rust loader/navigation matrix | 4 tests pass, including custom paths/names, workspace isolation and lexical/visibility negatives |
| JS/TS/TSX default receivers | 2 matrix tests pass; cold/subset, type/value separation, conflicts and shadowing |
| CPG/incremental/nav persistence | Full suites cover default receiver transitions and own-library round trips; manifest-only rename removes/restores the Rust edge and invalidates cached topology |
| Format / diff checks | pass |
| Clippy all targets with MCP | completes with warnings; no `-D warnings` claim |
| Immediate rebuild + Tier-A matrix | 104 / 104 OK |
| Immediate rebuild + Tier-A quick | completes with exit 2: corpus SHA drift against pinned `20c8490591a3`; 104 matrix OK, quiescent oracle, zero oracle/SUT error rates, 4 stale adjudications |

Ignored in both suites: `resolution_test::slice_elem_variant_reserved`.
Default suite was followed by the two JS matrix tests after adding value-import
typed-parameter cases; the MCP full suite includes that final test content.
Production source did not change after the final full-suite builds.

Quick summary is committed in `2026-09-04-rust-default-quick-summary.json`.
Pinned outcomes: `target-c-method` remains flip_candidate (Exact TP=5, FP=FN=0);
the two feature-gated literal addresses remain missing; ambiguous-symbol contract
is OK. In the live module-deps probe, main.rs:183 is now on both sides and
oracle_only is empty. Its sole Prism-only site is the MCP call at tools.rs:230.
Do not reinterpret stale pins or baseline-invalid sampled metrics as acceptance.
Sampled M2 still reports other Exact-tier misses and one FP; this slice does not
claim to close every Rust resolution defect. No M2 regression attribution is made
from a drifted historical baseline.

No full multicorpus run, baseline/pin/adjudication rewrite, independent review,
or live JavaScript language-server recall measurement. No claimed real-corpus
default-class gain: the selected Excalidraw sources lack direct named default
classes (`Library` uses the still-excluded indirect form).

## Custody and reproduction

Evidence root: `/private/tmp/prism-path-default-4GrCld`.
Control binary: `/private/tmp/prism-receiver-closure-HsaNcO/merged-prism`, version
`f3bf88e0b952`. `git diff f3bf88e 1886907 -- src Cargo.toml Cargo.lock build.rs`
is empty, so its production source equals the merged base. Candidate binary:
`target/release/prism`, version `18869079452d-dirty`. Both read the identical
current source tree, with persistent caches disabled. The raw site key sets are
equal, not merely equal-sized.

```sh
BIN nav --no-cache call-stats --repo /Users/wesleyjinks/code/slicing --dump-sites
BIN nav --no-cache callers --repo /Users/wesleyjinks/code/slicing --symbol module_deps --file src/navigation/module_graph.rs --confidence exact --format json
BIN nav --no-cache callees --repo /Users/wesleyjinks/code/slicing --symbol run_nav --file src/main.rs --confidence exact --format json
```

Raw JSONL, comparison script, complete gate logs, preserved sample dumps, and
served responses are retained in the evidence directory. The committed JSON
contains the bounded delta and key served artifacts so the result is not dependent
on an untracked temporary file alone.

Evidence archive: `/private/tmp/prism-path-default-4GrCld-evidence.tgz`, SHA256
`8adeb05f0706d040216d7984b2c6fbaf0d8c2d05843aed51b381b58f04f90d04`.
The new quick report and oracle snapshot were moved into this evidence directory;
existing committed baselines and the pre-existing untracked snapshot were untouched.

## Next bounded item

Owner approved the continuation after merge: indirect **local** default-class exports such as
`class Library ...; export default Library;`, with declaration identity, duplicate
and write barriers, then measure the relevant real receiver sites. Do not combine
that with arbitrary reexport traversal or Python initializer execution modeling.
Current custody: `docs/superpowers/handoffs/2026-09-04-indirect-default-handoff.md`.

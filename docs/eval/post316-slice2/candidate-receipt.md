# Post-316 Slice 2 candidate receipt

Status: first-review freeze requested; full gates and independent candidate review pending.

## Scope and result

This candidate retains byte-distinct nonzero JS/TS/TSX Variable occurrences inside the CPG while preserving the public legacy line index as deterministic first-wins. Step 4 resolves only actual stored DFG endpoint bytes and deduplicates identical edge rows. Step 5b selects one exact in-argument Use, refuses ambiguity before Def fallback, validates every graph payload field, and preserves full-member plus base supplementation. CPG cache semantics advance 94→95; navigation stays 53.

Per the ratified [design decision](design-decision.md), the repaired path starts at the recovered source argument occurrence: later caller Use77–82 → callee parameter Def21–26 `Exact` → callee body Use35–40 `NameOnly(CfgIncomplete)`. The actual caller producer edge remains Def46–51 → earlier Use58–63 `NameOnly(CfgIncomplete)`. No incoming caller edge to77–82 is invented. Byte-distinct DFG/RD identity remains successor scope.

## Fail-first and focused evidence

- Unchanged accepted base: `d2bbe7074d12fc98280248313d9767d237be33b8`, tree `74eef96d0726882ea0a820b6a7b9552a17fcc3f2`.
- Saved base RED logs: `/private/tmp/prism-post316-orchestration/slice2/base-red/`. O01–O06 distinguish raw occurrence presence, CPG collapse, Step4 endpoint identity, slot identity, Unicode/comment bytes, and member/base supplementation.
- Producer mechanism/design amendment: `/private/tmp/prism-post316-orchestration/slice2/base-cache/authoritative-later-use-result.md` and [design-decision.md](design-decision.md).
- Final Slice2 focused run: `cargo test same_line_occurrence_o -- --nocapture` — 14 passed, 0 failed, 0 ignored; external log SHA-256 `02ecc0ce81ce7a50d7a74730e11b2f4a17c9ce5e993b9f9bc813074d40d01bbd`.
- Complete CPG unit population: `cargo test cpg:: -- --nocapture` — 253 passed, 0 failed, 0 ignored; external log SHA-256 `e3234675fb51854780f5004f44c5b2d8a5404ec75929123a37b7b66c05c8703c`.
- `cargo fmt --all -- --check` and `git diff --check` pass.

The test population covers O01–O14 across JS/TS/TSX positives, exact primitive-field tuples, ambiguity/forgery/zero-width/slot/owner negatives, an existing Exact Step4 label, identical-row deduplication, full↔serial/parallel↔incremental↔warm-cache tuple parity, legacy first-wins queries, Contains/location multiplicity, and five caller source epochs.

## Genuine old cache and measured delta

- Genuine accepted-base v94 bytes: `/private/tmp/prism-post316-orchestration/slice2/base-cache/cache/cpg-cache.bin`, SHA-256 `b6d96307917e2767ddb2290ec8f61bb41d26bc032ff048cbbf3c3f69341a4468`.
- Candidate probe reports `Cache version mismatch (cached: 94, current: 95)`, then v95 rebuild→Hit with complete fresh/warm tuple equality: `/private/tmp/prism-post316-orchestration/slice2/o15-genuine-v94.log`, SHA-256 `c07ff72ad55a1ae7415091ae348f8a552c1fc7b0090ba04450ecd2062617809a`.
- Replay harness: `/private/tmp/prism-post316-orchestration/slice2/o15-genuine-v94-harness.rs`, SHA-256 `a7be084b959165b03455e53c253274d087d4ce3196cc6c7476a0f6c308af04a7`.
- Fixed O01 graph: base 10 nodes/10 edges/6,423 cache bytes; candidate 11 nodes/12 edges/6,550 cache bytes. Delta: +1 exact Variable, +1 Contains, +1 exact argument boundary, +127 serialized bytes.
- Debug micro-measurement reconstructing the seven-node exact index 1,000 times: 3.964 ms total (six legacy buckets, seven byte spans/references). This is a small-fixture measurement, not a corpus performance claim.

## Pending before final acceptance

Full Rust/Node/Python/authority/Tier-A gates, final source-bound manifests, and independent two-round-capped review remain pending. No push, PR, merge, rebaseline, or full multicorpus run is authorized.

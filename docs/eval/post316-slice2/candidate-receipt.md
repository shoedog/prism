# Post-316 Slice 2 candidate receipt

Status: implementation and capped review complete. Final test/custody checkpoint `7fc89c98bd3eaae99dc9dc959097b942ac96fe8c` is independently approved with 0 WRONG / 1 nonblocking performance SMELL. Production is unchanged from checkpoint `498fb6f68acaa7b87a5551b840967a9c28b50011`.

## Scope and result

This candidate retains byte-distinct nonzero JS/TS/TSX Variable occurrences inside the CPG while preserving the public legacy line index as deterministic first-wins. Step 4 resolves only actual stored DFG endpoint bytes and deduplicates identical edge rows. Step 5b selects one exact in-argument Use, refuses ambiguity before Def fallback, validates every graph payload field, and preserves full-member plus base supplementation. CPG cache semantics advance 94→95; navigation stays 53.

Per the ratified [design decision](design-decision.md), the repaired path starts at the recovered source argument occurrence: later caller Use77–82 → callee parameter Def21–26 `Exact` → callee body Use35–40 `NameOnly(CfgIncomplete)`. The actual caller producer edge remains Def46–51 → earlier Use58–63 `NameOnly(CfgIncomplete)`. No incoming caller edge to77–82 is invented. Byte-distinct DFG/RD identity remains successor scope.

## Fail-first and focused evidence

- Unchanged accepted base: `d2bbe7074d12fc98280248313d9767d237be33b8`, tree `74eef96d0726882ea0a820b6a7b9552a17fcc3f2`.
- Saved base RED logs: `/private/tmp/prism-post316-orchestration/slice2/base-red/`. O01–O06 distinguish raw occurrence presence, CPG collapse, Step4 endpoint identity, slot identity, Unicode/comment bytes, and member/base supplementation.
- Producer mechanism/design amendment: `/private/tmp/prism-post316-orchestration/slice2/base-cache/authoritative-later-use-result.md` and [design-decision.md](design-decision.md).
- Final Slice2 focused run after review hardening: `cargo test --offline same_line_occurrence_ -- --nocapture` — 16 passed, 0 failed, 0 ignored; `/private/tmp/prism-post316-orchestration/slice2/focused-final.log`, SHA-256 `f3c6db8c587dc033d3431ee5081cd19ee7f4453d31016dc4572cff4f329aa205`.
- Complete CPG unit population: `cargo test cpg:: -- --nocapture` — 253 passed, 0 failed, 0 ignored; external log SHA-256 `e3234675fb51854780f5004f44c5b2d8a5404ec75929123a37b7b66c05c8703c`.
- `cargo fmt --all -- --check` and `git diff --check` pass.

The test population covers O01–O14 across JS/TS/TSX positives, exact primitive and asserted-member field/base tuples, ambiguity/forgery/zero-width/slot negatives, an actual same-name/same-line caller collision, an existing Exact Step4 label, identical-row deduplication, full↔serial/parallel↔incremental↔warm-cache tuple parity, legacy first-wins queries, Step5c ReturnInput/ReturnFlow compatibility, Contains/location multiplicity, and five caller source epochs.

Review round 1 reported **0 WRONG / 1 nonblocking performance SMELL** and four finite verification gaps. The permanent G1 asserted-member vector has a genuine unchanged-base behavioral RED (`/private/tmp/prism-post316-orchestration/slice2/base-g1-red.log`, SHA-256 `d848103e1cd071851914226f27c3739ba527455f030596e8302aa4c01b99073e`): base produced only field83–101→left instead of all four field/base×slot rows. The reviewer-custodied G3 unchanged-base control likewise has no later ReturnInput. G2 is a preservation control and is positive on both artifacts.

## Genuine old cache and measured delta

- Genuine accepted-base v94 bytes: `/private/tmp/prism-post316-orchestration/slice2/base-cache/cache/cpg-cache.bin`, SHA-256 `b6d96307917e2767ddb2290ec8f61bb41d26bc032ff048cbbf3c3f69341a4468`.
- Candidate probe reports `Cache version mismatch (cached: 94, current: 95)`, then v95 rebuild→Hit with complete fresh/warm tuple equality: `/private/tmp/prism-post316-orchestration/slice2/o15-genuine-v94.log`, SHA-256 `c07ff72ad55a1ae7415091ae348f8a552c1fc7b0090ba04450ecd2062617809a`.
- Replay harness: `/private/tmp/prism-post316-orchestration/slice2/o15-genuine-v94-harness.rs`, SHA-256 `a7be084b959165b03455e53c253274d087d4ce3196cc6c7476a0f6c308af04a7`.
- Fixed O01 graph: base 10 nodes/10 edges/6,423 cache bytes; candidate 11 nodes/12 edges/6,550 cache bytes. Delta: +1 exact Variable, +1 Contains, +1 exact argument boundary, +127 serialized bytes.
- Fixed JS/TS/TSX debug measurements (50 CPG builds per dialect) were 28.728/32.486/32.393 ms on base and 29.975/34.351/33.869 ms on candidate. The `cfg(test)` graph-reconstruction helper took 26.2–26.4 ms to rebuild the candidate exact index 10,000 times per dialect (about 2.62 µs/rebuild; six buckets, seven spans/references). This does not measure direct production lookup or assembly cost. These are bounded fixture observations, not a throughput budget.
- Pinned Tokio `ecb5125a6787` is a Rust preservation control: 122,311 nodes in both artifacts; the unique canonical edge set is identical. Candidate edge instances are 182,553 versus base 182,557 because four exact duplicate DataFlow rows become multiplicity one. One cold debug CPG sample was 32.862 s candidate versus 33.206 s base. Tokio does not represent JS/TS/TSX occurrence prevalence or bucket cost; no pinned real JS/TS/TSX root was available and no corpus acquisition/full multicorpus run was authorized.
- Full G4 inputs, methods, raw totals, limits, and evidence hashes: `/private/tmp/prism-post316-orchestration/slice2/g4/receipt.md`, SHA-256 `f13f90db364c4425238affdecd34508cded4ffc366e4e18806bd769fce61ff00`.

## Final gates and review

Final test source passed default Rust 4,534/0/1, MCP 4,727/0/1, widest audit 4,750/0/1, and examples 32/0/0. Clippy exited 0; fmt and diff checks pass. Rust receipt: `/private/tmp/prism-post316-orchestration/slice2/final-rust-receipt.md`, SHA-256 `1ece02b4ac35fd7f1b9b6b42e5d2bf96e992ee44fa42f2a71475be26fc9a91d8`. The final acceptance narrows its Clippy wording: 254 warning-prefixed lines include summaries, and no blanket base attribution is claimed.

The delegated frozen-production gates passed grammar, authority 40/0, active Node 785 pass/0 fail/1 expected skip, Python 940, and Tier-A matrix 159/159. Tier-A quick was INVALID only for corpus-pin drift and the existing C-method 4/6 oracle probes; it was retained without retry or rebaseline. The excluded populations are the three historical real-sites Node custody tests, live adoption, and the human-triggered full multicorpus run. Non-Rust receipt: `/private/tmp/prism-post316-orchestration/slice2/final-gates/nonrust/nonrust-gates-receipt.md`, SHA-256 `1872b67c5fcf76a18a0047209235c681fbdc0c1444509128748f6cac76c06c0e`.

Because the round-1 delta changes only `src/cpg/same_line_occurrence_tests.rs`, frozen binaries and non-Rust results remain production-equivalent. Review round 2 independently replayed all 16 focused tests and the final base controls, then approved at the declared cap with 0 WRONG / 1 nonblocking performance SMELL. Final acceptance is preserved at [final-acceptance.md](final-acceptance.md), SHA-256 `2fec146ce050df8a379a16432608bd553c2c4b244df06f52125ab736d907fc69`; the evidence-reconciled [review-round2.md](review-round2.md) has SHA-256 `10cd9563195ae96370556b77612d89ef48f2cb725a1d92204197bd949cf151a9`. No push, PR, merge, rebaseline, or full multicorpus run is authorized.

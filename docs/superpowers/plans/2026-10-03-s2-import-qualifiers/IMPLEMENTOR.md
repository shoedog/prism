# S2 dispatch — R2 static-binding candidate

Source application starts from committed prototype `c35719e1132809dc835f64f476cf31e2c18bd35f`, parent `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, then the R2 patch `/Users/wesleyjinks/prism-evidence/s2/repair-r2/R2-src.patch`. Planning docs HEAD is `b8f5b2f3fdc39de32877952fdb4ccf01f1483548`. The planning clone's dirty src/tests initially matched all 770 prototype inputs byte-for-byte; the new repair is not yet committed. No Git writes, F read, adoption or merge are authorized to this worker.

The controller applies/cherry-picks c35719e1 onto a bound implementation checkout and then applies R2-src.patch and R2-docs.patch (registry + planning packet), or continues the prototype branch with that same repair. Preserve the existing artifact and reviewed context. Suggested product/docs commit boundaries are in FILES. Controller source application is not adoption. Verify the requested implementation/review model from the actual transcript (brief requests gpt-6.1-sol; a requested label is not execution evidence).

1. Cherry-pick c35719e1, then apply R2-src.patch with `patch -p1`; do not start from an isolated R1b strict candidate. Bind HEAD/base/dirty scope against repair-r2 source hashes and read SPEC §0/§2a/HANDOFF. S2-O7 selects F1/F2/F5/F6/corrected F8 and excludes F3/F4/F7 runtime cuts. Combined X must remain >=122 and every change must be CORRECT_STATIC_BINDING with native module/owner/full-span agreement.
2. Preserve every populated base row byte-for-byte, the S1b-4 CallSite projection, existing Tier-A baselines and immutable base tools. Refusal joins never grant positive authority. A semantic correction regenerates source hashes, rebuilt binary hashes, measurements and gates.
3. Rebuild all head tools from this implementation checkout; do not copy a planner binary as a substitute for a build. Use the exact dependency lock. The following commands create a local facts driver and freeze freshly built head tools:

```bash
cargo build --offline --release
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/build-facts.py --head
```

Immutable main executable and facts executable are listed by path and hash in `probes/reference-binaries.json`; obtain those retained immutable artifacts or rebuild main 4e592daa in an isolated source checkout and establish complete-stream parity before replacing any reference. Never overwrite them via `--head`. Public source roots and pinned native TypeScript are specified in `probes/public.py`; bind native/source/config inputs from retained receipts.

4. Rebind BUILD-MANIFEST `binaries.head-prism` and `head-dump_imports` to rebuilt tools; CONTROLLER-s2.sh checks these exact hashes. Replay complete public comparisons against immutable main in a fresh evidence directory:

```bash
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/compare-head.py \
  target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/public-NEW
cargo nextest run --offline --features mcp
cargo test --offline --features mcp --doc
cargo fmt --all -- --check
cargo clippy --offline --features mcp --all-targets
CARGO_NET_OFFLINE=true python3 scripts/mutgate/mutgate.py \
  --lane mutants/lane-s2-import-qualifiers.json --since 4e592daa --scope file --jobs 1 \
  --out target/s2-plan/scoped-NEW
```

`--since 4e592daa` is an ancestor of both prototype and planning HEAD; 6e4e0ef1 is not an ancestor of c35719e1. Scoped mutation coverage is advisory; authoritative registry omits `--since`/`--scope` after source anchors are committed. S2-02 remains a coverage SMELL unless a realistic failing regression kills it; no equivalence claim.

5. Run immediate-rebuild Tier-A matrix and S1b-4 byte parity; preserve all lane-P public rows. Skip Tier-A quick per R2 brief; record it unverified (prior run hung over an hour). No re-baselining and no new T ownership work. Full multi-corpus Tier-A remains human-triggered.
6. Controller privately runs README's F wrapper with freshly rebuilt/rebound base/head/head-facts/native tools, returning aggregates only. This worker never reads F. Cumulative review is serial, cap two; R1 used, R2 is the next round. At the cap classify new escape families as open-class and park for design. No restart, auto-merge or adoption by the worker.


Current R2 combined yield132/132/0/0; all required worker gates complete, advisory survivor disclosed. Controller review/adoption pending. Runtime F3/F4/F7 findings, including reflected codegen, are out of model under S2-O7; static write/alias refusals remain. No runtime safety assertion follows from this candidate. Review cap two: R1 used, controller R2 next. No restart/auto-merge/adoption by this worker.

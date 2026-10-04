# S2 dispatch — existing artifact, fail-closed qualifier identity

Owner S2-O6 authorizes the closed lexical refusal design in SPEC §2. The planner repaired the existing prototype; implementation starts from these exact src/tests bytes, not a fresh restart. Cumulative review may proceed after the controller's F/authoritative-mutgate/Tier-A-quick gates. Review cap is **two rounds**; classify at the cap and fold only bounded fixes. The local repair cap was three; bounded extensions for property/destructuring tokens and namespace receiver carriers were disclosed. See PROBES for the enumerated populations and outcomes. Unknown receiver ownership must refuse module-namespace identity closure; it is not permission to add allowed-use contexts.

Planning HEAD is `6e4e0ef19de578d4865d7297eb5cf72a5b6a27a6`; product base is merged P2 `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Product src/tests remain uncommitted. The controller owns Git writes, implementation/review model dispatch and F. Planner model requested by the owner is gpt-6.1-sol; verify any subsequent dispatched model from the actual transcript. No planner subagent/review dispatch or F read occurred.

1. Bind checkout, parent, dirty inventory, BUILD-MANIFEST hashes and frozen base/head tools. Read SPEC §0 decisions and current MEASUREMENTS before transferring any certificate. O1/O2 are controller interim adoption positions; O3 keeps T at base.
2. Review the closed predicate first: all unknown uses refuse; direct literal call/new/type/declaration/own-export are the allowed set. Follow provider refusal, importer/forwarder/namespace revocation and unproved writer-owner refusal to every table. Check both-grammar S2-W1 and positive controls. Preserve the S1b-4 resolver and CallSite projection, whole base-row fence and cache versions 108/64.
3. Review positive module ownership/occupancy, qualifier identity and exact Callable spans; then class fields, declared/re-exported namespaces and literal objects. No dynamic alias acceptance, instance methods, call-result objects, default-exclusion inference or ambient expansion is authorized.
4. Replay complete public comparisons. Every changed row must be CORRECT with native module/owner/full span agreement; no already-bound row may change. Reconcile yield/refusal causes and source/binary custody after any semantic correction. Do not normalize away a difference or re-baseline a regression.
5. Controller commits the exact FILES set on the planning branch, runs all registered mutants without scope, runs Tier-A quick, and runs CONTROLLER-s2.sh on F privately. Return aggregates including complete-row correctness and UNJOINABLE/reasons. A commit/rebuild changes binary version metadata: rebind hashes and replay measurements before transferring them.
6. Dispatch independent review serially with cap two. A closed finding names input/state, wrong result, bounded fix and realistic regression. An open-class population at the cap parks for design; do not discard the partially reviewed artifact. Adoption and merge belong to the controller/owner.

Reproduction from the workspace root (choose fresh output directories):

```bash
cargo build --offline --release
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/build-facts.py --head
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/e5-alias-boundary.py \
  target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/e5-NEW
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/prototype-controls.py \
  target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/controls-NEW
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/compare-head.py \
  target/s2-plan/bin/head-prism target/s2-plan/bin/head-dump_imports target/s2-plan/public-NEW
cargo nextest run --offline --features mcp
cargo test --offline --features mcp --doc
cargo fmt --all -- --check
cargo clippy --offline --features mcp --all-targets
CARGO_NET_OFFLINE=true python3 scripts/mutgate/mutgate.py \
  --lane mutants/lane-s2-import-qualifiers.json --since 6e4e0ef1 --scope file --jobs 1 \
  --out target/s2-plan/scoped-NEW
```

Freeze tools during a run. The helper's `--head` never overwrites immutable base binaries. F command and privacy contract are in README. Authoritative registry command omits `--since`/`--scope`; advisory coverage is insufficient for merge. Snapshots are local custody, not a commit/push/remote backup. Keep disk lean after preserving artifacts and receipts.

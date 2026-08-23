# p15a — Go provider reuse (roadmap #15a) — handoff

STATUS: DONE (code + docs). Commits: d81752f (WIP provider reuse), 2672ddd (remove
TEMP timing). Final tree has no timing code; `cargo check` clean at 2672ddd.
Gates + AFTER measurements: run by the controller (this lane's sandbox cuts long
commands).

## Baseline measurements (main + TEMP commit 0b4c29d, release build)

Per-construction wall time and total `prism nav --no-cache call-stats`:

etcd (3 clean runs):
| run | embedding-promotion | interface-dispatch | receiver-indices | total |
|-----|--------------------|--------------------|-----------------|-------|
| 1 | 7.42s | 7.27s | 7.12s | 36.43s |
| 2 | 7.11s | 7.27s | 7.28s | 36.24s |
| 3 | 7.23s | 7.10s | 6.95s | 35.82s |

prometheus (noisy host; 6 runs recorded, best-effort selection):
| run | promotion | dispatch | receivers | total |
|-----|-----------|----------|-----------|-------|
| 1 | 7.70s | 7.78s | 8.66s | 51.37s |
| 5 | 8.16s | 8.36s | 10.39s | 58.48s |
| 6 | 9.20s | 9.10s | 8.18s | 57.20s |

Sum of three constructions ≈ 21.5s of ~36s etcd / ~26s of ~55s prometheus →
removing one construction should save roughly 15–20% of build time.

## Equivalence argument per site

- call_graph.rs ~2766 (`apply_go_embedding_promotion`, plain) and ~3246
  (`apply_go_receiver_indices` rematerialization, plain): both use
  `from_parsed_files` == `from_parsed_files_with_package_import_paths(files, &{})`
  (go.rs:285-287). IDENTICAL constructions → deduped: one provider is built per
  build sequence (`plain_go_provider_for_build`, lazily, only when Go files exist)
  and threaded via new pub(crate) `_with_provider` variants.
- call_graph.rs ~2899 (`apply_go_interface_dispatch_with_scope_inputs`,
  import-path-aware): NOT deduped. The import path changes each file's owner
  identity inputs to `extract_from_file` (go.rs:322-333), so its extracted data
  (dispatch table, declaration snapshots) genuinely differs from the plain one.
  No proof that plain outputs are invariant under import-path inputs → kept as a
  second construction. Result: 3 constructions per build → 2.

## What the threaded provider carries

`plain_go_provider_for_build(files)` returns an `Option<GoTypeProvider>` built once
per build sequence via `from_parsed_files` (import-path-free). It is threaded by
reference into `apply_go_embedding_promotion_with_provider` and
`apply_go_receiver_indices_with_provider`; both fall back to constructing their own
provider when passed `None` (non-Go builds never pay for construction). The
import-path-aware dispatch site keeps its own construction unchanged.

## Behavior that could differ

None expected. The deduped sites previously ran byte-for-byte the same constructor
on the same `files`, so reusing one instance is value-identical (`GoTypeProvider`
extraction is pure over `files`; no interior mutation between the two call points).
The kept-separate site is untouched.

## Pre-change controls generated

- interface-manifests (unchanged tree): were written to
  `target/p15a/manifest-before-{caddy,prometheus,etcd,hugo}.json` but target/ was
  subsequently cleaned — they are GONE. Controller must regenerate them from an
  UNCHANGED tree (separate worktree of 18b585a) before comparing against this branch.
- call-stats controls: scratchpad ctrl514-{ripgrep,caddy,prometheus,etcd,hugo}.txt
  (read-only).

## Gate results (this lane, at f270919, before TEMP removal)

- `cargo fmt --check`: clean.
- `cargo test`: 3290 passed / 0 failed / 1 ignored (3320-class, matches main).
- `cargo build --release`: ok.
- `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut`: all rows ok (104/104),
  no failures.
- Five-corpus `prism nav --no-cache call-stats` vs ctrl514-*.txt:
  caddy / etcd / hugo / prometheus / ripgrep all RAW-BYTE IDENTICAL.
- Interface-manifest comparison: NOT done in-lane (pre-change manifests lost to
  the target/ clean, see above) — controller to regenerate + compare.
- AFTER measurements: not taken in-lane; controller to run the 3-run protocol on
  etcd + prometheus at 2672ddd (TEMP removed — per-construction timings will need
  the TEMP commit 0b4c29d cherry-picked or total wall time only).

Expected AFTER shape: promotion and receiver-indices share one construction → the
three per-construction timings should collapse to two (dispatch + one shared plain
construction); expected saving ≈ one plain construction ≈ ~7s of ~36s etcd (~19%)
and ~7–9s of ~55s prometheus (~13–16%).

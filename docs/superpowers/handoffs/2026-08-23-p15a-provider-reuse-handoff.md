# p15a — Go provider reuse (roadmap #15a) — handoff

STATUS: WIP committed (d81752f). Change implemented; gates not yet run. TEMP timing
instrumentation still present in `apply_go_interface_dispatch_with_scope_inputs` and
`apply_go_receiver_indices_with_provider` (env-gated `PRISM_P15A_TIMING`) — must be
removed before the final commit.

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

## Pre-change controls generated

- interface-manifests (unchanged tree): `target/p15a/manifest-before-{caddy,prometheus,etcd,hugo}.json`
  (untracked; copy elsewhere if target/ cleaned).
- call-stats controls: scratchpad ctrl514-{ripgrep,caddy,prometheus,etcd,hugo}.txt (read-only).

## Gate results

(to be filled)

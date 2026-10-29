# #527 cfg census — progress

| # | step | state | notes |
|---|---|---|---|
| 0 | preflight | done | x86_64 Ubuntu, 4 vCPU, 15 GiB, 30 GB free; rustc 1.97.0; host-cfg.txt; identities.json (all 6 clones match pins) |
| 1 | build prism | done | 732863c0, `cargo build --release --locked --bin prism` 2:36 wall, sha256 e35c18453205805236bcb7cd3addb90e6673b51cf39ce3dab029ab82c5967156 |
| 2 | prism corpora | done | git archive 732863c0 and 20c84905 (fetched; reachable) |
| 3 | probe validation | done | fixtures/expected.json reproduced, 0 mismatches; matrix (eval/fixtures/rust) E=19, map 100%, all (none); unit tests pass |
| 4 | dumps | done | all 8; sha256 + rows in dump-manifest.json; timings/dump-timings.txt (ruff 34:18 wall) |
| 5 | census + pool | done | repair round used (summary.md); pooled E=126068, A=120 (tokio 114, uv 4, ruff 2) → step 6 |
| 6 | adjudicate | done (stopped at rule 5) | tokio 72 sites: 4+1 configs, marker+E0308, declared-type control for candidates; 22 non-tokio sites not adjudicated (outcome fixed) |
| 7 | decide | done | DECISION.md: escalate to the owner (rule 5) |

# Slice 1 source manifest

- Base commit: `f5350044a18bf95f3deb51a41b4b09e9d585b134`
- Base tree: `2e68dde9c1b587869a93acefa7923a1d337652a3`
- Branch: `feat/post316-slice1-call-ownership`
- Frozen: `2026-09-16T08:23:44Z`
- Publication: local only; no push, PR, merge, or CI run authorized.

| SHA-256 | Path |
|---|---|
| `5cea43400b66778d29d701c5da1dd312396209bfa68768bc6f80497e7af711d5` | `src/ast.rs` |
| `b2289b776273a9f539a02d4ca89992774aa71c959efea1ef83e6b29498e7b893` | `src/call_graph.rs` |
| `3a1aa80390566971215372ded4b193df50a881de037c035cd635042dafc3250d` | `src/cpg_cache.rs` |
| `fc638686a03d45e6349a390b2d2e5b1fdfa3f6e707cddd4eaa75e505a2d58de3` | `src/navigation/call_edge_cache.rs` |
| `bb297e6d6606e8cd5b3d8e598dac92218ec88d8a8770c7b4f906fecd48e3a496` | `src/ast_call_execution_owner_tests.rs` |
| `5ba307b746d604209ade3bf66a0523bfe0712ae0d31c2e8d61e812bbc0a57bd5` | `src/cpg/nested_execution_owner_audit_tests.rs` |
| `330103fd7ba4204fc3652667dadb7d18b481591faa654f9ca4df2624dc839d7f` | `tests/navigation/callers_test.rs` |

Verification logs are under `docs/eval/post316-slice1/evidence/`. The invalid
compile and invalid multi-filter CLI attempts are retained and are not evidence.

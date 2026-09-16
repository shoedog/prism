# Slice 1 source manifest

- Base commit: `f5350044a18bf95f3deb51a41b4b09e9d585b134`
- Base tree: `2e68dde9c1b587869a93acefa7923a1d337652a3`
- Branch: `feat/post316-slice1-call-ownership`
- Frozen: `2026-09-16T08:58:39Z`
- Predecessor checkpoint: `565423021510162f081412a1b9c8a3e2273ace4e`
- Publication: local only; no push, PR, merge, or CI run authorized.

| SHA-256 | Path |
|---|---|
| `bef4beb28f9b77f0c9e15afa48866d84da85262b7b274540addcaca37116ce80` | `src/ast.rs` |
| `b2289b776273a9f539a02d4ca89992774aa71c959efea1ef83e6b29498e7b893` | `src/call_graph.rs` |
| `3a1aa80390566971215372ded4b193df50a881de037c035cd635042dafc3250d` | `src/cpg_cache.rs` |
| `fc638686a03d45e6349a390b2d2e5b1fdfa3f6e707cddd4eaa75e505a2d58de3` | `src/navigation/call_edge_cache.rs` |
| `6ad080f555ffce67d4a47c8ef3ede9c935263baa12614775f0e3d4006ed7df67` | `src/ast_call_execution_owner_tests.rs` |
| `c78028e9098fc62813454862739977409dc641c2f1accb5560620655980fbbc6` | `src/cpg/nested_execution_owner_audit_tests.rs` |
| `db8eafef7a2ed2b0ab5bd9326aab3c39ebed03b3a09044e4e75d01e3d7384e6d` | `tests/integration/inline_prop_receiver_test.rs` |
| `270823e6b342a2ad367ed69b64873fff29307ace74cebd81d4117ee43fd970cc` | `tests/integration/receiver_self_binding_test.rs` |
| `04522d6ef7793f050381ddf571d8464388fc8afcf2a72333d8a16070d20affac` | `tests/lang/tsx/jsx_call_test.rs` |
| `aa2a4af5095c54f519d752dfcb15786323bda5d6749f369680fef104572199e2` | `tests/navigation/callers_test.rs` |

Verification logs are under `docs/eval/post316-slice1/evidence/`. The invalid
compile, invalid multi-filter CLI, and diagnostic shape-probe failures are retained
and are not acceptance evidence; their repaired reruns are named in the receipt.

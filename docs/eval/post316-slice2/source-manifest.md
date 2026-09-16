# Post-316 Slice 2 first-review source manifest

- Base: `d2bbe7074d12fc98280248313d9767d237be33b8` / tree `74eef96d0726882ea0a820b6a7b9552a17fcc3f2`
- Worktree: `/private/tmp/prism-post316-slice1`
- Branch: `feat/post316-slice2-occurrences`
- Freeze state: uncommitted; controller commit requested.

| Path | SHA-256 |
|---|---|
| `src/cpg/build.rs` | `a5cd3e70afec74faca8101a531273efdbe3d7b4cb5077561969fb6a5b745fd94` |
| `src/cpg/same_line_occurrence_tests.rs` | `bed129541bc20eaaf51c2caaf2431d5bba0bbf0a3e9653baacc269b6ab8cb00c` |
| `src/cpg/nested_execution_owner_audit_tests.rs` | `cc441fb78377b6d24a14796f2ec5f76006c336d6b2dd7b8d4a7e3d44de779049` |
| `src/cpg/namespace_flow_audit_tests.rs` | `daa21655ba7dd18b73d8e10202c3e91414c6aba8dec3dd87d242115ad66fa7fc` |
| `src/cpg_cache.rs` | `a1a719ecce3d53b4c97abce38a1a4a5d4ba7bf4ec4821b2f375b14636e88c1f0` |
| `docs/eval/post316-slice2/design-decision.md` | `71d733097983dc6d4d77e01b376b57a0d11aec163482d9325b18586eface6717` |

Custody docs added with this manifest: `candidate-receipt.md` and `docs/superpowers/handoffs/2026-09-16-post316-slice2-occurrences.md`; hash them after their final bytes are written and before controller commit.

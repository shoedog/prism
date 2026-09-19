# Post-316 Slice 2 second-review source manifest

- Base: `d2bbe7074d12fc98280248313d9767d237be33b8` / tree `74eef96d0726882ea0a820b6a7b9552a17fcc3f2`
- Worktree: `/private/tmp/prism-post316-slice1`
- Branch: `feat/post316-slice2-occurrences`
- Production checkpoint: `498fb6f68acaa7b87a5551b840967a9c28b50011` / tree `7437d02af2f86994b9ed43fcaa3b813a3fa63fb5`
- Final reviewed test/custody checkpoint: `7fc89c98bd3eaae99dc9dc959097b942ac96fe8c` / tree `dff38f065792cb6d7c20bb32c146b438bde8fa89`
- Freeze state: source/test bytes are final and independently approved. A docs-only custody commit is requested for the corrected production tree SHA and final gate/review status. All production hashes remain byte-identical to `498fb6f6`.

| Path | SHA-256 |
|---|---|
| `src/cpg/build.rs` | `a5cd3e70afec74faca8101a531273efdbe3d7b4cb5077561969fb6a5b745fd94` |
| `src/cpg/same_line_occurrence_tests.rs` | `0a36d430d9f298f04f1eea3ba5582ce081b81feb076ca66b5c3fcb372f743ff3` |
| `src/cpg/nested_execution_owner_audit_tests.rs` | `cc441fb78377b6d24a14796f2ec5f76006c336d6b2dd7b8d4a7e3d44de779049` |
| `src/cpg/namespace_flow_audit_tests.rs` | `daa21655ba7dd18b73d8e10202c3e91414c6aba8dec3dd87d242115ad66fa7fc` |
| `src/cpg_cache.rs` | `a1a719ecce3d53b4c97abce38a1a4a5d4ba7bf4ec4821b2f375b14636e88c1f0` |
| `docs/eval/post316-slice2/design-decision.md` | `71d733097983dc6d4d77e01b376b57a0d11aec163482d9325b18586eface6717` |

Round-1 hardening changes only the test file above: asserted-member exact vectors, actual same-name/same-line collision coverage, and Step5c/legacy-query fresh-warm compatibility. Candidate production hashes are unchanged. Custody docs updated with this manifest: `candidate-receipt.md` and `docs/superpowers/handoffs/2026-09-16-post316-slice2-occurrences.md`; hash them after their final bytes are written and before controller commit.

Byte-exact independent-review custody added for the final docs-only commit:

- `docs/eval/post316-slice2/final-acceptance.md` — `2fec146ce050df8a379a16432608bd553c2c4b244df06f52125ab736d907fc69`
- `docs/eval/post316-slice2/review-round2.md` — `10cd9563195ae96370556b77612d89ef48f2cb725a1d92204197bd949cf151a9`
- `docs/eval/post316-slice2/review-evidence-manifest.sha256` — `fa82c245594b37a83d802c1814c381f022800a0e377b2246774f5b7877560129`

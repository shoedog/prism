# Post-316 slices: local implementation and verification

## Status and custody

All three slices completed local acceptance before publication. Slice 3 final independent verdict is **APPROVE — 0 WRONG / 0 SMELL** at source/test checkpoint `440c6f4e`. On 2026-09-18, the user authorized publication: branch `feat/post316-slice3-optional-inert` was pushed at documentation closeout `9d13d7f3`, and [PR #317](https://github.com/shoedog/prism/pull/317) was opened. PR #317 merged at `2026-09-19T05:57:22Z` as `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090` after all five [CI checks](https://github.com/shoedog/prism/actions/runs/35423474801) passed on head `6e6e978d06e2e157aa9ee4d44a3bc034e32d44e7`. No live adoption or rebaseline is claimed.

The active worktree is `/private/tmp/prism-post316-slice1` (its historical name is retained). The branches form one verified ancestor chain:

| Slice | Result | Local branch | Reviewed source/test checkpoint | Accepted closeout |
|---|---|---|---|---|
| 1 | Nested calls belong to their execution owner | `feat/post316-slice1-call-ownership` | `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7` | `d2bbe7074d12fc98280248313d9767d237be33b8` |
| 2 | Preserve byte-distinct same-line argument occurrences | `feat/post316-slice2-occurrences` | `7fc89c98bd3eaae99dc9dc959097b942ac96fe8c` | `f1df12e5cf6b1fc113ed6549967586b12e71651f` |
| 3 | Admit optional identifiers beside validated inert runtime defaults | `feat/post316-slice3-optional-inert` | `440c6f4e2dc0bc24521d67b172fa51226d007716` | Accepted; docs-only successor containing this summary |

A source checkpoint binds code and tests. A later closeout commit records receipts and acceptance; it is not a new production implementation. Slice 3 production was frozen at `fcb497327d941b1ec76f6a291b78fffdca84f885`; subsequent source checkpoints changed test modules only. Final source tree is `cce3d1e19192f8b7085ca1145fa4825b287ca0e4`. NonRust reuse through this checkpoint has an explicit complete-path production-equivalence receipt.

Each slice had a two-round independent review cap. Root explicitly disclosed converging, bounded test/evidence closure supplements for both Slice 1 and Slice 3, preserving each existing artifact and its frozen production. No broader review restart or production redesign occurred.

## Verification

Rust counts are pass/fail/ignored; feature suites overlap and must not be added together.

| Slice | Default | MCP | Widest feature suite | Examples |
|---|---:|---:|---:|---:|
| 1 | 4518/0/1 | 4711/0/1 | 4734/0/1 | 32/0/0 |
| 2 | 4534/0/1 | 4727/0/1 | 4750/0/1 | 32/0/0 |
| 3, final `440c6f4e` | 4550/0/1 | 4743/0/1 | 4766/0/1 | 32/0/0 |

Final Slice 3 clippy exited 0; formatting and diff checks are clean. Same-environment predecessor controls authenticate changed behavior and preserve original failures rather than silently rebaselining them.

Slice 3 nonRust verification:

- Grammar reproduction passed; callable authority: 40 results, zero failures.
- Node: 46 active modules, 786 selected, 785 passed, zero failed, one expected archive-input skip.
- Python: 940 cases covered as 895 initial eval passes + one targeted identical-binary-basename supplement + 44 unit passes. This was not one 940-pass run. Initial socket setup refusal was inadmissible; the sole later basename-sensitive mock failure also occurred on unchanged base, then passed with identical binary bytes under the expected executable name.
- Tier-A matrix: 159/159 successful rows against a freshly built frozen binary.
- Genuine predecessor cache: CPG95 is rejected by CPG96, followed by separate rebuild/Hit and complete fresh/warm equality; navigation remains53. Exact caller-Use→optional-entry-Def bytes are pinned. The genuine-cache build identity also differs, so this is not sole-cause version-isolation proof.

## Accuracy limits and exclusions

There is **no clean Tier-A quick accuracy claim** for this chain. Slice 1 and Slice 2 quick reports were INVALID due to corpus-pin drift and incomplete C-method oracle probes. Slice 3 quick produced no terminal report: it reached the authorized 1200-second wall cap and only its owned process tree was gracefully terminated. Its SUT was frozen, but its test corpus evolved during authorized edits. No accuracy or SUT-error-rate inference is made from that incomplete run, and no retry or rebaseline was performed. Its partial snapshot and termination evidence are retained.

Explicit exclusions are the three historical imported-props source-custody tests requiring missing real-sites/five `PRISM_AUDIT_*` inputs; one expected Node grammar-archive tamper branch skip; live adoption; and human-triggered full Tier-A corpus. Slice 3 value evidence is synthetic-only, with no fixed-corpus value census claim.

Slice 2's approved proof-contract correction establishes actual later argument Use→callee parameter Def→callee body connectivity. It does not invent a missing caller-definition→later-use RD edge. Slice 3 retains the independent initializer-free route and requires a nonempty certified runtime-default vector for the new mixed class. Neither slice widens RD identity, default-value flow, or public APIs.

## Original checkout and evidence

The original `/Users/wesleyjinks/code/slicing` remains on `feat/js-ts-module-binding-audit` at `36aec5f7a16e9eb90002f01235365cdc4ab6395a`. All 14 preexisting file fingerprints match the saved inventory. Of 11 planning artifacts, four are unchanged and seven match the authorized amendment snapshots byte-for-byte. No unexpected dirty paths were found; no original source reset/clean occurred.

Primary receipts:

- [Slice 1 final verification](post316-slice1/final-verification-receipt.md)
- Slice 2: `/private/tmp/prism-post316-orchestration/slice2/final-rust-receipt.md` and `slice2/final-gates/nonrust/nonrust-gates-receipt.md` under that orchestration root.
- Slice 3 Rust: `/private/tmp/prism-post316-orchestration/slice3/final-rust-receipt.md`, SHA256 `748ae15c765f49b9ed4134ba39ac43c987991c28c644bbec02a66d75fe06d30a`.
- Slice 3 nonRust: `slice3/final-gates/nonrust/nonrust-gates-receipt.md` and `cap-final-source-equivalence.{md,json,patch}` under the orchestration root.
- Bounded test closure: `slice3/g2/RECEIPT.md`, `CAP-SUPPLEMENT.md`; independent acceptance: `/private/tmp/prism-post316-orchestration/review-slice3/FINAL-ACCEPTANCE.md`, SHA256 `31b997df59365adf7f3fbe6a2219dba627776edcd6120121bf4cd808a06b2802`.
- Original custody: `final-custody/custody-audit.json`; orchestration handoff: `handoff.md` under the same root.

The commit containing this summary records documentation closeout after final acceptance; the reviewed source identity remains `440c6f4e`. Local verification does not authorize publication or operational adoption.

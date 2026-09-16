# Handoff — post-#316 Slice 3 optional/inert signatures

**Refreshed:** 2026-09-16 · **Worker:** `/root/verification_setup` · **Review cap:** 2

## Binding

- Accepted predecessor: `f1df12e5cf6b1fc113ed6549967586b12e71651f`, tree `f74d6c8e11eb0a771a233af972ace30cff33e0cf`.
- Production-equivalent reviewed checkpoint: `7fc89c98`; the predecessor delta is docs-only.
- Worktree: `/private/tmp/prism-post316-slice1`, branch `feat/post316-slice3-optional-inert`.
- Candidate manifest: `docs/eval/post316-slice3/source-manifest.md` and external copy under `/private/tmp/prism-post316-orchestration/slice3/implementation/`.

## Candidate behavior

`typescript_parameter_bindings` now computes the all-simple inert-default validator once.
Existing initializer-free optional admission remains unchanged. A signature with an initializer
admits its simple optional token only when validation succeeds with a nonempty inert-default
vector. `Some([])` remains a successful validation but cannot erase a type-only `=` refusal.
CPG cache version is 96; navigation remains 53.

## Evidence and work remaining

- Authenticated unchanged-base RED is retained under `/private/tmp/prism-post316-orchestration/slice3/red-prep/`: 14 missing TS/TSX inert rows and a missing parameter entry Def/edge; type-only and effectful controls pass.
- Candidate focused seven-control run passes. It preserves the existing fact that a defaulted sibling is not an entry Def; no default-value flow claim was added. Raw logs are external.
- `/root/review_slice1` has preserved the genuine predecessor CPG95 P01 cache using `app.ts` SHA `7aad90624342698abf1346e8b9e63683b2b3ac6398ffc2892b8d245c9d4fcdc3`: cold miss/write/hit, 13 nodes, 14 edges, 6812 bytes. Its receipt is `/private/tmp/prism-post316-orchestration/slice3/base-cache/RECEIPT.md`; candidate proof must use a direct 95-to-96 miss followed by a separate rebuilt-cache hit.
- Required: candidate cache controls, final Rust gates in a coordinated target, reviewer round 1, then non-Rust gates against separately frozen binaries. No full Tier-A corpus, live adoption, rebaseline, publication, or git commit is authorized.

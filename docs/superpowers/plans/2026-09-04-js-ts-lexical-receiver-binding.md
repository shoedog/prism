# Plan — JS/TS lexical-scope-aware receiver binding

**Base:** `5051918f61c99fda83eb18936992fb62025b7669`
**Branch:** `js-ts-lexical-receiver-binding-owner`
**Review cap:** two rounds

1. Reconcile PR #230 as merged and mark roadmap item 3 in flight.
2. Add RED fixtures that reverse the known parameter-shadow assertions and cover lexical/function scope, TDZ/hoisting, non-reaching nested scopes, direct-subset parity, and cache persistence.
3. Add `CallSite.receiver_lexically_bound` with serde default and comparison exclusion.
4. Implement one `ParsedFile` classifier over the existing receiver/function nodes; wire it into full and subset call-site construction.
5. Guard `ImportQualified` on the new fact; leave typed/new recovery pinned negative.
6. Advance CPG 57 to 58 and navigation 26 to 27 with targeted round-trip coverage.
7. Run focused gates, review round 1, bounded fixes, review round 2, then the full project and Tier-A gates.
8. Refresh the handoff and commit every stable checkpoint. Publication of this successor PR is pending an explicit publication decision after local verification.

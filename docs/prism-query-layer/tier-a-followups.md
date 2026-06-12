# Tier-A Followups (post-final-review, 2026-06-12)

Final whole-branch review record: `tier-a-final-review-2026-06-12.md` (8 MAJOR fixed
pre-merge; these are the MINORs + corrections carried forward).

1. **build.rs worktree refs (review #9):** in a linked worktree, refs/packed-refs live
   in the common git dir — also watch `git rev-parse --git-common-dir` paths. The
   harness's runtime HEAD check backstops this today.
2. **run_corpus stage decomposition (review #10):** ~150-line composition root under
   one try; extract per-measurement stages so error scope is per-stage.
3. **interfaces.py protocols not load-bearing (review #11):** inject oracle/SUT
   factories or isinstance-assert against the runtime_checkable Protocols.
4. **Snapshot key omits oracle version (review #12):** an oracle upgrade silently
   shifts live truth against an unchanged sample; key snapshots by oracle version too.
5. **--quick silently overrides --corpus (review #13):** make --quick modulate sample
   size only.
6. **Review #14 correction:** `profile.dev` was NOT smuggled into the consolidation —
   it is its own commit (`cda4d6a`); the reviewer read the squashed code-only diff.
7. **Adjudication re-anchoring (review #6, durable half):** migrate line-keyed records
   to content-fingerprint anchoring so corpus bumps don't stale the store wholesale.
8. **Matrix v2:** collision-rich fixture variants (minimal fixtures make every name
   repo-unique, so "pass" certifies name-reachability, not dispatch mechanics);
   decorator B2 flip-indicator needs a fixture that actually triggers the func_index
   quirk (current one passes).
9. **Python oracle:** pyright call-hierarchy floor failure (31–36% err) — try
   basedpyright, then the references+containment fallback (spec §2.2) to give Python
   a valid anchor corpus.
10. **M1 inventory_diff duplicate-collapse hardening + adjudication duplicate-key
    rejection** (task-14/16 review notes).
11. **Sample-precision artifacts:** multi-line-range TP double-count (task-15 note,
    superseded by 1:1 matching fix), inventory_diff set-dedup edge.

# Dirty checkout recovery

The15 dirty files were preserved in local-only recovery commita86bbeb on
`archive/dirty-callable-20260907`. The original branch was at8e7744e2. The working
checkout then switched to `feat/callable-umd-bridge` from merged main9a34ef62 and
was clean. No hard reset, git clean, evidence deletion or recovery-branch push.

Read-only byte comparisons and `git merge-base --is-ancestor fb2ffd9b origin/main`
establish the disposition:

| Population | Count | Disposition |
|---|---:|---|
| Observer README,index,index.test,schema,worker,provenance |6| Exact bytes in already-mergedfb2ffd9b; no unique implementation |
| Configured-observation readout/handoff and alias spec |3| Exact bytes on current main |
| Roadmap and alias readout/handoff |3| Old publication/successor wording; newer main contains subsequent slices; not unfinished code |
| Local publication pointer |1| Historical custody pointer superseded by current handoff |
| .superpowers/sdd/progress.md |1|412507-byte historical ledger through August24; preserved locally, not a current execution plan |
| eval/snapshots/prism-fb81481dafa7.json |1|1505125-byte symbol inventory; eval/README.md identifies these as fixed-SHA oracle inventories, not pending implementation |

The historical ledger and snapshot warrant retention, not public source staging or
an inferred multi-project completion task. No distinct unfinished implementation
was found in this dirty population, so no extra implementation completion slice is
needed. Existing roadmap deferrals remain separate owner-scoped work.

Recovery: inspect `git show a86bbeb:<path>` or create a separate recovery worktree
on the archive branch. Do not merge it into main: that would regress the observer
and publish local historical evidence. The working directory no longer contains
the two untracked ledger/snapshot files; their exact bytes remain in that commit.

An additional local bundle was verified with `git bundle verify`:
`/private/tmp/prism-umd-bridge-LqRG69/dirty-recovery.bundle`, SHA256
`296aa9127662fbe99ffbf797c046f1d91334347f829fc104b1a77119d561623b`.
It retains the recovery ref and requires base8e7744e2 already in this repository;
it is not a standalone full-history repository backup and is not published.

# Named worktree cleanup assessment — 2026-09-07

**All17 named directories were removed after explicit owner authorization and
fresh per-path custody, status, lock, live-use and mount checks.** None were parked.
The14 registered worktrees were removed through Git; the3 independent clones
were removed separately. Unrelated registrations and protected paths remain.
Recovery branches and archives remain intact. See deletion.json for exact results.

The assessed footprint was6710264 KiB (6.40 GiB); available space rose by5836076 KiB
(5.57 GiB) during removal. APFS sharing, hardlinks and concurrent writes mean the
two figures need not match. The largest removed directory was
slicing-a-receiver-provenance-s3:4252776 KiB (4.06 GiB), including4074600 KiB in target.

Most saved heads are merged:9 by ancestry,6 by patch/squash equivalence. Two
heads are not established merged:16c1-sol and detached phase0-review. HEAD alone
was insufficient: independent clones also hold other branches and unreachable
objects, and otherwise-clean worktrees hold ignored review/evaluation evidence.

Main was freshly fetched and fixed at8534f4af7bafd069728faa4ebbe4d5fc062e3edd.
The three independent clones' stale origin/main comparisons were discarded in
favor of this exact central SHA. The removed population was14 worktrees and3 clones.

## Per-directory disposition

All paths are under `/Users/wesleyjinks/code/`; full paths, heads, hashes and
allocated sizes are in assessment.json. "Merged" says nothing about discarded
build outputs or whether an old design was approved for implementation.

| Directory | Merge evidence / additional custody |
|---|---|
| slicing-13a-go-level3-design | PR220; patch-equivalent; only ignored target output |
| slicing-16-post-provenance | Ancestor; PR215/216; generated environment/cache |
| slicing-16c1-sol | **Independent clone, unmerged local history plus dirty source/design**; exact dirty snapshot and complete object store saved |
| slicing-18-rta-fallback | PR218; patch-equivalent; clean |
| slicing-4b-go-dot-import-v7 | PR219; patch-equivalent **parked design**, not implemented dot-import support; target output |
| slicing-a-receiver-provenance-s0 | PR205; aggregate patch ID matches squash; ignored evaluation JSON saved |
| slicing-a-receiver-provenance-s1 | PR207; aggregate patch ID matches squash; ignored evaluation JSON saved |
| slicing-a-receiver-provenance-s2 | Ancestor; PR209; ignored evaluation JSON saved |
| slicing-a-receiver-provenance-s2-closeout | Ancestor; PR211; clean |
| slicing-a-receiver-provenance-s2-custody | Ancestor; PR210; clean |
| slicing-a-receiver-provenance-s3 | Ancestor; PR214; ignored evaluation JSON saved; largest target directory |
| slicing-a-receiver-provenance-scope-prereq | Ancestor; PR212; generated environment/cache |
| slicing-p4b-go-dot-import | **Independent clone**; HEAD/main ancestor; eval branch aggregate patch matches PR172; extra java-ts WIP and all objects saved |
| slicing-phase0 | Ancestor; PR229; ignored .gitignore saved |
| slicing-phase0-review | Detached older Item2 intermediate;15 non-ancestor commits,12 patch-equivalent and3 unmatched; exact history and15 ignored review files saved; not certified merge-equivalent |
| slicing-phase0-sol | **Independent clone**; HEAD targets fix patch-equivalent to main; older Item2 branch and13 ignored files saved; all objects saved |
| slicing-targets-hint | Ancestor; PR243; clean |

## Recovery branches — custody only, do not merge wholesale

Prefix: `custody/worktree-20260907-` on the existing `shoedog/prism` origin.

| Suffix | Saved tip | Purpose |
|---|---|---|
| 16c1 | 1900682c3a9a193bd78e6a805f3796e320331574 | Two original non-ancestor documentation commits |
| 16c1-dirty | b7fc86010ed13953d04f9c4c78511c5070a19ee3 | Exact four-file snapshot on original parent; no correctness/test claim |
| p4b-eval | 158d49c5a69ccee088d4202a0aea510e222644f5 | Preserve original pre-squash eval history |
| p4b-java-ts | 1cd3322df1496ce2bde972f6fb041393b3502e55 | Preserve multiline-call-argument WIP |
| phase0-item2 | 5b56c35fc35a1afcc9eccb20463aee914411e2a9 | Preserve older Item2 intermediate |
| phase0-targets | b880b26aaff7f512769c301881e6791bf4ccc491 | Preserve original patch-equivalent target fix |
| phase0-review | 3a6a94920cb52527081588655149400ab564d555 | Name the formerly detached review commit |

The dirty snapshot used a temporary index and central object store, preserving the
original branch/index/worktree until authorized removal. Three Rust files contain627 additions
and130 deletions relative to their original HEAD, plus one77-line untracked design.
Some concepts later evolved on main; preservation is not permission to reintroduce
old fallback behavior. Reconcile a future selected change against current main.

## Archive contents and restore proof

This **archive branch**, `custody/worktree-cleanup-20260907`, is not an implementation
PR and should not be merged into main wholesale. It retains:

- Three complete Git object-store archives, including unreachable commits/blobs.
  No repository config, credentials or hooks are included in these remote archives.
- Original clone refs and reflog records in clone-ref-metadata.json.
- `ignored-and-dirty.tgz`:37 byte-hashed files (33 ignored evidence/metadata files,
  four dirty source/design files),1880844 uncompressed bytes. Caches, venvs and
  target output are excluded. No secret-like matches in a bounded pattern scan of
  these37 files; this is not an exhaustive secret audit of repository history.
- assessment.json: per-directory disposition, archive hashes and verification.

The initial --all/--reflog bundles restored all1335/1344/1524 ref/reflog commits but
omitted513/1013/563 additional commit objects. They were **not** accepted as full
clone backups. Complete object-store archives were then restored into independent
bare repositories with no alternates; full object ID/type/size populations match
and `git fsck --full --no-reflogs` passes:

| Clone | Objects restored | Commits restored |
|---|---:|---:|
| slicing-16c1-sol | 13089 | 1848 |
| slicing-p4b-go-dot-import | 14498 | 2357 |
| slicing-phase0-sol | 14822 | 2087 |

Use the named remote branches for ordinary recovery. To recover an unreachable
object, initialize a **new empty** Git repository, extract the matching objects
archive into its `.git`, inspect the saved refs/reflogs, and create a recovery
branch at the selected commit. Restore ignored evidence into a separate directory
first; do not extract over active source. Local-only Git metadata backups are
retained in the task root for exact original configuration/hook custody.

## Verification, limits and deletion boundary

Immediately before each removal, host-visible ps and recursive lsof found no
matching processes/open files. The12 containers checked had no overlapping mounts;
host mounts were clear. Identity, HEAD/status, saved-file hashes, refs/reflogs,
clone objects, hidden index flags, submodules and Git locks passed. Worktree reflog
tips remained covered by remote history or archived clone objects. Exact Git
metadata backups were also taken locally before removal. All17 paths are absent,
and central/other worktrees and protected repositories remain. No Docker state,
volumes, unrelated workspaces or recovery branches were deleted.

No application tests, builds, Tier-A or new implementation slice were run: this is
storage/custody work, verified by source hashes, ancestry/patch evidence, isolated
restoration, object fsck and remote-ref equality. Recovered code is unverified WIP.
The cleanup guidance kept the operation report-first until explicit authorization,
then required fresh gates before every removal. The startup guard once stopped
before any target operation because the approval checkpoint advanced the custody
branch; publishing that documentation-only checkpoint and refreshing remote refs
resolved it. No directory-level deletion failures or retries occurred.

Raw diagnostics and local-only backups:
`/private/tmp/prism-worktree-custody-K76Tg3`.
The overly broad central all-ref diagnostic bundle stays local-only and is not
part of publication. Unrelated recovery branch archive/dirty-callable-20260907 is
not published by this task.

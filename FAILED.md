# 630py-fold1: stopped early (remote moved)

Stopped before measuring. `perf/630-ud-python` is no longer at 2ecf19f5: it is at 9f70b224
(commits 6027dfb, 3a736ee, 9f70b22, "fold 1", pushed by another session at 10:14-10:16 UTC).
My plain push was rejected as non-fast-forward. I did not force, rebase or merge onto it, because
the two folds overlap (same files: file.rs, lexical_prune.rs, 04_fold1.py, obligations.jsonl,
dfg_label_store_test.rs, CLAUDE.md) and I cannot tell which is meant to win.

## Done (pushed to `claude/pr-713-round1-fold-fipr6m`, 4 commits on top of 2ecf19f5)
- e195680 RED: same-line, rebound-base, setter rows (RED on old code, verified); Opus F7 hazard rows
  (all pass on old code: no hazard defect found); `global` removed from both same.py fixtures.
- aba6782 fix: `keep_def` gets each raw candidate's own span at both sites (option b), Python D
  identifier-only; dominator-edge invariant test; Python cache custody test; Python survivor-cap test.
- 3304820 witnesses use an if/else both-arms kill (single `kill_line`, D cannot prove) instead of `global`
  (label, parity, provenance, taint, cross-file, eval/fixtures/python/dfg_reaching_killed_def).
- e516979 CLAUDE.md paragraph (identifier-only D, decorated owners, #634 cap rule, same-line judging).
- Local: `--lib python_prune`, `--test ast dfg_label`, `--test integration dfg_label`, `--test cli min_confidence` pass.

## Not done
- Full targeted suites at RAYON 1/4, mcp, clippy; corpus measurement (django/mypy/vitess/kubernetes,
  Go-addition check, item-2 yield split); REPORT.
- Tier-A `--matrix-only` cannot run here: `FATAL boundary_violation: policy_digest_moved,
  population_digest_moved` even with main's fixtures restored. Any change to eval/fixtures also moves the pinned
  `data.fixtures` digest in eval/tier-a.lock.toml (the earlier CI fold's `global` edit already did).

## Decision needed
Which fold-1 stands: the one on perf/630-ud-python (9f70b22) or mine? I can rebase mine onto it and keep only the
non-overlapping parts (invariant test, custody and cap tests, if/else witnesses), then measure.

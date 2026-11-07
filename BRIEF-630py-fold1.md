# PR #713 (#630 slice 5, Python U+D): round-1 review fold

Branch `perf/630-ud-python`. Start from its current head (it includes the CI fold `test-results/630py-ci`; read `REPORT-630py-ci.md`
there first). The two reviews are `review-713-sol.md` and `review-713-opus.md` (in this input). Both REQUEST_CHANGES with the same two
code defects. Fix both, add the listed tests, and re-measure.

## 1. Same-line coalesced defs (both reviewers; WRONG, MATERIAL)
`def f(): x = 1; x = 2` then `sink(x)` loses its only edge: named owners key def locations by (path, line) with the first span
(`file.rs` ~160/253), and `keep_def` judges both raw candidates at that coalesced location (`file.rs` ~563 and the twin site ~624), so the
window that starts at `x = 2` drops both.
Fix (Opus option b, preferred): pass each raw candidate's own span bytes to `lexical.keep_def` at both sites while still emitting the
coalesced location, so the coalesced edge survives whenever any of its same-line defs survives. This is shared with Go: report the Go
output change on vitess and kubernetes (expected additions only, from non-gofmt `;` lines; any Go removal is a bug). If the precise fix
turns out unworkable, use option a (keep a pair whenever the def's line holds more than one lvalue span of the path) and say why.
Tests: remove the `global` lines from both `same.py` fixtures in `tests/integration/dfg_label_store_test.rs` (RED on b6bf3aa7, green
after); a matrix row `x = 1; x = 2` / `use(x)` keeping 2→3; `eval/fixtures/python/dfg_reaching_same_line` must stay unchanged and its
`min_confidence_test` case must pass. Add the invariant Opus proposes: for every pair D removes in the Python matrix and the
fixture-tree walk, an edge into the same use from a def located in W's statement survives.

## 2. Attribute paths are not fixed storage in Python (both reviewers; WRONG, MATERIAL)
A rebound base (`o = other; o.x = 0; o = saved; sink(o.x)`) and a property setter that ignores a value both lose a real flow.
Fix in code: Python D applies to plain identifier paths only; every pair whose path has fields keeps under D (U is unchanged).
Measure the yield this gives up, split by base on django and mypy: field-path D removals at b6bf3aa7 in total, those whose base is a
parameter of the owner never written in it (`self`, `cls` and the like: Opus option b), and the rest. The owner will decide from these
numbers whether to restore option b under a documented contract (properties, descriptors and `__setattr__` out of model).
Tests: both reviewers' counterexamples (rebinding and the setter shape) as keep rows, RED on b6bf3aa7.

## 3. Smaller items (fold all; each is cheap)
- Python cache custody: a focused Python case with positive U and D counts, equal across cold, full-hit and partial-hit builds, where
  editing one file drops only its contribution (sol 3). A survivor-cap case for Python if the existing Go cap tests parameterize easily.
- Hazard rows (Opus F7), all keep unless the rule proves them: `try`/`finally`, `with suppress(..)`, `for`/`while` with `else` plus
  `break`/`continue`, a generator with `yield` between W and the use, a multi-line W, starred targets, annotated assignment with a value,
  `except .. as` as a non-dominating target.
- Witnesses kept alive with `global` (Opus F4): where an RD `Killed` shape that D cannot prove by construction gives a single
  `kill_line` (a kill in both arms of `if`/`else`, or a `with .. as` rebinding), use it; otherwise keep `global` with a comment naming
  the dependency on D's global exemption.
- CLAUDE.md Python paragraph: identifier-only D, decorated functions excluded (the owner is `decorated_definition`; Opus F5), and the
  #634 cap rule (a formerly capped owner can regain edges; Opus F6).
- Tier-A: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut`; paste any regression into the report.
  `dfg_reaching_same_line` must pass; `dfg_reaching_killed_def` is an intended removal (fix the fixture as for the witnesses).

## Measure (release merge-base vs new head; corpora via REST `gh api repos/shoedog/prism/releases/assets/<id> -H 'Accept: application/octet-stream'`,
check against SHA256SUMS asset 715288388: django 688449716, mypy 688449784, vitess 699857148, kubernetes (its id in CORPORA.md, asset
715288423); one at a time, push the report after each)
`nav --no-cache dfg-stats --edges` and `call-stats`, 4 threads, `add2.py` both directions. Report removed and added (Python: 0 added;
Go: additions only from item 1, each class explained), kept-label transitions (0, or each explained), removed by label and rule, call-stats
identical outside `dfg_labels`, and the item-2 yield split. Sample 20 removed Python edges per corpus and every Go addition up to 20,
with source lines and RIGHT/WRONG. Delete raw dumps; never push them.

## Do
`cargo fmt`; clippy clean on touched code; the file-size rule. Targeted at RAYON_NUM_THREADS 1 and 4: `--lib data_flow`, `--lib cpg`,
`--lib cpg_cache`, `--test ast`, `--test cli`, `--test integration`, `--test lang_python`, `--test lang_go`, `--test navigation`,
`--test algo_taxonomy`, and `cargo test --features mcp --test mcp`. For failures that also fail on the merge base in your environment
(running as root), show the base run beside them. RED-first commits, each in a reviewable step. Push `perf/630-ud-python` (plain push,
never force). Write `REPORT-630py-fold1.md` (with a PR-body addendum) and push it incrementally to `test-results/630py-fold1`. If you
stop early, push `FAILED.md` there with exactly what is done.

## Rules
Do not open PRs, comment on GitHub, or touch other branches. Never read or print dotfiles or credentials. Do not open the `eval/secbench`
directory and exclude it from every recursive search. Static analysis only: never execute Python or Go code or packages; prism only
parses corpora.

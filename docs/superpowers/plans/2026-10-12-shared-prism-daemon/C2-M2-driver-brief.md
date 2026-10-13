# Implementation brief: memreport driver — selectable kinds, a `build` kind, and `compare`

> **Superseded in part by review.** Two reviews changed what the driver does: a `build` repetition must have
> written both cache files; corpus names are restricted; comparability is checked per corpus and repetition,
> platform included; `compare` rescores from `raw/`; not-worse requires the ranges to separate; one-sided data
> gives `incomplete` and a gate short of data is `undecided`. `scripts/memreport/README.md` is the reference;
> this file is the brief as first issued.
>
> As issued to the implementer on 2026-10-13. One rule changed afterwards and is reflected below: the live-heap
> check is one-sided ("not higher"), because a candidate that allocates more tightly lowers the live heap
> without changing the index. The scratchpad paths are the session's and are kept as issued.

Worktree: `/Users/wesleyjinks/code/prism-c2-driver`, branch `feat/memreport-experiment-kinds` (from `origin/main`).
Work only in that worktree, only under `scripts/memreport/`. Python 3 standard library only. No Rust, no cargo.

## Why

`scripts/memreport/` measures where a `prism-mcp` index's memory goes (read `scripts/memreport/README.md` first).
The next step is a baseline-against-candidate experiment for a change that streams the CPG cache file on load and
on save. The experiment needs three things the driver cannot do yet:

1. run only some kinds (a cold build of one corpus takes 13 minutes and the experiment does not need it);
2. measure a **first build that writes the cache** several times (today that is the priming run, one per run);
3. compare two run directories under a pinned rule.

## What to build

### A. `run --kinds LIST`
- Comma-separated subset of `warm,cold,workload,refresh,build`. Default `warm,cold,workload,refresh`: with the
  default, behaviour and `summary.json` must be what they are today except for the one additive metric in (C).
- Unknown or repeated names, or an empty list: refuse before anything runs.
- The priming run happens only when at least one of `warm`, `workload`, `refresh` is selected.
- `run.json` records the kinds. `summarize --run DIR` must rescore a run directory that has only some kinds, and
  one written before this change (no `kinds` key: treat as the default four).
- The readout (`render`) must not break on a summary that lacks a kind; it already prints `n/a` for missing figures.
  Add a test that renders a `--kinds warm,build` summary without raising.

### B. kind `build`: a first build that writes the cache
- Each repetition gets its own empty cache directory `<run>/cache-build/<corpus>/<rep>/` and runs the same command
  as `warm` (`--cache-dir` that directory).
- Admissible only if the report itself says `cache.cpg_hit is False` and `cache.call_edges_loaded is False`
  (reasons: "the CPG cache was hit, so this was not a first build" / "the call-edge index was loaded, so this was
  not a first build"), plus the usual checks (`well_formed`, build-identity consistency, stages present).
- After the process exits, record the sizes of the cache files it wrote in the stored result (reuse
  `cache_file_sizes`), then delete that repetition's cache directory (each is hundreds of MiB).
- Derived metrics: `build_peak` = `peak_bytes` at stage `after_call_edges`; `build_ms` = `after_index.elapsed_ms`
  minus `after_load_repo.elapsed_ms`. A value that is unavailable is `None` and stays out of every statistic,
  as elsewhere.
- `build` is not part of the readout's four kinds: keep `KINDS` (render's tuple) as it is and add a separate
  constant for "every kind `run` accepts". Do not add a column to the readout.

### C. `warm` gains `load_ms`
`after_index.elapsed_ms` minus `after_load_repo.elapsed_ms`, `None` when either is unavailable. Additive.

### D. `compare --experiment m2 --baseline DIR --candidate DIR --out FILE.md [--json FILE.json]`
New module `scripts/memreport/memreport_compare.py` (keep `memreport.py` under 600 lines; it is 522 now, so put
new logic in new modules and only wire the subcommands in `memreport.py`).

It reads the two `summary.json` files and **refuses** (exit non-zero, message naming the difference) unless:
the two runs have the same corpus names; for each corpus the same commit, the same dirty count and the same
workload sha256 where a workload is recorded; the same `reps` and `settle`; the same thread count, the same
measured-process environment (`env`) and the same allocator; and **different** build identities (comparing a
build with itself is refused unless `--allow-same-build` is given, which exists for a noise check).

Pinned rule for experiment `m2` (make the experiment a table in code so another can be added):

| Metric key | Label | Unit | Role |
|---|---|---|---|
| `peak` | Load peak (cache-loaded, through the index load) | bytes | gate |
| `build_peak` | First-build peak (writes the cache) | bytes | gate |
| `load_ms` | Index load time | ms | not worse |
| `build_ms` | Index build time | ms | not worse |
| `steady_heap` | Steady live heap | bytes | not higher |

- **gate**, per corpus: `separated` = candidate max < baseline min; `reduction` = baseline median − candidate
  median; `effect` = reduction ≥ 100 MiB **or** ≥ 5 % of the baseline median; the corpus **passes** when both
  series have n ≥ 3 and `separated` and `effect`. Fewer than 3 runs on either side: "insufficient runs", not a
  pass and not a fail. The gate metric **passes** when at least two corpora pass.
- **not worse**, per corpus: ok when candidate median ≤ 1.05 × baseline median. The metric is ok when it is ok
  on every corpus that has both series; a corpus with a missing series is listed as not measured.
- **not higher**, per corpus: ok when the candidate median is at most 1 MiB above the baseline median.
- A metric absent from both runs is reported as "not measured" and decides nothing.

Output: a Markdown file with (1) both runs' provenance (build identity, git sha, cache version, allocator,
threads, env, reps, settle, per-corpus commit), (2) the rule text above, (3) one table per metric with, per
corpus: baseline median (min–max, n), candidate median (min–max, n), difference of medians (absolute and %),
and the per-corpus outcome, (4) a verdict table: one line per metric. Bytes are shown in MiB with one decimal,
milliseconds as integers, an unavailable value as `n/a` (never 0). `--json` writes the same content as data.
The comparison is pure (`compare(baseline_summary, candidate_summary, experiment) -> dict`), with the file and
argument handling separate, so it is testable without files.

## Tests (unittest, same style as the existing `test_memreport*.py`; keep every file under 600 lines)
Use the fake-binary helpers in `memreport_fixtures.py` (extend them as needed; the fake must be able to report a
cache miss for `build` and a hit for a second run into the same directory). Do NOT run a real `prism-mcp`
anywhere: another measurement is using this machine.
At least:
- `--kinds`: default equals today's run (same kinds, same raw file names); a subset runs only those kinds;
  unknown/repeated/empty are refused before any child starts; priming is skipped when no cached kind is selected;
  `summarize` rescoring a subset run gives the same summary as the run wrote; a pre-change `run.json` without
  `kinds` still summarizes.
- `build`: each repetition's cache directory is distinct and is gone afterwards; a repetition whose report says
  the cache was hit is inadmissible with the reason above; `build_peak` and `build_ms` are derived from the
  right stages; the recorded cache file sizes are those of the files the child wrote.
- `load_ms` derived for warm; `None` (and absent from statistics) when `elapsed_ms` is missing.
- `compare`: every refusal listed above, each with its own test; a gate pass (separated and effect by the
  absolute arm; by the fraction arm); overlap by one byte is not a pass; n = 2 is "insufficient runs";
  a gate that passes on one corpus only does not pass; not-worse at exactly 1.05 is ok and just above is not;
  a live heap exactly 1 MiB higher is ok and any lower figure is ok; a metric absent from both runs is "not measured"; the Markdown shows `n/a`
  and not `0` for an unavailable value.
- Run the whole suite and report the totals:
  `python3 -m unittest discover -s scripts/memreport -p 'test_*.py' > /tmp/claude-driver-tests.log 2>&1` is NOT
  allowed (do not use /tmp); write logs under
  `/private/tmp/claude-501/-Users-wesleyjinks-code-slicing/fd583811-fdc5-4426-9c4a-55872d2ecc1c/scratchpad/c2-driver/`
  and read the file. There are 145 tests today; none may be removed or weakened.
- For three of your new tests, show they bite: break the code they cover, see them fail, restore. Say which.

## Also
- Update `scripts/memreport/README.md`: the kinds table (add `build`), `--kinds`, the `compare` command, the
  pinned rule, and the run-directory layout (`cache-build/`, `kinds` in `run.json`).
- Style: match the existing files (plain functions, short docstrings that say what and why, no classes unless
  the existing code uses one for the same purpose, lines ≤ 115 columns as the existing code).
- Commit in logical steps with clear messages; never `git commit --amend`, never `git stash`, never force-push,
  do not push at all. End every commit message with these two lines exactly:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh`

## Report back
The commits (hash and subject), the full-suite totals, the three bite checks, every place you deviated from this
brief and why, and anything in the brief you found wrong or ambiguous and how you resolved it. If a requirement
cannot be met as written, do the closest correct thing and say so; do not silently drop it.

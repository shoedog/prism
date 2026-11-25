# Lane 6 census memo: provenance on real diffs (stage M)

Issue: #845. Plan: `PLAN.md` stage M (tasks M-1 to M-4). Aggregates only; the raw runs, items, labels and scripts are in
the lane's evidence directory. Base build: `origin/main` 9d1a7d06 (the plans commit 06157400a changes documents only).

## Result

**Go gate: GO.** On the base build, the share of `concern` and `warning` provenance findings (user_input and database) whose
claimed origin is right is **7.4%** with the request or handle object itself counted wrong and **12.9%** counted right, on
an inclusion-weighted labelled sample over 240 real diffs. Both are far below the 60% threshold, so the lane continues to
stage T. Over half of the sampled findings (not_variable) name a token that is not a variable at all (module, attribute,
callee, function, type, keyword, comment or string text); the structural slice S1 targets that class.

## Method (summary)

- Population (M-1): 240 real diffs from public repositories: the 120 diffs of the #602 population (apache-answer and
  miniflux-v2 (Go), calcom-cal.com and directus-directus (JS/TS), apache-superset and saleor-saleor (Python), 20 each; spec
  hashes reproduced), 60 more Python-heavy diffs (candidates 21 to 50 of the superset and saleor streams), and 60
  pattern-language diffs (15 each from redis (C), leveldb (C++), guava (Java), ripgrep (Rust); the four clones are
  full-history blobless clones made for this study because the benchmark clones are depth 1; selection head = the benchmark
  clone's commit). Selection rule: most recent non-merge, single-parent commits with at most 15 changed files and a changed
  non-test file of the family's language.
- Base run (M-2): `prism --algorithm provenance --format json --no-cache` plus `prism targets`, 240 of 240 complete.
- Labelled sample (M-3): findings of every severity, stratified by the finding file's language family x origin, at least 30
  per cell or the whole cell, seed 20261121, inclusion weights kept: 655 items in 26 cells. Readers saw the use line, the
  whole enclosing function (windowed above 160 lines), the file's imports and the lines the tool nominated, and named the
  origin they saw before any comparison; the verdict (right, wrong_origin, source_object, not_variable,
  insufficient_evidence) is derived by comparing the readers' blind label with the claimed origin. A third read resolved
  every disagreement. The rubric is `RUBRIC.md` of the evidence directory (v1.1, below).
- Source census (M-4): 400 changed lines, below.

## Population and volume (base build 9d1a7d06, all findings)

The volume table groups findings by the family of the diff's repository; the precision table below groups them by the family of the finding's file (a Python repository's diff can carry JS/TS files), so their per-family counts differ (py/js/go 2629/1197/1074 against 2438/1442/1020; both sum to 4900 over those three families).

| diff family | diffs | files | diff lines | findings | per diff | per 100 diff lines | concern | warning | info |
|---|---|---|---|---|---|---|---|---|---|
| py | 100 | 354 | 7209 | 2629 | 26.3 | 36.5 | 610 | 575 | 1444 |
| js | 40 | 150 | 2767 | 1197 | 29.9 | 43.3 | 510 | 162 | 525 |
| go | 40 | 156 | 1793 | 1074 | 26.9 | 59.9 | 377 | 126 | 571 |
| c | 15 | 37 | 729 | 322 | 21.5 | 44.2 | 118 | 59 | 145 |
| cpp | 15 | 22 | 178 | 122 | 8.1 | 68.5 | 82 | 0 | 40 |
| java | 15 | 51 | 158 | 34 | 2.3 | 21.5 | 0 | 0 | 34 |
| rust | 15 | 19 | 438 | 92 | 6.1 | 21.0 | 22 | 0 | 70 |

Origins over all 240 diffs: {'function_param': 1952, 'user_input': 1719, 'config': 857, 'database': 922, 'env_var': 20}. No `external_call` or `hardware` finding is produced.

## Precision of the base build, labelled sample (every severity)

`prec` = right / (labelled = sample minus `insufficient_evidence`); `so` = request or handle object itself, counted wrong or right. Interval is a 95% Wilson interval on the unweighted cell (the cell is the stratum, so weights do not change a cell's rate).

| file family | origin | population | sample | right | source_object | not_variable | wrong_origin | insufficient | prec (so wrong) | prec (so right) | 95% interval (so wrong) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| c | config | 55 | 30 | 0 | 7 | 20 | 3 | 0 | 0.00 | 0.23 | 0.00 to 0.11 |
| c | database | 59 | 30 | 0 | 0 | 14 | 14 | 2 | 0.00 | 0.00 | 0.00 to 0.12 |
| c | env_var | 1 | 1 | 0 | 0 | 1 | 0 | 0 | 0.00 | 0.00 | 0.00 to 0.79 |
| c | function_param | 89 | 30 | 7 | 0 | 11 | 12 | 0 | 0.23 | 0.23 | 0.12 to 0.41 |
| c | user_input | 118 | 30 | 1 | 0 | 23 | 3 | 3 | 0.04 | 0.04 | 0.01 to 0.18 |
| cpp | config | 10 | 10 | 0 | 0 | 10 | 0 | 0 | 0.00 | 0.00 | 0.00 to 0.28 |
| cpp | env_var | 1 | 1 | 0 | 0 | 1 | 0 | 0 | 0.00 | 0.00 | 0.00 to 0.79 |
| cpp | function_param | 29 | 29 | 3 | 0 | 22 | 0 | 4 | 0.12 | 0.12 | 0.04 to 0.30 |
| cpp | user_input | 82 | 30 | 1 | 0 | 16 | 10 | 3 | 0.04 | 0.04 | 0.01 to 0.18 |
| go | config | 116 | 30 | 0 | 0 | 18 | 12 | 0 | 0.00 | 0.00 | 0.00 to 0.11 |
| go | database | 126 | 30 | 0 | 1 | 16 | 13 | 0 | 0.00 | 0.03 | 0.00 to 0.11 |
| go | function_param | 442 | 30 | 7 | 4 | 18 | 1 | 0 | 0.23 | 0.37 | 0.12 to 0.41 |
| go | user_input | 336 | 30 | 1 | 2 | 11 | 16 | 0 | 0.03 | 0.10 | 0.01 to 0.17 |
| java | config | 8 | 8 | 0 | 0 | 6 | 0 | 2 | 0.00 | 0.00 | 0.00 to 0.39 |
| java | function_param | 26 | 26 | 6 | 0 | 20 | 0 | 0 | 0.23 | 0.23 | 0.11 to 0.42 |
| js | config | 197 | 30 | 0 | 6 | 15 | 6 | 3 | 0.00 | 0.22 | 0.00 to 0.12 |
| js | database | 196 | 30 | 3 | 0 | 14 | 11 | 2 | 0.11 | 0.11 | 0.04 to 0.27 |
| js | function_param | 457 | 30 | 5 | 1 | 23 | 1 | 0 | 0.17 | 0.20 | 0.07 to 0.34 |
| js | user_input | 592 | 30 | 4 | 4 | 15 | 7 | 0 | 0.13 | 0.27 | 0.05 to 0.30 |
| py | config | 471 | 30 | 2 | 1 | 16 | 9 | 2 | 0.07 | 0.11 | 0.02 to 0.23 |
| py | database | 541 | 30 | 1 | 1 | 20 | 8 | 0 | 0.03 | 0.07 | 0.01 to 0.17 |
| py | env_var | 18 | 18 | 0 | 0 | 18 | 0 | 0 | 0.00 | 0.00 | 0.00 to 0.18 |
| py | function_param | 839 | 30 | 5 | 2 | 22 | 1 | 0 | 0.17 | 0.23 | 0.07 to 0.34 |
| py | user_input | 569 | 30 | 3 | 1 | 16 | 9 | 1 | 0.10 | 0.14 | 0.04 to 0.26 |
| rust | function_param | 70 | 30 | 6 | 0 | 19 | 5 | 0 | 0.20 | 0.20 | 0.10 to 0.37 |
| rust | user_input | 22 | 22 | 0 | 0 | 17 | 4 | 1 | 0.00 | 0.00 | 0.00 to 0.15 |

**Go gate quantity.** Precision at `concern` and `warning` (user_input and database cells, all families, inclusion-weighted): **7.4%** with the request or handle object counted wrong, **12.9%** counted right. Threshold: below 60% both ways to continue. Result: **GO** (both far below 60%).

By family (same quantity):

| family | so wrong | so right |
|---|---|---|
| py | 6.9% | 10.3% |
| js | 12.7% | 22.9% |
| go | 2.4% | 8.2% |
| c | 2.4% | 2.4% |
| cpp | 3.7% | 3.7% |
| rust | 0.0% | 0.0% |

## Causes of wrong findings (labelled sample, every non-right and non-insufficient item; one cause each)

| cause | items |
|---|---|
| non_variable_token | 378 |
| other_function_definition | 71 |
| line_inheritance | 34 |
| request_object_itself | 31 |
| signature_substring | 18 |
| trigger_in_name | 17 |
| killed_or_later_definition | 10 |
| one_sided_boundary | 8 |
| other | 3 |
| precedence | 3 |
| trigger_in_comment_or_string | 2 |
| constant_masking | 2 |
| total | 577 |

## Readers

Base sample: 655 items. Reader 1 gpt-6.1-terra, reader 2 gpt-6.2-sol, third read Opus-5.5 on disputes. Tokens read by at least one reader as a variable: 265; agreed 198, disputed 67 (all went to the third read, none unresolved except where the third read said `insufficient_evidence`); Cohen's kappa over those tokens 0.68. Items where both readers said `non_variable` are final as `not_variable` and are not in the kappa.

Source census: 400 items, 422 tokens read as variables by at least one reader. Reader 1 Sonnet-5.5, reader 2 gpt-6.1-terra, third read Opus-5.5. Agreed 274, disputed 148; kappa 0.57. 11 tokens stay unresolved (third read `insufficient_evidence`).

Final labels are agreed labels or the third read. The sample readers worked under rubric v1.1 from the start (the clarification was written before the sample readers started and after the first census readers had begun). Readers saw no tool output about the items. Rubric v1.1 (chain rule added after the first census readers disagreed on `req.query[...]`; every disputed census token and every disputed sample token was resolved under v1.1; agreed labels were not re-read).

## Source census: recall of the base build (400 changed lines, py/js/go, 180 diffs)

Selection (before any head build, independent of both builds): changed lines of the 180 py/js/go diffs that have at least one value-read identifier by a syntax-free scan (strings, comments and docstrings blanked), `random.Random(20261122)`, strata family x handler (a handler is a function with a route decorator or registration, a parameter named `request`, `req` or `ctx`, or typed `*http.Request` or `*gin.Context`): py 58 handler + 102 other, js 60 + 60, go 60 + 60. Candidates per stratum: {"go/other": 338, "go/handler": 619, "js/other": 1503, "py/other": 2338, "py/handler": 58, "js/handler": 312}. Inclusion weights are kept (the table is weighted).

| origin class (final label) | labelled reads | weighted | reported with the same origin | recall (weighted) | reported with another origin | not reported |
|---|---|---|---|---|---|---|
| user_input | 44 | 243 | 37 | 71% | 2 {'config': 1, 'function_param': 1} | 5 |
| database | 25 | 219 | 2 | 1% | 7 {'config': 2, 'function_param': 1, 'user_input': 4} | 16 |
| external_call | 5 | 89 | 0 | 0% | 1 {'user_input': 1} | 4 |
| env_var | 3 | 16 | 0 | 0% | 0  | 3 |
| config | 13 | 119 | 6 | 79% | 4 {'user_input': 4} | 3 |
| function_param | 90 | 1445 | 33 | 38% | 25 {'user_input': 16, 'config': 7, 'database': 2} | 32 |
| hardware | 0 | 0 | 0 | n/a | 0  | 0 |
| source_object | 45 | 423 | 0 | 0% | 24 {'user_input': 13, 'database': 1, 'config': 3, 'function_param': 7} | 21 |

The base build never produces `external_call` or `hardware`, so recall for those classes is 0 by construction (5 external_call reads and no hardware reads in the sample; the census has no oversampled stratum for them). `source_object` rows are request or handle objects themselves: the base build reports them as user_input, config, database or function_param (the wrong-class column), never as a distinct class. Definition-only changes (a source written on a changed line and read only on unchanged lines) are the complement of this census's frame and were not enumerated; they are out of scope for recall and recorded as such. 33 of the 422 census tokens (22 agreed `insufficient_evidence` plus 11 unresolved after the third read) are excluded from every row; the recall figures are therefore for decided reads, and counting those 33 as misses would lower each class by at most the share of the excluded reads that belong to it (unknown). Definition-only changes: **deviation from M-4**: not tallied (the census frame cannot see them); deferred, to be sized in stage D if the design needs it. Recall has no interval here: the per-class counts are small (n shown) and every class is far below the 95% recall gate, so only the direction is used.

## Notes for the next stages

- Rubric v1.1 added a chain rule (the root of a member chain or call is labelled by what the whole chain yields, not
  `source_object`) after the first census readers disagreed on `req.query[...]`; every disputed token was resolved under
  v1.1, agreed labels were not re-read. Stage D may change the rubric for the request or handle object; if it does, both
  samples are re-adjudicated (DESIGN 6).
- A first census draft selected docstring lines (a defect of its line scanner); it was discarded before scoring and the
  fixed scanner produced the census above.
- Reader seats were adjusted during the run for operational reasons (the run's Claude-agent cap): census readers were
  Sonnet-5.5 and gpt-6.1-terra; sample readers gpt-6.1-terra and gpt-6.2-sol; third reads Opus-5.5.
- Causes above come from one gpt-6.1-terra pass over the 577 non-right, non-insufficient items; they seed the stage T rows
  and are not a verdict on any rule.

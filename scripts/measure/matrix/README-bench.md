# Benchmark rows (stage T): `bench-findings` and `bench-negatives`

`rows-bench.jsonl` holds one JSON object per line with exactly the fields `id`, `group`, `input`, `required`,
`truth`, `source`. Ids are `bf-nnn` (group `bench-findings`, enabled by S6a) and `bn-nnn` (group
`bench-negatives`, enabled by S6b). `input.case` is the repository-relative case directory; the other
`input` entries name files inside it. `source`: `product` (a tiny synthetic package run through the
baseline prism binary, tree and raw output kept), `synthetic` (a real output edited, or a stub, or settled by
reasoning; `required` says which fact settles it) or `probe` (unused here). Rows state behaviour and a truth,
never a rule. Check: `python3 -m unittest eval/secbench/test_matrix_rows_bench.py` from the repository root.

## `cases/bf-nnn/` (finding-level accounting)
- `row.json`: the ground-truth annotation as a harness row carries it: `sink` (`file`, `line`, `column`,
  `original` as `file:line:col`, `source_lines`), `source` (`file`, `start_line`, `seed` as the
  `--taint-source` values, `data_parameters`), `gt_status`, `gt_reason`.
- `invocation.json`: `{"taint": {argv, returncode, error, stdout_sha256, stderr}}` in the shape `run.py invoke()`
  writes (`<pkg>` and `<case>` stand for local paths). `taint` is null when ground truth is unavailable (the
  harness returns before the taint channel). A timeout has `returncode` null and `error` `timeout after 120s`.
- `taint.json`: the raw review output of the taint channel (absent when the channel failed). Real shapes
  from the baseline binary (`slicing 4.0.0 (9d1a7d06f653)`); it has no `rules` field, so rules rows add one
  by hand (synthetic).
- `pkg/`, `seed.json`: the package and the diff file the run read (all rows keep them).
- Truth: `line_hit` (sink_line, ambiguous, sink_file, elsewhere, none, error, not_scored), `findings` (one row per
  emitted `taint_sink` finding: file, line, match), `rules`.

## `cases/bn-nnn/` (negatives from fixed pairs)
- `meta.json`: `class`, `entry`, `id`, `name`, `vulnerableVersion`, `fixedVersion` (as spelled; absent when none),
  optional `fixCommit`, `sink` as `file:line:col` (null without ground truth), `source` (`file`, `line`, `name`).
- `observations.json`: `{"vulnerable": {"reach": <reach outcome>, "finding_line_hit": <line_hit>}}` (the positive
  control), or `{"gt_status": "gt_unavailable"}`.
- `vuln/`, `fixed/`: tiny packages with `package.json` (absent for metadata-only rows).
- `raw/` (product rows): `{vuln,fixed}-seed.json`, `-taint.json` (review taint channel seeded at the source
  function), `-frontier.json` (`nav taint-reaches` without `--sink`); `reach` is a frontier item on the mapped
  occurrence's line.
- Truth: `{"outcome": "verified_pair" | <reason>, "columns": {finding, reach, function_change, sink_removed}}`.
  For a verified pair `sink_removed` is false and finding/reach are booleans; for a verified removal
  `sink_removed` is true and finding/reach are null; for a reason every column is null. `{"undetermined": "<readings>"}`
  marks a row whose reason DESIGN and PLAN leave open. The set-level row `bn-026` has `set.json` and
  `entries/*` and truth `{"outcome": "unmet", ...}`.

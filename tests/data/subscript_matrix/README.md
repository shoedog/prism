# Subscript data-flow matrix

From the repository root, regenerate the cases and indexes, then check the
committed truth:

```sh
python3 -B tests/data/subscript_matrix/gen_rows.py
tests/data/subscript_matrix/truth/check.sh
```

From this directory, use `python3 -B gen_rows.py` and `truth/check.sh`.
The generator owns the fixture programs, row groups, marker spans and graph
expectations in `observations.jsonl`. See [BASELINE.md](BASELINE.md) for
the draft run and move receipt, and [DROPPED.md](DROPPED.md) for removals.

## Row and observation records

Each row ID names one file relative to `cases/`. Source, W, R and sink
lines are 1-based. Byte ranges are UTF-8 half-open intervals. Rebuilt rows
include a case SHA-256 and a `reads` list with role, line and bytes. In
cases with multiple reads, R is the primary read and R1 marks the
additional read.

The Rust baseline enables `control`, `baseline-gap` and `boundary`.
Controls retain their draft assertions. Baseline gaps pin the existing
verdict and byte-bound graph observations, including the absent complete
sink Uses in C01.java and M04.js. Boundary rows require a complete sink
Use, preserve the baseline verdict and forbid the marked edges. The
`recall-py`, `recall-js` and `recall-go` rows retain their native truth
and expected marked additions for later activation.

Stage M deferred new Q seeds for JS, TS, TSX and Go with zero product delta.
Stage T keeps the inherited controls, baseline gaps and boundary rows in those
modes. Their inherited recall groups stay disabled.

## Executed truth

Write contained, import-free cases with one call to `sink(value)`. Use `src()`
for a single marker or `src_a()` and `src_b()` for a mixed Python fixture.
The runner supplies these functions. A sink in a helper can be on a
different line from R. C01.c, C01.java and C01.rs are structural cases with
`n/a` truth. Both Y12 rows preserve the executed `not` result and pin
the argument-binding observations independently.

The [truth runner](truth/README.md) records runtime versions and observed
marker presence. Preserve the receipt lines for unchanged cases. Rebuilt
cases bind the executed result to the case SHA-256.

The `spellings/` inputs, `truth/spelling_truth.py` and
`truth/spelling_truth.jsonl` are historical assets. The active generator,
truth commands and tests operate on the case matrix.

## Python Q rows

Each Q row stores its expected sink bits, typed write/read sites and
addition labels, and per-source production verdicts in `markers`. A
selected-pair assertion follows the isolated source's production witness
through the marked DataFlow hop to the bound sink. An explicit node count
preserves an absent or colliding endpoint in a structural boundary.
The recall and limits expectations remain dormant in the normal stage T
run. See [BASELINE.md](BASELINE.md#stage-t-part-2-parent-red) for the
exact-parent receipt command.

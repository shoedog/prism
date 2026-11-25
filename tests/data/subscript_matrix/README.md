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

## Executed truth

Write contained, import-free cases using `src()` once and `sink(value)`
once. The runner supplies both functions. A sink in a helper can be on a
different line from R. C01.c, C01.java and C01.rs are structural cases with
`n/a` truth. Both Y12 rows preserve the executed `not` result and pin
the argument-binding observations independently.

The [truth runner](truth/README.md) records runtime versions and observed
marker presence. Preserve the receipt lines for unchanged cases. Rebuilt
cases bind the executed result to the case SHA-256.

The `spellings/` inputs, `truth/spelling_truth.py` and
`truth/spelling_truth.jsonl` are historical assets. The active generator,
truth commands and tests operate on the case matrix.

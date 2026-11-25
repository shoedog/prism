# Subscript data-flow matrix

Run `python3 -B tests/fixtures/subscript_matrix/gen_rows.py` from the checkout
root to regenerate `cases/` and `rows.jsonl`. Edit the generator for changes
to the case bodies, groups, marker spans or recorded base receipts.

## Row index

- `id` and `file` are both `<seed ID>.<extension>`. `file` is relative to
  `cases/`. `lang` is `py`, `js`, `ts`, `go`, `rs`, `java` or `c`.
- `group` is `control`, `S2`, `S3`, `S4`, `S5`, `S6` or `S7`.
  `class` names the fixture's behavior. `census` is `C1`, `C2`, `C3`,
  `C4`, `C6` or `none`.
- `expect` gives the assertion for a negative row. `no_route` rules out
  all routes. `no_exact_route` rules out routes over exact data-flow
  and argument edges and same-owner, same-line use-to-definition
  transfers. `expect_if_not` records the negative assertion for rows
  whose outcome the seed table leaves to the runner.
- `base_verdict` pins the line-seeded query's verdict on a control.
  R01.js and R01.py returned `SymbolNotFound` for the sink seed, so the
  controls have no verdict to record and fail the coverage gate.
- `label` checks the W-to-R edge by bytes. Values are `exact`, `killed`
  and `nonexact`, or a mapping of alias policies to those labels.
  Stage D selects the active policy and finalizes the policy-dependent
  rows before they are enabled.
- `pinned_main` takes precedence over the truth and label assertions.
  It records a verdict and reason, and a route where specified. A route
  can be `exact`, `nonexact`, `none` or `missing_sink_use`. The last
  value records a base observation and still fails the sink-use gate.
  `accepted_loss` records a reason and issue, while
  `accepted_loss_candidate` marks an undecided exception.
- `src_line`, `w_marker_line`, `r_marker_line` and `sink_line` are
  1-based within the case. `src_bytes`, `w_bytes`, `r_bytes` and
  `sink_bytes` are half-open UTF-8 byte ranges for the source target,
  marked write, marked read and complete sink argument. The generator
  computes them from the authored occurrence and emitted source.

## Case rules

Start `case()` in Python or `caseFn()` in JavaScript, TypeScript or Go
with an assignment of `src()` to `t`. Go requires the preceding
`package main` declaration. Call `sink(...)` once. Helpers can contain
that call, so its line can differ from the R marker. W and R are trailing
comments on the relevant write and read lines. Use one statement per line
with intentional multiline literals and expressions where specified.

Seed the read keys with safe values. Use typed maps in Go and erasable
TypeScript syntax. The cases contain their own helpers, import nothing,
and use the runner's prelude for `src()` and `sink(...)`. See the
[truth runner README](truth/README.md) for containment and execution.

`rows.jsonl` has no truth field. The runner writes `truth/truth.jsonl`.
C01.c, C01.java and C01.rs are analysis controls with `n/a` truth.
`subscript_matrix_rows` reads the receipt and starts with `control`
enabled. Missing complete sink-argument uses fail even on pinned rows.

## Not expressible

Every language assigned by the seed table has a case. Go requires a
trailing comma and a separate closing brace for P18's multiline map.
I04's Go expression closes on the same line as `t` because a bare
identifier at the end of the line would insert a semicolon.

# Stage T draft baseline

The reproduction command was `cargo test --test ast subscript_matrix::`.

Run before expectation edits on parent `757a6e177d9d4324657dd977f0b3ae87bd6f5f7b` with
the matrix overlay byte-identical to custody commit `1dc74573e`. The run ended
with 7 passing route self-tests, 2 failures and 598 filtered out.

| Draft input | SHA-256 |
|---|---|
| `rows.jsonl` | `6104e464069448b63d953e1183cd3c22f83006cbea85f55b3e891ed3942e32fe` |
| `truth/truth.jsonl` | `743444473389d7028c13f2fbd9a3da428a4af3174f674a0a6eed8b86ab5199cd` |

## Move verification

Before and immediately after the move, the manifest contained 229 files,
including all 211 cases. Each manifest line is the lowercase SHA-256, two
spaces, the path relative to the matrix root, and a newline. Lines are in
path order. The manifests compared byte for byte and both had SHA-256
`b63bc83d639152912c9001dc869b0b261b3b43d6fefd04f8485a9c5796415b54`.

## Named failures

`subscript_matrix_rows_have_truth` failed only for `R01.js` and `R01.py`.
Both have executed truth. The historical query returned `SymbolNotFound`
for the sink, so neither has a base verdict.

`subscript_matrix_rows` failed for 46 of 69 controls. The final column gives
the stage T disposition. Original truth comes from the draft receipt.

| Row | Draft failure | Draft truth | Disposition |
|---|---|---|---|
| `A01.js` | truth reaches, taint_reaches returned BoundaryExited | `reaches` | regroup → `baseline-gap` |
| `C01.java` | missing sink argument Use at case.java:[131, 135] | `n/a` | regroup → `baseline-gap` |
| `I03.py` | missing sink argument Use at case.py:[86, 92] | `reaches` | drop |
| `K02.py` | missing sink argument Use at case.py:[109, 115] | `not` | drop |
| `K05.js` | missing sink argument Use at case.js:[135, 143] | `not` | drop |
| `K12.py` | missing sink argument Use at case.py:[103, 109] | `not` | drop |
| `M04.js` | missing sink argument Use at case.js:[104, 110] | `not` | regroup → `baseline-gap` |
| `O01.js` | missing sink argument Use at case.js:[124, 130] | `not` | rebuild → `recall-js` |
| `O01.py` | missing sink argument Use at case.py:[99, 105] | `not` | rebuild → `recall-py` |
| `O02.py` | missing sink argument Use at case.py:[66, 72] | `reaches` | drop |
| `O04.py` | missing sink argument Use at case.py:[104, 115] | `reaches` | drop |
| `P01.go` | missing sink argument Use at case.go:[142, 148] | `not` | rebuild → `boundary` |
| `P01.js` | missing sink argument Use at case.js:[130, 136] | `not` | rebuild → `boundary` |
| `P01.py` | missing sink argument Use at case.py:[105, 111] | `not` | rebuild → `boundary` |
| `P03.go` | missing sink argument Use at case.go:[143, 149] | `not` | rebuild → `recall-go` |
| `P03.js` | missing sink argument Use at case.js:[131, 137] | `not` | rebuild → `recall-js` |
| `P03.py` | missing sink argument Use at case.py:[106, 112] | `not` | rebuild → `recall-py` |
| `P04.js` | missing sink argument Use at case.js:[147, 158] | `not` | drop |
| `P04.py` | missing sink argument Use at case.py:[122, 133] | `not` | drop |
| `P10.go` | missing sink argument Use at case.go:[122, 128] | `not` | drop |
| `P12.js` | missing sink argument Use at case.js:[147, 153] | `not` | drop |
| `P12.py` | missing sink argument Use at case.py:[115, 121] | `not` | drop |
| `P17.go` | missing sink argument Use at case.go:[93, 99] | `reaches` | drop |
| `P17.js` | missing sink argument Use at case.js:[79, 85] | `reaches` | drop |
| `P17.py` | missing sink argument Use at case.py:[56, 62] | `reaches` | drop |
| `P20.go` | missing sink argument Use at case.go:[80, 85] | `reaches` | drop |
| `P20.js` | missing sink argument Use at case.js:[75, 80] | `reaches` | drop |
| `P20.py` | missing sink argument Use at case.py:[52, 57] | `reaches` | drop |
| `P26.py` | unexpected all-exact route | `not` | regroup → `baseline-gap` |
| `P30.py` | no route through W at [30, 31] | `reaches` | regroup → `baseline-gap` |
| `R01.js` | missing sink argument Use at case.js:[93, 104] | `not` | drop |
| `R01.py` | missing sink argument Use at case.py:[69, 80] | `not` | drop |
| `X04.py` | missing sink argument Use at case.py:[106, 112] | `not` | drop |
| `Y02.go` | missing sink argument Use at case.go:[133, 139] | `not` | drop |
| `Y02.js` | missing sink argument Use at case.js:[126, 132] | `not` | drop |
| `Y02.py` | missing sink argument Use at case.py:[95, 101] | `not` | drop |
| `Y04.go` | missing sink argument Use at case.go:[135, 139] | `not` | drop |
| `Y04.js` | missing sink argument Use at case.js:[128, 132] | `not` | drop |
| `Y04.py` | missing sink argument Use at case.py:[97, 101] | `not` | drop |
| `Y07.py` | missing sink argument Use at case.py:[72, 78] | `not` | drop |
| `Y08.go` | missing sink argument Use at case.go:[103, 108] | `not` | drop |
| `Y08.js` | missing sink argument Use at case.js:[99, 104] | `not` | drop |
| `Y08.py` | missing sink argument Use at case.py:[75, 80] | `not` | drop |
| `Y09.go` | missing sink argument Use at case.go:[103, 108] | `reaches` | rebuild → `recall-go` |
| `Y09.js` | missing sink argument Use at case.js:[99, 104] | `reaches` | rebuild → `recall-js` |
| `Y09.py` | missing sink argument Use at case.py:[75, 80] | `reaches` | rebuild → `recall-py` |

## Stage T-a verification

`cargo test --test ast subscript_matrix::` passed all 11 tests with 598
filtered out. The run covered 17 controls, eight baseline gaps and 26 boundary
rows, including the seven inherited route tests and two new tests for missing
sink Uses and forbidden edges. The 29 recall rows remain disabled. The
`9d1a7d06f653` historical binary reproduced the line-seeded verdicts for all
80 retained rows and bound 63 distinct W/R/sink marks across the recall rows.
The recall publication and query-gain assertions remain unverified.

Regeneration and the native truth runner produced byte-identical results from
the repository root and the matrix directory. The truth checker passed from
both directories, and the truth runner's nine Python unit tests passed.
Python 3.12.13, Node 24.15.0 and Go 1.26.2 observed 28 `reaches` and 15
`not` results for the 43 rebuilt programs. All 37 unchanged truth records
and the five historical spelling files matched the draft bytes. The resulting
matrix has 46 `reaches`, 31 `not` and three structural `n/a` records. Both
Y12 observations are `not`.

`cargo fmt --all --check` passed. The full suite and mutgate were outside
this slice. Full-suite verification remains pending before merge.

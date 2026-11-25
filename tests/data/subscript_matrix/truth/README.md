# Executed truth for the subscript matrix

Put Node 24.15.0 and Go 1.26.2 on `PATH`. From the repository root:

```sh
python3 -B tests/data/subscript_matrix/truth/run_truth.py --out /tmp/subscript-truth.jsonl
python3 -B tests/data/subscript_matrix/truth/check_truth.py tests/data/subscript_matrix/rows.jsonl /tmp/subscript-truth.jsonl
python3 -B -m unittest discover -s tests/data/subscript_matrix/truth
```

From the matrix directory:

```sh
python3 -B truth/run_truth.py --out /tmp/subscript-truth.jsonl
truth/check.sh rows.jsonl /tmp/subscript-truth.jsonl
python3 -B -m unittest discover -s truth
```

Defaults are resolved from the scripts' location. The runner accepts
`[cases [rows]] [--out PATH]` and defaults to writing `truth/truth.jsonl`.
The checker accepts `[rows [truth]] [--cases DIR]` and defaults to the
committed receipt and the case directory beside the row index. It checks
coverage, record order, allowed observations and SHA-256 binding for
rebuilt cases.

Cases define `case()` in Python, `caseFn()` in JavaScript or TypeScript, or
`func caseFn()` in Go's `package main`. Start by assigning `src()` to `t`. Call
`sink(value)` once. The sink looks for the substring `TAINT-7f3a`, traverses
nested containers and fields, and tracks visited references through cycles.
The runner concatenates the script preludes with the cases. JavaScript runs
as ESM and TypeScript runs directly as `.ts` with native type stripping. Go
runs the case and prelude together in a temporary module.

The runner resolves Python 3.12 with offline `uv run --python 3.12 python` and
executes that binary directly, or reports and uses `python3` if unavailable.
Each executable row gets its own process, working directory, home directory and
10 second timeout. It inherits `PATH` and gets a temporary `HOME`. Go also gets
`GOFLAGS=-mod=mod`, `CGO_ENABLED=0`, `GOTOOLCHAIN=local` and a runner-local cache
with standard-library packages prepared before the first timed Go execution.
Go telemetry is disabled inside the temporary homes.

Preflight checks every row's fixture and all three preludes before execution.
Cases cannot import modules or packages, including via JavaScript `require()`.
Python disallows `open`, `exec`, `eval`, `compile`, `__import__`, `__builtins__`,
`breakpoint` and `input`. JavaScript and TypeScript disallow `process`,
`globalThis`, `fetch`, `eval` and `Function`. Comments and literal text are
ignored. Template and f-string expressions are checked as code. The Python and
JavaScript preludes import nothing. Go's prelude imports `fmt`, `reflect` and
`strings`. Use reviewed, lane-authored fixtures.

Row truth starts with runtime versions, followed by observations in row order.
An error, timeout or missing or repeated observation gives `inadmissible`.
Rust, Java and C rows get `n/a`.

# Executed truth for the subscript matrix

In each spelling file, write one key expression per line as it appears between
brackets. Blank lines and comments starting with `#` are ignored. A
`# setup: <statement>` directive gives the next expression local setup, for
example `# setup: p = "a"` before Python `f"{p}"`. For JavaScript, a plain
quoted or template identifier automatically gets a dot read. Use
`# dot: <identifier>` for escapes and computed identifiers. The runtime checks
that the expression evaluates to that property name and then reads it with dot
syntax. Each directive applies once. Spellings must be unique within a file.

Run from the matrix fixture directory with Node 24.15.0 and Go 1.26.2 on `PATH`.

```sh
python3 truth/run_truth.py cases rows.jsonl
python3 truth/run_truth.py --spellings spellings
truth/check.sh
python3 -m unittest discover -s truth
```

Use `--out PATH` on either runner command to change its receipt path.
`check.sh [rows [truth [spelling_truth]]] [--spellings DIR]` checks receipt
coverage, duplicate records and truth values. It reports every null spelling
pair and fails if a language has more than 2% nulls or either form of an
identical spelling pair is null.

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
Rust, Java and C rows get `n/a`. Spelling truth uses one timed program per
language. It tests every ordered pair on a fresh Python dict, JavaScript
ordinary object or Go `map[any]int` and records failed probes as `null`.

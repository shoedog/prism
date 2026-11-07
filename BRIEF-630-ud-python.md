# #630 slice 5 (Python): rules U and D for Python in the DFG producer
Repository shoedog/prism, base origin/main (a2c9c9f4). Create branch `perf/630-ud-python`. Read: issue #630's design of record (excerpt
below: rules U and D, the Python row of the language table, the Python matrix rows), CLAUDE.md's Go DFG prune paragraphs (U, D, S, O
for Go: what "removal-only", the counters and "cap counts survivors" mean), `src/data_flow/build/lexical_prune.rs` (U and D, today
gated to Go: `enabled: parsed.language == Language::Go`, region walk via `go_statements`), `src/data_flow/build/file.rs` (where they
are called), and the Python statement grammar the producer sees.

## Task
Enable rule U (a use on a plain write-only line of the same path is not a read) and rule D (dominating unconditional direct write in a
single-entry region) for **Python**, per the design. Go behaviour must stay byte-identical. Python regions: `block` (function body,
`if`/`elif`/`else`, `for`/`while` bodies and their `else`, `try`/`except`/`else`/`finally`, `with`, `match` `case` bodies, class
bodies). Unconditional direct write statements: `expression_statement` holding a plain `assignment` whose left side is exactly the path
(a single target; for tuple/list/starred targets only if you prove each element's write — otherwise exclude), and augmented assignment
(writes after reading). Excluded as d*: walrus (`:=`), loop targets (`for x in` — the iterable may be empty), `with .. as`, `except .. as`,
`import`/`from .. import` (unless you justify), annotated declarations without a value (`x: int`), chained assignment unless justified,
defs inside sub-statements, anything in a nested callable (`def`, `lambda`; comprehensions are scopes — treat as nested), alias twins.
No goto in Python, so no entry breakers; but `global`/`nonlocal` names: every occurrence in the function is the same binding, and a
nested `def` with `nonlocal x` or a call to any function may write a `global`/`nonlocal` name between d* and u — so D must NOT drop
pairs for names declared `global` or `nonlocal` in the owner (keep them), and nested-callable defs are always kept. `del x`: ends flow;
do not treat it as d* (keep). Field paths: identical paths only (`p.x` never dominates `p` or vice versa).
U for Python: `x = 1` is write-only; `x += 1`, `x = x + 1`, `a, b = b, a`, `x[i] = 1` (reads x), `x.f = 1` (reads x) are not write-only for x.
Count in the existing `dfg_pruned_write_only` / `dfg_pruned_dominated` counters. Removal-only; uncertain cases keep.

## Test-first (design §6 Python row + U row 0)
RED tests (fail on main for every drop row): function-scope reassign chain; `for x in []` target not dominating; `global x` chain kept;
`nonlocal` writer in a nested def before d* kept; `try: x = f() except: use(x)` with `x = 1` before → both kept; `with f() as x` NOT
dominating (excluded); walrus in a condition not dominating; class body assignment not dominating for a method's read; `match` case
bodies as regions; `del x` then def; write-only line rows (U); tuple-unpacking rows; same-line `x = x + 1`; a use in a nested
callable kept. Cross-cutting: `edges_after ⊆ edges_before` over the Python fixtures; Go fixtures byte-identical. RAYON_NUM_THREADS=1 and 4.
CACHE_VERSION: main's value + 1 (doc line, pins, CLAUDE.md cache sentence). CLAUDE.md: a Python sentence/paragraph beside the Go U/D text.

## Measure (release main vs head; corpora via REST `gh api repos/shoedog/prism/releases/assets/<id> -H 'Accept: application/octet-stream'`,
verify against SHA256SUMS asset 715288388; read the CORPORA.md asset 715288423 to find the Python corpora: django 688449716 plus any other
Python corpus listed there; vitess 699857148 as the non-Python control (must be byte-identical); one at a time; push the report after each)
`nav --no-cache dfg-stats --edges` and `call-stats` (4 threads). `add2.py` both directions. Report: removed/added (added must be 0),
kept-label transitions (must be 0), removed by label and by rule (U/D) and shape, counters, call-stats identical outside `dfg_labels`,
vitess identical, top-20 functions by drops. Stratified sample: 30 random removed edges per Python corpus (fixed seed) + 10 from the top-20,
source lines + RIGHT/WRONG (WRONG = any execution, including exceptions, loops, generators, closures/`nonlocal`/`global` writers, can carry
the def's value to the use). One WRONG blocks: fix, re-measure. Timing: django main vs head 2+2 interleaved. Delete raw dumps; never push.

## Do
`cargo fmt`, clippy clean on touched code; file-size rule (new module, e.g. `src/data_flow/build/python_prune.rs`, if a touched file
would pass 1,000 weighted lines; nothing over 2,000 grows). Targeted at RAYON 1 and 4: `--lib data_flow`, `--lib cpg`, `--lib cpg_cache`,
`--test ast`, `--test lang_python` (if it exists; else the python test target), `--test lang_go`, `--test navigation`, `--test
integration`, `--test algo_taxonomy`. Goldens: update only with per-edge justification (as #698 did). No full suite (CI), no mutgate.
Commit in reviewable steps (RED first); push `perf/630-ud-python` (plain push; if your session cannot, push your own branch and say so).
Write `REPORT-630-ud-python.md` (with a draft PR description) and push it incrementally with small files to `test-results/630-ud-python`.
If you stop early push `FAILED.md` there with exactly what is done.

## Rules
Do not open PRs, comment on GitHub, or touch other branches. Never read or print dotfiles or credentials. Do not open the `eval/secbench`
directory and exclude it from every recursive search. Static analysis only: never execute Python code or packages; prism only parses corpora.

---
## Design of record excerpt (issue #630)
## 2. The rules

Notation: owner function F; a *region* R is a node of a language's region kinds (§3) whose statements
are its direct children and which control enters only at its first statement (single-entry; §3 lists
the constructs that break this per language and disable D for that region). For a use occurrence u,
S_R(u) is the direct child of R that contains u. All positions are bytes.

### 2.0 Rule U, the use must be a read
A non-JS use is line-anchored (§1.1): `scoped_refs` returns every line holding an identifier `x`, and
`get_use` mints a zero-width `Use(x)` at the line start when no rvalue span of `x` exists on the line.
Drop (d, u) when u is such an anchor **and** every occurrence of `x` on that line inside F lies within a
plain lvalue span of exactly path p: an `=`/`:=` assignment, a declaration with initializer, or a Go
`var` (operator field checked per language: Go `assignment_statement.operator == "="`; Python not
`augmented_assignment`; JS not `augmented_assignment_expression`/`update_expression`; Rust not
`compound_assignment_expr`; C/C++/Java `assignment_expression` with `=`). Any occurrence not covered
by such a span (a read the rvalue query missed, an augmented write, `x++`, an occurrence inside a
different path's lvalue such as `yyVAL` in `yyVAL.union = …`, a nested callable) → `Keep`. Applied
before `get_use`, so the anchor `Use` is not minted at all (contract effect in §5). Member paths behave
the same (`yyVAL.union = yyLOCAL` lines pair forward with every later such line today, §9).
Sound by definition: a write statement reads nothing of the path it plainly overwrites; the alias twin
of a write is a write too. JS synthetic owners, member-only and object-literal projections already
carry exact read spans and are untouched.

### 2.2 Rule D, dominating redefinition (overwrite)
For direct use u (not in a nested callable) of path p with binding b, walk regions R_0 ⊂ … ⊂ R_k = body
of F. In R_i, d*(R_i) = the last def of (p, b) that is an **unconditional direct write statement** that is
a direct child of R_i textually before S_{R_i}(u) (not S itself). Let i* be the innermost i with a d*
and no *entry breaker* (§3) between d* and S_{R_i*}(u) inclusive. Then every def d of p is dropped unless
d ∈ {d*} ∪ { d : d*.end < d.start < S_{R_i*}(u).end } ∪ { d inside a nested callable } ∪ { alias-derived
twin defs inside that same range }.

Unconditional direct write statement: `assignment_statement`/`expression_statement(assignment)` whose
direct lvalue is p (not an assignment nested in a larger expression: `a = b || (x = 1)`, walrus,
C/JS comma/conditional operands), augmented assignment (writes p after reading it), declaration **with
initializer** or Go `var x T` (zero value), Go `:=`. Excluded: JS `var x;`, C/C++ `int x;`, Rust
`let x;`, loop targets (`for x in …`, `for (x of …)`, `range`), pattern/`catch`/`except as`/`with as`
targets, `if`/`switch` init statements, defs inside sub-statements of R, alias-derived twins, anything
in a nested callable.

Why it is sound without a CFG: every path to u enters R_i* at its top (single entry) and then executes
R_i*'s direct children in order until S(u); d* is one of them, before S(u), unconditional, and writes the
storage b. So on every path the last write to b before S(u) starts is d* or a write inside (d*, S(u)].
A def before d* in R, outside R, or after S(u) in R is overwritten before u on every path. Defs after u
but inside S(u) are kept because an inner loop in S(u) can carry them to u. Enclosing loops re-enter R at
the top, so they do not resurrect anything. Exceptions/panics: a throw between d* and u leaves R; a
handler is outside R or is a sub-statement containing u (then the defs in the `try` body are inside
(d*, S(u)] and kept). `defer`/`finally` bodies: closures are nested callables (kept); a `finally` block
is a region of its own. Goroutines/async/threads only run closures (kept) or other functions (out of
the intraprocedural edge set anyway). Runtime mutation through aliases, `eval`, reflection, Proxies,
setters, Bash `source` is out of model (CLAUDE.md "Exact is a static-binding grade"); D drops only pairs
whose own storage was provably overwritten, so an unmodeled extra writer never resurrects a dropped d'.
Field paths: D compares identical access paths only (`p.x` never dominates `p`, `p` never dominates
`p.x`); conservative.

Same statement: `x = x + 1` — d and u in one statement, d is not before S(u), so it is never d* for its
own read (the producer already skips `ref_line == def_line`).

Captured writers: `f := func() { x = 9 }; x = 2; f(); use(x)` reads 9 — the closure def is textually
before d* and must survive; hence "d inside a nested callable" is always kept, wherever it is.

## 3. Per-language binding and region tables

| Language | Binding facts today (`scope.rs`) | S | Regions for D (direct-children statement nodes) | Entry breakers (disable D in that region) / O disablers | Notes |
|---|---|---|---|---|---|
| Go | `block`, `if`, `for`, switch/select create scope; `:=` reuses in-scope binding (`visible_from` = stmt end); `var`/`const` new | yes | `block`, `expression_case`, `type_case`, `communication_case`, `default_case` | label as direct child of R in (d*, S(u)] (a `goto` from before d* may jump over it); O off when F has `goto` | `goto` cannot enter a block (spec); `fallthrough` enters next case at its top; `defer`/`go` take closures or evaluated args |
| Python | function scope `ImplicitFunction`; comprehensions, `lambda`, `class` scope | yes (comprehension/class only) | `block` (function body, `if`/`for`/`while`/`try`/`except`/`finally`/`with`/`match case` bodies) | none (no goto); O off never | `global`/`nonlocal`: same binding for all occurrences in F; `del x` ends flow (no resurrection); loop targets excluded as d* (empty iterable) |
## 6. Test-first variant matrix

Fixtures under `tests/fixtures/dfg_prune/<lang>/<case>.<ext>` plus `expected.jsonl` (one row per
pair: `path, def_line, use_line, keep|drop, reason`), driven by one table test per language in
`tests/ast/dfg_prune_test.rs` that builds the CPG and compares the **full** edge set (not only drops).
Each row below is one fixture; `→` lists the expected surviving defs for the marked use. A fixture is

Python: function-scope reassign chain; `for x in []` target not dominating; comprehension var split (S);
`global x` chain; `try: x = f() except: use(x)` with `x = 1` before → both; `with f() as x` dominates;
walrus in condition not dominating; nested `def` with `nonlocal x` writer before d* → kept; class body
assignment not dominating; `match case` bodies as regions; `del x` then def.

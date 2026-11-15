# xarm: the expansion-arm oracle for the U/D prune rows (#829)

Design of record: `docs/superpowers/plans/2026-11-14-829-macro-brackets/TEST-FORM.md` (sections 1, 4, 5, 6)
and `DECISIONS.md`. clang is an AUTHORING-time tool: the Rust tests read frozen data and need no clang.

## What it answers

For a pair `x@def -> x@use` of a fixture: does the lexical U/D proof still hold on what the
compiler sees? Four prism runs per fixture: T (source), Tb (source plus a trigraph-directive file,
which sets the census `incomplete` and turns U and D off: the baseline with no code change), X
(each main file replaced by clang's post-expansion token stream, macro free, source tokens on their
own lines) and Xb (X plus the poison file). Verdicts describe the pair on the compiler's tokens:

| verdict | meaning | assertion |
|---|---|---|
| CONTRADICTED | the rules keep the pair on X with its owner on: the proof fails once macros expand | must-keep |
| CONFIRMED | the rules drop the pair on X | may-drop (guide) |
| XOFF | kept on X only because its owner is off there (invalid program, parse error) | no basis |
| UNJUDGED | the pair does not exist on X, or the program is invalid (a macro-arity error, an unreadable header) | no basis |

Two runs per row: repo-only (what the repository holds) and declared (the row's configuration:
flags, prelude headers outside the repository, forced includes, dialect). A row is BOUNDARY when
the verdict flips from CONFIRMED to CONTRADICTED under the declared configuration, or when clang
opens an in-repo file the census does not read (`-H`; the census input set is an approximation:
C-like extensions plus literal include closure, `eval/secbench` excluded). Assumptions: A-ext
(outside macro acts as a function, D2), A-inputs (#813), A-config, A-dialect, A-encoding.

## Derived assertions (`derive` in `xarm.py`)

- `kept`: not BOUNDARY and CONTRADICTED; also BOUNDARY with A-inputs when prism sees the input
  (a census text or an orphan header the compiler gets as a forced include) or head already keeps.
- `pinned_hole`: BOUNDARY with A-inputs that prism cannot see, today's drop asserted (so #813
  flips it visibly); for A-ext, A-config, A-dialect, A-encoding one witness each, the rest `guide`.
- `dropped`: a validity control, a pruned-expected row whose pair a must-keep twin of the same
  test module holds with a superset of its files (the twin is red for the right reason only if the
  control prunes).
- `guide`: CONFIRMED, other controls, A-ext rows. Printed as `GUIDE ...` on stderr, never asserted.
- `no_basis`: XOFF, UNJUDGED, and `NO_BASIS` by decision (mf3, the header-unit import).

Orphan headers (a `.h`, `.hpp`, `.hh`, `.hxx` or `.inc` no main file includes) are forced includes:
the fixture author meant the header to be part of the translation unit (the census reads every
in-repo header). Outside names (`pthread_cleanup_*`, `Py_*`, `NPY_*`, `PG_TRY`, `_Py_*_SUPPRESS_IPH`)
get `-include pthread.h` or a prelude reproduced from the named version's headers.

## Workflow

```bash
cargo build --release                       # xarm uses target/release/prism (or $PRISM)
export PRISM_829_EXPORT_DIR=/tmp/xarm-export # fresh directory
cargo test -p prism --lib -- c_prune_brackets                      # pair rows -> <dir>/rows/<key>/
cargo test -p prism --lib -- export_inline_cases --ignored         # inline cases -> <dir>/inline/
unset PRISM_829_EXPORT_DIR
python3 scripts/dfg_prune/xarm.py label /tmp/xarm-export --jobs 6 --work /tmp/xarm-work
python3 tests/fixtures/dfg_prune/c/gen_obligations.py              # merges the inline rows
python3 tests/fixtures/dfg_prune/cpp/gen_obligations.py
python3 scripts/dfg_prune/xarm_receipts.py build /tmp/xarm-export   # optional
python3 scripts/dfg_prune/oracle_report.py                  # regenerates the TESTS.md block
```

In export mode the row tests assert nothing (`c_prune_oracle_rows::judge` writes the row and
returns). Rows are keyed by a content hash (files, links, pair; the tempdir root is normalized), so
a changed fixture reports `unlabeled row <key>`: re-run the workflow. `label` takes about 90 s for
about 390 rows at `--jobs 6`.

Data (under `tests/fixtures/dfg_prune/oracle/`): `pair_rows.jsonl` (Rust-built rows), `inline_rows.jsonl` (merged into the
`obligations.jsonl` rows as `oracle`, `oracle_repo`, `oracle_declared`, `config`, `assumption`,
`boundary`, `inputs_gap`, `head_drops`, `basis`, `assert`, `validity_control`, `seen_input`),
`receipts.jsonl`, `legacy_kinds.json`. The scripts live in `scripts/dfg_prune` (not under `tests/fixtures`, whose Python files the alias-twin receipt sweep parses); no data file has a C-like extension, and fixtures
with macros stay Rust strings: the census of an index of the prism repository must not read them.

## Other commands

- `xarm.py corpus REPO OUT [--flags ...]`: sweep a corpus. Every pair the rules drop on REPO is
  judged against the expansion (missing includes stubbed empty, in-repo directories on `-idirafter`).
  Output: verdict counts and `OUT/rows.tsv`. (L3 corpus differentials build on it.)
- `xarm_receipts.py run`: compiles and runs 15 receipts (one per class): `owner(0)` returns the
  first def's value, so the pair is live at run time. Not run in CI. Needs clang.

## Limits (disclosed)

One host configuration; code under other `#if` branches is unjudged. The lexer-only action does not
expand header-unit imports or read UTF-16 files (those rows are `no_basis`). Edges from the CLI carry
no owner, so a pair judged by the CLI is owner-blind (pair rows also carry the in-test, owner-aware
head result; the two agreed on all 284 rows). The X arm trips the phantom-declaration gate on
`__builtin_offsetof` (redis `dict.c`): oracle noise on corpora, not on these fixtures.

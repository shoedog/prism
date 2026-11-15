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

Forced includes. A header-borne row's owner file `#include`s the header explicitly (that is the
author's intent: `include_first` in `c_prune_brackets_r3_support.rs`), so the compiler needs no
forced include for it; "orphan" is decided by what clang opens (`-H`), not by a regex, so a
computed `#include HEADER` counts as reached. A forced include remains only where a row models
one: a net file (`defs.inc`, a C-like extension nothing includes, collected by the census without
an include: `g_sol_14`, `g2_sol2_04`) gets `-include`. Outside names (`pthread_cleanup_*`, `Py_*`,
`NPY_*`, `PG_TRY`, `_Py_*_SUPPRESS_IPH`) get `-include pthread.h` or a prelude reproduced from the
named version's headers, unless the repository defines the name itself.

Receipt overrides (`oracle/overrides.json`, `xarm.py derive`): when the oracle cannot judge a row
only because the post-expansion text mis-parses in tree-sitter (UNJUDGED `no-pair-in-expansion` or
XOFF `G1Error`, e.g. cpp rows x2 to x4 of `20_macro_brackets.cc`, where `for (; int *x = nullptr;)`
is mis-read), a runtime receipt (`xarm_receipts.py`, the program's own driver exits 0 exactly when
the pair is live) outranks the verdict and sets `assert: kept` with `basis: receipt`. A receipt
never outranks CONFIRMED or CONTRADICTED.

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

## The generator (L2, `xgen.py`, offline, not CI)

```bash
python3 scripts/dfg_prune/xgen.py run --out DIR --bound 3 --jobs 8      # exhaustive at bound 3
python3 scripts/dfg_prune/xgen.py run --out DIR2 --bound 4 --sample 150000 --seed 11
python3 scripts/dfg_prune/xgen.py keys DIR [DIR2 ...]                    # keys and counts
python3 scripts/dfg_prune/xgen.py freeze DIR [DIR2 ...] --out CANDS      # minimal member per key
python3 scripts/dfg_prune/xgen.py verify CANDS --out tests/fixtures/dfg_prune/oracle/generated_rows.jsonl
```

A program is the guard-shaped owner `probe` (`int x = 41; <site 1> x = 2; <site 2> return x;`,
pair `x@def -> x@return`) over: a site family (statement, guard, initializer, declarator,
variadic argument, argument, and four enclosing-group forms: the group opened at file scope, in a
sibling owner, in a declaration, closed in a following owner), an owner kind (C function, C++
function, template, method, local-class method), a macro kind (object, `A()`, alias chain),
macro bodies, a lexical variant (plain, splice between fragments, splice inside a token, comments
holding brackets, CRLF) and a definition channel (same file, header, census text). Bodies are
sequences of fragments (the three bracket kinds, digraphs, `,`, `;`, `if (c)`, `else`, `do`,
`while (0)`, `for (;;)`, declarations, `x = 3`, `c`; the enclosing macro `M(p, q, r)` adds the
parameters, `p;`, `q;` and small wrappers). BOUND K is the total number of body fragments over
the (at most two) macros. Bound K is enumerated exhaustively for the plain same-file form; a
larger bound is drawn at random with a fixed seed (`--sample`, `--seed`). The other lexical
variants and channels are applied to up to `--var-per-key` minimal contradicted programs of every
key and to a seeded sample (`--var-sample`) of the others.

Pipeline: render, a necessary-condition prefilter (bracket nesting of `{` and `(` after expansion
of the object macros and identity wrappers; `[` is ignored because clang accepts some
unterminated `[[` forms), `clang -fsyntax-only` (only programs clang accepts count), then the
oracle in batches of 400 programs per repository (names carry a per-program suffix; a batch whose
census is `Incomplete` is bisected). Verdicts: `CONTRADICTED` needs head to drop the pair (T below
Tb) and the compiler's tokens to keep it with the owner on; `NODROP` (head keeps it already),
`CONFIRMED`, `XOFF`, `UNJUDGED`, `INCOMPLETE` are not findings.

KEY = (effect classes, site family + macro kind, owner kind). Effect classes of the macro bodies:
`BR_UNBAL` (brackets that do not nest within the body), `BR_BAL`, `ARGSPLIT` (a comma outside
parentheses), `SEMI0` (a `;` outside every bracket: statement boundary, **tagged #831 and excluded
from this lane's gate and from the frozen rows**), `CTRL` (a control fragment), `WRITE` (a body
that writes or declares `x`), `PARAMS` (a parameter dropped, duplicated or reordered). A lexical
or channel variant of a program whose key is already contradicted plain counts toward that key;
a variant-only contradiction is a key of its own (suffixed with the variant). `freeze` writes the
minimal member of every key (fewest fragments, plain and same file first) to
`oracle/generated_rows.jsonl` (`source: generator`, `assert: kept`) after re-judging it alone with
the full oracle (`xgen.py verify`; a candidate that is not CONTRADICTED alone, or whose census inputs gap is non-empty, is dropped and listed); the Rust tests `c_prune_generated_rows` (one per family) read
them. The generator's own batches are not re-run in CI.

Exit criterion 3 of TEST-FORM: bound k fully judged, and bound k+1 (or two further batches) adds
no new key. The batches, their sizes and the keys they found are in `L3-BASE.md` (generator
section).

## L3 corpus tool (`xarm_l3.py`)

```bash
python3 scripts/dfg_prune/xarm_l3.py corpora
python3 scripts/dfg_prune/xarm_l3.py sweep redis --out .scratch829/l3/redis --tag base
python3 scripts/dfg_prune/xarm_l3.py compare .scratch829/l3/redis --base base --head fix
python3 scripts/dfg_prune/xarm_l3.py parity-langs ~/code/bench-repos/click ... --base-bin B --head-bin H --out DIR
python3 scripts/dfg_prune/xarm_l3.py cache-modes --repo ~/code/bench-repos/redis/src --prism BIN --out DIR
```

Corpora are read-only and are never built or run (`l3_corpora.json`: root, scope = the directories
whose files are tokenized, flags, config files). The files the census reads are copied to
`DIR/T` and `DIR/Tb` (plus the census-poison file), clang only lexes and preprocesses, outside
headers that do not exist are stubbed empty (one `-M -MG` pass names them), every in-repo header
directory goes on `-idirafter`. A corpus config file (cpython's `pyconfig.h`) is placed under
`DIR/config`, never inside the corpus; its source path and sha256 are recorded in the summary.

- D2 `sweep`: every pair the rules drop on the corpus (T below Tb) gets a verdict. The triage is
  keyed by the X arm's refusal reason at the pair's use line (the `c-prune-use` trace of the X
  run) for CONTRADICTED pairs, the owner's off reason for XOFF, and the cause for UNJUDGED
  (tokenize failure, no pair in the expansion, out of scope). Two X-arm normalizations remove
  oracle noise that no macro of the program caused: the source spelling of `offsetof`,
  `va_start`, `va_arg`, `va_end` and `va_copy` is restored (the headers substitute
  `__builtin_*`, which makes a type operand an identifier of the file in the X arm only), and
  an expanded `__attribute__((fallthrough));` becomes `;` (tree-sitter-c does not parse it in a
  `switch`). Known residual X-arm noise (XOFF `off:G1Error`, never CONTRADICTED): abseil's
  `switch (0) case 0: default: ...` log and check macros, and `sizeof(struct { int : ...; })`
  static assertions (`Py_ARRAY_LENGTH`), which tree-sitter does not parse.
- D3 `compare`: owners that were on under the base binary and are off under the head, grouped by
  the head's off reason; pairs the base drops and the head keeps (restored), grouped by the head's
  reason and split by the base's verdict: CONFIRMED (pure recall cost), CONTRADICTED (a correct
  restoration), XOFF, UNJUDGED. Run `sweep` once per binary into the same DIR (the expansion text
  is shared), then `compare`.
- D1 (same command): every edge of the base binary exists in the head's output with at least its
  multiplicity (monotonicity); counts the edges the head adds.
- D4 `parity-langs`: the edge-set digest of repositories of other languages is identical under
  both binaries (and across two base runs: determinism). `cache-modes`: cold, full-hit, a source
  edit and a header edit against the no-cache build of the same tree.
- Self-test of `compare`: using the base `Tb` run as the head's `T` restores every dropped pair.

## L4 design scoring (`xscore.py`, offline, no prism, no clang)

```bash
python3 scripts/dfg_prune/xscore.py selftest
python3 scripts/dfg_prune/xscore.py run --out .scratch829/l4            # L1 + L2 (k3) + L3, about 11 s
python3 scripts/dfg_prune/xscore.py run --pops l2 --l2dirs .scratch829/gen/k4a .scratch829/gen/k4b --out DIR
python3 scripts/dfg_prune/xscore.py report .scratch829/l4 [--md | --violations --pops L1-pair --classes MUSTKEEP]
python3 scripts/dfg_prune/xscore.py run --candidates my_candidates.py --out DIR
```

A candidate design is an OWNER-REFUSAL PREDICATE, `predicate(owner, db) -> None | reason`
(`xscore_candidates.py`, registered in `CANDIDATES`): None admits the owner (its pairs are as on the
T arm), a reason refuses it (its pairs are as on the Tb baseline arm). The score reads the saved
oracle results, so it needs neither prism nor clang: every head-dropped pair of the three
populations has a class (MUSTKEEP: CONTRADICTED or a `kept` row; PINNED: a pinned hole; BOUNDARY: an
A-ext guide row; CONFIRMED: recall; UNJUDGED: reported apart; the #831 programs of the generator,
whose macro body has a top-level `;`, are MUSTKEEP831) and the predicate's decision on the pair's
owner is counted per class: MUSTKEEP admitted = a STRICT violation, CONFIRMED refused = recall cost.

Populations: L1 (the frozen rows: pair rows from `--l1-export`, the Rust-test export, which must
hold every key of `pair_rows.jsonl`, inline rows from its `inline/` files, generator rows), L2 (the
`results.jsonl` of generator batches, re-rendered from their specs through `xgen.Prog`, so lexical
variants and channels are included), L3 (the `rows.tsv` and `T.json.gz` of each corpus' `base` sweep;
sources are re-read from the pinned checkouts of `l3_corpora.json`).

What a predicate sees (`xscore_lex.py`): the owner's tokens, spans and identifiers (`owner.spelled`),
`owner.enclosing_open` (an unclosed `(` before the owner, or a `)` after it that nothing in the file
opens), `owner.invocations(db)` (in-repo macro names followed by `(` and their argument groups), and
per-macro facts from the `#define` lines of the census inputs: `db.own(name)` (the definitions of the
name ORed) and `db.closed(name)` (also ORed over the macros the body names): `unbalanced` (bracket
nesting automaton of DESIGN 6.1, digraph brackets set it), `loose_comma`, `semi0`, `semi_any`, `ctrl`,
`brace`, `decl`, `free`, `grouped`; `BLOCK_MACROS` is the list of the 16 names. Never the clang
expansion. The predicates of `xscore_candidates.py` are the candidates of `L4-SCORES.md`
(0 admit-all, 1 tentative, 2 no-closure, 3 spelling-only, 4b with enclosing check, 5 any-macro, 6 and 6b
certificate, 7 depth-1, 8 fine-G8, 9 refuse-all, ablations A1 to A4). The redesign added candidate 10 (the rule of
`DESIGN-v2.md`: `sp_certificate`, with `owner_groups` for the owner pass and `exposed` for the interval
test over `#if` branches), its variants 10x, 10n, 10i, 10f, 10s and its ablations B1 to B8; a body's
paren groups are in `Facts.groups` (`xscore_lex.Group`), and `Facts.paren_open` (a `(` the body does not
close) feeds `hidden_opener`, the third half of the exposed-owner test (owner decision OD-C).

Limits (full list in `L4-SCORES.md`, section 6): the census is emulated (an approximation of the input
set, one lexer pass for defines with no C versus C++ dialect pass, no soft poison, no `incomplete`
handling); owners are located by a token-level rule without a parser and matched to prism's owner by
start line (98 of 5,523 cpython and 207 of 7,502 protobuf `on` owners unmatched, 18 cpython sound pairs not
placed); the open-group test is file-scope parenthesis balance with a first-branch-wins `#if`
heuristic; verdicts are the frozen X-arm verdicts of the base binary under one host configuration.
A design with 0 violations here is not known sound: the data holds known classes only.

## Other commands

- `xarm.py corpus ...` forwards to `xarm_l3.py` (the former sweep is its `sweep`).
- `xarm_receipts.py run`: compiles and runs 18 receipts (the 15 classes plus the three cpp rows
  x2 to x4 with their own drivers): `owner(0)` returns the first def's value, so the pair is live
  at run time. Not run in CI. Needs clang.

## Limits (disclosed)

One host configuration; code under other `#if` branches is unjudged. The lexer-only action does not
expand header-unit imports or read UTF-16 files (those rows are `no_basis`). Edges from the CLI carry
no owner, so a pair judged by the CLI is owner-blind (pair rows also carry the in-test, owner-aware
head result; the two agreed on all 284 rows). The X arm restores the source spelling of `offsetof` and the `va_*` builtins (see L3).

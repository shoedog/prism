# #829 L3 and L2 on base (corpus differentials and the bounded generator)

Base = branch `fix/829-macro-brackets` with the gates still stubs (behaves like main for U and D);
release binary of 2026-11-14 19:05. Tools: `scripts/dfg_prune/xarm_l3.py` (L3) and `xgen.py` (L2);
usage and semantics in `scripts/dfg_prune/XARM.md`. Raw outputs are under `.scratch829/l3/` and
`.scratch829/gen/` (git-excluded); this file holds the summary tables. Decision served: scoring
a design against recall (D3) and the merge gate (D1, D2, D4), and TEST-FORM exit criteria 3 and 4.

## 1. L3 corpus differentials

### Corpora, coverage and configuration

All corpora are read-only checkouts; nothing was built or run (clang lexes and preprocesses only).
The files the census reads are copied to the sweep directory; outside headers that do not exist
are stubbed empty (a `-M -MG` pass names them); every in-repo header directory is on `-idirafter`.

| corpus | commit | scope (translation units) | extra configuration |
|---|---|---|---|
| redis | `d2d3390` | `src/` (133 .c; the census sees all of the repo, `deps/` and `modules/` pairs are "out of scope") | none |
| cpython | `ef1fce0` (3.16-dev) | `Objects/`, `Python/`, `Parser/` (201 .c) | `-DPy_BUILD_CORE`, `-IInclude -IInclude/internal -I.`, and a **`pyconfig.h` from an installed CPython** (see below) |
| leveldb | `7ee830d` | whole repo (77 .cc) | `-I. -Iinclude` |
| protobuf | `9fff46d` | `src/` (319 .cc) | `-Isrc`; Homebrew abseil on `-idirafter` (below) |
| re2 | `6dcd83d` | whole repo (55 .cc) | `-I.`; Homebrew abseil on `-idirafter` |
| snappy | `2c94e11` | whole repo (11 .cc) | `-I.` |

- **cpython `pyconfig.h`**: the checkout's only `pyconfig.h` is the Windows one (`PC/pyconfig.h`), and
  there is no configure output on the host. The sweep uses the configure-generated `pyconfig.h` of
  the installed macOS aarch64 CPython 3.14.4
  (`~/.local/share/uv/python/cpython-3.14.4-macos-aarch64-none/include/python3.14/pyconfig.h`,
  sha256 `ddb1c5e2b60009fd6235838d4ee9b08f7174fc40c6b3764c5b744f1161d521a1`), copied to the
  sweep's `config/` directory (never into the corpus); macros added to 3.16 since are undefined.
  No hand-written stub was needed. Table: `scripts/dfg_prune/l3_corpora.json`.
- **abseil** (protobuf, re2): with the absl headers the corpora name stubbed empty, `ABSL_HAVE_BUILTIN(x)`
  in an `#if` is fatal and 309 of 319 protobuf files failed to tokenize (first attempt). The sweep puts
  the installed Homebrew abseil (`/opt/homebrew/include/absl`, LTS 20260107.1) on `-idirafter`, after
  every in-repo directory. The re2 result changed from "172 CONFIRMED" (stubs) to "38 CONFIRMED +
  142 XOFF" with the real headers (the XOFF pairs are the abseil mechanism below).
- Fix of the consult's offsetof noise: the X arm restores the source spelling of `offsetof` and
  the `va_*` builtins (`__builtin_offsetof(T, f)` is an ordinary call that makes `T` an identifier
  of the file and trips the phantom-declaration gate in the X arm only), and an expanded
  `__attribute__((fallthrough));` becomes `;`. redis: the 182 contradictions of the consult are
  gone (0 now); cpython: 783 pairs (724 of them in `dtoa.c`) moved from XOFF to CONFIRMED.

### D2: oracle sweep of every pair the rules drop

| corpus | mains | tokenized | stubbed hdrs | edges (T) | dropped pairs | by U | by D | CONFIRMED | CONTRADICTED | XOFF | UNJUDGED | out of scope | CONFIRMED spanning a macro name | kept on T, dropped on X |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| snappy | 11 | 11 | 3 | 15,835 | 19 | 19 | 0 | 19 | 0 | 0 | 0 | 0 | 0 | 4 |
| leveldb | 77 | 77 | 5 | 28,915 | 479 | 228 | 251 | 473 | 0 | 0 | 6 | 0 | 318 | 18 |
| re2 | 55 | 55 | 8 | 28,451 | 180 | 176 | 4 | 38 | 0 | 142 | 0 | 0 | 10 | 0 |
| redis | 133 | 133 | 3 | 560,026 | 30,408 | 20,604 | 9,804 | 21,605 | 0 | 0 | 1,820 | 6,983 | 16,380 | 27,111 |
| cpython | 201 | 195 | 35 | 1,105,326 | 28,809 | 21,597 | 7,212 | 15,238 | **50** | 88 | 1,102 | 12,331 | 11,566 | 109,888 |
| protobuf | 319 | 319 | 58 | 1,435,942 | 625 | 597 | 28 | 422 | 0 | 152 | 51 | 0 | 41 | 15 |
| **total** | 796 | 790 | | | 60,520 | 43,221 | 17,299 | 37,795 | **50** | 382 | 2,979 | 19,314 | 28,315 | |

"CONFIRMED spanning a macro name" = CONFIRMED pairs whose def-to-use lines spell an identifier that
is absent from the expansion's lines (a macro name or a keyword the headers rewrite; comments,
strings and directives excluded). It is the cost, as a count of lost recall, of the crudest design
"refuse any owner/pair that touches a macro name": 67% (leveldb), 76% (redis, cpython),
10% (protobuf) and 26% (re2), whose pairs mostly avoid macros. "Kept on T, dropped on X" is an upper bound
of recall headroom (not a sound number: it mixes in owners the rules turn off on T).

Owner states under the rules on base (T): see `xarm_l3.py report`; e.g. redis on 8,228 (237 more on without rule D) / off 2,739
(`G1File` 1,153 headers parsed as C, `G5` 831 poisoned macros, `G1Error` 680, ...); cpython on 10,695
(`G5` 6,076); protobuf on 7,748 (`G1File` 5,374).

### Triage of every non-CONFIRMED verdict

**CONTRADICTED: 50 pairs, all cpython, one mechanism, none live.**

| pairs | macros between def and use | mechanism |
|---|---|---|
| 37 | `Py_BEGIN_CRITICAL_SECTION` / `_END_` | in the default (GIL) build both expand to `{ ... }` |
| 7 | `Py_BEGIN_CRITICAL_SECTION2` / `_END_` | same, two objects |
| 5 | the same pair of macros plus `assert`, `Py_NewRef`, ... in the block | same |
| 1 | `Py_BEGIN_CRITICAL_SECTION2` after an `#ifdef Py_GIL_DISABLED` branch (inactive here) | same |

The shape is `PyObject *ret; Py_BEGIN_CRITICAL_SECTION(self); ret = f(...); Py_END_CRITICAL_SECTION();
return ret;`: the expansion is `{ ; ret = f(...); } ;`, an UNCONDITIONAL block, so the first def
(the declaration) is dead and dropping the pair is correct. The oracle calls it CONTRADICTED
because the rules on the expansion keep it: rule D does not see through a nested unconditional
block (the "unconditional wrapper" class of TEST-FORM section 1, which is stricter than
liveness by exactly rule D's own conservatism). Under the strict gate (OD-3) these 50 (0.33% of
the 15,288 judged cpython pairs) are the reference for the separate rule-D slice; on the other
five corpora there is no CONTRADICTED pair at all. Genuine live contradictions: **0 of 60,520**.

**XOFF (382): oracle noise, never a contradiction.** The owner is off in the X arm because the
expansion text parses with an error (`G1Error`):

| pairs | mechanism (isolated by deleting statements of the owner until the error went away) |
|---|---|
| 142 re2 + 152 protobuf (every file holding the pattern; isolated in re2 `dfa.cc`) | abseil `ABSL_LOG`/`DCHECK` macros expand to `switch (0) case 0: default: ! (c) ? (void) 0 : ::absl::log_internal::Voidify() && ...`, which tree-sitter-cpp does not parse |
| 76 cpython (`longobject.c` 54, `traceback.c` 15, `ceval_gil.c` 7) | `Py_ARRAY_LENGTH` expands to `sizeof(struct { int : -!!(...); })` (anonymous bit-field static assert), not parsed (isolated in `longobject.c`; the other two files hold the same expansion) |
| 12 cpython (`Parser/tokenizer/decoder.c`) | not isolated |

(The `Py_FALLTHROUGH` = `__attribute__((fallthrough));` statement, which accounted for 724 `dtoa.c`
XOFF pairs, is now normalized in the X arm.)

**UNJUDGED (2,979, no basis).** `no-pair-in-expansion` 2,881: code the host configuration does not
compile (inactive `#if` branches: the expansion's line is empty for 1,795 of redis's 1,820, 934 of
cpython's 1,004, all 6 leveldb and all 51 protobuf pairs, e.g. leveldb `db_bench.cc` Linux `/proc/cpuinfo`
code). `tokenize-failed` 98: six cpython files (mimalloc headers that `#error` when included
directly: "this file should be included from ...", one undefined function-like macro, one macro
arity error). The 19,314 "out of scope" pairs are in redis `deps/` and `modules/` and cpython
`Modules/`, `Include/` etc. (the census sees them, they are not translation units of the sweep).

### D3 recall ledger and D1 monotonicity (tool run on base)

`xarm_l3.py compare DIR --base TAG --head TAG` needs a head binary, which does not exist yet; on base
it was run in two ways to prove the tool.
- base against base (redis, cpython, leveldb): D1 0 violations, 0 edges added; D3 0 owners newly off,
  0 pairs restored.
- self-test, head = the rules-off run (`Tb`): D1 0 violations, adds exactly the dropped pairs
  (redis 30,408, cpython 28,809, leveldb 479); D3 restores all of them, split by the base's verdicts:
  redis CONFIRMED 21,605 / UNJUDGED 1,820 / out of scope 6,983; cpython CONFIRMED 15,238 /
  CONTRADICTED 50 / XOFF 88 / UNJUDGED 13,433 (1,102 + 12,331 out of scope, plus tokenize failures);
  leveldb CONFIRMED 473 / UNJUDGED 6. The reason column of the ledger is the head's owner state
  (`off:Incomplete` in this self-test); for a real head it is the gate's `Off::` reason, or
  `on(<X refusal reason>)` for a pair kept by an owner that stays on.
- The reference denominators for scoring a design (D3): the CONFIRMED columns above (pure recall cost
  if restored) and the per-corpus edge digests below.

### D4 parity

- **Other languages** (`parity-langs`, base binary; a head must match each digest, and two base runs
  agree: deterministic): click (Python, `8a1b1a3`) 10,858 edges `9c2b8e8a410764f7`; cobra (Go,
  `ad460ea`) 8,418 `7006a6efe08cc62d`; express (JS, `18e5985`) 7,416 `5dd34103d6926cba`; ripgrep
  (Rust, `82313cf`) 10,161 `8fc777efe109e334`.
- **C corpus edge digests** (T / Tb): redis `329593bf065e2953` / `4680db68e811d3bb`; cpython
  `10cbbee10ad41212` / `f3a7aba59aa0b2de`; leveldb `f101eb3c7471f8d4` / `8a22e10a99fc8159`; protobuf
  `83297cc25095ec2c` / `8010e51e276c2297`; re2 `0f69dd80ba7d6c7c` / `6b2b2bbe5e4d3244`; snappy
  `41d2a9370fb8b326` / `19a665b926645596`.
- **Cache modes** (`cache-modes`, redis `src/`, 217 files, 365,982 edges): cold = full-hit = no-cache
  digest `7b4ac90d69196155` (cold 10.7 s, full hit 4.9 s, no cache 9.9 s); after appending a
  function to `acl.c`: partial CPG cache hit, 365,985 edges, digest `5b8e3e251168ac43` = the
  no-cache build of the same tree; after adding two `#define`s to `adlist.h` (census digest moves):
  partial hit, same digest as no-cache. All four parity checks true.

### Runtimes (seconds, host busy with other jobs)

| corpus | tokenize | prism T | Tb | X | Xb | total |
|---|---|---|---|---|---|---|
| snappy | 23 | 0.4 | 0.3 | 0.3 | 0.2 | 1 |
| leveldb | 184 | 0.7 | 0.5 | 0.6 | 0.4 | 3 |
| re2 | 194 | 0.7 | 0.5 | 0.6 | 0.5 | 3 |
| redis | 45 | 9.5 | 8.8 | 4.2 | 3.4 | 33 |
| cpython | 400 | 33.2 | 26.5 | 18.5 | 13.2 | 505 |
| protobuf | 2,219 | 21.6 | 21.0 | 5.7 | 4.6 | 58 (+ tokenize) |

Tokenization (clang, 4 to 6 parallel) dominates; protobuf's is 37 minutes because every translation
unit includes abseil and libc++. The four-run set of a head costs only the prism runs (about 1 minute
for cpython and protobuf, seconds for the others) because the expansion text is made once.

## 2. L2 bounded generator (`xgen.py`)

Space (XARM.md, "The generator"): 10 site families (statement, guard, initializer, declarator, variadic
argument, argument, enclosing group at file scope / in a sibling owner / in a declaration / closed in a
following owner), owner kinds (C function, C++ function, template, method, local-class method), macro
kinds (object, `A()`, alias chain), macro bodies of at most 3 fragments (21 fragments; 29 for the
enclosing macro `M(p, q, r)`) with at most two macros, five lexical variants and three definition
channels. Only programs clang accepts count.

| run | what | candidates | prefiltered | clang-rejected | compiled and judged (plain) | variants judged | CONFIRMED | NODROP | XOFF | CONTRADICTED | UNJUDGED | keys (of which #831) | seconds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| k3 | **bound 3, exhaustive** (total fragments over both macros <= 3, plain, same file) | 1,455,420 | 586,482 | 838,270 | 30,668 | 8,921 | 16,330 | 9,508 | 3,926 | 9,306 | 519 | 123 (27) | 2,934 |
| k4a | bound 4, 150,000 random programs (seed 11) | 150,000 | 62,458 | 86,676 | 866 | 1,553 | 276 | 318 | 139 | 1,665 | 21 | 62 (24) | 383 |
| k4b | bound 4, every single-fragment insertion into 2,500 compiled bound-3 programs (seed 21) | 164,974 | 39,707 | 93,314 | 31,953 | 9,782 | 14,357 | 13,451 | 2,889 | 10,523 | 515 | 111 (47) | 852 |
| k4c | bound 4, 250,000 random programs (seed 12) | 250,000 | 104,701 | 143,839 | 1,460 | 2,249 | 525 | 556 | 242 | 2,334 | 52 | 84 (39) | 664 |

The space at bound 3 is 30 object cells x 38,410 body pairs (sum over a+b<=3 of 21^a x 21^b) +
12 enclosing cells x 25,260 bodies = 1,455,420; every member was prefiltered, rejected by clang or
judged. The prefilter (bracket nesting of `{` and `(` after expansion of the object macros and identity
wrappers) was checked against clang on 12,000 sampled programs: it never rejected one clang accepts.
Batch verdicts were checked against a one-program-per-repository run of the full oracle on 75
programs of the bound-2 run (15 per verdict): 0 mismatches; every frozen row was re-judged alone. Compile rate: 2.1% at bound 3 (0.4% for the random bound-4
samples, 20% for the neighbourhood batch).

**Keys** = (effect classes, site family + macro kind, owner kind). At bound 3: 123 keys, 27 tagged
#831 (a top-level `;` in an object macro's body; excluded, not frozen), **96 frozen**:

| effect classes | what | families and owners |
|---|---|---|
| `BR_UNBAL` (24 keys) | an object macro with brackets that do not nest within the body (`{` / `}`, digraph forms, `(`, `[`) | stmt, guard, arg, arg1 over C, C++, template, and for stmt/guard method and local-class owners, `A()` and alias kinds |
| `BR_UNBAL,CTRL` (24) | the same with `if (c)`, `else`, `do`, `while (0)`, `for (;;)` | same cells |
| `PARAMS` (12) | an enclosing-group macro drops, duplicates or reorders a parameter | enc1 to enc4 x C, C++, template |
| `BR_BAL,PARAMS` (12) | the same with balanced brackets | same |
| `BR_BAL,CTRL,PARAMS` (12) | the same under a control wrapper (the `J(p, q, r) p; if (c) { q; } r` class) | same |
| `ARGSPLIT,PARAMS` (12) | the same with a comma fragment | same |

`init` and `decl` sites produced no contradiction outside #831. Lexical variants (splice between
fragments or inside a token, comments holding brackets, CRLF) and channels (header, census text)
were applied to up to 6 minimal members of every key and a 0.3% sample of the rest: 8,921 programs at
bound 3 and as many again at bound 4: **no variant-only key**; verdicts equal the plain verdict.
Mapped to the hand classes: the enclosing-group, closer-in-following-owner, declaration-initializer
and dropped-overwriting-piece receipts are the enc/`PARAMS` keys; the unbalanced-brace guard rows are
`BR_UNBAL`; the alias-chain receipt is `stmt+alias`; digraph/comment/splice forms are the variants.
Not generated (kept as hand rows): external macros (A-ext), computed and transitive includes, Latin-1
bytes, depth 1000.

**Bound k+1 and exit criterion 3: NOT met as stated, with a short tail.** Across the three bound-4
batches the new keys relative to bound 3 are 16 (k4a), 27 (k4b) and 23 (k4c); all but two are #831
(`BR_UNBAL,CTRL,SEMI0|...` combinations). The two non-#831 new keys are
`PARAMS,WRITE|enc2|cxx` and `PARAMS,WRITE|enc4|cxx` (body `int *x ; p; r`: a harmless declaration
beside a dropped middle parameter; the `WRITE` class and the C++ cell make it a key only because it
needs four fragments; the C and template cells of the same class were not hit). k4a found them, k4b
re-found them, and after they are counted, **k4b (165,000 candidates) and k4c (250,000) added no
further non-#831 key**. They are frozen as rows 97 and 98. Sampled and neighbourhood batches cannot
certify an exhaustive bound-4 space (compile rate 0.4% for sampling); what they show is that the
key set at bound 4 is the bound-3 set plus one effect-class combination. Under the convergence rule
this is a tail of one class, not an open-class stream, but the design must not rely on the generator
for completeness: that is criterion F4 (a certificate).

**Frozen rows**: `tests/fixtures/dfg_prune/oracle/generated_rows.jsonl`, 98 rows (`source: generator`,
`assert: kept`), minimal member of each key, each re-judged ALONE with the full oracle (4 runs, `-H`
inputs check): 98 of 98 CONTRADICTED with head dropping the pair. Rust: `c_prune_generated_rows`
(one test per family: stmt 18 rows, guard 18, arg 6, arg1 6, enc1 12, enc2 13, enc3 12, enc4 13) and
`generated_rows_are_well_formed`; on base all 8 family tests are red ("N of N ... a live pair was
dropped"), the well-formedness test is green.

## 3. Not done and limits

- No head binary exists, so D1/D3 were exercised with base-vs-base and the rules-off self-test only.
- Corpora tokenize under the host configuration (macOS, default features); code under other `#if` branches
  is UNJUDGED (about 5% of the redis and cpython pairs).
- cpython uses a 3.14.4 `pyconfig.h` for a 3.16-dev tree; protobuf and re2 use the installed abseil
  (20260107.1), not the version the corpus pins.
- 12 cpython XOFF pairs were not isolated; the oracle does not model the `Modules/` and `Include/` pairs.
- The generator's frozen rows are the minimal member of each key found, not a covering set of all
  contradicted programs (15,469 programs map onto the 98 keys); rows are owner-aware in the Rust test
  while the oracle CLI is owner-blind (checked: 98 of 98 agree that head drops the pair).

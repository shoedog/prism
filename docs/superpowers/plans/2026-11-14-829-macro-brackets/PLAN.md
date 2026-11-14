# #829 implementation plan: macros that change bracket structure

Design: `DESIGN.md` in this directory. Read sections 1, 6 and 9 first.
Worktree: `/Users/wesleyjinks/code/prism-829`, branch `fix/829-macro-brackets`, base `a0f2c427`. Work only there.

**Goal of the task (state it in every review brief):** rules U and D for `.c` and C++ owners must not drop a live def→use pair because an in-repo macro (or a listed external one) changes the bracket structure the compiler sees. Removal-only: keeping more pairs is always acceptable; dropping fewer correct pairs in the control rows is a defect.

**Shape:** one PR, two commits. Commit A = tasks 1 to 6 (issue scope plus the external name list). Commit B = tasks 7 to 9 (comma facts and argument split, OD-1). Tasks 10 to 13 close the PR.
**Owner decisions (2026-11-14, final): OD-1 yes, OD-2 deferred and filed as issue #831, OD-3 yes; the "no" branches below do not apply.** They would have changed the task list: OD-1 no → skip tasks 7 to 9, do the "if no" text of DESIGN section 9. OD-3 no → drop `BLOCK_MACROS`, rows `b12`, `b13`, `x6`, `E01`. OD-2 is not implemented in either case.
**Caps (declare before dispatch):** implementer: at most 2 repair attempts per failing task before reporting back. Review: 3 rounds for the PR; if commit B is not converging after 2 rounds, split it into a stacked PR (it then needs its own cache version) and merge commit A.

## Rules for the implementer

- Tests first. For each task, write the tests, run them, and paste the failing output (RED) into your notes before writing product code.
- Run cargo tests through the memory guard: `~/prism-evidence/tools/memguard.sh 8 cargo test -p prism --lib -- <filter>`. Targeted tests only; the full suite runs in GitHub CI.
- No file that contains `#define` goes under `tests/fixtures/` or under any C-like extension anywhere in the repository (the loader would read it into the census of prism's own tree). Macro inputs live as Rust strings in the inline-case files and test files.
- File size: `python3 scripts/file_size.py <files>`. New files aim for 600 weighted lines. `src/call_graph/c/macro_census.rs` is at 1,441: add only what this plan lists there; the scan logic goes in the new `macro_census/brackets.rs`. Say in the PR that the file is over 1,000 and that its split is a separate move-only PR.
- Do not change `spelled_identifiers` output or the set `macro_call_arguments` returns for escapes. The new tokens must not consume or split identifier text.
- Do not touch `eval/secbench`. Do not push. Commit only when a task group is green.
- Probe directories with the review's and the design's inputs are in `.scratch829/` (git-excluded). `.scratch829/design/run.sh <dir>` prints owner verdicts and forward edges for a directory, using `target/release/prism`.

## Variant matrix (75 rows)

"Base" is head `a0f2c427`, measured with the release binary for the owner rows (`.scratch829/design/m_c`, `m_cc`, `m_c2`, `m_cc2`). Kind: **S** = soundness row (the dropped pair is live; clang-checked in DESIGN section 3), **C** = conservative row (the refusal is accepted, no liveness claim), **K** = control (pruning must survive).

### M1. Census facts, `unbalanced` (commit A) — `src/call_graph/c/macro_census_brackets_tests.rs`

Base: the field does not exist. After task 1's stub every "true" row is RED.

| id | input (one `.c` file unless noted) | `unbalanced` |
|---|---|---|
| U01 | `#define BEGIN {` | true |
| U02 | `#define END }` | true |
| U03 | `#define OPEN for (` | true |
| U04 | `#define CLOSE )` | true |
| U05 | `#define LB [` ; `#define RB ]` | true ; true |
| U06 | `#define CROSS ( { ) }` | true |
| U07 | `#define FLIP } {` | true |
| U08 | `#define OPEN_IF(c) if (c) {` | true |
| U09 | `#define WRAP(v) { f(v); }` ; `#define AT(a, i) ((a)[(i)])` ; `#define STMT do { g(); } while (0)` ; `#define EMPTY` | false each |
| U10 | `#define MSG "{ ( ["` ; `#define CH '}'` ; `#define QUOTE '\''` ; `#define ESC "\"{"` | false each |
| U11 | `#define CMT1 1 /* { */ + 2` ; `#define CMT2 3 // (` | false each |
| U12 | `.cpp`: `#define RS R"x({)x"` ; `#define SEP 1'000 + (2)` | false each |
| U13 | `#define DLB <%` ; `#define DRB %>` ; `#define DLS <:` ; `#define DRS :>` | true each |
| U14 | with U01/U02: `#define ENTER BEGIN` ; `#define ENTER2 ENTER` ; `#define BOTH(s) BEGIN s; END` | true each |
| U15 | `#define DUAL 0` then `#define DUAL {` ; the reverse order ; the two definitions in two files | true each |
| U16 | `#define CONT { \` + newline + `}` ; `#define CONT2 f( \` + newline + ` g(` | false ; true |
| U17 | `#define MC ( /* a` + newline + ` b */ )` ; `#define MC2 { /* a` + newline + ` b */` | false ; true |
| U18 | `#define VO(...) __VA_OPT__({)` ; `#define VO2(...) f(__VA_OPT__(1))` | true ; false |
| U19 | `#define BADQ "abc {` | `poisoned` true (no assertion on `unbalanced`) |
| U20 | `/*` newline `#define CBEGIN {` newline `*/` in a parsed file | true (accepted over-approximation, pinned) |
| U21 | `#define TB ??<` | `census.incomplete()` true (existing trigraph belt) |
| U22 | census text `defs.inc` (via `set_c_census_texts`): `#define PBEGIN {` ; `#define PMSG "("` ; `#define PQ " {` ; `#define PF(a, b) g(a, b)` | true ; false ; true ; false |
| U23 | `#define BEGIN {` then `#undef BEGIN` | true |
| U24 | digest: `#define X {}` versus `#define X {` | digests differ |
| U25 | `#define S(x) #x` ; `#define H(x) (#x "(")` | false each |
| U26 | `#define FN(a) (a` (function-like, body unbalanced) | true |

### M2. Owner rows, commit A — inline cases `20_macro_brackets.c` and `20_macro_brackets.cc` (sources below)

| id | lang | shape | kind | base | after | gate |
|---|---|---|---|---|---|---|
| b1 | C | `END;` … `BEGIN;` moves the scope of an inner `int *x` | S | drop | keep | MacroBrackets |
| b2 | C | the same through `LEAVE`/`ENTER` | S | drop | keep | MacroBrackets |
| b3 | C | `IF_BEGIN; x = 2; IF_END;` | S | drop | keep | MacroBrackets |
| b4 | C | `OPEN_IF(c); x = 2; IF_END;` | S | drop | keep | MacroBrackets |
| b5 | C | digraph macros `RBD`/`LBD` | S | drop | keep | MacroBrackets |
| b6 | C | `P_OPEN; c; P_CLOSE;` (unbalanced parentheses) | C | drop | keep | MacroBrackets |
| b7 | C | `LB; RB;` (unbalanced square brackets; not compilable, mechanism row) | C | drop | keep | MacroBrackets |
| b8 | C | `BOTH(sink(0));` (balanced wrapper of two unbalanced macros) | C | drop | keep | MacroBrackets |
| b9 | C | `sink(DUAL)` (one of two definitions is unbalanced) | C | drop | keep | MacroBrackets |
| b10 | C | `WRAP(0)`, `MSG`, `CH`, `CMT` (balanced; brackets in literals and comments) | K | drop | drop | none |
| b11 | C | `END BEGIN` only inside a string literal and a comment | K | drop | drop | none |
| b12 | C | `pthread_cleanup_push(h, 0); int *x = 0; pthread_cleanup_pop(0);` | S | drop | keep | MacroBrackets (name list) |
| b13 | C | `Py_BEGIN_CRITICAL_SECTION(op); … Py_END_CRITICAL_SECTION();` | C | drop | keep | MacroBrackets (name list) |
| x1 | C++ | issue `b1s`: `END;` … `BEGIN;`, `S x` with `operator=` | S | drop | keep | MacroBrackets |
| x2 | C++ | issue `f1s`: `OPEN; int* x = nullptr; CLOSE;` | S | drop | keep | MacroBrackets |
| x3 | C++ | `IF_BEGIN; x = 2; IF_END;` | S | drop | keep | MacroBrackets |
| x4 | C++ | the same in a `template` owner | S | drop | keep | MacroBrackets |
| x5 | C++ | `WRAP(0)`, `MSG` | K | drop | drop | none |
| x6 | C++ | `pthread_cleanup_push(h, 0); x = 2; pthread_cleanup_pop(0);` | C | drop | keep | MacroBrackets (name list) |

### M3. Multi-file and cache rows, commit A — `src/data_flow/build/c_prune_brackets_tests.rs`, `c_macro_census_tests.rs`

| id | setup | kind | base | after |
|---|---|---|---|---|
| mf1 | `h.h` holds `#define END }` / `#define BEGIN {`; `a.c` holds the `b1` owner (and no `#define`) | S | prunes (`after < before`) | `after == before`. Control: `a.c` alone (no file defines `END`/`BEGIN`) still prunes |
| mf2 | the defines arrive only as a census text `defs.inc` stamped with `set_c_census_texts` | S | prunes | no pruning |
| mf3 | `x.c` holds `#define DUAL 0`, `y.c` holds `#define DUAL {`; owner in `a.c` spells `DUAL` | C | prunes | no pruning |
| mf4 | the issue's exact files (below): `b1s.cc` parsed as C++ with `h.h` parsed as C | S | prunes; the pair `x@8 → x@9` is missing | the pair `x@8 → x@9` is present |
| cz1 | `c_macro_census_tests.rs`: `m.h` goes from `#define MARK {}` to `#define MARK {`; `a.c` is `"int f(void) {\n    int x = 1;\n    MARK;\n    x = 2;\n    return x;\n}\n"`; cold, full hit, partial hit | — | n/a | the partial hit rebuilds in full; its edge set equals a cold build of the edited tree and holds `x@2 → x@5`, which the first build does not |
| E01 | `Off` verdict per owner of `20_macro_brackets.c`: exact gate names from table M2 (`scan` called directly) | — | `Ok` for every row | as in M2 |

### M4. Census facts, `loose_comma` (commit B) — same test file as M1

| id | input | `loose_comma` |
|---|---|---|
| L01 | `#define PAIR 1, 2` | true |
| L02 | `#define INIT {0, 0}` | true |
| L03 | `#define F2(a, b) f(a, b)` ; `#define FORH for (i = 0, j = 0; i < n; i++)` | false each |
| L04 | `#define FWD(...) __VA_ARGS__` ; `#define CALL(...) f(__VA_ARGS__)` ; `#define BR(...) {__VA_ARGS__}` | true ; false ; true |
| L05 | `#define TOP(...) __VA_OPT__(,) __VA_ARGS__` ; `#define LOG(fmt, ...) printf(fmt __VA_OPT__(,) __VA_ARGS__)` | true ; false |
| L06 | `#define E(fmt, args...) printf(fmt, args)` (GNU named variadic) | true (fail closed) |
| L07 | `#define SE(x) ({ int a = (x), b = 0; a + b; })` | true |
| L08 | `#define SUB(a, i, j) a[i, j]` | true |
| L09 | with L01: `#define W2 PAIR` ; `#define CALLP f(PAIR)` | true ; true (blanket closure, pinned) |
| L10 | `.cpp`: `#define MAP std::map<int, int>` | true |
| L11 | census text: `#define PPAIR 1, 2` ; `#define PG(a, b) g(a, b)` ; `#define PE(args...) g(args)` | true ; false ; true |
| L12 | `#define SC "a, b"` ; `#define CC ','` | false each |

### M5. Owner rows, commit B — inline cases `21_macro_args.c` and `21_macro_args.cc`

| id | lang | shape | kind | base | after | gate |
|---|---|---|---|---|---|---|
| a1 | C | `F({ int a, bb; })` with `#define F(p, q) q p` | S | drop | keep | MacroArgSplit |
| a2 | C | `ID(F)({ int a, bb; });` (chained group) | C | drop | keep | MacroArgSplit |
| a3 | C | `USE(fn((struct P){1, 2}));` (brace-held comma under nested parentheses) | C | drop | keep | MacroArgSplit |
| a4 | C | `int arr[] = {PAIR};` | C | drop | keep | MacroComma |
| a5 | C | `int arr[2] = INIT;` | C | drop | keep | MacroComma |
| a6 | C | `fn2(FWD(1, 2))` | C | drop | keep | MacroComma |
| a7 | C | `MAX(a, fn2(b, a))`, `CALL(a, b)` (commas directly inside parentheses) | K | drop | drop | none |
| a8 | C | `fn((struct P){1, 2})` with `fn` a function | K | drop | drop | none |
| y1 | C++ | `F([&](int){ int a, b; });` | S | drop | keep | MacroArgSplit |
| y2 | C++ | `CHECK(v == V{1, 2});` with in-repo `CHECK` | C | drop | keep | MacroArgSplit |
| y3 | C++ | `MAX(a, fn2(b, a))` | K | drop | drop | none |
| y4 | C++ | `EXPECT_TRUE(v == V{1, 2});` with no definition in the repository | K | drop | drop | none |

Counts: M1 26, M2 19, M3 6, M4 12, M5 12 = 75 rows.

Commit A carries M1, M2 and M3 (51 rows); commit B carries M4 and M5 (24 rows) and extends `E01` to the `21_macro_args` cases.

## Fixture sources

**Changed after the test build (see `TESTS.md`, Deviations):** the C scope-shift shapes of b1, b2, b5, b12, b13 and a1 have no baseline pair, so their functions below are gate-only (no row markers; E01 pins their gate). The rows b1, b2, b5, b12 and b13 are carried by guard-shape twins (`if (cond) BEGIN; x = 2; END;` and the analogues) added at the end of `20_macro_brackets.c`; a1 is gate-only. The sources in `c_prune_inline_cases.rs` are authoritative.

Add each as a new entry of `INLINE_CASES` (`(file name, r####"…"####)`), verbatim. Line numbers matter only to the generated rows; do not hand-edit `obligations.jsonl`.

### `20_macro_brackets.c` → `src/data_flow/build/c_prune_inline_cases.rs`

```c
// Bracket macros (#829). `#define` lines are top-level, so every case shares one census.
int src(void);
void sink(int);
void sink2(const char *);
#define END }
#define BEGIN {
#define LEAVE END
#define ENTER BEGIN
#define IF_BEGIN if (cond) {
#define IF_END }
#define OPEN_IF(c) if (c) {
#define RBD %>
#define LBD <%
#define P_OPEN for (
#define P_CLOSE )
#define LB [
#define RB ]
#define BOTH(s) BEGIN s; END
#define DUAL 0
#define DUAL {
#define WRAP(v) { sink(v); }
#define MSG "{ ( ["
#define CH '}'
#define CMT 1 /* { */ + 2 // (
int cond;

void brace_scope(int x) {     // @b1:D x
    int r = 0;
    {
        int *x = 0;
        END;
        r = x;                // @b1:U:keep:unbalanced_brace_macro_moves_scope
        BEGIN;
    }
    sink(r);
}

void brace_transitive(int x) { // @b2:D x
    int r = 0;
    {
        int *x = 0;
        LEAVE;
        r = x;                // @b2:U:keep:macro_naming_an_unbalanced_macro
        ENTER;
    }
    sink(r);
}

void brace_control(void) {
    int x = src();            // @b3:D x
    IF_BEGIN;
    x = 2;
    IF_END;
    sink(x);                  // @b3:U:keep:unbalanced_macro_makes_the_write_conditional
}

void brace_function_like(int c) {
    int x = src();            // @b4:D x
    OPEN_IF(c);
    x = 2;
    IF_END;
    sink(x);                  // @b4:U:keep:function_like_unbalanced_macro
}

void brace_digraph(int x) {   // @b5:D x
    int r = 0;
    {
        int *x = 0;
        RBD;
        r = x;                // @b5:U:keep:digraph_brace_macro
        LBD;
    }
    sink(r);
}

void paren_macro(int c) {
    int x = src();            // @b6:D x
    P_OPEN; c; P_CLOSE;
    x = 2;
    sink(x);                  // @b6:U:keep:unbalanced_paren_macro_owner_off
}

void bracket_macro(void) {
    int x = src();            // @b7:D x
    LB; RB;
    x = 2;
    sink(x);                  // @b7:U:keep:unbalanced_square_macro_owner_off
}

void balanced_wrapper_of_unbalanced(void) {
    int x = src();            // @b8:D x
    BOTH(sink(0));
    x = 2;
    sink(x);                  // @b8:U:keep:wrapper_names_unbalanced_macros
}

void one_definition_unbalanced(void) {
    int x = src();            // @b9:D x
    sink(DUAL);
    x = 2;
    sink(x);                  // @b9:U:keep:any_unbalanced_definition_taints_the_spelling
}

void balanced_macros_still_prune(void) {
    int x = src();            // @b10:D x
    WRAP(0);
    sink2(MSG);
    sink(CH);
    sink(CMT);
    x = 2;
    sink(x);                  // @b10:U:drop:brackets_in_literals_and_comments_do_not_count
}

void not_spelled_still_prunes(void) {
    int x = src();            // @b11:D x
    sink2("END BEGIN");       /* END BEGIN */
    x = 2;
    sink(x);                  // @b11:U:drop:macro_named_only_in_a_literal_or_comment
}

void external_pthread_pair(void (*h)(void *), int x) { // @b12:D x
    int r = 0;
    {
        pthread_cleanup_push(h, 0);
        int *x = 0;
        pthread_cleanup_pop(0);
        r = x;                // @b12:U:keep:external_block_macro_pair
    }
    sink(r);
}

void external_cpython_pair(void *op, int x) { // @b13:D x
    int r = 0;
    {
        Py_BEGIN_CRITICAL_SECTION(op);
        int *x = 0;
        Py_END_CRITICAL_SECTION();
        r = x;                // @b13:U:keep:external_block_macro_pair
    }
    sink(r);
}
```

### `20_macro_brackets.cc` → `src/data_flow/build/cpp_prune_inline_cases.rs`

```cpp
// Bracket macros (#829), C++ owners. One census for the file.
int src();
void sink(int);
#define END }
#define BEGIN {
#define OPEN for (
#define CLOSE )
#define IF_BEGIN if (cond) {
#define IF_END }
#define WRAP(v) { sink(v); }
#define MSG "{ ( ["
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };
int cond;
void sink2(const char *);

int cpp_brace_scope(S x) {
    {
        int* x = nullptr;
        END;
        x = 5;                // @x1:D x
        x = 1;                // @x1:U:keep:unbalanced_brace_macro_moves_scope
        BEGIN;
    }
    return x.v;
}

int cpp_paren_scope(S x) {
    OPEN; int* x = nullptr; CLOSE;
    x = 5;                    // @x2:D x
    x = 1;                    // @x2:U:keep:unbalanced_paren_macro_moves_scope
    return x.v;
}

void cpp_brace_control() {
    int x = src();            // @x3:D x
    IF_BEGIN;
    x = 2;
    IF_END;
    sink(x + 1);              // @x3:U:keep:unbalanced_macro_makes_the_write_conditional
}

template <typename T>
void cpp_template_owner(T t) {
    int x = src();            // @x4:D x
    IF_BEGIN;
    x = 2;
    IF_END;
    sink(x + 1);              // @x4:U:keep:template_owner_spells_unbalanced_macro
}

void cpp_balanced_still_prunes() {
    int x = src();            // @x5:D x
    WRAP(0);
    sink2(MSG);
    x = 2;
    sink(x + 1);              // @x5:U:drop:balanced_macros_do_not_turn_the_owner_off
}

void cpp_external_pthread_pair(void (*h)(void *)) {
    int x = src();            // @x6:D x
    pthread_cleanup_push(h, 0);
    x = 2;
    pthread_cleanup_pop(0);
    sink(x + 1);              // @x6:U:keep:external_block_macro_pair
}
```

### Issue files for row mf4 (as Rust strings in `c_prune_brackets_tests.rs`)

`h.h`:
```c
#define END }
#define BEGIN {
```
`b1s.cc`:
```cpp
#include "h.h"
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };
struct T { int k; };
int b(S x) {
    {
        int* x = nullptr;
        END;
        x = 5;
        x = 1;
        BEGIN;
    }
    return x.v;
}
```

### `21_macro_args.c` → `c_prune_inline_cases.rs` (commit B)

```c
// Macro argument lists and comma-carrying macros (#829 part B). One census for the file.
int src(void);
void sink(int);
struct P { int a, b; };
int fn(struct P p);
#define F(p, q) q p
#define ID(v) v
#define PAIR 1, 2
#define INIT {0, 0}
#define FWD(...) __VA_ARGS__
#define CALL(...) fn2(__VA_ARGS__)
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define USE(v) sink(v)
int fn2(int, int);

void arg_split_scope(int x) { // @a1:D x
    int r = 0;
    {
        int *x = 0; int bb = 0;
        F({ int a, bb; })
        ;
        r = x;                // @a1:U:keep:brace_held_comma_in_macro_arguments
    }
    sink(r);
}

void arg_split_chain(void) {
    int x = src();            // @a2:D x
    ID(F)({ int a, bb; });
    x = 2;
    sink(x);                  // @a2:U:keep:brace_held_comma_in_a_chained_group
}

void arg_split_compound_literal(void) {
    int x = src();            // @a3:D x
    USE(fn((struct P){1, 2}));
    x = 2;
    sink(x);                  // @a3:U:keep:brace_held_comma_nested_in_parentheses
}

void loose_top_level_comma(void) {
    int x = src();            // @a4:D x
    int arr[] = {PAIR};
    x = 2;
    sink(x + arr[0]);         // @a4:U:keep:macro_with_a_top_level_comma
}

void loose_brace_held_comma(void) {
    int x = src();            // @a5:D x
    int arr[2] = INIT;
    x = 2;
    sink(x + arr[0]);         // @a5:U:keep:macro_with_a_brace_held_comma
}

void loose_variadic_top(void) {
    int x = src();            // @a6:D x
    sink(fn2(FWD(1, 2)));
    x = 2;
    sink(x);                  // @a6:U:keep:variadic_arguments_outside_parentheses
}

void tight_commas_still_prune(int a, int b) {
    int x = src();            // @a7:D x
    sink(MAX(a, fn2(b, a)));
    sink(CALL(a, b));
    x = 2;
    sink(x);                  // @a7:U:drop:commas_directly_inside_parentheses
}

void non_macro_call_still_prunes(void) {
    int x = src();            // @a8:D x
    sink(fn((struct P){1, 2}));
    x = 2;
    sink(x);                  // @a8:U:drop:brace_held_comma_in_a_function_call
}
```

### `21_macro_args.cc` → `cpp_prune_inline_cases.rs` (commit B)

```cpp
// Macro argument lists (#829 part B), C++ owners. One census for the file.
int src();
void sink(int);
#define F(p, q) q p
#define CHECK(...) check_impl(__VA_ARGS__)
#define MAX(a, b) ((a) > (b) ? (a) : (b))
struct S { int v; S& operator=(int i) { v = v * 10 + i; return *this; } };
struct V { int a, b; };
bool check_impl(bool);
bool operator==(V, V);
int fn2(int, int);

int cpp_arg_split_scope(S x) {
    {
        int* x = nullptr; int b = 0;
        F([&](int){ int a, b; });
        x = 5;                // @y1:D x
        x = 1;                // @y1:U:keep:brace_held_comma_in_macro_arguments
    }(0);
    return x.v;
}

void cpp_init_list_in_macro(V v) {
    int x = src();            // @y2:D x
    CHECK(v == V{1, 2});
    x = 2;
    sink(x + 1);              // @y2:U:keep:initializer_list_comma_in_macro_arguments
}

void cpp_tight_commas_still_prune(int a, int b) {
    int x = src();            // @y3:D x
    sink(MAX(a, fn2(b, a)));
    x = 2;
    sink(x + 1);              // @y3:U:drop:commas_directly_inside_parentheses
}

void cpp_external_macro_still_prunes(V v) {
    int x = src();            // @y4:D x
    EXPECT_TRUE(v == V{1, 2});
    x = 2;
    sink(x + 1);              // @y4:U:drop:brace_held_comma_in_an_external_macro
}
```

## Tasks

### Task 0. Preflight
1. `git status` clean, branch `fix/829-macro-brackets`.
2. `~/prism-evidence/tools/memguard.sh 8 cargo test -p prism --lib -- macro_census c_prune cpp_prune c_macro_census` and note the totals (the baseline).
3. Read `src/call_graph/c/macro_census.rs` (`Tok`, `tokenize_seq`, `entry`, `merge`, `permissive_define`, `close`, `digest`, `macro_call_arguments`) and `src/data_flow/build/c_prune.rs` (`Off`, `note_name`, `scan`).

### Task 1. Census tests for `unbalanced` (RED)
1. In `macro_census.rs`, add `pub unbalanced: bool` and `pub loose_comma: bool` to `MacroFacts`, and private `unbalanced`, `loose_comma` to `RawDef`, all defaulting to `false`, copied in `close`. No logic yet. This only lets the tests compile.
2. Create `src/call_graph/c/macro_census_brackets_tests.rs`; register it in `src/call_graph/c.rs` under `#[cfg(test)]`. Copy the `census(&[(file, source)])` helper from `macro_census_s6b_tests.rs`. Add a helper that stamps a census text on a parsed file with `ParsedFile::set_c_census_texts`.
3. One test function per group of table M1 (U01 to U26), asserting `census.get(name).unwrap().unbalanced`. Name them `u01_…` so the mutation lane can cite them.
4. Run: `… cargo test -p prism --lib -- macro_census_brackets`. Expected RED: every row whose expected value is `true`, and U24. Rows expecting `false`, U19 and U21 pass already; that is fine and expected.

### Task 2. The bracket scan (GREEN for task 1)
1. `Tok`: make identifiers borrowed and add punctuation, so the scan allocates no strings:
   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   enum Tok<'a> { Ident(&'a str), Open, Close, /* { } [ ] , */ Punct(u8), /* <% %> <: :> */ Digraph }
   ```
   Adapt `tokenize_seq` and `macro_call_arguments` (it matches `Ident`/`Open`/`Close`; give it a catch-all arm that ignores the new tokens).
2. `tokenize_seq`: when `seq` is `Some`, push `Punct(c)` for `{ } [ ] ,`; push `Digraph` and advance two bytes for `<%`, `<:` (arm on `<`), `%>` (a new arm beside the existing `%:` arm) and `:>`. These arms sit before the catch-all `_ => i += 1`. The identifier set and the `ok`/`paste` results must not change: run the existing census tests to confirm.
3. New module `src/call_graph/c/macro_census/brackets.rs` (`mod brackets;` next to `mod raw_string;`):
   ```rust
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub(super) struct Shape { pub unbalanced: bool, pub loose_comma: bool }
   /// The DESIGN 6.1 automaton over a body's ordered tokens.
   pub(super) fn body_shape(seq: &[super::Tok<'_>]) -> Shape
   /// The same automaton over raw bytes (no literal or comment tracking): the permissive fallback.
   pub(super) fn raw_shape(rest: &str) -> Shape
   /// Permissive verdict for the text after a macro name: per dialect, the token scan when the
   /// line tokenizes, else the raw scan; ORed.
   pub(super) fn permissive_shape(rest: &str) -> Shape
   ```
   In commit A, implement the bracket half only (`loose_comma` stays `false`; the comma, `__VA_ARGS__` and `__VA_OPT__` handling arrive in task 8). Put unit tests for `body_shape` on hand-built token slices in a `#[cfg(test)] mod tests` of this file (crossing, closer first, leftover opener, digraph).
4. `entry`: replace `tokenize(p.body, cxx)` by `tokenize_seq(p.body, cxx, Some(&mut seq))`; set `def.unbalanced = body.ok && brackets::body_shape(&seq).unbalanced`.
5. `permissive_define`: `def.unbalanced = brackets::permissive_shape(rest).unbalanced`. The `undef` branch of `collect_permissive` already resets the definition to defaults; leave it.
6. `merge`: OR `unbalanced`. `close`: start from the definition's own flag and OR over `reaches`, exactly like `control`.
7. `digest`: tag `c-macro-census-v3`; flag bytes `[control, poisoned, unbalanced, loose_comma]`.
8. Add `pub fn unbalanced_count(&self) -> usize` beside `poisoned_count` (tests and trace use it).
9. Run the task 1 tests (GREEN) and the whole census set: `-- macro_census` and `-- c_macro_census`.

### Task 3. Owner rows (RED)
1. Add `20_macro_brackets.c` to `c_prune_inline_cases.rs` and `20_macro_brackets.cc` to `cpp_prune_inline_cases.rs` (sources above).
2. Regenerate: `python3 tests/fixtures/dfg_prune/c/gen_obligations.py` and `python3 tests/fixtures/dfg_prune/cpp/gen_obligations.py`. The diff of each `obligations.jsonl` must be additions only (13 and 6 rows).
3. Create `src/data_flow/build/c_prune_brackets_tests.rs`, registered in `src/data_flow/build/mod.rs` under `#[cfg(test)]`:
   - a `counts(files, target, language_of)` helper like the one in `c_prune_fold2_tests.rs` (frozen producer versus `Producer::file_refs_with_census`), accepting `.cc` targets parsed as C++ and census texts;
   - rows mf1 to mf4 of table M3, each with its control (same owner, no macro definition: prunes);
   - E01: a helper that parses a case, finds each `function_definition` (and `template_declaration`) owner by name and calls `super::c_prune::scan`, asserting `Err(Off::MacroBrackets)` for b1 to b9, b12, b13 and `Ok` for b10, b11.
4. Run `-- c_prune_matrix cpp_prune_matrix c_prune_brackets`. Expected RED: rows b1 to b9, b12, b13, x1 to x4, x6 (kept expected, dropped found), mf1 to mf4, E01. b10, b11, x5 pass. If a keep row reports "inadmissible missing baseline pair", the marker lines are wrong: fix the fixture, not the row file.

### Task 4. The owner gate (GREEN for task 3)
In `c_prune.rs`:
1. `Off::MacroBrackets` with a doc comment ("an invoked in-repo macro, or a listed external one, has unbalanced brackets in its replacement list: block and statement structure differ from the parse").
2. `const BLOCK_MACROS: &[&str]` with the 16 names of DESIGN 6.3 and a doc comment that names the D2 boundary and the `clang -E` evidence.
3. `note_name`: in the name match, `n if BLOCK_MACROS.contains(&n) => set_off(Off::MacroBrackets)`; in the census branch, `if facts.unbalanced { set_off(Off::MacroBrackets); }`.
4. Update the module doc (gate list) of `c_prune.rs` and of `macro_census.rs` (facts list).
5. Run task 3's tests (GREEN), then `-- c_prune cpp_prune` in full: the removal-only sweeps and every older matrix row must still pass.

### Task 5. Cache
1. `src/cpg_cache.rs`: `CACHE_VERSION` 189, with a changelog line in the style of v187 ("v189: C/C++ owners that spell an in-repo macro with unbalanced brackets in its replacement list, or a listed external block macro, are off for rules U and D (#829); the census digest gains the bracket facts"). If commit B lands in the same PR, extend that line; do not bump twice.
2. Update the pinned asserts: `src/cpg_cache.rs` (`cache_versions_are_pinned_for_cpg_semantics`) and three in `src/cpg_cache/dfg_codec_tests.rs`.
3. Row cz1 in `src/data_flow/build/c_macro_census_tests.rs`: a second parity test modelled on `cache_parity_cold_full_partial_when_a_header_define_changes`, with the constants of table M3. It must be RED when the digest omits the new bytes (check by temporarily removing them).
4. `CLAUDE.md` says "CPG cache (currently v188)": set it to v189 in task 10.

### Task 6. Commit A
`cargo fmt --all`, the targeted tests of tasks 1 to 5, `python3 scripts/file_size.py` on every touched file. Commit: `fix(dfg): C/C++ owners off when a macro has unbalanced brackets (#829)`.

### Task 7. Census tests for `loose_comma` (RED) — OD-1
Add rows L01 to L12 (table M4) to `macro_census_brackets_tests.rs`. Expected RED: every `true` row.

### Task 8. Comma facts and argument split (GREEN for task 7, then RED/GREEN for the owners)
1. `brackets.rs`: complete the automaton (DESIGN 6.1): `Punct(b',')` and `Ident("__VA_ARGS__")` set `loose_comma` unless the nearest non-transparent stack entry is `(`; `Ident("__VA_OPT__")` directly before `Open` pushes a transparent entry. `raw_shape` gets the same comma rule and treats the byte string `__VA_ARGS__` as the identifier. Add `pub(super) fn named_variadic(rest: &str) -> bool` (an identifier character directly before `...`, blanks allowed, inside the leading parenthesis group).
2. `parse_define`: record `named_va: bool` on `Parsed` when a parameter has the `name...` form. `entry`: `def.loose_comma = (body.ok && shape.loose_comma) || p.named_va`. `permissive_define`: `shape.loose_comma || brackets::named_variadic(rest)`. `merge` and `close`: as for `unbalanced`. Add `loose_comma_count`.
3. Run task 7's tests (GREEN).
4. Add `21_macro_args.c` and `21_macro_args.cc`, regenerate both `obligations.jsonl` (8 and 4 new rows), extend E01 for the new case. Expected RED: a1 to a6, y1, y2.
5. `macro_call_arguments`: return
   ```rust
   pub(crate) struct MacroGroups { pub args: BTreeSet<String>, pub split: bool }
   ```
   and compute `split` in the existing group walk: keep the parenthesis `depth` logic as it is (group ends and chaining must not move), and beside it keep a stack of openers for the group; `Punct(b',')` with `{` or `[` on top sets `split`; a `}`/`]` that does not match the top, or a `Digraph`, sets `split`. Put the stack logic in `brackets.rs` if it keeps `macro_census.rs` smaller.
6. `c_prune.rs`: `Off::MacroComma`, `Off::MacroArgSplit`; `note_name`: `if facts.loose_comma { set_off(Off::MacroComma); }`; `scan`: after the walk input is computed, `if groups.split && off.is_none() { off = Some(Off::MacroArgSplit); }`, then `out.escaped.extend(groups.args)`.
7. Run `-- c_prune cpp_prune macro_census` (GREEN). The older rows must not move: in particular `13_fold2_macro_args` and the chained-invocation tests in `c_prune_fold2_tests.rs`.

### Task 9. Commit B
As task 6. Commit: `fix(dfg): C/C++ owners off on comma-carrying macros and brace-split macro arguments (#829)`.

### Task 10. Documentation
`CLAUDE.md` (keep the existing dense style; plain statements, no history of rounds):
- **"The C/C++ macro census …" paragraph:** the census also records `unbalanced` (replacement list not well-nested in `()`, `[]`, `{}`; any digraph bracket counts; brackets in literals and comments do not) and `loose_comma` (a comma or `__VA_ARGS__` whose innermost bracket in the body is not a parenthesis; a GNU named variadic counts); both are unioned over definitions, closed over referenced macros, read by token scan in parsed files (a pass that cannot read the body gives no verdict) and by token scan with a raw-byte fallback in permissive texts; both are in the digest (`c-macro-census-v3`).
- **"C rules U and D …" paragraph:** add the gates to the "An owner is off for …" list: an invoked in-repo macro with `unbalanced` or one of the listed external block macros (`Off::MacroBrackets`), one with `loose_comma` (`Off::MacroComma`), and a comma directly inside `{}` or `[]` within an in-repo macro's argument list (`Off::MacroArgSplit`); name the reason in one sentence each (the compiler's blocks differ from the parse; arguments are split at commas outside parentheses only). In the "Out of model (disclosed, owner D2)" sentence add: an external macro's expansion is well-nested and comma-tight, except the listed names. Add the open hole of OD-2 with its issue number, #831. Point to this design.
- **"C++ rules U and D …" paragraph:** one clause: the three gates apply to C++ owners unchanged.
- "CPG cache (currently v188)" → v189.
- Trace sentence: the new `off:` verdict names.
`scripts/mutgate/README.md`: no change.

### Task 11. Mutation lane
Create `mutants/c-macro-brackets.json` in the format of `mutants/rust-macro-scope.json` (`lane`, `source`, `lib_test_prefixes` naming the lib test modules, `mutations`: id → `[file, old text, new text, [killing tests]]`). Write the anchors against the final code. Mutants:

| id | mutation | killed by |
|---|---|---|
| MB-01 | `body_shape`: a closer pops without checking the opener kind | U06 |
| MB-02 | `body_shape`: a closer on an empty stack is ignored | U07 |
| MB-03 | `body_shape`: leftover openers at the end are ignored | U01, b1 |
| MB-04 | `tokenize_seq`: the digraph arms removed | U13, b5 |
| MB-05 | `tokenize_seq`: `[`/`]` not emitted | U05, b7 |
| MB-06 | `entry`: verdict taken even when the body did not tokenize (`body.ok &&` removed) | U17 (`MC` must stay false) |
| MB-07 | `merge`: `unbalanced` not ORed | U15, b9 |
| MB-08 | `close`: no propagation of `unbalanced` over `reaches` | U14, b2 |
| MB-09 | `permissive_define`: shape not applied | U22, mf2 |
| MB-10 | `digest`: the two new bytes omitted | U24, cz1 |
| MB-11 | `note_name`: the `facts.unbalanced` refusal removed | b1, x1, E01 |
| MB-12 | `BLOCK_MACROS` emptied | b12, x6 |
| MB-13 | `body_shape`: comma checked against "stack empty" only (brace-held commas pass) | L02, a5 |
| MB-14 | `body_shape`: `__VA_ARGS__` not treated as a comma carrier | L04, a6 |
| MB-15 | `__VA_OPT__` parenthesis not transparent | L05 |
| MB-16 | `macro_call_arguments`: `split` never set | a1, y1 |
| MB-17 | `note_name`: the `facts.loose_comma` refusal removed | a4, a5 |

MB-13 to MB-17 only with commit B. Running the lane follows the controller's current gate policy (cloud session); it is not a local step.

### Task 12. Verification (before reporting done)
1. `cargo fmt --all --check`.
2. Targeted tests through the memory guard, totals reported: `-- macro_census`, `-- c_macro_census`, `-- c_prune`, `-- cpp_prune`, `-- cpg_cache`, `-- dfg_codec`.
3. `cargo build --release`, then `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (the change is under `src/call_graph/`; paste any flip into the PR).
4. End to end with the release binary, on the probe directories: `.scratch829/design/run.sh .scratch829/design/<dir>` for `p_b`, `p_xf`, `p_cbe`, `p_ctrans`, `p_cif`, `p_cpth`, `p_carg`, `p_xarg`. Expected: `c-prune-owner off:MacroBrackets` (or `off:MacroArgSplit` for the two `arg` directories) and the pair listed in DESIGN section 2 present again. `p_csemi` and `p_xsemi` stay holes (OD-2): say so.
5. `python3 scripts/file_size.py` on every touched file; none over 2,000; report those over 1,000.
6. State what was not run: the full suite (CI), the mutation lane (controller), any corpus measurement.
7. Optional, only if the controller asks: on leveldb and the cpython sample, count owners per new `off:` verdict from the trace and put the three numbers in the PR body. Disclosed, not a gate (the owner waived yield measurement for this fix).

### Task 13. PR text
Title: `fix(dfg): C/C++ U/D refuse owners whose macros change bracket structure (#829)`. Body: the hole and the table of DESIGN section 2 (which rows are fixed, which stay open under OD-2); the three gates; the name list and its evidence; cache v189; the file-size note for `macro_census.rs`; rows added (counts per table); what was verified and what was not; the tier-2 mutation trigger is not met (no resolution result, execution context or test oracle changes), the PR adds lane `c-macro-brackets`.

## Review brief notes (for the controller)

- Goal sentence above; ask for WRONG/SMELL and MATERIAL/IMMATERIAL tags.
- The class is open at one known edge (DESIGN 4.6). A finding in that class is recorded against OD-2, not against this PR.
- Ask reviewers who claim "the compiler reads X" to attach the `clang -E` or run output.
- If a reviewer finds another way a well-nested, comma-tight body yields an unbalanced expansion, that is a counterexample to DESIGN section 7 and is MATERIAL: fix the rule, do not add a special case.

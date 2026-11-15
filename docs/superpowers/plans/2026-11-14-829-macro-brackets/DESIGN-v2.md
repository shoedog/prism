# #829: macros that change bracket structure (C/C++ rules U and D): final design (v2)

**Status:** design of record for the implementation. `DESIGN.md` is the tentative design and stays as history; `PLAN-v2.md` is the plan for this document.
**Base:** branch `fix/829-macro-brackets` at `9da89ed2` (gates still stubs), release binary of 2026-11-14 19:05, Apple clang 21.0.0.
**Owner decisions on this document (2026-11-14, `DECISIONS.md`):** OD-A candidate 10; OD-B 31 names; OD-C close the two-file form of A-span in this PR (the hidden-opener half of G10, sections 5.1 and 5.3); OD-D accept the interval cascade. Section 12 keeps the options as they were put.
**Binding decisions** (`DECISIONS.md`): the truth gate is STRICT (no dropped pair may be CONTRADICTED by the expansion-arm oracle, no per-case waiver); recall is a reported guide; boundary policy OD-6; a positive certificate is the direction (OD-7).
**Goal (for materiality):** rules U and D are removal-only. A dropped def→use pair must also be dropped when the same rules read what the compiler sees. Refusing an owner (keeping its pairs) is always acceptable; its cost is recall.
**Probes of this phase:** `.scratch829/redesign/` (git-excluded). The sources that matter are reproduced in section 11.

## 1. The rule in brief

An owner keeps rules U and D only when prism can prove **structure preservation**: every macro invocation in the owner's text expands to a well-nested token string that stands where the invocation stood, and no token of the owner is an argument of an invocation that starts outside it. Four checks give the proof; a failed check turns the owner off for U and D.

1. **Brackets (G7, `Off::MacroBrackets`).** The owner spells no in-repo macro whose replacement list, or that of a macro it names, does not nest in `()`, `{}`, `[]`; no listed external block macro; no in-repo macro that names a listed one.
2. **Body argument lists (G8, `Off::MacroBodySplit`).** No macro the owner spells (closed the same way) has a body in which an argument list holds a comma inside `{}` or `[]`, or a macro whose expansion can carry a comma outside its own parentheses.
3. **Owner argument lists (G9, `Off::MacroArgSplit`, `Off::MacroArgLoose`).** The same two conditions for the argument lists written in the owner's own text, except inside a function-call parenthesis that is itself inside the argument list.
4. **Not captured (G10, `Off::OwnerExposed`).** By the file's own parentheses, the owner does not lie inside an argument list opened before it or closed after it; and the file spells, before the owner, no in-repo macro whose expansion can leave a `(` open (the list could then be closed in the file that includes this one).

Compared with the tentative design: check 4 is new (the enclosing-group class); the blanket "the owner spells a macro with a comma outside parentheses" gate (G8 of `DESIGN.md`, 92% of its recall cost) is replaced by checks 2 and 3, which refuse only where such a comma can reach an argument list.

Scores (section 9): 0 must-keep violations on every population; recall cost 1.1% of the sound drops of the six corpora (399 of 37,777 pairs, 214 of 20,274 owners), against 10.5% for candidate 4b and 54.7% for 6b. Of the 399 pairs, 67 (106 owners, three cpython files) are the price of the hidden-opener half of check 4.

## 2. What the scoring left open, and what was found

`L4-SCORES.md` left three candidates with no violation: 4b (tentative plus the enclosing check), 8 (4b with a finer comma rule, tuned on the rows) and 6b (a shape certificate). The task was to choose with a mechanism-level argument, because the generator is not complete.

**Candidate 8 is unsound.** It refuses a loose-comma macro only where that macro is spelled inside a bracket. A macro whose own body splits an argument is spelled nowhere near a bracket:

```c
#define F(p, q) q if (c) p
#define G(p, q) q p
#define HI F({ int a, b; })
#define JI G({ int a, b; })
int body_split(int c) { int x = 41; int b = 0; { HI; x = 2; JI; return x; } }
```

Oracle (`probe.py probes/bodysplit`, C and C++): the pair `x@9 -> x@15` is dropped on head and CONTRADICTED; the expansion is `b ; } if ( c ) { int a ; x = 2 ; b ; } { int a ; return x ; }`. Candidate 8 admits the owner; 4b, 6b and the rule of this document refuse it. The class is outside the generator's grammar (its bodies hold no nested invocation), so the 0 in `L4-SCORES.md` for candidate 8 was a statement about the rows only. This is the reason the rule below is derived from an invariant and not from the rows.

**What each remaining option costs** (L3, CONFIRMED drops lost; all have 0 violations on the scored data):

| option | recall cost | argument | note |
|---|---|---|---|
| 4b: tentative + enclosing check | 10.5% (3,973) | sound by section 6 (it refuses a superset of what the rule needs) | 3,647 of the 3,973 pairs are owners that merely spell a macro such as `#define INIT {0, 0}` |
| 8: finer comma rule | 2.4% (912) | **unsound** (above) | rejected |
| 6b: shape certificate (body must be expression-shaped) | 54.7% (20,658) | also sound for brackets; additionally refuses every body with `;`, a control keyword, a brace or a declaration keyword | buys robustness against statement-level classes nobody has found (section 4.3) |
| 10: this document | 1.1% (399) | section 6 | chosen |
| 10 without shielded parentheses | 2.6% (979) | section 6 without lemma V | fallback if lemma V is not accepted (OD-A) |
| 10 + "a reachable body holds a top-level `;`" | 7.2% (2,730) | closes the #831 shape too | for information; #831 is not this lane |

Hybrids considered and not taken: a certificate whose failure only escapes names or only keeps pairs near the macro (no argument for locality: an unbalanced expansion shifts every enclosing block, `DESIGN.md` option K2); 6b with a looser shape (it would still pay for `;` bodies, 11,697 pairs, without a demonstrated hole).

## 3. Preprocessor facts the argument uses

PF1, PF2 and PF4 to PF6 were checked with Apple clang 21.0.0 (`.scratch829/redesign/probes/clang/`); PF3 is the standard's rule (C11 6.10.3.1 and 6.10.3.4), and the PF6 outputs are what it predicts.

| # | fact | check | result |
|---|---|---|---|
| PF1 | Arguments are collected from unexpanded tokens and split at commas outside **parentheses** only | `DESIGN.md` C9 | `F({ int a, bb; })` gives `bb; } { int a` |
| PF2 | A function-like macro name is invoked only when the next token is `(`; the look-ahead does not expand macros; a later rescan does invoke it | `pf2.c`: `K E (1, 2)` and `ID(K E (1, 2))` with `#define E` (empty) | `K (1, 2)` (not invoked); `[2 1]` (invoked on the rescan of `ID`'s argument) |
| PF3 | Each argument is fully macro-expanded before substitution; the result is rescanned with the rest of the source | standard | |
| PF4 | A `)` that comes from a macro does not close an argument list; a `(` that comes from a macro opens one | `pf5.c`, `pf5b.c` | `BEGIN .. END` with `#define END )`: "unterminated function-like macro invocation"; `BEGIN .. )` with `#define BEGIN W3(`: accepted |
| PF5 | An argument list may continue from an included file into the file that includes it; `#include` inside an argument list is an error | `spans_file_end.c`, `inc_in_args.c` | accepted (the function between is rewritten); "embedding a #include directive within macro arguments is not supported" |
| PF6 | Where the commas of an expansion land | `pf7.c` | `K(f(9, PAIR))` -> `<f(9, 1, 2)|>`; `K(PAIR)` -> `<1|2>`; `K(N(PAIR))` with `N(p) { p }` -> `<{ 1|2 }>`; `K(INIT)` -> `<{0|0}>` |

Not checked: GCC (not installed). PF5's second half is clang's behaviour; GCC documents directives inside macro arguments as undefined and may accept `#include` there (section 7, A-span).

## 4. The property

### 4.1 Terms
- **Brackets:** `( ) { } [ ]` as preprocessing tokens. A token string is **WN** when they nest.
- In a WN string every comma has an **innermost bracket**, or none. The comma is **tight** (it is `(`), **held** (it is `{` or `[`) or **bare** (none).
- A string is **comma-clean (CC)** when it is WN and every comma in it is tight. A CC string can be put anywhere, including inside braces inside an argument list, without creating a held or a bare comma.
- A **callee group** is the argument list of a function-like macro invocation, as the preprocessor delimits it when it collects it.
- **Reach:** the macros a body names, transitively (the census's existing closure over free identifiers that are in-repo macros). `f*` is fact `f` ORed over a macro and its reach.

### 4.2 Structure preservation (SP)
For an owner `O` with text `T_O`, in every configuration the compiler accepts:
- **SP1 (not captured).** No token of `O` belongs to the argument list of an invocation whose name token lies outside `O`.
- **SP2 (islands).** The compiler's tokens for `O` are `T_O` with each outermost invocation (a macro name and the argument groups it consumes) replaced by its expansion, and each such expansion is WN.

Consequence: every bracket of `T_O` outside invocations pairs for the compiler as tree-sitter pairs it; no bracket of an expansion encloses or releases an owner token outside the invocation. Block scopes, regions and statement nesting outside invocations are the ones rules U and D read. Tokens inside an invocation's parentheses are already escaped by name (E3), and the names a body reads, writes or declares are escaped through `free` and the parameters.

### 4.3 What SP does not cover
SP is about brackets. Two statement-level classes are outside it and keep their existing treatment:
- **Control-prefix bodies** (`if (c)`, `else`, `do`, `for (;;)`): tree-sitter reads the statement after the macro as a declaration typed by the macro (ineligible in `.c`, escaped in C++), a nested function, or an error (`DESIGN.md` 4.5). Evidence: the generator's `CTRL` fragments produced a contradicted key only together with `BR_UNBAL` or `PARAMS` (`L3-BASE.md`, keys table), at bound 3 exhaustively and in the bound-4 batches.
- **A top-level `;` in a body used inside a declaration** (#831, deferred by the owner; the test-form consult found no U or D drop in its probes).
A design that wants these closed by construction is 6b (54.7%) or "10 + top-level `;`" (7.0%, closes #831's shape only). OD-A names the choice.

## 5. The rule, exactly

### 5.1 Census facts (`src/call_graph/c/macro_census.rs`, new `macro_census/brackets.rs`)

Per definition, one ordered scan of the replacement list. The body tokenizer (`tokenize_seq`) gains the tokens `{ } [ ] ,` and the four digraph brackets beside identifiers and parentheses; literals, comments and raw strings are read as today, in both dialects.

```
stack entries: Paren{root} | VaOpt | Brace | Bracket        root: None | Param | Name(identifier)
"__VA_OPT__" directly before "(":   that "(" is pushed as VaOpt (transparent)
"(":   root = Param      when the token before it is a parameter or __VA_ARGS__, or the ")" of a VaOpt
              Name(t)    when it is any other identifier t
              the root of the paren that ")" closed, when the token before it is that ")"  (a chained group)
              None       otherwise                                         -> push Paren{root}
")":   top is Paren or VaOpt ? pop : unbalanced
"{" "[":  push          "}" "]":  top is its opener ? pop : unbalanced
digraph bracket (<% %> <: :>):  unbalanced          end with a non-empty stack: unbalanced
"," or the identifier __VA_ARGS__  (innermost = the top entry that is not VaOpt):
       innermost is not a Paren          -> loose_comma
       innermost is Brace or Bracket     -> for every Paren on the stack:
                                              root Param -> held_param;  root Name(t) -> held_after += t
identifier x that is not a parameter     -> for every Paren on the stack:
                                              root Param -> fwd_param += x;  root Name(t) -> fwd_after += (t, x)
```

Beside the stack, the scan matches parentheses alone (a second stack that ignores braces and square brackets: `(` and the `(` of `__VA_OPT__` push, `)` pops when it is not empty); when it is not empty at the end the body has `paren_open`.

Raw facts of a definition: `unbalanced`, `loose_comma`, `held_param`, `paren_open` (booleans) and `held_after`, `fwd_param`, `fwd_after` (name sets, not exported).
- **`unbalanced`:** the replacement list is not WN. Any digraph bracket sets it (the C++ `<::` exception is not modelled).
- **`loose_comma`:** some comma or `__VA_ARGS__` of the body is bare or held, so the expansion may not be CC. Same definition as the tentative fact.
- **`paren_open`:** the replacement list holds a `(` that no later `)` of it closes, counting parentheses alone. Its expansion can leave a parenthesis open for the source text that follows, which is how an argument list comes to start inside a macro (PF4). A stray `)` does not set it: a closer from a macro closes nothing (PF4). `paren_open` implies `unbalanced`.
- **Body callee group:** a paren group of the body that directly follows a parameter, an in-repo macro name, or the `)` of a body callee group (a chain). Which names are macros is known only at closure time, so the scan records the root of every group that follows an identifier: `held_param` / `held_after` say that such a group holds a held comma or `__VA_ARGS__` (at any depth), `fwd_param` / `fwd_after` name the non-parameter identifiers inside it.
- A paren after an identifier that is no in-repo macro (`f((T){ a, b })`, `sizeof(..)`, `if (..)`) is no callee group. The wider reading "after any identifier" scores the same on every population (candidate 10i, section 9) and would make the facts independent of the macro set, but it refuses every owner that calls a macro of the named-argument idiom, `#define foo(...) foo_impl((struct foo_opts){ __VA_ARGS__ })`; it is not taken.
- A GNU named variadic parameter (`args...`) sets `loose_comma` and `held_param` (fail closed; the name is not tracked).
- A pass that cannot read the body (`!ok`) contributes no fact; the definition is then soft-poisoned unless another pass read it, and that pass supplies the facts (existing rule).

Union and closure (`merge`, `close`): the booleans are ORed and the name sets are unioned over all definitions of a name (an `#undef` clears nothing). After the existing reach computation:

```
unbalanced*(M) = OR of unbalanced over M and reach(M)
loose*(M)      = OR of loose_comma over M and reach(M)
split(M)       = held_param(M)
                 or some t in held_after(M) is an in-repo macro
                 or some x in fwd_param(M) is an in-repo macro with loose*(x)
                 or some (t, x) in fwd_after(M): t is an in-repo macro, x is an in-repo macro with loose*(x)
arg_split*(M)  = OR of split over M and reach(M)
paren_open*(M) = OR of paren_open over M and reach(M)
```

`MacroFacts` exports the four closed booleans as `unbalanced`, `loose_comma`, `arg_split`, `paren_open`.

**Census texts that are not parsed C/C++ (`collect_permissive`).** The text after the name (parameter list included: its parentheses nest, its commas are tight, and it follows no identifier, so it is no callee group) is tokenized in both dialects; each dialect that reads it runs the scan with **every identifier treated as a parameter root** (the parameter list is not parsed, so a group after any identifier is a callee group and every identifier inside one is a forwarded name: an over-approximation, like the existing rule that keeps parameters in `free`), and the results are ORed. A line no dialect can tokenize sets `unbalanced`, `loose_comma`, `held_param` and `paren_open`; a named variadic in the leading group sets `loose_comma` and `held_param` (fail closed; this replaces the raw-byte automaton of `DESIGN.md` 6.2, which could miss a bracket next to an unterminated literal).

### 5.2 Owner checks (`c_prune::scan`, `.c` and C++ owners, after the existing gates)

- **G7 `Off::MacroBrackets`.** Some name the owner spells (the existing `spelled_identifiers`, whatever node the parser made of it) is in `BLOCK_MACROS` (section 5.4), or is an in-repo macro with `unbalanced`, or is an in-repo macro whose transitive `free` set holds a `BLOCK_MACROS` name.
- **G8 `Off::MacroBodySplit`.** Some in-repo macro the owner spells has `arg_split`.
- **G9 `Off::MacroArgSplit` / `Off::MacroArgLoose`.** One pass over the owner's normalized token stream (every variant `macro_call_arguments` reads today: both dialects, both splice modes; any variant that refuses, refuses) with a bracket stack. A paren is
  - a **callee paren** when it directly follows an in-repo macro name (object-like or function-like, any definition) or the `)` of a callee paren (a chained group, as `macro_call_arguments` walks them);
  - a **shielded paren** when it is opened inside a callee paren and directly follows an identifier that is no in-repo macro (a function call, a keyword such as `sizeof`, an external macro);
  - a plain paren otherwise. A paren after an identifier outside every callee paren shields nothing.
  A token inside at least one callee paren and inside no shielded paren is **checked**. A checked comma whose innermost bracket is `{` or `[` refuses (`MacroArgSplit`), as does a closer that does not match its opener or a digraph bracket inside a callee paren. A checked identifier that names an in-repo macro with `loose_comma` refuses (`MacroArgLoose`).
- **G10 `Off::OwnerExposed`.** Section 5.3.

All four gates apply to U and D. The first refusal found is reported; the order among gates is otherwise unchanged. `Off::MacroComma` (stub) is removed.

What the parser already guarantees, pinned by fact tests and not re-checked: an owner whose text holds a digraph bracket, or a conditional directive inside an argument list, is `G1Error` or no owner (`probes/digraph_owner`, `probes/if_in_args`); an owner without a parse error has nesting brackets in its own text, and each branch of a conditional inside it holds whole items of the list it stands in (statements, declarations, fields, enumerators), so every branch nests by itself. The G9 pass reads directive lines as tokens too, as `macro_call_arguments` does; a conditional's own parentheses nest on its line.

### 5.3 The exposed-owner test (new `src/data_flow/build/c_file_parens.rs`)

Input: the file's source and the byte ranges of its owner nodes (`parsed.all_functions()`, known in `data_flow/build/file.rs` before the first owner is scanned). Computed once per file, on first use, kept in a `OnceCell` beside `declared_types`.

1. **Events.** The file is cut at every owner start and end. Each piece goes through the census's phase code (`translate`, `splice`, `strip_comments`) and a lenient scan that emits, in order: `(` and `)` in code; `If` (`#if`, `#ifdef`, `#ifndef`), `Elif` (`#elif`, `#elifdef`, `#elifndef`), `Else`, `Endif`. Nothing else of a directive line counts (its parentheses, a `#define` body, an `#error` text). A string or character literal that does not end on its line ends there (code the compiler accepts has none; skipped text may). Cutting at owner boundaries gives every owner an exact position in the event list without offset maps; a cut is a token boundary outside comments and literals in tree-sitter's reading.
2. **Variants.** Trigraphs on and off, lenient and strict splices (variants with identical event lists are merged); dialect: C++ only for a file parsed as C++ (the dialect assumption of slice 6c; a C reading of a multi-line raw string would see stray parentheses), C and C++ for a `.c` file. Any variant that refuses, refuses.
3. **Intervals.** One forward pass keeps the interval `[lo, hi]` of the running balance (`(` adds 1, `)` subtracts 1, not clamped) over the branches of conditional compilation: at `If` push the current interval; at `Elif`/`Else` record the branch end and restart from the pushed interval; at `Endif` the interval becomes the hull of the branch ends, plus the pushed interval when no plain `#else` was seen (no branch may be taken).
4. **Interval test.** With `a` the owner's start and `b` its end: the owner is exposed when `hi(a) > min(0, least lo before a)` (in some configuration a literal `(` before the owner is still open at its start) or `least lo after b < hi(b)` (in some configuration a `)` after the owner closes something no literal `(` after the owner opened). An owner range that is not a cut is exposed (fail closed).

5. **Hidden opener (OD-C: the two-file form of A-span).** The scan also reads the identifiers of code (not of directive lines, comments or literals). The owner is exposed when, in some variant, a piece before its start cut holds an identifier that names an in-repo macro with `paren_open` (closed). `build` takes the census as a predicate; the test compares the ordinal of the first such piece with the ordinal of the owner's start cut. An owner that itself spells the macro is already off (G7); an owner before the first spelling is not affected.
   - *Why it is needed.* The interval test sees an argument list that a macro opens only through its literal `)` after the owner. When the owner's file is included by another file, that `)` can be in the includer (PF5), where no test on this file sees it (`probes/aspan`, section 11: the program returns 42, head drops the pair).
   - *What it refuses.* Every owner whose start lies after the first code spelling of such a macro in its file, whatever closes the parenthesis and wherever: also when a literal `)` closes it before the owner, and when the spelling is in a branch the build does not take. The spelling may be at file scope, inside another owner's text (`probes/aspan_sib`) or inside a declaration.
   - *Census inputs.* The fact is the union over every definition of the name in every census input, as for the other facts. A definition in an input the census does not read is A-inputs (#813). The union has a price: cpython defines a `CASE(NAME)` that opens a function call in one test file (`Modules/_testlimitedcapi/slots.c`, closed by its `ENDCASE()`), and three other files define and use a local, balanced `CASE` inside one function each; the owners after that use are refused (106 owners, 67 sound pairs; the other five corpora hold no `paren_open` macro at all, cpython one more, never spelled in a scoped file).
   - *Not taken:* applying the half only to files that some `#include` names (a file nothing includes cannot end inside an argument list, so the 67 pairs would return). It would rest on include visibility (computed and forced includes), the A-inputs family; a recall follow-up if wanted. Also not taken: recording only an unmatched `(` that follows a macro name or a parameter. That is not compositional: `N(Z)` with `#define Z (` and `#define N(p) W3 p` opens a list although no body holds "name, then `(`".

Costs of the interval test, disclosed: the hull does not correlate two conditionals with the same condition. A file that opens a parenthesis under `#ifdef X` and closes it under a later `#ifdef X` gets a wide interval, and every owner before and after the pair is refused (pinned by the scorer's self-test). Text under `#if 0` with unbalanced parentheses does the same to the owners after it. Neither occurs in the six corpora: the interval test refuses none of the 20,274 `on` owners (B8, the rule with the interval test and without the hidden-opener half, refuses exactly the 108 owners that B7, the rule without G10, refuses; candidates 10 and 10f agree on every owner).

### 5.4 External names (`BLOCK_MACROS`)

The 16 names of OD-3, plus 15 (OD-B). Evidence for the additions: a scan of public headers for replacement lists that do not nest (`extscan.py`):

| headers | not well-nested (statement level) | file scope only |
|---|---|---|
| macOS SDK `usr/include` (1,671 files, 39,795 macros) | `pthread_cleanup_push`, `pthread_cleanup_pop` | `__BEGIN_DECLS`/`__END_DECLS` and six library twins (`extern "C" {`): never inside an owner |
| CPython 3.11, 3.12 | `Py_BEGIN_ALLOW_THREADS`, `Py_END_ALLOW_THREADS`, `Py_TRASHCAN_BEGIN_CONDITION`, `Py_TRASHCAN_END`, `Py_TRASHCAN_SAFE_BEGIN`, `Py_TRASHCAN_SAFE_END`, `_Py_BEGIN_SUPPRESS_IPH`, `_Py_END_SUPPRESS_IPH` (+ `Py_TRASHCAN_BEGIN` through closure) | |
| CPython 3.13, 3.14 | the critical-section names of the list, `Py_BEGIN_CRITICAL_SECTION_SEQUENCE_FAST`, `Py_END_CRITICAL_SECTION_SEQUENCE_FAST`, the two IPH names; internal: `Py_BEGIN_CRITICAL_SECTION_MUT`, `..2_MUT`, `mi_commit_mask_foreach(_end)` | |
| NumPy 2.x | `NPY_BEGIN_SUPPRESS_IPH`, `NPY_END_SUPPRESS_IPH` | |

Added: `Py_TRASHCAN_BEGIN_CONDITION`, `Py_BEGIN_CRITICAL_SECTION_SEQUENCE_FAST`, `Py_END_CRITICAL_SECTION_SEQUENCE_FAST`, `_Py_BEGIN_SUPPRESS_IPH`, `_Py_END_SUPPRESS_IPH`, `NPY_BEGIN_ALLOW_THREADS`, `NPY_END_ALLOW_THREADS` (NumPy's aliases of the CPython pair; the wrapper closure cannot see a header outside the repository), `NPY_BEGIN_SUPPRESS_IPH`, `NPY_END_SUPPRESS_IPH`, and PostgreSQL's `PG_TRY`, `PG_CATCH`, `PG_FINALLY`, `PG_END_TRY`, `PG_ENSURE_ERROR_CLEANUP`, `PG_END_ENSURE_ERROR_CLEANUP` (from the fixture preludes, reproduced from `elog.h`; PostgreSQL headers were not scanned on this host). Not added: the two `_MUT` names and the mimalloc pair (internal headers an extension cannot include). Effect on the scored data: 4 more boundary rows closed (17 to 13 open), no recall change on the six corpora (cpython defines its own names in-repo).

### 5.5 Trace
`PRISM_C_PRUNE_TRACE=1`: `c-prune-owner<TAB>off:MacroBrackets|off:MacroBodySplit|off:MacroArgSplit|off:MacroArgLoose|off:OwnerExposed<TAB>file<TAB>line`. The L3 recall ledger groups restored pairs by these names.

### 5.6 Digest and cache
- `CMacroCensus::digest`: tag `c-macro-census-v3`; per-macro flag bytes `[control, poisoned, unbalanced, loose_comma, arg_split, paren_open]`. The name sets are not in the digest: they only feed `arg_split`, which also depends on the macro set, and every macro name is in the digest. The aggregate `c-census:` identity of the text inputs is computed from the same digest, so a census text whose new facts change moves it.
- `CACHE_VERSION` 188 to 189 in the PR that turns the gates on (a full hit would otherwise replay edges pruned under the old gates), with the changelog line and the pinned asserts. The exposed test reads the owner's own file, which is already in the cache identity, and `paren_open`, which is in the census digest: a partial hit with a different digest already rebuilds in full.

## 6. Why admitted owners satisfy SP

Premises for an owner `O` in file `F`: the existing gates (no parse error; no `#define`, `#undef`, `#include` or OpenMP pragma in the body; no reachable `##`, recursion or unreadable body; a complete census; no phase divergence), G7 to G10, and the assumptions of section 7. "Valid" means a configuration the compiler accepts; the oracle and the row tests judge only those.

**Lemma P (parentheses never unbalance).** Every expansion is balanced in parentheses. Bodies are WN (G7, closed over reach; names that arrive through arguments are spelled by the owner, so G7 covers them too). An argument is a piece between commas at parenthesis depth 0 (PF1), so it is balanced in parentheses whatever its braces do. Substitution and rescanning preserve the balance. So only `{}` and `[]` can go wrong, and only through an argument cut at a held comma.

**Lemma 1 (a clean macro keeps arguments clean).** Let `M` be an in-repo macro with `unbalanced` and `arg_split` false, invoked with arguments that are CC after full expansion. Then (a) every callee group collected while `M`'s expansion is rescanned has arguments that are CC after full expansion; (b) the expansion is WN; (c) if `loose_comma` is also false, the expansion is CC.
*Proof,* by induction on expansion depth (finite: recursion is poisoned; a name that is not re-expanded stays a plain token, which cannot unbalance anything). After substitution the string is the body with each parameter replaced by a CC string and `__VA_ARGS__` by CC strings joined with commas. A callee group met during the rescan opens at a `(` that directly follows a function-like macro name. That `(` is either
- a body paren that follows a parameter, an in-repo macro name or a chained `)`: a body callee group. `arg_split` is false, so inside it no body comma and no `__VA_ARGS__` is held, and every comma of the group is either a separator (depth 0) or tight in a nested paren; parameter strings are CC and bring no bare comma. Each argument is CC before expansion. The macros named in it are literal body names inside the group (forwarded names, so `loose*` is false for each, or `split` would be true) or names inside parameter strings (spelled inside a callee group upstream, where the same condition held). By (c) their expansions are CC, so the argument is CC after full expansion; or
- a paren inside one parameter string. The string is CC and already fully expanded, so the group's arguments are CC and stay so (this is the deferred case of PF2: a name that met its `(` only after a neighbour expanded); or
- a body paren after any other token (an identifier that is no in-repo macro included), or at the start of the body. The token before it is no macro name, or is a name the scan already passed without invoking it (PF2), so the paren is no callee group in this rescan. It can become one only in a later rescan, that is, when the whole expansion is part of an argument; `M` is then named inside a callee group, `loose_comma` is false, and by (c) the expansion is CC: every paren group of a CC string has CC arguments.
(b) follows from WN bodies, WN arguments and (b) for nested invocations. For (c): the body's commas are tight, `__VA_ARGS__` stands at tight positions, the substituted strings are CC and nested expansions are CC by induction (`loose_comma` is closed over reach). ∎

**Lemma V (a shielded paren is a unit).** Let `P` be a shielded paren of the owner: opened inside a callee paren `G`, directly after an identifier `f` that is no in-repo macro. Then (a) `P` never delimits a macro's arguments, so `f ( .. )` stays in the token stream as part of the argument of `G` it was written in, whole, wherever that argument is substituted, dropped or duplicated; (b) no comma inside `P` is ever at depth 0 of a callee group that contains `P`; (c) in a valid program the tokens between `P`'s parentheses nest.
*Proof.* (a) and (b): `f` is fixed and is no macro (PF2), and arguments are substituted whole. (c): by lemma P everything that expands inside `P` is balanced in parentheses, so `P`'s `(` still pairs with `P`'s `)`; a C or C++ parser accepts no parenthesized token sequence whose braces or square brackets do not nest. ∎
So whatever happens inside `P` (a held comma, a loose macro, a split) stays inside `P`, inside `G`'s invocation, whose identifiers are all escaped by E3, and leaves `P` WN. A paren after an identifier **outside** every callee paren is different: an invocation inside it is an outermost invocation of the owner, and a split there re-pairs the owner's own braces (`probes/outer_shield`, section 11: valid, dropped on head, CONTRADICTED). That is why only parens opened inside a callee paren shield.

**Lemma 2 (owner groups are clean).** Every argument of a callee group of `O` is, after full expansion, WN, and every comma of it is tight or lies inside a shielded-paren unit.
*Proof.* A callee group is the group after a macro name that the preprocessor invokes there (the pass treats every in-repo macro name before a `(` as one, and every chained group as one: both over-approximate, covering an object-like alias and an expansion that ends in a function-like name). By G9 no checked comma is held, so outside shielded units every comma of an argument is tight; the units are WN and opaque (lemma V). Macros named at checked positions have `loose_comma` false, so by lemma 1(c) their expansions are CC. A nested callee group at a checked position is itself a callee group of `O` and satisfies the same conditions. ∎
Lemma 1 was stated for CC arguments; it holds unchanged for the arguments of lemma 2, with each shielded unit treated as one opaque WN token: the unit brings no comma to any depth-0 position (lemma V(b)).

**Lemma O (no open parenthesis without `paren_open`).** If no body reachable from the macro names spelled in a stretch of code has `paren_open`, no expansion in that stretch leaves a parenthesis open at its end.
*Proof.* Count parentheses alone and reduce a token string to `)`..`)` `(`..`(`: its unmatched closers, then its unmatched openers. A body without `paren_open` reduces to closers only. The arguments of an invocation are balanced as collected (PF1), and an argument expanded before substitution (PF3) is, by the same count, without unmatched openers. Expansion replaces a name, with its balanced argument groups, by a string built from such bodies and arguments; putting closers-only strings into a string adds no unmatched opener, and removing balanced groups adds none. By induction over the expansion steps, a stretch that starts with no unmatched opener from a macro ends with none. (A rule that looks only for "a macro name, then an unmatched `(`" in one body is not enough: `#define Z (`, `#define N(p) W3 p`, `N(Z)`.) ∎

**Lemma 3 (not captured).** Under A-span, G10 implies SP1.
*Proof.* Suppose a token of `O` is an argument of an invocation whose name lies outside `O`. Its opening parenthesis is one of three things. (i) A literal `(` of `F` before `O`: its match by raw counting is where the list ends (PF1), so it is still open at `O`'s start: the first half of the interval test. (ii) A `(` from the expansion of an in-repo macro in the code of `F` before `O`: by lemma O some name spelled there has `paren_open`: the hidden-opener half, wherever the list closes. (iii) A `(` from a macro defined outside the repository, or from the tail of a file that `F` includes (PF5): the list ends at a literal `)` that no literal `(` after `O` opened (a `)` from a macro cannot end it, PF4): the second half of the interval test when that `)` is in `F`; A-span is the assumption that it is. An opener in a file that includes `F`, before the `#include`, is not possible (PF5). The interval contains the balance of every configuration, and the lexing variants contain the compiler's reading. ∎

**Theorem.** If `O` passes the existing gates and G7 to G10, then under the assumptions of section 7 `O` satisfies SP in every valid configuration.
*Proof.* SP1 is lemma 3. For SP2 take an outermost invocation of `O`. Its name is an in-repo macro spelled by `O` (an external one is covered by A-ext), so `unbalanced` and `arg_split` are false (G7, G8). Its arguments, if any, are clean by lemma 2. By lemma 1(b) its expansion is WN. If the expansion ends in a function-like macro name and the next owner token is `(`, that group is a chained callee group of `O`, and the same argument applies. The invocation consumes exactly the groups the pass treats as its argument groups, so the owner tokens outside invocations are unchanged. ∎

**Which premise each known class breaks** (classes of `L4-SCORES.md` section 4, plus this phase's):

| class | example | premise | ablation that reopens it (section 9) |
|---|---|---|---|
| unbalanced body, alias chain, wrapper of a listed name (BR_UNBAL, W) | `BEGIN`/`END`, `NPY_BEGIN_ALLOW_THREADS` in-repo wrapper | G7 | B1: 53 + 14 + 48 rows, 4,956 L2 programs; B2: 2 rows |
| owner inside an enclosing group (E) | `J(int f() {..} , .. )` | G10 | B7: 16 + 4 + 50 rows, 1,947 L2 programs |
| argument split at the owner (S, D) | `F({ int a, bb; })`, `SF(({ int a, b; }))` | G9 held | B3: 1 + 11 rows |
| loose macro re-split on a rescan | `H(INIT)` | G9 loose | B4: 13 + 1 rows |
| **split inside a body** (new) | `#define HI F({ int a, b; })` | G8, held comma | B5: none of the frozen rows: new rows required (section 10) |
| **argument list opened by a macro before the owner, closed in the including file** (new, OD-C) | `BEGIN_IMPL` with `#define BEGIN_IMPL W3(`; the `)` in `unity.c` | G10, hidden opener | B8: none of the frozen rows: new rows required (the oracle cannot judge them: section 10) |
| **loose macro inside a body's group** (new rows; candidate 8 handled it) | `#define HI H(INIT)`, `#define DF ID(F BR)` | G8, forwarded loose macro | B6: none of the frozen rows: new rows required |

## 7. Boundary policy per assumption (OD-6)

| assumption | statement | policy | rows |
|---|---|---|---|
| **A-inputs** | the census reads every in-repo definition the compiler uses | pinned holes for #813: unchanged, 21 rows assert today's drop | no predicate can close them |
| **A-ext** | a macro defined outside the repository acts as a function: its expansion is WN and CC, it passes each argument on whole, and it does not forward to an in-repo macro; except the names of `BLOCK_MACROS` | disclosed (D2, wording extended in CLAUDE.md); the listed names are closed in code; an in-repo wrapper of a listed name is closed by the free-name closure | 41 boundary guide rows: 24 closed by the 16 names, 28 by the 31 (OD-B); the rest stay guide rows (external aliases and statement-prefix macros) |
| **A-config** | the definition in effect is one the census read (`-D`, a forced include) | disclosed | guide rows |
| **A-dialect** | C99 or later, C++11 or later, standard preprocessing | disclosed; the one witness (`33 g2o03`, a digit separator) is closed because the census ORs both dialects | pinned row flips to kept |
| **A-encoding** | UTF-8 or ASCII-compatible sources | disclosed | |
| **A-span** (new; narrowed by OD-C) | an argument list that a macro defined outside the repository opens, or that is still open at the end of a file the owner's file includes, closes before the end of the owner's file | the form with an in-repo macro spelled in the owner's file is **closed** (section 5.3, step 5). The rest is disclosed, with one pinned-hole witness and its runtime receipt (`probes/aspan3`: `open.h` ends with a literal `W3(`, `impl.c` includes it, `unity.c` includes `impl.c` and holds the `)`; the program returns 42 and head drops the live pair; no candidate refuses it, 6b included) | `aspan`, `aspan_sib` and the C++ twin are must-keep rows by receipt (section 10); a file nothing includes needs no assumption (an open list at its end is a compile error) |

## 8. Where the scorer's emulation differs from prism, and whether the design depends on it

| difference (`L4-SCORES.md` section 6) | direction for the real rule | does a score depend on it? |
|---|---|---|
| one lexer pass for definitions, no C/C++ dialect split, no soft poison | the real census ORs more passes: it refuses a superset | no violation can appear; recall may be slightly lower than scored |
| census input set approximated; `incomplete` not emulated | same facts where the inputs agree; `incomplete` turns everything off | no |
| census texts read by the same lexer, parameter lists parsed; the real rule fails closed on an unreadable line and treats every identifier of a census text as a parameter root | superset of refusals | no |
| owners located by a token rule; the scorer's owner can start before the real one (it swallowed `W3(` in the first `hull2` probe and reported `arg_split`) | the real owner is the tree-sitter node; such an owner is refused by G10 instead | checked two ways (`cands_diag.py`): with the exposed test alone, every row that needs it is refused (50 of 50 generator rows, 16 pair rows, 4 inline rows, 1,947 L2 programs), and the two groups do not overlap; and candidate 10 on an owner cut down to start after every leading macro token (`10-narrow`, the worst case of the difference) scores the same on every population (0 violations, 399 pairs). The Rust row tests are the final check |
| open group: first-branch-wins for `#if`, parentheses read by one lexer | the real test is the interval hull over all lexing variants: a superset | no (10 and 10f agree on every owner of every population) |
| callee and shield use `db.defined` over the emulated inputs | same rule on the real census | no |
| hidden opener: the first code identifier of the file with `paren_open`, compared with the scorer's owner start | the same test on the real pieces and census (more lexing variants: a superset) | no; an owner the scorer starts too early spells the macro itself and is refused by G7, as in prism |
| verdicts are those of the base binary under one host configuration | unchanged by the design | the L3 compare with the head binary is the gate (PLAN-v2 task 9) |

The scores are evidence for the rows' classes. The argument of section 6 is what covers shapes the rows do not hold; the two new body classes show the difference.

## 9. Scores

`python3 scripts/dfg_prune/xscore.py run --out .scratch829/redesign/l4v2` (17 s; candidates 10.. and B.. in `xscore_candidates.py`; the 15 earlier candidates score exactly as in `L4-SCORES.md`, 150 of 150 cells). Cells: must-keep pairs still dropped / must-keep pairs head drops; `c` = CONFIRMED drops lost.

| candidate | L1 pair | L1 inline | L1 generator | L2 bound 3 | L2 bound 4 batches | L3 (6 corpora) | L3 recall cost | owners refused | pinned open | boundary open |
|---|---|---|---|---|---|---|---|---|---|---|
| **10 (this design)** | 0 / 113 | 0 / 30 | 0 / 98 | 0 / 6,903 | 0 / 8,566 | 0 / 50 | 1.1% (399 / 37,777) | 214 / 20,274 | 21 / 23 | 17 / 41 |
| 10x: 10 with the 31 names (OD-B) | 0 | 0 | 0 | 0 | 0 | 0 | 1.1% (399) | 214 | 21 | 13 |
| 10n: 10 without shielded parens | 0 | 0 | 0 | 0 | 0 | 0 | 2.6% (979) | 232 | 21 | 17 |
| 10i: body callee groups after any identifier | 0 | 0 | 0 | 0 | 0 | 0 | 1.1% (399) | 214 | 21 | 17 |
| 10f: first-branch-wins enclosing test | 0 | 0 | 0 | 0 | 0 | 0 | 1.1% (399) | 214 | 21 | 17 |
| 10s: 10 + top-level `;` in a reachable body | 0 | 0 | 0 | 0 | 0 | 0 | 7.2% (2,730) | 374 | 21 | 17 |
| 4b | 0 | 0 | 0 | 0 | 0 | 0 | 10.5% (3,973) | 474 | 21 | 17 |
| 8 | 0 | 0 | 0 | 0 | 0 | 0 | 2.4% (912) | 130 | 21 | 17 |
| 6b | 0 | 0 | 0 | 0 | 0 | 0 | 54.7% (20,658) | 3,724 | 21 | 13 |
| B1: 10 without `unbalanced` | 53 | 14 | 48 | 4,956 | | 0 | | | | |
| B2: 10 without the list and wrapper closure | 2 | 0 | 0 | 0 | | 0 | | | 22 | 40 |
| B3: 10 without G9 held | 1 | 11 | 0 | 0 | | 0 | | | | |
| B4: 10 without G9 loose | 13 | 1 | 0 | 0 | | 0 | | | | |
| B5: 10 without G8's held-comma half | 0 | 0 | 0 | 0 | 0 | 0 | | | | |
| B6: 10 without G8's forwarded-macro half | 0 | 0 | 0 | 0 | 0 | 0 | | | | |
| B7: 10 without G10 | 16 | 4 | 50 | 1,947 | 1,937 | 0 | | | | |
| B8: 10 without G10's hidden-opener half | 0 | 0 | 0 | 0 | 0 | 0 | 0.9% (332) | 108 | 21 | 17 |

Candidate 10: recall cost by corpus: cpython 399 of 15,220 (2.6%; `block_list` 275, hidden opener 67, `unbalanced` 51, `MacroArgLoose` 6), 0 on redis, leveldb, protobuf, re2 and snappy. Owners: cpython 212 of 5,425 (106 of them by the hidden opener: section 5.3), redis 1, protobuf 1. The 326 `block_list` and `unbalanced` pairs are critical-section and lock macros around plain blocks; only a rule-D slice that sees through unconditional nested blocks recovers them (`DECISIONS.md` OD-3). Without shields, 580 more pairs are lost in cpython's generated `Parser/parser.c`: `CHECK(.., _PyAST_X(.., EXTRA))`, where `EXTRA` is five comma-separated arguments inside a function-call parenthesis. L1 guide cost: 16 of 30 pair-row and 16 of 33 inline-row CONFIRMED drops are refused (rows built to sit next to hazards); L2: 2,767 of 16,330 (adversarial bodies, not a recall estimate).

The probes of section 11 against the candidates (`evalprobe.py`): `bodysplit` (c, cc) CONTRADICTED: 8 admits, 10 refuses, B5 admits. `bodyfwd`, `deferred` (c, cc) CONTRADICTED: 10 refuses, B6 admits. `shield_nested` (c, cc) CONTRADICTED: 10 refuses. `outer_shield` CONTRADICTED: 10 refuses. `shield_safe` all CONFIRMED: 10 admits, 10n and 8 refuse. `hull2` CONTRADICTED, runtime 42: 10 refuses (`exposed_open_before`). `aspan` (c, cc) and `aspan_sib`: runtime 42, head drops the pair; 10 refuses (`exposed_hidden_opener`), B8, 4b and 6b admit; the oracle says CONFIRMED (section 10). `aspan3`: runtime 42, every candidate admits, 6b included (section 7).

## 10. Tests that follow from the design

- **New must-keep rows** (red on base; labelled by the oracle, C and C++ twins): `bodysplit`, `bodyfwd`, `deferred`, `shield_nested`, `outer_shield`, `hull2` (an opener and a closer under `#else` branches). They give B5 and B6 a violated row each, which is premise coverage for every premise of section 6.
- **Guide rows:** `shield_safe` (CONFIRMED and admitted: the recall the shield buys).
- **Hidden opener (OD-C).** Must-keep rows `aspan` (C and C++; the opener macro at file scope) and `aspan_sib` (the opener macro inside a sibling owner); a control with the same macro spelled only after the owner (not refused). Pinned hole: `aspan3` (A-span as narrowed).
  **Why the oracle misjudges these rows, and their truth basis.** The X arm replaces each `.c`/`.cc` file by its own post-expansion tokens, lexed with that file as the main file. `impl.c` alone never sees `defs.h` (only `unity.c` includes it), so `BEGIN_IMPL` is a plain identifier there, the "expansion" equals the source, and the rules drop the pair on it too: CONFIRMED. In `unity.c`'s stream, where the macro does rewrite the function, the tokens spelled in `impl.c` are not written anywhere (only main-file tokens are kept). The verdict describes a compilation that never happens. So: a row may declare its translation unit (`tu`); when the pair's file is not the `tu`, the oracle records `UNJUDGED` (`target-not-a-translation-unit`) instead of judging the file alone, and the existing receipt rule applies (a runtime receipt outranks UNJUDGED, never CONFIRMED or CONTRADICTED): `overrides.json` gives `assert: kept` (for `aspan3`: `pinned_hole`, assumption `A-span`) with `basis: receipt`. The receipt is the program itself: `main` returns `f(0)`, 42 exactly when the first def of `x` reaches the `return` (`xarm_receipts.py`, not run in CI). Optional, not required: judge such a row from the `tu`'s token stream by writing the tokens spelled in the pair's file to that file's X text.
- **Generator (L2 addendum).** The grammar cannot derive a body that holds an invocation. Add a `nest` family: an outer object or function-like macro whose body is `K(<fragments>)` or `ID(K <macro>)` with an inner macro from the enclosing-macro fragment set, at statement and guard sites. One offline run at bound 3 for the family; new keys are frozen as rows. Budget: two hours of machine time; if it yields a contradicted key that candidate 10 admits, stop and return to design.
- **Fact tests** for the automaton (group roots, held commas, forwarded names, chained groups, `__VA_OPT__`, named variadic, permissive fail-closed), the closure (`arg_split` through reach, through a forwarded loose macro, and not for a group after a non-macro identifier), the digest, the owner pass (callee, shielded, plain parens; chained groups; a paren after an identifier outside a callee paren), `paren_open` (own, through reach, `) (`, a brace-only block macro, `__VA_OPT__`), the event scan, the interval test (conditional hull, `#if 0` text, directive lines, the cascade) and the hidden opener (before, inside and after the owner; in a directive line; in a comment), and the parser guarantees of section 5.2.

**The 80 quarantined tests.** The facts `unbalanced` and `loose_comma` keep the tentative definitions, so their tests return as fact tests:

| file | tests | disposition |
|---|---|---|
| `macro_census_brackets_tests.rs` | 38 | un-ignore unchanged (`u22` and `l11` already agree with the fail-closed reading of an unreadable permissive line) |
| `macro_census_brackets_r1_tests.rs` | 21 | un-ignore; one expectation changes: `g_sol_11_permissive_unreadable_line_with_only_a_comma` now has `unbalanced` true as well (a permissive line no dialect reads sets every fact); the digest tests gain an `arg_split` twin |
| `macro_census_brackets_r2_tests.rs`, `_r3_tests.rs` | 7 + 8 | un-ignore unchanged (lexical readings of bodies) |
| `c_macro_census_tests.rs` | 4 | un-ignore; the digest tag is `v3` and the flag bytes are six |
| `c_prune_brackets_tests.rs`, `c_prune_brackets_r1_tests.rs` | 1 + 1 | un-ignore; both pin the name list: update to the decided list (OD-B) |

None is deleted. A test that fails after un-ignoring for a reason other than the named changes (one expectation, the digest tag, the name list) is a finding for review, not something to re-baseline.

## 11. Probe sources (git-excluded originals in `.scratch829/redesign/probes/`)

```c
/* bodyfwd */                               /* deferred */
#define F(p, q) q if (c) p                  #define ID(v) v
#define G(p, q) q p                         #define BR ({ int a, b; })
#define H(p) F(p)                           #define DF ID(F BR)
#define J(p) G(p)                           #define DG ID(G BR)
#define INIT { int a, b; }                  /* owner: { DF; x = 2; DG; return x; } */
#define HI H(INIT)
#define JI J(INIT)                          /* shield_nested */
/* owner: { HI; x = 2; JI; return x; } */   #define BLK(p) { p }
                                            #define AB int a, b;
/* shield_safe: all CONFIRMED */            /* owner: { H(BLK(AB)); x = 2; J(BLK(AB)); return x; } */
#define PAIR 1, 2
#define ARR (int[]){ 3, 4 }
#define H(p) FIRST(p)
#define FIRST(p, ...) p
/* owner: H(two(PAIR)); H(one(ARR)); x = 2; return x; */
```

```c
/* outer_shield: `one(` is outside every callee paren and shields nothing */
int outer_shield(int c) {
    int x = 41; int b = 0;
    one(({ { F({ int a, b; }); x = 2; G({ int a, b; }); sink(x); } 1; }));
    return 0;
}
/* hull2 */                                  /* aspan: defs.h, impl.c, unity.c */
#define W3(p, q, r) p; r                     #define W3(p, q, r) p; r
#if defined(NEVER_DEFINED_829)               #define BEGIN_IMPL W3(
#else                                        /* impl.c */
W3(                                          BEGIN_IMPL
#endif                                       int dummy829;
int dummy829;                                int f(int c) { int x = 41; int b; int a = 0, d;
int f(int c) { int x = 41; int b;                x = 2; d = 0, a = 1; return x + a; }
    int a = 0, d; x = 2; d = 0, a = 1;       /* unity.c */
    return x + a; }                          #include "defs.h"
#if defined(NEVER_DEFINED_829)               #include "impl.c"
#else                                        )
)                                            int main(void) { return f(0); }
#endif
```
```c
/* aspan_sib: defs.h */                      /* aspan3: defs.h holds W3 only */
#define W4(o, p, q, r) o p; r                /* open.h */
#define OPEN_IMPL W4(                        int before829;
/* impl.c */                                 W3(
int g(int c) {                               /* impl.c */
    return c OPEN_IMPL ; }                   #include "open.h"
, int f(int c) { int x = 41; int b;          int dummy829;
    int a = 0, d; x = 2; d = 0, a = 1;       int f(int c) { .. as aspan .. }
    return x + a; }                          /* unity.c: #include "defs.h", #include "impl.c", ")" */
/* unity.c: #include "defs.h", #include "impl.c", ")", main returns f(0) + g(0) */
/* aspan C++ twin: the owner is { int x = 41; int b = 0; int a = 0, d; x = 2; int e = 0, b; return x + 1; } */
```
(In the probes each statement of an owner is on its own line: a same-line pair is never dropped.)

## 12. Owner decisions

**OWNER-DECISION OD-A: which rule.** *Decided: candidate 10.* (The percentages below predate the hidden-opener half; section 9 has the current ones.) Recommended default: **candidate 10** as specified (0 violations on every population; 0.9% recall cost; the argument of section 6). The trade-offs:
- 10 without shielded parens (10n): the argument loses lemma V, the one step that appeals to "a program the compiler accepts"; cost 2.4% instead of 0.9% (580 pairs of one generated cpython file). Take this if the owner wants the proof to rest on token mechanics only.
- 4b: simplest to state, same soundness (it refuses a superset), 10.5%.
- 6b, or 10 + top-level `;`: also close statement-level shapes by construction (section 4.3), 54.7% or 7.0%. No hole of those classes is known; #831 is deferred.

**OWNER-DECISION OD-B: external-list additions.** *Decided: 31 names.* Recommended default: **add the 15 names of section 5.4** (31 in all). Listing a name only refuses more; the six corpora lose nothing. Alternative: add only `Py_TRASHCAN_BEGIN_CONDITION` and the two NumPy allow-threads aliases (the names the rows already use) and leave the rest to D2.

**OWNER-DECISION OD-C: A-span.** *Decided 2026-11-14: close the two-file form in this PR.* Specified in sections 5.1 (`paren_open`), 5.3 step 5, lemma O and lemma 3; cost 67 pairs and 106 owners in cpython; the three-file form and external openers stay the disclosed A-span. The text as it was put: Recommended default: **disclose it and pin the witness.** It needs one macro call split across two files with the opener hidden in a macro; the oracle cannot judge it; no candidate closes it. Alternative (about 40 lines, expected recall cost 0): a census fact "some reachable body has an unmatched `(`" and "the file spells such a macro: its owners are off", which closes the two-file form and leaves the form where an included file's tail opens the list.

**OWNER-DECISION OD-D: the interval test's cascade (section 5.3).** *Decided: accept.* Recommended default: **accept it** (sound, simple, 0 owners on six corpora) and let the L3 ledger show `off:OwnerExposed` counts; correlate equal conditions only if a corpus shows a cost.

## 13. Verified and not verified

**Verified by running it here:** PF1 to PF6 with clang; the probes with the oracle (C and C++ twins of five) and with every candidate; the runtime receipts of `hull2`, `aspan` (C and C++), `aspan_sib` and `aspan3` (each returns 42); the header scans; the scores above, the bound-4 batches included; that the scorer changes leave the 15 earlier candidates' 150 cells unchanged; `xscore.py selftest`; that digraph owners and conditionals inside argument lists are `G1Error` on head.
**Not verified:** GCC's behaviour for PF2 to PF5; PostgreSQL headers (names taken from the fixture preludes); the rule in Rust (the scores are an emulation: section 8); the `nest` generator family (not written); the runtime cost of the exposed test (one extra pass over each C/C++ file with an `on` owner: to be measured, PLAN-v2 task 9); corpora beyond the six.

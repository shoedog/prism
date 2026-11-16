# #829: macros that change bracket structure (C/C++ rules U and D): amended design (v2, review repair)

**Status:** amended proposal after sol round 1 **REJECT**; candidate **12-all-roots** is recommended. Do not implement rejected candidate 10 or treat 10n as a sound fallback. OWNER-DECISION OD-A2 (section 12) supersedes OD-A only when the owner accepts it; `DECISIONS.md` is unchanged historical authority. `PLAN-v2.md` follows this proposal.
**Checked checkout:** `b608adc590efeb376b7ce080a3d3782355cf44cd`; unchanged product binary `slicing 4.0.0 (0273cf3324cb)`, Apple clang 21.0.0. The gates remain stubs; scores emulate refusal, not an implemented fix.
**Binding decisions:** STRICT expansion-arm truth gate, no per-case waivers; OD-B 31 names; OD-C close the two-file A-span form; OD-D accept G10's interval cascade. Existing #813/A-ext/A-span boundaries are retained, not enlarged to excuse the rejection.
**Goal:** a source def→use pair dropped by rules U/D must also be dropped by the same rules on clang's expansion. Refusing the owner keeps its pairs and is sound; recall is a guide.
**Evidence:** `.scratch829/astra/` (current execution), `.scratch829/review-sol/` (supplied review), `.scratch829/redesign/` (historical design). Section 13 separates them.

## 1. The amended rule in brief

An owner retains U/D only with a structure-preservation certificate:

1. **G7:** refuse a spelled/reachable macro with a non-nesting body, a listed external block name, or its in-repo wrapper.
2. **G8:** refuse a spelled/reachable macro whose body has a **potential callee group** containing a brace/bracket-held comma or a reachable loose-comma macro. A potential group follows **any identifier**, a parameter, or the close of another potential group. Census non-membership is not a stable-name certificate.
3. **G9:** apply the same checks to **every position** inside potential groups in the owner's text. **No shields.** Groups after unknown identifiers and their chains count too.
4. **G10:** keep the accepted interval and hidden-opener refusal unchanged: an owner must not be captured by an argument list beginning outside it.

Candidate 12 costs **2,246 / 37,777 = 5.95%** of the six corpora's confirmed drops; **432 / 20,274 owners** refused. All scored must-keep populations have zero violations. This misses the ideal 2.4% target. Plain 10n + 31 names costs 979 pairs (2.59%) but admits the new **external identity-wrapper** witnesses, so cannot be the fallback under STRICT. No corpus frequency claim justifies admitting them.

The implementation is simpler than a stable-identifier or parameter-flow certificate: no external-name allowlist, no compiler dependency, no shield state, and body roots no longer depend on membership in the census. Recovering the cost through a separate proved comma/parameter-flow certificate is deferred, not assumed.

## 2. Rejection, sibling enumeration, and choice

**WRONG / MATERIAL, inherited and reproduced:** candidate 10 admits sol's `ID(sizeof E({ int a, b; }, c))` when external `E` expands to nothing. Argument prescan erases `E`; the rescan of `ID` invokes the in-repo `sizeof(p,q,r)` on the formerly shielded group. Its brace-held comma splits the argument. Both owners are on; T=0, Tb=1, X=1, Xb=1 for the live `x` pair; runtime prints `probe(0)=41 probe(1)=2`.

**WRONG / MATERIAL, new:** removing shields alone is insufficient. Make `ID` external as `EXT(v) v`, also keep `E` external/empty, and spell `EXT(sizeof E(...))`. Candidate 10n recognizes no callee group at all. Both external definitions satisfy the written A-ext properties: empty/identity expansion, WN/CC replacement tokens, whole-argument substitution, no introduced in-repo callee. The same contradiction occurs inside an in-repo body `#define HI EXT(sizeof E(...))`, with the owner spelling only `HI`. G8 also needs broader roots. These are not #813 input holes: the outside header is outside the indexed repository and every in-repo definition is present.

The same scan change refuses external object-like aliases to a known function-like macro (`E -> F`) in owner text and macro bodies, including C++ twins. Those aliases additionally cross the existing A-ext no-forwarding boundary; they are useful must-keep regression rows, but **the identity-wrapper witnesses independently require this repair within the existing model**. No decision to admit a contradiction or strengthen A-ext is proposed.

Two design rounds completed: round 1 reproduced erasure/aliases and measured candidates 11 and 12; round 2 distinguished the in-model identity-wrapper siblings and checked insertion, chains and parser controls. A final consistency audit extended verification once to repair two closed scorer omissions (`__VA_OPT__` chain roots and named-variadic fail-closed facts), without changing the proposed rule. The VaOpt full program was CONTRADICTED and admitted by the old scorer; its corrected result refuses. The named-variadic full program was already G1Error. The defect population is closed by mechanism (section 6), not by a bound on alias depth. The log records predictions, alternatives, falsifiers and results: `.scratch829/astra/hypotheses.md`.

| option | six-corpus confirmed drops lost | disposition |
|---|---:|---|
| 10 + 31 names | 399 (1.06%) | rejected: unstable shields |
| 11: 10n + 31 names | 979 (2.59%) | rejected: unknown-root identity rescans, owner and body |
| **12: all roots, no shields, 31 names** | **2,246 (5.95%)** | recommended, proof below; owner approval needed to supersede OD-A |
| stable-name shields | unscored | source spelling, keywords, and census absence cannot certify stability in C; obtaining external definitions changes the input model |
| finer bare-comma / parameter-flow certificate | unscored | possible follow-up; must prove substitution position, erasure and rescans before admitting anything |

Historical candidate 8 is also unsound (`#define HI F({ int a, b; })`, CONTRADICTED); 4b and 6b's old zero scores are not proofs or safe fallbacks for newly enumerated classes. The former document's blanket soundness endorsements are withdrawn. The local corpora measure cost only; no search or corpus count establishes wild-code rarity.

## 3. Preprocessor facts and current clang receipts

Rules: [argument substitution](https://eel.is/c++draft/cpp.subst), [rescan](https://eel.is/c++draft/cpp.rescan), [stringization](https://eel.is/c++draft/cpp.stringize), [pasting](https://eel.is/c++draft/cpp.concat); C11 6.10.3.1–4. Every compiler mechanism used in the amended lemmas has a synthetic clang check below. These examples test the premises; the induction, not finite examples, gives the general argument.

Run `python3 .scratch829/astra/facts.py` and `python3 .scratch829/astra/facts_more.py`; exact commands/stdout/stderr are in `facts/<name>.json`. For all token-output fixtures the command is `clang -std=gnu2x -E -P .scratch829/astra/facts/<name>.c`. Successful preprocessing is a token-output claim only, not a syntax-acceptance claim.

| fact | synthetic file | observed result |
|---|---|---|
| PF1: collect unexpanded arguments; only parens shield separating commas | `collection` | `F({ int a,b; })` -> `b; } { int a` |
| PF2: lookahead does not expand the next token or backtrack; later rescans may invoke the skipped name | `lookahead`, `body_lookahead` | bare `F E (1,2)` / body expansion -> `F (1,2)`; wrapped in `ID` -> `2 1` |
| PF3: ordinary parameter occurrences use prescanned arguments; **# and ## are exceptions** | `stringize` | `S(BAD)` -> `"BAD"`; `ID(BAD)` -> `}`; `CAT(BE,GIN)` -> `{` |
| object aliases and a returned name meet a following group on rescan | `alias_chain` | `ALIAS(1,2)` -> `2 1`; `ID(F)(3,4)` -> `4 3` |
| unseen insertion can expose a chained group without erasing its literal parens | `insert` | external-style `E -> ()`, `ID(F E(1,2))` -> `2 1` |
| `__VA_OPT__`'s syntax parens are not emitted; varargs insert separators | `vaopt` | `V()` -> `f(0)`; `V(1,2)` -> `f(0,1,2)` |
| PF6: tight/loose expansion commas have different argument boundaries | `comma` | `K(f(9,PAIR))` -> `f(9,1,2) |`; `K(PAIR)` -> `1 | 2`; `K(INIT)` -> `{0 | 0}` |
| a stringized and an ordinary occurrence of the same parameter differ | `mixed_stringize` | `BOTH(V)` -> `"V", 17` |
| disabled recursive macro names are emitted rather than expanded forever | `disabled_recursion` | `A -> B -> A` emits `A`; `SELF(1)` emits `SELF(1)` |
| a fixed punctuation token blocks callee adjacency | `fixed_punctuation` | `ID(F + E(1,2))` -> `F + (1,2)` |
| erasing a whole invocation exposes a chain on later rescan | `empty_consumed` | `ID(F DROP(ignored)(1,2))` -> `2 1` |
| optional tokens can become a callee root | `vaopt_root` | `PICK(1)` -> `2 1`; `PICK()` -> `(1,2)` |
| PF4: replacement-list `(` can open a call; a macro-produced `)` cannot close argument collection | `open`, `close` | `BEGIN int x; )` -> `int x;`; `BEGIN int x; END` -> unterminated invocation diagnostic (expected failure) |
| PF5: argument collection can continue out of an include; clang rejects include directives during collection | `span`, `include_inside` | `clang -std=gnu11 -fsyntax-only .scratch829/astra/facts/<name>.c`: accepts `span`; rejects `include_inside` with the embedding-include diagnostic |

For `#p`, the original argument is replaced by one bracket/comma-free string-literal token; if that parameter also has ordinary occurrences, those occurrences separately use prescan. `##` can synthesize a new macro name and suppress prescan of its operands; the existing reachable-paste poison remains mandatory. `__VA_OPT__` and empty arguments can erase tokens; the certificate never infers identifier survival from them.

No GCC run. PF5's include rejection is specifically clang's behaviour. Accepted full programs, runtime output and the same-binary four-arm oracle for the new structural witnesses are in section 11.1.

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

Tokenize `{ } [ ] ,` and digraph brackets in addition to current tokens, preserving identifier/literal behaviour. One ordered body scan uses `Paren{potential}`, `VaOpt` (transparent), `Brace`, `Bracket` stack entries:

```
"__VA_OPT__" directly before "(": push VaOpt (its syntax parens are not emitted)
"(": potential = previous token is ANY identifier/parameter/__VA_ARGS__
                  or previous token closes a potential paren or a VaOpt
     push Paren{potential}
"{", "[": push; closer: require matching top, otherwise unbalanced
end with nonempty stack, or any digraph bracket: unbalanced
"," or __VA_ARGS__ (innermost ignores VaOpt):
    innermost not Paren -> loose_comma
    innermost Brace/Bracket and some potential paren open -> group_held
non-parameter identifier x inside any potential paren -> group_ids += x
```

A second parentheses-only stack (including `__VA_OPT__` syntax parens) ignores braces/brackets: unmatched `(` at end sets `paren_open`. A stray `)` alone does not. This intentionally over-approximates.

Raw fields are `unbalanced`, `loose_comma`, `group_held`, `paren_open`, `group_ids`. Compared with rejected v2, replace `held_param`/`held_after` and `fwd_param`/`fwd_after` by the boolean/set above: **do not resolve roots against the census**. `f((T){a,b})` and `foo_impl((struct opts){__VA_ARGS__})` are now hazardous body groups even if `f`/`foo_impl` has no in-repo definition. A group after punctuation or at the start of the body is plain; why delayed invocation of it is still covered is lemma R.

Union facts across every definition and `#undef`, as before. A GNU named variadic sets `loose_comma` and `group_held` (fail closed). An unreadable normal pass contributes no facts, and retains existing soft-poison semantics; a successful other pass supplies them.

```
unbalanced*(M) = OR unbalanced over M and reach(M)
loose*(M)      = OR loose_comma over M and reach(M)
split(M)       = group_held(M)
                 or any x in group_ids(M): x is in-repo and loose*(x)
arg_split*(M)  = OR split over M and reach(M)
paren_open*(M) = OR paren_open over M and reach(M)
```

Export the same four closed booleans: `unbalanced`, `loose_comma`, `arg_split`, `paren_open`. Closure over reachable bodies is still needed for all four; unknown roots do not imply unknown forwarded-name facts.

**Permissive census texts:** scan the text after the name in both dialects, OR successes; all identifier-following groups are potential, and every identifier inside one is forwarded (parameter identities are unknown). A leading parameter-list group follows no identifier and is plain. If neither dialect reads the line, set all four raw hazard booleans; named variadics fail closed as above. This preserves the prior conservative permissive policy.

### 5.2 Owner checks (`c_prune::scan`, `.c` and C++ owners, after existing gates)

- **G7 `Off::MacroBrackets`:** unchanged; spelled listed name, spelled in-repo name with `unbalanced`, or its transitive `free` set containing a listed name.
- **G8 `Off::MacroBodySplit`:** a spelled in-repo macro has `arg_split`.
- **G9 `Off::MacroArgSplit` / `Off::MacroArgLoose`:** one bracket-stack pass over every normalized token variant used today (both dialects/splice modes; any refusal refuses). A **potential callee paren** directly follows any identifier, or the `)` of a potential group. All other parens are plain. A token is checked whenever **any potential paren is open**, irrespective of nested calls. Refuse a checked comma held by `{`/`[`, a mismatched closer/digraph within a potential group, or a checked identifier naming an in-repo macro with closed `loose_comma`. There is **no shield kind or exemption**. Tokenization failure keeps existing `G2` refusal.
- **G10 `Off::OwnerExposed`:** unchanged, section 5.3.

These gates refuse the entire owner for both U/D. The first refusal is traced; remove the `MacroComma` stub.

**Parser scope, corrected:** do not claim every conditional inside an argument list implies G1Error. A direct expression argument interrupted by `#if` is G1Error in the pinned `conditional_direct_valid` fixture (clang accepts it). A compound-statement argument containing complete `#if` block items is admitted: `conditional_block` has T/X owners on, three source-dropped pairs all CONFIRMED, and candidate 12 admits. Read all branch tokens conservatively; whole block items nest independently. These are parser-shape pins, not a universal guarantee for arbitrary directive extensions. Digraph owners keep their existing parser pin. The standard-preprocessing/dialect boundary is unchanged; no extra refusal is justified by the admitted block witness alone.

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

**Lemma R (potential groups cover adjacency changes).** No source identifier is assumed stable. A literal `(` immediately after any identifier or a potential group's `)` is checked even if that predecessor disappears, aliases, is not invoked yet, or eventually denotes an ordinary function. Empty expansion, an identity wrapper, and insertion of balanced `()` cannot turn such a group into an *unchecked* callee group. Rescanning may change the name adjacent to it, but not its checked status.

A literal `(` after fixed punctuation, or at the start of a replacement body, cannot become a callee on the current left-to-right scan by expanding a previously skipped name: lookahead does not backtrack (PF2). If it is exposed by consuming/dropping an intervening macro invocation, it is a checked chain, or lies within that invocation's checked argument group. If it becomes callable on a later enclosing rescan, the containing argument is required to be CC. A body supplying such a group with held commas has `loose_comma`, so G8/G9 reject its use in that argument. A `(` supplied by an external replacement lies in a WN/CC fragment under A-ext; one supplied by an in-repo body is covered by the same body/argument induction. A parameter used as a root and `__VA_OPT__` activation are explicitly potential roots. Thus no *hazardous* group is justified by a predecessor's spelling or by an absent census entry.

Checks: `lookahead`, `body_lookahead`, `alias_chain`, `insert`, `vaopt`, plus `erase`, `external_identity`, `body_identity`, `known_chain` (sections 3 and 11.1). This replaces the rejected lemma V; there are no opaque shield units or compiler-validity shortcuts.

**Expansion text is not equivalent preprocessing input (code review r2, owner decision 2026-11-15: STRICT stays the base-binary contract).** The oracle judges by the base binary on the expansion arms; the head's own gates on the expansion text are not part of the contract. The difference shows up as XOFF on head-vs-head and is conservative by PF2. In `#define N f`, `#define ARG ((struct S){1, 2})`, `N ARG;` the head admits the source owner but refuses the expansion text `f ((struct S){1, 2});` (`MacroArgSplit`: a literal `(` after an identifier with a brace-held comma). The source can never make that group a callee group. After `N` is replaced by `f`, the next preprocessing token is the name `ARG`, not `(`, and lookahead does not expand, so `f` is not invoked even when it is a function-like macro. The expansion text, in which `(` follows `f` literally, would invoke it. Checked with `clang -std=gnu11 -E -P` with `#define f(a, b) DISCARD_B(a)` in force: `N ARG;` gives `f ((struct S){1, 2});` while the literal `f ((struct S){1, 2});` gives `DISCARD_B((struct S){1);`. Owner tokens reach a macro-made group only (a) as arguments of an in-repo macro, which E3 escapes, (b) after an owner-spelled identifier or a potential group's `)`, which lemma R checks, or (c) through an unbalanced body, which G7 refuses. So a head refusal of the expansion that the source owner does not share marks no hazard of the source. The L3 compare (head vs base, zero monotonicity violations, zero CONTRADICTED still dropped) remains the gate.

**Lemma 1 (clean substitution and rescan).** Let an in-repo macro have false `unbalanced` and `arg_split` and receive CC arguments after ordinary prescan. Its expansion is WN; if `loose_comma` is also false, its expansion is CC. Every nested group that collects arguments receives CC arguments.

*Mechanism.* Induct over actual finite expansion steps. WN bodies with WN substituted strings remain WN until argument collection; the only way a brace/bracket can be cut is a comma held by it (PF1). In every potential body group, G8 prohibits literal held commas/held `__VA_ARGS__`, and every literal forwarded in-repo macro has false `loose*`. CC substitutions introduce no bare or held comma. Separating commas at the group's own parenthesis depth leave each argument CC. Nested expansions therefore preserve the induction. Groups within substituted CC strings have CC pieces; newly adjacent/deferred calls are covered by lemma R. Suppressed recursive names remain tokens; existing recursion poison is conservative, not a premise that all preprocessing recursion diverges.

Ordinary parameter occurrences use full prescan. **Stringized occurrences do not:** `#p` becomes one string-literal token, which has no structural brackets or commas. Dropping or duplicating such a token cannot break WN/CC. Reachable `##` remains poisoned rather than being treated as ordinary substitution. `__VA_OPT__` contributes either nothing or its replacement tokens, with transparent syntax parens in the scan; `__VA_ARGS__` is conservatively treated as a possible comma at each occurrence. When `loose*` is false every emitted comma is tight, proving CC. Unknown external fragments use the existing A-ext WN/CC/whole-argument premise; that premise does **not** say their names survive.

**Lemma 2 (owner arguments are clean).** At any potential owner group, G9 checks all depths. Raw arguments have no brace/bracket-held comma. Each in-repo macro in them has false `loose*`; G7/G8 cover its own body and reach. Lemma 1 therefore keeps them CC during prescan. An enclosing group after an external identity macro is checked just like an in-repo group. Erasing its inner predecessor, returning an alias, or rescanning it later does not escape these conditions (lemma R). No reliance on the compiler accepting a shield's contents is needed.

For external identity wrappers, the proof uses WN/CC preservation **on clean arguments**, just as for in-repo macros; it does not assume the fully rescanned result is already clean on arbitrary unsafe input. Interpreting A-ext as that final-result guarantee would exclude identity-wrapper failures by assumption. OD-A2 recommends refusing the measured shape regardless of that stronger reading; no such interpretation is used to excuse a contradiction.

**Coverage boundary, not a new waiver:** A-ext still excludes an unseen replacement that introduces/forwards to an arbitrary in-repo callee not represented by source/body reach. Otherwise G7/G8 cannot know which hidden body to certify. The alias witnesses in section 11.1 are nonetheless refused by their visible held-comma groups; this does not prove arbitrary hidden aliases safe. The decisive identity-wrapper witnesses introduce **no** such name and are covered without changing A-ext. Likewise this certificate does not close arbitrary unlisted external unbalanced bodies, missing census inputs, or the already pinned three-file A-span form. Claiming universal soundness outside those existing boundaries would be false.

**Lemma O (no open parenthesis without `paren_open`).** If no body reachable from the macro names spelled in a stretch of code has `paren_open`, no expansion in that stretch leaves a parenthesis open at its end.
*Proof.* Count parentheses alone and reduce a token string to `)`..`)` `(`..`(`: its unmatched closers, then its unmatched openers. A body without `paren_open` reduces to closers only. The arguments of an invocation are balanced as collected (PF1), and an argument expanded before substitution (PF3) is, by the same count, without unmatched openers. Expansion replaces a name, with its balanced argument groups, by a string built from such bodies and arguments; putting closers-only strings into a string adds no unmatched opener, and removing balanced groups adds none. By induction over the expansion steps, a stretch that starts with no unmatched opener from a macro ends with none. (A rule that looks only for "a macro name, then an unmatched `(`" in one body is not enough: `#define Z (`, `#define N(p) W3 p`, `N(Z)`.) ∎

**Lemma 3 (not captured).** Under A-span, G10 implies SP1.
*Proof.* Suppose a token of `O` is an argument of an invocation whose name lies outside `O`. Its opening parenthesis is one of three things. (i) A literal `(` of `F` before `O`: its match by raw counting is where the list ends (PF1), so it is still open at `O`'s start: the first half of the interval test. (ii) A `(` from the expansion of an in-repo macro in the code of `F` before `O`: by lemma O some name spelled there has `paren_open`: the hidden-opener half, wherever the list closes. (iii) A `(` from a macro defined outside the repository, or from the tail of a file that `F` includes (PF5): the list ends at a literal `)` that no literal `(` after `O` opened (a `)` from a macro cannot end it, PF4): the second half of the interval test when that `)` is in `F`; A-span is the assumption that it is. An opener in a file that includes `F`, before the `#include`, is not possible (PF5). The interval contains the balance of every configuration, and the lexing variants contain the compiler's reading. ∎

**Theorem.** If `O` passes the existing gates and G7 to G10, then under the assumptions of section 7 `O` satisfies SP in every valid configuration.
*Proof.* SP1 is lemma 3. For SP2 take an outermost invocation of `O`. Its name is an in-repo macro spelled by `O` (an external one is covered by A-ext), so `unbalanced` and `arg_split` are false (G7, G8). Its arguments, if any, are clean by lemma 2 (and newly adjacent groups by lemma R). By lemma 1(b) its expansion is WN. If the expansion ends in a function-like macro name and the next owner token is `(`, that group is a chained callee group of `O`, and the same argument applies. The actual invocation consumes a subset of the potential groups; the scan may additionally check ordinary function/keyword groups. Every actual consumed group has clean arguments, so tokens outside the actual invocations retain their bracket pairing. ∎

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
| **A-ext** | a macro defined outside the repository acts as a function: its expansion is WN and CC, it passes each argument on whole, and it does not forward to an in-repo macro; except the names of `BLOCK_MACROS` | disclosed (D2, wording extended in CLAUDE.md); the listed names are closed in code; an in-repo wrapper of a listed name is closed by the free-name closure. Empty/identity externals are allowed; no stable-name inference | 41 historical boundary guide rows: candidate 12 closes 32, leaves 9 open. The concrete new alias rows are additional must-keeps, all refused |
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
| owners located by a token rule; the scorer's owner can start before the real one (it swallowed `W3(` in the first `hull2` probe and reported `arg_split`) | the real owner is the tree-sitter node; such an owner is refused by G10 instead | checked two ways (`cands_diag.py`): with the exposed test alone, every row that needs it is refused (50 of 50 generator rows, 16 pair rows, 4 inline rows, 1,947 L2 programs), and the two groups do not overlap; and historical candidate 10 on an owner cut down to start after every leading macro token (`10-narrow`, the worst case of the difference) scores the same on every population (0 violations, 399 pairs). The Rust row tests are the final check |
| open group: first-branch-wins for `#if`, parentheses read by one lexer | the real test is the interval hull over all lexing variants: a superset | no (10 and 10f agree on every owner of every population) |
| potential group roots use every identifier; loose forwarded names still use `db.defined` | the same rule on real tokens/census; no shield or membership-based root | no |
| hidden opener: the first code identifier of the file with `paren_open`, compared with the scorer's owner start | the same test on the real pieces and census (more lexing variants: a superset) | no; an owner the scorer starts too early spells the macro itself and is refused by G7, as in prism |
| verdicts are those of the base binary under one host configuration | unchanged by the design | the L3 compare with the head binary is the gate (PLAN-v2 task 9) |

The scores are evidence for the rows' classes. The argument of section 6 is what covers shapes the rows do not hold; the two new body classes show the difference.

## 9. Historical scores (rejected design; not evidence of soundness)

`python3 scripts/dfg_prune/xscore.py run --out .scratch829/redesign/l4v2` (17 s; candidates 10.. and B.. in `xscore_candidates.py`; the 15 earlier candidates score exactly as in `L4-SCORES.md`, 150 of 150 cells). Cells: must-keep pairs still dropped / must-keep pairs head drops; `c` = CONFIRMED drops lost.

| candidate | L1 pair | L1 inline | L1 generator | L2 bound 3 | L2 bound 4 batches | L3 (6 corpora) | L3 recall cost | owners refused | pinned open | boundary open |
|---|---|---|---|---|---|---|---|---|---|---|
| **10 (rejected)** | 0 / 113 | 0 / 30 | 0 / 98 | 0 / 6,903 | 0 / 8,566 | 0 / 50 | 1.1% (399 / 37,777) | 214 / 20,274 | 21 / 23 | 17 / 41 |
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

### 9.1 Current scores (2026-11-15 amendment)

Commands and detailed ledger: `L4-SCORES.md` amendment addendum. `python3 scripts/dfg_prune/xscore.py run --out .scratch829/astra/l4` replays all frozen default populations; the bound-4 batches and new review witnesses have separate receipts. Candidate 12: zero MUSTKEEP violations in L1 pair 113, inline 30, generator 98; L2 bound 3 6,903, bound 4 8,566; L3 50. The new 17 contradicted pairs are all refused. Insertion/named-variadic probes with T already G1Error are controls, not added to the contradiction denominator.

| corpus | confirmed drops lost / located confirmed | cost | owners refused / located on |
|---|---:|---:|---:|
| redis | 7 / 21,605 | 0.032% | 2 / 5,062 |
| cpython | 2,215 / 15,220 | 14.55% | 284 / 5,425 |
| leveldb | 0 / 473 | 0% | 0 / 1,384 |
| protobuf | 24 / 422 | 5.69% | 140 / 7,295 |
| re2 | 0 / 38 | 0% | 5 / 886 |
| snappy | 0 / 19 | 0% | 1 / 222 |
| total | **2,246 / 37,777** | **5.95%** | **432 / 20,274** |

The extra cost over 10n is 1,267 pairs / 200 owners; over candidate 10 it is 1,847 pairs / 218 owners. Body-root broadening adds no measured cost over broader owner roots in these six corpora, but `body_identity` requires it. G10 still contributes its 67 pairs / 106 owners. The corpora are small convenience samples, not an estimate of global frequency. Scores retain the unlocated/one-host/frozen-oracle qualifications of section 8 and L4-SCORES.

## 10. Tests that follow from the design

- **New must-keep rows** (red on base; labelled by the oracle, C and C++ twins): `bodysplit`, `bodyfwd`, `deferred`, `shield_nested`, `outer_shield`, `hull2` (an opener and a closer under `#else` branches). They discriminate the historical B5/B6 ablations. The amendment adds separate root and no-shield obligations; finite rows do not establish complete premise coverage.
- **Guide rows:** `shield_safe` (CONFIRMED; now conservatively refused). Add an ordinary call with tight arguments and the admitted conditional compound-block case as positive controls.
- **Hidden opener (OD-C).** Must-keep rows `aspan` (C and C++; the opener macro at file scope) and `aspan_sib` (the opener macro inside a sibling owner); a control with the same macro spelled only after the owner (not refused). Pinned hole: `aspan3` (A-span as narrowed).
  **Why the oracle misjudges these rows, and their truth basis.** The X arm replaces each `.c`/`.cc` file by its own post-expansion tokens, lexed with that file as the main file. `impl.c` alone never sees `defs.h` (only `unity.c` includes it), so `BEGIN_IMPL` is a plain identifier there, the "expansion" equals the source, and the rules drop the pair on it too: CONFIRMED. In `unity.c`'s stream, where the macro does rewrite the function, the tokens spelled in `impl.c` are not written anywhere (only main-file tokens are kept). The verdict describes a compilation that never happens. So: a row may declare its translation unit (`tu`); when the pair's file is not the `tu`, the oracle records `UNJUDGED` (`target-not-a-translation-unit`) instead of judging the file alone, and the existing receipt rule applies (a runtime receipt outranks UNJUDGED, never CONFIRMED or CONTRADICTED): `overrides.json` gives `assert: kept` (for `aspan3`: `pinned_hole`, assumption `A-span`) with `basis: receipt`. The receipt is the program itself: `main` returns `f(0)`, 42 exactly when the first def of `x` reaches the `return` (`xarm_receipts.py`, not run in CI). Optional, not required: judge such a row from the `tu`'s token stream by writing the tokens spelled in the pair's file to that file's X text.
- **Generator (L2 addendum).** The grammar cannot derive a body that holds an invocation. Add a `nest` family: an outer object or function-like macro whose body is `K(<fragments>)` or `ID(K <macro>)` with an inner macro from the enclosing-macro fragment set, at statement and guard sites. One offline run at bound 3 for the family; new keys are frozen as rows. Budget: two hours of machine time; if it yields a contradicted key that candidate 12 admits, stop and return to design.
- **Fact tests** for the automaton (group roots, held commas, forwarded names, chained groups, `__VA_OPT__`, named variadic, permissive fail-closed), the closure (`arg_split` through reach, through a forwarded loose macro, and for a group after an unknown identifier), the digest, the owner pass (potential and plain parens; no shields; unknown roots and chained groups), `paren_open` (own, through reach, `) (`, a brace-only block macro, `__VA_OPT__`), the event scan, the interval test (conditional hull, `#if 0` text, directive lines, the cascade) and the hidden opener (before, inside and after the owner; in a directive line; in a comment), and the parser guarantees of section 5.2.

**The 80 quarantined tests.** The facts `unbalanced` and `loose_comma` keep the tentative definitions, so their tests return as fact tests:

| file | tests | disposition |
|---|---|---|
| `macro_census_brackets_tests.rs` | 38 | un-ignore unchanged (`u22` and `l11` already agree with the fail-closed reading of an unreadable permissive line) |
| `macro_census_brackets_r1_tests.rs` | 21 | un-ignore; one expectation changes: `g_sol_11_permissive_unreadable_line_with_only_a_comma` now has `unbalanced` true as well (a permissive line no dialect reads sets every fact); the digest tests gain an `arg_split` twin |
| `macro_census_brackets_r2_tests.rs`, `_r3_tests.rs` | 7 + 8 | un-ignore unchanged (lexical readings of bodies) |
| `c_macro_census_tests.rs` | 4 | un-ignore; the digest tag is `v3` and the flag bytes are six |
| `c_prune_brackets_tests.rs`, `c_prune_brackets_r1_tests.rs` | 1 + 1 | un-ignore; both pin the name list: update to the decided list (OD-B) |

None is deleted. A test that fails after un-ignoring for a reason other than the named changes (one expectation, the digest tag, the name list) is a finding for review, not something to re-baseline.

- **Review-repair rows:** every contradicted pair in section 11.1 is must-keep, including sol's erasure, alias depths, external identity wrappers in owner/body, known/unknown aliases and chains. No boundary waiver for these rows. Freeze four-arm and runtime receipts with the base revision; include external headers as oracle preludes outside the indexed file set. Parser-off insertion rows are existing-gate controls, never fabricated REDs. Include # stringization and ## poison fact tests. PLAN-v2 details the row and mutation obligations.

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

### 11.1 Review amendment probes and exact receipts

Reproduce from this worktree:

```bash
python3 .scratch829/astra/probe.py
python3 .scratch829/astra/round2.py
python3 .scratch829/astra/late_checks.py
python3 .scratch829/astra/facts.py
python3 .scratch829/astra/facts_more.py
python3 .scratch829/astra/parser_direct.py
python3 .scratch829/astra/check_scores.py
```

Each structural probe saves its source, outside header, runtime driver, four oracle arm sources/traces, exact command arrays, output and result JSON under `.scratch829/astra/probes/<name>/`. `check_scores.py` checks actual T/Tb/X/Xb counts, runtime output and candidate decisions; its 17-row manifest is `.scratch829/astra/l4-probes.json`. The header is **not in the indexed T/Tb repository**. No product edits intervene between arms. `provenance.json` binds checkout/compiler/binary; `MANIFEST.sha256` binds evidence bytes.

Canonical sol source (the current reproduction adds a blank line before the owner, hence its pair is 6→12):

```c
#include <external829.h>        /* outside header: #define E */
#define ID(v) v
#define sizeof(p,q,r) q if(r) p

int probe(int c) {
    int x = 41;
    int b = 0;
    {
        ID(sizeof E({ int a, b; }, c));
        x = 2;
        ID(sizeof E({ int a, b; }, 1));
        return x;
    }
}
```

The in-model sibling replaces `ID` with external `EXT`, removes the in-repo ID definition, and adds `#define EXT(v) v` to the outside header. The body twin moves the two call texts into in-repo `HI` and `JI` definitions; the owner spells `HI;` and `JI;`. The compiler still emits the same conditional overwrite. External macro tokens do not introduce an in-repo name: `sizeof` was already in the argument/body.

Exact command form, e.g. `erase` (all expanded absolute argument arrays are in `result.json`):

```bash
clang -std=gnu11 -I .scratch829/astra/probes/erase -fsyntax-only .scratch829/astra/probes/erase/source.c
clang -std=gnu11 -I .scratch829/astra/probes/erase -E -P .scratch829/astra/probes/erase/source.c
clang -std=gnu11 -I .scratch829/astra/probes/erase .scratch829/astra/probes/erase/runtime.c -o .scratch829/astra/probes/erase/program
.scratch829/astra/probes/erase/program
```

For `_cc` twins use `.cc` and `-std=gnu++17`. **All 20 programs syntax-check and execute successfully** (unused-expression/variable warnings are retained). All drivers print **`probe(0)=41 probe(1)=2`**; exit status alone is not the liveness evidence. For the following 17 pairs, T=0, Tb=1, X=1, Xb=1, both owners on, oracle **CONTRADICTED**:

| probe | live pair (`x`) | 10+31 / 10n+31 / 12 |
|---|---|---|
| `body_alias_c` | `source.c:7 -> 13` | **admit** / **admit** / refuse |
| `body_alias_cc` | `source.cc:7 -> 13` | **admit** / **admit** / refuse |
| `body_erase` | `source.c:8 -> 14` | refuse / refuse / refuse |
| `body_identity` | `source.c:7 -> 13` | **admit** / **admit** / refuse |
| `erase` | `source.c:6 -> 12` | **admit** / refuse / refuse |
| `erase_alias1` | `source.c:6 -> 12` | **admit** / refuse / refuse |
| `erase_alias2` | `source.c:6 -> 12` | **admit** / refuse / refuse |
| `external_alias_c` | `source.c:5 -> 11` | **admit** / **admit** / refuse |
| `external_alias_cc` | `source.cc:5 -> 11` | **admit** / **admit** / refuse |
| `external_identity` | `source.c:5 -> 11` | **admit** / **admit** / refuse |
| `external_identity_fn` | `source.c:5 -> 11` | **admit** / **admit** / refuse |
| `external_identity_insert` | `source.c:5 -> 11` | **admit** / **admit** / refuse |
| `fn_empty` | `source.c:6 -> 12` | refuse / refuse / refuse |
| `known_alias` | `source.c:6 -> 12` | refuse / refuse / refuse |
| `known_chain` | `source.c:6 -> 12` | refuse / refuse / refuse |
| `known_empty` | `source.c:7 -> 13` | refuse / refuse / refuse |
| `vaopt_chain` | `source.c:6 -> 12` | refuse / refuse / refuse |

`insert_parens` / `external_insert_parens` use external `E -> ()` and known `F() -> K`, exercising insertion and chaining. Both already have T `off:G1Error`: no contradicted source-dropped pair, so no new-gate RED credit. `facts/insert.json` independently shows the expansion `2 1`. The named-variadic program also compiles and prints 41/2 but is T G1Error; retain it as a third existing-gate control. The `vaopt_chain` program is a real CONTRADICTED dropped pair and must-keep: `#define H(...) __VA_OPT__(F)({ int a, b; }, __VA_ARGS__)`, called as `H(c)` / `H(1)`. `late_checks-red.log` records the scorer miss, `late_checks.log` its correction. No noncompiling fixture contributes to the must-keep denominator.

The simple shape variants are closed by one mechanism rather than a growing list: arbitrary empty-alias depth, ordinary argument prescan, unknown identity wrappers, object-like aliases, function-like names not initially followed by `(`, macro invocations erased as a unit, balanced token insertion, and chained groups. `#` discards the original tokens into a literal; `##` remains poisoned where census-visible and remains subject to A-ext where external. No observed result is used to claim arbitrary external body semantics are known.

## 12. Owner decisions

**OWNER-DECISION OD-A2 — supersede rejected OD-A. Recommended default: candidate 12-all-roots.** Remove shields and broaden both owner/body potential-group roots to every identifier, with 31 names and unchanged G10. Cost 5.95% (2,246 pairs, 432 owners), above the ideal target. Plain 10n is no longer a sound fallback: the external identity-wrapper and body twin violate it under the written assumptions. Accept the measured cost for this bounded repair; a separate finer comma/parameter-flow certificate can pursue recall later. No new assumption or per-row waiver is requested. Until accepted, this is a proposal, not a change to `DECISIONS.md`.

**OD-B, OD-C, OD-D remain decided:** 31 names; close the two-file A-span form with `paren_open`; accept the uncorrelated G10 interval cascade. Preserve the pinned #813 and three-file A-span disclosures. No request to reopen them.

The independent alternatives were recorded before reading `controller-options.md`: no shields; broader potential roots; a finer comma certificate; external-definition certification. The controller options supplied no additional premise. We did not use corpus absence as a proof of safety or claim that high-defect-risk shapes are too rare to refuse.

## 13. Verified and not verified

**Current execution:** compiler fact outputs (section 3), 20 structural probe programs (17 with contradicted dropped pairs, three insertion/named-variadic controls already G1Error), runtime output 41/2 for all 20, corrected parser pins, scorer selftest and 12 focused candidate checks, default L1/L2/L3 replay and bound-4 replay, historical score-cell parity. Exact commands/output and provenance are in `.scratch829/astra/`; the score addendum states all exclusions. The compiler probes and four oracle arms use this clone's same unchanged base binary; no product fix was executed.

**Inherited only:** the original header frequency scans, original G10/aspan runtime receipts, original mutation/size projections and old candidate owner-narrowing study. Current L3 scores re-evaluate candidate predicates against saved base verdicts; they do not rerun corpus compilation or prove corpus SUT identity beyond the saved inputs.

**Not verified:** a Rust implementation, Rust targeted/full tests, Tier-A, mutation kills, the new `nest` generator run, GCC, new header/GitHub prevalence research, broader corpora, or runtime overhead of the eventual gates. No release rebuild, main integration, commit or push was attempted in this design-only sandbox. The planned implementation must freeze the new rows and execute its own RED/GREEN and Tier-A/full-suite gates before claiming completion.

**Stop-hook follow-up:** root `VERIFICATION.md` records 20 portable scorer regression tests GREEN against 11 behavioral failures on the exact pre-change scorer, plus rerun population checks. The guarded full Rust suite was attempted but could not start (exit 76: sandbox denies `ps`); it remains **environment-limited**, not passed. No Rust implementation was added.

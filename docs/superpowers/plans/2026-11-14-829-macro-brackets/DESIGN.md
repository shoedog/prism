# #829: in-repo macros that change bracket structure (C/C++ rules U and D): design

**Status:** owner decisions 2026-11-14: OD-1 yes (commit B in scope), OD-2 deferred and filed as issue #831 (not implemented here), OD-3 yes (the 16-name list). Section 9 keeps the original wording.
**Test status:** the test matrix was built first and is red on base; see `TESTS.md`. It found that the C scope-shift probes of section 2 do not drop a pair on head (`TESTS.md`, Deviations).
**Worktree and base:** `fix/829-macro-brackets` at `a0f2c427` (main after #826 and #823). Every "on head" result below was produced by the release binary built from that commit.
**Goal (for materiality):** rules U and D (`.c` slice 6b, C++ slice 6c) are removal-only. A dropped def→use pair must carry no value at run time. An in-repo macro must not be able to make the compiler's block or statement structure differ from tree-sitter's in a way that drops a live pair. Refusing (keeping pairs) is always acceptable.

## 1. Recommendation in brief

1. The macro census records, per definition, whether the replacement list is **not well-nested** in `()`, `[]`, `{}` (`unbalanced`). The fact is unioned over definitions and closed over referenced macros, like `control` and `poisoned`.
2. An owner that spells such a macro is **off for both U and D** (`Off::MacroBrackets`). Escaping only names is not enough: section 5, option 2.
3. A short list of well-known block-opening and block-closing macros from system and CPython headers gets the same refusal by name (OD-3).
4. Recommended in the same PR, as a separate commit (OD-1): the census also records a **comma or `__VA_ARGS__` outside parentheses** (`loose_comma`), and an owner is off when a comma inside an in-repo macro's argument list sits directly inside `{}` or `[]` (`Off::MacroArgSplit`). This closes a second way a balanced macro body produces an unbalanced expansion.
5. Not in this PR (OD-2): a macro whose replacement list holds a top-level `;` and is used inside a declaration. It is a real hole on head with a different mechanism; deferred and filed as issue #831.

## 2. The hole, reproduced on head

All rows: prism `nav dfg-stats --edges` with `PRISM_C_PRUNE_TRACE=1`; the owner is `on` in every case; "live" means the compiled program shows the value flows.

| probe | shape | pair dropped on head | compiled result |
|---|---|---|---|
| issue `b1s.cc` | `#define END }` / `#define BEGIN {` in `h.h`; inner `int* x` between `{` and `END;` | `x = 5` → `x = 1` (U) | prints 451: live |
| issue `f1s.cc` | `#define OPEN for (` / `#define CLOSE )`; `OPEN; int* x = nullptr; CLOSE;` | same (U) | prints 451: live |
| `c1.c` | C form: `{ int *x = 0; END; r = x; BEGIN; }` in `int b(int x)` | parameter `x` → `r = x` (D) | prints 42: live |
| `t1.c` | `#define LEAVE RB`, `#define RB }` (transitive) | same (D) | prints 42: live |
| `if1.c` | `#define IF_BEGIN if (c) {` / `#define IF_END }`; `x = 1; IF_BEGIN; x = 2; IF_END; return x;` | `int x = 1` → `return x` (D) | prints `1 2`: live when `c == 0` |
| `e.c`, `e.cc` | digraph spelling: `#define RB %>` / `#define LB <%` | parameter `x` → `r = x` (D) | prints 42: live |
| `p.c` | **external** `pthread_cleanup_push(h, 0); int *x = 0; pthread_cleanup_pop(0); r = x;` | parameter `x` → `r = x` (D) | prints 42: live |
| `a1.c` | balanced body `#define F(p, q) q p`; `F({ int a, bb; })` | parameter `x` → `r = x` (D) | prints 42: live |
| `a1.cc` | same macro; `F([&](int){ int a, b; });` | `x = 5` → `x = 1` (U) | prints 451: live |
| `s1.c`, `s1.cc` | `#define M 1; if (c) q`; `int x = M, z = 3;` | parameter `z` → `return z` (D) | prints `7 3`: live when `c == 0` |

Three things here go beyond the issue text:
- `if1.c`: no declaration sits between the pair. The macro pair makes an unconditional-looking write conditional, so rule D's dominance proof fails for a name declared elsewhere.
- `p.c`: the macros are not in the repository. POSIX `pthread_cleanup_push`/`pop` open and close a block (checked with `clang -E`, section 3).
- `a1.*` and `s1.*`: every macro body is bracket-balanced. They are two further classes (sections 4.4 and 4.6).

## 3. Compiler claims and how each was checked

Apple clang 21.0.0. Sources are in `.scratch829/design/clang/` (git-excluded); the ones the tests need are reproduced in PLAN.md.

| # | claim | command | result |
|---|---|---|---|
| C1 | issue probes are live | `clang++ -std=c++17 -w b1s_run.cc -I../../probes/b && ./a.out`; same for `f1x_run.cc` | `451`, `451` |
| C2 | the C brace form is live | `clang -std=c11 -Wall c1.c && ./a.out` | `42` |
| C3 | transitive form is live | `clang -std=c11 -w t_run.c && ./a.out` | `42` |
| C4 | `IF_BEGIN`/`IF_END` makes the write conditional | `clang -std=c11 -w if_run.c && ./a.out` | `1 2` |
| C5 | digraphs `<%` `%>` are braces in a macro body | `clang -std=c11 -w e_run.c && ./a.out`; `clang++ -fsyntax-only e.cc` | `42`; accepted |
| C6 | `pthread_cleanup_push`/`pop` are unbalanced | `clang -E -P p.c` | `{ struct __darwin_pthread_handler_rec __handler; ... ;` and `... (__handler.__routine)(__handler.__arg); };` |
| C7 | the pthread form is live | `clang -w p_cpth/p.c && ./a.out` | `42` |
| C8 | `Py_BEGIN_ALLOW_THREADS`/`Py_END_ALLOW_THREADS`, `Py_TRASHCAN_BEGIN`/`Py_TRASHCAN_END` are unbalanced | `clang -E -P -I<Python 3.9, 3.11, 3.12 headers> py.c` | `{ PyThreadState *_save; _save = PyEval_SaveThread();` / `PyEval_RestoreThread(_save); }`; `do { ...` / `... } while (0);` |
| C9 | the preprocessor splits macro arguments at commas outside parentheses only; braces do not protect a comma | `clang -E -P a1.c`; `clang -std=gnu11 -w a1.c && ./a.out`; `clang++ -std=c++17 -w a1.cc && ./a.out` | `bb; } { int a`; `42`; `451` |
| C10 | a top-level `;` in a macro re-reads later declarators | `clang -std=c11 -w s1.c && ./a.out` | `7 3` |
| C11 | C: a declaration is not a statement after `if (c)` | `clang -std=c11 -fsyntax-only k1.c`; same with `-std=c2x` | `error: expected expression` (both) |
| C12 | C++: `if (c) int x = 2;` is valid and scopes `x` to the `if` | `clang++ -std=c++17 -w k1.cc && ./a.out` | `1 1` |
| C13 | C: no declaration in a `for` condition; C++ allows it | `clang -std=c11 -fsyntax-only forc.c`; `clang++ -std=c++17 -fsyntax-only forc.cc` | error; accepted |
| C14 | `__VA_OPT__(,)` emits its content in place, without the parentheses | `clang -std=c2x -E -P vaopt.c` | `printf("x" , 1, 2)`; `, 1, 2` |
| C15 | a wrapper `#define BOTH(s) BEGIN s END` is well-nested after expansion | `clang++ -w wrap_run.cc && ./a.out` | `2` |

Not checked here: the CPython 3.13+ critical-section macros (`Py_BEGIN_CRITICAL_SECTION` and friends). No 3.13 headers are installed on this machine. They are listed in the belt from the CPython documentation; listing a name can only refuse more.

## 4. The class, enumerated

Setting: owner text `T` parses without error (G1) and holds no directive (G2). The compiler sees `T` with every macro invocation replaced by its expansion. tree-sitter sees `T`.

### 4.1 Which brackets
All three kinds: `(`/`)`, `{`/`}`, `[`/`]`, as tokens outside string literals, character literals, raw strings and comments. Braces move block scope and control structure (`b1s`, `c1`, `if1`). Parentheses do too (`OPEN for (`, C13). No compiling scope shift was found for `[`/`]` alone, but the check is one stack and a crossing such as `( { ) }` needs all kinds, so all three are in.
The digraphs `<%` `%>` `<:` `:>` are brackets (C5). The body reader does not model the C++ `<::` exception, so **any** of the four digraph spellings in a body sets the fact (fail closed).
Trigraphs (`??<` and others) on a directive line already set the repo-wide `incomplete` flag (`trigraph_directive`), so every C/C++ owner is off.
`<`/`>` are not tracked: they nest nothing for the preprocessor or for block scope.

### 4.2 Per definition
"Not well-nested" is the usual stack rule over the body's tokens: a closer must match the innermost open bracket; nothing may stay open at the end. So `} {` and `( { ) }` are unbalanced although the counts match.
- The body is the replacement list only. A function-like macro's parameter list is not part of it. Parameters are atoms (section 4.4 covers what arguments can do).
- `#` and `#param` hold no bracket. `##` already poisons the macro (G5).
- `__VA_OPT__( ... )`: its parentheses are counted as an ordinary pair; the content is counted where it stands.
- A body the pass cannot read (unterminated literal or comment) gives no bracket verdict in that pass. The definition is then poisoned unless another pass read it, and that pass supplies the verdict. This is the existing soft-poison rule.

### 4.3 Closure
The compiler expands macros named in a body. So the fact is closed over referenced macros, by the census's existing reach relation (free identifiers that are in-repo macros):
- `#define ENTER BEGIN`: unbalanced through `BEGIN` (`t1.c`).
- A pair split over two macros: each one is unbalanced by itself.
- `#define BOTH(s) BEGIN s END`: flagged, although its expansion is well-nested (C15). Computing the net effect would need an ordered bracket string per macro; the conservative answer is accepted.

### 4.4 Arguments (balanced bodies, unbalanced expansion)
With every body well-nested, an expansion is well-nested when every argument is. The preprocessor splits arguments at commas outside **parentheses** only (C9). A comma directly inside `{}` or `[]` therefore hands the macro two pieces with unbalanced brackets, and a macro that reorders, drops or stringizes a piece emits them unbalanced. tree-sitter parses the invocation without error in both languages: C accepts a compound statement as a call argument (`F({ int a, bb; })`), C++ a lambda (`a1.cc`).
`WRAP({)` itself cannot occur in an error-free parse: tree-sitter's grammar only has matched brackets, so an owner without a parse error has well-nested brackets in its own text. The hazard needs a comma, not a lone bracket.
The same split can happen on a rescan, with commas that come from macro bodies: `#define INIT {0, 0}` then `H(INIT)` where `H(p)` is `K(p)`; `#define PAIR 1, 2` then `{ PAIR }` inside another macro's argument. Section 6 gives one rule that covers the first-order and the composed forms.

### 4.5 Balanced bodies that introduce a statement or a scope
Bodies such as `if (c)`, `else`, `for (;;)`, `while (c)`, `do`, `switch (c)` attach the statement that follows. Checked on head, C and C++:

| form | tree-sitter / prism on head | hole? |
|---|---|---|
| `IF_C x = 2;`, `WHEN(c) x = 2;`, `ELSE x = 2;`, `DO x = 2; WHILE0;`, `NEVER x = 2;` (same line or next line) | parsed as a **declaration** of `x` whose type is the macro. `.c`: a declaration whose specifier names an in-repo macro is ineligible; C++: non-scalar base type escapes the name. The pair `x@decl → x@use` is kept. (C++ `WHEN(c) x = 2;` is a parse error instead: owner off.) | no |
| `IF_C { x = 2; }`, `IF_C return x;`, `SWITCH { case 1: .. }` | parse error, owner off (`G1Error`) | no |
| `WHEN(c) { x = 2; }` | parsed as a nested function definition, owner off (`G1Nested`) | no |
| C++ `IF int x = 2;`, `WHEN(c) int x = 2;`, `ELSE int x = 2;`, `ONCE int x = 2;` (C12: valid C++, scoped to the substatement) | parse error, owner off | no |
| the same in C | not valid C (C11) | no |
| `IF_C; ELSE; x = 2;` | prism drops `x@decl → use`; the compiler also runs `x = 2` unconditionally | no (correct drop) |

So no keyword body is a hole today. The reason is structural: a statement that directly follows a macro name without `;` is read by tree-sitter as a declaration typed by that macro, or not at all. No new rule is proposed for this group; the matrix does not pin it beyond what 6b/6c already pin (`macro_typed_declaration`).

### 4.6 Statement boundaries (separate class, OD-2)
`#define M 1; if (c) q` used as `int x = M, z = 3;`: tree-sitter sees a declaration of `z`; the compiler sees `int x = 1; if (c) q, z = 3;`, a conditional write to the **outer** `z` (C10). Rule D drops the outer def. The body is balanced and comma-tight; the mechanism is a top-level `;`. See OD-2.

### 4.7 Macros outside the repository
Decision D2 of slice 6 says an external macro acts as a function. `pthread_cleanup_push`/`pop` (C6, C7) and CPython's block macros (C8) break that for brackets. A name list cannot be complete; it covers the standard and documented cases (OD-3), and the D2 disclosure gains "its expansion is well-nested".

### 4.8 Where the macro has to be spelled
Only in the owner's own text. An unbalanced macro used at file scope, or to write a function header (`FUNC_START(f) ... FUNC_END`), either leaves no `function_definition` owner or does not change the scopes inside a cleanly parsed one. A `#define` inside the body is G2.

## 5. Options

### Fact
| # | option | for | against |
|---|---|---|---|
| F1 | new per-macro flags in the census (`unbalanced`, and `loose_comma` under OD-1), in the digest | exact trace verdicts; the ledger can count them; no change of meaning for existing flags | one more digest field; cache version bump |
| F2 | set `poisoned` for these bodies | no consumer change (G5 already turns the owner off) | `poisoned` means "the facts of this macro are unreliable" and has a soft/hard recovery rule that does not fit a readable body; the trace would say `G5` for three different causes; the digest and cache still change |
| F3 | set `control` | none | `control` only disables D; `b1s`/`f1s` are U drops |
| F4 | an ordered bracket string per macro, composed through the closure, so `BOTH(s)` stays usable | no refusal for balanced wrappers | much more state and proof for a rare shape |
| F5 | expand macros and re-parse | the real answer to every macro class | a preprocessor inside prism: a new subsystem |
| F6 | closed-world shape certificate: a macro is inert only when its body is expression-shaped or one complete statement; any other body turns the spelling owner off | one rule for brackets, commas and `;` | recall unknown (every multi-statement and declaration macro refuses); a redesign of census consumption, not a fix for #829 |

**Chosen: F1.** F6 is the direction to take if a further structural class appears after OD-2; it is noted, not planned.

### Consumer
| # | option | for | against |
|---|---|---|---|
| K1 | owner off for U and D when it spells a flagged macro (G5 style) | closes scope shift and control shift; one line in `note_name`; matches how `poisoned` is consumed | an owner loses all pruning for one macro |
| K2 | escape only the names declared between an unbalanced pair | keeps more pairs | (a) the extent is unknown without expansion: a `}` macro shifts **every** enclosing block, so nearly every local of the enclosing blocks is affected; (b) `if1.c` drops a pair of a name declared outside the pair; (c) rule D's region and window logic also assumes tree-sitter's blocks |
| K3 | D off only | none | `b1s` is a U drop through a wrong binding |

**Chosen: K1**, for both rules. "Spells" is the existing rule: every identifier token of the owner's text (`spelled_identifiers`) and every name-kind node goes through `note_name`, whatever the parser made of it, invoked or not.

## 6. The rule

### 6.1 Census facts (per definition, then unioned, then closed)

A body is scanned as ordered tokens (identifier, `(`, `)`, `{`, `}`, `[`, `]`, `,`, digraph bracket), with the census's existing literal, comment and raw-string reading, in both dialects.

```
stack = []                                # entries: '(' '{' '[' and 'T' (the parenthesis of __VA_OPT__)
tight() = the last stack entry that is not 'T' is '('

identifier "__VA_OPT__" directly before "(":  the next "(" is pushed as 'T'
identifier "__VA_ARGS__":                     if !tight(): loose_comma
"(":   push '('  (or 'T')
")":   top is '(' or 'T' ? pop : unbalanced
"{" "[":  push
"}" "]":  top is the matching opener ? pop : unbalanced
",":   if !tight(): loose_comma
digraph bracket (<% %> <: :>):  unbalanced
end:   stack not empty: unbalanced
```

- `unbalanced`: the replacement list is not well-nested.
- `loose_comma` (OD-1): a comma or `__VA_ARGS__` whose innermost enclosing bracket in the body is not a parenthesis: top level (`#define PAIR 1, 2`), inside braces (`{0, 0}`, `{ int a, b; }`) or inside square brackets. A GNU named variadic parameter (`args...`) sets it for the definition (fail closed; the name is not tracked).
- A pass that cannot read the body (`!ok`) contributes neither fact (section 4.2).
- **Union:** `merge` ORs both, so any definition taints the spelling, as for `poisoned` (`#define DUAL 0` plus `#define DUAL {`; an `#undef` does not clear it).
- **Closure:** in `close`, both propagate over `reaches`, exactly like `control`. A macro that names a flagged macro anywhere in its body is flagged.
- **Passes:** the parsed-file collector reads each definition in several passes (comment-stripped and comment-kept, C and C++ dialect). Verdicts are ORed over the passes that read the body. A readable `#define` inside a block comment already creates facts in the comment-kept pass; it also creates these (pinned, accepted).

### 6.2 Permissive census texts
Texts that are not parsed C/C++ (`collect_permissive`) have no lexer state. For each define line, for each dialect: if the rest of the line (parameter list included; its parentheses are balanced and its commas tight) tokenizes, use the token scan above; if not, use the same automaton over raw bytes with no literal or comment tracking. OR the results. A named variadic (`ident...` in the leading parenthesis group) sets `loose_comma`. Direction: a format-string macro such as `#define PMSG "("` stays clean when the line is readable; anything unreadable is counted as written.

### 6.3 Owner gates (`c_prune::scan`, `.c` and C++)
- G7 `Off::MacroBrackets`: the owner spells an in-repo macro with `unbalanced` (closed), or a name of `BLOCK_MACROS` (OD-3):
  `pthread_cleanup_push`, `pthread_cleanup_pop`, `pthread_cleanup_push_defer_np`, `pthread_cleanup_pop_restore_np`, `Py_BEGIN_ALLOW_THREADS`, `Py_END_ALLOW_THREADS`, `Py_BEGIN_CRITICAL_SECTION`, `Py_END_CRITICAL_SECTION`, `Py_BEGIN_CRITICAL_SECTION2`, `Py_END_CRITICAL_SECTION2`, `Py_BEGIN_CRITICAL_SECTION_MUTEX`, `Py_BEGIN_CRITICAL_SECTION2_MUTEX`, `Py_TRASHCAN_BEGIN`, `Py_TRASHCAN_END`, `Py_TRASHCAN_SAFE_BEGIN`, `Py_TRASHCAN_SAFE_END`.
- G8 `Off::MacroComma` (OD-1): the owner spells an in-repo macro with `loose_comma` (closed).
- G9 `Off::MacroArgSplit` (OD-1): in the owner's token stream, some comma inside the parenthesis group that follows an in-repo macro name (and inside every group chained directly after it, the existing `macro_call_arguments` walk) has `{` or `[` as its innermost enclosing bracket. A mismatched closer or a digraph bracket inside such a group counts too.
All three apply to U and D. Order among gates is unchanged: the first refusal found is reported.

### 6.4 Cache and trace
- `CMacroCensus::digest`: tag `c-macro-census-v3`; the per-macro flag bytes become `[control, poisoned, unbalanced, loose_comma]`. A partial cache hit with a different digest already rebuilds in full; the aggregate `c-census:` identity of the text inputs is computed from the same digest.
- `CACHE_VERSION` 188 → 189 (a full hit would replay edges pruned under the old gates), with the changelog line and the three pinned asserts.
- `PRISM_C_PRUNE_TRACE=1`: `c-prune-owner<TAB>off:MacroBrackets|off:MacroComma|off:MacroArgSplit<TAB>file<TAB>line`.

## 7. Why this is sound (and what it does not claim)

**Claim.** Let an owner pass G1, G2, G5 and G7 to G9, and let external macros be well-nested and comma-tight (D2, extended). Then every macro invocation in the owner expands to a well-nested token string.

Terms: a string is *tight* when every comma in it has a parenthesis of that string as its innermost enclosing bracket.

1. Every body that can be expanded belongs to a macro named by an owner token or by a body already reached (no `##`: G5; no recursion: G5). By G7 and G8 and the closure, each such body is well-nested and tight, and every `__VA_ARGS__` in it stands directly inside parentheses.
2. Every argument group in the owner is free of brace-held and bracket-held commas (G9). Its arguments are the pieces between commas at parenthesis depth 0, so each argument is well-nested and tight.
3. Substituting tight, well-nested strings for parameters in a tight, well-nested body gives a tight, well-nested string: a comma that came from the body keeps its parenthesis; a comma from an argument has a parenthesis inside that argument; `__VA_ARGS__` joins arguments with commas at a position that is directly inside parentheses.
4. On rescan, a nested invocation's group is a parenthesis group of a tight, well-nested string. Splitting it at its depth-0 commas gives well-nested, tight pieces: a piece with an unmatched `{` would put the splitting comma inside that brace, against tightness. Induction over expansion depth finishes the claim.

Consequence: outside macro invocations, the brackets of the owner pair for the compiler exactly as tree-sitter pairs them, and no bracket from an expansion encloses an owner token outside the invocation. Tokens inside an invocation's parentheses are already escaped by name (E3).

Without OD-1 (only G7), step 2 and the comma parts of steps 1 and 3 are missing, and the claim holds only for invocations whose arguments are not split inside braces: `a1.c` and `a1.cc` remain holes.

**Not claimed:**
- statement structure inside or next to a well-nested expansion: the `;` class (4.6, OD-2). Keyword bodies are covered by other gates (4.5);
- an external macro outside the name list (D2);
- an in-repo macro invoked from an external macro's body.

## 8. Existing gates: what already covers what

| gate | covers | does not cover |
|---|---|---|
| `poisoned` (G5) | `##`, unreadable bodies, recursion, UCN names | a readable unbalanced body (all probes are `on`) |
| `control` (G6) | `goto`, `case`, `default`, returns-twice, asm; D only | `if (c) {`; U |
| free names and parameters escape (E3) | names a macro reads, writes or declares | structure: no name is involved in `END` |
| identifiers inside macro parentheses escape (E3) | names inside `F({ int a, bb; })` | the outer `x` whose scope the expansion moves |
| macro-typed declarations ineligible (`.c`), non-scalar base escapes (C++) | every keyword-prefix shape of 4.5 | nothing in this design |
| `KeywordMacro` (C++) | a macro **named** like a keyword | bodies |
| `PhaseDivergence`, `trigraph_directive` | trigraph spellings of brackets | digraphs (`e.c`) |

No part of the new rule duplicates an existing gate.

## 9. Owner decisions

**OWNER-DECISION OD-1: close the argument-split class (4.4) in this PR?**
Verified hole on head in C and C++ (`a1.c`, `a1.cc`). The macros needed (`#define F(p, q) q p`) are contrived; the fix is small (one more body fact from the same scan, one site check in the existing argument walk) and has the proof in section 7.
Cost: owners that spell a macro with a comma outside parentheses refuse (`#define VEC(x, y) ((vec2){(x), (y)})`, `#define INIT {0, 0}`, `#define ARGS a, b`, a statement macro declaring `int a, b;`), and owners that pass an initializer list or a multi-declarator block to an in-repo macro. A finer rule (refuse only when such a macro's expansion can land inside another macro's argument group) exists, but needs a predecessor analysis and a longer proof; not proposed.
*Decision 2026-11-14: yes.* **Recommended default: yes, as commit B of the same PR.** If review of commit B has not converged after two rounds, split it into a stacked PR and merge commit A alone. If the owner says no: drop PLAN part B, add the 4.4 text to the "out of model" list in CLAUDE.md and file an issue with `a1.c`/`a1.cc`.

**OWNER-DECISION OD-2: the statement-boundary class (4.6).**
Verified hole on head (`s1.c`, `s1.cc`), also contrived. A fix would record "top-level `;` in the body" and refuse an owner that spells such a macro anywhere but as a whole expression statement. Multi-statement macros without `do { } while (0)` are common in older C, so the recall cost is unknown and probably larger than OD-1's.
**Recommended default: not in this PR. File an issue (draft below) and list it in CLAUDE.md as an open hole.** *Decision 2026-11-14: deferred, filed as issue #831.* If two more structural classes turn up, take option F6 instead of a fourth flag.

> Draft issue: *C/C++ U/D: an in-repo macro with a top-level `;` used inside a declaration re-reads later declarators (hole on main).* `#define M 1; if (c) q` and `int x = M, z = 3;`: tree-sitter reads a declaration of `z`, the compiler a conditional write to the outer `z`; rule D drops the outer def (`s1.c` prints `7 3`). Options: a census fact for a top-level `;` plus a position check (the macro must be a whole expression statement), or the shape certificate F6. Refs #829, #630.

**OWNER-DECISION OD-3: the external name list (4.7).**
`pthread_cleanup_push`/`pop` is a verified hole on head with real system headers. The list is a belt like `RETURNS_TWICE`; it changes the D2 boundary by naming exceptions.
*Decision 2026-11-14: yes.* **Recommended default: include the 16 names of 6.3.** If the owner says no: drop rows `b12`, `b13`, `x6` and the list; keep the D2 wording change.

## 10. Size and shape

One PR, two commits (A: brackets and the name list; B: commas and argument split), about 230 lines of product code and about 800 lines of tests and rows.
- New module `src/call_graph/c/macro_census/brackets.rs` (the scan, the raw fallback, the argument-group check; about 150 lines).
- `src/call_graph/c/macro_census.rs` grows by about 50 lines (tokens, two fields, merge, closure, digest). It is at 1,441 weighted lines today: over 1,000, under the 2,000 ceiling. The PR says so; its split is a separate move-only PR.
- `src/data_flow/build/c_prune.rs` grows by about 35 lines (three `Off` variants, the name list, the gates).
- No file crosses 2,000 weighted lines; no file already over 2,000 is touched.

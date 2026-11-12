# #630 slice 6 — rules U and D for C and C++: design-of-record (2026-11-11)

**Status:** owner-approved 2026-11-11. The recommended design is H1, the staged structural core plus an in-repo macro census.

## How this design was produced
The process was hard+ and unframed:
- A neutral facts pass described prism's C/C++ handling.
- Two independent design consults (Fable, astra) received only the problem statement. A third idea set was kept separate.
- An evaluator pooled the three anonymised idea sets, verified claims against the code, and considered hybrids.

The full evaluation follows below. Its section 7 decisions were taken by the owner as recommended.

## Owner decisions
| # | decision |
|---|---|
| D1 | Macros: an in-repo `#define` census with a cache digest (slice 6a). Names passed to, or named by, an in-repo macro get no U and no D. |
| D2 | Disclosure accepted: macros defined outside the repository (system and third-party headers) act as functions. |
| D3 | C++: a scalar type gate (primitive, enum, pointer, or `auto` with a literal). Class-typed locals keep all pairs. |
| D4 | Overloaded operators that bind scalars by non-const reference are disclosed. Only `>>` escapes. |
| D5 | `.h` files: off until 6c, then C++ rules. |
| D6 | A `.c` file compiled as C++ is out of model. |
| D7 | Measurement on redis `src/` (C) and leveldb (C++), with cpython sampled for the macro boundary. redis is source-available, so it goes to the draft corpus release only. |
| D8 | Slices 6a, 6b, 6c, with a cap of 3 review rounds each. 6d only on measured yield. |
| D9 | Field paths deferred to 6d. |

---

# Evaluation: rules U and D for C and C++ (prism #630 slice 6)

Evaluator's reading: static only, worktree `~/code/prism-s6design` at `7e2c028b` (origin/main). Nothing
was built, run or edited. "(read, not run)" marks behaviour inferred from code that I did not execute.
I did not open the tree-sitter grammar `node-types.json` files: they live under `~/.cargo`, a dot-directory.
So grammar facts that only note C states (for example, that tree-sitter-c has no `augmented_assignment_expression`)
are unverified here. Where they matter, the design below does not depend on them, or an obligation row
pins them.

Inputs: note A, note B, note C (origin withheld), `../BRIEF-consult.md`, `../FACTS-cpp-grounding.md`.

Goal used for materiality, from the brief: **removal-only precision for C/C++ U and D.** A dropped pair must
carry no real value path. Refusing is always acceptable. Non-C/C++ output must be byte-identical. Narrow
out-of-model forms need a principled boundary that the owner accepts.

Severity tags follow the steering rule: WRONG (a constructible wrong drop, with input and result) or SMELL,
and MATERIAL or IMMATERIAL to that goal.

---

## 1. What I verified in the code, and corrections to the notes

### 1.1 Verified (read, not run)

- **Flipping `enabled` does nothing on its own.** `LexicalFacts::new` (`src/data_flow/build/lexical_prune.rs:68-162`) sets
  `enabled: false` for C/C++. After the Python, Rust and JS branches, `classify_write_in` (228-354) walks only Go
  statement kinds, `pure_path` (552-578) knows only `identifier` / `attribute` / `selector_expression`, and `is_region`
  (546-551) knows only Go kinds. So slice 6 needs a C/C++ classifier, pure path, region predicate and owner switch,
  in a module beside `rust_prune.rs` and `python_prune.rs`. All three notes agree; note C states it most precisely.
- **`window` (399-463) climbs every enclosing region.** It returns the innermost region that holds a qualifying
  direct-child write before the occurrence's child. Only Go checks labels, and Go checks only the region's direct
  children (`children(region, Language::Go)`). The region walk uses `go::statements::parent`, which is generic:
  it flattens only Go's `statement_list`.
- **`keep_def` (489-541) keeps every def inside a nested callable**, and `proof` refuses U for occurrences inside
  one. This supports note C's H2.10 (lambda writes) argument.
- **The initializer-span trap is real.** For `int y = x;` the `declaration` capture hands the `init_declarator` to
  `extract_lvalue_spans_from_node` (`src/ast/lvalues.rs:63-145`). Its text `"y = x"` matches no branch of
  `extract_lvalue_paths` (593-641), so the function recurses and pushes a span for every identifier child,
  **including `x`**, and for `int y = f(x);` both `f` and `x`. `classify_write` (210-226) checks only that the span
  equals a node whose `pure_path` is the span's path. A C classifier that walks up to `declaration` without checking
  the field would make `x` a plain write. U would then drop the read in `int y = x;`, and D would treat that line
  as a dominating write of `x`. note C found this (§0.3). A and B missed it.
- **Dereference stores are recorded as defs of the pointer.** `*p = v` gives span = the `pointer_expression`, path `p`
  (`lvalues.rs:597-604`). `*p++ = c` gives span = the identifier `p` with path `p` (the recursion runs through
  `update_expression`). `keep_def` judges every recorded def, not only classified writes.
- **Compound assignment reads are recorded.** `is_augmented_assignment` (`src/ast/rvalue_identifiers.rs:31-55`) also
  matches the operator token text, so `x += 1` records a real read of `x` whatever the grammar's node kind. The
  dead `augmented_assignment_expression` spelling that note C reports is **IMMATERIAL** to U safety. The classifier
  must still compute `plain` from the operator token.
- **C++ owners include `template_declaration`** (`src/languages/mod.rs:136`), and `callable_boundary_node_types` is a
  superset of `function_node_types` (145-160). Owner identity includes `function_start_line` (`VarLocation`,
  `src/data_flow/types.rs:7-19`).
- **The precedents the notes cite exist.** Rust `mutably_borrowed` is owner-wide by name (`rust_prune.rs:146-`). The
  Rust module doc (lines 1-37) states the accepted disclosures: auto-ref `&mut self`, interior mutability, and "a
  macro expanding to a write ... can bring back the value of a dropped def; that loss is accepted". The
  owner closed hidden **reads** with a census (fold 3), not with a disclosure.
- **Census plumbing exists.** `RustMacroCensus::from_files` produces a digest that is stored in the cached DFG, and a
  changed digest forces a full rebuild (`src/cpg/build.rs:692-704`). The CPG cache is at v173 (`src/cpg_cache.rs:325`).
- **The test baseline is the frozen producer.** `rust_prune_tests.rs:53-69` compares against
  `Producer::file_refs_frozen` (cc78c2b1, 2026-11-02). It checks subset, equal labels, and counters equal to the
  removed edges. Only its producer loop is frozen. It predates #645 (`c_jumps`, 2026-11-05), and it appears to call the
  shared, unfrozen CFG/RD helpers, but I did not trace this fully. note A's caution holds: for C/C++ the frozen
  loop is not proven equal to current main. See row `Z01`.

### 1.2 Corrections to the notes (my findings)

| # | Tag | Finding |
|---|---|---|
| F1 | **WRONG, MATERIAL** (note C H4.3, note C options 1–3) | note C calls D on a pointer's own path safe when the candidate def is a dereference store. Input: `p = buf; *p++ = c; p = buf; parse(p);`. The `*p++` def is recorded as path `p` (§1.1). `p = buf` (line 3) is a direct-child plain write before `parse(p)`, so D drops `p@2 → p@4`. But the characters stored through `*p++` are read by `parse(p)` through the re-pointed cursor, and that edge is main's only edge for the flow. Result: a real value path is dropped. Resetting a cursor is a very common C idiom. **Fix (structural):** D may drop a candidate only if its raw span is an identifier, spelled as the path's base, in a variable-write position: the `left` of an `assignment_expression` (any operator), the unwrapped declarator of a declaration or `init_declarator`, or a parameter. Every other recorded def is kept: dereference, subscript and update recursions, initializer-value spans and alias twins. note A states this ("candidate defs must be actual pointer-variable writes, never normalized dereference stores"). note B's "`*p` never gets D" is ambiguous about the candidate side. |
| F2 | **WRONG, MATERIAL** (note C option 3, "bare-argument occurrences ... after the use cannot restore before it") | The position rule assumes straight-line code. Input (C++): `x = seed; s = x; for (;;) { x = 1; restore(); use(x); bind(x, s); }`, where `bind(int&, int)` stores `&x` and `s`, and `restore()` writes `s` back through the stored pointer. On iteration 2, `use(x)` reads `seed`. D drops `x@seed → use` (W = `x = 1`). The `bind` occurrence lies after `child.start`, so option 3 does not refuse. The loss is real: main's chain `x@seed → s → bind line` has no edge into `x`. The same applies to C macros that take an address (`#define BIND(v) (ptr = &(v))`) and to backward `goto`. **Fix:** an escape occurrence counts if it lies before `child.start`, or anywhere inside the outermost loop enclosing the use, or anywhere at all when the owner has a `goto`. Simpler: use the owner-wide set, which is the Go-S / Rust `mutably_borrowed` precedent. Labels are not affected: note C's label-position rule is about control entry, and a back edge re-enters the region at its top. |
| F3 | **WRONG, MATERIAL** (note C H2.3 "owner-declared ... by name") | Input: global `g`; `void f(void) { g = 0; g = 1; { int g = 5; use_local(g); } restore_g(); use(g); }`, where `restore_g` writes the saved old global value back. `g` is declared in the owner (inner block), so the by-name census admits it. D drops `g@0 → use(g)@last` (W = `g = 1`), but `restore_g` can bring back `0`. The same applies to a use before a later same-block local declaration (`g = 1; ... use(g); int g;` is legal C). **Fix:** resolve each use-line occurrence and W lexically to one declaration in the owner; anything ambiguous or unresolved keeps the pair. note A requires binding identity (B01). note B does not address it. |
| F4 | **WRONG, MATERIAL** (missed by all three) | Struct-valued C locals that get a sub-object write. Input: `struct S s = a; struct S u = s; s = b; s.f = u.f; use(s);`. D drops `s@1 → use(s)` (W = `s = b`). But `use(s)` reads `s.f`, which carries `a`'s field through `u`. Main has no edge from the `s.f` def to a bare `s` use: member paths search `field_expression` lines only (FACTS §1.5, read, not run). So the chain is cut. The same holds for unions (`u.a = ...`) and for array members. Go is protected by its field-write → ancestor-use recall edges. C has none. **Fix:** D refuses an identifier `x` whose declarator is not a pointer declarator when some def rooted at `x` has fields whose raw first hop is `.` or `[`. In C++ the scalar type gate makes this moot. |
| F5 | **WRONG, MATERIAL to option 2b** (note C 2b, "assignment replaces the value" disclosure) | 2b admits class-typed locals, but its escape set covers only bare arguments, `>>`, reference initializers, by-reference captures and structured bindings. Input: `std::string x = a; std::string t = x; x = b; x.append(t); use(x);`. D drops `x@1 → use`. `x.append(t)` mutates `x` with `a`'s value and records no def of `x` (it is a member call). The same holds for `v.swap(saved)`, range-for by reference over `x`, and `Obj o = x;` (a converting constructor). The Rust equivalent (an auto-ref `&mut self` receiver) was disclosed, but in C++ it is pervasive. **Fix:** keep the scalar type gate (note B item 4, note C option 2), or add every receiver and range-for occurrence to the escape set. That would gut 2b's recall gain. |
| F6 | SMELL, IMMATERIAL to soundness, MATERIAL to C++ recall/accounting (note C §0.6) | note C says the two passes of a templated function decide alike if `enabled` admits both. They do not. For the `template_declaration` owner, its inner `function_definition` is a callable boundary: `collect` skips only `owner.id()`. So every def is nested and the pass keeps all pairs. Its edges carry their own `function_start_line`, so the template copies survive while the inner pass prunes. Not unsound, but removal on templates is near zero, and the "pair absent" matrix checks must be owner-keyed (as `owner_pairs` already does for Rust). **Fix:** when the owner is a `template_declaration`, exclude its own inner `function_definition` from the boundary list. Row `T08` asserts the two passes give identical survivors. |
| F7 | **WRONG, MATERIAL, rare** (missed by all three) | OpenMP. Input: `int x; #pragma omp parallel { x = tid; x = 0; use(x); }` with shared `x`. D drops `x@tid → use` (W = `x = 0`), yet another thread's `x = tid` can run between this thread's W and its use. The pragma creates no callable boundary. (An unsynchronized race is UB, but `omp critical` / `atomic` forms are defined.) **Fix:** turn the owner off when its body contains a `#pragma omp` or `#pragma acc`. note A/B list concurrency in general, but no note names an in-owner construct that creates it. |
| F8 | SMELL, IMMATERIAL (note A "exception completion" WRONG) | It is closed by the existing window, verified by reading `window`. For `x = seed; try { x = may_throw(); } catch (...) { sink(x); }`, the catch region holds no write, so the walk climbs to the body region. There W = `x = seed`, and the `try` body's def lies between W and `child_end`, so it is kept. A throwing W leaves its region, because W is a direct child and no handler sits between W and the region. note C H1.7 is right. note A's hazard applies only to a naive rule. |
| F9 | SMELL, IMMATERIAL (note A "conditional preprocessing" WRONG) | It is closed by the direct-child rule: a write inside `preproc_if` / `preproc_ifdef` is not a child of the `compound_statement`, so it never dominates (note C H5.1). |
| F10 | SMELL, MATERIAL to recall (note B item 3) | "Owner off when any macro invocation is not proven a plain function call" cannot be decided without a census. Every call could be a macro, so it means near-zero recall. B's per-name variant implicitly needs a `#define` census. |
| F11 | SMELL, MATERIAL (note C "U is unaffected" by a taken address) | Input: `p = &x; ...; sink(*p); x = 0;` on one line, or `x = compute();` where `compute` reads through a stored `&x`. U drops `x@D → L`, although `x`'s storage is read on `L`. Go and Rust U do not guard this, so precedent tolerates it. But C out-parameters make it common, and the guard is free once the escape set exists. **Fix:** apply the escape set to U as well. note A (A01) and note B ("possibly no U") lean this way. |
| F12 | SMELL, MATERIAL, rare (missed except note A in general terms) | A hidden conditional through an object-like macro. `#define MAYBE if (flag)` followed by `MAYBE x = 0;` parses as a declaration of a new `x` with type `MAYBE`. As a direct child of a `case_statement` region (which main's shadow walk does not treat as a block), it would dominate. Protothreads-style macros also hide `case` labels (`LC_SET`). **Fix:** a declaration dominates only when its type node is a built-in or specifier type, or (with a census) a `type_identifier` that is not an in-repo macro name. Census control flags cover hidden `case` / `goto`. |
| F13 | SMELL, MATERIAL (note C census label detection) | Label detection inside macro bodies is not needed. A hidden label is reachable only through a `goto` or a `case`. A visible `goto` whose target is not a visible `labeled_statement` turns D off for the owner. A `goto` or `case` token in an in-repo macro body sets the census control flag. External macros fall under the boundary. Tokenizing `ident :` heuristically is removed from the review surface. |

note C's other "analysed safe" claims hold on my reading: scope-exit destructors (H2.9, moot under the type gate),
lambda writes (H2.10), unions as killers (H2.12), shadowing missed by the walk (§0.5), balanced `#if` (H5.1) and
C++ exceptions (H1.7). note C's label-position rule (option 3: no `labeled_statement` and no nested `case` in
`[W.start, child_end)`) is sound by the entry argument in §5.2. Direct-child `case`s of a nested `switch` inside
that range need not count.

---

## 2. Pooled hazards

Found by: A / B / C = the note names the hazard as a threat (not only in passing). "Status" says what closes it in
the recommended design (§5).

| # | Hazard (rule threatened) | A | B | C | Likelihood | Status in recommendation |
|---|---|---|---|---|---|---|
| 1 | Explicit read on the write line: `x += 1`, `x = x + 1`, `sink(x); x = 0;`, chained/comma forms (U) | yes | – | yes | very common | existing occurrence/real-read proof; `plain` from operator token |
| 2 | Initializer identifiers recorded as lvalue spans (`int y = x;`) (U, D) | – | – | yes | very common | classifier binds to the declarator field only (T01) |
| 3 | Macro hides a read of the assigned local (`x = NEXT;`) (U) | yes | yes | yes | rare, concentrated in interpreters and kernels | census (in-repo); boundary X1 (external) |
| 4 | Macro hides a write or restore, or takes an address, without spelling the name (D, U) | yes | yes | yes | uncommon | census marks touched names; X1 |
| 5 | Macro hides control: `goto`/`case`/`setjmp` (protothreads, TRY/CATCH) (D) | yes | – | yes | rare | census control flag; visible goto to invisible label → owner D off |
| 6 | Macro as a statement prefix (`MAYBE x = 0;`) makes W conditional (D) | yes | – | – | rare | declaration-type rule (F12) |
| 7 | `#define` / `#undef` / `#include` inside the body (both) | – | yes | yes | rare | owner off |
| 8 | Conditional compilation removes a write (D) | yes | yes | yes | very common | write inside `preproc_*` is never dominating (structural) |
| 9 | Unbalanced `#if`, statement macros without `;` → ERROR (both) | yes | – | yes | common in macro-heavy C | `has_error` → owner off |
| 10 | Overloaded `operator=` reads the old value; class methods mutate (U, D) | yes | yes | yes | class locals very common | C++ scalar type gate (F5) |
| 11 | C++ references: `T&` parameters, `>>`, reference/structured-binding initializers, by-reference init-capture, converting constructors (D, U) | yes | yes | yes | out-params very common | C++ owner-wide escape set |
| 12 | Address taken (`&x`, macro arguments that take it) (D, U) | yes | yes | yes | very common for out-params | owner-wide escape, applied to U too (F11) |
| 13 | Read through an alias on a nominal write-only line (U) | yes | (yes) | no (says safe) | common ingredients | escape set applied to U |
| 14 | Dereference store recorded as a def of the pointer (D candidate) | yes | (partial) | **no, claims safe** | very common | variable-write certificate on candidates (F1) |
| 15 | Index-insensitive `a[]`, `.`/`->` collapse, truncation (both) | yes | yes | yes | extremely common | `pure_path` = identifier only; field paths deferred |
| 16 | Receiver re-pointing for field paths (D on `p->f`) | yes | yes | yes | common | field D deferred (6d) |
| 17 | Sub-object write after W carries the old value (`s.f = u.f`) (D on struct identifiers) | (general: partial writes) | – | – | occasional | refuse non-pointer bases with `.`/`[` sub-path defs (F4) |
| 18 | Globals, statics, members, `extern`/`thread_local` (D; U via callee reads) | yes | – | yes | members very common in C++ | lexical binding to an automatic local (F3) |
| 19 | `static` local declaration taken as a dominating write | yes | – | yes | common | storage-class exclusion |
| 20 | `volatile` / `_Atomic` / `std::atomic`, signal handlers | yes | yes | yes | rare | qualifier exclusion; type gate |
| 21 | `goto` into the window, any nesting depth (D) | yes | yes | yes | very common in C cleanup code | label-position rule |
| 22 | Switch body before the first case; Duff/nested `case` (D) | yes | yes | yes | common / rare | region definition + nested-case position rule |
| 23 | `setjmp` / `vfork` / returns-twice (D) | yes | yes | yes | rare | owner D off |
| 24 | Computed goto, `asm goto`, inline-asm outputs, SEH (D) | yes | yes | yes | rare | asm/SEH → owner off; computed-goto targets are visible labels |
| 25 | Conditional-expression writes `c && (x = 0)`, `?:`, unbraced arms, `for` headers (D) | yes | – | yes | common | only an `expression_statement` / declaration that is a direct child of a region dominates |
| 26 | Exception before the store (D) | yes | – | analysed safe | common | structural (F8) |
| 27 | Lambda / nested-function timing and capture (both) | yes | – | analysed safe | common in C++ | nested boundary; nested defs kept |
| 28 | Shadowing / declaration identity (both) | yes | – | partial (by name) | common | lexical resolution (F3) |
| 29 | Same-line coalescing (both) | yes | yes | yes | moderate | settled per raw def span |
| 30 | Templates: dependent types; duplicate owner pass | yes | – | yes | common in C++ | type gate refuses dependent `T`; template boundary fix (F6) |
| 31 | `.h` parsed as C (#529) carries C++ semantics | yes | – | yes | common in C++ repos | `.h` owners off until 6c, then C++ rules |
| 32 | Value-copy aliases (#632) and twins | yes | – | yes | common | twins are never candidates (F1 certificate) and never dominating |
| 33 | Multi-declarators, structured bindings, direct-init, VLA, statement expressions | yes | – | yes | mixed | unknown forms keep; structured bindings ineligible |
| 34 | Parse errors (both) | yes | – | yes | corpus-dependent | `has_error` |
| 35 | OpenMP / accelerator pragmas sharing locals across threads (D) | (concurrency, general) | (general) | – | rare (HPC) | owner off (F7) |
| 36 | Loop or backward-goto reuse of a reference created after the use (D under position rules) | – | – | **introduced by option 3** | uncommon | owner-wide escape (F2) |
| 37 | Placement lifetime, bit-fields, packed structs | yes | – | yes (bit-fields: no hazard) | rare | identifier-only D; address-taken covers placement |
| 38 | Coroutines | yes | – | yes (safe) | rare | no guard needed |
| 39 | Configuration / external-header uncertainty (both) | yes | – | partial | universal | boundary X1, or config-backed facts (not recommended now) |
| 40 | Baseline mismatch: the frozen producer is not current main for C/C++ (evidence) | yes | – | – | – | row `Z01` |

Coverage summary. note A has the broadest and most accurate hazard inventory, including #13, #14 and #40, and the
best evidence-design cautions, but gives no executable rule set. note B is a short checklist: it gets the main
classes and misses the traps (#2, #14, #17, #18, #31, #35). note C is the most implementable. It has the only
grounding of the initializer trap (#2), but its "safe" verdicts on #13 and #14, its by-name binding (#18/#28) and
its option 3 escape rule (#36) are wrong.

---

## 3. Pooled options, evaluated

Recall is a rough share of the removals that a perfect oracle would make on identifier paths, field paths
included, in redis-style C and leveldb-style C++. All of these numbers are hypotheses to be replaced by
measurement. The notes' own ranges are close to mine.

| Option (source) | Soundness and residual | Recall C / C++ | Cost, review risk | Fit |
|---|---|---|---|---|
| **O0 leave off** (A) | sound, trivially | 0 / 0 | none | does not deliver slice 6 |
| **O1 U only, isolated scalars** (A-A) | U-only proof is short. Residual: macro hidden reads unless the line is macro-free | 5–20 / 3–15 | small, low risk | hooks fit; D windows unpopulated |
| **O2 owner-wide quarantine U/D** (A-B; B items 3, 5; C-opt1 for C) | Sound once F1, F3, F4 are added; residual is macros | 15–40 / C++ 0 under C-opt1 | ~300 wl; low risk; recall poor on `goto` cleanup code (redis has hundreds of `goto`s in `src/` per note C) | fits per-owner `LexicalFacts` |
| **O3 C++ via owner-wide escape + scalar gate** (C-2; B items 2, 4; A-A/B's binding restrictions) | Sound for C++ scalars and pointers. Residual: overloaded operators other than `>>` that retain a reference (X3) | – / 15–30 | +150–200 wl; moderate (escape list is syntax enumeration, but each entry fails closed) | fits |
| **O3b "assignment replaces" disclosure** (C-2b) | **WRONG as specified** (F5); repair needs receiver escapes, which cancels the gain | – / (20–35 claimed) | – | not recommended |
| **O4 localized refusals** (A-C; C-3 positional) | Label positions are sound and simple. Positional escapes are **unsound** (F2) unless loop-corrected; the corrected form is sound but adds an argument | 50–75 / 30–50 | +120 wl; label rule low risk; loop-corrected escape moderate | fits: per-occurrence check inside `window` |
| **O5 field paths, stable base** (C-4a/4b; A-E subset; B excludes) | 4a (value structs, `.` only) sound; 4b-strict sound up to macros; 4b-disclose adds a Rust-style boundary. U on fields carries the alias-read residual (Python precedent) | +10–30 points in C | +200 wl; moderate-high (operator and stability census, prefix kills) | needs `.`/`->` raw-syntax reads; `AccessPath` has no operator |
| **O6 in-repo `#define` census** (C-5; B per-name variant; A: necessary but not sufficient) | Turns the macro residual into "macros not defined in parsed repo files act as functions" (X1). Fails closed on `##`, unparseable bodies, duplicates (union) | enables U on call lines (most U recall) | ~250 wl; low risk if every judgement over-approximates; cost: full rebuild when the census digest changes (Rust precedent) | the plumbing exists (`build.rs:692-704`); scoped builds must read the whole repo |
| **O7 configuration-backed facts** (A-D) | Closes macro and type uncertainty for a named configuration; needs agreement across configurations or a configuration-scoped acceptance | 45–75 / 35–65 (A) | very large; clang per TU, source-location mapping, cache keyed on build context; high risk | poor: the producer is per-file and syntax-only, and type enrichment runs after the DFG |
| **O8 occurrence-level control + object proofs** (A-E) | Highest ceiling; needs a separate conservative proof graph | 60–85 / 45–80 (A) | very large | poorest; not a slice |
| **Preprocessing authority** (A, axis) | fail-closed = O1-level recall; source-model boundary = X1 (with census) or X1-wide (without); config evidence = O7 | – | – | decided by owner D1 |

Weighing these against the Rust history (many fold rounds), the designs that close classes **structurally** are
cheap to argue and review:

- the direct-child region rule (#8, #25, #26);
- identifier-only paths (#15);
- the candidate variable-write certificate (#14, #32, #2 on the candidate side);
- lexical binding to an automatic local (#18, #19, #28);
- the scalar type gate (#10);
- owner-wide escape by name (#11, #12, #13, #36);
- label positions (#21, #22).

Each closes a class with one predicate, not with an enumeration of idioms. The weakest structural pieces are the
C++ escape list (a closed syntactic list that fails closed) and the census's tokenizer (it over-approximates, so
it fails closed).

---

## 4. Hybrids considered

| Hybrid | Composition | Soundness | Recall C / C++ | Cost, risk | Verdict |
|---|---|---|---|---|---|
| **H1** structural core + census, staged | O2's owner gates for rare constructs + O4 label positions + O3 for C++ + O6 census + F1/F3/F4/F7/F11/F12 fixes; field paths later | sound; residual only X1 (external macros) plus X3 (C++ operators) | 50–70 / 15–30 (identifier paths) | 6a census ~250 wl, 6b C ~450 wl, 6c C++ ~200 wl; each slice reviewable | **recommended** |
| **H2** note C's "C-first positioned", then C++ 2b + 3 | C-3 + 4-U + macro disclosure; then 2b | **WRONG** as specified (F1, F2, F3, F5) | high on paper | moderate | reject unless repaired; repaired, it becomes H1 without the census |
| **H3** quarantine first, recover classes later (note A's B→C) | O2, then O4 per class | sound with the fixes | 15–40 → 50–75 | smallest first slice | dominated by H1: the label-position rule is as simple as the owner gate and recovers most C `goto` code at once |
| **H4** disclosure-first H1 | H1 without the census: U only on macro-free lines; macros' hidden writes, escapes and control disclosed | sound under a wider boundary X1-wide | 40–60 (U lower) / 15–30 | ~650 wl over 2 slices | **fallback** |
| **H5** H1 + config-backed facts for C++ (O7) | adds clang-derived types and expansion | best C++ precision | / 35–65 | very large | defer; revisit only if C++ recall matters to a consumer |
| **H6** U-only floor | O1 restricted to macro-free lines | sound, no disclosure | 5–15 / 3–10 | small | the floor if the owner accepts no macro boundary at all |

---

## 5. Recommendation: H1, the staged structural core with an in-repo macro census

### 5.1 Rule set

Names. A **path** is an identifier path (no fields) for both rules in slices 6a–6c. An **occurrence** is an
identifier-class node in the owner with the base's text, as `LexicalFacts::collect` already records them.
**Escape**, **eligible**, **variable write** and **region** are defined below. "In-repo macro" means a
`preproc_def` or `preproc_function_def` in any parsed file of the repository (the census).

**Owner gate.** Both rules are off for the owner unless all of these hold:

- G1 The owner is a C `function_definition` in a `.c` file (6b), or a C++ `function_definition` /
  `template_declaration`, or a C-grammar owner in a `.h` file under the C++ rules (6c). For a
  `template_declaration`, its own inner `function_definition` is not a nested boundary (F6). `!owner.has_error()`.
- G2 The body contains no `preproc_def`, `preproc_function_def`, `#undef`, `preproc_include`, and no `#pragma`
  whose first token is `omp` or `acc` (F7).
- G3 No `gnu_asm_expression` / asm statement and no `seh_*` node in the owner.
- G4 No identifier of the returns-twice family is spelled in the owner (`setjmp`, `_setjmp`, `sigsetjmp`,
  `__sigsetjmp`, `__builtin_setjmp`, `savectx`, `vfork`, `getcontext`).
- G5 The owner invokes no in-repo macro marked **poisoned** (`##`, a body that fails to tokenize, recursion
  through the transitive closure).
- G6 (D only) Every `goto` target is a visible `labeled_statement` of the owner, and the owner invokes no in-repo
  macro marked **control** (`goto`, `case`, `default`, a returns-twice name, `asm` in its transitive body).

**Eligible binding.** For every use-line occurrence, and for D also W's target:

- E1 The occurrence resolves lexically, innermost scope first, to a single declaration in the owner: a parameter,
  or a local declared before the occurrence in an enclosing block, `for` initializer, condition or `catch`.
  Declarators are unwrapped through pointer, array and parenthesized declarators. Function declarators,
  structured bindings, C++ reference declarators, and declarations typed by an in-repo macro name are
  ineligible. An unresolved, ambiguous or nested-callable binding is ineligible. For D, W's target resolves to
  the same declaration.
- E2 The declaration has no `static` / `extern` / `thread_local` / `_Thread_local` storage class and no `volatile`
  / `_Atomic` qualifier. C++ (and `.h` under the C++ rules) also requires the **scalar gate**: the type is a
  `primitive_type`, a `sized_type_specifier`, an `enum_specifier`, or any pointer declarator, or `auto` with a
  literal initializer. References are excluded.
- E3 The spelling has **not escaped** anywhere in the owner (by name, any binding):
  - `&x`, `&(x)`, `&x.f`, `&x[i]`;
  - any occurrence inside the argument list of an in-repo function-like macro invocation;
  - membership in the transitive free-identifier set of any in-repo macro invoked in the owner;
  - the spelling is itself an in-repo macro name.

  C++ and `.h`-as-C++ additionally treat as escapes:
  - a bare or parenthesized `x` as an argument of any `call_expression`, `new_expression`, constructor or brace
    initializer, or field initializer list;
  - an operand of `>>`;
  - any occurrence inside the initializer of a reference declarator or a structured-binding declaration;
  - a by-reference capture (`&x`, `&r = …x…`);
  - the bare initializer of a declaration whose type fails the scalar gate (a converting constructor).
- E4 (D, C grammar) If `x`'s declarator is not a pointer declarator, no recorded def rooted at `x` has fields whose
  raw first hop is `.` or a subscript (F4).

**Writes and regions.**

- **Plain write (U):** the span is exactly an identifier that is either (a) the `left` of an
  `assignment_expression` whose operator token is `=`, or (b) the unwrapped declarator identifier of a
  `declaration` / `init_declarator` (with or without an initializer). It is never inside a `value` field, a
  `preproc_*` directive or a nested callable.
- **Dominating write (D):** a plain write that also satisfies one of:
  - for (a), the assignment is the direct expression of an `expression_statement` whose parent is a region;
  - for (b), the `declaration` is a direct child of a region, has an initializer, and its type node is a built-in or
    specifier type, or a `type_identifier` that is not an in-repo macro name (F12).

  A declaration without an initializer never dominates.
- **Region:** a `compound_statement` whose parent is not a `switch_statement`, or a `case_statement` that is a
  direct child of a switch body.
- **Variable-write certificate (D candidate):** the candidate def's raw span is an identifier, with the path's base
  as its text, in position (a), in the `left` of an assignment with any operator, in position (b), or in a parameter
  (F1). Every other recorded def (dereference, subscript and update recursions, initializer-value spans, alias
  twins) is kept.

**Rule U.** Drop (def, use line L) for path `x` iff G1–G5 hold, L has occurrences, every occurrence lies inside a
plain write of `x` with no real read (the existing `proof`), E1–E3 hold for them, and no identifier token on L
names an in-repo macro whose transitive free set contains `x` or is poisoned.

**Rule D.** Drop (def, use line L) for path `x` iff G1–G6 hold, the candidate passes the variable-write
certificate, E1–E4 hold, every occurrence on L has a window (existing `window`, with C regions and dominating
writes), and no `labeled_statement` and no `case_statement` that is not a direct child of a switch body starts in
`[W.start, child_end)` at any depth. The candidate also lies outside every window (existing `keep_def`).

Everything else is inherited unchanged: same-line pairs are judged per raw def span, U runs before `get_use`, D
after, and the shared counters are reused. The #634 survivor cap applies, with formerly capped owners reported
separately.

### 5.2 Soundness argument

**U.** A line L reads binding `b` only through one of the following, and each is ruled out:

- an identifier token of `b`: every occurrence is a write target, with no real read, and binds `b` (E1);
- an alias: E3 excludes `&`, references and macro address-taking;
- a macro expansion: E3 and the census exclude in-repo macros; external macros fall under X1;
- a callee reading `b`: `b` is an automatic, unescaped local (E1–E3), so no callee can name it;
- an implicit operation (`operator=`, conversion, destructor): the scalar gate (E2) excludes them in C++, and C has
  none;
- asm (G3).

So L carries no value of any def of `b`.

**D.** Fix a dropped pair (d, L). Every occurrence on L reads `b`. W completely overwrites `b`: it is a scalar or
pointer, or a struct with no sub-object writes (E4). W is a direct-child statement of region R, which encloses the
occurrence and ends before the occurrence's child.

1. **Entry.** R is entered only at its top (a compound statement, or a case via dispatch or fallthrough) or at a
   label or nested `case` inside it. Those entries in `[W.start, child_end)` are refused. An entry before W flows
   through W, since W is a direct child of R. Back edges re-enter R at its top. A throw from W's evaluation leaves R,
   since no handler lies between W and R. Hidden entries are refused: `setjmp` (G4), `asm goto` and SEH (G3), macro
   `goto`/`case` (G6, X1), and a visible `goto` to an invisible label (G6).
2. **Order.** The window keeps every def in W's statement or between W and `child_end`. So any execution from d to
   the occurrence passes a completed W after d.
3. **No restoration.** After W, an older value can return to `b` only through:
   - a spelled write of `b`: it is its own def, kept inside the window, and the copy it restores has its own
     surviving chain from d, because the save precedes W and is not dominated;
   - a write through an alias or reference, a macro, a callee or another thread: excluded by E1–E3 and G2, or X1
     for external macros;
   - a sub-object write: excluded by E4;
   - asm (G3).

The candidate is a value write of `b` itself (certificate), so its value is exactly what W overwrites. Pointee
stores recorded under `b`'s spelling are never dropped (F1). Hence d's value cannot reach the occurrence.

### 5.3 Out-of-model disclosures the owner must accept

- **X1 External macros.** "A macro not defined in a parsed file of the repository (system or third-party headers
  outside the tree, compiler built-ins, `-D` / `-include`) acts as a function. It reads and writes only storage
  that its arguments spell, takes a local's address only where the argument spells `&`, and hides no jump target,
  `case` label or returns-twice call."
  - Boundary: such a macro cannot know an owner's local names, so it can reach a local only through its arguments.
  - Known residual: argument-writing external macros (`va_copy`; BSD `queue.h` iterators, which usually fail to
    parse) and external TRY/CATCH headers.
  - This is narrower than Rust's accepted "macro expanding to a write".
- **X2 Unparsed in-repo headers.** Generated or excluded headers count as external under X1. Prism's loader parses
  `.h/.hh/.hpp/.hxx` under the root unless excluded.
- **X3 (C++) Operator convention.** "An overloaded operator other than `>>` that binds a scalar operand by non-const
  reference and retains it is out of model." Operands go by value or const reference by convention. `<<` is
  deliberately not an escape, because escaping on it would refuse every logged variable. The alternative is in D4.
- **X4 Translation language.** "`.c` files are compiled as C." Projects that compile `.c` as C++ get the C rules,
  which treat bare arguments as by-value.
- Inherited and already settled: the #634 survivor cap.

No disclosure is needed for C++ exceptions, scope-exit destructors, lambdas, coroutines, unions,
conditional-compilation writes or shadowing: each is closed by structure.

### 5.4 First obligation rows (RED-first; each first establishes that the baseline emits the pair)

Implementation traps first:

- **T01 `decl_init_read`:** `int y = x;` keeps `x@D → L`, and is not a dominating write of `x` (initializer spans
  exist). Variants: `int y = f(x);` and `int *p = q;` (`p` is a plain write, `q` is not).
- **T02 `deref_cursor_reset_keep`:** `p = buf; *p++ = c; p = buf; parse(p);` keeps `p@2 → p@4`. Also `*p = v` and
  `**p = v` candidates.
- **T03 `global_inner_shadow_keep`:** F3's input keeps. **T04 `use_before_local_decl_keep`:** `g = 1; use(g); int g;`.
- **T05 `static_local_keep`:** `static int n = 0; use(n); n = 5;` keeps `n@5 → use`. Also `extern` and `volatile`
  rows.
- **T06 `struct_subpath_restore_keep`:** F4's input. Its twin `struct_whole_assign_drop` (no sub-path writes) drops.
- **T07 `compound_assign`:** `x += 1` keeps U into it and is not plain, so it never dominates. `x = x + 1` keeps.
- **T08 `template_two_passes_agree`:** the survivors of the `template_declaration` and `function_definition` passes
  are equal, on one line and on two.
- **T09 `omp_owner_off`.**
- **T10 labels:** `label_before_write_drop`, `label_in_window_keep`, `label_nested_in_window_keep`,
  `label_after_use_drop`, `goto_cleanup_tail_drop` (the redis shape), `goto_invisible_label_owner_off`,
  `computed_goto_label_keep`.
- **T11 switch:** `before_first_case_keep`, `case_region_drop`, `fallthrough_keep`, `duff_nested_case_keep`,
  `nested_switch_cases_in_window_drop` (direct-child cases of a nested switch do not refuse).
- **T12 escape:** `addr_taken_keep_D`, `addr_taken_keep_U` (`p = &x; sink(*p); x = 0;`), `param_addr_keep`.

Base rows:

- `write_only_drop`
- `dominated_drop` / `own_keep` (note A's three-row `00_basic`)
- `same_line_per_span`
- `chain_keep`, `comma_keep`, `ternary_keep`, `unbraced_if_keep`, `and_assign_U_drop_D_keep` (`c && (x = 0);`)
- `braced_if_keep`, `loop_top_drop`, `for_header_keep`, `do_cond_use`
- `preproc_write_keep`, `preproc_use_drop`, `define_in_body_off`, `include_in_body_off`
- `setjmp_off`, `asm_off`, `error_owner_off`
- `twin_never_candidate`, `param_dominated_drop`, `nested_fn_def_keep`, `decl_no_init_never_dominates`
- `index_keep`, `field_keep`

Census rows (6a/6b):

- `macro_free_read_keep` (`#define NEXT (x+1)` … `x = NEXT;`)
- `macro_param_read_drop`
- `macro_arg_touch_keep`, `macro_free_write_keep`
- `macro_control_off` (a protothreads-style `case`)
- `macro_paste_poison`
- `macro_named_local_keep`
- `macro_typed_decl_not_dominating`
- `macro_other_file`, `macro_redefined_union`
- `census_digest_partial_hit_rebuild`
- `scoped_build_census_equals_full`

C++ rows (6c):

- `class_type_keep` (`std::string`), `method_receiver_keep` (`x.append(t)`), `range_for_class_keep`,
  `converting_ctor_keep` (all closed by the type gate)
- `size_t_drop`, `auto_literal_drop`, `auto_call_keep`
- `bare_arg_escape_keep`, `stream_extract_keep`, `stream_insert_drop` (X3)
- `ref_init_keep_U` (`int &r = x; sink(r); x = 0;`), `structured_binding_keep`, `ref_init_capture_keep`
- `lambda_def_kept`, `try_catch_drop`, `try_body_def_keep`, `coroutine_drop`
- `member_unqualified_keep`, `this_field_keep`, `local_class_method_keep`
- `header_cpp_semantics_keep` (a `.h` parsed as C with `f(x)`, `f(int&)` declared elsewhere, keeps)

Evidence rows:

- **Z01** On the whole C/C++ fixture tree, the frozen producer equals current main with C/C++ pruning disabled.
  If not, add a test seam that disables C/C++ U/D and use it as the control.
- **Z02** The removal-only sweep over every `.c/.h/.cc/.cpp/.hpp` under `tests/fixtures` (skipping `secbench`)
  shows survivors ⊆ control, equal labels, and counters = removed. `macro_heavy.c` and `cpg_stack_regression` are
  stress inputs.
- **Z03** Non-C/C++ outputs are byte-identical. The other languages' matrices stay green, plus one Java/Lua/Bash
  fixture.
- **Z04** Cold, full-hit and partial-hit builds agree.
- **Z05** Cap counting and emission agree, and a regained owner is reported separately.

### 5.5 Corpora

- **redis `src/` (C).** Exclude `deps/` and pin the commit at measurement. All three notes favour it, and A and C
  inspected it. It is application C with frequent `goto` cleanup (it exercises the label rule), pointer-heavy code
  (it exercises F1 and the deferred field paths), moderate macros, and good parse quality.
- **leveldb (C++, without `third_party/`).** It has references, RAII, `std::string`, `Status` values and few
  templates. That exercises the type gate, the escape set and `.h` handling. It is small, so treat it as a
  correctness probe rather than a recall estimate.
- **cpython `Objects/` + `Python/` (sampling only).** The point is the census's value and on-rate under macro-heavy
  code (`ceval` macros read locals), and the X1 residual. Sample about 30 drops.

Measure locally against main: removed only, 0 added, 0 label changes, `call-stats` identical, non-C/C++
byte-identical. Report strata: language, rule, refusal reason (gate G*, E1–E4, label, census, type gate), `.h`
separately, giant and formerly capped owners separately. Sample 30 drops per rule per corpus with verdicts, plus
a sample of retained opportunities by refusal reason (this tells which 6d fold pays). Uploading redis to the
`corpora-v1` release needs the owner's licence call (D7).

### 5.6 Slicing plan (declare a cap of 3 review rounds per slice, per the convergence rule)

- **6a — C/C++ macro census (behaviour-neutral).**
  - `CMacroCensus::from_files` over every parsed C/C++ file. Per macro name it records the parameters, the
    transitive free-identifier set (identifier tokens minus parameters; keywords may stay in, since extra names only
    refuse more), the `control` flag and the `poisoned` flag. Multiple definitions are unioned.
  - A digest of exactly these facts goes in the cached DFG, with a full rebuild on change (`build.rs:692-704` shape).
    Scoped builds read the whole repo.
  - Cache version bump. ~250 wl. Review is low-risk because every judgement over-approximates.
- **6b — C rules for `.c` owners.**
  - A new `c_prune.rs`: classifier, region, resolver, escape, E4, certificate, label positions, gates; ~450 wl.
  - Explicit `Language::C` dispatch in `lexical_prune.rs`; no shared predicate is broadened.
  - Fixtures under `tests/fixtures/dfg_prune/c/`, and a `c_prune_tests.rs` copying `rust_prune_tests.rs`'s shape,
    including `owner_pairs`.
  - Measure on redis.
- **6c — C++ and `.h` owners.**
  - Scalar gate, C++ escape set, template boundary fix (F6), and C++ rules applied to C-grammar `.h` owners. ~200 wl.
  - Measure on leveldb.
- **6d — optional folds, each only if 6b/6c's retained-opportunity sample shows the yield.**
  - (i) Loop-corrected positional escapes for C++ (F2's fix).
  - (ii) U on field paths (Python precedent; needs the alias-read disclosure or an escape rule on the base).
  - (iii) 4a value-struct field D.
  - (iv) 4b pointer-base field D: strict (no call in the window) or disclosed.

  Each is its own measured slice. Measure yield before folding a refusal cut.

If the owner prefers fewer PRs, merge 6a into 6b. The census then lands with its first consumer, at about 700 wl,
the upper end of what converged for Rust.

---

**6a census gate (fold 2).** The census records an `incomplete` flag, and the rule for 6b is: `incomplete` disables U and D for ALL C/C++ (repo-wide, not per file or per macro). It is set by a `#define` line that names no macro, an unreadable macro parameter, an unterminated block comment, an uncertain raw-string scan, or any in-repo C/C++ file the loader skipped (NotUtf8, ParseFailed, Unreadable, TooLarge, Symlink; the loader stamps every parsed C/C++ file, so full, partial and scoped builds agree). This closes X2 for skipped in-repo files: they are no longer treated as external. Definition lines are the union of the comment-stripped and the unstripped text, so a lexer divergence adds macros (refusing more) and never loses one.

## 6. Fallback: H4, disclosure-first (H1 without the census)

Use it if the owner declines the census's full-rebuild cost, or if 6a fails to converge within its cap. The rule set
is the same, with these changes:

- **U** is admitted only on lines whose tokens, apart from the target, are literals, operators, and identifiers
  resolving (E1) to eligible locals or parameters: no calls, no other names. No macro can be invoked on such a line,
  because a local's spelling cannot be an object-like macro in compiling code once `#define` in the body is gated.
  So U needs no macro disclosure, at a large recall cost: `rc = f();`, `s = STATE_X;`.
- **D** treats macros as X1-wide: "a macro's expansion may write, take the address of, or hide control over a local
  without spelling it; that is out of model". This is wider than Rust's precedent, because C macros have no hygiene.
  It is still bounded by G6's visible-goto rule and F12's declaration-type rule ("never" for a `type_identifier`).
- **Floor (H6).** If the owner rejects X1-wide too, ship U-only on macro-free lines (no D). It is sound with no
  macro disclosure.

---

## 7. Decisions only the owner can make

| # | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | Macro policy | (a) in-repo census + external boundary X1 (H1); (b) disclosure-first X1-wide (H4); (c) U-only floor (H6); (d) configuration-backed facts (O7) | (a): closes hidden reads the way the owner required for Rust (fold 3), with a verifiable boundary |
| D2 | Accept X1/X2 wording (external and unparsed-header macros act as functions) | accept / narrow / reject → D1(c) | accept |
| D3 | C++ class-typed locals | scalar type gate / "assignment replaces" disclosure (2b, needs receiver escapes per F5) | type gate |
| D4 | C++ overloaded operators binding scalars by non-const reference (X3) | disclose (escape `>>` only) / positive operand certificate (escape unless every other operand is a literal or eligible local) | disclose; measure the certificate's cost on leveldb if the owner hesitates |
| D5 | `.h` owners (#529) | off until 6c, then C++ rules on C-grammar owners / fix #529 first / C rules (unsound on C++ headers) | off, then C++ rules |
| D6 | `.c` compiled as C++ (X4) | accept / turn `.c` off where a C++ build is detected | accept |
| D7 | Corpus publication | upload redis `src/` to `corpora-v1` / keep the measurement local | owner's licence call; local is sufficient for 6b |
| D8 | Slicing and caps | 6a/6b/6c(/6d) with cap 3 each / merge 6a+6b | 6a/6b/6c, 6d only on measured yield |
| D9 | Field paths in slice 6 | defer to 6d / include 4a in 6b / 4b strict / 4b disclosed | defer; decide 4b strict vs disclosed only after 6b's opportunity sample |

---

## Summary

The three notes agree that C/C++ need their own classifier, region and gate beside the existing per-language
modules. They disagree on what is safe. My reading of the code confirms note C's initializer-span trap and note A's
dereference-store trap. note C wrongly calls the dereference store safe. I found four further constructible wrong
drops that a design must close: by-name local detection lets a global through, note C's positional escape fails in
loops, struct sub-object writes after a dominating write, and OpenMP-shared locals. note C's "assignment replaces"
C++ variant (2b) is unsound for member-function mutation.

I recommend **H1**, a staged structural design. Identifier paths only. Both rules apply only to a binding that
resolves lexically to an automatic, unescaped local. Pointer stores are never dropped as pointer defs. Dominating
writes are whole statements directly inside a single-entry region, with no label or nested `case` between the
write and the use. The C++ scalar type gate and an owner-wide reference-escape set cover C++. An in-repo `#define`
census closes the macro residual down to one principled boundary: external macros act as functions. It ships as
6a (census, behaviour-neutral), 6b (C), 6c (C++ and `.h`), with field paths and finer escapes as optional measured
folds. It is measured on redis `src/` and leveldb, with cpython for macro sampling. The fallback is the same design
without the census: U only on macro-free lines, and the macro boundary widened to all macros. The owner decides
the macro policy and its wording, the C++ class and operator conventions, `.h` handling, `.c`-as-C++, corpus
publication, slicing and field paths.

**6a census gate (fold 3, owner-approved past the cap).** Further `incomplete` triggers:
- a non-ASCII byte or UCN outside string/char literals and comments in any directive line (any non-ASCII byte when the literal boundaries are uncertain), or a trigraph anywhere in one;
- an unreadable define-looking line in any pass;
- any `Unreadable` loader skip, including directories;
- skipped text-include files;
- review mode with unparsed C/C++ or text-include files in the repository.

Text-only include files (`.inl .inc .ipp .tcc .def .cu .cuh .H .C .HPP .CPP`, plus in-repo quoted-include targets with unmapped extensions) are read into the census. Their hashes and the skipped-file aggregate join the cache topology key (`c-census:` entries), so a full hit cannot replay a stale census. C-dialect passes are unioned. Soft poison is kept per source definition. Control matches the substrings setjmp, vfork, getcontext and savectx.

**6b fold 2 (round-2 review).**
- Delimited (`\u{..}`, `\U{..}`) and named (`\N{..}`) UCNs are UCN starts in the one identifier rule: hazardous on define/undef lines, and a token holding one is non-ASCII (poisons a macro name or parameter). A bare `\u`, `\U` or `\N` followed by anything else is no UCN.
- Macro arguments come from the token stream: every identifier between the parentheses after an in-repo macro name (or a `va_*` builtin) escapes, whatever node the parser produced (call, `macro_type_specifier`, declarator). `(M)(x)` is not an invocation; it is escaped conservatively.
- Census text is read strictly as UTF-8 (oversize files and text includes); a failure is `incomplete` with a `c-census:text-failed:<path>` = `not-utf8` identity entry.
- Angle includes (`<p>`, `include_next`, `import`) read every in-repo file whose path equals `p` or ends in `/p`, transitively, and join the identity as for quoted targets. The exotic belt applies to include lines (non-ASCII byte or UCN in the target spelling), not their trigraphs. Review mode counts unparsed angle targets in its gap.
- The freshness bootstrap recomputes census membership from a fresh walk (the loaded `ParseFailed` skips are kept, since they need the parser), so a census text file created between load and probe is stale; the identity stays apart from `scope_graph_inputs`.

**6b fold 3 (round-3 review, owner decision "every file is a header"), superseded by the hybrid note below.** Kept from it: the aggregate `c-census:aggregate` identity, the no-C skip, the chained-invocation fix (`ID(TRACKED)(x)` escapes every directly following `(` group). Measured: reading every file with the full C lexer made the census `incomplete` on redis 7.2.4 and cpython 3.12.7 (Makefile, ChangeLog, `.rst`, `.py` comments, one non-UTF-8 `.py` trip the lexer-state and unreadable-define triggers) and cost +14% on prism's own tree.

**6b hybrid (owner decision after fold 3).**
- Parsed C/C++ files: the full census lexer, unchanged.
- Include discovery is the primary input. Targets of `#include`/`#include_next`/`#import` (quoted or angle) are collected from every parsed C/C++ file (comment-kept and comment-stripped variants, both dialects, splice and trigraph modes) and transitively from every census text; normalized (`.`/empty dropped, `..` lexical, `\` as `/`); resolved against every in-repo directory (path equals the target or ends with `/<target>`, quoted too, covering `-I`). An angle target that climbs out of the root is external; a quoted target drops the leading `..` it cannot resolve (it is includer-relative; over-matching only adds facts). Every resolved target that is not a parsed C/C++ file is read as census text whatever its extension or language.
- The C-like-extension net: every in-repo file with a C-like extension (`.c .h .cc .cpp .cxx .hh .hpp .hxx .h++ .c++ .inl .inc .ipp .tcc .tpp .txx .def .cu .cuh .x`, any case) that is not a parsed C/C++ file is read too.
- Non-parsed census texts use a permissive scan: phase-1 normalization (BOM, CR, trigraphs, both splice modes, unioned; NUL-stripped variant for BOM-less UTF-16), then any `#`/`%:`/`??=` + blanks + `define`/`undef` + identifier name anywhere on a line is a definition (`undef` a bodiless fact), body identifier runs are free names, `##` poisons, control spellings set control, a non-identifier name is no fact, no comment or string tracking, never `incomplete` from lexer state. Non-UTF-8 is decoded lossily (UTF-16 with a BOM as UTF-16). Only an unreadable file or directory, or a file over 64 MiB containing `define`/`undef`, is `incomplete`.
- The walk reads only parsed files, resolved targets and C-like files (a directory listing locates them); `.git/` and `eval/secbench` are never entered. The aggregate identity covers the facts, the files read, the contributors and the lost inputs.
- Residual (documented, same boundary as D2): a header with an unusual extension reached only through a form prism cannot resolve (a computed include, a generated header absent from the repository) acts like an external header. Precise follow-ups: #813; residual lexing shapes: #812.

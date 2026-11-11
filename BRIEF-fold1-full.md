# Fold 1 brief: PR #811 (#630 slice 6b, rules U and D for C `.c` owners), after review round 1 of 3

You are the implementer, on branch `perf/630-s6b-c-rules` (PR #811, head cf8810f7). Make plain pushes (no force-push, no rebase). Both round-1 reviews are appended.

**Never wait for a human.** Nobody will answer questions in this session. Where something is unclear, take the conservative reading (keep the pair), record it in the report, and continue. If something blocks you, push what you have and write the report with `Progress: STOPPED` and the reason. Push code commits as soon as tests pass, before measuring.

## Step 0
Merge `origin/perf/630-s6a-c-macro-census` (now e06c71f5: census folds 2 and 3) into your branch. Do not rebase.
- Resolve conflicts keeping both sides. The cache stays v175, above 6a's v174.
- The census API gained: the `incomplete` triggers; text-only include files stamped on `ParsedFile`; `c-census:` topology entries.
- Make sure 6b still consumes `incomplete` and the census through the merged API, and that `c_prune_reads_the_repo_wide_census` still passes.

## Items (from both reviewers; each fixed and pinned by a keep row that fails on cf8810f7)
1. **Declarations under wrappers are invisible to E1** (sol #1 and Opus W1; both reviewers). `scope_decls` sees only direct `declaration` children, so a declaration under `preproc_if`/`preproc_ifdef`/`preproc_else`/`preproc_elif`, a `labeled_statement` or a nested `case` is skipped, and the occurrence binds an outer automatic local.
   - Fix conservatively: any name declared under such a wrapper anywhere in the owner is added to the owner-wide `escaped` set (refused). Alternatively, descend into the wrappers and refuse on ambiguity.
   - Rows: sol's conditional-`extern` input (U and D), Opus's `#ifdef USE_GLOBAL extern int n;` input, and the `static`-plus-recursion variant.
2. **Macro spellings parsed as keywords or types** (sol #2). Census lookup sees only identifier node kinds, so `#define const x = saved;` used as `const long y = 0;` escapes.
   - Fix: find spelled macro names from the owner's preprocessing identifier tokens (its source text, tokenized with the census's identifier rule), independent of node kind. Apply every census fact: free names, arguments, macro-name refusal, and the poisoned and control flags.
   - Rows: sol's input, plus a type-name macro.
3. **The OpenMP/OpenACC gate on logical lines** (sol #3). Normalize continuations and comments before reading a directive's first token, so `#pragma /*gap*/ omp` and a continued `#pragma \` + newline + `omp` are caught. Reuse the census's line normalization if possible.
   - Rows: the commented and continued forms.
4. **Macro arguments on non-bare callees** (Opus S1). `it->MACRO(x)` and `(MACRO)(x)` must apply the argument escape when the callee's last identifier is an in-repo function-like macro.
   - Row for each.
5. **Smaller items:**
   - document the `ms_*` node refusal in G3 (Opus S3) and split it out in the ledger;
   - add the certificate-left keep row `int y = (q = p);` with a later overwrite and use of `p` (sol #6);
   - add a bare-aggregate restoration row (sol #5: a struct local copied whole and restored);
   - fix the `va_keep` row so it pins `ap`;
   - add the C-specific cap row (Z05).
6. **Survivor cap** (sol #4): this is the owner's #634 rule, already disclosed. Make sure the CLAUDE.md slice-6 paragraph carries the #634 sentence. No code change.
7. **Performance** (Opus S6, optional): memoize the per-node binding resolution if it is simple, and report the timing.

## Measurement (after the code is pushed)
- Same corpora and pins as before: redis 7.2.4 `src/` and cpython v3.12.7 `Objects/ Python/ Include/`. The non-C controls must be identical.
- **Base:** the merged 6a head e06c71f5 (6a changes no output).
- **Against the base:** removed only, 0 added, 0 label changes.
- **Against cf8810f7:** the fold may only restore edges. Report the restored count, by item.
- **Samples:** the design's strata, §5.5. Draw 30 U and 30 D on redis and 15 U and 15 D on cpython (seed 811). Add 20 retained-opportunity samples on redis: pairs the rules refused, with the refusal reason. Push `samples-fold1.jsonl` and `timing.txt` to the results branch.
- **Ledger:** refusal causes per owner, as before, with `ms_*` split out. Add the count of owners refused by item 1.
- **Timing:** redis, 3+3 interleaved. Report the means.

## Output
- Write `REPORT-fold1.md` on `test-results/630-s6b`.
- It contains: items with commits and RED rows, the tests, the measurement tables, the samples summary, the ledger and the timing.
- Its last non-blank line must be `Progress: DONE` or `Progress: STOPPED`.

## Rules
- Run tests with a memory cap where available.
- Static analysis only: never build or execute corpus code.
- Do not open PRs, comment on GitHub, or touch branches other than `perf/630-s6b-c-rules` and `test-results/630-s6b`.
- Never read or print dotfiles or credentials.
- Do not open `eval/secbench`, and exclude it from every recursive search.

---
# Appendix A: round-1 sol review
Reviewed `f26f4555...cf8810f7` statically. I found three material wrong-drop scenarios. The survivor cap also conflicts with the brief’s absolute prohibition on added edges.

These are source-level proofs, not executed repros. No builds, tests, edits, or git writes were performed.

1. **WRONG · MATERIAL — E1 skips declarations inside preprocessing conditionals.**  
   [c_prune.rs:521](./src/data_flow/build/c_prune.rs:521)

   `scope_decls` examines only direct `declaration` children. It skips this inner `extern` and resolves subsequent occurrences to the outer automatic local:

   ```c
   int x;
   int read_x(void) { return x; }
   void sink(int);

   void f(void) {
       int x = 0;
       {
   #if 1
           extern int x;
   #endif
           x = 7;          /* D */
           x = read_x();    /* L */
           sink(x);
       }
   }
   ```

   U drops `x@D → L`: the only visible `x` on L is a plain assignment target, and the incorrect resolution admits it as automatic and unescaped. But `read_x()` reads the global value `7`. This is a real value path. G2 permits `#if`, and the existing reference-shadow filter retains references whose def is inside this same block.

   **Fix:** account conservatively for declarations beneath preprocessing wrappers within the same lexical scope. If competing declarations cannot be resolved, refuse the name rather than fall back outward. Add U and D regressions involving a conditional `extern` shadow.

   **Does not apply:** when the hidden declaration is absent, or resolution already refuses the spelling.

2. **WRONG · MATERIAL — census consumption misses macro spellings parsed as keywords/types.**  
   [c_prune.rs:123](./src/data_flow/build/c_prune.rs:123), [c_prune.rs:236](./src/data_flow/build/c_prune.rs:236)

   Census lookup occurs only for four identifier node kinds. A macro named `const` appears as a qualifier, so its free names never enter `escaped`:

   ```c
   #define const x = saved;
   long src(void);
   void sink(long);

   void f(void) {
       long x = src();     /* D */
       long saved = x;
       x = 0;              /* W */
       const long y = 0;
       sink(x);            /* L */
   }
   ```

   Expansion restores `x` through `x = saved; long y = 0;`. The correct census contains free names `x` and `saved`, but the consumer ignores this invocation. D therefore drops `x@D → L`, although L reads the original value.

   This also bypasses `poisoned` and `control` handling for macro names represented by unexamined token kinds. The pinned grammar confirms that `const` is a `type_qualifier`. [tree-sitter-c v0.24.1 grammar](https://raw.githubusercontent.com/tree-sitter/tree-sitter-c/v0.24.1/grammar.js)

   **Fix:** discover spelled macro names from preprocessing identifier tokens independently of parser identifier-node classification, then apply all census facts. Add qualifier/type-token macro regressions.

   **Does not apply:** when the macro spelling reaches the existing lookup, or the owner/name is otherwise refused. This is an in-repository macro, outside the accepted D2 disclosure.

3. **WRONG · MATERIAL — comments or continuations bypass the OpenMP gate.**  
   [c_prune.rs:103](./src/data_flow/build/c_prune.rs:103)

   The purported logical-line check uses physical lines and raw characters. For `#pragma /*gap*/ omp parallel`, its first argument word is empty, so G2 stays on.

   ```c
   void lock(void);       /* both functions use one shared mutex */
   void unlock(void);
   int src(void);
   void sink(int);

   void f(void) {
       int x = 0;
   #pragma /*gap*/ omp parallel num_threads(2) shared(x)
       {
           lock();
           x = src();     /* D */
           unlock();
           lock();
           x = 0;         /* W */
           unlock();
           lock();
           sink(x);       /* L */
           unlock();
       }
   }
   ```

   D drops `x@D → L`. A valid schedule lets thread A complete W, thread B execute D, then thread A execute L. Every shared access is synchronized, so the counterexample does not depend on a data race.

   The grammar represents pragma arguments as opaque `preproc_arg` text; the recursive identifier walk does not repair this missed gate. [Pinned grammar](https://raw.githubusercontent.com/tree-sitter/tree-sitter-c/v0.24.1/grammar.js)

   **Fix:** normalize continuations and comments before identifying directives and their first preprocessing token. Add commented and continued OpenMP/OpenACC rows.

   **Does not apply:** to ordinary spellings already caught by G2, or when another gate disables the owner.

4. **WRONG · MATERIAL against the brief’s absolute no-additions contract — formerly capped C owners regain edges.**  
   [lexical_prune.rs:112](./src/data_flow/build/lexical_prune.rs:112), [file.rs:749](./src/data_flow/build/file.rs:749)

   Consider a `.c` function containing `long x = 0;`, then **2,050 separate-line `x = 1;` statements**, then `return x;`.

   Base exceeds the 2,048-definition RD limit and the million-edge fallback cap, emitting no owner edges. At head, U removes assignment-target pairs and D leaves the last assignment’s edge into `return x`. That edge is added relative to base.

   **Fix:** preserve base cap admission for C owners if additions are prohibited; disabling C pruning for fallback owners is a conservative bounded option. Add the missing C-specific Z05 regression.

   **Does not apply:** if the owner-approved inherited #634 exception remains authoritative. The design and CLAUDE.md explicitly allow it; this review brief explicitly forbids additions. Those contracts need reconciliation.

5. **SMELL · MATERIAL — the evidence does not complete the design’s required coverage.**

   The appendix supplies 50 judgments, split **20/10 U/D for redis and 15/5 for cpython**, rather than §5.5’s 30 per rule per corpus. Individual sample records are absent from the reviewed diff, so I cannot independently endorse “50/50 correct.” Retained-opportunity sampling, E1/E2/E4 refusal accounting, giant/formerly-capped strata, and C-specific Z04/Z05 evidence are also missing.

   Some named rows are weaker than their advertised obligation: the struct rows use member accesses rather than the central bare-aggregate restoration case; `va_keep` pins unrelated `x`, not preservation of `ap`. The computed-goto row is masked by G1 under the pinned grammar, which does not support that goto form.

   **Fix:** provide inspectable samples, complete the missing strata, and add discriminating obligation/cache/cap rows. This is an evidence gap, not an independent correctness blocker.

6. **SMELL · IMMATERIAL — the surviving certificate-left mutant is not proven equivalent.**  
   [c_prune.rs:734](./src/data_flow/build/c_prune.rs:734)

   `int y = (q = p);` recursively produces an initializer-derived lvalue span for RHS `p`. Its parent is an assignment expression. Removing the `left` check admits that span as a certificate candidate; declaration equality need not reject it.

   **Fix:** add this keep row with a later overwrite and use of `p`. The current implementation retains the protective check; the concern is the claimed redundancy and mutation coverage.

The remaining structural protections look sound on reading: F1 candidates are certified, initializer-value spans are excluded, compound/self assignments retain reads, comma/nested assignments do not dominate, E3 is owner-wide, and E4 conservatively refuses member/subscript roots. Ordinary label/case windows, loop re-entry, switch regions, spelled returns-twice names, and GNU nested functions are handled.

Both existing-test changes are legitimate for their stated purpose. `x += 2` preserves the #645 CFG shape while providing a genuine read for its pinned edge. The `lexical_prune_tests` adjustment retains subset and label assertions while allowing intended C removals.

The v175 pins are consistent. CLAUDE.md describes the intended rules, but its guarantees need the fixes above. Explicit C dispatch supports non-C isolation; the reported byte comparisons and test totals remain supplied receipts.

**SMELL · IMMATERIAL — timing characterization:** the supplied means are **11.53 s → 12.955 s**, approximately **+12.4%**, not +10%. Report those figures directly; two runs per revision do not establish a stable performance estimate.

VERDICT: REQUEST_CHANGES

---
# Appendix B: round-1 Opus review
# Review: PR #811, #630 slice 6b (C rules U and D for `.c` owners), round 1 of 3

Reviewer: Opus (static reading only). Worktree `~/code/prism-6b-review` at cf8810f7. Diff `f26f4555...HEAD`.
Design of record: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md` §5.1, §1.2, §2, §5.4.
Goal (from the brief): removal-only precision. A dropped def→use pair must carry no real value path, no edge
may be added, surviving labels must not change, and non-C output must be identical.

Nothing was built or run. Grammar facts are for tree-sitter 0.25.10 and tree-sitter-c 0.24.1 (Cargo.lock),
taken from memory of those grammars. I did not open `node-types.json`.

## Summary

The structure follows §5.1 closely. Every gate and rule in the design maps to code:
- G1 to G6;
- E1 to E4;
- the variable-write certificate;
- direct-child regions;
- the label and nested-`case` window refusal;
- the switch-body exclusion;
- compound assignment never dominating;
- census consumption.

Non-C paths are unchanged: every new branch is gated on `Language::C` or `self.c`, and `decl` and `c_decl` are
inert for other languages.

I found one constructible wrong drop. It comes from the E1 resolver's model of where declarations sit. The fix is
small and closed. Everything else is a SMELL.

---

## WRONG

### W1 — WRONG, MATERIAL (rare): E1 does not see declarations that are not direct children of a scope, so a conditionally compiled `extern`/`static` shadow resolves to an outer automatic local

**Where.** `c_prune.rs::resolve`, `scope_decls` and `decl_idents`. `scope_decls` scans only the direct
`declaration` children of a `compound_statement`, plus the `case_statement`s of a switch body. A declaration
wrapped in `preproc_ifdef`, `preproc_if`, `preproc_else` or `preproc_elif` is skipped. So is one wrapped in a
`labeled_statement`, if tree-sitter-c 0.24 admits C23 `L: int x;`, or in a non-direct (Duff) `case_statement`.
The walk then goes on to the enclosing scope, and can resolve the occurrence, W, and an assignment candidate (the
certificate also calls `resolve`) to an outer automatic local. E2 is then judged on the wrong declaration.

**Input.**
```c
int n;                       /* file scope */
void save(void); void restore(void); void sink(int);
void f(void) {
    int n = 0;               /* outer automatic */
    {
#ifdef USE_GLOBAL
        extern int n;        /* hidden: names the global (C11 6.2.2p4) */
#endif
        n = 7;               /* d, line 11 */
        save();              /* saves global n */
        n = 1;               /* W */
        restore();           /* writes the saved 7 back into global n */
        sink(n);             /* line 15: reads 7 under USE_GLOBAL */
    }
}
```
Trace (read, not run):
1. The occurrence `n` in `sink(n)`. `resolve` finds no direct declaration in the inner block, then finds
   `int n = 0` in the body block, so the binding is the outer automatic. E2 and E3 pass.
2. The window. In the inner block the latest plain direct-child write is `n = 1`, and `c_window_decl` resolves it
   to the same outer declaration. E4 passes and there is no label.
3. `keep_def` on d = `n = 7`. The span is an identifier, the `left` of an assignment, so the certificate passes.
   `resolve` gives the outer declaration, so `c_decl == window.decl`. d lies outside the window, so D **drops
   `n@11 → n@15`**.

Under `USE_GLOBAL`, 7 reaches `sink` through `save`/`restore`. `save` and `restore` are external, so main's
only edge carrying that value is the dropped one. This is design F3's scenario (a binding with linkage, restored
by a callee), and it gets past the lexical resolution that F3's fix requires.

A `static` variant needs no global, only recursion. Write `static int n;` under `#ifdef`, then
`n = 1; if (depth) f(depth - 1); sink(n); n = 5;` in the inner block. D drops `n@5 → sink(n)` (d lies after
`child_end`), but the recursive call leaves 5 in the static before `sink` reads it.

**Why it matters here.** §5.1 E1 says the occurrence "resolves lexically ... to a single declaration ... An
unresolved, ambiguous ... binding is ineligible". A conditionally present declaration makes the binding
ambiguous, so it must refuse. The parsed tree contains both branches, so the hazard can be seen statically.

**Fix (bounded).** Either of these:
- (a) In `scope_decls` and in the `for` and switch scans, also descend through `preproc_if`, `preproc_ifdef`,
  `preproc_else`, `preproc_elif`, `preproc_elifdef`, `labeled_statement`, `attributed_statement` and non-direct
  `case_statement`. Do not cross a `compound_statement`, `for_statement` or `function_definition`. If any
  declaration of the name is found under such a wrapper, return `None`.
- (b) Simpler, owner-wide, by name, matching how E3 is built. In `scan`, add to `escaped` every name declared by a
  `declaration` whose parent is not a `compound_statement`, a direct-child `case_statement`, or a
  `for_statement` (as its initializer).

Add a keep row, `preproc_hidden_extern_shadow_keep` (the input above), and its `static`/recursion twin. Each
first establishes that the baseline emits the pair. Amend the CLAUDE.md E1 sentence to match.

**When it would not apply.** In owners with no declaration under a preprocessor conditional or a label. That is
almost all of redis and cpython, so the expected recall cost of (b) is near zero.

---

## SMELLs

### S1 — SMELL, IMMATERIAL: the macro-argument escape (E3) is applied only when the call's `function` is a bare identifier
`walk` escapes the argument identifiers only for `call_expression` with `f.kind() == "identifier"`. Two forms
are missed:
- The preprocessor expands a function-like macro name after `->` or `.`. With `#define next(v) next_impl(&(v))`,
  the call `it->next(x);` takes `&x`, but the call's function is a `field_expression`, so `x` is not escaped.
  The census does add the macro's params and free names, but not the actual argument.
- Arguments of a `macro_type_specifier`.

Both are contrived. **Fix:** escape the arguments of any call whose function subtree contains a token that is an
in-repo macro name. **Would not apply** when no in-repo function-like macro shares a name with a struct field
that is called.

### S2 — SMELL, IMMATERIAL: array decay of a member is not an escape for U
`char *p = s.arr;` is not `&`, so `s` stays eligible for U. A line such as `sink(*p), s = t;` is then write-only
for `s`, even though it reads `s`'s storage through `p`. The surviving chain `s@d → p = s.arr → p@L` still
carries the value, so no flow is lost. D is protected by E4 (member root). This matches the Go/Rust U precedent,
so I note it only.

### S3 — SMELL, IMMATERIAL: G3's `kind.starts_with("ms_")` refuses `__declspec`, calling-convention, `__restrict` and pointer modifiers
This refusal is not in the design or in CLAUDE.md, and the ledger reports it as "asm/SEH". It costs little: 2
owners on each corpus. Either narrow it to `seh_*` plus the asm kinds, or document it.

### S4 — SMELL, IMMATERIAL: `subobject_free` finds the declarator again by a zero-width byte lookup and fails open
If `descendant_for_byte_range(start, start)` ever lands on another node, `pointer` is false and the name is that
node's text, which is not in `member_roots`, so E4 passes. This is safe on tree-sitter 0.25: the
`node_end <= range_start` skip lands on the identifier. But it is a fail-open belt. Carry the declarator `Node`
(or its id) from `binding`, instead of a byte offset.

### S5 — SMELL, MATERIAL to the evidence the design asked for (not a blocker): sampling and strata fall short of §5.5
§5.5 asks for 30 sampled drops per rule per corpus, a retained-opportunity sample by refusal reason,
E1–E4/label strata, and formerly capped or giant owners reported separately. The report has:
- 20 U + 10 D on redis, and 15 U + 5 D on cpython;
- no retained-opportunity sample;
- owner-level trace only, with no per-name strata;
- no capped-owner stratum.

D is the riskier rule, and 10 samples is thin. The rule is also *inferred* from whether the use line is
write-only, not taken from a counter. `samples.jsonl`, `timing.txt` and the ledger source are not in
`~/prism-evidence/.../s6b/`, so this reviewer cannot audit the 50/50 claim (custody).

**Recommended:**
- raise redis D to 30, and archive `samples.jsonl` and `timing.txt` beside the brief;
- disclose Z04/Z05 as not added. The report already says so, but CLAUDE.md does not. Z04 is partly covered by
  6a's digest rows, and Z05 by the generic cap tests.

The retained-opportunity sample only serves 6d decisions, so deferring it is proportionate.

### S6 — SMELL, IMMATERIAL to soundness: the +10% redis time
The runs are 2+2. Base varies by 0.8 s (11.14 s and 11.92 s); head ran 12.87 s and 13.04 s. So the roughly
1.4 s mean increase is likely real, but its size is loose. The cost has visible sources:
- `c_window_decl` resolves the occurrence again at every region where it finds a candidate write;
- `certified_candidate` runs `resolve` again for every `keep_def` call, uncached;
- `eligible_declared` tokenizes the specifier text on every call;
- `trace` reads the environment once per owner.

Memoizing `binding` by node id would remove most of it. A follow-up issue is enough, unless the #437 lane has a
time budget for C builds.

### S7 — SMELL, IMMATERIAL: hygiene
- Dead parameters and bindings: `classify_write(.., owner)` with `let _ = owner`, `chain_root` with
  `let _ = parsed`, and `keep_def` with `let _ = c`.
- The row `seh_ident_off` passes through G1 (a parse error), not through the G3 spelling it is named for. The
  report says so; rename the row or note it in a comment.
- The rows were not written RED-first. The guard spot-check table is a reasonable substitute: each drop row fails
  on base, and each keep row was killed by disabling its guard.

### S8 — SMELL, IMMATERIAL: CLAUDE.md wording
- It says `_Pragma` is part of the "textual belt". It is detected from the identifier node.
- E1 should state the hidden-declaration refusal once W1 is fixed.
- G3's `ms_*` refusal (S3) is not documented.
- Otherwise the paragraph matches the code. The v175 pins are consistent in `cpg_cache.rs` (constant and test),
  in all three `dfg_codec_tests.rs` asserts, and in CLAUDE.md. `git grep` finds no stale 174 pin.

### S9 — SMELL, IMMATERIAL, for #810 (census, not 6b): `_Pragma` in a macro body is neither `control` nor `poisoned`
A macro wrapping `_Pragma("omp parallel")` could hide F7's thread sharing. Every invocation shape I could build
(`PAR for (...)`, `OMP(omp parallel for)`) is a parse error, so G1 turns the owner off. I list it for #810's
census work.

---

## Focus items checked, and found correct by reading

- **F1 pointer stores.** `*p = v` spans a `pointer_expression`, so `declared()` gives `None` and the def is kept.
  `*p++ = c` spans an identifier under `update_expression`: not assignment-left, not a declarator, so it is kept.
  `p++`, subscript and field paths are kept. Alias twins fail the text check (`y` against base `x`), which the
  `tw` row pins.
- **F4/E4.** `member_roots` covers field and subscript roots, read or write, through parentheses. It is applied
  to non-pointer declarators at the window.
- **F7.** The textual G2 belt (`#pragma omp|acc`, `#define`, `#undef`, `#include`, `#include_next`, `#import`),
  the node kinds and `_Pragma` are all handled.
- **F3/E1** for visible declarations: globals are never resolved, because the walk ends at the
  `function_definition`. Declaration order is honoured (`ident.start <= occ.start`, so `int x = x` binds
  itself, as in C). Ambiguity refuses, and K&R refuses.
- **The initializer span trap.** In `int y = x;` the `x` span has parent `init_declarator`, but not as its
  declarator, so `classify_write` gives `None`. The certificate also refuses it.
- **goto/labels.** `entry_in_range` scans the region's subtree for labels and non-direct cases in
  `[W.start, child_end)`. A window found in an outer region covers inner regions whole. G6 compares goto
  targets against the owner's labels. Computed goto parses as an error (`l7` holds through G1).
- **switch/case.** A switch body is not a region. A direct-child `case_statement` is. Statements before the first
  case have no region (`s1`). Fallthrough is kept because the window climbs to the region before the switch
  (`s3`). The Duff case is refused (`s4`), and direct cases of a nested switch inside the window do not refuse
  (`s5`).
- **Loops.** The window spans the child that contains the use, so loop-carried defs inside it are kept. Defs
  after `child_end` are reached only through re-entry at the region top, which passes W.
- **setjmp.** G4 checks the spelled names, and the census `control` flag covers them inside macros.
- **Rule D.** The certificate's declaration equals `window.decl` for every window. Compound assignment is
  filtered out of W. Assignments that are chained, inside a comma, or under `?:`/`&&` are never statement
  children, and neither are declarations without an initializer. GNU nested functions turn the owner off (`nf`).
- **Rule U.** `x += 1` and `x = x + 1` carry real reads. `int y = x` is not a write of `x`. Occurrences inside
  the arguments of an in-repo macro escape. The `va_*` builtins escape their arguments. A comma or `&&` write is
  write-only for its own target only. Occurrences of field and type identifiers only add constraints.
- **Census consumption.** Every name-kind node (identifier, type, field, label) adds its macro's free names and
  params to `escaped`, adds macro names to `macro_names`, sets G5 on `poisoned` and clears `d_ok` on `control`.
  `incomplete` turns every owner off. Production passes `CMacroCensus::from_files(census_files)`, which is the
  whole repository for scoped builds. `of_file` is used only in `cfg(test)`.

## The moved tests

- **`for_in_switch.c` line 8, `x = 2;` changed to `x += 2;`: legitimate.** The #645 row pins CFG edges and the
  RD label `x 5->8 exact`. On base, that pair ran into a write-only line, which is an anchor edge that U is meant
  to remove. With `x += 2`, line 8 reads `x`, so the same reaching fact stays observable, and the CFG edges do not
  move. There is no `.cpp` twin. An alternative would have kept `x = 2;` and added a read, but either is fine.
- **The `lexical_prune_tests` change: legitimate.** `.c` moves from byte-identical to subset + labels + the F1
  invariant, while `.h`-as-C and C++ stay byte-identical. On base, that test's equality assertion for C is also
  the Z01 evidence: the frozen producer equals main without C pruning on the fixture tree.

## Survivors claimed to be belts

I agree with each claim:
- The W-target declaration check is subsumed by candidate-declaration equality. Every candidate bound to an inner
  declaration lies inside the outer window.
- `is_plain_declarator_chain` is subsumed by `declared()`.
- The assignment `left` check is unreachable from today's extraction.
- The G3 spellings are masked by G1.

Keep all of them as belts.

## Verdict

One closed, enumerable WRONG (W1) with a bounded fix and two new keep rows. Everything else is SMELL.

VERDICT: REQUEST_CHANGES

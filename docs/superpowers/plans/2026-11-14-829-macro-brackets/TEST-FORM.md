# #829 macro/parse mismatch: form of the test suite (design consult, Fable)

Worktree `prism-829`, branch `fix/829-macro-brackets`, HEAD `ccce81a0`, release binary of 12:58 (gates are stubs, so "head" below behaves like main for U and D). Apple clang 21.0.0. Nothing tracked was edited. Scripts and saved results: `.scratch829/consult/probes/`.

## 0. Bottom line

1. **Stop hand enumeration now.** Three serial chains each ended MORE-GAPS and each opened new classes. Under the convergence rule that is an open class: more rounds re-roll the same distribution. The hazard space is "everything the preprocessor, the lexer and the build configuration can do", and no list of shapes closes it.
2. **A mechanical oracle exists and works.** Take clang's post-expansion token stream, lay it out as macro-free text with source tokens on their own lines, run the same prism binary on it, and compare. A pair that prism drops on the source but keeps on the expansion is a drop whose lexical proof does not hold for what the compiler sees. Checked on 101 enumerator probes, 67 existing obligation rows, 543 generated programs with a runtime cross-check, and redis and leveldb without compile commands; cpython needs a configuration (section 1).
3. **The #818 layered form applies in outline, not in content.** Keep: a denominator, rows that carry measured state plus mechanical truth plus a policy, corpus differentials, wrong-design scoring, objective exit. Drop: grammar cells and "a cell determines main's outcome" (measured false here), span rows, per-occurrence truth labels, the consumer harness. Add: an assumption ledger for BOUNDARY rows, a bounded generator as the completeness instrument, a recall ledger, and a monotonicity gate.
4. **"Is enumeration complete?" cannot be answered for this class. Replace it** with three checkable claims: every known mechanism is a production of a generator whose bounded space the oracle has judged; the oracle finds no untriaged contradiction on corpora; the design is a positive certificate, so an unseen hazard lands in "refuse", and enumeration then bears on recall only.

## 1. What I checked

**The oracle (script `probes/xarm.py`).** Four prism runs per repository: T (source), Tb (source plus one file with a trigraph directive, which sets the census `incomplete` and turns U and D off repo-wide, giving the baseline with no code change), X (each main file replaced by its expansion), Xb (X plus the same file). For each pair dropped in T: CONFIRMED (also dropped in X), CONTRADICTED (present and kept in X with the owner on), XOFF (the owner is off in X), UNJUDGED (the pair does not exist in X).

- `clang -E` alone is not enough: a multi-line invocation is printed on one line, so every owner inside an argument group loses its line numbers (first attempt: all rows unjudged). `clang -fsyntax-only -Xclang -dump-tokens` gives each token's expansion and spelling location; verbatim and argument tokens keep their spelling line, body tokens stay on the current line. That is a lexer-only action, so a file that does not parse still yields a stream.
- **Enumerator probes (101 directories: 47 of opus and opus3, 35 of sonnet3, 19 of the design phase).** I compared verdicts with the lists for the opus and design probes, not row by row for sonnet3.
  - Every class-E probe that opus compiled as live came out CONTRADICTED (`dropp`, `lam`, `lam2`, `sib`, `closer`, `decl`, `cls`, `tpl`, `tmpl`, `ns`, `encl2`, `strip`, with their C++ twins), and so did the issue probes (`p_b`), `if1.c` and `a1.cc`.
  - The "checked clean" probes came out CONFIRMED (`commaop`, `proto2`, the keyword-prefix sets `p_k2`, `p_k3`).
  - Probes that need a flag or an outside header came out CONFIRMED with the repository alone and CONTRADICTED once it was given (`extencl` and `extapply` with `-include`, `c89` with `-std=c89`; also five inline rows with `-include pthread.h`, below). That is a mechanical definition of BOUNDARY.
  - Probes that are not valid programs (census-fact probes such as sonnet3 `c1`, `c2`, `l1` to `l4`) came out XOFF.
  - The design's C scope-shift probes (`c1.c`, `t1.c`, `a1.c`, `p.c`) came out with no contradicted pair. That reproduces TESTS.md deviation 1 (the baseline never had the pair) without a reviewer.
  - **Side finding, outside this consult's question:** the statement-boundary probes behind OD-2 and #831 (`s1.c`, `s1.cc`) show no U or D drop on head. T and the baseline have identical edges, also with the enumerators' asm twin; the live pair `z@2 -> z@7` is missing from the baseline itself, because the inner `z` shadows it before any prune rule runs. DESIGN section 2 and the #831 draft say "rule D drops the outer def". Worth checking before anyone works on #831.
- **Existing rows (54 keep rows and 13 drop rows of inline cases 20 to 33, C).** All 13 drop rows are CONFIRMED. The keep rows:

  | count | oracle result | rows |
  |---|---|---|
  | 17 | CONTRADICTED with the repository alone | in-repo macros |
  | 5 | CONTRADICTED once `-include pthread.h` is given | `w1`, `w2`, `w3`, `w8`, `b12` |
  | 18 | CONFIRMED; they name CPython, NumPy, PostgreSQL or `-D` definitions the fixture does not hold | not re-run with those headers |
  | 8 | **CONFIRMED with nothing missing** | `b6`, `b8`, `a3`, `a4`, `a5`, `a6`, `w7`, `g2s07` |
  | 3 | XOFF | `b7`, `b9` (the expansion is not a valid program), `g2o03` (needs `-std=c2x`) |
  | 2 | not dropped on head | `k5a`, `k5b` |
  | 1 | UNJUDGED | `a2` |

  The eight rows in bold assert "kept" where the proof holds on the compiler's tokens. They encode the tentative rule's over-refusal, not a requirement.
- **Runtime cross-check (scripts `probes/gen.py` plus an inline driver).** 7,826 programs (two object-like macros with bodies of up to two fragments from 12, two use sites, guard-shape owner); 543 compile; head drops the pair `x@4 -> x@8` in 325. Oracle against "does `probe(0)` or `probe(1)` return 41":

  | oracle | runtime live | runtime dead |
  |---|---|---|
  | CONFIRMED | 0 | 298 |
  | CONTRADICTED | 15 | 12 |

  No live pair was confirmed. The 12 dead-but-contradicted programs are unconditional wrappers (`{` ... `}` with the write inside): rule D does not prune across a nested block on plain text either, so the oracle is stricter than liveness by exactly rule D's own conservatism. Filtering the 7,826 candidates and judging the 543 took under half a minute; the 325 runtime drivers took longer than everything else.
- **redis, no compile commands.** 133 of 133 `src/*.c` files tokenized; three missing headers were stubbed empty (`release.h`, `port.h`, `sys/epoll.h`); in-repo directories went on `-idirafter` (with `-I` a bundled `msvc_compat/stdint.h` shadowed the system header and 78 files failed). About two minutes including the four prism runs.
  - Head drops 23,425 pairs in `src/`: **21,423 CONFIRMED (91.5%), 182 CONTRADICTED (0.8%), 1,820 UNJUDGED (7.8%)**. The unjudged pairs sit mostly in owners with no token in the stream, that is, code the host configuration does not compile.
  - All 182 contradictions are one mechanism in two files (`dict.c`, `ebuckets.c`): `offsetof(T, f)` expands to `__builtin_offsetof(T, f)`, which makes the type name an identifier of the file and fires the phantom-declaration gate in the X arm only. That is oracle noise, found by grouping on the X arm's refusal reason. **Genuine contradictions on redis `src/`: 0.**
  - 27,046 pairs are dropped in X and kept in T. Treat it as an upper bound on recall headroom, not as a sound number.
- **leveldb (C++), no compile commands.** 77 of 77 files tokenized. In-repo headers resolve once the script applies the census's own suffix rule for a missing include (it adds `include/`); five outside headers were stubbed (gtest, gmock, benchmark, kcpolydb, windows). Head drops 479 pairs: **473 CONFIRMED, 6 UNJUDGED, 0 CONTRADICTED.** With gtest stubbed, the `TEST` macros stay unexpanded in both arms. The run took about seven minutes, nearly all of it Python parsing token dumps of standard-library headers.
- **cpython `Objects/`, no compile commands: this is where guessing stops.** 26 of 77 files tokenized. 46 stop at `#error "this header requires Py_BUILD_CORE define"`. With `-DPy_BUILD_CORE` only 22 tokenize: the checkout's only `pyconfig.h` is the Windows one (`PC/pyconfig.h`), the include guess picks it up, and 51 files then fail on `WINAPI_FAMILY_PARTITION`. The host `pyconfig.h` is generated by `configure`, which I did not run (corpus code is not to be built here). On the 26 files: 402 CONFIRMED, 55 UNJUDGED, 2 CONTRADICTED (one variable in `mimalloc/segment.c`, refused in the X arm with the same reason code as the redis noise; I did not trace its trigger).
- **Do cells determine main's outcome? No.** Script `probes/sig.py` gives each redis `src/` owner a signature from the token stream (which expansion effects occur inside it). Every signature with more than a handful of owners has both `on` and `off` owners (`inert-only` 2,809 on, 161 off; `ctrl` 178 on, 5 off). Main's verdict depends on how tree-sitter happens to parse the unexpanded text, not on what the macro does. The signature flags are rough: `arg_split` has false positives from nested expansions inside an argument and `ctrl` counts `do { } while (0)`.
- **Population on redis `src/`.** 5,436 owners; 1,511 (28%) contain no macro token (this count includes owners the host configuration does not compile) and hold 6% of the confirmed drops; 23% of post-expansion tokens come from macro bodies (233k of 997k). Expansion instances that are not well-nested inside an owner: 0. Owners inside an enclosing invocation: 0. The enumerators' corpus scans agree: the classes are rule-derived, with no instance in the wild so far.

## 2. Options, by axis

### A. What a row asserts

| option | pros | cons |
|---|---|---|
| A1. Named pair kept or pruned, kind tagged by hand (S, C, K): today | Readable; already design-neutral for S rows; red on base | C rows have no evidence (8 of 54 audited keep rows assert "kept" where the oracle confirms the drop); K rows make recall a requirement; the tag is a reviewer's judgement |
| A2. Census fact values and gate reasons (F rows, the first matrix) | Pins the mechanism | Encodes the tentative design; about 75 tests |
| A3. Differential row: fixture plus configuration; must-keep set computed by the oracle and frozen as data | No hand judgement; covers every pair of the fixture; red, green and guide are derived | Needs clang at authoring time; strict (proof), not liveness |
| A4. Runtime witness: a driver prints the value | The strongest truth; what "live pair dropped on main" means | One driver per row; cannot run on corpora; needs a compiling program |
| A5. Whole-fixture golden of main's dropped pairs | Makes "the fix only keeps more" checkable per fixture | Data to maintain |

### B. Oracle

| option | pros | cons |
|---|---|---|
| B1. Hand reasoning from the rules | Covers everything | The source of the C rows and of DESIGN section 2's wrong "pair dropped" entries (TESTS.md deviation 1) |
| B2. Compile and run (the enumerators' `41 2`) | Truth | Ad hoc; receipts are not in the tests; not for corpora |
| B3. Expansion arm: clang token stream, macro-free text, same binary | Mechanical; fixtures and corpora; needs no product patch and no design; also measures recall | Judges by prism's own rules on plain text, so it assumes 6b and 6c are sound there; one build configuration; noise where an expansion trips a heuristic gate |
| B4. Structural labeller (SAME / DIFFERS on nesting, statement boundaries, bindings) | Independent of U and D | It is a second design of "which difference matters"; every new class would need a labeller change |
| B5. clang AST dataflow as an independent liveness oracle | Independent truth on any program | A dataflow analysis to write and trust; not proportionate |

### C. How coverage is proven

| option | pros | cons |
|---|---|---|
| C1. Reviewer recall | Finds classes nobody modelled | Open class; no stopping rule; where we are |
| C2. Cells from the tentative facts (unbalanced, loose comma, ...) | Easy to tabulate | Partitions one design, not the hazard |
| C3. Axis map: knowledge × lexing × expansion effect × owner position | Finite; gives every row coordinates and the generator its grammar | Cells do not determine outcomes (measured); a map, not a proof |
| C4. Corpus sweep with the oracle | Measures what occurs; denominators per corpus | Relative to corpora and one configuration |
| C5. Bounded-exhaustive generator plus oracle | A real denominator: every compiling program inside the bound is judged | Complete only for the grammar it is given |
| C6. Premise coverage: each premise of the design's soundness argument has a row that fails when it is removed | Ties coverage to the proof | Waits for the design |
| C7. Assumption ledger for inputs prism cannot see | Finite; turns BOUNDARY from a pile into a list | Needs owner placement per assumption |

### D. Regression detection on corpora

| option | pros | cons |
|---|---|---|
| D1. Monotonicity: the fix's C/C++ edge set contains main's | Exact property of a refusal-only fix; two runs per corpus | Says nothing about needless refusals |
| D2. Oracle sweep on the fix: no untriaged CONTRADICTED | Finds remaining holes where they occur | Noise clusters need a short triage table |
| D3. Recall ledger: pairs restored, split by oracle verdict and by gate reason | The guide the owner asked for; compares designs | Upper bounds only where the X arm is itself conservative or blind |
| D4. Byte parity for other languages, cache cold, full and partial | Cheap, existing tools | None |
| D5. Consumer harness (taint and so on) | Shows user-visible effect | Edges only come back; not needed for this lane |

### E. Wrong-design sensitivity

| option | pros | cons |
|---|---|---|
| E1. Implement each wrong design in the product | Faithful | Hundreds of lines per variant |
| E2. Score designs as owner-refusal predicates in the harness: restore every baseline pair of a refused owner, then check must-keep and count lost CONFIRMED drops | No product code; works before a design exists; gives soundness and recall per design | Fits "owner off" designs; a finer design (escape names only) needs its own model |
| E3. Mutate the implemented gate | Standard | After the design; mutation gates are not per-PR any more |

### F. Exit criteria for enumeration

| option | pros | cons |
|---|---|---|
| F1. A reviewer round reports no gaps | Simple | Has not happened in three chains; the counts are not falling (round 3: 13, 10, 3, 3) |
| F2. Capture-recapture estimate | A number | Biased; the classes are not exchangeable |
| F3. Mechanical: generator dry at bound k and k+1, corpus sweep clean, wrong designs rejected, audit done | Objective and checkable | Relative to the grammar and corpora |
| F4. Certificate: the design refuses whatever it cannot prove inert | Makes completeness irrelevant to soundness | A design requirement, with a recall cost to measure |

## 3. The #818 form: what fits, what does not, what is new

**Fits.**
- Layering, and the rule that a row holds measured state, mechanical truth and a policy, with red and green derived.
- Demoting everything that encodes the provisional design.
- A pre-design instrument that produces "post-fix" results for any candidate. In #818 that was a reference-arm patch. Here it is E2 plus the X arm, and it needs no patch.
- Tiers with mechanical acceptance and declared caps.

**Does not fit.**
- Grammar cells and "each cell determines main's outcome". The defect is a missing premise of a proof, not a wrong walk over node kinds; outcomes are mixed in every cell (section 1).
- Span rows and per-occurrence truth from the clang AST. The unit stays (owner, pair), and truth is differential.
- A second classifier for corpora. The X arm is the independent side.
- The consumer harness and whole-fixture edge goldens with attribution. A refusal-only fix has a simpler invariant (D1).

**New here.**
- **Truth depends on the build configuration.** #818 had none of that. It needs the assumption ledger and a `config` field per row.
- **Recall is a first-class output**, because every candidate fix is a refusal.
- **Two grades of truth**: live at run time, and "the proof fails on the expansion". The second contains the first (15 against 27 in the generated sample).
- **The generator is the completeness instrument**, not a filler of empty cells.
- **Lifecycle rows** (cache parity, loader discovery) are a separate invariant, "incremental equals fresh", which the oracle does not touch.

## 4. Recommended form

Each layer names the decision it serves.

**L0. Assumption ledger and axis map.** Serves BOUNDARY placement and the design's scope.
- Five assumptions, each with one or two witness rows and a policy (in this PR, #813, or disclosed):
  - A-ext: an external macro acts as a function (D2), with the OD-3 names as exceptions.
  - A-inputs: the census reads every in-repo definition the compiler uses (#813): computed, absolute, symlinked, case-folded, NFC/NFD, `eval/secbench`, out-of-root, VFS, header unit, forced include.
  - A-config: which definition is in effect (`-D`, `#ifdef`, a repo shim of a listed name).
  - A-dialect: C99 or later, C++11 or later, standard preprocessing.
  - A-encoding: UTF-8 or ASCII-compatible sources.
- A row is BOUNDARY when its verdict is CONFIRMED with the repository alone and CONTRADICTED under its declared configuration, or when clang opens an in-repo file the census did not read. Both are computed, not judged.
- Axis map, used only as coordinates: knowledge (parsed, census text, external, unread) × lexing (plain, splice, digraph, raw string, comment and literal forms) × expansion effect (brackets, argument split, deletion or reorder, statement boundary, control prefix) × owner position (spells the macro, inside an enclosing group, header made by a macro, nested owner).

**L1. Oracle-labelled rows.** Serves accepting or rejecting a design on the known classes, and the red-on-base claim.
- Row = fixture files + `config` (flags, prelude headers, dialect) + assumption id or none.
- A script (the grown-up `xarm.py`, next to `gen_obligations.py`) computes per fixture: main's dropped pairs, the must-keep set (CONTRADICTED), the may-drop set (CONFIRMED) and the unjudged rest, and writes them into the obligations data. The Rust test reads data and needs no clang.
- Derived: RED when main drops a must-keep pair; a control is a may-drop pair, reported and not asserted.
- A runtime driver is kept for one row per class (about 15), as the receipt for "a live pair is dropped on main".

**L2. Bounded generator.** Serves "can we design against this set" now and "is the design sound inside the bound" at merge.
- Productions come from the enumerated classes: body fragments (brackets of all three kinds, digraphs, `,`, `;`, control keywords, declarations, parameters, `#`, `__VA_ARGS__`), macro kind (object, function, variadic, alias chain, wrapper of an external name), use site (statement, guard, initializer, declarator, argument, enclosing group opened at file scope, in a sibling or in a declaration), owner kind (C function, C++ method, template, lambda-local class), definition channel (same file, header, census text), lexical variant (splice, comment, CRLF).
- Only programs clang accepts count. The oracle judges all of them; contradictions are grouped by a key (expansion effect, site, owner position). Minimal members of each key are frozen as L1 rows.
- An enumerator finding is new only if the grammar cannot derive it. It then adds a production, not a row.
- Runs offline, not in CI. The probe filtered about 7,800 candidates and judged the compiling ones in under half a minute on this host.

**L3. Corpus differentials.** Serves design selection (recall) and the merge decision.
- On the corpora that tokenize (redis and leveldb do by guessing; cpython needs a supplied `pyconfig.h`; the other three were not tried): D1 monotonicity, D2 oracle sweep with a triage table keyed by the X arm's refusal reason, D3 recall ledger, D4 parity for other languages and cache modes.
- The ledger reports, per corpus and per gate reason: owners newly off, pairs restored, and how many of those were CONFIRMED (pure recall cost) or CONTRADICTED or unjudged.

**L4. Design scoring before implementation (E2).** Serves choosing the design and showing the suite discriminates.
- Candidates as refusal predicates: the tentative rule (unbalanced, loose comma, name list); the same without closure; spelling-only; no enclosing-group check; "any macro token refuses"; an inertness certificate.
- Each gets two numbers per corpus and on the generator: must-keep pairs still dropped, and CONFIRMED drops lost. Rough anchor from redis: refusing every macro-touched owner would lose 94% of the confirmed drops.
- The suite passes this layer when every known-wrong candidate has at least one violated must-keep row.

**Exit criteria for the enumeration phase (all mechanical).**
1. The audit of the existing rows is done and every disagreement is resolved (section 5).
2. Every axis value with an existing row or corpus mass has a witness with an oracle verdict.
3. The generator's space at bound k is fully judged, and bound k+1 (or two further batches) yields no new key.
4. The corpus sweep has no untriaged contradiction.
5. The known-wrong candidates of L4 are rejected.
6. At most one further reviewer round, with mechanical acceptance (the program is CONTRADICTED on the candidate and its key or assumption is new). Cap: one round per tier. An open-class result at the cap changes the grammar, not the round count.

**What waits for the design.**
- Fact rows, name-list pins, digest and aggregate-identity checks, `Off::` reasons and trace names, the cache-version test, CLAUDE.md text, mutants.
- Which controls become contractual.
- Depth and performance rows (closure cost).
- The policy of every assumption.

**Not worth doing.**
- Further hand-enumeration rounds in the current form.
- More members of A-inputs (each new path mapping is the same assumption; the work is #813's).
- B4 and B5.
- A runtime driver for every row.
- A census of every macro invocation by full cell cross-product: the oracle sweep answers the same question directly.
- clang in the Rust test path.
- More encoding and depth variants than one per assumption and one smoke row.

## 5. The existing tests (about 280 in four folds)

**Keep as they are.**
- Every pair row whose pair the oracle marks CONTRADICTED. They become L1 rows with a computed basis. In the audited sample that is 22 of 54 keep rows, and up to 18 more once their outside headers are supplied.
- The admissibility rule (the baseline must hold the pair), the owner-aware pair identity of round 3, and the `sink(x + 1)` convention for C++.
- The tempdir loader harness and the cache parity rows (`cz1` to `cz3`, terra3-03) as the lifecycle layer, minus their digest assertions.

**Change.**
- Add `config` to every row that names an outside macro. Without it the oracle, and any reader, cannot tell a D2 row from a hole. This also turns the hand tag BOUNDARY into a computed one.
- Demote controls (K) from "must prune" to recorded may-drop pairs. Keep a control as an assertion only where it guards test validity (the row could not fail otherwise).
- Re-tag C rows by audit. Those the oracle confirms as droppable (`b6`, `b8`, `a3` to `a6`, `w7`, `g2s07` so far) become guide rows: a design may keep or drop them, and the ledger counts it. Those whose expansion is not a valid program (`b7`, `b9`; TESTS.md says the same of `mf3`) are recorded as "no truth basis".
- Export the inline cases and the tempdir fixtures to files for the audit. A regex over the two inline-case files did it for 15 C cases here; the loader rows need a small dump hook in the test helpers.

**Quarantine until the design (about 80 tests).**
- `macro_census_brackets_tests.rs` and its r1, r2, r3 files (values of `unbalanced` and `loose_comma`).
- `block_macros_list_is_the_design_list`, `g_sol_01_test_names_are_the_production_list`, the 16-name loops as requirements, `cz3_premise_*`, and the digest halves of the `cz` rows.
- Mark them `#[ignore = "tentative design"]` or move them behind one module switch, so that the red count means "required behaviour missing" only.

**Drop.**
- Kind tags as the source of truth, and TESTS.md's hand-kept totals; generate the table from the data.

## 6. Boundary items

- Each BOUNDARY row gets an assumption id and a `config`. The oracle is run twice, repository-only and declared, and the two verdicts are stored.
- Policy per assumption, decided once:
  - in this PR: the rows become must-keep;
  - deferred (#813, #831): the rows assert today's drop as a pinned, named hole, so the later fix flips them visibly. `#[ignore]` hides that;
  - disclosed: one witness stays as a pinned hole and CLAUDE.md names the assumption.
- For A-inputs a second mechanical check is cheap: the in-repo files clang opened for a translation unit (`-H` or `-MM`) must be a subset of the census inputs. On corpora that gives #813 a denominator. I did not run it.
- **Baseline gaps are a neighbouring class, not this lane.** In `c1.c` and `s1.c` the live pair is absent before U and D run (the parser's phantom declaration shadows the outer name). The same sweep can count them (pairs kept in X that the source baseline never had), but the count mixes in edges that exist only because body tokens become visible, so it needs its own filter. I did not measure it.
- OD-2 (#831) needs no special handling. If the generator finds contradictions from `;` bodies, those keys are tagged #831 and excluded from this lane's gate. The `s1` probes themselves show none (section 1).

## 7. The controller's ideas

**1. Oracle from `clang -E`, by a structure labeller or by running prism on the expanded file; compile-and-run for liveness.** Agree with the second form and checked it. Three changes:
- `-E -P` and line markers fail for every multi-line invocation, which is all of class E. Use the token dump and place tokens by spelling location.
- Do not build the SAME / DIFFERS labeller. Deciding which structural difference matters is the design problem again. Running the unchanged rules on the expansion asks the right question with no new judgement.
- Compile-and-run is worth keeping for one row per class, not for all. The cross-check shows the differential misses no live pair in 325 cases, and it is stricter than liveness in a known way.

**2. Cells: macro source × replacement-list facts × invocation position × owner kind × lexing mode.** Close to my axis map. Two changes:
- "Replacement-list facts" are the tentative design's vocabulary. Key on the effect of the expansion at the site, read from the token stream (nesting, deleted or reordered argument tokens, a statement boundary, an enclosing group). `DROP(p, q, r) p + r` has no body fact at all.
- Do not expect cells to carry outcomes or to prove coverage. They are coordinates for rows, grammar for the generator and keys for triage.

**3. Corpus census of every invocation by cell, with main's verdict and whether the structure differs.** The feasibility worry is settled for redis and leveldb: guessed include paths plus empty stubs are enough. It is real for cpython, which needs its generated config header. I would run the oracle sweep per dropped pair, not a census per invocation: redis has about 22,000 inert expansion instances and no structural one inside an owner, so a per-invocation table is mostly zeros, while the sweep gives the two numbers that matter (contradictions and recall).

**4. Wrong designs to reject.** Agree with the list. Add the tentative rule as written (it is known to miss class E and the deleting macro), and the two extremes ("nothing new" and "any macro token refuses"), which bound the recall axis. Score them as refusal predicates in the harness, not as product code. "Owner off only for D" is covered by the C++ write-only rows.

**5. The rows stay "required behaviour", keyed to cells.** Agree for the S rows. Disagree for three groups: controls (recall is a guide), C rows without an oracle basis (eight of 54 in the audited sample), and the fact and list tests, which are the tentative design in test form.

**Absent from the controller's file:** an exit rule; the configuration dependence of truth; recall measurement; a completeness instrument other than cells; what to do with the lifecycle rows.

## 8. Owner decisions

- **OWNER-DECISION 1: stop the enumeration chains now and switch to the mechanical form.** Default: yes. The budget already spent belongs to this blocker; a fourth chain needs the generator first.
- **OWNER-DECISION 2: adopt the expansion-arm differential as the row oracle and the corpus gate.** Default: yes, as an authoring and gate script with frozen data; clang is not a test dependency.
- **OWNER-DECISION 3: which truth gates a design.** STRICT: no dropped pair may be CONTRADICTED. LIVE: only runtime-live pairs must be kept. Default: STRICT for fixtures, generator and corpora, with a written mechanism-level proof required to waive a contradicted pair. Cost: drops across unconditional wrapper macros are lost (opus counted about 80 cpython owners that drop 0 to 90 pairs each this way).
- **OWNER-DECISION 4: recall as guide.** Default: controls stop being assertions; the ledger is reported in the PR; no threshold. If the owner wants a floor, set it after L4 shows the spread between candidates.
- **OWNER-DECISION 5: quarantine the roughly 80 tentative-design tests until the redesign.** Default: yes, ignored with a reason, not deleted.
- **OWNER-DECISION 6: BOUNDARY policy per assumption.** Defaults: A-ext disclosed, with the listed-name exceptions decided by the design; A-inputs to #813 as pinned holes; A-config disclosed; A-dialect disclosed (C89 `//`, traditional cpp); A-encoding disclosed, UTF-16 without BOM already refuses.
- **OWNER-DECISION 7: ask the redesign for a positive certificate (option F6's direction).** Default: yes as the direction, chosen only after L4 gives its recall cost. With a list of hazards, soundness rests on enumeration being complete, which this lane cannot show.
- **OWNER-DECISION 8: generator bound and budget.** Default: bodies of up to three fragments, two macros, the site and owner lists of L2, one offline run of at most a few hours; exit at "no new key from k to k+1".
- **OWNER-DECISION 9: the eight audited conservative rows and the non-compiling rows.** Default: guide rows and "no truth basis", per section 5.
- **OWNER-DECISION 10: baseline gaps caused by the same mismatch (section 6).** Default: out of scope for #829; disclose the class next to D2 and re-check #831's premise against the side finding in section 1 before scheduling it.

## 9. Verified and not verified

**Verified by running it here.**
- The oracle's verdicts on 101 probe directories, 15 exported inline C cases and 543 generated programs.
- The runtime cross-check table.
- redis `src/`: tokenization rate, verdict counts, the one noise mechanism (read at source and expansion in `dict.c`; same shape in `ebuckets.c`).
- leveldb: tokenization rate and verdict counts, with in-repo headers resolved.
- cpython `Objects/`: tokenization rates with and without `-DPy_BUILD_CORE`, and the cause of the failures.
- The three BOUNDARY probes and five inline rows that flip when their flag or header is supplied.
- The `s1.c` and `s1.cc` side finding, with the poison baseline and with an asm twin.
- Mixed outcomes per signature on redis.

**Not verified.**
- cpython beyond 26 of the 77 `Objects/` files. It needs a host `pyconfig.h` and the core defines, that is, a per-corpus configuration.
- protobuf, re2, snappy: not run. C++ translation units are slow in the probe because Python parses the full token dump.
- The oracle on C++ fixtures beyond the probe directories; the C++ inline cases were not exported.
- The loader and cache rows: not exported, not audited.
- The X arm's own blind spots for compiler builtins (`__builtin_va_*` spellings are not the `va_*` names prism knows). I saw one noise mechanism; others may exist. They affect the recall bound first, and could in principle confirm a pair wrongly.
- One host configuration: code under other `#if` branches is unjudged (8% of redis drops).
- That labels of common edges are unchanged when pairs are restored (D1 was stated on edge keys).
- The signature flags are a sketch; the per-argument reconstruction is not written.
- E2 scoring: only the "any macro token" anchor was computed, from the signature table.
- The 18 audited rows that need CPython, NumPy or other outside definitions were not re-run with those headers.
- Whether #831's own text uses a probe other than `s1.c`/`s1.cc`; I tested those two only.
- The generator covers one owner shape and object-like macros only; it found the brace class and nothing else, as its grammar predicts.

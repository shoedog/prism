# #630 slice 6d: yield measurement (measurement only, no product change)

Base: `main` a63d51f2 (slice 6c). Measurement code: branch `test-input/630-s6d-yield` (commit 78fdb4c9, never to be merged; patch also in `measurement.patch` here). Issue #820 read, not commented. Static parse only: nothing was built or run from any corpus.

## 0. Answer in one paragraph

Muting the escapes recovers a lot of edges: all five candidates together remove **3,396 more edges than 6c's current 364 on leveldb+re2+snappy (9.3x; protobuf parses but yields 0)**, and 18,140 vs 992 once protobuf is made measurable (see §2). No candidate ever **added** an edge (0 added edges in all 100 muted corpus x switch runs). But the yield is very unequal, very concentrated, and the candidates are strongly synergistic:

* **C1 (call arguments) is the biggest lever** (unique yield 2,567 edges official / 13,050 suppl.; alone 310% / 558% of the current yield) but only a minority is certifiable by a bounded local callee proof: of the 202 distinct variables C1 frees (suppl.), 31 have every blocking callee resolved to one in-repo definition taking the argument by value / `T*`, 6 more only through overload sets in which every overload is by-value/`T*`, 80 are blocked by (at least one) method call that needs receiver typing, and 82 hit an external/unresolved callee, a reference parameter, arity/variadic, a constructor/brace init or an overloaded reference. At edge level 919 of 5,535 edges (17%) are all-A; 616 of the 668 leveldb C1 edges are one variable (`status` in `db_bench_sqlite3.cc`).
* **C2 (size_t/intN_t spellings)** and **C4 (class-pointer locals)** are cheap, bounded and census-provable, and their yield is real but modest on hand-written code (C2 alone 88 edges official; C4 alone 52 official), large only in protobuf (generated/unit-test code: 2,409 of the 2,697 protobuf-x C4 edges are in `message_differencer_unittest.cc`, 1,841 of them the variable `item`). The sampled C4 types are generated test messages, which the census cannot prove; the leveldb/re2 C4 edges are real in-repo classes.
* **C3 (fix E arithmetic, scalar casts)** yields almost nothing alone (8 edges, 2 variables) and 221 edges of unique yield only in combination: **drop it from a slice unless folded into C1/C5**.
* **C5 (operand certificate)** alone is 93% (official) / 67% (suppl.) of the current yield; by reason, the dominant blockers are a global/unknown other operand (277 edges), a cascade from another candidate (229), a call (175) and a local with a non-keyword type (75); `x + *p`/deref is nearly absent (2 edges).
* **P1 (.c phantom `a * x;`)**: **0** firing sites on redis 7.2.4 `src/` (756 shaped statements whose first identifier is not an object in its own file; none is declared as an object, enumerator or object-like macro in any in-repo header).
* **Protobuf v28.2 `src/` yields 0 at 6c and under every switch** because the repo-wide C census is `incomplete` (`uncertain_raw_string` in `compiler/cpp/enum.cc`), which turns U/D off for all C/C++. That is a finding of its own; I added a clearly labelled supplementary corpus `protobuf-x` (the same tree minus that one file) to get a yield for the largest corpus.

## 1. Setup

Corpora (parse only, SHAs resolved):

| corpus | ref | resolved SHA | note |
|---|---|---|---|
| leveldb | `7ee830d0` | 7ee830d02b623e8ffe0b95d59a74db1e58da04c5 | full clone, `third_party/` removed |
| re2 | tag `2024-07-02` | 6dcd83d60f7944926bfd308cc13979fc53dd69ca | depth 1 |
| snappy | tag `1.2.1` | 2c94e11145f0b7b184b831577c93e5a41c4c0346 | depth 1, `third_party/` removed |
| protobuf | tag `v28.2` (the tag is spelled `v3.28.2` by `git describe`) | 9fff46d7327c699ef970769d5c9fd0e44df08fc7 | depth 1, only `src/` kept, any `third_party` removed |
| protobuf-x | (suppl.) protobuf `src/` minus `src/google/protobuf/compiler/cpp/enum.cc` | same | the only census-incomplete reason (iterated until none) |
| redis | tag `7.2.4` | d2c8a4b91e8c0e6aefd1f5bc0bf582cddbe046b7 | depth 1; P1 measured on `src/` only |

All tags existed; no fallback to a default branch was needed.

Runs: `RAYON_NUM_THREADS=4 prism nav --no-cache dfg-stats --edges --repo <corpus>`, release builds of main (a63d51f2) and of the branch; edge dumps compared as full multisets of the complete JSON line (from file/line/path/access, to ..., confidence, doubt, kill_line); no dict is keyed on (from, to). Configurations: default, `cpp_off`, each candidate alone (and sub-switches), all together, and all-but-X for each X (unique yield). 21 runs x 5 corpora + main/default.

### Switch code points (`PRISM_6D_MUTE=<comma list>`, unset = main)

`all` = C1..C5 together; `c1`=`c1call,c1ctor,c1brace`; `c3`=`c3arith,c3cast`; `c5`=`c5cmp,c5arith,c5shift,c5other`.

| switch | code point (file, function) | what it mutes |
|---|---|---|
| `cpp_off` | `lexical_prune.rs`, `c_owner` construction | no C/C++ owner scan for `Language::Cpp` files: U/D off for all C++ owners (denominator). `.c` files unaffected |
| `c1call` | `cpp_prune.rs::note_node`, arm `argument_list`, kind `call_arg` | bare (lvalue-root) argument of an ordinary call; **excludes** scalar casts (below) |
| `c1ctor` | same arm, kinds `mem_init` (field initializer), `new_arg` (`new T(x)`), plus `direct_init` (`Foo f(a);` escape in the `declaration` arm) | constructor-style arguments |
| `c1brace` | same arm, kind `brace_init` (`initializer_list`) | brace initializers |
| `c3cast` | same arm, kind `call_arg_cast` (a new kind split out of `call_arg`): the call's callee is a keyword-scalar functional cast (`int(x)`, `unsigned(x)`) or `static_cast<T>(x)` with `T` a keyword scalar or a pointer to one | argument of those casts |
| `c3arith` | `cpp_prune.rs::note_node`, arm `assignment_expression`, both the `assign_rhs` immediate escape and the deferred (`Need::Scalar`) escape | the right side's escape of `=` when the peeled right side is a `binary_expression` or `unary_expression` (`h.m = x + 1`); a right side that is itself an lvalue root, a call, etc. still escapes. The operators inside the right side are still certified by the operand certificate |
| `c2` | `cpp_prune.rs::scalar_gate` (E2) | admits `size_t ssize_t ptrdiff_t intptr_t uintptr_t intmax_t uintmax_t [u]intN_t [u]int_leastN_t [u]int_fastN_t`, with or without leading `::`/`std::`, whether the grammar reads them as `primitive_type`, `type_identifier` or `qualified_identifier`. Applies to declarations and parameters through `scalar_gate` only: `keyword_scalar` (casts, array locals, the operand certificate's cast rule) is unchanged, so `(size_t)x` is still a non-scalar cast |
| `c4` | `cpp_prune_names.rs::note`, arm `declaration` (kind `nonscalar_decl`) | declarator names of a non-scalar-base declaration whose declarator is a pointer declarator (through parentheses): `Iterator* it`, `const Slice* s`. Non-pointer class-typed locals still escape |
| `c5cmp` `c5arith` `c5shift` `c5other` | `cpp_prune_operand.rs::certify` (immediate and deferred, so also the `settle` fixpoint) | whole operand certificate by operator family of the operator node: cmp = `< > <= >= == != <=>`; arith = `+ - * / % & \| ^`; shift = `<< >>`; other = logical `&& \|\|`, comma, compound assignment, subscript, `?:` |

Not muted by any switch (so not part of any yield here): `ctor_conv`, `addr_of`, `ref_init`, `range_for`, `lambda_capture`, `type_name`, `template_arg`, `enumerator`, `cast` (non-scalar casts), `assign_lhs`, `unmodelled_block_form`, and the `nonscalar_decl` escape of non-pointer declarators.

`PRISM_6D_LOG=<file>` (default config only) appends one TSV record per escape (owner file and line range, kind, escaped name, line, callee/operator/reason) and one per P1 candidate; this log is what joins an extra-removed edge to the escapes that blocked its variable (join key: file, innermost owner range containing both edge lines, variable name).

### Default-identity proof

With `PRISM_6D_MUTE` unset the branch binary produces a byte-identical sorted edge dump to the main binary (hence an equal full multiset; `cmp` of `LC_ALL=C sort`ed outputs):

| corpus | edges | main == branch default |
|---|---|---|
| leveldb | 28,915 | identical |
| re2 | 29,661 | identical |
| snappy | 15,835 | identical |
| protobuf | 1,436,579 | identical |
| protobuf-x | 1,435,811 | identical |

(The measured binary was built just before a `cargo fmt --all` of the same files; fmt changed layout only.)

## 2. Yield per corpus

Cells: edges removed in addition to the default (default-minus-switch multiset size), and as a percentage of the 6c yield on that corpus (edges present with C++ rules off and absent at default, `cpp_off`→default). **Added edges: 0 in every one of the 100 muted runs** (multiset muted-minus-default is empty), so no finding of the "added edge" kind. `cpp_off` itself never loses an edge relative to default either (default minus cpp_off = 0).

| switch | leveldb | re2 | snappy | protobuf (official) | protobuf-x (suppl.) | SUM official | SUM suppl. (3 + protobuf-x) |
|---|---|---|---|---|---|---|---|
| 6c yield (cpp_off→default) | 151 | 194 | 19 | 0 | 628 | 364 | 992 |
| c1call | 668 (442%) | 447 (230%) | 12 (63%) | 0 (n/a) | 4401 (701%) | 1127 (310%) | 5528 (557%) |
| c1ctor | 0 (0%) | 1 (1%) | 0 (0%) | 0 (n/a) | 6 (1%) | 1 (0%) | 7 (1%) |
| c1brace | 0 (0%) | 0 (0%) | 0 (0%) | 0 (n/a) | 0 (0%) | 0 (0%) | 0 (0%) |
| c1 | 668 (442%) | 448 (231%) | 12 (63%) | 0 (n/a) | 4407 (702%) | 1128 (310%) | 5535 (558%) |
| c2 | 57 (38%) | 28 (14%) | 3 (16%) | 0 (n/a) | 640 (102%) | 88 (24%) | 728 (73%) |
| c3arith | 0 (0%) | 0 (0%) | 1 (5%) | 0 (n/a) | 0 (0%) | 1 (0%) | 1 (0%) |
| c3cast | 0 (0%) | 0 (0%) | 0 (0%) | 0 (n/a) | 7 (1%) | 0 (0%) | 7 (1%) |
| c3 | 0 (0%) | 0 (0%) | 1 (5%) | 0 (n/a) | 7 (1%) | 1 (0%) | 8 (1%) |
| c4 | 20 (13%) | 32 (16%) | 0 (0%) | 0 (n/a) | 2697 (429%) | 52 (14%) | 2749 (277%) |
| c5cmp | 10 (7%) | 16 (8%) | 65 (342%) | 0 (n/a) | 127 (20%) | 91 (25%) | 218 (22%) |
| c5arith | 6 (4%) | 13 (7%) | 1 (5%) | 0 (n/a) | 1 (0%) | 20 (5%) | 21 (2%) |
| c5shift | 0 (0%) | 8 (4%) | 0 (0%) | 0 (n/a) | 0 (0%) | 8 (2%) | 8 (1%) |
| c5other | 10 (7%) | 100 (52%) | 0 (0%) | 0 (n/a) | 157 (25%) | 110 (30%) | 267 (27%) |
| c5 | 38 (25%) | 187 (96%) | 114 (600%) | 0 (n/a) | 328 (52%) | 339 (93%) | 667 (67%) |
| all | 1525 (1010%) | 1595 (822%) | 276 (1453%) | 0 (n/a) | 14744 (2348%) | 3396 (933%) | 18140 (1829%) |

Reading: percentages above 100% are normal (the muted rules remove more than the whole 6c yield). The 6c yield on leveldb (151) reproduces issue #820.

### Unique yield and overlap

Unique yield of X = (all together) minus (all but X). Distinct vars = distinct (file, owner, variable) among the extra edges.

| candidate | unique yield, official sum (all − all-but-X) | unique yield, suppl. sum | single (alone), official | single, suppl. | distinct vars (suppl., single) |
|---|---|---|---|---|---|
| C1 call_arg (all kinds) | 2567 | 13050 | 1128 | 5535 | 202 |
| C2 scalar spellings | 479 | 2986 | 88 | 728 | 139 |
| C3 fix-E arithmetic + scalar casts | 74 | 221 | 1 | 8 | 2 |
| C4 class-pointer locals | 729 | 7943 | 52 | 2749 | 95 |
| C5 operand certificate | 1838 | 3964 | 339 | 667 | 119 |
| all together | 3396 | 18140 | sum of singles: 1608 | 9687 | 1135 |

Per corpus (suppl. set):

| corpus | all | C1 unique | C2 unique | C3 unique | C4 unique | C5 unique |
|---|---|---|---|---|---|---|
| leveldb | 1525 | 1326 | 222 | 9 | 56 | 665 |
| re2 | 1595 | 1142 | 175 | 6 | 671 | 994 |
| snappy | 276 | 99 | 82 | 59 | 2 | 179 |
| protobuf-x | 14744 | 10483 | 2507 | 147 | 7214 | 2126 |

Synergy is large: the sum of singles is far below the all-together number (official 1,608 vs 3,396; suppl. 9,687 vs 18,140) because a variable usually carries several escape kinds at once, and because an `assign_rhs` escape of `y = x` fires only while `y` itself is escaped (cascade): freeing `y` frees `x` for free. A candidate's single yield therefore understates its value inside a slice that takes others, and its unique yield overstates what it can deliver on its own.

### Concentration (read the edge counts with this)

Edges are def-use pairs, so one large owner dominates:
* leveldb C1: 616 of 668 edges are the single variable `status` in `benchmarks/db_bench_sqlite3.cc`.
* protobuf-x: 3,964 of the 4,407 C1 edges, 583 of the 640 C2 edges and 5,675 of the 14,744 `all` edges are in generated `*.pb.cc/.h`, almost all the `target` / `cached_has_bits` chain of the `_InternalSerialize`/`ByteSizeLong` bodies. 2,409 of the 2,697 protobuf-x C4 edges are in `message_differencer_unittest.cc` (the variable `item` alone is 1,841). The leveldb/re2 C4 edges are real in-repo class pointers (`Iterator* iter` in `table/table.cc`, `util/cache.cc`, ...), i.e. the census-provable kind.
* The distinct-variable column is the less skewed measure: 76 variables C1 (official), 202 suppl.

## 3. C1: callee classification of ALL extra-removed C1 edges

Mechanical classification, `examples/defs6d.rs` indexes every function declarator of the corpus (name, parameter kinds `val|ptr|ref|rref|fnptr`, template flag, method flag) and each logged `call_arg` event is classified by callee name and argument position. Categories: **A** one signature in-repo, parameter at that position is by value / `T*` (one case by-value class type); **B** in-repo by reference; **C** in-repo overloaded or template (split: `all-byval` = every overload/template has a by-value/`T*` parameter at that position, or not); **D** external/unresolved (no in-repo declarator by that name: stdlib, libc, test macros, `reinterpret_cast<..>`, `init_declarator`); **E** method call (`obj.m()`/`p->m()`, receiver type unknown to the classifier); **F** arity/variadic beyond the declared parameters. The index is name-based, not scope-resolved: same-named unrelated functions make overload sets look worse than they are (C is pessimistic), and A can be wrong when an unrelated function of the same name has the same signature (disclosed).

Distinct blocking call-argument events behind the extra edges (suppl. set, de-duplicated by owner, variable, line, callee, argument position):

| A | C all-byval | C other | E method | D external | F arity | B by-ref | total |
|---|---|---|---|---|---|---|---|
| 236 | 107 | 36 | 238 | 85 | 5 | 1 | 708 |

Edges (suppl., 5,535 C1 edges): every blocking call event A: 919 (17%); A or all-byval overload sets: 936 (17%); the rest have at least one E/D/F/B/other-C event or only constructor/brace/cascade events (13 edges). Excluding generated `*.pb.*` code: 903 of 1,571 edges are all-A, but 616 of them are the single leveldb `status` variable.

Per variable (the less skewed view):

| set | distinct vars | all A | A + C all-byval | needs receiver typing (E) + A/C | has D / B / F / ctor / reference-overload | cascade only |
|---|---|---|---|---|---|---|
| leveldb+re2+snappy | 76 | 18 | 5 | 21 | 30 | 2 |
| + protobuf-x | 202 | 31 | 6 | 80 | 82 | 3 |
| non-generated only | 138 | 27 | 5 | 37 | 66 | 3 |

So a callee-parameter proof restricted to "in-repo single definition, by value or `T*`" certifies roughly 15-24% of the freed variables (and 17% of edges); allowing closed overload sets adds 3-7%; allowing method calls with receiver typing (parameter or constructor-local receivers, as 6b's P6-lite does for resolution) could add up to another 27-40% of the variables depending on the set (generated protobuf code is mostly `stream->EnsureSpace(target)` method calls on a typed parameter `EpsCopyOutputStream* stream`); external (libc/stdlib/test macros) needs a signature table.

## 4. C5: operand certificate breakdown (667 edges, suppl.; official 339)

By operator family of the operator that blocked (distinct events): cmp 108, other 80, arith 44, shift 5; edge level: pure families other 263, cmp 215, cmp+shift 48 (a `for (...; i < n; ...)` head with a `<<` elsewhere), arith+other 66, arith+cmp 35, the rest smaller. By why the OTHER operand failed (edges having that reason; not exclusive):

| reason | edges |
|---|---|
| global/unknown name (not an owner local: constant, enumerator, member, macro) | 277 |
| other operand's name escaped by another candidate (cascade) | 229 |
| call (`x + f()`) | 175 |
| local with non-keyword type (`size_t`/class; C2 interplay) | 75 |
| member access | 10 |
| assignment/update/compound-literal/too-large operand | 18 |
| string literal, qualified global, dereference/address-of (`x + *p`), non-local subscript | 7 |

Pure single reason edges: global/unknown 215, cascade 198, call 76, local non-keyword 41; the remaining 137 are mixed or a rare single reason (member 7, qualified 2, string 2, deref 2). Detail with counts per (family, reason) in `tables/c5class.json`.

## 5. Certifiability samples (68 sampled edges; `samples.jsonl`)

15 edges per candidate from the sorted extra set (seed 6302, `random.Random(6302).sample`), pooled over leveldb, re2, snappy and (supplementary, because official protobuf yields nothing) protobuf-x: 15 C1, 15 C2, **8 C3 (all that exist, pool 8)**, 15 C4, 15 C5. The pools are dominated by protobuf-x (53 of 68 sampled edges): reading them as representative of leveldb/re2/snappy would be wrong.

Method for (a): for each edge I read the def, the use line, the lines between and the loop structure in the corpus source. Every sampled pair is **dead** (use cannot observe the def's value): either the use line is only a declarator/initialiser or plain assignment target of the variable (rule U), or the pair is backward with no loop holding both lines, or an unconditional same-block overwrite lies between (rule D), or (C3#3 only) the loop body redeclares the block-scoped local before the use. Loop extents were judged by indentation/brace matching, not a parser. This does not say that muting is sound: these edges are removed correctly *in these spots*, and sampling by edge draws mostly long straight-line owners. It shows no unsound loss in the sample, and that the soundness question is entirely about the escape, which (b) addresses.

Verdicts (a) and a bounded-local-proof reading (b), all 68 rows:

| candidate | n | really dead | bounded local proof |
|---|---|---|---|
| C1 | 15 | 15 yes | 4 yes, 4 partly (overload sets or typed receiver), 7 no (external callee / mixed) |
| C2 | 15 | 15 yes | 15 yes (census proof that the spelling is declared nowhere in the repo and not shadowed in the owner; std guarantees the typedef is an integer type) |
| C3 | 8 | 8 yes | 8 yes (scalar local cast to a keyword scalar or arithmetic right side whose operators pass the operand certificate: a prvalue cannot bind `int&`) |
| C4 | 15 | 15 yes | 15 partly: the proof (type name is a repo class, so `T * x;` is not a product) fails here because the sampled types are generated protobuf test messages not in the corpus; it works for in-repo classes |
| C5 | 15 | 15 yes | 2 yes (pure cascades), 9 partly (global/unknown or non-keyword-typed other operand needs a census-backed scalar proof), 4 need a callee/type proof (call, member) |

Row table (full detail, events and the C1 callee classes per sample in `samples.jsonl`):

| id | corpus:file:def→use | var | edge | really dead? | bounded local proof? | what would certify it |
|---|---|---|---|---|---|---|
| C1#0 | protobuf-x:descriptor.pb.cc:4285→4381 | `target` | exact/None | yes | partly | method call (needs receiver typing and class method lookup) |
| C1#1 | re2:compile.cc:661→677 | `id` | nameonly/killed | yes | yes | every blocking callee is a single in-repo definition taking the argument by value / T* |
| C1#2 | protobuf-x:cpp_features.pb.cc:307→296 | `target` | nameonly/cfg_incomplete | yes | no | method call (needs receiver typing and class method lookup); overload/template with a reference or short overload |
| C1#3 | protobuf-x:descriptor.pb.cc:10029→10022 | `target` | nameonly/cfg_incomplete | yes | no | method call (needs receiver typing and class method lookup); overload/template with a reference or short overload |
| C1#4 | leveldb:db_bench_sqlite3.cc:561→514 | `status` | nameonly/cfg_incomplete | yes | yes | every blocking callee is a single in-repo definition taking the argument by value / T* |
| C1#5 | leveldb:db_bench_sqlite3.cc:582→535 | `status` | nameonly/cfg_incomplete | yes | yes | every blocking callee is a single in-repo definition taking the argument by value / T* |
| C1#6 | re2:pcre.cc:374→362 | `matches` | exact/None | yes | partly | method call (needs receiver typing and class method lookup) |
| C1#7 | protobuf-x:descriptor.pb.cc:7676→7705 | `target` | nameonly/killed | yes | no | method call (needs receiver typing and class method lookup); overload/template with a reference or short overload |
| C1#8 | protobuf-x:descriptor.pb.cc:5615→5635 | `target` | nameonly/killed | yes | no | method call (needs receiver typing and class method lookup); external/unresolved callee (stdlib/libc/macro, no in-repo signature); overload/templat... |
| C1#9 | protobuf-x:descriptor.pb.cc:3201→3239 | `target` | nameonly/killed | yes | no | method call (needs receiver typing and class method lookup); overload/template with a reference or short overload |
| C1#10 | protobuf-x:descriptor.pb.cc:14063→14035 | `target` | nameonly/cfg_incomplete | yes | partly | method call (needs receiver typing and class method lookup) |
| C1#11 | protobuf-x:type.pb.cc:1133→1179 | `target` | exact/None | yes | no | method call (needs receiver typing and class method lookup); external/unresolved callee (stdlib/libc/macro, no in-repo signature); overload/templat... |
| C1#12 | protobuf-x:descriptor.pb.cc:10830→10837 | `target` | nameonly/killed | yes | no | method call (needs receiver typing and class method lookup); overload/template with a reference or short overload |
| C1#13 | protobuf-x:descriptor.pb.cc:4352→4381 | `target` | exact/None | yes | partly | method call (needs receiver typing and class method lookup) |
| C1#14 | leveldb:db_bench_sqlite3.cc:578→581 | `status` | nameonly/killed | yes | yes | every blocking callee is a single in-repo definition taking the argument by value / T* |
| C2#0 | protobuf-x:type.pb.cc:2220→2231 | `cached_has_bits` | exact/None | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#1 | protobuf-x:descriptor.pb.cc:9608→9616 | `cached_has_bits` | nameonly/killed | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#2 | protobuf-x:descriptor.pb.cc:13706→13692 | `cached_has_bits` | nameonly/cfg_incomplete | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#3 | protobuf-x:descriptor.pb.cc:3130→3127 | `cached_has_bits` | nameonly/cfg_incomplete | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#4 | leveldb:version_set.cc:97→89 | `left` | nameonly/cfg_incomplete | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#5 | leveldb:bloom.cc:49→30 | `bits` | nameonly/cfg_incomplete | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#6 | protobuf-x:descriptor.pb.cc:8422→8500 | `cached_has_bits` | nameonly/killed | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#7 | protobuf-x:descriptor.pb.cc:3698→3711 | `cached_has_bits` | nameonly/killed | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#8 | protobuf-x:descriptor.pb.cc:3261→3353 | `cached_has_bits` | nameonly/killed | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#9 | protobuf-x:descriptor.pb.cc:3127→3140 | `cached_has_bits` | nameonly/killed | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#10 | protobuf-x:descriptor.pb.cc:8987→8975 | `cached_has_bits` | nameonly/cfg_incomplete | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#11 | protobuf-x:descriptor.pb.cc:10819→10816 | `cached_has_bits` | nameonly/cfg_incomplete | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#12 | protobuf-x:descriptor.pb.cc:12686→12718 | `cached_has_bits` | nameonly/killed | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#13 | protobuf-x:parser.cc:732→727 | `digit` | nameonly/alias_unstable | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C2#14 | protobuf-x:descriptor.pb.cc:12001→12015 | `cached_has_bits` | exact/None | yes | yes | type spelling is size_t/[u]intN_t (std-guaranteed integer typedef): needs a census proof that no in-repo class/typedef/using/template-parameter/enu... |
| C3#0 | protobuf-x:tokenizer.cc:1202→1205 | `code` | exact/None | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#1 | protobuf-x:tokenizer.cc:1191→1188 | `code` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#2 | protobuf-x:tokenizer.cc:1195→1188 | `code` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#3 | protobuf-x:tokenizer.cc:1195→1191 | `code` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#4 | protobuf-x:tokenizer.cc:1205→1202 | `code` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#5 | protobuf-x:tokenizer.cc:1209→1202 | `code` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#6 | protobuf-x:tokenizer.cc:1209→1205 | `code` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C3#7 | snappy:snappy_unittest.cc:385→375 | `n` | nameonly/cfg_incomplete | yes | yes | scalar local cast to a keyword scalar (static_cast<char>(code)) or an arithmetic right side whose own operators pass the operand certificate: the r... |
| C4#0 | protobuf-x:message_differencer_unittest.cc:2014→2005 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#1 | protobuf-x:message_differencer_unittest.cc:4234→4243 | `nested` | nameonly/killed | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#2 | protobuf-x:message_differencer_unittest.cc:2448→2444 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#3 | protobuf-x:message_differencer_unittest.cc:2727→2718 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#4 | protobuf-x:message_differencer_unittest.cc:1854→1858 | `item` | exact/None | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#5 | protobuf-x:message_differencer_unittest.cc:4036→4039 | `item` | exact/None | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#6 | protobuf-x:message_differencer_unittest.cc:4152→4169 | `item` | nameonly/killed | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#7 | protobuf-x:message_differencer_unittest.cc:2228→2265 | `item` | nameonly/killed | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#8 | protobuf-x:message_differencer_unittest.cc:2222→2232 | `item` | nameonly/killed | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#9 | protobuf-x:message_differencer_unittest.cc:2209→2232 | `item` | nameonly/killed | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#10 | protobuf-x:message_differencer_unittest.cc:3263→3249 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#11 | protobuf-x:message_differencer_unittest.cc:2222→2208 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#12 | protobuf-x:message_differencer_unittest.cc:2126→2143 | `item` | nameonly/killed | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#13 | protobuf-x:message_differencer_unittest.cc:2132→2128 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C4#14 | protobuf-x:message_differencer_unittest.cc:1805→1803 | `item` | nameonly/cfg_incomplete | yes | partly | pointer local `T* x` whose type name T is a class/struct declared in the repo (census) and not also an object/enumerator/function spelling, so `T *... |
| C5#0 | protobuf-x:message.cc:172→157 | `bytecode_estimate` | nameonly/cfg_incomplete | yes | partly | the callee must be resolved to a function whose return type is a built-in scalar |
| C5#1 | protobuf-x:descriptor.pb.cc:4366→4315 | `n` | nameonly/cfg_incomplete | yes | yes | resolves by itself once the other operand name stops escaping (cascade of another candidate) |
| C5#2 | protobuf-x:descriptor.pb.cc:4388→4326 | `n` | nameonly/cfg_incomplete | yes | yes | resolves by itself once the other operand name stops escaping (cascade of another candidate) |
| C5#3 | leveldb:db_impl.cc:1351→1334 | `allow_delay` | nameonly/cfg_incomplete | yes | partly | the callee must be resolved to a function whose return type is a built-in scalar |
| C5#4 | leveldb:histogram.cc:223→221 | `r` | nameonly/cfg_incomplete | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#5 | snappy:snappy-test.cc:250→290 | `err` | nameonly/killed | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#6 | re2:prog.cc:323→307 | `flags` | nameonly/cfg_incomplete | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#7 | re2:prog.cc:320→305 | `flags` | nameonly/cfg_incomplete | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#8 | re2:prog.cc:314→301 | `flags` | nameonly/cfg_incomplete | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#9 | protobuf-x:text_format.cc:621→593 | `reserved_field` | nameonly/cfg_incomplete | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#10 | protobuf-x:subprocess.cc:197→161 | `input_pos` | nameonly/cfg_incomplete | yes | partly | the callee must be resolved to a function whose return type is a built-in scalar; the other operand is an owner local whose declared type is not a ... |
| C5#11 | re2:dfa.cc:1686→1663 | `start` | nameonly/cfg_incomplete | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#12 | protobuf-x:message.cc:389→384 | `totalBits` | nameonly/cfg_incomplete | yes | partly | the callee must be resolved to a function whose return type is a built-in scalar |
| C5#13 | protobuf-x:tokenizer.cc:842→858 | `prev_line` | exact/None | yes | partly | the other operand (not an owner local: global, class member, enumerator, macro or constant) needs a census-backed proof that it is a built-in scala... |
| C5#14 | protobuf-x:message_differencer.cc:1631→1668 | `match` | exact/None | yes | partly | the other operand is an owner local whose declared type is not a keyword scalar (size_t/uintN_t spelling or class): C2-style typing needed |

Notes on individual samples: C2#5 (`bits`) and C2#13 (`digit`) have a def on a line that does not write the variable (an alias-twin/anchor artifact: `h % bits`, `val *= 10`); the pair is dead anyway because its use is a write-only declarator.

## 6. P1: the `.c` phantom `a * x;` boundary on redis 7.2.4 `src/`

Logged in `c_phantom::escape_phantom` (the `.c` path only): a declaration whose type is a bare `type_identifier`, one pointer declarator, no initialiser, qualifier or storage class, whose type name is NOT spelled as an object in the same file (the case the current code does not escape). Only owners that reach the walk are counted (not off by G1/G2/census/parse error): **756** such statements, 732 with a single `*`, 70 distinct first identifiers.

Header lookup (python over every `.h` of `src/`, regex for `extern` and column-0 object definitions, enumerators, object-like `#define`s, typedef/tag names): first identifier is a type in a header 672 (+3 whose type name is also an object-like macro: `RedisModuleString` -> `robj`), declared nowhere in the headers of `src/` 81 (libc/hiredis/`.c`-local types: `FILE`, `redisReply`, `sentinelRedisInstance`, ...). **Firing rate (first identifier is an object, enumerator or object-like macro of an in-repo header, and not a type): 0 of 756.** Extending the lookup to every header of the whole redis tree (`deps/` included) finds two macro hits and no object (`sds` is also `#define sds hisds` in `deps/hiredis/sdscompat.h`, and `sds` is a typedef in `sds.h`).

Five example sites (all are real declarations; none is a phantom): `acl.c:1327 listNode *ln`, `connection.c:160 ConnectionType *ct`, `networking.c:1775 listNode *ln`, `replication.c:2087 redisDb *dbarray`, `server.c:7169 sds *argv_tmp`. Full counts in `tables/p1.json`. Statements whose first identifier IS spelled as an object in the same file are already escaped and are not counted here (no denominator for those was measured).

## 7. Readings taken conservatively, limits, and what I could not do

* Candidate yields are edges, which overweights big owners (§2 concentration); distinct variables are reported next to them. I did not weight by function count.
* Protobuf: official yield is 0 under every switch because of the census flag; every number that includes protobuf-x is supplementary and labelled so. The tables show both sums.
* Attribution of an edge to escape kinds is by variable name within the innermost owner containing both lines (the escape is owner-wide by name anyway). 0 of the extra edges were unattributed. A variable freed by a cascade is attributed to the call events of the variable it cascaded from only through its own events; the transitive root cause is not chased (3 variables "cascade only").
* C1 classification is name based over a function-declarator index (not scope or type resolved); methods are classed E without receiver typing; macros such as `ASSERT_GE` are D. A is a lower-trust bound than it looks (same-name unrelated functions can mask a reference parameter), C and D are pessimistic.
* C2 as implemented admits the spellings only in the E2 gate; admitting them as cast targets or array element types was not measured. The census-based condition ("no in-repo declaration of that spelling") was not implemented in the switch: the measured yield is an upper bound if a corpus declares one of those spellings (I did not check for in-repo declarations, a reading for the proof's cost).
* C3a was defined as "right side is a binary/unary expression", not "certified by the operand certificate"; the operators inside the right side are still certified, so it measures exactly the relaxation named in the task. `reinterpret_cast`/`const_cast` are not counted as scalar casts in C3.
* "Really dead" was read in source with indentation-based loop extents; 68 samples are not a proof of soundness.
* P1 header lookup is regex based (not parsed C), classes `obj/enum/macro` are first-appearance hits, and only owners that reach the walk are counted.
* Not done: timing (not needed); no tier-A harness run (no product change); mutants/tests not touched; the `.h` and `nullptr` items of #820 not measured (outside the brief).
* Sample selection note: with the pool dominated by generated protobuf code, per-candidate verdicts about "bounded proof" for C1 reflect `stream->EnsureSpace(target)`-style method calls more than leveldb/re2 free-function calls.

Raw tables: `tables/table_per_corpus.tsv` (all corpora, all switches, plus sums), `tables/counts4.tsv`, `tables/counts_px.tsv`, `tables/c1class.json`, `tables/c1_var_class.json`, `tables/c5class.json`, `tables/p1.json`. Tools: `tools/*.py`. Raw edge dumps are not committed.

Progress: DONE

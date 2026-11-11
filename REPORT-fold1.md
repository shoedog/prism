# Fold 1 report: PR #811 (#630 slice 6b), after review round 1

Branch `perf/630-s6b-c-rules`; code heads `9beb115` (fold) and `29b0950` (trace field only). Plain pushes, no force-push, no PR, no GitHub comment. Brief sha256 matched (72047a4c…f4bb).

## Step 0
Merged `origin/perf/630-s6a-c-macro-census` e06c71f5 (merge commit `0ac5709`). One conflict, in CLAUDE.md (census paragraph): took 6a's new census paragraph (with "changes no output yet" replaced by "slice 6b consumes it") and kept 6b's C-rules paragraph. Cache stays v175. 6b still consumes `incomplete` and the census through the merged API; `c_prune_reads_the_repo_wide_census` passes.

## IMPORTANT FINDING (not mine to fix: 6a/#810): census fold 3 turns every C owner off on redis 7.2.4 `src/`
`incomplete` is set by "any directive line holding a non-ASCII byte". Redis has three: `listpack.c:1690`, `quicklist.c:1756-1757` (`#define TEST(name) printf("test — %s\n", name);`, an em dash inside a string). So on the unmodified redis `src/` the head prunes NOTHING (ledger: 4072 owners `off:Incomplete`, 0 removed, 0 added, byte-identical to base). Taken conservatively: I did not change the census. To get meaningful numbers I also measured `redis-ascii`, a copy of `src/` with those 3 lines' em dash replaced by `-` (the only change). Recommend #810 narrow the belt (a non-ASCII byte inside a string or comment of a directive is not an identifier hazard) before 6b can deliver on redis-like trees.

## Items, commits, RED rows
All in `9beb115` unless noted. RED = the row fails on cf8810f7 (verified by restoring cf8810f7's `c_prune.rs` and running `c_prune_matrix`: exactly these 10 rows fail; with the fix all pass).

| item | change | RED rows (fail on cf8810f7) |
|---|---|---|
| 1 declarations under wrappers | any `declaration` whose parent is not a block, a direct `case`, a `for` header (so under `#if`/`#ifdef`/`#else`/`#elif`, a label, a nested `case`) adds its declarator names to the owner-wide `escaped` set | h1 (sol's conditional `extern`, U), h2 (Opus's `#ifdef USE_GLOBAL extern`, D), h3 (`static` + recursion) |
| 2 macro spellings as keywords/types | `spelled_identifiers` (macro_census.rs: translate, splice, comment-strip, census tokenizer, both dialects) over the owner text; every census fact applied (free names, params, macro-name refusal, `poisoned`->G5, `control`->no D); a literal/comment that cannot be read fails the owner closed (G2) | k1 (`#define const x1 = saved1;`), k2 (`#define short ...`) |
| 3 OpenMP/OpenACC gate | directive lines read from `normalized_texts` (continuations spliced, comments stripped, both dialects) | o1 (`#pragma /*gap*/ omp`), o2 (continued `#pragma \` + `omp`), o3 (`#pragma /* gap */ acc`) |
| 4 macro callee forms | argument escape applies when the callee's last identifier (`f`, `a->f`, `a.f`, `(f)`) is an in-repo macro or a `va_*` builtin | a1 (`it->stepper(x)`), a2 (`(stepper)(x)`) |
| 5 smaller | `G3Ms` split from `G3` (`ms_*` nodes) and documented in CLAUDE.md; keep row c1 (`int y = (q = p);`, later overwrite and use of `p`); bare-aggregate row t07 (struct copied whole and restored; keep own use, drop restored use); `va_keep` pinned on `ap` (g8: `ap = ap0; ap = ap1; va_end(ap)` kept by the va escape); Z05 cap test | c1, t07, g8 pass on both revisions (they pin guards; the certificate-left mutant was not re-run, so c1's discriminating power is unproven) |
| 6 survivor cap | no code change; the #634 sentence is in the CLAUDE.md slice-6 paragraph; Z05 `c_prune_formerly_capped_owner_regains_survivors_per_634` pins it (a 2,050-assignment `.c` owner regains edges; `.cc` and `.h` stay capped) | n/a |
| 7 performance | not done (optional); timing below | n/a |
| trace | `hidden_decls=` field in `PRISM_C_PRUNE_TRACE` (`29b0950`) | n/a |

Also: CLAUDE.md E1/E3/G2/G3 text and Z04/Z05 disclosure updated. Fixtures: `tests/fixtures/dfg_prune/c/06_fold1.c` (no `#define`), inline `12_fold1_macros.c`; obligations.jsonl 76 -> 92 rows (generated). `cargo fmt --all --check` clean.

## Tests
`cargo test -p prism --lib -- c_prune c_macro macro_census lexical_prune dfg_codec cpg_cache`: 133 passed, 1 failed (`cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`, a root-user environment failure, same as the round-0 report; it fails on base too). The full `cargo test` suite, `--test ast` and `--test lang_c` were NOT re-run after the fold (only the filtered lib run above), and no mutgate lanes were run.

## Measurement
Release builds of base e06c71f5, cf8810f7 and head; `nav --no-cache dfg-stats --edges`, `RAYON_NUM_THREADS=4`; unique edges keyed by (from, to, labels). Corpora at the pins: redis 7.2.4 `d2c8a4b9…`, cpython v3.12.7 `0b05ead8…` (`Objects/ Python/ Include/`). Nothing built or executed.

| corpus | base | cf8810f7 | head | head vs base: removed / added / label-changed | head vs cf8810f7: restored / removed / label-changed |
|---|---|---|---|---|---|
| redis `src/` as is | 290559 | 255455 | 290559 | 0 / 0 / 0 (census `incomplete`, see above) | 35104 restored (cf8810f7 pruned; head is off) |
| redis-ascii | 290559 | 255455 | 255846 | 34713 / 0 / 0 | 391 / 0 / 0 |
| cpython | 2405403 | 2395777 | 2395791 | 9612 / 0 / 0 | 14 / 0 / 0 |

Counters (expanded): redis-ascii `dfg_pruned_write_only` 0 -> 22963, `dfg_pruned_dominated` 0 -> 14469 (cf8810f7: 23218 / 14632). cpython 9 -> 6750 and 0 -> 3351.
Restored edges by item (head vs cf8810f7, from the per-owner trace): redis-ascii 377 edges are in 66 owners that cf8810f7 pruned and head now turns off with G5 (item 2: the poisoned-macro refusal now applies to macro names found as preprocessing tokens); 14 edges are in owners that stay on (items 1/2/4 growing the escaped set). cpython: 14, all in owners that stay on; its owner ledger is unchanged. I did not separate items 1, 2 and 4 within those 14 (no per-name instrumentation). Items 3 and 5 restore nothing measurable here.
Non-C controls (`tests/fixtures/dfg_prune/{go,rust,python}`): `dfg-stats --edges` and plain `dfg-stats` md5-identical between base and head.

### Ledger (owners; each once; head `29b0950`)
| verdict | redis-ascii | cpython | redis as is |
|---|---|---|---|
| owners | 4176 | 6590 | 4176 |
| on (U and D) | 3986 | 2653 | 0 |
| on, D off (G6) | 5 | 24 | 0 |
| off Incomplete | 0 | 0 | 4072 |
| off G1 file (`.h` as C) | 57 | 985 | 57 |
| off G1 parse error | 47 | 175 | 47 |
| off G2 | 10 | 67 | 0 |
| off G3 asm/SEH | 2 | 2 | 0 |
| off G3Ms (`ms_*` nodes) | 0 | 0 | 0 |
| off G4 | 0 | 0 | 0 |
| off G5 (poisoned macro token) | 69 (was 3 at cf8810f7) | 2684 | 0 |
| on owners with a declaration under a wrapper (item 1; its names are refused, the owner stays on) | 29 | 44 | n/a |

(The earlier report's "2 owners ms_" was really asm/SEH: G3Ms is 0 on both corpora.)

### Samples (`samples-fold1.jsonl`, seed 811)
30 U + 30 D on redis-ascii and 15 U + 15 D on cpython drawn from the head-vs-base removed edges (rule inferred from whether the use line is a pure write; one redis sample, R13, was inferred wrongly and is a D drop); plus 20 retained-opportunity samples on redis-ascii (10 in owners that are off, 10 in owners that are on), each with def/use text and owner verdict. The strata of §5.5 are not separated beyond U/D, and the per-name E1/E2/E4 refusal cause of retained pairs in `on` owners is not attributed (trace is owner-level). I read the 60 redis and 30 cpython drops against the source: all 90 are correct (a pure write at the use, a dominating write in the same region with no label/case in the window, or a def after the use with no enclosing loop; checked in context for the D ones including R13, R42, R50, C21, C22, C27). Retained samples are recorded, not judged. Many sampled "defs" are the flat fallback's backward or initializer pairs; the drops are right.

### Timing (redis-ascii, 3+3 interleaved, release, `--no-cache`, 4 threads; `timing.txt`)
base 10.312, 10.257, 10.862 s (mean 10.477); head 10.690, 10.458, 11.541 s (mean 10.896). About +4.0% (+0.42 s); six runs, noisy. Memoization (item 7) not done.

## Conservative readings taken
- Census `incomplete` on redis left as is (finding above).
- Any `declaration` not under a block/direct case/for header escapes its names (over-refuses vs. descending into wrappers).
- Spelled-token pass applies G3/G4/`_Pragma` spellings as well as census facts; tokens in strings/comments are ignored; an unreadable literal fails the owner closed.
- `(M)(x)` is not a macro invocation in C; escaped anyway.
- Measured on redis-ascii in a /tmp copy; the repo was not touched.

Progress: DONE

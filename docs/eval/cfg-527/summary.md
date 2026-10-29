# #527 cfg census — summary

Unit: one (call site, Exact target) pair, target file `.rs`, non-fixture. Class cells are `not implied / implied`. **A** = (a) ∧ ¬implied.

## Table A — per corpus and pooled (7 real corpora)

| corpus | E | G | A | (a1) ¬impl / impl | (a2) | (b-std) | (b-other) | (c) | stat-false | map rate |
|---|---:|---:|---:|---|---|---|---|---:|---:|---:|
| prism | 34136 | 10307 | **0** | 0 / 4 | 0 / 0 | 1 / 119 | 349 / 9834 | 2 | 0 | 1.0000 |
| ruff | 56702 | 6267 | **2** | 0 / 54 | 2 / 97 | 4 / 50 | 65 / 5995 | 0 | 0 | 0.9999 |
| ripgrep | 4793 | 1431 | **0** | 0 / 6 | 0 / 10 | 0 / 1 | 4 / 1410 | 0 | 0 | 1.0000 |
| uv | 16140 | 1831 | **4** | 4 / 18 | 0 / 0 | 6 / 3 | 150 / 1650 | 0 | 0 | 0.9996 |
| tokio | 5910 | 5018 | **114** | 94 / 29 | 20 / 57 | 94 / 58 | 1753 / 2913 | 0 | 0 | 0.9986 |
| clap | 6002 | 548 | **0** | 0 / 0 | 0 / 0 | 1 / 2 | 19 / 526 | 0 | 0 | 1.0000 |
| axum | 2385 | 983 | **0** | 0 / 0 | 0 / 0 | 6 / 35 | 29 / 913 | 0 | 0 | 1.0000 |
| **pooled** | 126068 | 26385 | **120** | 98 / 111 | 22 / 164 | 112 / 268 | 2369 / 23241 | 2 | 0 | — |
| prism-quick (separate) | 11103 | 594 | **0** | 0 / 0 | 0 / 0 | 0 / 0 | 1 / 593 | 1 | 0 | 1.0000 |

## Table B — per rung, pooled

| corpus | E | G | A | (a1) ¬impl / impl | (a2) | (b-std) | (b-other) | (c) | stat-false | map rate |
|---|---:|---:|---:|---|---|---|---|---:|---:|---:|
| constructor_local | 3438 | 793 | **2** | 0 / 10 | 2 / 73 | 0 / 49 | 12 / 647 | 0 | 0 | — |
| field_typed | 4178 | 130 | **0** | 0 / 0 | 0 / 2 | 0 / 1 | 3 / 124 | 0 | 0 | — |
| free_single | 8994 | 2771 | **0** | 0 / 0 | 0 / 0 | 3 / 0 | 304 / 2464 | 0 | 0 | — |
| import_qualified | 5 | 0 | **0** | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 | 0 | — |
| local_def | 29446 | 12104 | **0** | 0 / 3 | 0 / 0 | 0 / 108 | 38 / 11955 | 0 | 0 | — |
| qualified_owner | 38473 | 6253 | **114** | 97 / 86 | 17 / 26 | 107 / 44 | 1906 / 3970 | 1 | 0 | — |
| qualifier_owner | 20 | 4 | **0** | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 4 | 0 | 0 | — |
| return_typed | 10224 | 2486 | **0** | 0 / 11 | 0 / 32 | 0 / 23 | 25 / 2395 | 0 | 0 | — |
| self_receiver | 15385 | 1301 | **0** | 0 / 0 | 0 / 28 | 1 / 37 | 25 / 1210 | 0 | 0 | — |
| stem_single | 1008 | 167 | **1** | 1 / 0 | 0 / 0 | 0 / 0 | 27 / 139 | 1 | 0 | — |
| typed_param | 14897 | 376 | **3** | 0 / 1 | 3 / 3 | 1 / 6 | 29 / 333 | 0 | 0 | — |

## Table C — fence cost (Exact pairs demoted), share of E

| corpus | E | F1 strict | F2 alt-aware | F2 + (c) | F3 (false + unknown) |
|---|---:|---|---|---|---|
| prism | 34136 | 350 (1.03%) | 0 (0.00%) | 2 (0.01%) | 10305 (30.19%) = 10305 + 0 |
| ruff | 56702 | 71 (0.13%) | 2 (0.00%) | 2 (0.00%) | 6230 (10.99%) = 6230 + 0 |
| ripgrep | 4793 | 4 (0.08%) | 0 (0.00%) | 0 (0.00%) | 1388 (28.96%) = 1388 + 0 |
| uv | 16140 | 160 (0.99%) | 4 (0.02%) | 4 (0.02%) | 1355 (8.40%) = 1355 + 0 |
| tokio | 5910 | 1961 (33.18%) | 114 (1.93%) | 114 (1.93%) | 4841 (81.91%) = 4841 + 0 |
| clap | 6002 | 20 (0.33%) | 0 (0.00%) | 0 (0.00%) | 182 (3.03%) = 182 + 0 |
| axum | 2385 | 35 (1.47%) | 0 (0.00%) | 0 (0.00%) | 892 (37.40%) = 892 + 0 |
| **pooled** | 126068 | 2601 (2.06%) | 120 (0.10%) | 122 (0.10%) | 25193 (19.98%) = 25193 + 0 |
| prism-quick (separate) | 11103 | 1 (0.01%) | 0 (0.00%) | 1 (0.01%) | 594 (5.35%) = 594 + 0 |

## T origin histogram (pairs with that origin among T's conjuncts, pooled)

| origin | pairs |
|---|---:|
| file_inner | 2064 |
| impl | 103 |
| item | 1168 |
| macro_cfg | 3885 |
| mod_decl | 13208 |
| mod_inline | 8752 |

## Atom-family histogram (pairs whose T mentions the family, pooled)

| family | pairs |
|---|---:|
| debug_assertions | 18 |
| feature | 13342 |
| other | 2181 |
| target | 5115 |
| test | 17431 |

## Fixture pairs (excluded above)

Pooled fixture-path pairs: E=1212, G=74, A=0.

## Coverage

| corpus | files parsed | packages | parse-error files | parent_unknown files (pairs) | cfg_if! files | attr parse errors |
|---|---:|---:|---:|---|---:|---:|
| prism | 1026 | 17 | 0 | 103 (1244) | 0 | 0 |
| ruff | 1874 | 51 | 2 | 1 (0) | 0 | 0 |
| ripgrep | 100 | 11 | 0 | 0 (0) | 0 | 0 |
| uv | 615 | 72 | 0 | 0 (0) | 0 | 0 |
| tokio | 776 | 12 | 0 | 9 (112) | 0 | 3 |
| clap | 324 | 8 | 13 | 24 (0) | 0 | 0 |
| axum | 223 | 59 | 0 | 44 (0) | 0 | 0 |
| prism-quick | 337 | 1 | 0 | 33 (9) | 0 | 0 |

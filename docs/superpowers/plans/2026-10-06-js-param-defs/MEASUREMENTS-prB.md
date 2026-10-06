# PR-B measurements (callback identity)

`[MEASURED]` 2026-10-06, static analysis only; no corpus package executed. Base main `da0604b3` (`prism-base-da0604b3`, sha256 `294b18d0…548f`). Head = final planner prototype (source manifest `prB/final-source-manifest.txt`), measured binary `prism-head-b10` (`90e17676…1409`); byte dumpers built from this packet's `probes/byte_dump.rs` against each tree (`prism-base-da0604b3-bytes` `184812b3…6b0b`, `prism-head-b10-bytes` `fc07124d…a81da`). Rows marked *(b6)* come from the earlier head `b6` (`f74a293f…`), which lacks three synthetic-only refinements added after the SecBench audit (B-D11 own-alias derivation, `this.x` receiver fence, for-in `var` function-scope binder); they are kept only where b10 was not re-run and say so. Evidence root `~/prism-evidence/js-param-defs/prB/` (`public-b10/`, `sb-rows-b10/`, `sb-callback-*`, `joint*/`, `sb-eligible-*`, `nav-*`, `tier-a-quick/`, `perf/`, `gates/`, `superseded/`).

## 0. Census (Gap 1, main with PR-A)
See `CENSUS.md` §"PR-B Gap-1 refresh". X 4,497 / T 5,292 / SB 68,828 anonymous callables; call-argument parents 3,910 / 5,094 / 58,263; JSX 406 / 0 / 45. Nesting: top level 281 / 343 / 6,974, only-anonymous ancestors 2,223 / 2,246 / 31,470, inside a named callable 1,993 / 2,703 / 30,384. Projected new parameter Defs (bare-used anonymous formals) 1,139 / 2,139 / 33,936.

## 1. Byte rows (owner + byte identity; `rowdiff.py` with RE-OWNED pairing; `adjudicate.cjs` final)
| Corpus | base rows | head rows | ADDED | CORRECT | WRONG | UNDECIDED | LOST | LOST correct / WRONG / undecided | RELABELLED | RE-OWNED |
|---|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|
| X | 63,523 | 76,249 | 15,415 | 15,415 | 0 | 0 | 2,689 | 0 / 2,689 / 0 | 0 | 0 |
| Xi (installed) | 63,523 | 76,249 | 15,415 | 15,415 | 0 | 0 | 2,689 | 0 / 2,689 / 0 | 0 | 0 |
| T | 714,977 | 365,428 | 19,305 | 19,305 | 0 | 0 | 368,854 | 0 / 368,854 / 0 | 0 | 0 |
| prism corpus JS/TS fixtures | 54,150 | 54,165 | 15 | 15 | 0 | 0 | 0 | — | 0 | 0 |
| SecBench, 574/583 packages | 2,128,722 | 1,790,382 | 255,886 | 255,874 | **12** | 0 | 594,226 | 0 / 594,225 / 1→WRONG (PB14) | 2 | 0 |

- **ADDED** are all synthetic-owner rows (`owner|synthetic`): X 2,473 parameter + 12,942 local/member/alias; T 3,269 + 16,036. Labels X: Exact 10,677, CfgIncomplete 3,881, Killed 802, SameLine 23, AliasUnstable 32; T: 14,105 / 4,798 / 356 / 10 / 36. `USE_NOT_READ` (E4) 0 on X and T. X ≡ Xi (identical changed-row files).
- **Step 2 (parameter rows):** X EXACT_OK 1,266 / EXACT_PRIOR_WRITE 83; T 1,640 / 8. Prior-write Exact is PR-A D13 / E6 semantics ("reaches on an unflagged route"); `synthetic_rows_and_labels_equal_the_named_control` shows synthetic passes inherit legacy labels unchanged.
- **LOST** rows are all removed by the E3 fence and are all checker-WRONG: X symbol mismatch 2,444, flow-insensitive alias twin 200, declaration-span mismatch 43, E7 2; T 337,947 / 29,890 (+103 alias twins whose binding is not visible at the Def) / 911 / 3. LOST Exact rows: X 269, T 215. T's base had 350k+ false rows from big TS compiler functions whose nested arrows re-bind `node`, `t`, etc.
- **RE-OWNED:** 0 everywhere, by construction (B-D2). The pairing rule was positively controlled (PROBE-LOG PB4).
- **E9 duplicates** (ADDED rows whose wire identity already exists in base, i.e. a callback nested in a named function owned twice): X 2,551 (16.5 % of ADDED), T 3,960 (20.5 %).
- **Wire rows** (`nav dfg-stats --edges`, CLI output) project exactly from the byte rows on base and head (projection check true for every public corpus).

## 2. SecBench
### 2a. All-package byte rows (`measure_sb_b.py` + `adjudicate_sb_b.py`, b10, 300 s per producer)
- 574/583 packages admitted; the same 9 as PR-A are excluded on both sides because a **base** producer timed out (`total.js_3.4.6` ×2, `atropa-ide`, `cejs`, `clean-css`, `natural`, `react-native`, `three`, `vant`; head finished 5 of them). Call-site dumps identical for all 574.
- ADDED 255,886: 255,874 CORRECT, **12 WRONG** — E11 residue on minified lines where two same-name declarators share a line (`lodash` ×4 versions, `phpjs`): the synthetic alias twin is keyed by `(name, line)` and lands on the non-alias declarator. Bounded fix (key by lvalue start byte) recorded for review, not applied (PB13). Step 2: EXACT_OK 21,864, EXACT_PRIOR_WRITE 547 (E6 parity), UNREACHABLE 1 (`ts-process-promises` `_super.call` after `return`, E8 parity).
- LOST 594,226: 594,225 checker-WRONG (symbol mismatch 556,777; flow-insensitive alias twin 34,649; declaration span 2,510; alias not visible 175; E7 114) + 1 UNDECIDED shown WRONG by mechanism (PB14). **LOST correct: 0.**
- RELABELLED 2, both NameOnly → NameOnly (`CfgIncomplete` → `SameLine`) on minified one-line files (`modjs`, `mithril`); 1 CORRECT, 1 collapsed-mixed. No NameOnly → Exact relabel anywhere. Mechanism not traced (legacy rows on lines whose other edges were fenced).
- Adjudicator corrections found on SecBench (oracle artefacts, not product changes; PB13): CommonJS `exports`/`module` lexical re-resolution, ECMAScript `var`/function-declaration merge, member-path collapse to matching accesses. The b6 run before these corrections reported 1,018 ADDED WRONG.

### 2b. `callback_argument_parameter_registration` (97 rows, unmodified packages; `secbench_callback.py`)
| | harness outcome | callee-tolerant outcome |
|---|---|---|
| base | 96 prism_error, 1 reached_function_only | 96 prism_error, 1 reached_function_only |
| head (b6 and b10 identical) | 96 prism_error, 1 reached_function_only | 91 prism_error, 2 partial, 4 reached_function_only |

All 97 sources are top-level callbacks; 95 payload formals are member-only (`req.url`), so they still have no Def (PR-C). The 2 bare payloads (`bitty_0.1.0`, `shit-server_1.0.0`) now bind but stop at an outgoing call made from the top-level callback, which has no call site (non-goal). Harness `prism_error` is `callees --location <source>` = `LocationOutOfRange` at an anonymous callable on base and head (owner O1). **Standalone traced conversions: 0.**

### 2c. Joint counterfactual (PR-B identity + one bare read per payload, callback kept anonymous)
`sb_joint_checkpoint.cjs` inserts `;void <payload>;` as the first statement of each source callback (R1 `member` checkpoint without R1's naming rewrite; byte coordinates remapped), then the same `measure()`:
| | callee-tolerant traced | harness |
|---|---:|---|
| base | 1 / 97 | 96 prism_error + 1 traced |
| head (b6 and b10 identical) | **90 / 97** (+2 partial, +5 reached_function_only) | 96 prism_error + 1 traced |

Payload-specific BFS (r2-opus-A F5, on callee-tolerant detail): **90 / 90**; 89/90 source Defs are synthetic-owned; 75 distinct source files (R1's own dedup of the joint population was 34 clusters — different key).

### 2d. Eligible sweep regression check (373 GT rows, `secbench_subset.py --select eligible`)
| | traced | payload-specific | partial | reached_function_only | not_reached | prism_error |
|---|---:|---:|---:|---:|---:|---:|
| base | 120 | 114 | 33 | 74 | 7 | 139 |
| head (b6 and b10 identical) | 121 | 115 | 35 | 76 | 7 | 134 |

Transitions: +2 traced (`path-traversal/proxey_0.4.2`, `prototype-pollution/grunt-util-property_0.0.2`, both payload-specific from a synthetic-owned formal), prism_error → partial 2, prism_error → reached_function_only 3, and **−1 traced: `prototype-pollution/dot-prop_2.0.0`**, whose base credit was the checker-WRONG E3 row `set path@24 → forEach-callback path@32` (PROBE-LOG PB12). No base payload-specific credit is lost for any other package.

## 3. Non-JS byte identity (same base/head binaries)
| Corpus | non-JS byte rows base / head | identical | wire identical | call sites identical |
|---|---|---|---|---|
| Python `black` | 20,539 / 20,539 | yes | yes | yes |
| Go `caddy` | 72,681 / 72,681 | yes | yes | yes |
| Rust prism snapshot | 54,129 / 54,129 (non-JS) | yes | yes (non-JS) | yes |

The prism snapshot also contains 14 JS/TS test-fixture files (`tests/fixtures/…`, incl. `sanitizer-suite-js-ts/`): their rows go 21 → 36, i.e. 15 ADDED synthetic rows, all checker-CORRECT (Express/Koa/Fastify handler callbacks), 0 LOST.

## 4. Call sites and navigation
- `call-stats --dump-sites` byte-identical on X, Xi, T, black, caddy, prism and all 574 admitted SecBench packages (§2a).
- `call-stats` summary: only `dfg_labels.*` and `return_flow.return_input_edges` change (X 3,863 → 3,789). All 74 removed ReturnInput edges start at zero-width Use anchors that were endpoints of LOST E3 rows; `return_flow_edges` and every skip counter are unchanged (PROBE-LOG PB11).
- **Nav identity — STOP-1** (`nav_identity.py`, seeded lines opening or inside anonymous callables; `nodes-at`, `ego --location`, `callers --location`, `callees --location`, `repo-map`; per-binary nav caches): *(b6)* X 1/241 queries differ, T 2/81; all are `ego`. The X difference (`ConvertElementTypePopup.tsx:499`) is 7 fewer nodes / 7 fewer edges, every missing node a zero-width Use anchor of a LOST checker-WRONG row (removed by the E3 fence); T's two were not inspected. `callers`, `callees`, `repo-map`, `nodes-at` samples: identical; no `<cb@` anywhere. b10 re-run on X: identical result (1/241, the same `ego` query). See SPEC-prB §8 STOP-1.
- RD counters (X `dfg-stats`): `functions_over_cap` 0 → 0; `functions_without_cfg` 1,751 → 3,708 (+1,957 passes for which RD has no CFG — synthetic passes; not broken down by shape; their rows fall back to `NameOnly(CfgIncomplete)`). T and lodash: §5.

## 5. Performance
`probes/perf_b.py` (`prB/perf/perf.json`): byte dumper = full CPG build; 2 runs per side, base and head alternated; plus one `nav --no-cache dfg-stats` per side for the RD counters. Other sessions kept the host at load ≈ 9 throughout (start 8.97, end 8.46); both sides ran under the same load.

| Corpus | build base (s) | build head (s) | head/base | peak RSS base → head (MB) | RD over-cap base → head | RD without CFG base → head |
|---|---|---|---:|---|---|---|
| X | 42.8 / 43.3 | 31.9 / 32.2 | **0.75** | 770 → 939 (×1.22) | 0 → 0 | 1,751 → 3,708 |
| T | 292.7 / 293.7 | 183.4 / 184.8 | **0.63** | 4,016 → 2,902 (×0.72) | 0 → 0 | 7,398 → 10,081 |
| SecBench `lodash_4.17.10` (UMD; one anonymous wrapper spans the library) | 124.6 / 124.5 | 70.9 / 72.8 | **0.57** | 658 → 329 (×0.50) | 2 → 2 | 766 → 956 |

No regression; the 25 % STOP threshold is not approached. The synthetic passes alone cost +45 % on X before the B-D13 memo (b1, PB5: DFG 19.2 s → 35.8 s); the memo removes per-pass whole-file CFG rebuilds and per-edge rvalue re-queries that base already paid, which is why head is faster than base and why T and lodash also drop RSS. X's RSS rises with its +20 % rows. SecBench all-package byte-dump time (b10, contended, 4 parallel jobs): base 2,109 s → head 1,061 s summed over 574 packages; worst single-package ratio ×1.24 among packages over 5 s (contended timings; see `sb-rows-b10/status.jsonl`).

## 6. Gates
| Gate | Result |
|---|---|
| touched tests | `cpg::callback_identity_tests` 22/22; `js_param_defs_tests` incl. the intended re-pin `curried_arrows_bind_only_the_named_outer_formal` |
| `cargo nextest run --features mcp` (final source) | **5,196 passed, 1 skipped** (reserved `slice_elem_variant_reserved`); doctests 2/2 |
| mutation, advisory scoped (`--since origin/main --scope fn`; untracked files made visible with `git add -N`, then reset) | 27/27 KILLED (PR-B mutants in changed fns + PD-11 + P2-M11) |
| mutation, authoritative PR-B subset (PD-30…PD-54 + PD-11; PD-53 re-run after its last edit) | 26/26 KILLED, all admissible |
| `cargo fmt --check` | clean |
| clippy `--all-targets --features mcp -- -W clippy::all` vs same-environment base worktree (`da0604b3`) | 235 / 235 normalized warning records, 0 added / 0 removed |
| Tier-A `--matrix-only` (fresh release build) | 178 / 178 ok |
| Tier-A quick TS/JS frame (12/stratum, 1,800 s budget, PR-B-private nav cache) | excalidraw-ts **VALID**, secbench-node **VALID**; TP/FP/FN identical to PR-A r2: TS callers raw 65/7/112 exact 58/1/119, callees raw 203/47/14 exact 178/31/39; Node callers 9/0/56, callees 11/2/35; no `<cb@` in any report |

## 7. What is not verified
- F (controller-only). The F row delta (7,216 call-argument + 322 JSX anonymous callables) is the largest unmeasured one.
- Nav byte identity beyond the sampled queries (X 60 lines / T 20 lines); STOP-1 is open.
- Mechanisms of T's two `ego` diffs (b6) and the 2 SecBench NameOnly → NameOnly relabels.
- Complete independent CFG/flow soundness (binding CORRECT is static identity; E4/E6/E8/E9 parity disclosed).
- Python/Go/Rust controls cover three corpora only; other languages are covered only by the language gate in code.
- The 9 SecBench packages whose base producer timed out.
- Full all-lanes mutation run and human-triggered multi-corpus Tier-A.
- The measured binaries were built together with the byte-dumper example (cargo feature unification); a bin-only build is byte-different. Tier-A used the bin-only build of the same source; behavioural equivalence of the two builds is assumed, not tested row-for-row.

## 8. Binary and source binding
`prism-head-b10` was built from the source recorded in `prB/final-source-manifest.txt` (sha256 `00c62b42…38b7`); the tree matches that manifest at handback (re-checked after removing a temporary debug test). An earlier `b6` → final-`b7` check showed a formatting-only change leaves the `prism` binary bit-identical (`f74a293f…`).

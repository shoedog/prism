# PR-A measurements (EVALUATION §3.5 row-correctness method)

`[MEASURED]` 2026-10-05.
- **Base:** `prism-base-006573d9` (sha256 `d2babb6d6e415866c6896fad7cad17f545a246d7939b20444cde32542447b7af`).
- **Head:** `prism-head-prA3` (sha256 `b8838d2ce9c663bc7ed9c80b30c25d7c21f329f840854789ab8b1c009e21e553`), built from the uncommitted prototype on `006573d9`.
- **Evidence:** `~/prism-evidence/js-param-defs/{public-prA3,sb-rows-prA3,secbench}/`.
- **Commands:**
  - `probes/measure-public.sh BASE HEAD OUT` (X, Xi, T and the three controls);
  - `probes/measure-secbench-rows.sh BASE HEAD OUT 4`;
  - `node probes/adjudicate.cjs <TS 5.9.3> <root> <changed.jsonl> <out>`.

**Row classes.** Rows are diffed as a multiset of `dfg-stats --edges` rows, keyed by row identity without labels.
- RELABELLED: the same identity carries a different label.
- RE-OWNED: not observable on DFG rows, which carry no owner field.
- Call rows (`call-stats --dump-sites`) are compared byte for byte, so any RE-OWNED call would show up there. None did.

## 1. Row delta and per-oracle adjudication
| Corpus | ADDED (bare / rest) | of which Exact | NameOnly (cfg_incomplete / killed / sameline) | LOST | RELABELLED | Step 1 checker binding | Step 2 Exact | Call sites |
|---|---|---|---|---|---|---|---|---|
| X excalidraw | 36 (0 / 36) | 25 | 11 / 0 / 0 | 0 | 0 | 36 CORRECT | 25 OK | byte-identical |
| Xi installed | 36 (0 / 36) | 25 | 11 / 0 / 0 | 0 | 0 | 36 CORRECT | 25 OK | byte-identical |
| T TypeScript/src | 1,057 (911 / 146) | 402 | 641 / 9 / 5 | 0 | 4 Exact→NameOnly(sameline) | 1,057 + 4 CORRECT | 402 OK | byte-identical |
| SB 574 of 583 packagesᵃ | 827 (321 / 502 / 4ᵇ) | 405 | 351 / 61 / 10 | 0 | 3 Exact→NameOnly(sameline) | 827 + 3 CORRECT | 403 OK, 2 PRIOR_WRITEᶜ | byte-identical (574) |

ᵃ Nine packages time out (300 s) in `dfg-stats` on **both** binaries and are excluded from both sides: total.js ×2, atropa-ide, cejs, clean-css, natural, react-native, three, vant. react-native's head `call-stats` also timed out; that is the one "differing" call-site package, and the difference is a timeout, not content.

ᵇ The 4 "other" rows are line-1 rows in minified UMD bundles. The checker binding is CORRECT; the shape heuristic cannot see the parentheses.

ᶜ `postcss.js:20→24` and `js-data Mapper.js:1403→1449`. In each, a conditional or nested reassignment precedes the Use on some path. Prism's Exact means "reaches on an unflagged CFG route". The same-shape **plain-parameter control is byte-identical on base** (PROBE-LOG P14): `function pc(plugins){ if(…){ plugins = plugins[0] } return make(plugins) }` has Def@1→Use@5 Exact on base, and the full `Mapper.js` with `...args` replaced by `args` gives identical rows base and head. These are therefore label parity, not a PR-A defect. EVALUATION's strict step-2 rule would call them WRONG; disclosed as E6.

**Adjudication by oracle, in order.**
- **Step 1, TS 5.9.3 checker binding.** 1,920 ADDED def→use rows across X/T/SB: all CORRECT, 0 WRONG, 0 UNDECIDED. The Use binds a formal of the callable that starts on the Def line.
  - There are 7 RELABELLED rows. In each, the next-line Use binds the new arrow formal, while base's Exact came from the same-line declarator that names the arrow (`const file = forEach(xs, file => …)`; E1, PROBE-LOG P7). The relabel is Exact → NameOnly, the safe direction.
  - 0 LOST rows, so there is no regression to adjudicate.
- **Step 2, write-reference check on Exact rows.** 830 OK (X 25, T 402, SB 403) and 2 PRIOR_WRITE (parity, footnote ᶜ).
  - Two oracle false alarms were removed after reading the source: `options = Object.assign({…}, options)`, where the Use is inside the write's own right-hand side.
  - CFG reachability was **not** independently checked.
- **Step 3, SecBench.** See §4.
- **Step 4, Tier-A.** See §5.
- **Step 5, blinded sample.** Not needed: no class was left undecided by steps 1–2.

## 2. What the D11/D12 cut cost (prototype prA2 → prA3, T)
The first T measurement (prA2) had 5 WRONG def→use rows (1 Exact) and 57 arg→formal rows. Of those 57, 26 were Exact and only 1 of the 26 was checker-CORRECT (PROBE-LOG P11). The cut removed 65 rows:
- **8 def→use rows:** 5 WRONG and 3 correct (`documentRegistry.ts 197→198, 197→214`; `customTransforms.ts 146→147`).
- **57 use→def rows:** 1 correct Exact, 3 checker-WRONG NameOnly, 47 callee-unresolved and 6 no-call.
- Of the 26 Exact use→def rows, sampled ones bind global or other-module calls to name-inferred callables (E5).

The SecBench targets are unchanged by the cut.

## 3. Projection check (CENSUS.md vs measured)
| | X | T | SB |
|---|---|---|---|
| projected new Defs / Use lines | 33 / 38 | 846 / 1,015 | 744 / 973 |
| measured ADDED def→use rows | 36 | 1,057 | 827 |

## 4. SecBench conversions (§3.5 step 3, unmodified packages, same harness `measure()`)
**Targets** (`probes/secbench_subset.py --select targets`; R1 inspection `ba2f57c6…`):

| Row | R1 / base | head |
|---|---|---|
| assign-deep, decal 2.1.3, deep-override, mixin-deep 2.0.0, think-helper, viking04-merge (ws2 rest) | reached_function_only ×5, prism_error ×1 (deep-override) | **traced ×6** |
| port-killer (bare arrow) | reached_function_only (`source_binding_no_def`) | **traced** |
| is-svg (bare arrow) | reached_function_only (`source_binding_no_def`) | reached_function_only. The source is now bound; next break is B-plain-argument (residual gap). |
| portprocesses (bare arrow) | prism_error (nested-callback capture) | partial. The source is now bound; next break is D-cjs (residual gap). |

- Rest: **6/6 trace**.
- Bare arrow: **1/3 trace**. All 3 gain their source Def. The other 2 stop at downstream barriers that PR-A does not address. They are recorded as residual, not WRONG.
- The brief's expectation "arrow ≥ 3" is **not met**; r2-opus-A F2 had established only the Def gap for these rows, not the full path.
- Payload-specific BFS (r2-opus-A F5): 7/7 traced credits reach the sink from the payload formal without touching a sibling formal.

**Regression sweep** over every GT-eligible row (`--select eligible`, base then head, sequential, 4 workers): see §4b.

## 4b. Eligible-row regression sweep (373 GT-eligible rows, same harness, base then head)
| Outcome | base | head |
|---|---|---|
| traced | 113 | **120** |
| partial | 32 | 33 |
| reached_function_only | 80 | 74 |
| prism_error | 141 | 139 |
| not_reached | 7 | 7 |

- Base reproduces R1's measured outcomes exactly (113 traced / 141 errors / 80 / 32 / 7).
- **Exactly 8 rows change, all in the improving direction:**
  - reached_function_only → traced: 6;
  - prism_error → traced: 1 (deep-override);
  - prism_error → partial: 1 (portprocesses).
- **0 rows regress.** No row moves from traced to anything else.
- Payload-specific BFS: base 107/113, head 114/120. The +7 new credits are all payload-specific. The 6 non-specific credits are the same rows on both sides (pre-existing; r2-opus-A F5).

Evidence: `~/prism-evidence/js-param-defs/secbench/eligible-{base,head}/`. Base took 962 s and head 974 s.

## 5. Gates (final prototype)
| Gate | Result | Evidence |
|---|---|---|
| Touched tests | `cargo test --lib js_param_defs` 15/15; `required_parameter` 23/23; `nested_execution_owner` 11/11 | this session |
| `cargo nextest run --features mcp` | 5,152 run, **5,152 passed**, 1 skipped (245 s) | `gates/nextest3.log` |
| Mutgate, advisory scoped `--since origin/main --scope fn` | 19/19 KILLED (18 PD + rebound P2-M11) | `gates/mutgate-adv2.log` |
| Mutgate, authoritative lane `--lane mutants/js-param-defs.json` | 18/18 KILLED (68 s) | `gates/mutgate-auth3.log` |
| Mutgate, authoritative full (all lanes) | **139/139 KILLED**, 7:04 | `gates/mutgate-full2.log` |
| `cargo fmt --check` | clean | — |
| clippy, CI form (`--all-targets --features mcp -- -W clippy::all`) | exit 0; no warning in a touched hunk or new file. The `-D warnings` form fails on lib-test warnings that all lie outside this diff; no same-base clippy control was run. | `gates/clippy2.log` |
| Tier-A `--matrix-only --allow-stale-sut` (fresh release build) | 178/178 ok | `tier-a/tiera-matrix.log` |
| Tier-A quick, r2 frame (`--lang ts,js --sample 12`, seed 42) | both VALID; **every count identical to r2**: TS callers raw 65/7/112, exact 58/1/119; callees raw 203/47/14, exact 178/31/39; Node callers 9/0/56; callees 11/2/35 | `tier-a/quick-r2frame-prA3/` |
| Non-JS controls (Python black, Go caddy, Rust prism snapshot `prism-20c8490591a3`) | DFG and call-site rows **byte-identical**: 20,539 / 72,681 / 54,150 DFG rows; 6,637 / 20,705 / 49,302 call sites | `public-prA3/{R_black,G_caddy,RS_prism}.*` |

- The default `tier-a --quick` (3 corpora, prism-quick included) was also VALID, on the intermediate prA2 binary. Its call output is byte-identical to prA3's, because call sites are unchanged on every corpus.
- After the measurements, `cargo fmt` reformatted one `#[cfg(test)]` file. The rebuilt release binary (`prism-head-prA3fmt`, sha256 `cdc22b31…`) differs from prA3 only in the embedded source-tree hash (`PRISM_CACHE_BUILD_IDENTITY`). X rows are byte-identical between the two.

> **Current authority (R3, 2026-10-06): STOP; PR-B PARKED.** The R3 draft loses ES-correct earlier sources when later defaults are supplied. No further implementation, gate, measurement, adoption or shipping authority. Read MEASUREMENTS-prB-R3.md and HANDOFF-repair-prB-r3.md. The R2b source/binary/table claims below are historical and confer no R3 acceptance. Source base f8c768b3; docs base35d780e1.

# PR-B R2b retained repair: complete measurement, Rust quick gate unmet

STOP: **none on the admitted measured population**. This is a self-measured result, not independent review or adoption. The single owner-authorized R2b cap extension continued the retained R2 artifact; no restart or extra source round. Full acceptance is **not all-green**: Rust quick is INVALID due to oracle_timeout, with a known sandbox denial of corpus-target Cargo writes. The proposed retry was not exercised under the supplied command-class denial rule.

Source repair base `cb99663090966c8b128d19649f3b7fc820907f7a`; docs base/HEAD `2cb7102a0b4c186a1a6c09c586dae4620c8e6de8`; product measurement main `da0604b3bcaa983436f58921ba68386ddf48bb8a`. Current evidence: `/Users/wesleyjinks/prism-evidence/js-param-defs/prB/repair-r2b`. Frozen724-file source/test/mutant manifest and bin/sha256.json bind all product measurements.

## Fix and regression evidence

A same-name body `var` under executable parameter expressions receives an internal entry definition sourced from the formal. Its own binding's RD pass preserves reads before assignment and kills copied-formal flow after assignment. An uninitialised `var` is not a kill. Default writes to the formal, nested binding shadows and hoisted same-name functions are excluded. Same-line RHS-before-write admission and the existing inline-if CFG bypass are bounded to the copy seam; bypass retains CfgIncomplete. No global CFG or owner/navigation symbol widening. `DefSite.implicit_entry` and may-reaching RD facts support this internal definition; it is not emitted as a variable. Cache111 and coupled PD-11/P2-M11 guards reject the previous generation.

Synthetic Node7/7 exact assertions first verified `h`, `rec`, `rec2`, overwrite and same-name function behavior. Functions receive the function object instead of the formal copy; [ES2025 FunctionDeclarationInstantiation step28.5.3–5](https://tc39.es/ecma262/2025/multipage/ordinary-and-exotic-objects-behaviours.html#sec-functiondeclarationinstantiation). Four original copy regressions fail unchanged R2 and pass after repair. Final49 callback test functions cover JS/TS/TSX with full source-byte endpoints/labels and negative controls. Six new guard mutants PD-77–82 cover entry admission, kill, default write, function refusal, uninitialised var and inline-if bypass.

An additional same-line synthetic case emits a WRONG formal-value row after overwrite. The same-environment frozen product main and R2 emit the identical full row in all3 languages, so the initial ADDED attribution and STOP were refuted; the defect remains WRONG and unrepaired. All12 diagnostic cases were enumerated before correcting the overbroad line-only test predicate. No inherited WRONG was downgraded. PROBE-LOG.md, edgecase-base.bytes.jsonl, edgecase-main-parity-proof.json and STOP-refuted.json retain the history.

## Four original rows: source-bound ES oracle

**E12 — TS 5.9.3 parameter-environment blind spot; the ES-fixture oracle decides.** Source extra-asciinema index.js SHA256 `eb61cb4be4c8b3ce0a61a4fafc41cccad79a1a8cc4738ce59cc3a623f14e6b87`, owner rec [2215,2539). Primary TS details are preserved; only3 exact source-bound lost rows have E12 sidecars.

| Row | Original endpoints | ES verdict | Frozen main → R2b result |
|---|---|---|---|
|1|formal2228→line69 collapsed2245|CORRECT: RHS reads initial body copy|KEPT1→1; full endpoint/owner/access/path and Exact identity|
|2|formal2228→line71 collapsed2373|WRONG: line69 body assignment kills formal value|LOST1→0|
|3|formal2228→line73 collapsed2463|WRONG: same kill before closure read|LOST1→0|
|4|bodyvar2251→signatureline68 collapsed2215|WRONG: signature f is a binder, not an evaluated read; body binding invisible there|LOST1→0|

Four-row-ES-verdicts.json retains original labels, selected spans, E4 flags, binary/input bindings and exact multiplicities. Focused pair307→317 rows,13 ADDED/3 LOST, identical call sites.

## Full byte tables

Entries are CORRECT/WRONG/UNDECIDED with exact byte/owner/access/label multiset pairing. RE-OWNED pairs differing only in owners; no such current rows.

| Corpus | ADDED C/W/U | LOST C/W/U | RELABELLED C/W/U | RE-OWNED C/W/U |
|---|---:|---:|---:|---:|
| X | 15513/0/0 | 0/3415/2 | 0/0/0 | 0/0/0 |
| Xi | 15513/0/0 | 0/3415/2 | 0/0/0 | 0/0/0 |
| T | 19434/0/0 | 0/374016/0 | 0/0/0 | 0/0/0 |
| SecBench (576 admitted / 583 recorded) | 269476/0/0 | 0/913296/18 | 1/0/1 | 0/0/0 |

X/Xi each63523→75619; T714977→360395; admitted SecBench2707204→2063366. All583 manifest-ok roots have one unique receipt. 7 explicit whole-pair exclusions; 484 admitted nonempty changed packages completely adjudicated. Aggregate class totals equal raw rowdiff totals. Raw TS LOST:3 CORRECT/913293 WRONG/18 UNDECIDED; effective ES moves only the3 exact E12 rows to WRONG. No other LOST CORRECT, ADDED WRONG or RE-OWNED WRONG.

## Identity, navigation and O1/O2

All6 public/control call-site outputs and every admitted SecBench pair are identical; byte-to-wire projection equals CLI rows on both public sides. Non-JS row identity: Black20539, Caddy72681, pinned mixed-Prism54129. Its JS-only deltas are separately checked; whole mixed output need not match.

X nav241 paired queries:207 successful/34 unchanged structured LocationOutOfRange refusals;3 differences, all exact removed-WRONG proofs. T81:61 successful/20 unchanged refusals;2 differences, both removal-proved. No graph additions or non-graph changes. Refusals establish identity only. Same queries and verified main receipts were reused; current head freshly captured.

O1 all97 current receipts per side: standalone main96 errors/1 function-only; head91 errors/4 function-only/2 partial/0 traced. Joint checkpoint counterfactual main96 errors/1 traced; head90 traced/5 function-only/2 partial. Anonymous96 omit callees and named1 invokes it. Joint input SHA256 `1e6090a9923a2ae898dd50fe1ba8ef662078ff17b964f8a825eb2e2de9e9811c`; earlier wrong UTF8-split input runs are inadmissible and receive no population credit. **O2 remains standalone**;90 joint results are potential, not unmodified-package tracing credit.

## Gates and explicit exclusion

- Full nextest with mcp:5223 passed,1 pre-existing skipped,0 failed (5224 selected);214.976s. Skip: SliceElem classifier at tests/integration/resolution_test.rs:576.
- Doctests2/2; callback functions49/49; fmt and git diff checks clean.
- Authoritative isolated guard mutants21 selected/21 admissible/21 KILLED; fn-scoped advisory14/14 against cb996630. No zero-selection or failed-probe credit.
- Clippy main/current235/235 concrete warnings with fresh byte-bound main archive control, same environment.
- Immediate release rebuild then Tier-A matrix178/178. Quick Excalidraw/TS and SecBench/node VALID,0 oracle/SUT errors. Rust quick INVALID: incomingCalls10s oracle timeout at72.161s total; cargo target/debug/.cargo-lock denied. An unchanged-settings retry would repeat the denied command class; not exercised. No timeout relaxation, corpus build-script execution or green credit.
- Quick run validity does not imply zero accuracy discrepancies: TS47 and node24 pending source-bound records are retained in tier-a-quick-pending.json and PR-description-supplement.md. No same-environment main quick control was run; these are not attributed to R2b or rebaselined.
- Python SecBench52 tests plus14 subtests; Node oracle/checkpoint30/30; ES semantics7/7. Invalid discovery/schema/cache/socket probes retained without evidentiary credit.

## Contended performance observation

Direct wait4 byte dumper full CPG build, RAYON_NUM_THREADS=2, overlapping static captures/checkers. These are measured wall/RSS observations, not isolated regression attribution. Current X wall ratio remains below1.25; no build-time STOP observed.

| Corpus | Base → head CPG wall seconds | Base → head peak RSS MiB | Wall ratio |
|---|---:|---:|---:|
| X | 176.397 → 137.775 | 691.7 → 834.3 | 0.781 |
| Xi | 221.560 → 149.731 | 714.3 → 1031.9 | 0.676 |
| R_black | 24.656 → 20.127 | 369.9 → 368.5 | 0.816 |
| G_caddy | 76.750 → 39.196 | 718.3 → 674.3 | 0.511 |
| RS_prism | 318.073 → 86.186 | 972.5 → 948.3 | 0.271 |
| T | 1001.182 → 341.591 | 3812.4 → 2733.8 | 0.341 |

## Recorded SecBench exclusions

Timeout per operation600s, six static workers, RAYON_NUM_THREADS=2. A failed operation excludes both sides. Paired unrun operations receive no credit.

| Package | Failed operation / observation |
|---|---|
| prototype-pollution/total.js_3.4.6 | base.bytes: exit124, 600.125s; both sides excluded |
| command-injection/total.js_3.4.6 | base.bytes: exit124, 600.082s; both sides excluded |
| path-traversal/atropa-ide_0.2.2-2 | base.bytes: exit124, 600.041s; both sides excluded |
| redos/clean-css_4.1.10 | base.bytes: exit-9, 366.967s; both sides excluded |
| redos/natural_5.1.0 | base.bytes: exit124, 601.568s; both sides excluded |
| redos/react-native_0.63.0-rc.0 | base.bytes: exit124, 600.041s; both sides excluded |
| redos/three_0.122.0 | base.bytes: exit124, 600.744s; both sides excluded |

## Binaries and commit-ready custody

- `bin/prism-head-r2b` SHA256 `2ed7fe99c221cb439f945ef23149c3d006f7c0237f6f8d8b21fe762ee6ff0bbf`.
- `bin/prism-head-r2b-bytes` SHA256 `947ed1e53009898adf9c8ff2ea8aae64aae28fdc3e507bfafcd2bac1790ce174`.
- `R2b-src.patch`, cumulative from cb996630: `fix(js): model kill-aware parameter-to-body entry copies`.
- `R2b-docs.patch`, cumulative from2cb7102a: `docs(eval): record PR-B R2b ES oracle and full measurements`.

Full changed-file lists and hashes: patch-manifest.json. Both patch apply checks use pristine Git archives. Final source/docs/binaries and completed measurement evidence are snapshotted with manifests. Controller owns Git writes; no commit, push, merge, adoption or new dispatch.

Not verified: undecided byte-row correctness; valid Rust quick oracle result/build-script facilities; excluded SecBench pairs; F/frontend-portal; unsampled nav; human-triggered full Tier-A multi-corpus run; isolated performance and fresh full-corpus RD-cap counters; runtime execution of corpus packages; general legacy parity defects beyond the seam. No independent review or exhaustive correctness claim. Process census was sandbox-refused; owned tool sessions and receipts establish completion, not a system-wide process census. STOP: none; Rust quick acceptance gate remains unmet.

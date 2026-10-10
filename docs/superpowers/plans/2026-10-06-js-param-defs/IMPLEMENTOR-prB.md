# R10 final landing dispatch — cache 114

The R10 candidate is the bounded source fold on **96817370ffb65dd3de14fe574789b6bf31e99ac5**, with docs based on **29a1d2d9139590baf3a281d1f32e233f01337026**. `repair-r10/source-manifest.json` and binary-binding.json bind the uncommitted candidate; controller owns its eventual commit SHA. R10 is COMPLETE and clean on all 579 admitted roots, with zero row/call-site/non-JS/navigation differences and no STOP; read MEASUREMENTS-prB-R10.md and final-custody-check.json for the terminal receipts. Landing is authorized only after a COMPLETE clean certificate, all required gates, owner E13 reaffirmation/revision and private F/controller Rust quick. No merge/adoption authority is granted here.

## Exact patches and custody

1. On a clean source96817370 checkout, apply checked `~/prism-evidence/js-param-defs/prB/repair-r10/R10-src.patch`. The source patch contains four AST hunks, full row controls and cache114/coupled mutants; it does not reapply historical PR-B work.
2. On docs29a1d2d9, apply checked R10-docs.patch. Preserve separate source/docs provenance. Each patch has git apply --check and exact reconstructed-byte receipts against a clean export with no Git metadata.
3. Rebind all728 src/tests/mutants files, including the untracked callback module, tests and fixtures, to R10 source-manifest.json before transferring gates. Commit source/docs only on the controller's landing checkout; all worker Git metadata writes were prohibited.

Both registries are required: `mutants/js-param-defs.json` **109** entries (PD110–115 added), and `mutants/lane-p-tsconfig-paths.json` coupled **P2-M11**. CACHE_VERSION114, PD-11/P2-M11 mutate114→113. Historical R9 cache113 receipts cannot replace fresh R10 gates.

## Fresh source-bound gates

R10: nextest --features mcp5247passed/0failed/1reservedskip; doctests2passed; PD109/109killed and admissible; coupled P2-M111/1killed (other120 lane-P mutants not rerun); fmtclean; same-environment local-main c8de720b clippy371/371 emitted warning multisets equal,229distinctlocations each; semantic763/763 +200kind cells with full matrix rows equalR9; Tier-A178/178 after immediate release rebuild; TS and Node quickVALID. VALID proves runnable admission, not precision. Worker/controller parity22/22, full probe suite71/71, F aggregate consumer8/8, shellsyntax pass.

In the eventual controller landing checkout, rebind hashes and retain new gate directories:

```bash
cargo nextest run --offline --locked --features mcp
cargo test --offline --locked --doc --features mcp
cargo fmt --all -- --check
python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --authoritative --jobs 4 --out "$R10_CONTROLLER_NEW_PD_GATES"
python3 scripts/mutgate/mutgate.py --lane mutants/lane-p-tsconfig-paths.json --only P2-M11-cpg-cache-version --authoritative --jobs 1 --out "$R10_CONTROLLER_NEW_CACHE_GATES"
cargo clippy --offline --locked --all-targets --features mcp -- -W clippy::all
cargo build --offline --locked --release
cd eval
.venv/bin/tier-a --matrix-only --allow-stale-sut
cd ..
cargo build --offline --locked --release
cd eval
.venv/bin/tier-a --quick --allow-stale-sut
```

The full quick includes controller-owned Rust. Use --allow-stale-sut only after an immediately preceding completed same-checkout release rebuild. Compare clippy with a same-environment clean main; never rebaseline. Full multi-corpus Tier-A remains human-triggered. Preserve each producer's wall time and excluded population. R10 worker does not re-run the seven excluded historical main producers or claim performance/O1/live-MCP verification.

## Controller-only F command

Use the corrected helper **and consumer**. Type-annotated JS bucket1 alone has the E13 exception; bucket2 non8xxx diagnostic JS and bucket3 everything else remain gated. Raw and after-override tables are both published. INADMISSIBLE Node proofs leave the oracle verdict unchanged; only the fixture-backed allowlist may override. Raw outside LOST CORRECT without that proof, effective outside LOST CORRECT, or outside ADDED WRONG is STOP. Private rows stay private; worker never opens frontend-portal or executes corpus packages.

```bash
PACKET="$HOME/code/prism-pd-plan/docs/superpowers/plans/2026-10-06-js-param-defs"
R10="$HOME/prism-evidence/js-param-defs/prB/repair-r10"
TS_JS="$HOME/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js"
SEAM_CENSUS_BIN="$R10/bin/seam-census" \
CORPUS_F_ROOT="$CONTROLLER_PRIVATE_F_ROOT" \
PRIVATE_EVIDENCE_ROOT="$CONTROLLER_NEW_PRIVATE_EVIDENCE_DIR" \
bash "$PACKET/CONTROLLER-pd.sh" diff "$TS_JS" \
  "$HOME/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3" \
  "$R10/bin/prism-head-r10" \
  "$HOME/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3-bytes" \
  "$R10/bin/prism-head-r10-bytes"
```

The controller supplies the private root and a NEW evidence directory; hashes are in repair-r10/binary-binding.json. The owner reaffirms/revises E13 using the corrected measured extent: React Native bucket1ADDED885/84/0/0, LOST11/385/26/0; aurelia LOST0/10/0/0. Earlier sixCORRECT/fiveINADMISSIBLE and all-diagnostic figures are superseded. The owner's actual decision wording is unchanged.

F10/F11, recovered annotated-JS residue/file-level guard, malformed-arrow recovery, oracle class-self-name limitation, and cross-file/namespace enum merging remain recorded follow-ups. An unexplained row class, call-site/non-JS/navigation difference, outside adverse row or unrelated gate failure is STOP. Finish collect-all measurement before reporting the complete STOP population.

## Historical appendix — superseded dispatches

The material below is retained for provenance only. Its source bases, cache versions, counts, and authorization boundaries do not dispatch the R10 head.

**R6 continuation (2026-10-06):** Owner prB-repair-r6-brief.md authorizes the closed CERT-1 repair on the retained b249da95 candidate, loop3/3. Historical R5b STOP receipts below remain evidence; current state is MEASUREMENTS-prB-R6.md / HANDOFF-repair-prB-r6.md. No completion or adoption is claimed.
**Historical R5b supersession (2026-10-06):** Original R5 questions are resolved by prB-repair-r5b-brief.md. At R5b, the candidate was STOP on35 actual non-formal outside-certificate rows; read MEASUREMENTS-prB-R5.md/HANDOFF-repair-prB-r5.md. Historical receipts remain unchanged below.

> **Historical R4 authority (2026-10-06): STOP; PR-B PARKED.** R4 repaired ordered conditional parameter sources and bounded early errors. Six invoked-default design cells still require callable timing/captured side-effect design; 18 main-correct synthetic rows remain lost. No adoption or shipping claim. Read MEASUREMENTS-prB-R4.md and HANDOFF-repair-prB-r4.md; earlier results remain historical. Source base 1af4301f; docs base a3dda010.

# Historical retained R2b operation

The bounded worker fold/measurement/custody is complete on admitted evidence. Read MEASUREMENTS-prB-R2b.md/HANDOFF-repair-prB-r2b.md and repair-r2b final-custody-check.json/patch-manifest.json. Rust quick is INVALID after oracle_timeout and known denied corpus-target writes; no denied-class retry, timeout relaxation, all-green claim or auto-adoption. Controller verification remains required before full gate acceptance. Frozen source is cache111; binaries/patches are current R2b, source base cb996630/docs base2cb7102a.

No source edit, restart, extra review round, Git writes, corpus execution or frontend-portal access was authorized beyond the targeted brief. The dispatch below is historical and retained solely for provenance.

# Historical IMPLEMENTOR dispatch — js-param-defs PR-B callback identity, R2

**Historical R2 state: STOP.** Four checker-CORRECT LOST rows in extra-asciinema_1.0.0 trigger the explicit R2 brief stop rule. Preserve the retained source and exact-base patches; do not continue repair or reclassify the rows without a new owner instruction. See MEASUREMENTS-prB-R2 and external STOP-secbench-evidence.

Dispatch from committed source `origin/proto/js-param-defs-prB` = `cb99663090966c8b128d19649f3b7fc820907f7a` plus `~/prism-evidence/js-param-defs/prB/repair-r2/R2-src.patch`. Docs/eval base is committed `2cb7102a0b4c186a1a6c09c586dae4620c8e6de8` plus `R2-docs.patch`. The respective R1 patches are already folded into those commits. Product comparison base remains main `da0604b3` (PR-A merged). This is the owner's disclosed one-time cap extension after FIX at review 2/2: targeted repair of closed instances on the retained artifact, with no restart, adoption, merge or further review authority.

Read SPEC-prB B-D1–B-D14 and the exact `prB-repair-r2-brief.md`. Never open frontend-portal, execute corpus packages or perform Git writes. Use `--cache-dir ~/prism-evidence/js-param-defs/cache`. Exact is a static-binding grade. Owner STOP-1(a), O1 and standalone O2 remain in force.

| Finding | Bounded repair | Required controls |
|---|---|---|
| N1 / S-e / S1 | Read classifier climbs parentheses, non-null, as, satisfies and type assertions; simplify member boolean | Identifier/member writes refused; compound reads remain; missing wrapped Def collection remains legacy parity |
| R2-W1 | Callable-body function declarations use the hoisted environment | Simple-parameter sharing, unrelated function and actual inner block |
| R2-W2 | Expression parameters separate the body var/function environment | Direct/closure defaults, computed keys, simple and typed parameter controls |
| R2-W3 | Carry admitted reference occurrence bytes onto synthetic Use endpoints | Both same-line read orders and original Def/alias-twin regressions |
| R2-W4 | Refuse synthetic Def/Use admission under with bodies, including closures created there | Preserve evaluated with expression and outside/ordinary-block reads; legacy named parity |
| S-d / S2 | Current committed custody pins and binding Def scope wording | Historical R1 measurements remain identified as historical |

Re-measure X, Xi, T and SecBench byte tables: ADDED/LOST/RELABELLED/RE-OWNED split CORRECT/WRONG/UNDECIDED. LOST CORRECT and ADDED WRONG must each be zero. Preserve non-JS byte and call-site identity. Sample X/T ego/nodes-at/callers/callees/repo-map; every change must be explained by removed checker-WRONG rows. Re-run O1 standalone and joint-counterfactual SecBench, keeping O2 standalone authority. R1's perf and independent samples are inherited unless re-run.

Run full endpoint/label regressions failing on `cb996630`, mutants per guard, one `cargo nextest run --features mcp`, advisory scoped mutgate, fmt, clippy parity 235/235, immediate release rebuild then matrix 178 and quick VALID. Do not transfer historical gates to R2 or rebaseline failures. Advance the existing CPG semantic cache version to 110 and preserve coupled PD-11/P2-M11 intent.

Freeze production and byte binaries under `prB/repair-r2/bin/`, record SHA256/source manifest, and write exact-base R2 patches. Current evidence and handoff: `MEASUREMENTS-prB-R2.md`, `HANDOFF-repair-prB-r2.md` and external `prB/repair-r2/`. Controller owns commits.

STOP and report, without choosing: NEW FAMILY beyond these seams; nonzero LOST CORRECT; nav change unexplained by removed WRONG rows; non-JS nonidentity. F remains controller-only.

Final R2 also refuses object-backed with-body lvalues before emitted or kill-only Def admission, and removes unproved with-body Use inventory. Its additional regression fails on the preliminary R2 binary; ordinary-block and legacy named controls stay admitted. Preliminary cache109 captures are retained as superseded; final cache110 binaries and measurements carry current authority.

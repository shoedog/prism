# R9b final landing dispatch — source 96817370

The retained source is `96817370ffb65dd3de14fe574789b6bf31e99ac5`. R9b closes the bookkeeping STOP: the controller authorized TYPE_PREDICATE (F9), all 62 rows have independent per-row AST/checker proofs, and controller/worker directive parity passes. Product source remains frozen. The collect-all certificate is COMPLETE and clean on admitted roots; excluded roots receive no semantic credit. E13 reaffirmation or revision, private F, and controller Rust/full quick remain landing prerequisites. No merge or adoption is authorized by this dispatch.

## Exact commits and custody

1. Retain source commit `96817370ffb65dd3de14fe574789b6bf31e99ac5` (`fix(js-param-defs): fold R9 recovery and binding boundaries`) on `origin/wip/js-param-defs-prB-r3`. Its parent is `2431cb104db7828d11ae409dc207c1e339270c93`; do not reapply the historical R9 source patch to 96817370.
2. Retain docs commit `5ddbf1d64991b7b9f609b1643b0951bd475b996f` (`docs(js-param-defs): bind corrected R9 evidence and STOP`), then apply `~/prism-evidence/js-param-defs/prB/repair-r9b/R9b-docs.patch` relative to that exact base. Proposed docs commit: `docs(js-param-defs): close R9 bookkeeping STOP with independent proofs`.
3. In the controller's landing checkout, bind the complete src/tests/mutants tree to 96817370, including files untracked in the planning checkout. Rebind the docs patch and evidence manifests before transferring any result. All 727 product/test/mutant files matched at R9b entry; controller owns Git writes. There is no R9b source patch.

Current evidence: `MEASUREMENTS-prB-R9b.md`, `HANDOFF-repair-prB-r9.md`, and `~/prism-evidence/js-param-defs/prB/repair-r9b/{REPORT,HANDOFF}.md`. The R9 reports preserve the historical STOP as explicitly superseded history.

The actual cache pin is `CACHE_VERSION = 113`; both `mutants/js-param-defs.json` PD11 and `mutants/lane-p-tsconfig-paths.json` P2M11 mutate 113 → 112. The authoritative PD registry has 103 anchored mutants, including PD100–109. F10 `RdResult.reaching_edges` and F11 alias-twin ordering remain follow-ups.

## Gates and controller landing order

R9's source-bound gates stay valid because R9b changes no src/tests/mutants bytes or frozen binaries. They were not rerun: nextest `--features mcp` 5,241 passed / one reserved skip, doctests 2 passed, PD mutants 103/103 killed and admissible, coupled P2-M11 1/1 killed, fmt clean, clippy head/main 235/235 with equal warning multisets, Tier-A matrix 178/178, TS and Node quick VALID. The remaining 120 lane-P mutants were not rerun in R9. VALID is admission, not a precision claim. Historical perf/sample/MCP results are not fresh R9b gates.

Fresh R9b checks are the full probe unit suite (50/50), controller/worker parity (6/6), four early-error allowlist positives plus four generic-parser negatives, and shell syntax. Both raw directives remain strict; escaped literals remain non-strict. The same-environment 5ddbf1d6 helper control fails three escaped cases before the fix.

Before landing: obtain the owner's E13 reaffirmation/revision on the corrected extent, run the private F command below, and run the controller Rust/full quick. On the eventual clean landing checkout, preserve complete gate logs using new evidence directories:

```bash
cargo nextest run --offline --locked --features mcp
cargo test --offline --locked --doc --features mcp
cargo fmt --all -- --check
python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --authoritative --jobs 4 --out "$R9_CONTROLLER_NEW_PD_GATES"
python3 scripts/mutgate/mutgate.py --lane mutants/lane-p-tsconfig-paths.json --only P2-M11-cpg-cache-version --authoritative --jobs 1 --out "$R9_CONTROLLER_NEW_CACHE_GATES"
cargo clippy --offline --locked --all-targets --features mcp -- -W clippy::all
cargo build --offline --locked --release
cd eval
.venv/bin/tier-a --matrix-only --allow-stale-sut
cd ..
cargo build --offline --locked --release
cd eval
.venv/bin/tier-a --quick --allow-stale-sut
```

The full quick includes the controller-owned Rust check. `--allow-stale-sut` requires the immediately preceding rebuild in that same checkout. Keep the same-environment main clippy control and report any exclusions or regressions; do not rebaseline. Full multi-corpus Tier-A remains human-triggered. Record wall time for every later producer per root; R9b's producer-times.json preserves new timings and explicitly labels unavailable historical times.

## Controller-only F command — parity block resolved

The bounded raw-literal directive fix is authorized by prB-repair-r9b-brief.md and tested. The command is ready for the controller. Never open `frontend-portal` in an implementer/reviewer session or evaluate corpus packages. The controller supplies the private root and a NEW private evidence directory:

```bash
PACKET="$HOME/code/prism-pd-plan/docs/superpowers/plans/2026-10-06-js-param-defs"
R9="$HOME/prism-evidence/js-param-defs/prB/repair-r9"
TS_JS="$HOME/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js"
SEAM_CENSUS_BIN="$R9/bin/seam-census" \
CORPUS_F_ROOT="$CONTROLLER_PRIVATE_F_ROOT" \
PRIVATE_EVIDENCE_ROOT="$CONTROLLER_NEW_PRIVATE_EVIDENCE_DIR" \
bash "$PACKET/CONTROLLER-pd.sh" diff "$TS_JS" \
  "$HOME/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3" \
  "$R9/bin/prism-head-r9" \
  "$HOME/prism-evidence/js-param-defs/prB/bin/prism-base-da0604b3-bytes" \
  "$R9/bin/prism-head-r9-bytes"
```

The script binds the binaries and probes, verifies byte-to-wire projections and call-site equality, and applies the fixture-backed early-error allowlist. Other compilation failures are INADMISSIBLE. Pinned TypeScript 5.9.3 diagnostics define the type-annotated `.js`/`.jsx` boundary. Detailed private paths and rows remain private; only aggregate counts leave. Outside LOST CORRECT or ADDED WRONG, a non-JS/call-site change, or an unexplained navigation difference is STOP. The independent f8c768b3 certificate and its new TYPE_PREDICATE proofs are in the R9b packet; this F command compares main/head tables and identity.

Frozen binary hashes remain in `repair-r9/binary-binding.json` and are freshly checked in `repair-r9b/entry-binding.json`. The source commit and the docs patch must retain separate provenance. After the required owner/controller gates pass, the controller may land the retained candidate; merging and adoption require their own authority.

## Historical appendix — superseded dispatches

The material below is retained for provenance only. Its source bases, cache versions, counts, and authorization boundaries do not dispatch the R9 head.

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

# R9 landing dispatch — retained candidate, STOP / parked

The R9 repair is frozen. The corrected collect-all certificate has differing rows outside its enumerated classes, so this dispatch does not authorize landing, further repair, a fresh candidate, or merging. First obtain the owner's disposition of the complete STOP population and reaffirmation or revision of E13. Preserve the source and evidence already reviewed. Current measurements and exclusions are in `MEASUREMENTS-prB-R9.md`, `HANDOFF-repair-prB-r9.md`, and `~/prism-evidence/js-param-defs/prB/repair-r9/REPORT.md`.

## Exact inputs and commits

Source base: `origin/wip/js-param-defs-prB-r3` = `2431cb104db7828d11ae409dc207c1e339270c93`, plus `repair-r9/R9-src.patch`. The patch includes the existing untracked source files as modifications against that base, and passes `git apply --check` on an extracted clean base. Docs base: `b2aa4f21a901632ea58e2c348f8bdbd4c7a5fa32`, plus `repair-r9/R9-docs.patch`; it includes the intervening docs commit `3f6e582ed07a6194c717e20828ca1b47256e5ee3`. Controller owns all Git writes; the repair engineer made none.

Proposed separate preservation commits:

1. `fix(js-param-defs): fold R9 recovery and binding boundaries`
2. `docs(js-param-defs): bind corrected R9 evidence and STOP`

The frozen source pins `CACHE_VERSION = 113`; both `mutants/js-param-defs.json` PD11 and `mutants/lane-p-tsconfig-paths.json` P2M11 test the 113 → 112 invalidation. The authoritative PD registry has 103 exact anchored mutants, including PD100–109 for R9. Neither `RdResult.reaching_edges` nor alias-twin ordering was changed; F10 and F11 remain follow-ups.

## Source-bound gates

R9 has nine RED-on-2431cb10 / GREEN regression groups, 48 Node semantic controls, 56 function-kind × strictness × parameter-shape cells, and the retained 763 semantic / 2,289 grammar cells plus the 200 kind cells. Language-specific syntax uses only valid JS, TS, or TSX variants; TS enums/type predicates and JSX are not asserted as valid plain-JS/.ts syntax. See MATRIX-param-env-R9.md for all cells and negative controls, including ten extra JSX emit/runtime controls.

The current gate receipts record nextest `--features mcp` 5,241 passed / one skipped; doctests 2 passed; mutants 103/103 killed with zero invalid probes; fmt clean; clippy 235/235 with an equal warning multiset; Tier-A matrix 178/178 and TS/Node quick VALID. These bind the frozen R9 source/binaries, not a later controller checkout. Do not rebaseline a regression.

After an owner-authorized landing disposition, apply the exact patches in the respective clean bases and rebind all source hashes and dirty scope. In that landing worktree run the following gates and preserve their complete output:

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
.venv/bin/tier-a --quick --allow-stale-sut
```

The final command includes the controller-owned Rust quick, excluded from the worker's TS/Node-only quick. A stale allowance is valid only after the immediate release rebuild in the same worktree. Full multi-corpus Tier-A remains human-triggered. Re-run both mutation registries' coupled cache guards and the authoritative PD subset using the packet's recorded mutant runner; use the same-environment main clippy control rather than assuming historical warnings. Report any environment exclusion explicitly.

## Controller-only F step

**BLOCKED:** final controller parity control found a WRONG override for an escaped directive string (worker INADMISSIBLE, controller WRONG). The helper remains unchanged under the R9 STOP rule. Do not run this command until the bounded raw-literal directive correction is separately authorized and tested; see MEASUREMENTS-prB-R9.md and controller-escape-control.json. The command below is retained for review, not ready for execution.

Never open `frontend-portal` in an implementer/reviewer session and never evaluate corpus packages. The controller supplies the private root and a NEW private evidence directory. This is the concrete F command; the root value remains private:

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

The script binds all supplied binaries, checks byte-to-wire equality and call sites, and applies the R9 fixture-backed early-error allowlist. Other Node compilation failures are INADMISSIBLE; no CORRECT/WRONG credit. It defines type-annotated `.js`/`.jsx` using TypeScript 5.9.3 diagnostics, not `@flow`, and publishes inside/outside aggregate tables. Keep private paths, rows, and detailed verdicts in the private directory. A differing non-JS/call-site row, unexplained navigation delta, outside LOST CORRECT or ADDED WRONG, or outside-class certificate row remains STOP. Owner E13 wording stays unchanged until the owner rules on the corrected extent.

The frozen R9 binaries and SHA256 values are in `repair-r9/binary-binding.json`; the source manifest and exact-base patch checks are in that evidence packet. No automatic merge or adoption is part of this dispatch.

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

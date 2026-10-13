# Handoff — Lane PKG R1 bounded repair; controller review next

**Written:** 2026-10-04T23:34:59.876818+00:00 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/prism-pkgres · proto/workspace-package-resolution · **Measured state:** `[MEASURED]` HEAD 92c1d0bc05ac90f4d2505f2deb2eb309f08e4318 · Tree DIRTY (7 tracked owned files and untracked packet) · Probe git rev-parse HEAD / git status --short · Output repair-r1/checkout-status-final.txt
**Predecessor:** supplied prototype and R1 reviews; previous packet handoff archived in repair-r1/historical-packet.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker using the canonical steering bootstrap/handoff-template.md. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not. No actual transcript-model identity verification is claimed.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` repair-r1-brief assigns this worker; no subagents dispatched — RESOLVED assigned scope. Global host-session liveness is not claimed.
**(b) Custody exposure** — `[MEASURED]` no Git writes; owned bytes remain dirty/untracked. Local immutable binaries, source/packet snapshots and separately based patches are bound by repair-r1/final-custody.json — RESOLVED local custody; controller Git custody remains OPEN. No off-machine backup claim.
**(c) In flight / irreversible** — `[MEASURED]` all known worker build/test/oracle/public producers completed; their logs and tool completion receipts retained — RESOLVED known producers. No private F or S2 execution/adoption.
**(d) Authorization granted but not exercised** — “Read and follow /Users/wesleyjinks/prism-evidence/pkgres/planning/repair-r1-brief.md exactly.” Brief boundaries: “no git writes, never open private F, keep disk use lean.” Controller Git/F actions remain outside worker authority.

## 1. Resume order

1. `cd /Users/wesleyjinks/code/prism-pkgres && git status --short && git diff --check` — seconds, expect seven tracked owned changes plus packet. Read repair-r1/final-custody.json and this packet's BUILD-MANIFEST.json; verify source/probe/binary hashes before transferring any receipt.
2. Review R1-REPORT, MEASUREMENTS, VERIFICATION and OQ. Source patch applies to 92c1d0bc; packet patch applies to c89bc5b7. These are distinct sibling bases, not a single branch. Use IMPLEMENTOR's composition order.
3. Controller handles independent review, Git custody and any private F execution from bound binary roles. Broader package/S2 work requires a separate authorized bounded increment; preserve this repaired artifact.

**STOP conditions:** Any public wrong binding or false Exact/lost main edge: report and choose nothing. Ownership/source/population drift invalidates transferred receipts. No private F, Git writes, network, source restart, loader/source-output policy or S2 adoption in worker scope. Diagnostic cap two; bounded cache/one-class guard extensions disclosed, no restart.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Checkout/review binding | done | `[MEASURED]` both full reviews read; checkout-status-final.txt; distinct 92c/c89/main bases |
| Material WRONG repairs | done | `[MEASURED]` R1-REPORT findings map; prechange-reproductions.json; 10 behavioral prototype REDs; full final GREEN |
| Three-way API / S2 contract | done | `[MEASURED]` Bound / ProvenUnresolved / Unsupported; SPEC; #imports explicitly Unsupported |
| Differential gate | done | `[MEASURED]` differential-r1-final/summary.json: 5096 cases, 2242/861/1993; zero wrong/false absence; 1651 native-bound Unsupported |
| Persisted cache regression | done | `[MEASURED]` cache-r1-final/summary.json: 12 genuine warm/edit/cold checks, both artifacts |
| Full MCP suite / doctests | done | `[MEASURED]` full-r1-final.log and totals: 5163/0/1, 2 doctests, 24 package tests |
| Requested one nextest | done | `[MEASURED]` nextest.log: 5160/0/1 before final typesVersions guard; cannot certify final source |
| fmt/clippy/mutgate | done | `[MEASURED]` final logs; clippy warnings182; advisory26/26 killed |
| Tier-A/S1b | done | `[MEASURED]` matrix178 OK; S1b411 scenarios/1234 byte-identical files |
| Public/lane-P preservation | done | `[MEASURED]` public-r1-final and module-audit: zero changed/lost/wrong, installed-X correct module proof+1 |
| Patch/build/custody | done | `[MEASURED]` final-source-binding, BUILD-MANIFEST, final-custody; final census after tests, explicit timing |
| Undefined Opus repros | pending | `[UNKNOWN]` C3,C15,C17-C21,C23,C24,C26: definitions/public fixtures not supplied; request sent, no answer |
| Independent review / F / Git | pending | `[INHERITED]` controller boundary; current gates are self-pass only |
| S2 / full-feature adoption | parked | `[MEASURED]` Unsupported native-bound1651 is explicit gap; no new S2 run; historical planning result0/132 only |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| SPEC / IMPLEMENTOR | Old dirty-base dispatch and early scheme absence | `[MEASURED]` committed source92c plus R1, packetc89; authority precedes scheme; three-way S2-O9 contract |
| Build / measurements / verification / handoff | Prototype107/63, 5147 suite, old binary roles/current-S2 claims | `[MEASURED]` current108/64, final5163 suite, R1 immutable roles; old documents archived, historical packet notes explicit |
| Verification link | Nonexistent root report | `[MEASURED]` packet-local VERIFICATION-root.md link corrected |
| missing() helper | exact_target null falsely passes negative assertions | `[MEASURED]` resolved_targets required empty; literal-star witness now behavioral RED |
| Root typesVersions guard | R1 canonical file rung could bypass entry-level guard | `[MEASURED]` guard moved before file probes; full144-case population passes |
| Cargo custody | Top-level executable assumed identical to measured CLI | `[MEASURED]` retained MCP dependency artifact matches measured CLI; top-level differs; no substitution or feature-only causal claim |
| Memory | None | No relevant memory used; no update authorized or made |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Complete undefined repro coverage | pending | Owner supplies public fixture descriptions; add native-bound witnesses in a bounded follow-up | Missing definitions | OQ undefined10IDs |
| 2 | Independent review / F acceptance | pending | Reviewer binds exact patches; controller rebinds and executes F script if authorized | Controller/reviewer | BUILD-MANIFEST, CONTROLLER-pkg.sh |
| 3 | Git custody | pending | Controller commits literal FILES groups with proposed messages | No worker Git authority | R1-src.patch / R1-docs.patch |
| 4 | Ordinary lane-P JSX and native-bound Unsupported | parked | Separate spec/native-witnessed increments; no S2-O9 exclusion for Unsupported | Scope/owner authority | OQ; differential gap1651 |

## 5. Invariants and traps — do not do these

- Never open private F, write Git, fetch/install or adopt S2; worker authority stops at reviewable patches.
- Never flatten Unsupported into native absence; only ProvenUnresolved supports S2-O9.
- Derive nested fixture writer path and real ProjectService owner before probing; an absent guessed writer is inadmissible.
- call-stats alone does not materialize nav sidecar; query callees and require a non-null symbol for Exact identity.
- Root typesVersions authority must precede canonical file probing, not only entry().
- Run Tier-A from this checkout's eval directory; root-cwd editable import selected the wrong module and was inadmissible.
- Never substitute top-level Cargo executable for a manifest-bound measured binary; mixed feature/transitive artifact identities differ.
- Single nextest predates final guard; use final full Cargo receipt for final source. Failed receipts and old planning controls stay historical.
- No helper examples remain; don't race helper creation/removal against all-target fmt/clippy. Keep offline debug0/incremental0 builds and lean snapshots.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Source prototype | `92c1d0bc05ac90f4d2505f2deb2eb309f08e4318` |
| Packet base | `c89bc5b74df05e1f8746792c0cfad2805336b0f9` |
| Main comparison | `4e592daa7858a195eb3a9eb77c83dfbc763b49fa` |
| R1 CLI | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/bin/r1-prism` |
| R1 facts | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/bin/r1-facts` |
| R1 resolution helper | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1/bin/r1-resolution` |
| Oracle | `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js` |
| Evidence | `/Users/wesleyjinks/prism-evidence/pkgres/repair-r1` |
| Packet | `docs/superpowers/plans/2026-10-04-workspace-package-resolution` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "Final R1 admits only native-matching module/owner and Exact callable identities in the measured differential/public populations" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: repair-r1/differential-r1-final/summary.json and public-r1-module-audit.json. Early root-typesVersions candidate was refuted and corrected in place; this verdict does not claim full TS conformance or S2 adoption.

**Questions the owner owes an answer to:** Supply public definitions for the ten undefined Opus IDs to complete that requested coverage. Controller decides independent acceptance/Git/private F and any later S2 increment; those decisions are outside this repair.

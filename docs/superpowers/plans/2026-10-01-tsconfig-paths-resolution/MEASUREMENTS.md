# Lane-P P1 repair round 1 measurements

[MEASURED] Integrated parent `7f5a862afd002dd5953108a3ab5b611d8745bde9`, branch `feat/tsconfig-paths-p1`; base `dab8251c6013b28b0db4cb4042f36db444eb556c`. No Git writes. This report supersedes prototype-era operational receipts in the prior version of this document. Evidence is local under `target/repair-r1/`; private corpus F was never opened.

The same-environment base binary and import-facts helper were freshly built offline from a Gitless archive of the base. The pre-repair binary was freshly built before production edits. Final production is frozen in `head/prism-r1-verified`; source and executable hashes are recorded in BUILD-MANIFEST and `source-binary-binding.json`. TypeScript 5.9.3 is supplied offline through PRISM_TYPESCRIPT.

## Finding dispositions

The inherited denominators are Opus **7 WRONG / 5 SMELL** and sol61 **4 WRONG / 2 SMELL**. No finding was downgraded or discarded. Each mechanism was repaired in place, with conservative refusals retaining complete base rows.

| Findings | Repair / disposition | Measured cost and evidence |
|---|---|---|
| sol W1 (A) | CLOSED: separate alias export projection proves every named, star and import-forward hop; relative/namespace consumers retain legacy tables | C97 refuses 24 formerly admitted rows; X loses zero of 3,121 gains, including 2,943 indirect terminals. Both-grammar integration/packet negatives and I12 kill |
| Opus W1 / sol W2 (B1) | CLOSED by refusal: dotted last-suffix replacement occupant or opacity | C90 8 rows return to base, including body/declaration and multiple-dot variants; M31 kill |
| Opus W2 (B2) | CLOSED by refusal: trailing slash and directory target spellings | C91 6 rows return to base; M32/M33 kill |
| Opus W3 (B3) | CLOSED by refusal: JS terminals when the specifier package or @types package is occupied at any ancestor | C92 8 package rows return to base; two absent-package positive rows retained; M34 kill; above-root cache parity. X has zero JS-family alias targets |
| Opus W4 (B4) | CLOSED by refusal: matching in-scope ambient exact/wildcard declarations | C93 12 rows return to base in .ts/.d.ts and both target grammars; M35 kill |
| Opus W5 / sol W3 (C) | CLOSED by whitelist barrier: raw membership syntax and output options without explicit exclude | C94 20 and C95 4 rows return to base. Original 197 controls lose zero gains; X cost zero. Six explicit-exclude output positives retained; M36–M38 kill |
| Opus W7 / sol W4 (D) | CLOSED: exact occupancy dependencies, ambient patterns and ancestor package facts join topology | Both CLI cache probes pass; new 16 cases / 32 add-remove directions, unrelated text preserves cache bytes/mtime; I13 kill |
| Opus W6 / sol S1 (E) | Config maps Rc-shared, raw paths JSON not duplicated; decline work skips ambient scans and duplicate export extraction | CLOSED: Nx median wall +15.8%, median RSS +16.6% versus base; declined Bundler shows no observed time penalty. Full comparator/ranges below |
| Opus S1 | CLOSED by refusal: allowJs-off JS alias and barrel terminals | C96 2 rows return to base; relative parity controls pass; M11 kill |
| Opus S2 / sol S2 | CLOSED: post-split integration selectors and meaningful config-fingerprint witness | All 11 legacy mutants plus three new alias/cache/case mutants killed on final source |
| Opus S3 | CLOSED: warning on the 200,000-entry budget failure | Pre-repair missing-warning assertion RED; repaired 200,001 filler fixture returns two complete base rows with warning |
| Opus S4 | CLOSED: any folded config-name collision is a barrier regardless of read order | Unit exercises both orders; I14 killing mutant killed. Native case-sensitive filesystem coexistence is not verified on this Mac |
| Opus S5 | CLOSED: shared integration helpers | `tests/integration/js_paths_common.rs`; full suites pass |

## Public yield

Final frozen-binary public/oracle run: X 19,219 sites / 3,121 changes, R 953 sites / zero changes, T 61,712 sites / zero changes. Every changed row is CORRECT_STATIC_BINDING; classes for R/T are empty. Zero site-key or metadata changes. X/R/T unchanged ProjectService ownership disagreements: 6/144/15 sites in 2/17/5 files; changed disagreements zero. Final receipts: public-verified/FINAL-SUMMARY.json.

X retains 3,121/3,121 existing Exact gains; 178 target their direct module and 2,943 terminate through barrels/forwarding. Alias modules: 3,087 .ts, 34 .tsx; terminals: 3,090 .ts, 31 .tsx. Across all 3,156 paths-associated X rows, explicit JS-family substitutions and resolved JS-family module/terminal targets are each zero (X-yield.json). X loss by cause: hop proof 0, dotted replacement 0, slash 0, JS/package 0, ambient 0, membership/output whitelist 0, allowJs 0. No prior X gain falls back to base. Final receipts bind input hashes, site metadata, source-backed expected terminal spans and every class.

## Controls and source-bound RED/GREEN

Regenerated packet: **317 scenarios / 333 sites / 80 changes** = **76 CORRECT_STATIC_BINDING + 2 CORRECT_STATIC_REFUSAL + 2 UNPROVEN**. All preservation rows equal complete base; zero site-key changes or preservation violations. Existing C80/S6 ownership disagreement gains remain UNPROVEN and parked under OQ2, never certified.

Original 197 scenarios / 213 sites retain their 72 changed rows with zero loss. Added 120 scenarios contribute eight safe gains: six output-option controls with explicit exclude and two JS package-absent controls. The other new controls enforce the repair cuts. Pre-repair-to-repair loss: **84 rows**, broken down C90 8, C91 6, C92 8, C93 12, C94 20, C95 4, C96 2, C97 24. This is measured conservative refusal cost, not an assertion that every formerly admitted row is wrong. Pre-repair oracle classes over 164 changed rows were 125 CORRECT_STATIC_BINDING, 2 CORRECT_STATIC_REFUSAL, 21 UNPROVEN and 16 WRONG_OR_SPAN_MISMATCH. All required new scenario families have JSX and TSX witnesses.

The pre-repair archive with the new regression tests runs 12 repair groups: 11 behavioral failures and one preservation pass. The preserved config-swap group discriminates cache config bytes from exact dependency occupancy and kills I01; it does not claim a new behavioral repair. Budget warning separately fails before repair. Compiler, selector and environment errors are inadmissible, recorded in hypothesis-probe-result.log. Final-source mutants: **38/38 kernel killed** (30 legacy + 8 new) and **14/14 integration/library killed** (11 legacy integration + 2 new integration + 1 case-collision library). Every integration/library mutant compiles, selects exactly one test and produces the intended assertion panic; no zero-test or setup failure counted. M10/M17 needed additional witnesses because overlapping refusal gates masked their initial controls; the full defect/selector population was enumerated and rerun after adding C36b/C98.

## Suites and parity

| Full suite | Passed | Failed | Ignored | Groups |
|---|---:|---:|---:|---:|
| default | 4,827 | 0 | 1 | 29 |
| mcp | 5,020 | 0 | 1 | 31 |
| all-features | 5,043 | 0 | 1 | 31 |

PRISM_TYPESCRIPT points to the hash-pinned offline 5.9.3 package. Existing ignored test: `resolution_test::slice_elem_variant_reserved`. `cargo fmt --check` and `git diff --check` pass. Full all-features/all-targets clippy: base 372 warning diagnostics, head 372, no new warnings, measured in the same environment.

Tier-A matrix after immediate release rebuild: **170 ok / zero regressions**, no baseline edits. Paired same-corpus Tier-A quick with rust-analyzer completed but **both baseline-invalid**: C-method 4/6, C-name 4/6 and oracle error rate 0.1333 exceeds 0.10. Both SUT error rates are zero; M1, every SUT probe value and pinned SUT values are identical. One oracle site list differs only in order. Pinned outcomes in both: one flip candidate, two missing, one ok. Base matrix 168 ok / two expected paths-positive RED; repaired matrix 170 ok. Same-environment control therefore does not attribute the invalid quick to this change. No quick-green claim or rebaseline; tier-a-quick-comparison.json preserves detail. Full multi-corpus Tier-A is human-triggered and not run.

S1b-4: **411 scenarios / 639 sites / 1,234 artifacts** byte-identical between fresh base and frozen repaired binary. Summary SHA-256 `b550a2c7466fdbe4d331f93843d44f0bfcfb6c62febf81f115c86a9ab64dd5ca`. Old interrupted/overwritten-binary artifacts are superseded and inadmissible; the full population was enumerated before rerunning.

## Performance and limits

Nx fixtures reproduce the reviewer shape: 500 aliases, 8,000 callers, 500 implementation files, 64,000 call sites; Bundler declines and one-key wildcard is a comparator. Three paired runs alternate execution order. Resource accounting uses an isolated one-child Python wrapper (monotonic wall, macOS RUSAGE_CHILDREN peak RSS), avoiding the sandbox-denied BSD time -l sysctl. Each run reads 64,000 actual site rows and validates Exact count: base 0, admitted head 64,000, declined head 0.

| Scenario | Base/head median seconds | Time ratio | Base/head median peak RSS MB | RSS ratio | Base/head maximum RSS MB |
|---|---:|---:|---:|---:|---:|
| Nx 500 aliases | 6.930 / 8.022 | 1.158 | 764.2 / 890.8 | 1.166 | 770.3 / 892.6 |
| Bundler decline | 8.979 / 7.141 | 0.795 | 650.1 / 735.3 | 1.131 | 767.1 / 760.3 |
| One-key wildcard comparator | 8.521 / 14.039 | 1.647 | 657.3 / 774.7 | 1.179 | 760.6 / 903.7 |

Nx meets the approximate 10–20% target in wall and memory, and removes the reviewed files-times-paths allocation mechanism. Bundler has no observed time penalty; median RSS is 13.1% higher while maximum RSS is lower, so this does not prove literally zero overhead. The wildcard comparator is slower, with timing variation; all measured runs are retained, not averaged into the better Nx result. No isolated root-cause or steady-host performance claim is made for that comparator. Child median CPU seconds: Nx 9.437/10.916, Bundler 11.292/9.732, wildcard 10.785/15.167. The initial batch overlapped other repair verification and is retained separately in perf-loaded; the table is the serial final batch, with no other repair jobs in flight.

Verification-gate follow-up adds an executable regression bound over the process receipts: Nx and declined Bundler median time/RSS must each be at most 1.20x base. Repaired receipts pass both checks. A fresh three-pair replay of the immutable pre-repair binary fails both checks: Nx base/pre-repair median 8.054/14.642 seconds and 770.2/2366.0 MB RSS (3.07x memory); declined Bundler 6.801/10.607 seconds and 768.1/2237.1 MB RSS (2.91x memory). Each run validates 64,000 actual sites. This supplies same-environment behavioral RED/GREEN for the sharing/performance repair, rather than relying only on inherited reviewer timings. No production or Rust test changes occurred after the full suites. Root VERIFICATION.md records exact commands and the per-behavior negative/edge coverage.

Private F is controller-only; CONTROLLER-paths.sh points to the final frozen binary and emits its hash. No Git commit/push, network/install, independent round-2 review, full multi-corpus Tier-A, Linux case-sensitive filesystem run or concurrent-filesystem custody audit was performed. The packet's stable-tree assumption and parked S6/OQ2 boundary remain. Local evidence/snapshots require controller commit/external custody. Two internal passes were declared; one bounded, disclosed extension at the cap fixed finite decline ordering/duplicate-extraction work without restarting the artifact.

> **Controller note (2026-10-01):** the landed cache stays **105 / 61**, one bump from main 104 / 60. The repairer's 106 / 62 only separated its own iterations. Verification receipts that name 106 / 62 refer to the frozen repair binary.

## Controller F acceptance after implementation repair r1 (private, aggregates only; 2026-10-01)

- **Changed rows:** **2,343**, all `CORRECT_STATIC_BINDING`. That is 2 fewer than the 2,345 before the repair: the refuse-on-doubt cuts refused 2 correct F rows, which is a disclosed conservative cost.
- **Disagreements and keys:** 0 changed rows disagree with tsserver ownership; 0 keys added or removed.
- **Refusal histogram:** HOP 717, GUARD 23, UNCLASSIFIED 23.

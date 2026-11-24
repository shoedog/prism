> Current integration authority — 2026-11-12: This product line is integrated onto freshly fetched main `4f9e3a3eb8d2fc30c59d3741a494eafd6b1d3ac3` in `feat/review-crossfile-context-main-sync-20261112`. All old acceptance/test totals below are inherited historical receipts on their original bases, not validation of this integration. Before each next product slice, synchronize with current main and record the exact resulting base. Every delegate, regardless of provider, runs affected/dependent slice checks; run the full workspace suite before merge. This owner cadence supersedes older per-slice full-suite wording below. Keep required scoped gates and named Tier-A triggers. The twelve historical slices need one final-tree affected integration set, not twelve repeated runs. Preserve pinned historical controls and use the new main base separately for integration regression controls. No push/merge is authorized by this document.

# Review taint context: continuation of #602

Date: 2026-11-08. Implementation base: `4c8620a2b130be19f242ff19d657869ccd12364e`.
Owner instruction: start cross-file review taint; the controller architects and orchestrates, delegated agents implement.
Worktree: `/private/tmp/prism-crossfile-602`, branch `feat/review-crossfile-context`.
Status: S1 and S2a/S2b/S2c.1p have accepted local source and evidence custody. S2c.1 is accepted and committed locally at 6a7150cce941761c59974ccb1a6b96a40de3b89e: six files,791 changed lines, main941 weighted; both independent R1 reviews approve, canonical78 passed/3 intended failures to GREEN81/0, full workspace8867/0/23, complete RSC6/6,RBD8/8,RSB6/6 and scoped6/7 with inherited MO01 anchor refusal/no credit. Thirty-six green mutation baseline executions and26 actual mutant failures cover20 unique IDs. S2c.1 evidence custody is complete at package f96338f4/parent e76621b7; S2c.2 source is accepted and committed locally at 27806fbcfad605593580ef1389e07cdadf16815b: six files660changes, main962weighted, canonical83/3intended to86/0 with identical tests, both independentR1 approvals, full8871/0/23, scoped10/11 with inheritedMO01 uncredited and completeRCR6/RSC6/RBD8. Thirty-eight green mutation baselines and30failed mutant instances cover20unique IDs. S2c.2 local evidence custody is complete at package271929dd/parent3ece71e3, with228files4092257bytes,184sealed entries and54archive members; all old1507files remain unchanged. Layer-A performance and S3–S5 remain incomplete. Whole feature and performance remain incomplete.

## 1. Authority and retained work

This continues, rather than replaces, [the original #602 spec](https://github.com/shoedog/prism-evidence/blob/main/jsts-lane/602/spec/SPEC.md).
Retain its design, fixtures, falsifiers and measurement history. This addendum reconciles later decisions and the 4.0 crate split.
PR-0 source attribution and PR-1 import hints are on the assigned base. Do not repeat them.

Later decisions override the original draft:
- [M2 decision](https://github.com/shoedog/prism/issues/602#issuecomment-6565991615): B accepted with narrowed call-argument-driven loading, Go declaring files and depth two; no renewed pre-implementation M2 gate. Historical static-closure cost is not product timing.
- [Crossing decision](https://github.com/shoedog/prism/issues/602#issuecomment-6566291596): NameOnly crossings by default, Candidate evidence; retain explicit Exact mode.
- `src/review_context/import_targets.rs` defines best-effort loading hints, never proof. Notes say "likely in" or "one of", never assert a resolved target or sink.
- Current CLAUDE.md mutation rules apply, superseding the old spec's no-per-PR gate.
- The default remains diff-only with diagnostic boundary notes. Expansion is opt-in and stays unavailable until all exposed output formats are correct.

Scope: JS/JSX/TS/TSX, Python and Go. Keep #623 nav-index attribution, Rust/C/C++ expansion, package-entry resolution, general source/DFG repairs, and Tier-A gate redesign outside this lane.

## 2. Architecture

The core library remains independent of prism-review. Core owns captured repo/manifest inputs, import-hint resolution, graph traversal and optional traversal policy. Review owns taint roots, notes, expansion orchestration, result union and rendering.

Expose narrow additive core facade operations over the existing captured `ReviewInputs` and attribution machinery. Do not make the private module tree public wholesale. A call's hint is never converted to an edge or confidence grade.

Keep existing `review`, `run_review`, `run_algorithm`, `taint::slice` signatures. `TaintConfig` is currently exhaustive: preserve its fields and constructors. Use additive scoped/trace entry points or internal policy parameters, with legacy wrappers retaining default behavior. Put review-specific options on non-exhaustive `prism_review::api::AlgorithmParams`, not a core-to-review dependency.

CLI single/multi review and targets must share preparation/orchestration. Existing preloaded `run_review`/`run_algorithm` uses captured listings/manifest inputs and the originally captured repo root: no directory rewalk or caller-supplied alternate repository. The reused Go hint resolver may read candidate declaration text from the captured listing under that root; these are live advisory hints, not a snapshot guarantee. Its current `read_to_string` has no actual-byte cap after listing: the listing's 2MiB metadata filter is only a capture-time check. Earlier "bounded Go reads" wording refers to the listed candidate population and memoization, not a hard live-read byte guarantee. S4 targeted source admission must enforce its own actual-byte limits. An additive prepared/context-aware entry point accepts explicit loading authority/options when expansion is needed. Document whether old APIs return diagnostics only and reject/ignore unsupported expansion options visibly; never silently claim expansion occurred.

Round zero is the current diff-only input/CPG. Keep it intact for every non-taint algorithm and its cache. Context expansion builds a separate uncached taint CPG using captured source/manifest rules and additional files. Retain owned source/parse-quality maps until every serializer and target projection completes; no self-referential owner structure is needed.

## 3. Layer A: targeted diagnostic notes

Capture the taint run's seeds and reachable value locations in an additive trace result, retaining root identity/provenance for S3 rather than flattening all roots irreversibly. Intersect value-use occurrences with call argument byte ranges, then use existing import hints for calls without a loaded resolved callable. Do not approximate this as "any tainted line containing a call" or match the callee identifier as an argument. S1 leaves existing public outputs identical; S2 wires the diagnostic population into them.

Reuse source/owner/path identity when reconstructing occurrences. An ambiguous occurrence that cannot be attributed to the argument must not become a definite tainted-argument claim. Keep the inherited false negatives explicit rather than widening call resolution.

For loaded-callable suppression, retain original §1.1's site-specific `resolve_call_site_full(site).resolved` presence check against the loaded universe. This is a diagnostic presence check, not a request to add its candidates to taint edges. The non-navigation `resolve_call_site` wrapper removes multi-target `FuncValueField` results; the provisional S1b.3 draft's suggestion to substitute that wrapper is superseded. Existing CPG/traversal filtering stays unchanged. No equivalence between the two routes is assumed.

Typed notes include site byte identity, file/line, callee, ordered candidate paths, hint basis and kind. Deduplicate by site plus targets, deterministic sort, ten displayed notes plus one omitted-count summary. `Unresolvable` is visible; external/unknown calls are not in-repo boundary notes. Go uses declaring-file hints, not every file in its package.

A note claims possible unanalysed continuation, never a vulnerability or binding proof. Until B ships do not recommend a nonexistent flag. Notes never change findings, evidence, severity or exit codes. They are not filtered as findings by min-confidence.

Route notes through single/multi JSON and review warnings, located SARIF note notifications, targets warnings, and text stderr. Preserve non-note payloads. The old "A adds notes" and "off is byte-identical" wording is resolved as: finding/evidence/non-note payload identity, permitting the declared diagnostic additions. No unconditional warning on unaffected reviews.

## 4. Layer B: bounded expansion

Retain mode off/imports; crossing exact/nameonly (default nameonly); depth 1-4 (default 2); max files default 64; fixed 8 MiB context bytes; 20-second cooperative guard between bounded steps. Never promise interruption of an in-progress uninterruptible build. Every bound/skip is visible, and the last completed result remains usable.

Root/eligibility rule: A may diagnose the run's existing framework roots. B's loading population is a subset of the active run's roots: only enabled diff-line roots and explicit user-selected sources in the loaded diff universe; explicit-source mode retains the existing suppression of diff roots. Every later frontier must descend from these roots. A context file never supplies a fresh explicit, framework, IPC or inline-source root. An unrelated framework source merely present in a diff file does not independently expand B. This interprets the accepted "identifiers the diff touched" by path provenance, not same-name matching. Report explicit source requests outside the loaded universe instead of silently broadening it.

Keep two root populations distinct. Expanded analysis retains every originally enabled root in the original diff-file universe, including framework/IPC/inline roots, as the original spec's seed_files contract requires. The narrower diff/explicit population controls loading only. Do not silently restrict all final findings to the narrower population. Eligible-frontier discovery needs its own eligible-root trace or equivalent provenance-preserving reachability: a multi-source selected parent from an ineligible framework root must not mask another permitted root reaching the same value.

At each round follow eligible argument-bearing boundary calls using the captured import hints. Recheck against current loaded identities; cycles do not reload files. Follow barrels only through the existing bounded hop primitive. Reject paths outside the authorized repo/listing; reuse loading limits and manifest/source identity rules. Extra loaded candidates remain hints and can only produce findings through the actual graph.

For Go, B loads only `declaring` paths. Empty `declaring` means no expansion and an explicit unresolved/unsupported diagnostic, even when the hint's `files` contains the whole package. A may retain suitably qualified candidate hints. Tests cover empty, singleton and multiple declaring-file sets.

The optional Exact crossing gate belongs inside traversal, including return-flow traversal and synthesized routes that can cross files, not after witness selection. It gates edges into context files (including context-to-context), not same-file edges or original diff-to-diff routes. A permitted alternative path must remain discoverable when a refused path is encountered first. NameOnly crossings retain their actual labels and Candidate tier; never upgrade a hint.

Run baseline A once; preserve each baseline finding and aligned evidence unchanged. Add only context-related final findings under the inherited category/file/line union contract; suppress and count novel diff-only results caused by resolution perturbation. Reconcile blocks and diagnostics separately. Expanded source/AST/parse-quality maps must accompany context findings through JSON/review/text/SARIF/targets and filtering. Preserve the original spec's review-owned context bundle on ReviewRun; do not replace ReviewOutcome.inputs or add context to the original input maps. The bundle owns union maps, so no CPG borrow escapes its build/run scope.

A completed round is an atomic bundle: CPG/result, evidence, source/AST/parse-quality maps, admitted context set and next frontier. A guard firing during the next load/build retains the previous completed bundle plus a Bound diagnostic; it does not label incompletely analyzed files as analyzed or pair previous evidence with next-round maps. A deterministic injected-clock test covers interruption after loading and before completion.

No public B flag until minimal complete format fidelity is present. S4 can be an internal engine with tests; S5 exposes it. Default-off and mixed non-taint results remain invariant. Context source changes must be observed even on round-zero cache hits. No context cache in v1.

## 5. Bounded implementation and review

Two review rounds per artifact. At the cap classify: closed enumerable defects get a targeted fold; converging extension is disclosed; open-class findings return to design. No replacement/restart of the artifact without owner approval.

| Slice | Deliverable | Bound |
|---|---|---|
| S1 | Captured taint frontier, byte-qualified detector, typed notes, narrow core hint facade | No context loading, no CLI flags; internal/library tests |
| S2 | Notes wired into every current consumer and format; docs | Complete Layer A; findings/evidence invariant |
| S3 | Optional seed policy and in-traversal crossing policy | Legacy wrappers unchanged; direct graph tests |
| S4 | Internal expansion rounds, loading bounds, source custody and union | Not exposed publicly; production-like integration tests |
| S5 | Full CLI/facade/targets enablement, output fidelity and current-main measurement | Opt-in B; no unvalidated format |

Aim for <=600 weighted lines per new file and small independently reviewable diffs. Estimate each slice before implementation; split along a real responsibility boundary if it grows beyond a reviewable unit. Do not combine all five into a big-bang change.

## 6. Acceptance and evidence

Every behavior gets a RED on its exact assigned base plus positive and negative/edge controls. Diagnostic probes state predicted/falsifying outcomes beforehand. A failed probe is inadmissible, not a result. All failures attributed to the change require a same-environment base control.

Layer A matrix: JS CJS/ESM, TS aliases, TSX, Python relative/absolute imports, Go qualified/same-package; adjacent calls on one line; nested and multiline arguments; argument versus callee; shadowed import; already-loaded target; unknown/external; candidate/unresolvable wording; deterministic dedupe/cap; no non-taint notes.

B adds: depth/file/byte/time bounds; cycles/barrels; unsupported/unreadable/non-UTF8/oversized/bad-parse context; distinct roots and context-source leakage negatives; Exact/NameOnly alternate paths; return-flow; baseline union and evidence alignment; partial/cache-hit equivalence; mixed algorithm identity; all formats under nominal/scoped and confidence floor.

Before claiming implementation done: `cargo fmt --all --check`, full `cargo test --workspace` with totals, relevant file-size checks, scoped authoritative mutation gate plus new/substantively changed lane. Tier-A matrix and quick follow an immediate release build when core graph/AST/resolution/navigation changes trigger them. Record #762 refusals with exact-base controls instead of modifying/re-anchoring the harness in this feature lane. Full multi-corpus Tier-A stays owner-triggered.

Heavy builds run serially across this lane and honor shared host-quiet markers. No source mutation in main, sibling lanes or bench corpora. Use isolated scratch worktrees for historical corpus diffs. Controller performs git custody operations; no push/merge is included in current implementation dispatch.

Historical M2 numbers are INHERITED, not fresh proof. Recover immutable #602 raw evidence from the named release archive, verify its hash, and retain new candidate output separately. Reuse the 120-diff population where available for post-implementation yield/noise/timing; do not silently substitute a new corpus or revive the waived entry gate. Report useful findings separately from sink counts. No LLM-adoption rerun is required.

Current custody reconciliation: S2c.1 evidence package f96338f4cc8078ea69a50847b0e7cc96e218e476, parent reconciliation e76621b7772db7684ce9b0df200f207119b8bfd8, completes local custody:279files/4007626bytes,269payloads/10metadata,290origins,242sealed entries and25archive members; all committed blobs and1228older files verified. Thirteen packages total1507files/32254497bytes. The source review loop remains closedR1. Packaging cap1 corrected two generated documentation pointers/wording only; no source or runtime rerun. Subsequent S2c.2 source27806fbc and accepted docs2281c302 are custodied at package271929dd0ceb0fb770a8617c0f6cc0e8dd3ff6ef/parent3ece71e30e000a0aaf4448875681a89df8125768. Its228files4092257bytes bring fourteen packages to1735files36346754bytes; root verified all origins,184sealed entries,54archive members,nine patch-file reconstructions and committed blobs. Layer-A performance and S3–S5 are next. Historical S2c.1 counts above stay unchanged.

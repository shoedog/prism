# PR-B R8 — step-0 STOP, candidate remains parked

The R8 certificate is **NOT COMPLETE and cannot be claimed clean**. Step 0 falsified the proposed guard's full-coverage precondition: **15 of the 82 ADDED rows, in four exact synthetic owners, are neither subtree-recovered nor below an ERROR ancestor**. The guard would cover only 67/82 rows. As directed, source was not changed, no broader refusal was chosen, and steps 1–4 were not attempted. PR-B remains parked.

Written 2026-10-10T02:55:14.802661+00:00. All results below are static whole-file parser observations, not runtime or oracle correctness verdicts. R7's original verdict labels are retained for custody only.

## Checkout and input binding

[MEASURED] `plan/js-param-defs-prB`, docs HEAD `d587206c108bf49f9e049d830d65342bbea18dfc`; all 727 files under src/tests/mutants exactly equal source `2431cb104db7828d11ae409dc207c1e339270c93`, including five untracked files present in that commit. No missing, extra, or different path. Entry and final SHA manifests agree; Git HEAD and status are unchanged. See entry-binding.json and final-invariants.json.

[MEASURED] Original STOP packet: 88 rows = 82 ADDED + 6 LOST, copied without modification to R7-STOP-rows.json. All 82 ADDED rows map to 25 exact owners in 19 files. Each of those 19 files' SHA256 matches R7's per-root inputs.json before requests and after parsing. Exact ranges, owner names, all_functions membership, and Prism's existing dfg_owner_name were required to match each request. No owner was excluded and no row was omitted.

## Step 0: expectation, probe, complete coverage

Expectation written before parsing: all 82 owners' rows are covered by subtree ERROR/MISSING or an ERROR ancestor; a single neither owner falsifies the premise. Alternative: Flow syntax can produce a clean arrow subtree inside a file with parse errors elsewhere. This probe separates owner recovery from file recovery. Exact node identity and input hashes also exclude the alternative of attributing a nearby ERROR to the wrong owner.

Probe: step0_probe.rs, built against the existing Prism release library, uses `ParsedFile::parse(file, source, Language::JavaScript)` on each complete file; Prism's pinned grammar is used directly. It enumerates all subtree children, including unnamed MISSING nodes, and every ancestor through program. `has_error` agrees with the explicit descendant census for all 25 owners. No corpus source was executed. Exact build/run commands and linked library/source/request/result SHAs are in probe-binding.json; build and stderr logs are empty.

| Category | Rows | Distinct owners |
|---|---:|---:|
| Subtree ERROR/MISSING | 63 | 18 |
| ERROR ancestor only | 4 | 3 |
| Neither — STOP | 15 | 4 |
| Total | 82 | 25 |

Categories are mutually exclusive: subtree recovery takes precedence. Ten of the 18 subtree-recovered owners (33 rows) also have ERROR ancestors; these are not double-counted. Coverage is 67 rows/21 owners. All 19 files have parse errors; that does not establish callable recovery.

Complete per-row table: step0-row-table.json (82 original rows plus classification). Complete per-owner table: step0-owner-table.tsv (25 owners). Raw source text, full parse shape, every recovery node, and every ancestor: step0-facts.jsonl. Full four-owner details: step0-neither-owners.json.

## Precise uncovered owners — complete STOP population

| File | Owner | Byte range (half-open) | Rows | Parsed parameter | Ancestors |
|---|---|---|---:|---|---:|
| Libraries/Lists/VirtualizedList.js | `<cb@724:8>` | 24042..25219 | 3 | `ChildListState` (`identifier`) | 95 |
| Libraries/Lists/VirtualizedSectionList.js | `<cb@310:46>` | 9455..10054 | 6 | `ViewToken` (`identifier`) | 3 |
| Libraries/Pressability/Pressability.js | `<cb@530:42>` | 17995..18347 | 4 | `boolean` (`identifier`) | 3 |
| Libraries/Text/Text.js | `<cb@205:42>` | 6339..6665 | 2 | `boolean` (`identifier`) | 3 |

All four are `arrow_function` nodes with a single `parameter: identifier` and a `body: statement_block`. Each has `has_error=false`, zero ERROR/MISSING descendants, and zero ERROR ancestors. The parsed parameter texts are Flow type names or a Flow return-type spelling, as shown above. Full source and S-expressions remain in step0-neither-owners.json; no inference from an external oracle is needed for these parser observations.

VirtualizedList's immediate parent is call_expression; its 95-node ancestor chain includes formal_parameters/rest_pattern and an outer arrow starting line 718. Those surrounding nodes can have `has_error=true` without being ERROR nodes. VirtualizedSectionList has expression_statement → labeled_statement → program. Pressability and Text have sequence_expression → expression_statement → program. No node in these chains is ERROR. Extending the guard to any ancestor with has_error, or to the whole file, would be a different refusal rule; no such choice was authorized or made.

## Guard and existing predicate

No guard implemented or adopted. Existing `contains_recovery` is a nested helper in `src/ast.rs` in function_local_value_bindings (around line 4853), detecting subtree ERROR/MISSING. Its discovery is source inspection only; it was neither reused nor extended because step 0 failed. Named and synthetic passes both retain the exact R7 source bytes.

## Later measurements and gates — explicitly not run

| Corpus | Step-2 removed rows / refused owners / R7 CORRECT removed | Admissibility rule and row counts | R8 certificate admitted / excluded | ADDED C/W/U/I | LOST C/W/U/I | RELABELLED C/W/U/I | RE-OWNED C/W/U/I |
|---|---|---|---|---|---|---|---|
| X | Not run | Not evaluated | Not evaluated (1 planned root) | Unmeasured | Unmeasured | Unmeasured | Unmeasured |
| Xi | Not run | Not evaluated | Not evaluated (1 planned root) | Unmeasured | Unmeasured | Unmeasured | Unmeasured |
| T | Not run | Not evaluated | Not evaluated (1 planned root) | Unmeasured | Unmeasured | Unmeasured | Unmeasured |
| SecBench | Not run | Not evaluated | Not evaluated (583 planned roots) | Unmeasured | Unmeasured | Unmeasured | Unmeasured |

C/W/U/I means CORRECT/WRONG/UNDECIDED/INADMISSIBLE. There are no R8 byte-table verdicts or zero claims. The 6 LOST rows were not re-adjudicated; no oracle-inadmissibility rule was adopted. Historical R7 certificate/tables/gates remain historical evidence, not R8 verification.

- Identity, navigation and removed-WRONG proofs, O1 standalone/joint: not run.
- Matrix and guard mutant: not added or run; no R8 DESIGN-CHANGE count claimed.
- Full nextest with mcp, doctests, mutants, fmt, clippy parity, Tier-A matrix, TS and Node quick: not run (step-0 STOP). No source implementation was performed and no full-suite completion claim is made.
- Frozen R8 product binaries: not built. The evidence-only step0-probe SHA256 is `6ffdb001dc51d722c23e71a05a2fc506e8554635512e1dd167e036a8cea6c510`; it is not a SUT candidate.
- R8-src.patch and R8-docs.patch: not generated; source and repository docs are unchanged.
- CONTROLLER-pd.sh / F companion: not changed. No F guard-cost or main-vs-R8 command exists because no R8 head exists. Do not run/adopt R8 on F.
- No Git writes, network access, browser, corpus-package execution, frontend-portal access, or delegation.

## Delivery, custody, and controller action

REPORT.md and HANDOFF.md are final for this STOP. PROBE-LOG.md contains the expectation/alternative/result and final custody audit. STOP.json and step0-summary.json are machine-readable. The whole packet is sealed in final-checkpoint.tgz; final-checkpoint.json gives its SHA and exact member hashes. Source custody is the unchanged committed R7 reference plus entry-binding.json; earlier R7 evidence was not overwritten.

Suggested evidence/docs custody commit message for the controller: `docs: record PR-B R8 parse-recovery coverage STOP`. No source commit is proposed and the worker made no commits. Controller decision is required before any further repair or expanded refusal; no automatic retry, restart, adoption, or merge is authorized by this stopped packet.

---

## Controller note (2026-10-09): PR-B stays parked

The owner's condition for R8 was: if the parse-recovery guard does not cover the rows, PR-B stays parked. Step 0 shows it covers 67 of 82. No source changed; the working tree equals `origin/wip/js-param-defs-prB-r3` at `2431cb10`.

**Why a per-callable guard cannot work here.** In the four uncovered owners, tree-sitter reads a Flow return-type annotation as a syntactically clean bare-arrow function whose only parameter is the type name (`boolean`, `ViewToken`, `ChildListState`). The parse error is a sibling, not an ancestor or a descendant.

**What would cover all 82, if the owner reopens this:** a *file-level* rule — no synthetic passes in any file where the parser reports an error. All 22 affected files are Flow-annotated and have parse errors elsewhere. That is coarser, so its yield cost on X, T and F would have to be measured first; it has not been.

**State of the candidate:** certificate against `f8c768b3` complete and clean on 579 admitted roots (R7); matrix 763 + 200 cells; all gates green. The only open defect class is rows emitted from parser-misread Flow syntax in one package.

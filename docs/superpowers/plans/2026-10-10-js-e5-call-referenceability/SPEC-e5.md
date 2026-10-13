# E5 draft — owner-selected union, stopped prototype

Status: **STOPPED / INCOMPLETE; not ready for review, adoption or landing.** Step-1 entry and patch base is `d01fd65fd0c821bb54887d9c48b4519d96f49776`, branch `plan/js-e5-call-referenceability`; product main is `bab0dfbd`. `/Users/wesleyjinks/prism-evidence/js-e5/plan-brief-step1.md` supersedes the narrow step-0 recommendation. The verbatim prior spec is preserved in step1/entry-SPEC-e5.md.

## Goal and owner decision

Reduce false Exact cross-file FreeSingle calls using only the caller's own syntax and the target's declaration syntax. Exact is a static binding by declaration, scope, import and export syntax; a local alias or parameter sharing a callable's spelling elsewhere is not that proof. The owner chose **arm (a) union R-bound (b), with drop**, after the narrow rule removed no target on private F. The narrow rule alone is no longer recommended.

The first complete union test suite found an unrelated MCP readiness failure, invoking the explicit STOP. Full details and source-bound receipts are in [REPORT-step1.md](REPORT-step1.md). The uncommitted prototype is retained; no adoption claim is made.

## Rule of record: (a) or (b), drop

At both existing FreeSingle arms, refuse a JS/TS/TSX cross-file target when either predicate applies:

**(a), unchanged step-0 arm:** ordinary Source call, non-JSX, unqualified, caller and target differ, own-file binder is `Unproven("unbound")`, and target is in the negative syntax set. That set contains declarations in files with root import/export syntax, methods, and unnamed function values inferred from object pairs, class fields, member/subscript assignments or variable-initializer wrapper calls. Explicit function-expression self-names and direct variable-to-function bindings are not inferred solely by position.

**(b), R-bound:** caller is an ES module by the product parser's root import/export statements, the singleton is cross-file and unqualified, and the calling file has no syntactic import or require evidence for that name. This includes module-caller alias/MayCall/position/unbound and JSX paths. Own-file evidence is extracted conservatively by the existing `ParsedFile::extract_imports`; scalar, destructured, renamed and nested requires can exempt the name. It only exempts refusal, and does not prove binding to the target. Qualified calls and earlier positive ladder rungs retain their behavior. No repo-wide import resolution or tsconfig parity is introduced.

Graph-target resolution declines the refused singleton so the ordinary ladder decides; final R5 drops it with LocalBindingUnproven. Full/skeleton/direct-subset constructors retain caller facts; remove/merge replace them and serialization persists them. CACHE_VERSION stays 115; both mutant cache pins stay 115; navigation edge cache remains 62. All five new regression groups pass, but real corpus validation and new guard mutants remain incomplete.

## Measured evidence and its limits

The following are **historical step-0 real-build measurements**, not sums, projections or measurements of the current union. [MEASUREMENTS.md](MEASUREMENTS.md), [CENSUS.md](CENSUS.md), and their JSONL receipts retain full rows and exclusions.

| Corpus | Step0 arm (a): W / C / U / I removed | Step0 experimental R-bound: W / C / U / I removed | Current union real capture |
|---|---|---|---|
| X | 0 / 0 / 0 / 1 | 27 / 0 / 34 / 37 | NOT RUN: STOP |
| T | 53 / 0 / 0 / 2 | 201 / 0 / 0 / 2 | NOT RUN: STOP |
| SB579 | 126 / 0 / 0 / 325 | 48 / 0 / 0 / 215 | NOT RUN: STOP |
| Controller F | 0 / 0 / 0 / 0 | census price only: 109 W, 0 C, 4 U, 0 I | Controller pending |

F's supplied step0 base and narrow head both retain 113 Exact FreeSingle targets, 109 WRONG and four UNDECIDED; sites and graph bytes are unchanged. The planner has no private-row access. The union's (a)-only/(b)-only/both and JSX/ordinary splits, every changed target/site/graph adjudication, inventory and non-JS/non-call parity are pending. Its UNDECIDED removals must each be described and up to ten public samples disclosed; none are claimed measured now. Do not transfer step0 zero cost/parity into union acceptance.

Pinned oracle remains TypeScript 5.9.3, static allowJs/checkJs, ESNext/Bundler, preserved JSX, no ambient package types and no emit. Identity binds physical callable bytes. Signature-only identity is UNDECIDED; CORRECT requires value-declaration identity or the admitted literal unshadowed scalar-require/physical-callable proof. Native failures remain INADMISSIBLE. Raw and after-override are equal, zero overrides. Reused base binaries were built from 40ad8390 whose product source equals bab0dfbd according to the controller; all binary hashes were freshly checked. Four historical excluded roots remain excluded with their original reasons and timings, not zero-cost observations.

## Deliberate test expectation changes and fixture dependencies

**No existing test expectation was changed in Step 1 before STOP.** The full first-suite population is in REPORT-step1.md. The prospective alias-policy expectation change is `js_binding_shorthand_test::b5_nested_declarators_keep_base`: module callers without import/require evidence must drop the formerly Exact nested alias/var fallback relationships; imported scope controls retain complete values. Three other failures are fixture dependencies, not proposed assertion weakening: `td_duplicate_for_heads_negative_control_single_head_binds` and `rep_unbound_name_through_an_unrelated_parameter_keeps_base_at_r5` need script callers for script-global scope controls; `c41_with_reason_keeps_base_at_r5` needs script caller and target to retain its pre-existing reason-scoped guard control. These changes are NOT IMPLEMENTED because STOP preceded repairs. The MCP readiness failure is outside both categories. The explicitly excluded parameter-slot example remains unchanged.

## R-local pricing and recommendation

| Predicate (census only) | Corpus | WRONG yield | CORRECT cost | UNDECIDED | INADMISSIBLE | Scalar-require CORRECT costs |
|---|---|---:|---:|---:|---:|---:|
| R-local | X | 19 | 0 | 11 | 0 | 0 |
| R-local | T | 148 | 0 | 0 | 0 | 0 |
| R-local | SB | 1249 | 10 | 17 | 24 | 10 |
| R-local-no-require | X | 19 | 0 | 11 | 0 | 0 |
| R-local-no-require | T | 148 | 0 | 0 | 0 | 0 |
| R-local-no-require | SB | 449 | 0 | 2 | 24 | 0 |

Scalar require is `local_declaration` in the historical census axis, so naive R-local includes ten CORRECT costs, all literal scalar requires in SB. Exempting every require-initialized local yields zero known CORRECT cost and keeps 19/148/449 WRONG (616 total), with 13 UNDECIDED and 24 INADMISSIBLE exposures. This broad exemption sacrifices 800 WRONG and 15 UNDECIDED SB rows. These are whole-base census prices, not real or post-union marginal deltas. Controller summaries publish both predicates for F pricing.

**Make R-local a follow-up.** Do not broaden this stopped union. The no-require predicate needs a separate bounded runtime measurement and regression matrix; the naive census axis violates zero-cost acceptance.

## Deliberately uncovered behavior and D12

Keep qualified calls, same-file targets, script-global declarations called from scripts, proven imports/requires, earlier positive rungs, non-JS output, bare/rest incoming slots and callback/synthetic owner identities. Arm (a)'s Source boundary is unchanged; arm (b) deliberately overrides the earlier alias/MayCall/position/JSX fallback policy only for module callers without name evidence. CommonJS local declarations outside R-bound are not added to the rule; R-local is priced only. No runtime corpus execution, global binding graph, module-path proof or TypeScript-resolution parity is introduced.

**D12 remains unchanged.** It continues refusing outgoing new-formal paths reaching every cross-file Exact FreeSingle argument edge. The union has not been measured on remaining argument reachability and cannot justify removing the blanket protection. Any selective refusal or incoming bare-formal repair requires a separately authorized increment.

## Acceptance criteria

- Zero CORRECT Exact resolutions lost on every admitted public corpus and controller F; any loss invokes STOP.
- Zero new WRONG row, target, site or topology; every differing site and graph row classed and source-bound adjudicated.
- Non-JS call rows and non-call graph digests byte-identical; symbol/file inventories identical; only call-derived navigation and dependencies may change and their counts must be disclosed.
- Every actual UNDECIDED removal described, up to ten public samples, with no hidden yield credit. INADMISSIBLE deltas and producer exclusions remain explicit.
- New ordinary and JSX refusal paths each main-RED/head-GREEN with preserving import/require/same-file/script/qualified/non-JS controls; caller-fact lifecycle, serialization and consumers tested. Add and kill scoped predicate mutants.
- cargo fmt --check, full cargo nextest run --features mcp with totals, doctests, scoped mutation, fresh release, cache-bound Tier-A matrix and TS/JS quick with same-worktree fresh base control; excluded parameter-slot example separately reported and not repaired. Rust/full multi-corpus boundaries unchanged.
- Controller actual-shell parity passes on frozen step1 heads before F acceptance. Review is controller-dispatched Opus and gpt-6.1-sol, at most two rounds, after the stopped lane is reopened and candidate evidence is complete.

## Owner decisions

| Decision | Current authority / disposition |
|---|---|
| Breadth | Owner selected arm (a) union R-bound (b); narrow-only recommendation superseded. Prototype retained at STOP, not adopted. |
| Refusal | Owner selected drop. Step0 NameOnly retained known WRONG argument relationships. |
| JSX | Part of selected R-bound; actual ordinary/JSX split pending. Owner may cut JSX after real measurements. |
| R-local | Recommend follow-up with explicit require exemptions and separate real measurement. |
| Private acceptance | Controller runs provisional CONTROLLER.md diff after reopening and completing public gates; zero public loss alone cannot establish F acceptance. |
| D12 | Leave unchanged; no E5 authority to narrow or remove it. |

## STOP and custody

STOP is the existing MCP readiness test outside alias-policy/repairable call-binding-fixture purpose. Cause is unresolved; no regression attribution or flakiness exoneration is made. No product repair or MCP retry followed it. The later user-hook synthetic-helper verification extension is recorded below. Current head suite: 5298 run, 5292 pass, six failures, one reserved skip. Raw failures include the one explicitly excluded inherited example. Pre-union control: 5293 run, 5292 pass, one excluded failure, one skip. Release completed from this worktree and immutable heads are under step1/bin with SHA256 binding. Remaining gates and paired union captures were not run.

Source/packet changes stay in the working tree. Relative-d01fd delivery patches and snapshots live under /Users/wesleyjinks/prism-evidence/js-e5/step1/. Step0 evidence is read-only. No Git write, corpus execution, network/provider call, private directory access or independent review occurred. Controller owns committing and any reopening/adoption decision.

## User verification-hook extension

The subsequent user verification hook (`stop:5:/Users/wesleyjinks/.codex/hooks.json`) explicitly requested missing tests and root `## Verified` / `## Not verified` sections. One verification extension added two synthetic native-oracle groups without changing product code or existing expectations. Both fail on the exact archived d01fd65f helper (0 pass / 2 fail); the complete current native-oracle suite passes 13/13. Controls include every require initializer shape, variable/parameter/binding-element/function/class locals, unbound/import/destructured require, both union arms, overlap and script/import negatives. All 734 product files and frozen binary hashes remain unchanged, so Rust totals and the readiness STOP stand. Controller parity, main runtime REDs, remaining product gates and real union corpus captures stay unrun. The root verification file is ignored by Git and is explicitly included as a new file in the refreshed docs patch. Evidence is step1/verification-hook/verification-result.json, base-oracle-red-final.tap and head-oracle.tap.

# E5 — owner-selected union, measured prototype

**Status: PLANNER COMPLETE / READY FOR CONTROLLER F AND SPEC REVIEW.** Entry/delivery patch base d994c75d; main product bab0dfbd; union source preserved at e164178b. Adoption remains pending controller F and independent reviews (Opus and gpt-6.1-sol, cap two rounds). The former readiness STOP is resolved by the step1b controller ruling and a serial head PASS.

## Goal and owner decisions

Reduce false Exact cross-file FreeSingle calls using only caller-file syntax and target declaration syntax. Exact is a static binding by declaration, scope, import and export syntax. Owner chose unchanged step0 arm(a) union R-bound arm(b), with drop, after arm(a) removed no target on F. Narrow-only recommendation is superseded. JSX remains included, with its measured split below; the owner can cut it separately. R-local is an accepted follow-up; D12 stays unchanged.

## Rule of record: (a) or (b), drop

At both existing FreeSingle arms, refuse a JS/TS/TSX cross-file target when either predicate applies:

**(a), unchanged step-0 arm:** ordinary Source call, non-JSX, unqualified, caller and target differ, own-file binder is `Unproven("unbound")`, and target is in the negative syntax set. That set contains declarations in files with root import/export syntax, methods, and unnamed function values inferred from object pairs, class fields, member/subscript assignments or variable-initializer wrapper calls. Explicit function-expression self-names and direct variable-to-function bindings are not inferred solely by position.

**(b), R-bound:** caller is an ES module by the product parser's root import/export statements, the singleton is cross-file and its product CallSite.qualifier is None (including indirect sites), and the calling file has no syntactic import or require evidence for that name. This includes module-caller alias/MayCall/position/unbound and JSX paths. Own-file evidence is extracted conservatively by the existing `ParsedFile::extract_imports`; scalar, destructured, renamed and nested requires can exempt the name. It only exempts refusal, and does not prove binding to the target. Call sites with a retained product qualifier and earlier positive ladder rungs retain behavior. Physical source spans for some qualifier-less fallback/indirect records are qualified; they are disclosed separately below. No repo-wide import resolution or tsconfig parity is introduced.

Graph-target resolution declines the refused singleton so the ordinary ladder decides; final R5 drops it with LocalBindingUnproven. Full/skeleton/direct-subset constructors retain caller facts; remove/merge replace them and serialization persists them. CACHE_VERSION stays 115; both mutant cache pins stay 115; navigation edge cache remains 62. All ten E5 regression groups pass in the full suite; seven refusal/lifecycle/consumer groups are main-RED. Public capture and four scoped guard mutants are complete.


## Real union measurements

| Corpus | WRONG removed | CORRECT lost | UNDECIDED removed | INADMISSIBLE removed |
|---|---|---|---|---|
| X | 27 | 0 | 34 | 37 |
| T | 201 | 0 | 0 | 2 |
| SB | 171 | 0 | 0 | 418 |

| Corpus | Arm | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | a_only | 0 | 0 | 0 | 0 |
| X | b_only | 27 | 0 | 34 | 36 |
| X | both | 0 | 0 | 0 | 1 |
| T | a_only | 0 | 0 | 0 | 0 |
| T | b_only | 148 | 0 | 0 | 0 |
| T | both | 53 | 0 | 0 | 2 |
| SB | a_only | 123 | 0 | 0 | 203 |
| SB | b_only | 45 | 0 | 0 | 93 |
| SB | both | 3 | 0 | 0 | 122 |

| Corpus | Arm(b) invocation | WRONG | CORRECT | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|---|
| X | ordinary | 14 | 0 | 11 | 1 |
| X | jsx | 13 | 0 | 23 | 35 |
| X | qualified | 0 | 0 | 0 | 1 |
| T | ordinary | 201 | 0 | 0 | 2 |
| T | jsx | 0 | 0 | 0 | 0 |
| T | qualified | 0 | 0 | 0 | 0 |
| SB | ordinary | 47 | 0 | 0 | 123 |
| SB | jsx | 0 | 0 | 0 | 5 |
| SB | qualified | 1 | 0 | 0 | 87 |

Physical JSX is classified by the native invocation node, not the internal jsx flag:66 JSX spans are carried by IndirectResolution records with jsx=false. The table separately discloses89 qualified physical spans (1 W/88 I);66 are indirect and23 Source records have no retained product qualifier. The unchanged real R-bound guard acts on qualifier-less product call sites; no blanket physical-qualified preservation claim is made. Every ordinary row in this measured b population has Source origin: the real R-bound-source subset is X14 W/0 C/11 U/1 I, T201 W/0 C/0 U/2 I, SB47 W/0 C/0 U/123 I. R-bound minus R-bound-source therefore includes both physical JSX and qualified fallback records.

All581 call-site and graph comparisons are admitted. Every differing call site (890) and graph row (2208) is classed and adjudicated in `union-call-site-adjudications.jsonl` / each graph.adjudicated.jsonl. Added targets/sites/graph rows:0/0/0. All43953 original serialized non-JS call rows are byte-identical; all581 non-call digests match. All581 symbol/file inventories match; 225 call-derived module dependencies are removed. Sampled caller/callee queries cover affected roots; aggregate deltas are `{"callees": 27, "callees_unresolved_placeholders": 27, "callers": 148}`. Unresolved source placeholders are disclosed separately from resolved relationships. Full receipts retain every inventory/query command and any reused base artifact.

Every one of the 34 UNDECIDED removals is listed with its local binding, oracle reason and up to ten public samples in [UNDECIDED-step1.md](UNDECIDED-step1.md). U/I are exposure, not yield. Zero classified CORRECT loss does not certify unknown oracle cost. [REPORT-step1.md](REPORT-step1.md) contains every graph-kind/verdict count and gate command; [MEASUREMENTS.md](MEASUREMENTS.md) retains historical experiments.

Controller F historical base/narrow head:113 retained,109 W/0 C/4 U/0 I; arm(a) removes0. Historical R-bound census price109 W/4 U. Union F real measurement is pending and is never inferred from those prices.

## Existing fixtures and deliberate expectations

| Existing test | Fixture edit | Purpose retained |
|---|---|---|
| `td_duplicate_for_heads_negative_control_single_head_binds` | Caller `export function run` → script `function run`; target was already a script | `L2 f: Exact free_single f@1-1` unchanged; negative duplicate/for-head scope control remains reachable outside module refusal. |
| `rep_unbound_name_through_an_unrelated_parameter_keeps_base_at_r5` | Caller `export function run` → script `function run`; helper/target unchanged scripts | `L5 f: Exact free_single f@1-3` unchanged; unrelated helper parameter remains outside caller scope. |
| `c41_with_reason_keeps_base_at_r5` | Caller and target declarations lose `export` | `L3 f: Exact free_single f@1-3` unchanged; `Unproven("with")` still proves the older R5 drop is reason-scoped. |

| b5 scenario (both `.jsx` / `.tsx`) | Deliberately changed result | Reason |
|---|---|---|
| Bare nested `const { f }` block, L4 | `Exact free_single lib:f@1-3` → `drop LocalBindingUnproven` | Module caller has no import/require evidence, even for in-scope alias. |
| Nested `if` / `let { f }`, L4 | Same Exact → drop | Same module-caller alias policy decision. |
| Nested `var { f }` block, L4 | Same Exact → drop | Union overrides unsupported nested var alias fallback. |
| Nested `var { f }` block, L6 | Same Exact → drop | Union overrides unsupported hoisted var alias fallback. |
| Same-line nested const block, first L2 call | Same Exact → drop | Module alias lacks import/require evidence; the second L2 call was already drop. |
| Module-scope nested var block, L5 | Same Exact → drop | Module-level var fallback lacks import/require evidence. |

All already-refused expectations stay unchanged. Imported b5 controls stay at complete values `L5 f: Exact import_member lib:f@1-3` and `L7 f: Exact import_member lib:f@1-3` in both grammars. Six scenario values change (twelve across the two grammars); the bare/if cases share one expectation vector. No assertion is weakened.

## R-local price and accepted disposition

| Predicate (base census only) | Corpus | WRONG | CORRECT | UNDECIDED | INADMISSIBLE | Literal scalar-require C costs |
|---|---|---|---|---|---|---|
| R-local | X | 19 | 0 | 11 | 0 | 0 |
| R-local | T | 148 | 0 | 0 | 0 | 0 |
| R-local | SB | 1249 | 10 | 17 | 24 | 10 |
| R-local-no-require | X | 19 | 0 | 11 | 0 | 0 |
| R-local-no-require | T | 148 | 0 | 0 | 0 | 0 |
| R-local-no-require | SB | 449 | 0 | 2 | 24 | 0 |

**Accepted disposition: follow-up, outside this slice.** Naive axis-based R-local costs ten CORRECT literal scalar-require bindings. Exempting every require-initialized local has zero classified CORRECT cost and retains 616 WRONG (19/148/449), plus 13 UNDECIDED and 24 INADMISSIBLE exposures. These are whole-base census prices, not marginal post-union yield. Both predicates remain in controller summaries; no runtime R-local work was added in Step1b.

## Deliberately uncovered behavior and D12

Keep call sites with a retained qualifier, earlier proven qualified rungs, same-file targets, script-global declarations called from scripts, syntactic import/require exemptions, earlier positive rungs, non-JS behavior, callback identities and incoming formal slots. CommonJS local aliases outside R-bound remain out of scope. No repo-wide import resolution, tsconfig parity or runtime package execution. Arm(a) Source/non-JSX boundary is unchanged; arm(b) deliberately overrides unsupported alias/MayCall/position/JSX FreeSingle fallbacks only for module callers without name evidence.

**D12 remains unchanged by owner instruction.** It still refuses outgoing new-formal paths reaching every cross-file Exact FreeSingle argument edge. Any narrower D12/incoming repair needs a separately authorized increment.

## Acceptance criteria and gates

- Zero CORRECT Exact loss on each admitted public corpus and controller F; any loss invokes STOP. Public criterion passed; F pending.
- Zero new WRONG/target/site/topology; every differing call site and graph row adjudicated. Public added counts all0.
- Non-JS call bytes and non-call graph digests identical; symbol/file inventories identical; disclose call-derived navigation/dependency changes. All581 public roots passed.
- Every UNDECIDED removal disclosed; preserve INADMISSIBLE exposures and four producer exclusions without yield or zero-cost credit.
- Main-RED/head-GREEN refusal paths and full-value negative controls; current scoped guard mutants4/4 killed/admissible.
- Full serial suite, fmt, doctests, fresh release, adapted matrix and same-worktree base/head TS/JS quick complete with excluded example reported. Rust/full Tier-A boundaries remain explicit.
- Actual-shell controller parity14/14 before F; private zero cost and independent reviews remain gates before adoption.

## Custody and limits

Entry and patch base d994c75d; preserved union source e164178b. Product src/vendor bytes remain identical to the frozen union build; Step1b changes four integration fixtures/expectations and the lane registry, plus measurement/packet tools. CACHE_VERSION and both pins stay115; D12 and navigation cache62 are unchanged. Public and controller captures use original frozen step1/bin/head-prism, head-facts, head-graph; hashes are verified in binary-binding.json and step1b-frozen-head-reuse.json. Base calls/facts/graphs and admitted matching-command base navigation artifacts reuse read-only step0 captures. Base binaries40ad8390 equal main bab0dfbd product source: supplied controller control and the freshly verified empty full Git diff across src/Cargo/build/vendor (base-main-product-equality-step1b.json). New module/byte helpers and gate binary are frozen in step1b-helper-source-binding.json. Fresh quick builds exact main and restored head in this same worktree, immediately before each quick; every modified source byte is restored by hash. No product source changed, so original public/controller head binaries require no re-freeze.

Private F union capture; controller Opus/gpt-6.1-sol spec review (cap two rounds); adoption/landing/Git custody; R-local runtime; D12 narrowing or incoming-formal repair; the four excluded SecBench roots. Full human-triggered Tier-A multi-corpus and Rust quick were not run. Rust quick is excluded because rust-analyzer build scripts/proc macros can execute corpus code and write outside granted roots. No provider/network calls, corpus package execution, Git writes or directory named frontend-portal were used.

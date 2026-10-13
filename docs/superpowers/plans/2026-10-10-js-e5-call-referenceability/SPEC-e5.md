# E5 — owner-selected union, measured prototype

**Status: STEP 2B — R1 trivia class closed; new-head gates and581-root identity complete (2026-10-13).** Patch base 3df0ddfa; frozen main product bab0dfbd. Adoption remains pending controller F and independent reviews (Opus and gpt-6.1-sol, cap two rounds; confirming round 2/2). Current evidence: REPORT-step2b.md and HANDOFF-step2b.md. Zero head-to-head differing call/graph rows on X,T and all579 admitted SecBench roots carries step2 tables/adjudications unchanged. Earlier step2 gate receipts are historical.

## Goal and owner decisions

Reduce false Exact cross-file FreeSingle calls using only caller-file syntax and target declaration syntax. Exact is a static binding by declaration, scope, import and export syntax. Owner chose unchanged step0 arm(a) union R-bound arm(b), with drop, after arm(a) removed no target on F. Narrow-only recommendation is superseded. Source-origin JSX and super(...) stay subject to arm (b); IndirectResolution records retain main behaviour under the controller R3 ruling. R-local is an accepted follow-up; D12 stays unchanged.

## Rule of record: (a) or (b), drop

R1 terminal names come from the entity's terminal identifier/property field, ignoring comments and whitespace. Import-alias locals, direct awaited-import operands, and recovered exported-require arguments also ignore comment children. The binder reads the actual import-alias local identifier so trivia cannot falsely arm (a) as an unbound call. Both refusal predicates and all rulings are unchanged. The former terminal-trivia defect is fixed; its differing-terminal twin stays refused. The complete syntax audit and fixtures are in CLASS-AUDIT-step2b.md.

At the final unqualified R5 singleton rung, after same-file R4 returns, refuse a JS/TS/TSX caller’s cross-file target when either predicate applies. The separate graph-target FreeSingle path is Rust-only; its E5 guard was dead and has been removed (R6).

**(a), unchanged step-0 arm:** ordinary Source call, non-JSX, unqualified, caller and target differ, own-file binder is `Unproven("unbound")`, and target is in the negative syntax set. That set contains declarations in files with root import/export syntax, methods, and unnamed function values inferred from object pairs, class fields, member/subscript assignments or variable-initializer wrapper calls. Explicit function-expression self-names and direct variable-to-function bindings are not inferred solely by position.

**(b), R-bound:** the site has Source origin, caller is an ES module by root import/export statements, and the calling file has no syntactic import evidence for this name. An own-file `Unproven("unbound")` site with a JS/TS/TSX target outside `js_ts_free_single_refusals` is exempt from arm (b), so arm (a) alone decides it (R2). Local bindings and non-JS targets do not get that exception. This conservatively keeps module-to-script globals as main does, including the remaining limits below.

Evidence starts with `ParsedFile::extract_imports` (scalar, destructured, renamed and nested requires), plus E5-only collection of TypeScript import-require-equals locals, including `export import`; namespace import aliases whose right-most name equals the local; and names destructured from awaited dynamic `import()`. The vendored grammar recovers exported import-require-equals as an import_alias with a missing semicolon followed by a parenthesized string: E5 recognizes only that external-import shape. Other renamed namespace aliases remain refused, including the two `export import format = ts.getPathFromPathComponents` rows. This is exemption evidence, never positive target identity. The shared import extractor is unchanged. Both target and caller fact builders share one is-module helper in `call_graph_js_referenceability.rs`.

R1 deliberately does not cover `(await import(...))`, `await (import(...))`, `.then(...)`, or `as`/angle-bracket type assertions around the awaited import. Exported-require recovery requires one string semantic child and does not peel nested parentheses or argument assertions. These are explicit negative fixtures; wrapper syntax is not trivia.

Indirect records retain main behaviour. Source JSX aliases stay refused; Source `super(...)` remains subject to the same arm. Earlier positive import/local/qualified rungs retain their behaviour. No repo-wide import resolution or tsconfig parity is introduced.

Current step2b gates:5311 run/5310 pass/1 inherited example failure/1 skip; doctests2/2; scoped authoritative mutations19/19 killed/admissible; matrix178/178; valid identical same-worktree TS/JS quick; controller tests14/14;162 four-way fixtures/648 invocations. Exact commands and limits: REPORT-step2b.md. Historical step2 gate receipts below are retained separately.

Final R5 drops a refused singleton with LocalBindingUnproven. Full/skeleton/direct-subset constructors retain both facts; remove/merge replace them and serialization persists them. CACHE_VERSION and both registry pins stay 115; navigation edge cache stays 62. Guard controls and D12 controls pass; all19 scoped authoritative E5+PD-29 mutants are killed/admissible. Earlier full suite:5309 run,5308 pass,1 inherited example failure,1 skip; hook rerun:5307 pass,2 failures (example plus unattributed MCP timing),1 skip; doctests2/2; matrix178/178; fresh same-worktree TS/JS quick passes. Step1 receipts below are historical.



## Current union measurements — step2 tables carried through step2b identity

| Corpus | Prediction W/C/U/I | Measured W/C/U/I |
|---|---|---|
| X | 19 / 0 / 11 / 1 | 19 / 0 / 11 / 1 |
| T | 201 / 0 / 0 / 2 | 201 / 0 / 0 / 2 |
| SB | 170 / 0 / 0 / 354 | 170 / 0 / 0 / 354 |

All prediction columns match. Step1-to-step2 differences: 132 fully enumerated rows, all explained by the controller rulings ({"SB":{"R3|INADMISSIBLE":64,"R3|WRONG":1},"X":{"R3|INADMISSIBLE":36,"R3|UNDECIDED":23,"R3|WRONG":8}}). Nothing added; 43953 non-JS call rows byte-identical; all581 non-call digests and inventories identical. 758 target / 758 call-site / 1938 graph-row adjudications complete. Full arm, origin/physical, graph and navigation tables: REPORT-step2.md / MEASUREMENTS-step2.json. Every U removal: UNDECIDED-step2.md. All93 required three-way fixtures pass their designated controls; an additional R1 trivia audit historically failed preservation; step2b closes it. Table in REVIEWER-FIXTURES-step2.md.

SecBench current per-root 170 | 0 | 0 | 354 versus distinct-source 134 | 0 | 0 | 293 (W/C/U/I); 11 of579 admitted roots contribute, representing 10 distinct sources. The two total.js roots contribute identical97 rows (36 W/61 I). Measured CORRECT zero does not cover import-equals or module-to-script globals; fixtures supply that coverage.

## Historical step1 union measurements (superseded for current rule)

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

Controller F historical base/narrow head:113 retained,109 W/0 C/4 U/0 I; arm(a) removes0. Historical R-bound census price109 W/4 U. **Union F real measurement (controller, 2026-10-13):** 113 of 113 removed, 109 W / 0 C / 4 U / 0 I; 0 targets, sites or graph rows added; non-JS call sites identical; all 113 are ordinary Source calls, so the JSX and indirect part of arm (b) contributes nothing on F. Details in MEASUREMENTS.md, "Controller private run, union head".

## Existing fixtures and deliberate expectations

| Existing test | Fixture edit | Purpose retained |
|---|---|---|
| `td_duplicate_for_heads_negative_control_single_head_binds` | Caller `export function run` → script `function run`; target was already a script | `L2 f: Exact free_single f@1-1` unchanged; negative duplicate/for-head scope control remains reachable outside module refusal. |
| `rep_unbound_name_through_an_unrelated_parameter_keeps_base_at_r5` | Caller `export function run` → script `function run`; helper/target unchanged scripts | `L5 f: Exact free_single f@1-3` unchanged; unrelated helper parameter remains outside caller scope. |
| `c41_with_reason_keeps_base_at_r5` | Caller and target declarations lose `export` | `L3 f: Exact free_single f@1-3` unchanged; `Unproven("with")` still proves the older R5 drop is reason-scoped. |

| b5 change against main, both `.jsx` / `.tsx` | Arm | Before → after |
|---|---|---|
| Bare nested const, L4 | b | Exact free_single lib:f@1-3 → LocalBindingUnproven |
| Bare nested const, L6 | a | Same Exact → drop |
| Nested if/let, L4 | b | Same Exact → drop |
| Nested if/let, L6 | a | Same Exact → drop |
| Nested var, L4 | b | Same Exact → drop |
| Nested var, L6 | b | Same Exact → drop |
| Same-line const, first L2 | b | Same Exact → drop |
| Same-line const, second L2 | a | Same Exact → drop |
| Module-scope const, L5 | a | Same Exact → drop |
| Module-scope var, L5 | b | Same Exact → drop |

Ten values change per grammar relative to main: four from arm (a), six from arm (b), twenty total. The former “already drop” described step0 rather than main. The test is renamed `b5_nested_declarators_follow_e5_and_preserve_imports`; registered selectors are reconciled where applicable. Imported controls still assert complete `L5 f: Exact import_member lib:f@1-3` and `L7 f: Exact import_member lib:f@1-3`. No assertion is weakened.

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

Keep call sites with a retained qualifier, earlier proven qualified rungs, same-file targets, script-global declarations called from scripts or unbound module callers, syntactic import/require exemptions, earlier positive rungs, non-JS behavior, callback identities and incoming formal slots. CommonJS local aliases outside R-bound remain out of scope. No repo-wide import resolution, tsconfig parity or runtime package execution. Arm(a) Source/non-JSX boundary is unchanged; arm(b) applies only to Source module callers without name evidence and subject to R2. Remaining limit: an unbound call from a module to a top-level CommonJS function, or a nested declaration in a script file, stays as main has it; this belongs with R-local. Follow-ups: key the indirect producer on object plus property; never offer super to free-function rungs.

**D12 remains unchanged by owner instruction.** Six new script-caller/script-target w2m/w2d variants require full Exact FreeSingle targets and visible argument edges before checking that D12 refuses outgoing new-formal reachability. PD-29 now selects those six, while the existing variants assert E5’s drop. D12 still refuses outgoing new-formal paths reaching every cross-file Exact FreeSingle argument edge. Any narrower D12/incoming repair needs a separately authorized increment.

## Acceptance criteria and gates

- Zero CORRECT Exact loss on each admitted public corpus and controller F; any loss invokes STOP. Current public criterion passed; private step2b F pending. Historical step1 F passed (0 CORRECT lost), not transferred to step2b.
- Zero new WRONG/target/site/topology; every differing call site and graph row adjudicated. Public added counts all0.
- Non-JS call bytes and non-call graph digests identical; symbol/file inventories identical; disclose call-derived navigation/dependency changes. All581 public roots passed.
- Every UNDECIDED removal disclosed; preserve INADMISSIBLE exposures and four producer exclusions without yield or zero-cost credit.
- Main-RED/head-GREEN refusal paths and full-value negative controls; current scoped authoritative E5+PD-29 mutants19/19 killed/admissible; prior step1 four-mutant receipt is historical.
- Full serial suite, fmt, doctests, fresh release, adapted matrix and same-worktree base/head TS/JS quick complete with excluded example reported. Rust/full Tier-A boundaries remain explicit.
- Actual-shell controller parity14/14 before F; private zero cost and independent reviews remain gates before adoption.

## Custody and limits

Current step2b patch base is3df0ddfa; frozen main isbab0dfbd. Step2b binary binding and581-root head-to-head identity are in step2b/binary-binding.json and head-identity.json. The remaining paragraph describes historical step2 custody. The R1–R7 bounded fold is retained in the working tree and snapshotted under step2; historical union source e164178b is the predecessor. New head CLI/fact/graph/module/byte helpers are frozen and source-bound in step2/binary-binding.json. Base binaries and calls/facts/graphs, plus matching-command base navigation artifacts, reuse read-only step0 receipts (step2/base references the retained step1 base view). A fresh empty product diff confirms40ad8390 equals bab0dfbd across src/Cargo/build/vendor. This is reuse, not fresh public base-producer execution. Both quick sides and reviewer fixtures do execute fresh base controls. Every head source byte is restored after the exact-main same-worktree control.

No private step2b F capture, confirming independent review round2/2, Git custody/landing, R-local runtime, D12 narrowing/incoming repair, live MCP refresh, Rust/full Tier-A or full unscoped mutation lane is claimed. The four original SecBench producer exclusions remain excluded. No provider/network calls or corpus package execution occurred. CACHE_VERSION/both registry pins115 and navigation cache62 remain unchanged. CLAUDE.md is unchanged; its prospective sentence is drafted below.

## Review coverage and prospective CLAUDE.md sentence (R7)

The measured corpora contain no import-equals or module-to-script-global rows classified CORRECT. The observed zero does not cover those shapes; the new synthetic regression fixtures do. SecBench historically counts identical total.js_3.4.6 rows under command-injection and prototype-pollution: step1 per-root 171 W/418 I versus distinct-source 135 W/357 I; 14 of 579 admitted roots contribute any removal. Step2 measured counts are134 W/293 I distinct-source versus170 W/354 I per-root, from11 contributing roots /10 distinct sources.

Draft for CLAUDE.md’s R5 description when the PR is made (CLAUDE.md is unchanged here): “For JS/TS/TSX Source calls at unqualified cross-file FreeSingle, E5 drops unbound ordinary calls to negative-syntax targets and module-caller calls without syntactic name evidence; unbound possible script targets are decided by the target-syntax arm alone, and import-equals/awaited-import evidence only exempts refusal.”

Hook stop:5 requested a fresh full suite after STOP:5309 run,5307 pass,2 failures,1 ignored. The out-of-scope queued_calls cumulative-wait timing assertion passed5/5 isolated head checks; no cause or base attribution is claimed. These are historical step2 receipts. Step2b closes the terminal-trivia defect; current commands are in REPORT-step2b.md.

## Verification hook stop:5 continuation

On unchanged source, a fresh `cargo nextest run --offline --all-targets --features mcp --no-fail-fast` ran5311 tests:5310 passed,1 inherited d7 example failed,1 skipped (3 slow). Complete assertion left/right values freshly match the retained exact-main same-environment control. Both MCP timing tests pass; no isolated run or code repair followed. Coverage inspection found no missing RED/negative-case regression; no source/test changes. Root VERIFICATION.md now has explicit Verified and Not verified sections, exact commands/totals and exclusions. Receipts: step2b/verification-hook-stop5/{full-suite.log,full-suite-result.json,coverage.json}. No current STOP; private F/independent review remain controller gates.

# Slice I1 review — round 1 of at most 2

**Verdict: FIX.** The direct candidate-list change is additive, but its consumers can promote unrelated namespace targets to Exact, and normalization can admit a nonexistent `.js` module through an appended filename such as `x.js.ts`. The stronger promise that every previously resolved call remains unchanged is also false through star barrels. These are bounded findings; this review does not recommend restarting the artifact.

## Binding and evidence

Reviewed `/Users/wesleyjinks/code/prism-js-i1`, branch `feat/js-relative-import-extension`, clean HEAD `aa74eae0063a21b7f5a8225cca1212e33161a468`, base `origin/main = e2701e89c2fb09cd268232b041b95f7d1ce1e7ea`, four commits, seven changed files. Read-only GitHub connector reads of [PR #368](https://github.com/shoedog/prism/pull/368) returned the same head/base; its diff matched `git diff origin/main` byte for byte. No GitHub writes or worktree edits were made.

Executed the two authorized selections, with offline dependency resolution:

- `cargo test --offline --lib js_relative_candidates`: **29 passed, 0 failed, 1,390 filtered**; [unit.log](sol-scratch/unit.log).
- `cargo test --offline --test integration js_relative_extension`: **20 passed, 0 failed, 1,120 filtered**; [integration.log](sol-scratch/integration.log).

All synthetic comparisons ran the supplied release binaries in the same environment with separate, fresh cache directories. Their reported versions were branch `aa74eae0063a` and frozen main `40ad8390d6cb5544a063346033c7bb1817ad3855`. The latter is older than the current PR base. A local source comparison between that revision and the PR base found no changes in the resolver/export/namespace/path/owner/props files used for the findings. This binds the relevant source seam, not the whole binaries; the same-environment behavior controls below are against the explicitly supplied frozen-main binary, not a freshly built `e2701e89` binary.

Fixtures, full `record_kind: call_site` JSONL, stderr, and hypothesis/probe logs are retained under [sol-scratch](sol-scratch/). An initial extractor used the wrong JSON discriminator; its empty arrays were inadmissible. The intact raw output was subsequently parsed using `record_kind`. An initial `paths` probe used unsupported `moduleResolution: bundler`; its failed canonical alias control made that probe inadmissible for alias safety. The corrected `paths_admitted_*` probes use the admitted `node` configuration and pass their canonical control.

The corpus/checker and 1,011-row matrix measurements in the brief are **supplied evidence**, not independently rerun here. Their zero existing-target changes are measurements of those corpora, not a universal additivity proof.

## Findings — WRONG first

### W1 — WRONG / MATERIAL: finding a namespace module without the requested export promotes an unrelated function to Exact

**Confidence: 100/100.** Trigger: a redundant-segment namespace import, a successfully parsed target module that lacks the requested value export, and an unrelated same-stem file exporting that name.

Minimal fixture [namespace_unrelated_only](sol-scratch/fixtures/namespace_unrelated_only/):

```ts
// app.ts
import * as N from './a/../x.js';
export function g() { N.f(); }
// x.ts
export function other() {}
// other/x.ts
export function f() {}
```

The call records are:

| Binary | `N.f()` result |
|---|---|
| Frozen main | `other/x.ts:f`, **NameOnly/import_qualified** |
| Branch | `other/x.ts:f`, **Exact/import_qualified** |

Raw evidence: [main](sol-scratch/outputs/namespace_unrelated_only-main.jsonl), [branch](sol-scratch/outputs/namespace_unrelated_only-branch.jsonl). The imported module has no `f`; `other/x.ts:f` cannot be its static binding. This is a newly wrong **grade**, not a newly added target identity. An Exact-only consumer now accepts this unrelated function.

Mechanism: the new normalization at `src/call_graph.rs:6715` makes module lookup succeed. `src/resolution_js_namespace.rs:19-20` then returns `None` when export lookup fails. Its caller at `src/resolution.rs:2611-2619` interprets that as permission for legacy `exact(matched, ImportQualified)`. Before the change, module absence reaches the explicit NameOnly E7 return at `src/resolution_js_namespace.rs:48-57`. The same failure occurs with the extensionless redundant spelling and with a type-only export; see `namespace_missing_export_plain` and `namespace_type_export` outputs.

**Recommended fix:** retain whether the module/proof was newly admitted, and keep the existing NameOnly base outcome when a newly admitted namespace module lacks positive export/identity proof. Do not return the sentinel that invokes Exact stem fallback. Pin target, grade, kind, and the negative export result, including an unrelated-only target and a type-only export.

**Alternative:** fail closed on absent namespace value exports and other failed positive proofs for every namespace import. That removes unsound legacy edges too, but broadens this slice and changes existing behavior; it needs an explicit compatibility decision. Another narrow alternative is to leave newly normalized namespace calls at E7 unless positive proof succeeds; this loses no proven new Exact edge, but retains speculative NameOnly targets.

**Does not apply** when positive export proof selects exactly one base identity, or the raw stem pool is empty. `namespace_export` is a separating control: main has two NameOnly targets; branch correctly selects only `x.ts:f` Exact. That rules out mere successful module lookup as sufficient proof of Exact.

### W2 — WRONG / MATERIAL: normalization appends extensions to an explicit `.js` filename and binds a module that does not exist

**Confidence: 99/100.** Trigger: `./a//x.js`, only `a/x.js.ts` on disk, and that file exports `f`. There is no `a/x.js`, same-stem `a/x.ts`/`a/x.tsx`, or `a/x.js/index.*`.

| Binary | Named `f()` result |
|---|---|
| Frozen main | `UnknownName`, no targets |
| Branch | `a/x.js.ts:f`, **Exact/import_member** |

Evidence: [fixture](sol-scratch/fixtures/suffix_no_module/), [main](sol-scratch/outputs/suffix_no_module-main.jsonl), [branch](sol-scratch/outputs/suffix_no_module-branch.jsonl), [hypothesis/result log](sol-scratch/suffix-probe-log.md).

`src/call_graph.rs:6761-6766` appends every extension to the already suffixed normalized path, sets `found_file`, and prevents actual suffix substitution at line 6785. TypeScript's documented explicit `.js` lookup substitutes the suffix with `.ts`, `.tsx`, `.d.ts`, then considers `.js`/`.jsx`; it does not select `x.js.ts`. The expected absence here follows from that exhaustive lookup, independently of overloads or declaration-vs-implementation identity. [TypeScript module reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html#file-extension-substitution).

This differs from the deliberate **real `x.js` wins** policy: here the import has neither that literal module nor its source counterpart. The source is named for a different emitted path. Main's canonical `./a/x.js` already has this legacy append-extension defect; I1 carries it into a previously unresolved spelling. Preserving the canonical legacy defect does not make the newly emitted edge a correct import binding. The `dotted_ts` probe also shows `a/x.js.ts` beating an available `a/x.ts` for the redundant spelling.

**Recommended fix:** keep the outer legacy-nonempty return, but do not append source extensions to a recognized explicit suffix in the new normalization fallback. For `.js`, inspect the actual normalized literal and the deliberately supported directory case, then use same-stem `.ts`/`.tsx`. Add an absent-module control and a competing `x.js.ts`/`x.ts` case. Audit the same bounded suffix-appending population for `.jsx`, `.mjs`, `.cjs`, `.ts`, and `.tsx`; these all use that loop.

**Alternative:** refuse newly normalized explicit-suffix resolution when an appended-suffix candidate would win. This is smaller and preserves all old literal lists, but leaves those unusual new spellings unresolved. A broader alternative repairs the canonical legacy lane too, at the cost of changing existing edges and violating this slice's compatibility boundary. Canonical/redundant list parity must be narrowed where canonical legacy behavior is known to be unsound.

**Does not apply** to a genuine indexed literal `x.js`, ordinary extensionless resolution, or a same-stem substitution without misleading appended candidates. This finding is about the new fallback, not a demand to change the explicitly retained old canonical lists.

### W3 — WRONG / MATERIAL against the stated compatibility contract: new star hops can remove an existing Exact edge

**Confidence: 100/100 for the changed result.** This is a contract failure; the branch's rejection of the now-visible ambiguity is semantically correct.

Fixture [star_conflict](sol-scratch/fixtures/star_conflict/): `app.ts` imports `f` from `./barrel`; `barrel.ts` contains `export * from './a'; export * from './b.js';`; both `a.ts` and `b.ts` export distinct `f` functions.

| Binary | `f()` result |
|---|---|
| Frozen main | `a.ts:f`, **Exact/import_member** |
| Branch | `UnknownName`, no targets |

[Main record](sol-scratch/outputs/star_conflict-main.jsonl), [branch record](sol-scratch/outputs/star_conflict-branch.jsonl). The caller's `./barrel` candidate list is unchanged. `apply_js_export_resolution` at `src/call_graph.rs:2291-2303` feeds the broader resolver into every export hop; `src/js_exports.rs:645-687` now collects two identities and blocks the export. The same removal is reproduced for bare-dot and redundant-segment star hops, and for a class competing with the old function (`star_dot_conflict`, `star_segment_conflict`, `star_conflict_class`). Controls `star_same_identity` and `star_missing_export` retain `a.ts:f` Exact on both binaries, separating identity conflict from generic extra-star suppression.

**Recommended fix:** reconcile the acceptance contract before merge: allow newly discovered import/export ambiguity to remove an old unsound Exact edge; add these conflict/identity/empty-export controls and explicitly limit “additive” to legacy module-candidate lists. Do not preserve an invalid Exact binding just to maintain counts. The currently stated stronger compatibility criterion is not met.

**Alternative if strict output preservation is mandatory:** retain a legacy export projection and admit the new projection only for previously unresolved call sites. That requires two projections and an explicit legacy authority policy, retains known wrong old edges, and reduces consistency across callers. It is substantially more costly than the candidate-level change.

**Does not apply** when the newly found branch lacks the name, reaches the same declaration, or an explicit named export overrides the competing stars; nor is it a violation once the contract expressly permits these precision corrections. It should not be “fixed” by disabling conflict detection.

### S1 — SMELL / MATERIAL: the new tests do not cover every conjunct or consumer consequence

Evidence: the 29 candidate tests and 20 integration tests pass, including W1/W2/W3 being reachable in separate fixtures. `tests/integration/js_relative_extension_test.rs:274-305` covers missing exports and type-only imports only for the named-import lane. It does not exercise namespace proof failure. The integration barrel test has one star hop, not the conflict population in W3. Receiver/cardinality and props-owner consequences are not exercised by the new integration file.

A concrete **predicted surviving mutant**, established by inspecting every row but not executed here, is replacing the directory-only predicate at `src/call_graph.rs:6748` with:

```rust
let directory_only = module_path == "." || module_path == ".." || module_path.ends_with('/');
```

This keeps the tested bare dots and trailing slash, but loses the terminal `/.` and `/..` cases. The only new test ending in nonbare `/..` is `../..` from `pkg/app.ts`, which escapes before candidate probing, so it cannot discriminate this predicate. For caller `pkg/app.ts`, `./a/..` over `pkg.ts` and `pkg/index.ts` must return only the index; the mutant additionally returns `pkg.ts`, changing first-winner and single-candidate behavior. The supplied branch correctly selects `pkg/index.ts:f` in [parent_directory_only](sol-scratch/fixtures/parent_directory_only/). Removing normalized `index.jsx`, `index.mjs`, or `index.cjs` probes would also survive the new selections by inspection; their index fixtures use only JS/TS/TSX.

**Recommended fix:** add table-driven terminal `/.` and `/..` cases with a same-stem file, all six index extensions, and source-bound consumer negatives/ambiguity cases. Assert complete rows, not just twin parity or absence of one kind of Exact edge. **Alternative:** register the helpers and consumer seam in the mutation gate; this has tooling cost and complements rather than replaces the missing fixtures. **Does not apply** to already pinned conjuncts: `.ts`/`.tsx` substitution and order, legacy-nonempty short circuit, bare dots, root guard, internal segment handling, and index/substitution suppression have meaningful discriminating rows. The 11 supplied mutant kills do not establish exhaustiveness.

### S2 — SMELL / MATERIAL, follow-up: bare-dot forwarding and caller-path barrels remain narrower than direct relative resolution

`src/js_exports.rs:230-232,331-337,579-615` treats `.`/`..` import-forward literals as nonrelative and retains a blocked claim. `forward_dot` remains `UnknownName` on both binaries; equivalent direct `export { f } from '.'` changes from `UnknownName` to `pkg/index.ts:f` Exact on the branch. [Forwarded result](sol-scratch/outputs/forward_dot-branch.jsonl), [direct result](sol-scratch/outputs/direct_dot-branch.jsonl). This is a completeness discrepancy, not a demonstrated wrong edge or lost existing edge.

The admitted paths control resolves both relative and alias calls on both binaries. A `./x.js` or `.` hop inside that same barrel produces a new correct relative edge but keeps the alias call unresolved (`paths_admitted_js_hop`, `paths_admitted_dot`). A competing new star does **not** produce a wrong alias edge: `via_unresolved_star`, checked at `src/resolution.rs:3811-3813`, blocks it. Supporting logic is `src/js_paths.rs:448-494` and `src/js_exports.rs:255-256`. `file_matches_module` is not a competing JS named-import authority here: the live R4c JS arm uses export facts; its other branch is Python.

**Recommended follow-up:** explicitly scope I1 to the legacy relative projection and carry a bounded parity item for bare-dot import-forward eligibility and caller-project hops. Reuse literal classification where appropriate, while preserving the raw-whitespace and paths/snapshot proof fences. **Alternative:** unify all lanes now; cost is a broader source-presence, configuration, precedence, and refusal review, not a safe one-line helper substitution. **Does not apply** to new direct relative named/re-export calls or ordinary `./x.js` import-forwarding, which works in `forward_js`; no merge blocker is established merely by leaving the conservative lanes unchanged.

### S3 — SMELL / IMMATERIAL: documentation overstates sharing

`CLAUDE.md:212-215` says JS/TS relative specifiers “share one candidate list”; the retained caller-project paths and raw forwarding eligibility are exceptions, and namespace resolution also retains suffix-sibling logic. The candidate-level additivity description itself is accurate. Cache versions **115** and **63**, both pins, and the unchanged skip-policy version **2** are correctly coordinated (`src/cpg_cache.rs:251-254,798-799`; `src/navigation/call_edge_cache.rs:106-108,733`).

**Recommended fix:** qualify the sentence as the legacy relative candidate projection and mention the separate paths/forwarding boundaries. **Alternative:** link a small resolution-policy table; more documentation to maintain. **Does not apply** if “share” is expressly scoped to the consumers of this helper. No persisted-state wrong result was demonstrated; cache rejection/version checks were read, not separately exercised.

## Answers to the remaining policy questions

**Precedence (Q2):** keep a genuine indexed literal `x.js` winning for this compatibility slice. With `app.ts` importing `./x.js` and different `f` implementations in `x.js` and `x.ts`, the branch/main policy chooses the JS callable; TypeScript-first chooses the TS callable and changes an existing binding. With `x.js` plus `x.d.ts`, the current policy retains the JS implementation; TypeScript-first selects the declaration, requiring a separate declaration-to-body policy or losing that implementation edge. `Exact` describes prism's modeled binding; it cannot be advertised as unconditional checker agreement across those collisions. TypeScript's order is a reasonable later versioned policy, but silently importing it into this additive slice would defeat its purpose. [Documented TypeScript order](https://www.typescriptlang.org/docs/handbook/modules/reference.html#file-extension-substitution).

**Exclusions (Q3):** omitting `.d.ts` as a substitution target is appropriate for this body-oriented slice; declaration-only files do not supply an implementation body. That does not mean declarations are generally irrelevant to source precedence, and it does not justify bypassing an unrelated file. Leaving `.jsx`/`.mjs`/`.cjs` replacement for a later slice is an intentional completeness boundary. The new `.mjs`/`.cjs` negative tests use `.ts` distractors rather than actual `.mts`/`.cts` sources, so they do not fully pin those specific exclusions. No lost existing edge from those exclusions was found. Namespace sibling substitution already models some of these spellings separately; “not modeled” should be read as this new candidate lane, not the entire resolver.

**Path adversaries (Q4):** the fresh fixtures confirmed source-bound file-before-index ordering for normalized extensionless imports, directory-only trailing slashes and terminal `..`, exact-case lookup, ordinary dotfile substitution, and refusing root escape/reentry and terminal query/fragment spellings. A directory named `x.js` wins its index before same-stem substitution, in accordance with the explicitly inherited canonical policy. W2 identifies the concrete wrong appended-suffix case. Normalization does not use filesystem canonicalization, case folding, or URL decoding. Query/fragment spellings were checked only as terminal `?raw`/`#part`; this is not a proof for every loader or URL form. Directory-package manifests and host-specific module modes were outside these probes.

**Negative controls (Q5):** named missing-export + unrelated-function, type-only import, and type-only named re-export fixtures drop `UnknownName` on both supplied binaries. Namespace missing-export and type-only export do not have that safety: W1 demonstrates the false Exact promotion. Do not generalize the named-import tests to all consumers.

**Tests and persisted state (Q7/Q8):** see S1 and S3. The PR reports full-suite runs before the final two commits and explicitly says they were not rerun at final HEAD. This review neither transfers that earlier full-suite result to final HEAD nor attributes its six supplied failures to I1. Only the two selections above were executed locally.

## Checked and found sound

- Checkout, four-commit diff, live PR head/base/diff correspondence, clean tree, supplied binary identities, and relevant frozen-main/base source seam.
- Exact preservation of nonempty old candidate lists: the extracted literal algorithm and extension/index order are unchanged, and the wrapper returns that same list before fallback.
- Existing single-candidate owner/props/executable-owner proofs retain their cardinality guards; candidate-list preservation protects previously nonempty direct module identities. Newly found ambiguous lists do not automatically prove a class.
- Normalization's lexical root guard and directory-only behavior in the tested cases; same-stem `.ts` then `.tsx` substitution; refusal of excluded suffix substitutions in the selected tests.
- Genuine literal JS precedence, file-before-index order, normalized directory/index suppression of substitution, ordinary dotfile/case/query/fragment fixture results, and named-import negative controls.
- The corrected admitted paths projection controls and unresolved-star refusal; bare-dot forwarding remains conservative.
- CPG/nav cache version and pin changes, and where both readers reject a version mismatch.
- 29 unit and 20 integration tests passed. No worktree modifications, git writes, binary rebuilds for fixtures, corpus execution, or restricted-directory access.

## Not checked

- Full project suite, MCP-feature suite, doc tests, Tier-A harness, repository mutation gate, or independent execution of the implementer's 11 mutants. The identified surviving mutants are static predictions.
- Independent rerun of X/T/F, checker declarations, variant matrix, or aggregate receipts. Private corpus directories were not opened.
- A rebuilt release binary at current base `e2701e89`; comparison used the authorized frozen main and its unchanged relevant source seam.
- Runtime execution, loaders, symlink traversal, directory `package.json` entrypoints, every query/fragment shape, or platform-specific casing behavior.
- Exhaustive end-to-end receiver, imported-props, executable-owner, incremental-cache or persisted-cache invalidation scenarios; the listed consumer code and cache version guards were read.

**Before merge:** fix W1's namespace authority transition and W2's new appended-suffix false bindings, add their regressions, and reconcile W3's call-level compatibility criterion with sound ambiguity rejection. Keep the artifact and apply targeted fixes. S1–S3 identify coverage/documentation/follow-up work; they are not independent blockers without the demonstrated WRONGs.

**VERDICT: FIX — W1, W2, and W3's compatibility-contract reconciliation must be addressed before merge.**

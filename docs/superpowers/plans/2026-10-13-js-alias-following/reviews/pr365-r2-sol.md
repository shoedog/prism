PR #365 — confirmation review, round 2 of 2

APPROVE. The material round-1 defects are closed. The fold is sound on the current matrix. Two immaterial observations remain below; neither needs another review round. No implementation changes, Git writes, or GitHub posts were made.

Reviewed `/Users/wesleyjinks/code/prism-js-matrix`, clean branch `test/js-alias-variant-matrix`, HEAD `35557277393f50e53d6db810f37219fdb44751e1`, base `origin/main` = `e2701e89c2fb09cd268232b041b95f7d1ce1e7ea`, four commits. Confirmation scope was `e9d411cc..HEAD`, the two fold commits. Both round-1 reports were read. References below are relative to that worktree unless marked as scratch.

The requested destination, `/Users/wesleyjinks/prism-evidence/js-alias/review-365/r2-sol.md`, is outside this session's writable roots. This completed report is saved at `/Users/wesleyjinks/prism-evidence/js-alias/review-365/r2-sol-scratch/r2-sol.md`; the controller can copy it to the requested destination.

Findings, WRONG first

F1. **WRONG · IMMATERIAL — the new ambiguity check counts declaration lines rather than declarations.** Residual edge of sol R3, not a new material blocker. Confidence: 100/100.

Evidence: `generator/lib.cjs:10–17`. This valid synthetic source has two different `f` declarations starting on line 1:

```js
function f() { return 'outer'; } function caller() { function f() {
return 'inner'; } const h = f; return h(); // SITE
}
```

The marked call calls the inner `f`. A TypeScript syntax walk finds outer `f` at lines 1–1 and inner `f` at lines 1–2, with zero syntax errors. `declLine(src, 'f')` returns 1 without rejecting the ambiguity. The copied reference verdict grades an exact outer `a.js:f@1` as CORRECT. The Rust rule has the same file/name/start-line identity. The current fixture corpus does not use this ambiguous identity, so no present pin is wrongly graded by this residual case. The round-1 same-name/different-line and different-name/same-line cases are fixed.

Recommended fix: for the generator's supported declaration grammar, count declaration occurrences, and reject multiple same-name declarations on one line, including when a TARGET marker cannot distinguish them. Keep markers for declarations on different lines. Add the source above as an exporter rejection test. Alternative: carry declaration columns/spans through truth, dump normalization and matching; this supports the shape but costs a broader identity/schema change. This does not apply where a callable's file/name/start-line triple is unique. Scratch evidence: `same-line-ambiguity.cjs`, `same-line-grade.cjs`.

F2. **SMELL · IMMATERIAL — the PR's full-suite attribution sentence is stronger than its evidence.**

The live [PR description](https://github.com/shoedog/prism/pull/365) reports four local full-suite failures and says none is caused by this PR, while explicitly saying three MCP lazy tests were not rerun against the base. That qualification is useful, but those three causal claims lack the required same-environment control. Increased concurrent fixture work and ambient machine load could produce the same timeout observation; a green CI workflow does not distinguish them. This is an evidence-wording concern, not a demonstrated matrix defect.

Recommended fix: state that the parameter-slot failure was reproduced on the base, while attribution of the three load-sensitive MCP failures remains unverified. Alternative: run the same failing workload against the base under comparable load; cost is another controlled execution by the controller. This does not apply once a discriminating same-environment control establishes the attribution. I did not run the unfiltered suite or attribute those failures myself.

Round-1 closure

| Brief row / round-1 IDs | Confirmation |
|---|---|
| Stack overflow / Opus W1, sol S2 | **Closed.** `actual_rows` owns the fixture `par_iter` inside `build_pool::install`. `nav_session` installs onto that same OnceLock pool. Rayon's installed `Registry::in_worker` executes directly when the current registry matches, instead of entering `in_worker_cross` and stealing unrelated fixture work while waiting. This removes the diagnosed cross-pool nesting mechanism, rather than merely increasing the outer stack. Local 1-worker and 4-worker runs each passed 23 tests and all 1,159 rows. |
| Memo direct-call truth / sol R1, Opus W2 | **Closed.** All 12 call-form rows now have `none`; the corresponding pins are NONE-OK. Their sources still directly invoke the object returned by React memo. JSX truths remain unchanged. |
| Parameter shadows import / sol R2, Opus S5 | **Closed.** Both JS and TS use `may: []`. `keep` is exported, and an invisible consumer can supply it to the parameter. Both pins are MAY-NONE, with exactly the two NONE-OK → MAY-NONE ledger entries. |
| Callable identity / sol R3 | **Material cases closed; F1 is the immaterial residual.** Rust and reference both require file AND name AND matching line when provided. The Rust negative tests and scratch reference probe reject both original false-match cases. Unmarked duplicates on distinct lines are rejected; TARGET selects the intended distinct line. |
| Deleted pins, permanent acceptance, regression set / sol R4, Opus W3/W5/S4 | **Closed within the documented verdict-transition policy.** Bless compares against the committed HEAD file, not working pins; deleted working rows retain their history. Ledger matching requires id/from/to. Unknown and stale entries fail. Orphan working pins block bless; deliberate fixture removal requires a visible manual pin removal. New baseline-absent IDs are intentionally characterization additions, not regressions. Tests enumerate multiple simultaneous failures and the widened regression set. Same-verdict target changes remain reported but unguarded by explicit decision. |
| Exported pins / Opus W4 | **Closed.** `export.cjs` writes only fixtures; the README routes pins through bless. Scratch regeneration produced exactly the committed fixture bytes and no expected file. |
| UTF-16/UTF-8 ordering / sol R5 | **Superseded.** Export no longer writes expected rows. Rust is the sole pin renderer; the former two-writer ordering disagreement cannot create a pin mismatch through the documented flow. |
| File length / sol R6, Opus S10 | **Closed.** The generator is split into shared, local, parameter and import modules; the longest generator file is 364 lines. |
| Missing shapes / sol S1, Opus S1/S2/S3 | **Closed for the adopted bounded fold.** The specified scope, namespace, path, hook, textual-alias and NameOnly additions are present. The table below records the entire 148-row addition. This is not a demand to implement every suggestion in Opus's longer optional list. |
| Policy and private context / sol S3 | **Closed.** The dual-extension truth has one literal-JS target and explicitly labels prism's policy; it no longer unions two resolution policies. All 12 context sources export Ctx, making unseen providers constructible. |
| Script globals / Opus S6 | **Closed.** The two affected shapes explicitly state the script-loading condition, in both languages. |
| Truth structure and checker / Opus S9 | **Closed to the specified structural scope.** Rust checks target files and supplied lines, name presence on those lines, and ANON markers. Export validates declarations. `run.cjs` sets a failing status on named, unexplained checker disagreements, with four documented shape exceptions. This is structural validation plus a partial checker cross-check, not a semantic proof for untyped sources. |
| PR description / Opus S12 | **Closed for the original claims.** The live description names the current WRONG rungs and reports the original red CI/overflow. It binds to the reviewed head and 1,159 fixtures. F2 concerns separate new attribution wording. |
| Deliberately unchanged lazy contract, truth digest, same-verdict target guard | **Decision preserved.** No new blocker asserted for those explicit boundaries. |

Fold guard and Git behavior

`baseline()` sets the git subprocess's directory to `CARGO_MANIFEST_DIR`. Starting Cargo or the test binary from a subdirectory therefore cannot change what `HEAD:./tests/.../expected.jsonl` means. HEAD names a commit tree; no parent, merge base, branch name or remote ref is needed. Detached and shallow checkouts with the baseline at HEAD therefore have the required information. Those checkout modes were source-reasoned, not created or mutated during this review.

If HEAD is unborn, or its tree lacks the baseline, or git cannot run, bless explicitly refuses and never calls the writer. There is no unsafe working-file fallback. Initial introduction requires a committed baseline (an empty committed file can serve as bootstrap); this prerequisite is documented. Normal mode never calls `baseline`, so CI needs neither Git history nor a bless-capable checkout. CI must leave the bless variable unset.

The guard examines every actual row, accumulates every unaccepted regression, and checks ledger problems against the rows about to be written. `from` must equal the HEAD verdict; `to` must equal the new verdict. A previous acceptance cannot excuse an improvement followed by another regression through the supported workflow: its entry becomes stale when the pinned verdict improves and must be removed. Changes to targets with unchanged verdicts remain whole-row differences, requiring pin review even though they do not require a transition entry.

`write_atomic` writes a sibling temp file and renames it over expected, avoiding partial-file reads. Separate processes use different PID temp names. This is atomic replacement, not a claim of crash-durable fsync or support for concurrent independent bless operations. The documented single-test bless command avoids sibling structural tests observing the pre-update ID list while adding fixtures.

Scratch `guardcheck` copies the common/guard files byte-for-byte. It copies the baseline/write functions, adapting only the baseline directory/ref into parameters for testing. All 19 tests passed, including the original model/guard tests, an actual read of the worktree HEAD baseline while invoked from scratch, an unavailable-path refusal, and complete atomic replacement in scratch. No worktree bless was run.

New and corrected truth audit

There are exactly 148 new IDs and 31 edited existing fixtures. I read the new source templates and their distinct scope/form expansions, traced naming and language expansion, inspected their targets and competing files, and cross-checked all 179 added/edited inputs with TypeScript 5.9.3. This is template/expansion review, not a claim of separately hand-reading every duplicated axis rendering.

| Added family | Rows | Authored truth and source reason |
|---|---:|---|
| trap2 on ident, ns-member, let-reassigned, object-param-destructure, hook-result-destructure, alias-of-param | 36 | Adding two unrelated decoys does not change the lexical value. Known aliases/hook returns remain one; conditional reassignment remains `{f,g}`; unknown opts-derived values remain `may: []`. Exactly 12 final rows carry NameOnly candidates. |
| rhs-shadowed-at-site | 6 | `a0 = imp` is initialized at module scope. A caller-local `imp = N.g` cannot change a0's saved f value. One f is right. |
| ns-via-alias-hop | 18 | M holds the imported namespace; M.f is f, at fn/module/outer scopes. |
| hook-result-two-keys / hook-result-two-slots | 24 | Each private useThing returns g first and f second. Destructuring selects the second key/slot; one f is right in both scopes. |
| ident-own-name-import | 20 | The RHS f/F is the imported callable; aliases have different names. Call and JSX forms select that target, including enclosing scope. |
| ident-same-file-fn | 18 | The RHS local1 is the same-file function, not a similarly named function in callee/decoy files. |
| ident-rhs-is-outer-const / ident-rhs-is-catch-param | 8 | The outer const takes opts.cb; the catch variable takes an arbitrary thrown value. Neither is established by the imported f. `may: []` is right. |
| ident-rhs-is-outer-param / ident-rhs-is-own-param / ns-receiver-is-parameter | 6 | Unknown exported parameters determine the value, including the namespace-like receiver. `may: []` is right; exporting keep makes an in-repo argument possible. |
| same-stem-two-dirs and .js-suffix twin | 4 | The explicit directory chooses a/callee or b/callee, respectively; the other same-stem file is a real wrong-target trap. |
| dir-index / parent-file / file-beats-directory | 6 | The targets are pkg/index, the parent callee file, and the sibling callee file ahead of its directory index. The competing index in the last shape has a distinct f identity. |
| js-suffix-is-not-a-directory | 2 | Only callee/index exists. The explicit callee.js specifier has no implementation file; binding it to the directory index would be wrong. None is right. |

No wrong or newly arguable authored truth was found in these additions. The checker named targets for 127 of the 179 audited rows; zero site-source syntax errors were found. Seven named answers differ: the six JS hook-result-two-slots rows (the checker chooses the first signature from the inferred union array), and `import.named-js-suffix-literal-and-ts.call.ts` (the explicit policy row). Source inspection supports the authored second-slot f in all six JS rows; the policy discrepancy is disclosed, not presented as TypeScript's rule. Untyped answers do not certify unknown parameters or external React APIs.

All 12 memo call corrections and both parameter corrections were checked in the serialized fixtures and pins. All 12 context edits preserve MAY truth while exposing Ctx. The script contract edits and the single-target policy edit are present in regenerated data. The complete 1,159-fixture regeneration from scratch copies of the generator was byte-identical to the committed fixtures.

No additional material hole was identified for the named upcoming bounded slices during this confirmation. The new rows distinguish initializer scope from use scope, namespace-value hops from callable hops, second hook result selection, real textual-alias behavior, NameOnly refusal, directory precedence and suffix traps. Acceptance of this fold is not an exhaustive guarantee over arbitrary JS/TS programs or an approval of those product slices.

Checked and found sound

- Final checkout/base/branch/fold binding; clean worktree before and after review; no product-source change in the fold.
- `env -u PRISM_BLESS_JS_ALIAS_MATRIX RAYON_NUM_THREADS=1 ... cargo test --offline --test integration js_alias_matrix`: **23 passed, 0 failed, 1,120 filtered**, 59.89 s test time. All 1,159 rows matched.
- The same command with `RAYON_NUM_THREADS=4`: **23 passed, 0 failed, 1,120 filtered**, 12.90 s test time. All 1,159 rows matched.
- Pool fix mechanism, guard decisions, stale-ledger checks, original identity counterexamples, source/target structure, all added shape templates and corrections, and byte-identical regeneration.
- Scratch copied guard/baseline/write checks: **19 passed, 0 failed**. Their output is recorded in this session; runnable sources are in `r2-sol-scratch/guardcheck`.
- Live PR metadata matches the exact local head/base. GitHub's commit-workflow response reports CI run `38468459447` completed successfully for the requested head. The owner's supplied brief reports all nine checks green; the individual-job request timed out and adds no evidence.
- Durable scratch evidence: `new-fixtures.json`, `changed-fixtures.json`, `source-inventory.txt`, `checker-added-and-edited.json`, `identity-probe.cjs`, same-line probes, copied generator, exported fixtures, and guardcheck sources. No corpus package was executed; excluded directories were not opened.

Not checked

- The unfiltered project suite, the other 1,120 integration tests, nextest/MCP, doctests, clippy, fmt, mutation gates or Tier-A. The authorized review command was the focused matrix suite; no implementation-completion claim is made.
- A new CLI-vs-library dump comparison or cache-mode comparison. The library route was re-read; round-1 parity receipts and the PR's new parity claim were not independently rerun.
- Linux/case-sensitive/Windows execution locally. The successful GitHub workflow is remote evidence, not local platform execution; individual job logs were not retrieved.
- Actual detached/shallow/unborn repository construction, bootstrap blessing, or any Git writes. The Git cases above combine source analysis with existing-HEAD and unavailable-path scratch controls.
- The real worktree bless writer or concurrent/crash-fault blessing. Scratch atomic replacement and pure guard tests were exercised instead.
- React package execution, arbitrary external providers, the optional full-matrix checker run, or source truths unchanged by the fold beyond the round-1 closure scope.
- The requested out-of-root report copy. Controller action remains to copy this completed scratch report to the requested path.

VERDICT: APPROVE — material round-1 findings closed; two immaterial observations, no further review round required.

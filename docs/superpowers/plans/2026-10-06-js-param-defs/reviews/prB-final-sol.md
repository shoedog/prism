# PR-B final review — candidate 2431cb10

**Result: FIX. Three MATERIAL WRONG findings; one MATERIAL SMELL and one IMMATERIAL SMELL. Review round 1/1, with no extension or source repair.** The three wrong behaviors have concrete fixtures and bounded fixes on the retained candidate. E13 is respected: the accepted Flow population is not a blocker.

Review binding: `/Users/wesleyjinks/code/prism-pd-rev-sol`, clean branch `review-final`, plan HEAD `b2aa4f21a901632ea58e2c348f8bdbd4c7a5fa32`. Source reviewed is `git diff origin/main origin/wip/js-param-defs-prB-r3`: main `a57c5fcf858a8d977f2656d5814750a32823cd00`, candidate `2431cb104db7828d11ae409dc207c1e339270c93`. This plan checkout contains main product source, so I exported and built the exact candidate and main in isolated, non-Git scratch directories. The packet's older main `da0604b3` has identical src/tests/Cargo/build/vendor/embedded-asset bytes to the current comparison main. All 727 paths in R7's source/test/mutant manifest match candidate Git objects, with no missing, extra or different file.

All source line references below refer to **2431cb10**, not the main source in the plan checkout. [Durable evidence](prB-final-sol.evidence.json) contains the complete 64 synthetic fixture sources, parser facts, full byte/owner/label rows from both fresh builds and both packet binaries, Node controls, hashes, build logs and hypothesis/probe/result log. No corpus package was executed, no directory named `frontend-portal` was opened, and no Git writes or PR comments were made.

**W1 — WRONG / MATERIAL: class receiver writes become enclosing callback Defs and false Exact edges.**

Evidence: **reproduced**, `src/ast_callback_identity.rs:204`, `src/data_flow.rs:619`, and `src/ast_callback_identity.rs:1201`. Minimal `case.js`:

```javascript
register(function () {
  this.x = 1;
  const C = class {
    y = (this.x = 2);
  };
  use(this.x);
});
```

Head emits `<cb@1:10>:this.x`, Def line 4 bytes **66..72** → Use line 6 bytes **90..96**, **Exact**. It also incorrectly marks the outer line-2 Def → line-6 Use `NameOnly(Killed { kill_line: 3 })`. Main emits no rows on this fixture, so these are additions, not an attributed named-function regression. Both fresh builds and retained binaries give identical complete rows; all parsers report a clean tree.

The field initializer's `this` is the instance receiver; the line-6 `this` is the callback receiver. `js_ts_span_in_own_scope` stops only at explicit callable nodes, so a field/static-block write is admitted as an outer synthetic Def. The receiver fence is applied to the **reference**, which is outside the class, and therefore misses the Def-side receiver change. Ordinary and static field variants reproduce the Exact edge in JS, TS and TSX. A static-block variant also adds a wrong cross-receiver row, with confidence varying by statement layout. Nine field/static-field/static-block language cells were checked. A class-method write is correctly excluded.

Independent synthetic Node controls instantiate the class where needed and invoke the callback with a separate object: each field/static-field/static-block writes its own receiver, while the outer read remains **1**. This separates two receivers from the alternative explanation of an ordinary write to the same binding. It also distinguishes this from E13: the source is valid, unannotated JavaScript and has no parse recovery.

Fix: fence Def admission and RD kills by receiver identity as well as reference identity. Class field values and static blocks must not supply an enclosing synthetic owner's `this` Def or kill. Alternatively, model explicit receiver identities for both endpoints and compare them. Preserve outer-environment computed-key/heritage evaluation rather than excluding every class subtree. A regression should reject the line-4 Def/edge and erroneous kill, retain the outer line-2 → line-6 Exact row, and cover ordinary/static fields, static blocks and method/arrow/computed-key controls.

Self-critique: this targets newly emitted synthetic rows. The named legacy pass can have related inherited defects; that is not evidence this PR newly regressed named output. Writes to a captured lexical variable in a class are a different case and must not be rejected merely because they are inside a class. Exact's static-binding meaning does not excuse this example: the receivers themselves differ.

Classification: **CLOSED INSTANCE of E3 / receiver containment** (the existing `synthetic_this_member_def_stops_at_a_nested_receiver` family). Confidence **100/100** for the reproduced input; it would collapse if the saved endpoints belonged to the same receiver or were not emitted by the candidate, both of which the controls rule out.

**W2 — WRONG / MATERIAL: a class declaration's inner self-name is conflated with its mutable outer binding.**

Evidence: **reproduced**, `src/ast_callback_identity.rs:622–625` and `src/ast_callback_identity.rs:259`. Minimal `case.js`:

```javascript
register(function () {
  class C { m() { use(C); } }
  C = 1;
  use(C);
});
```

Head adds a `<cb@1:10>:C` edge from the outer assignment, line 3 bytes **55..56**, to the method's `C`, line 2 bytes **45..46**, `NameOnly(CfgIncomplete)`. That Use resolves to the class's inner self-binding, which still denotes the original constructor after the outer `C` is reassigned. Main emits no rows for the anonymous fixture. The legitimate outer assignment → outer line-4 Use is separately present as Exact and must remain.

The scope match recognizes the named class-expression node kind `class`, but omits `class_declaration` and `abstract_class_declaration`. The latter two fall through to the enclosing block's class declaration binding. Five declaration cells reproduce: ordinary JS/TS/TSX and abstract TS/TSX. The named-class-expression control does not leak; the named-function control has the same inherited bad row on both sides. The independent Node control saves the original constructor, reassigns outer `C`, then calls the original method: it observes the original constructor inside and **1** outside. Parser recovery, a text-only name collision and a newly introduced named-pass regression are ruled out.

Fix: represent the inner class-name environment for ordinary and abstract declarations, while retaining the outer declaration binding. Alternatively, refuse cross-boundary rows for the declaration's own name until that environment is represented. Tests should remove the cross-binding row, keep the outer assignment/read row, and retain unrelated captured names and class-expression controls. Merely demoting confidence is insufficient: zero false edges includes NameOnly edges.

Self-critique: this is not a claim that a captured ordinary variable is frozen when a method is created. It is the distinct class self-name binding. Nor is it a demand to fix every legacy named-pass row; the blocker is the new synthetic row.

Classification: **CLOSED INSTANCE of E3 / class self-name binders**. Confidence **100/100** for the reproduced JavaScript case; an identical inner/outer binding would invalidate it, but the identity control directly distinguishes them.

**W3 — WRONG / MATERIAL: erased TS wrappers bypass the accepted direct-EVAL refusal.**

Evidence: **reproduced**, `src/ast_callback_identity.rs:507–516` and `src/data_flow.rs:582`. Minimal `case.ts`:

```typescript
register(function (f) {
  eval!("f = 2");
  use(f);
});
```

Head admits the callable with `EVAL=false` and emits the formal Def, bytes **19..20**, → Use bytes **48..49**, **Exact**, under `<cb@1:10>`. The R7 census reports the same false EVAL flag. Main has no row. TS 5.9.3 erases this to `eval("f = 2")`, which is direct eval; the accepted B-D15 rule requires refusing **all synthetic formals** in this callable. This is a refusal-policy violation, not a proposed change to Exact's static-binding grade.

The callee walker unwraps only `parenthesized_expression`, leaving a non-null/assertion wrapper instead of the underlying `eval` identifier. The bounded population checked is non-null, `as`, `satisfies`, and composed wrappers in TS/TSX, plus angle-bracket type assertion in TS. The four basic wrapper kinds comprise nine applicable language cells; compositions reproduce too. All are parser-clean. TS erasure and Node execution show the wrapped calls change the local formal from **1** to **2**. Parenthesized direct eval is correctly refused; sequence and optional eval stay indirect, observe **1**, and remain admitted. Those controls separate transparent erasure from the alternative of an intentionally indirect call.

Fix: peel erasure-transparent runtime-expression wrappers to the actual callee operand before the direct-eval identifier check, retaining optional/member/sequence-call exclusions. For type assertions, inspect the value operand, not the type child. Alternatively, use one shared TS runtime-expression normalizer for these classifiers. Regressions should require no formal Def or formal-derived endpoint for every direct wrapper and composition, while preserving locals and all indirect/member/optional controls.

Self-critique: the finding would not apply to a genuinely indirect call, a member named `eval`, or a contract explicitly limited to literal unwrapped spellings. B-D15 accepts direct eval across JS/TS/TSX, and these wrappers erase to a direct call. No provider or corpus behavior is used to establish it.

Classification: **CLOSED INSTANCE of B-D15 / EVAL refusal**, with the already enumerated TS expression-wrapper shapes. Confidence **100/100** for the observed refusal-policy violation; the printed erasure and direct/indirect value controls are the discriminating evidence.

**S1 — SMELL / MATERIAL: the certificate does not independently census SEAM/EVAL/ARGS.**

Evidence: **static**, `~/prism-evidence/js-param-defs/prB/repair-r5/seam_census.rs:13–14`; its SHA matches the helper binding in R7 `binary-binding.json`. It calls the product's `js_ts_parameter_refusals` to generate all three class flags and `dfg_owner_name` to determine admission. R7 uses this census binary. `repair-r6/certificate.py:98–104` then accepts changed rows using those flags. Replay/provenance checks can independently validate bytes and counts while still inheriting the product's classification error.

Minimal control: W3's `eval!()` fixture. The product and census both say EVAL=false; independent TS erasure says direct eval. This demonstrates shared classification, not an observed false pass in a particular retained corpus. A wrong positive class flag could similarly excuse an out-of-class removal. Therefore the certificate establishes the mechanically checked partitions under the implemented predicates, not independent correctness of those predicates.

Fix: independently derive formal BoundNames, parameter-expression/body-binding seams, direct callees and mapped-arguments conditions from a separate syntax implementation, then join exact callable/endpoint bytes. Alternatively, independently audit the complete changed-owner population and record disagreements explicitly. Keep the current multiset/provenance checks as separate evidence.

Self-critique: this does not refute the 579-root accounting, prove any retained changed row was misclassified, or erase the independent Node/matrix controls. It is an assurance gap and is **not a standalone blocker**. An independent complete census elsewhere, if supplied and bound to these captures, would remove this finding.

Classification: **NEW FAMILY — certificate independence gap**, evidence-only. Confidence **100/100** that the supplied census shares the product predicates; semantic effects on the retained corpus remain unproved.

**S2 — SMELL / IMMATERIAL: IMPLEMENTOR-prB is historical, not an actionable final landing dispatch.**

Evidence: **static**, `docs/superpowers/plans/2026-10-06-js-param-defs/IMPLEMENTOR-prB.md:1`, `:8`, `:12`, `:16`, and `:31`. The current header still names R6's stopped continuation; the dispatch names `cb996630` plus the R2 patch and cache110, while the reviewed candidate is 2431cb10/cache112. It has no current landing section incorporating SPEC §8's E13/R7 decisions. The requested `MATRIX-param-env.md` is also absent in this plan checkout; I read the supplied `MATRIX-param-env-r7.md` and its retained historical matrix/current R7 extension instead.

Minimal state: a controller given this document alone to land the final candidate receives a historical STOP and old source/patch/cache instructions, not a procedure for the current candidate. The document explicitly labels that dispatch historical, so no incorrect landing is demonstrated and I do not tag it WRONG.

Fix: add a clearly current section binding the candidate/source branch, final semantic cache generation, accepted owner decisions, gate receipts and controller-owned Git actions. After repairing W1–W3, bind that section and its evidence to the resulting revision. Alternatively, link an equally explicit current landing brief and make the historical document an archive. Refresh the SPEC status/header and matrix path at the same time without rewriting historical receipts.

Self-critique: the final review brief itself correctly binds 2431cb10 and the owner decisions, so this did not compromise this source review. A controller following that brief rather than the historical dispatch would avoid the ambiguity. This is nonblocking documentation maintenance.

Classification: **CLOSED INSTANCE of S-d/S2 — dispatch/custody documentation**.

**Verification and evidence limits**

I built candidate and current main libraries offline with `--release --offline --locked`; candidate also built the CLI. The byte helper was compiled against each fresh library. All **64 synthetic fixtures / 128 side comparisons** are byte-identical between fresh and retained packet binaries. The candidate's existing callback suite passed **56 tests, 0 failures, 1319 filtered out**. It does not cover the three failures above. The initial archive omitted 23 embedded assets and failed to build; that probe was inadmissible, corrected by exporting the exact assets, and supplies no candidate failure attribution. No production/test/registry source was edited.

Named seam rows in my control are byte-identical to main; the non-formal control remains admitted. Synthetic seam formal refusal, EVAL parentheses versus optional calls, ARGS strictness versus nested non-arrow/arrow references, both-direction default/body fences, and kill-only nested writes versus nested rebinds behave as expected in the saved controls. The existing navigation tests passed: synthetic symbol resolution/seeds refuse the owner spelling, symbol views hide its variables, and location taint seeds can see them. Eight additional non-Flow type-text controls pass: TS/TSX function types/interfaces and JS comment/string type text produce no synthetic rows; real annotated TS/TSX arrows define `value`, not their `boolean` type token.

I independently summed the final per-root ledgers: **579 admitted = X + Xi + T + 576 SecBench; 7 excluded; 0 unprocessed**. Effective tables reproduce the brief: X ADDED 15,513 CORRECT, LOST 3,415 WRONG / 2 UNDECIDED; T ADDED 19,434 CORRECT, LOST 374,016 WRONG; SecBench ADDED 288,223 CORRECT / 82 WRONG, LOST 948,304 WRONG / 6 CORRECT / 25 UNDECIDED, plus RELABELLED 1 CORRECT / 1 UNDECIDED. Xi matches X. The three R7 binary hashes match their bindings. These are audited supplied counts, not freshly rerun corpus captures or correctness verdicts.

For E13, all **82 ADDED + 6 LOST** STOP rows map to **22 files**, and every file has `@flow` in its opening bytes. This confirms the disclosed population's boundary; I do not reopen its acceptance or treat those rows as blockers. W1–W3 are separate valid-JS/TS binding/refusal defects, with clean parser trees.

The excluded roots are swiper, the two total.js roots, atropa-ide, natural and three (six main-producer timeouts), plus clean-css (main killed with exit -9, cause unknown). Their exclusion is explicit and earns no semantic credit. I did not rerun or attribute those producer failures to the candidate.

The UNDECIDED rows are honestly retained as uncertain, not credited as WRONG. Both X rows concern aliases `lastX/lastY` sourced from the outer `event`, then used where the nested callback has a different `event` formal; removal is benign on this sample. A sampled CEjs row connects an `o` local in `new_node` to `o` in separate `doAlertResize`, also benign. Mithril's ambiguous-owner row and the remaining uncertainty are not resolved by those samples; there is no blanket proof that all 25 SecBench losses are harmless.

I inspected the certificate's 26 distinct SyntaxError-exception owners/proofs. Recorded failures are mainly raw Flow/type syntax or recovered fragments rejected by Node's wrappers, not evidence of a specific ECMAScript parameter early-error rule. Together with S1, this limits the stronger semantic interpretation of the certificate. The focused synthetic strict/default, legal-undefined and invalid-catch controls plus the R7 kind-matrix test are positive evidence for their particular guard paths; I did not independently reconstruct every main-table SyntaxError override in its original syntax/context.

The retained nextest log ends with **5,232 passed, 1 skipped**. R7 reports 2 doctests, effective mutants 93/93, clippy parity 235/235, Tier-A matrix 178/178 and TS/Node quick VALID. Those gates were not rerun here and do not certify the missing fixtures. The brief's controller Rust quick and private F aggregates are supplied evidence, not my executions.

Cleanup: the isolated source/build/fixture directory and both target trees have been removed. No workspace target directory was used. The durable evidence file preserves the observations, and prB-final-sol.snapshot.tgz snapshots the review and evidence. Controller owns repairs, commits and any landing decision.

**Verdict: FIX (closed, enumerable fixes).** Repair W1–W3 on the retained candidate, preserving the accepted seam/Flow boundaries and the positive controls. At the declared **1/1 cap**, these are closed instances with bounded remedies; I have not dispatched another round, restarted the artifact, or proposed a spec change. S1 and S2 are nonblocking SMELLs.

What I did not check:

- The full local test suite, mcp-feature build, all mutations, clippy, Tier-A or performance; only the fresh release builds, 56 callback tests and described probes were run locally.
- Fresh whole-corpus row/call-site/non-JS/navigation captures, every retained row's oracle verdict, the complete independent semantic class census, or all SyntaxError overrides.
- Semantic behavior of the seven excluded roots; every UNDECIDED loss; exhaustive grammar completeness beyond the enumerated controls; an exhaustive search for Flow-style misparses outside annotated files.
- Private F source or the controller's quick-run artifacts; no corpus-package execution, Git/PR writes, implementation fixes, merge or adoption.

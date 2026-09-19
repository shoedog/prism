# Slice 2 independent review — round 2 of 2

**APPROVE — WRONG: 0 / SMELL: 1. Core acceptance G1–G4 is satisfied. Final Rust gate reconciliation is now complete; see FINAL-ACCEPTANCE.md for final totals and validation limits.**

Reviewed final commit `7fc89c98bd3eaae99dc9dc959097b942ac96fe8c`, tree `dff38f065792cb6d7c20bb32c146b438bde8fa89`, against accepted base `d2bbe7074d12fc98280248313d9767d237be33b8`. Its production is byte-identical to round1 checkpoint `498fb6f68acaa7b87a5551b840967a9c28b50011`. The only source delta is the permanent occurrence-test file, SHA256 `0a36d430d9f298f04f1eea3ba5582ce081b81feb076ca66b5c3fcb372f743ff3`; three other changed paths are custody documents. Independently compared final committed production/test bytes to the isolated test archive and verified all source-manifest hashes.

This is the final capped review, targeted to the complete four-group round1 list. No new production review or optional polish was opened. No source edits, git writes, broad suite reruns, publication or additional agents were performed.

## Findings and closure

**WRONG: none.** Round1 SMELL S1 remains nonblocking: repeated validation of a same-line/path exact bucket may scale quadratically across repeated lookups. The bounded measurements below do not establish a production slowdown or corpus-wide performance guarantee; speculative optimization is not required.

| Round1 group | Closure evidence |
|---|---|
| G1 asserted-member exact vectors | Permanent O06 now pins all four full-field/base × left/right rows for repeated `(runtime as any).X` in TS/TSX, all Exact, with `runtime.Y` excluded. Independently checked literal source spans83–101,84–91,102–120,103–110 and target tokens21–25,26–31. Final candidate passes. The identical permanent test body on unchanged base fails behaviorally for both dialects: only field83–101→left survives. |
| G2 actual owner collision | Permanent O12 now authenticates two byte-distinct `same` functions with identical name/line2 identity, rejects their call sites and boundaries, and pins outer147–152→input21–26 Exact. JS/TS/TSX pass on candidate and unchanged base. This preserves the bounded refusal, with a nonvacuous positive outer control. |
| G3 Step5c and legacy queries | Permanent O13 now pins exactly one ReturnInput from callee Use47–52 to ReturnValue(return40–53,slot0,expression47–52), preserves ReturnFlow→caller resultDef57–63 with suppress_shortcut=true, and preserves only producer Def21–26→earlierUse33–38 NameOnly(CfgIncomplete). Fresh/warm complete graph parity, first-wins var_node/var_node_for_location and all_defs_of are asserted. Candidate passes across three dialects. Identical permanent body on unchanged base fails only on empty ReturnInput rows in fresh/warm across all three; remaining assertions pass. This is intended bounded additive connectivity, not caller RD recovery or a confidence promotion. |
| G4 measured costs and scope | Raw fixed-fixture timings, index microtimings, pinned Tokio summaries and canonical multisets independently reconciled. Tokio pin/tree and clean state verified. Four prepared corpus roots are Rust/Go/Python; the explicit absence of a pinned real JS/TS/TSX corpus is retained as a measurement limitation. No acquisition/full multicorpus run was authorized. |

## Independently executed review evidence

- `round2-focused.log`: **16 selected, 16 passed, 0 failed, 0 ignored**, final permanent test bytes, isolated source/target.
- `round2-base-controls.log`: **3 selected, 1 passed, 2 behavioral failures**, identical final G1/G2/G3 bodies on unchanged base in the same environment. G1 prints both dialect deficit vectors; G3 prints missing ReturnInput on all six dialect×fresh/warm cases; G2 passes. No compile/setup failure or zero-selection result was used as evidence.
- `round2-base-test-harness.rs` preserves the exact selected permanent test bodies plus existing public-API helpers. Only test registration/instrumentation differs from archived base production.
- Round1 remains applicable because production is unchanged: focused14pass, all original CPG253 plus5 independent supplemental probes=258pass; six public-API desired tests fail behaviorally on base; TS/TSX distinct serial Step5b/full/incremental/warm parity, actual collision and distinct-node ambiguity independently pass. No scheduler control was mislabeled an independent implementation.

## Measurement reconciliation

Fixed O01 population per dialect: base10nodes/10edges; candidate11nodes/12edges. Fifty debug builds totaled JS28.728→29.975ms, TS32.486→34.351ms, TSX32.393→33.869ms. Candidate index reconstruction10,000 samples totaled26.409/26.202/26.311ms, respectively. This micro uses the test-only graph reconstruction helper; production assembly builds the equivalent index during materialization. It is not a direct end-to-end production lookup benchmark.

Pinned Tokio `ecb5125a6787b9d8eb818b1b00973bcd55ae77c0`, tree `4a9edd6dc27bdd9f876195a8720d5a324f1c3b3e`: 781 parsed/63 skipped files,15threads. Both graphs122,311nodes. Base182,557edge instances/candidate182,553. Independent parse of both raw multisets confirms **298,088 identical unique canonical node/edge rows**, with exactly four DataFlow(NameOnly(CfgIncomplete)) edge multiplicities changing2→1. No unique graph fact is lost. Single debug CPG samples33.206s base/32.862s candidate; total census44.854s/46.567s. These bounded samples establish scale and non-JS preservation, not JS occurrence prevalence, lookup complexity or a throughput budget. No pinned real JS/TS/TSX corpus measurement was available.

## Gate evidence already reconciled

Read supplied nonRust receipt, actual logs and matching hashes:

- Grammar: baseline and shipped parser trees reproduce.
- Authority: parsed40results/0failures.
- Node:786tests,785pass/0fail/1expected grammar-archive skip across reported46active modules.
- Python:896eval/tests +44adoption unit =940pass, no skips.
- Tier-A matrix:159actual `ok` log rows; report matrix159.
- Tier-A quick: **INVALID** with `corpus_sha_drift:498fb6f68aca != pinned20c8490591a3` and C-method4/6 successful oracle probes. SUTerror_rate0, report SHA256 `c6cc88dbf0c75a9ffdb960d1919c8cda9febc74f365b2e4deb89f09124363775`. This is not a clean accuracy gate, attributed regression or rebaseline authority.

The nonRust frozen binaries predate the test-only hardening. Their production-equivalence adoption is supported by the final committed diff and source hashes. Final Rust full-default/MCP/widest/examples/clippy/fmt/diff receipts remain to be reconciled separately; checkpoint totals4532/0/1,4725/0/1,4748/0/1 are supplied checkpoint evidence, not final-source completion claims.

Explicit exclusions remain: three historical imported-props source-custody tests requiring absent real-sites archive/provenance inputs; one expected grammar archive branch skip; live `adoption/tests/test_prism_adoption.py`; human-triggered full Tier-A multicorpus. Quick was not retried or rebaselined.

## Custody and cap

One documentation typo was identified and routed for normal closeout: source-manifest's checkpoint tree field is wrong. Actual `498fb6f6^{tree}` is **`7437d02af2f86994b9ed43fcaa3b813a3fa63fb5`**, matching the nonRust receipt; source/test hashes and reviewed commit are unambiguous. Correct that field in the documentation fold; this is not a production or behavior blocker.

At cap: **converged**. Four finite acceptance gaps are closed, production unchanged, zero WRONGs, one nonblocking SMELL. No third review or source change is required. Root may proceed to final evidence-only reconciliation and durable custody, preserving the explicit validation limits. Local review approval does not authorize publication, merge, adoption, live effects or full multicorpus execution.

## Evidence-only closeout

Final Rust logs reconcile4534/4727/4750pass, each0fail/1standingignore; examples32pass. Clippy completed with254warning-prefixed lines (including summaries; no blanket base attribution), fmt/diff independently pass. Source/test remains7fc89c98. FINAL-ACCEPTANCE.md supersedes the pending gate state recorded at core-review time. No third source review occurred.

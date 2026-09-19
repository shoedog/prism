# Exact caller-read slice — final acceptance

**APPROVE — 0 WRONG / 1 nonblocking SMELL**, with the validation limits below.

## Accepted source

- Commit `14e86083c2c754ed412d440742bd5763be388e42`.
- Tree `a0411db6d858fe34806376ce41d2621853848d8b`.
- Source manifest SHA256 `310b5152d598800a7520181fe53be58cb3677d3ca7421cc900c8005e78e92aa7`.
- Approved budget850 production/1800 total; actual798/1662. Internal supplemental exact caller-read facts retain their own RD labels, closed recursive syntax admission, legacy compatibility and exact validated CPG endpoints. No global VarLocation identity, kill semantics, public API or loop/member/default authority expansion.

This is evidence-only final reconciliation after the one explicitly declared post-design validation round. The original implementation cap-two PARK-DESIGN and accepted closed-grammar design remain recorded; no silent review reset. No new source review or suite rerun was performed here.

## Core evidence

`POST-DESIGN-CORE-REVIEW.md` SHA `fedd9869d69f923acf2a66e14f3cd4223540294ffd3000407e780259123eeca6` records independently authenticated source and executed same-environment controls:11 focused tests passed;144 public dialect/cases produced4009→4021 complete primitive rows with exactly12 source-anchored expected additions, zero removals/unexpected additions. All132 refusal cases/3681rows equaled unchanged base; saved legacy184 and refusal753rows also matched. Saved malformed-endpoint and missing-fact/boundary controls passed.

The remaining **SMELL** is permanent regression assertion strength: some committed typed/Unicode/refusal assertions are narrower than the complete independent oracle. That oracle was actually executed and is preserved with fixtures, expected tuples and full outputs; this is nonblocking and not a missing behavioral verification or demonstrated bad result.

## Final-source verification reconciled

| Gate | Actual terminal result |
|---|---|
| Rust default |4559 passed /0 failed /1 ignored |
| Rust MCP |4752 passed /0 failed /1 ignored |
| Rust widest (`mcp detached-owner-audit`) |4775 passed /0 failed /1 ignored |
| Examples |32 passed /0 failed /0 ignored |
| Clippy /fmt /diff |Clippy exit0,258 warning lines including summaries; fmt/diff clean |
| R07 genuine cache |1 passed: genuine96 direct Miss, rebuild97, warm Hit with complete observed graph/legacy/call/query parity |
| R07 synthetic cost |1 passed, all9paired scenarios;1000-callable medians JS+6.815%,TS+1.029%,TSX−1.249%; specified25% threshold not breached |
| Node active population |785 passed /0 failed /1 skipped (786 total) |
| Python deterministic /adoption units |896 passed, separately44 passed |
| Callable authority |40 results,0 failures |
| Grammar verification |Both unchanged baselines and shipped patched parser trees reproduce byte-for-byte |
| Tier-A matrix |159/159 ok |
| Tier-A quick |Terminal **INVALID**; completed report, no retry/rebaseline |

Rust receipt SHA `4cb6ab550932b7ff9c1206267f99dd12a8cb7563a179fd39246c59050039bcf7`; R07 receipt SHA `00628af266e4aa4b19ab70887145c1fbb38718fdcdb50d833f2b9a8f4eab65d7`; nonRust receipt SHA `7dc8996150ad21970f0b5e1db1c83c3680ec287d01b8fd7106c4cee1943d7625`. Actual terminal logs, cost samples/medians, raw quick report and artifact hashes were inspected.17 nonRust log-manifest entries,3 frozen release/helper binaries and both2-entry matrix/quick binary manifests match. Quick report/run JSON SHA `e909333b769d02028312e38d824b98e59fc7f391b039a29c9df33c12c66e25e3`.

## Explicit validation limits

- **No Tier-A accuracy, regression or flip claim.** Quick report binds clean corpus/SUT/harness14e86083 but sets `baseline_invalid:true`: `corpus_sha_drift:14e86083c2c7 != pinned20c8490591a3` and `stratum C-method:4/6 successful probes`. SUT error rate0.0 does not make accuracy valid. It finished in roughly four minutes; this is not the predecessor's timed-out quick run. No rebaseline/retry occurred.
- Standing Rust ignore: `resolution_test::slice_elem_variant_reserved`. Node's one expected skip is the grammar archive tamper branch requiring `PRISM_GRAMMAR_ARCHIVES`.
- Historical imported-props audit module excluded because all five provenance inputs, including `real-sites.jsonl`, are absent. Live `eval/adoption/tests/test_prism_adoption.py` excluded; no authorization for live adoption. Full Tier-A multi-corpus run excluded as human-triggered.
- Earlier Python attempts failed before useful behavioral collection due missing pytest/local prism-eval in the isolated environment. Those are inadmissible setup results, not source regressions. The locked offline repair and final preflight precede the terminal896+44passing runs; no aggregate retry total is substituted.
- Clippy warning origin was not compared against unchanged base. Cost measurements are descriptive fixed synthetic parse-plus-CPG samples; peak memory unavailable. No broad performance or real-corpus recall claim. Genuine old-cache rejection is not isolated version-only causation; nav53 remains source-bound without a navigation artifact claim.
- Verifier reports clean source custody after copying and removing only lane-generated quick files/venv. Publication, merge, deployment and production effects are separate and not authorized by this acceptance.

**FINAL VERDICT: APPROVE for the bounded source contract, with completed runnable verification and the explicit INVALID Tier-A accuracy limitation.**

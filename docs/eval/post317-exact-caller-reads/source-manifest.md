# Exact caller-read occurrence final source freeze

- Planning base: `b4e421c7cdfa2076f6cfea50fb18d3ee16bdf010`, tree `7db35e8ff2a070531b01800eda317bdddb20c4d1`.
- Production predecessor: merged PR317 `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`.
- Accepted spec: `docs/superpowers/plans/2026-09-18-post317-next-increments/specs/01-exact-caller-read-occurrences.md`, SHA-256 `52f8648fa0c4c0f07817dc9985a379dabe9943600486cfcc8c1e270c22485b50`.
- Cache contract: CPG `97`; navigation `53` unchanged.
- Design-repair documentation checkpoint: `d2d1d2ce8f6a192e52d4adf5178f9690fbd0ad79`, tree `052b6a692b3b214f397e8fb34b1978351a4347d8`.
- Guardrails: final formatted population is **798 production-file changed lines / 1,662 production-plus-test changed lines**, below the controller-amended **850/1,800** limits. The amendment supersedes only the historical 700/1,600 planning estimate; it does not rebaseline behavior, scope, evidence, or review requirements.
- Candidate source/test commit: pending controller checkpoint. The hashes below are the frozen bytes submitted for that checkpoint and the one post-design validation round.

## Source and test bytes

| Path | Role | SHA-256 |
|---|---|---|
| `src/ast.rs` | closed named-node, field, token and lexical admission grammar | `0fc8a5848b92ddfc117270c2c359799482cf71fed57723421219a0d1cf01d82e` |
| `src/data_flow.rs` | exact fact identity, production and lifecycle | `40f6af402a9a092bf17921042b4b3400e5b8f95965421c608f7e16f69ca8f317` |
| `src/cpg/reaching.rs` | same-solve exact classification | `28e16f27a5a5ee8f8c7b60ba622c2910d5ef14df6c45628e186a22cb932a5c2d` |
| `src/cpg/build.rs` | exact-only validated Step 4 union and test module | `021d7b5454dff7cafc0b952cb470d1bb8a84189cfb5d15a6d9c61cb081bc8f5f` |
| `src/cpg.rs` | internal reaching-definition export | `87bb7cb0255ad7876a179258b702f58056a453ffa3c61220e84066eb7f539b10` |
| `src/cpg_cache.rs` | CPG cache 97 | `419dbd933c39caf8dfadbde2f818bb13eac3ca41e9f52e34c892623e26687e2f` |
| `src/cpg/same_line_occurrence_tests.rs` | predecessor golden and lifecycle assertions | `6be2416827b9d15f385d0978a738c6e2e871be70db8fd9b9832773467811fb68` |
| `src/cpg/exact_caller_read_tests.rs` | A1-A7/R1-R12, unknown policy, exact facts and lifecycle | `c6e0751e5dddd0ac6b8637b8f62b31a835d93ce5b740ac871a36613da2ea8ac3` |

Historical replay patches for the pre-design freezes remain under `/private/tmp/prism-post317-implementation/freeze2/`. The preserved design-repair patch is `/private/tmp/prism-post317-implementation/design-repair-overbudget.patch`, SHA-256 `c87c5b3c83bacead12cb5362d66f7fab041e7613b207e277c8dd03d5c8f6eec4`; its bytes match the final source/test hashes above.

## Focused evidence

- Final design-repair population: exact caller `9/0/0`; predecessor same-line `16/0/0` with Rayon 1 and `16/0/0` with Rayon 4; optional-parameter preservation `31/0/0`; inert-default preservation `22/0/0`. Log SHA-256 values are `b274a036a9e07c52e03be5b4ef75ffd089558906bbd2d0c218fb7e5ba59d67be`, `d2997b6bc899505fcae754527dd5c5725a4f70f73d7236fbbe61dd2a7943567c`, `3c4d9eb52505156ae147dd3ad9fbc81e44ba90efbac44acb284c0b483b97f33e`, `ada5acc2e7d89e2e3fc99c49c6c9c2e6b4f752e5ea31cf0c9cf98ef756dbeca1`, and `eef2b855406255256b0250b430f47c1779b9ff53121ce54723ffa1635fc11d39`. `cargo fmt -- --check` passed.
- The complete R1-R12 refusal population is behavioral RED on parked predecessor `1a8354b2`: 44 labeled dialect/case failures with exact later tuples, log SHA-256 `9c233d5d4fd8658f405e4579fcfdf0b5b34074dc51b7fb320253760100e96dfb`. The earlier `using`-in-JavaScript setup failure is retained as inadmissible and was corrected before this RED run.
- Compact focused-freeze receipt: `/private/tmp/prism-post317-implementation/design-repair-focused-freeze.md`, SHA-256 `0d01dd610825ea8e5b47ad959c585bdba9bc0c0699a28e6dc6c3940fc8289ba9`. These results establish freeze readiness, not correctness approval; the one post-design validation round and final gates remain pending.

- Unchanged-base public assertions: one preservation test passed and two desired-flow tests failed with the complete 15 missing later rows. The same patch in an independent `git archive 9fb6c823` reproduced `1 pass / 2 fail`; logs SHA-256 `02bcf600386521df35c863bb1f646e0bfa6f22fc87161be59fc7611c39ef0943` and `202229461f9fb498b526460d348614a593c6856523539003e2b3be75a1057ef6`.
- The narrow ReturnInput golden is independently behavioral RED on base across JS/TS/TSX fresh and warm: `0 pass / 1 fail`, log SHA-256 `62dffb59f5f73bd5c47e21383e17dd60f3ee009f46ab53cf89612a6b53af609f`.
- Candidate exact controls: `8 pass / 0 fail`, log SHA-256 `395937b009618b079d279690319401f65ae1310b5d69d21958c9693e8d3257ac`.
- Candidate predecessor controls: `16 pass / 0 fail` with Rayon 1 and Rayon 4; log SHA-256 values `91fb2e02506cc33e46bef61d5ac96e47938f05c73e027ef560425b330a1d6dc8` and `65622837269f9ecf2b0817dcbf27965165b24e4e8442912b829aba017ca61464`.
- Optional/default preservation: 46 optional-name selected tests and 5 inert-default tests passed; logs SHA-256 `66217999b5ad1564520f082a3bd4f9c1069f67eb66ca06ec66acf21102445bba` and `198f576cbab80a875fdfa0389b171710208ed636d606e58c2af24685502432c1`.
- Round-one finite repair: exact caller tests `8/0/0`, predecessor same-line tests `16/0/0`, optional-parameter tests `31/0/0`, and the exact inert-default preservation control `1/0/0`. Log SHA-256 values: `d150e1dd6c8e8750187daa559315d2e1e2f4c2dacfb86dacfb53f6537e7fd78e`, `5f727825e2757e25bcad4c44eb0750826431e6c51004e6e3f96ecc321def164e`, `7c10711bbf98edf2d35a5aa3425e5a423e3209af67af8cb84dbcd7529fed2686`, `250a7f8305ca5954233d8c5141b20c63827c06ade271bb076af9462e596c06a4`.
- Round-one reviewer report: `/private/tmp/prism-post317-implementation/review/REVIEW-round1.md`, SHA-256 `810d3974ce4250868da86926261d5cab78acae022f3e6193c9b723d7c72c9aae`; all four bounded WRONG mechanisms and two evidence gaps are represented by permanent complete-graph, refusal, lifecycle, and identity assertions.
- Hypothesis/probe/result log: `/private/tmp/prism-post317-implementation/hypothesis-probe-log.md` (updated after this source-manifest edit; bind its final hash in the controller checkpoint receipt).

Historical R07 cache evidence on the superseded first production freeze passed v96 refusal, v97 rebuild and warm parity; receipt `/private/tmp/prism-post317-implementation/r07/cache-control-receipt.md`, SHA-256 `05cb69c50d7023b6a918eee1fbea4e480f633bc19e7388ba838d2bbd59ec9453`. The independent verifier must replay genuine v96 refusal, final-v97 rebuild, fresh/warm parity, and candidate cost against the final checkpoint; the historical result is not final-source evidence. No publication or merge is authorized here.

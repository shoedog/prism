# Exact caller-read occurrence freeze

- Planning base: `b4e421c7cdfa2076f6cfea50fb18d3ee16bdf010`, tree `7db35e8ff2a070531b01800eda317bdddb20c4d1`.
- Production predecessor: merged PR317 `9fb6c823e5fa8e6c07ace8c5b41abe9bc5acf090`.
- Accepted spec: `docs/superpowers/plans/2026-09-18-post317-next-increments/specs/01-exact-caller-read-occurrences.md`, SHA-256 `52f8648fa0c4c0f07817dc9985a379dabe9943600486cfcc8c1e270c22485b50`.
- Cache contract: CPG `97`; navigation `53` unchanged.
- Guardrails: 565 production-file changed lines and 1,236 production-plus-test changed lines, below the accepted 700/1,600 limits.

## Source and test bytes

| Path | Role | SHA-256 |
|---|---|---|
| `src/ast.rs` | callable and binding admission | `6120ad85cc50f9db819834efa5ac4034502c519d0c1959f72c79a01208c8f883` |
| `src/data_flow.rs` | exact fact identity, production and lifecycle | `86b8f438b68e7eb781e92e5c75a5cc07e67727d3914800a1142136ead36ac62c` |
| `src/cpg/reaching.rs` | same-solve exact classification | `28e16f27a5a5ee8f8c7b60ba622c2910d5ef14df6c45628e186a22cb932a5c2d` |
| `src/cpg/build.rs` | Step 4 union and test module | `3b76680e9a809c7fbc087548f4e72170a8a07b3e9b307e41ee0eb2941ce12e90` |
| `src/cpg.rs` | internal reaching-definition export | `87bb7cb0255ad7876a179258b702f58056a453ffa3c61220e84066eb7f539b10` |
| `src/cpg_cache.rs` | CPG cache 97 | `419dbd933c39caf8dfadbde2f818bb13eac3ca41e9f52e34c892623e26687e2f` |
| `src/cpg/same_line_occurrence_tests.rs` | predecessor golden and lifecycle assertions | `3637c7f4addeab62cca5af5a6dba7ec7290f25258888cec4720b0a9d68ca17e5` |
| `src/cpg/exact_caller_read_tests.rs` | R01-R06 acceptance population | `8bdc6029624e6a57eecabd3402a8e5247b91db92cbc8cce24269febec9b80c6c` |

External replay patches: `candidate-tracked.patch` SHA-256 `a90a1dd22c6c5df37387395faedc60a05f5012cb6a3a982c720d66d6991669bc`; `new-test.patch` SHA-256 `7e13806cc902e2ba61ab4da59ea68102fbb65bd25cb6e0c15aebf8da2a63b2ba`, under `/private/tmp/prism-post317-implementation/freeze1/`.

## Focused evidence

- Unchanged-base public assertions: one preservation test passed and two desired-flow tests failed with the complete 15 missing later rows. The same patch in an independent `git archive 9fb6c823` reproduced `1 pass / 2 fail`; logs SHA-256 `02bcf600386521df35c863bb1f646e0bfa6f22fc87161be59fc7611c39ef0943` and `202229461f9fb498b526460d348614a593c6856523539003e2b3be75a1057ef6`.
- The narrow ReturnInput golden is independently behavioral RED on base across JS/TS/TSX fresh and warm: `0 pass / 1 fail`, log SHA-256 `62dffb59f5f73bd5c47e21383e17dd60f3ee009f46ab53cf89612a6b53af609f`.
- Candidate exact controls: `8 pass / 0 fail`, log SHA-256 `395937b009618b079d279690319401f65ae1310b5d69d21958c9693e8d3257ac`.
- Candidate predecessor controls: `16 pass / 0 fail` with Rayon 1 and Rayon 4; log SHA-256 values `91fb2e02506cc33e46bef61d5ac96e47938f05c73e027ef560425b330a1d6dc8` and `65622837269f9ecf2b0817dcbf27965165b24e4e8442912b829aba017ca61464`.
- Optional/default preservation: 46 optional-name selected tests and 5 inert-default tests passed; logs SHA-256 `66217999b5ad1564520f082a3bd4f9c1069f67eb66ca06ec66acf21102445bba` and `198f576cbab80a875fdfa0389b171710208ed636d606e58c2af24685502432c1`.
- Hypothesis/probe/result log: `/private/tmp/prism-post317-implementation/hypothesis-probe-log.md`, SHA-256 `f3108b140661d6d2928896f9271466f50d8fb17138d5851a44efb215cfe7c84f` at freeze preparation.

R07 genuine-cache and finite-cost evidence is delegated against these exact source bytes. This manifest becomes review-ready only after that receipt is linked; no publication or merge is authorized here.

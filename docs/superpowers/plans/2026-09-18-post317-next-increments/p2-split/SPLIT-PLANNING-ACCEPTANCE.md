# P2 split planning acceptance

**Verdict:** APPROVE after round 2 of 2. This accepts the staged planning artifact only. It dispatches P2a syntax/integrity/frequency work under controller authority and leaves P2b native readiness deferred.

## Bound artifact

- Artifact manifest: `artifact-manifest.json`, SHA-256 `3b5e61b3f802ed961a46c2260f42ead3d387b7e0d86a1092119fd0cac323223a`.
- P2a normative spec: `P2A-SPEC.md`, SHA-256 `6b681c35baa365260f9466d3479d2b04a5398add57b8a5229e4f72386a5e4d0c`.
- Original full-contract reference: P2 spec SHA-256 `e6e1bb3e8a928943149e59cfa29598c1e15142d10468c32fe5704a70132c1f71`.
- Input packet: 414 selected Excalidraw files, 5,036,151 bytes, 5 declaration-only, 62 test-path; manifest SHA-256 `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696`.
- Native source binding for archive runner: accepted Prism `14e86083c2c754ed412d440742bd5763be388e42`, tree `a0411db6d858fe34806376ce41d2621853848d8b`.

## Round 1 findings and resolutions

1. **WRONG — unspecified cross-observer join key.** The draft did not say how callable/parameter rows formed exact, unmatched, or ambiguous joins. P2a now defines callable and parameter candidate arrays, requires a unique parent callable group, excludes raw trivia/full spans from equality, and emits deterministic zero/one/many Cartesian rows with cardinalities and match ordinals.
2. **WRONG — unrecomputable IDs.** `context` and `dimensions` appeared in callable/parameter IDs without ordered scalar schema. P2a now fixes both arrays, field order, null/boolean/unknown rules, and controls for class/object/computed contexts.
3. **WRONG — nonexclusive file statuses and numerators.** Recovered declaration files and one-observer-unavailable files had more than one plausible terminal state. P2a now has an exclusive five-status priority table plus separate clean/recovered/unavailable and valid/recovery-tagged/cross-join numerator rules.

Round 2 verified all seven artifact hashes and found no remaining WRONG or SMELL. The P2a packet remains `prism.p2a-syntax-census.v1`, `measurement_kind:syntax_frequency_only`, and `authorizes_runtime_edge:false`.

## Scope and gates

P2a may report only authenticated source syntax inventories, exact joins, recovery/refusal coverage, and separate destructuring/rest/arrow frequency tables. It does not establish native owner/slot/entry/call-target/Step5b readiness, constructible demand, runtime flow, compiler semantics, dependency closure, private parity, or production admission.

P2a retains the original active ceilings: Rust/schema 900 lines, compiler observer 350, tests/fixtures/helpers 850, total active source 2,100; runtime 512 members, 8 MiB source, 200,000 rows, 128 MiB output, and 15 minutes. The same-schema behavioral baseline RED, complete focused controls, frozen core review cap two, public cold/repeat, and stale/refreshed/restored synthetic mutation sequence remain mandatory. No public parsing occurs before core approval.

P2b is deferred. It requires a separate dispatch, budget allocation, review cap, accepted P2a source/schema/output binding, and fresh native-readiness proof before any owner/slot/entry/call/Step5b claim.

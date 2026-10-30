# W4 admission grammar — design-only acceptance

## Accepted bindings

**ACCEPT DESIGN.** This is the one targeted design escalation review after implementation review round2 ended PARK-DESIGN. It is not a third implementation review, implementation acceptance, or authority to bypass the controller's next validation-cap dispatch.

- Existing parked source: `1a8354b2d80fdedc3b1ab877f5b63ec50913a09d`, tree `0f89f8488327ed8c4184289b213b81970ce9118e`.
- Main amendment: `/private/tmp/prism-post317-implementation/design-gate-closure.md`, SHA256 `7808bee475df710769cc73ea19cede21d0b0a34fc80108d3922fcb6623979d5f`.
- Normative child/token/lexical appendix: `design-anonymous-token-table.md`, SHA256 `931005cc7a7f60f5b39e19592b22976028c1fe2642a00266732b6da2cbd3a2b1`.
- Both author-confirmed frozen; production/test tree unchanged. The accepted contract is the two artifacts together; earlier draft hashes are superseded.

## Why the mechanism is now closed

The amendment replaces “accepted statement followed by default-accept descendants” with a finite recursive language. Every runtime-bearing named node and field is explicitly claimed; every visible anonymous child is checked in its parent context; every admitted identifier atom receives its lexical check. Unknown node, field, modifier, operator or unsupported leaf form refuses expansion. Rejection does not recurse into a rejected wrapper seeking an accepted child.

The one-pass grammar inventory covered all admitted JavaScript productions and each TypeScript override, including the inherited productions used by TSX. Pinned node-schema SHA256s were authenticated. The appendix accounts for ordinary function/parameter/declaration/return punctuation, calls/arguments/member syntax, assignment `=`, binary `+`, and TS non-null `!`; there is no generic punctuation/modifier skip. TS optional-call `?.`, assignment `using`, and definite-assignment declaration `!` are explicitly refused in their respective contexts. This is a positive context contract rather than another class denylist.

Erasure is limited to the exact reviewed TS/TSX type fields. Runtime children remain recursively checked. Strings and `as_expression`/`satisfies_expression`/`type_assertion` were deliberately removed from admission after pinned schema inspection showed that the initial draft's “leaf”/“named type field” assumptions were false. This narrowing preserves the existing literal R01/R02 and required lifecycle positives, without adding type-wrapper authority. Non-null expressions remain checked wrappers with one runtime child.

Runtime identifier/property-identifier leaves reject raw backslashes and exact `eval`, including root/parameter/declaration names and callee descendants. Ordinary unescaped Unicode remains allowed; erased type-only identifiers remain outside runtime inspection. This closes escaped direct-eval/binding spellings without introducing decoding or global identity changes. Parenthesized/non-null callees are already outside the admitted call-function forms, and the leaf rule independently prevents wrapper-based weakening of reflection refusal.

## Concrete corrections resolved during this design review

1. Nonempty string nodes own named content/escape children; they are not AST leaves. Final design refuses all strings, with an explicit negative and numeric-literal positive.
2. TS `as`/`satisfies` type components are ordered children, not named fields; TS type assertions use leading type arguments and are absent in TSX. Final design refuses these wrappers instead of pretending field-only erasure supports them.
3. The failure summary now correctly says the12 new rows occur after the class-field boundary; the field-initializer edge existed on base. Decorated class refusal does not falsely assume JavaScript parser recovery.
4. Anonymous tokens and runtime leaf spellings are fully covered by the normative appendix, including the source-backed distinctions above. No unknown accepting wildcard remains in the design.

Schema evidence: `design-schema-proof.json`, SHA256 `320a20a4b2668fac3d4823cf26f4e6b0b50ecd8cdd5aedd0f1b686d6069bfa75`. Pinned `grammar.js`/`common/define-grammar.js` definitions, rather than node-kind spelling alone, supplied the anonymous-token and wrapper ordering proof.

Native reflection probe: `design-escaped-identifier-probe.js`, SHA256 `1e7e3136e354564eec21a3de914ff8a4ec3bc35da287f026e2eef915188a9e68`; log SHA256 `0337f52dd70a935fdcdbe26d804611962c1899674f78bae03c993cef418fde9b`. Escaped and parenthesized eval each mutate a local parameter from1 to2. Pinned TypeScript5.9.3 erases `eval!` to `eval`, and the emitted program also returns2. Indirect/global eval would not produce the local mutation. This is runtime rationale for the conservative grammar restriction, not candidate graph or test-suite evidence.

## Implementation and validation gates retained

- A1–A7 preserve authentic NameOnly/Exact producer pairs, fixed legacy output, ordinary `+`, and unrelated-binding positives beside member use/write. A7 type/nonnull fixtures must authenticate runtime byte endpoints and exclude erased type identifiers.
- R1–R12 provide the finite rejected population, including both class-expression forms, unsupported/default/unknown syntax, erased-wrapper exclusions, modifier contexts, reflection spellings and ordinary Unicode controls. The private unknown-kind sentinel and real `new_expression` negative must both exercise the production default-refuse route.
- Before production changes, stage exact fixture bytes and primitive oracles on1a835 and unchanged base9fb6 in the same environment. Record preservation-PASS separately from desired behavioral RED; no fabricated RED for a currently passing rejection. Preserve the already authenticated class-expression differences.
- Keep W1–W3 and S1–S2 closed, including the saved malformed-endpoint and184-row legacy compatibility controls. No new RD, public identity/query, member, owner, default, solver or cache semantics. CPG97/nav53 stay fixed.
- **Budget remains a pre-edit gate:** current643 production/1510 total; targets690/1595, hard guardrails700/1600. This review did not construct or count an implementation and does not claim the targets are feasible. Project before editing and recount formatted changes; stop for controller scope/budget action if they do not fit. Do not hide complexity in compressed code or silently weaken the grammar/tests.
- R07 genuine-cache/final-cost and the final-source full Rust/nonRust/Tier-A gates remain required. Historical partial gates are not final acceptance. Preserve all corpus/accuracy exclusions.

The existing source artifact remains parked until the controller explicitly authorizes the bounded repair with a newly declared validation cap. Retain the existing artifact and evidence; no restart, publication, merge or cleanup follows from this design verdict.

WRONG: 0
SMELL: 0
DESIGN VERDICT: ACCEPT

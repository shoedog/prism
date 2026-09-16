# Slice 1 round-one repair receipt

## Exact 16 default-suite failures

Unchanged-base control: 16 passed / 0 failed / 0 ignored in the reviewer's `ROUND1-base-controls-supplement.md`. Candidate evidence is in `evidence/round1-repair-hypotheses.md`, the two receiver logs, and the final focused log.

| # | Test | Classification and concrete result |
|---:|---|---|
| 1 | `contextual_prop_receiver_shape_barriers` | Mixed: removed the overbroad whole-owner error refusal, restoring its clean recovery call; its two arrow/generator calls now have explicit zero-site assertions because their nearest callables are unindexed. |
| 2 | `local_callable_alias_authority_and_receiver_barriers` | Repaired regression: clean call beside a sibling recovery node is retained. |
| 3 | `local_callable_interface_authority_and_receiver_barriers` | Repaired regression: clean call beside a sibling recovery node is retained. |
| 4 | `local_contextual_alias_declaration_barriers` | Repaired regression: clean call beside a sibling recovery node is retained. |
| 5 | `local_props_alias_receiver_barriers` | Repaired regression: clean call beside a sibling recovery node is retained. |
| 6 | `local_generic_alias_argument_and_receiver_barriers` | Repaired regression: clean call beside a sibling recovery node is retained. |
| 7 | `private_props_interface_receiver_barriers` | Repaired regression: clean call beside a sibling recovery node is retained. |
| 8 | `receiver_self_named_nested_callback` | Intended ownership change: nearest anonymous nested callable is unindexed; exact site count is zero in full/subset for JS/TS/TSX while the raw receiver-binding assertion remains. |
| 9 | `receiver_self_nested_callback` | Intended ownership change: nearest anonymous nested callable is unindexed; exact site count is zero in full/subset for JS/TS/TSX while the raw receiver-binding assertion remains. |
| 10 | `receiver_self_nested_declaration` | Intended duplicate removal: exact site count changes 2 to 1; the independently indexed nearest owner remains and still refuses exact receiver resolution. |
| 11 | `receiver_self_outer_parameter` | Intended duplicate removal: exact site count changes 2 to 1; the indexed nested owner remains and still refuses exact receiver resolution. |
| 12 | `receiver_self_typed_named_shadow` | Intended duplicate removal: exact site count changes 2 to 1; retained site remains unresolved as required. |
| 13 | `receiver_self_typed_outer_parameter` | Intended duplicate removal: exact site count changes 2 to 1; retained site still resolves exactly to `item`. |
| 14 | `test_jsx_regular_calls_still_work` | Intended ownership change: `fetchUser` inside an unindexed effect callback is absent; direct `fetch` under indexed `fetchUser` remains. |
| 15 | `test_typescript_receiver_binding_parse_recovery_is_scope_bounded` | Repaired regression: exact clean call nodes outside the sibling error subtree are retained; receiver-resolution assertions pass unchanged. |
| 16 | `test_typescript_recovery_parse_uncertainty_fails_closed` | Repaired regression: call inventory is retained while receiver recovery still fails closed. |

Post-repair focused totals: inline receiver 26/0, receiver-self 36/0, TSX 1/0, TypeScript recovery 2/0. No receiver-resolution production code changed.

## Mandatory acceptance coverage

- C01/C03/C10: fixed complete CallSite metadata and Call/Return/DataFlow endpoint tuples for JS/TS/TSX; full, skeleton, subset and bincode paths; eager/direct/inner calls; exact resolution and multiplicity. Existing caller-only epoch/incremental checks remain. Parallel/serial control passed 1/0.
- C05/C06: computed-key callback ancestry and exact eight-route vectors across three dialects; nonzero recovery parse count, no borrowed anonymous ownership, unrelated clean owner retained.
- C07: Unicode precedes distinct same-name/same-line owners; raw byte tuples remain distinct and full/skeleton/subset graph identities refuse narrowly.
- C08/C09: complete root rows, Python nested preservation, Rust macro controls 46/0, contained-rvalue controls 3/0, capture/parameter/same-line refusal 1/0.
- C11: exact navigation symbols, bytes, call-site lines, reasons and multiplicity for inner/direct; outer is exactly empty. Final candidate 1/0; unchanged base 0/1.
- Focused final candidate: 8 passed / 0 failed. Same test patch on unchanged base: 2 passed / 6 failed; class and Python preservation are the two passes.

## Genuine base-cache provenance

Base archive: `f5350044a18bf95f3deb51a41b4b09e9d585b134`. Writer source SHA-256 `375b5a5a7b89b60b758945af89f5ae90547883939f9c75acd941c6ec4941ac8f`; writer binary SHA-256 `46a93fe5fff46e2dba566fcff5b1df6784eaf3d24e2675c94a018fcb9f18eed6`.

Pristine cache root: `/private/tmp/prism-post316-orchestration/genuine-old-cache-pristine`. It contains CPG v93 and navigation v52. Hashes remained byte-identical across a second unchanged-base load (`evidence/genuine-old-cache-base-{pre,post}.sha256`). Candidate loading a fresh copy logged `cached: 93, current: 94`, rebuilt CPG and navigation to 94/53, and produced cached caller rows `direct@3, inner@2` equal to uncached candidate with `outer` absent.

The genuine run also changes build identity, so it proves compatibility refusal and endpoint parity. The existing forced-version tests isolate the 93/94 and 52/53 guards.

## Current gate state

Round-one source and focused tests are complete. Full default/MCP/audit/examples, clippy, Node/authority, Python and Tier-A gates remain to run against the controller checkpoint. The historical pre-repair default run was 4,498 passed / 16 failed / 1 ignored and is superseded for candidate acceptance by the repaired source; its failures remain preserved and mapped above.

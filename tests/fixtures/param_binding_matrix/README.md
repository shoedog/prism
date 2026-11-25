# JavaScript parameter-binding matrix

`rows.jsonl` contains one case per line. A suffix splits a plan row into
independent cases. Group C uses the plan's `k` IDs and group D uses its `h`
IDs. Shared JS-family fixtures run in each declared language.

## Row schema

| Field | Meaning |
|---|---|
| `id` | Unique plan ID or plan ID with a sub-case suffix |
| `group` | `control`, `A`, `B`, `C`, `D` or `E` |
| `langs` | Nonempty list of `js`, `ts`, `tsx`, `go` or `py` |
| `files` | Fixture filename to source map. The build helper uses the filename stem and the extension for the current language. Multi-file fixtures with explicit extensions in imports use a single language. |
| `expect` | Nonempty list of assertions |
| `class` | Census metric and shape, or baseline control class |
| `truth` | `correct`, `wrong` or `ambiguous`. `wrong` means the row pins the existing unimported-export resolver defect, including rows that require later binding admission to refuse that resolution. |
| `issue` | Required for `wrong` rows |
| `unexecuted` | Reason the assertion needs a measurement or design decision beyond the fixture APIs. The test prints the ID and skips that row. |

An endpoint is `file:function:path@line:col`. Lines and columns are 1-based.
The column counts characters. The test converts it to a UTF-8 byte offset and
compares the filename, owner, access path, line and both byte offsets. A string
endpoint uses the literal path spelling to determine the expected end. For a
different source spelling, use `{"at":"file:function:path@line:col","text":"…"}`.
Use `"text":""` for a zero-width occurrence. An anonymous owner is
`<anon@L:C>` with its source start line and byte column.

## Assertion kinds

| Kind | Fields and check |
|---|---|
| `def` | `at` identifies exactly one variable Def. |
| `edge` | `from` identifies a Use and `to` identifies a Def or Use. Optional `label` is `exact` or `nameonly/<doubt>`. Optional `multiplicity` checks the number of matching edges, including parallel edges. The default requires at least one. |
| `no_edge` | `from` and optional `to` prohibit the same Use-to-Def or Use-to-Use edge. Without `to`, it prohibits transfers to other functions. Optional `label` limits the prohibition to that label. |
| `no_edge_into` | `to` prohibits incoming Use-to-Def or Use-to-Use transfers from other functions. Optional `label` limits the prohibition to that label. |
| `resolves` | `call` identifies one extracted site. `targets` is the full set of `file:function@start_line` targets, with `confidence` and `route` applied to each target. |
| `drop` | `call` has no targets and the given `reason` from `resolve_call_site_full`. |
| `reaches` | `from` is a prior Def. The occurrence at `to` must appear in the `dfg_forward_reachable` walk. The labeled API exposes visited node IDs so equal file/line/path entries stay distinct. |
| `not_reaches` | Same endpoints and walk as `reaches`, with the target absent. |
| `trace_boundary` | `taint_trace` from the line of the Def at `root` must record the Use at `from` to the Def at `to` as a boundary for that root. The target must be absent from that root's frontier. |
| `counter` | `key` is a dot-separated key in `call_stats`, `return_flow` or `dfg_labels`. `value` is an unsigned integer. An absent key is zero. External census and oracle keys appear only in rows marked `unexecuted`. |

A `call` object has `site` in `file@line:col` form at the start of the call
expression and the extracted `callee` name. Optional `end` is the exclusive
`line:col` position, and optional `owner` is the extracted caller name.

The ambiguous rest cases `b03` and `b09a` assert only the settled offsets.
`e02a` asserts the anonymous resolution and leaves the bare-arrow transfer
for stage D. `h09a`–`h09c` retain the desired edge assertion while stage D
decides the admissible receiver forms and their graph nodes. `e09a` uses a
receiver parameter with an existing single-owner name-search route.

For a baseline run, enable `control` and one feature group in
`src/cpg/param_binding_matrix_tests.rs`. Run `cargo test --lib
param_binding_matrix -- --nocapture` to display each row's result. The test
reports each row before failing and prints individual language/assertion
failures. Restore `ENABLED` to `&["control"]` afterward.

## Census classes by row

| Row | Group | Census class |
|---|---|---|
| `c01` | control | `control/fixed-slot` |
| `c02` | control | `control/parenthesized-arrow` |
| `c03a` | control | `control/go-spread` |
| `c03b` | control | `control/python-splat` |
| `c04` | control | `control/literal-after-spread` |
| `c05` | control | `control/builtin-drop` |
| `c06` | control | `control/arguments` |
| `c07` | control | `control/unimported-export` |
| `c08` | control | `control/unimported-export` |
| `a01` | A | `M1/unknown-offset` |
| `a02a` | A | `M1/spread-source` |
| `a02b` | A | `M1/known-prefix` |
| `a03a` | A | `M1/unknown-offset` |
| `a03b` | A | `M1/unknown-offset` |
| `a03c` | A | `M1/unknown-offset` |
| `a03d` | A | `M1/unknown-offset` |
| `a04a` | A | `M1/constructor` |
| `a04b` | A | `M1/receiver` |
| `a04c` | A | `M1/optional` |
| `a05a` | A | `M1/chain-identity` |
| `a05b` | A | `M1/chain-identity` |
| `a05c` | A | `M1/chain-identity` |
| `a05d` | A | `M1/call-result` |
| `a05e` | A | `M1/constructor-chain` |
| `a06a` | A | `M1/literal-length` |
| `a06b` | A | `M1/literal-length` |
| `a06c` | A | `M1/literal-length` |
| `a06d` | A | `M1/literal-length` |
| `a06e` | A | `M1/unknown-offset` |
| `a06f` | A | `M1/unknown-offset` |
| `a06g` | A | `M1/literal-length` |
| `a06h` | A | `M1/overload` |
| `a07a` | A | `M1/two-rows-one-site` |
| `a07b` | A | `M1/two-rows-one-site` |
| `a08a` | A | `M1/excluded-call-apply` |
| `a08b` | A | `M1/excluded-call-apply` |
| `a09a` | A | `M1/refusal-reachability` |
| `a09b` | A | `M1/refusal-reachability` |
| `b01a` | B | `M2/rest` |
| `b01b` | B | `M2/arrow-rest` |
| `b01c` | B | `M2/method-rest` |
| `b01d` | B | `M2/rest` |
| `b02a` | B | `M2/rest` |
| `b02b` | B | `M2/rest` |
| `b02c` | B | `M2/rest` |
| `b03` | B | `M2/ambiguous-offset` |
| `b04` | B | `M2/ts-this` |
| `b05a` | B | `M2/rest-pattern` |
| `b05b` | B | `M2/rebound-rest` |
| `b06` | B | `M2/nameonly` |
| `b07` | B | `M2/rest-object-literal` |
| `b08` | B | `M2/refusal-reachability` |
| `b09a` | B | `M2/least-offset` |
| `b09b` | B | `M2/literal-length` |
| `b09c` | B | `M2/literal-length` |
| `b10a` | B | `M2/no-variable-source` |
| `b10b` | B | `M2/no-variable-source` |
| `b10c` | B | `M2/no-variable-source` |
| `b10d` | B | `M2/no-variable-source` |
| `k01a` | C | `M3/bare-arrow` |
| `k01b` | C | `M3/bare-arrow` |
| `k02` | C | `M3/inferred-name` |
| `k03` | C | `M3/inferred-name` |
| `k04` | C | `M3/inferred-name` |
| `k05a` | C | `M3/literal-leaf` |
| `k05b` | C | `M3/rebound-formal` |
| `k06a` | C | `M2/cross-file-free-single` |
| `k06b` | C | `M3/cross-file-free-single` |
| `k06c` | C | `M3/cross-file-free-single` |
| `k07a` | C | `M2+M3/imported` |
| `k07b` | C | `M2+M3/same-file` |
| `k08a` | C | `M3/return-flow` |
| `k08b` | C | `M3/return-flow` |
| `h01a` | D | `M4/array-method` |
| `h01b` | D | `M4/array-method` |
| `h01c` | D | `M4/array-method` |
| `h01d` | D | `M4/array-method` |
| `h01e` | D | `M4/array-method` |
| `h01f` | D | `M4/array-method` |
| `h01g` | D | `M4/array-method` |
| `h01h` | D | `M4/array-method` |
| `h02a` | D | `M4/reducer` |
| `h02b` | D | `M4/reducer` |
| `h03a` | D | `M4/promise` |
| `h03b` | D | `M4/promise` |
| `h04` | D | `M4/named-local` |
| `h05a` | D | `M4/named-imported` |
| `h05b` | D | `M4/shadowed` |
| `h05c` | D | `M4/local-let` |
| `h06a` | D | `M4/callback-role` |
| `h06b` | D | `M4/no-formal` |
| `h06c` | D | `M4/destructured-formal` |
| `h07a` | D | `M4/member-only-formal` |
| `h07b` | D | `M4/mixed-formal` |
| `h07c` | D | `M4/member-receiver` |
| `h07d` | D | `M4/optional-receiver` |
| `h07e` | D | `M4/this-receiver` |
| `h07f` | D | `M4/shared-line` |
| `h07g` | D | `M4/no-receiver-node` |
| `h08` | D | `M4/in-repo-receiver` |
| `h09a` | D | `M4/call-result-receiver` |
| `h09b` | D | `M4/call-result-receiver` |
| `h09c` | D | `M4/call-result-receiver` |
| `h10a` | D | `M4/excluded-method` |
| `h10b` | D | `M4/excluded-method` |
| `h10c` | D | `M4/excluded-method` |
| `h11` | D | `M4/array-method` |
| `h12a` | D | `M4/instance-override` |
| `h12b` | D | `M4/instance-override` |
| `h13` | D | `M4/same-line-exact-call` |
| `h14a` | D | `M4/refusal-reachability` |
| `h14b` | D | `M4/refusal-reachability` |
| `e01` | E | `M5/named-target` |
| `e02a` | E | `M5/anonymous-target` |
| `e02b` | E | `M5/anonymous-target` |
| `e03a` | E | `M5/mixed-targets` |
| `e03b` | E | `M5/mixed-targets` |
| `e04a` | E | `M5/payload-arity` |
| `e04b` | E | `M5/payload-arity` |
| `e05` | E | `M5/fanout` |
| `e06a` | E | `M5/written-formal` |
| `e06b` | E | `M5/nameonly-caller` |
| `e06c` | E | `M5/spread-caller` |
| `e06d` | E | `M5/excluded-call-apply` |
| `e07` | E | `M5/imported-holder` |
| `e08a` | E | `M5/forward-depth` |
| `e08b` | E | `M5/forward-depth` |
| `e09a` | E | `M5/existing-name-search` |
| `e09b` | E | `M5/default-formal` |
| `e10` | E | `M5/engine-boundary` |
| `e11` | E | `M5/call-result` |
| `e12` | E | `M5/cross-file-free-single` |
| `e13` | E | `M5/anonymous-caller` |
| `e14` | E | `M5/same-line-exact-call` |
| `e15` | E | `M5/destructured-slot` |
| `e16a` | E | `M5/refusal-reachability` |
| `e16b` | E | `M5/refusal-reachability` |
| `t1-1` | A | `M1/multiple-literal-spreads` |
| `t1-2` | A | `M1/literal-trailing-comma` |
| `t1-3` | A | `M1/string-spread-boundary` |
| `t1-4` | A | `M1/nested-known-spread-boundary` |
| `t2-1` | A | `M1/unknown-then-empty-spread` |
| `t2-2` | A | `M1/known-spread-past-arity` |
| `t2-3` | A | `M1/typed-tuple-spread-boundary` |
| `t3-1` | A | `M1/literal-spread-element-boundary` |
| `t4-1` | A | `M1/tagged-template-implicit-first-argument` |
| `t4-2` | A | `M1/satisfies-wrapped-literal-spread` |
| `t5-1` | A | `M1/overridden-array-iterator` |
| `s1-1` | A | `M1/literal-leaf-after-empty-spread` |
| `s1-2` | A | `M1/discarded-array-in-sequence-spread` |
| `s1-3` | A | `M1/optional-member-call-chain-identity` |
| `s1-4` | A | `M1/same-line-distinct-spread-offsets` |
| `s1-5` | A | `M1/spread-inside-array-valued-element` |
| `s2-1` | A | `M1/constructor-and-method-share-callee-spelling` |
| `s2-2` | A | `M1/known-spread-preserves-base-and-projection` |
| `s2-3` | A | `M1/surplus-after-spread-cannot-certify-return-flow` |
| `s3-1` | A | `M1/type-assertion-cannot-shorten-runtime-array` |
| `s3-2` | A | `M1/equal-call-spans-and-callee-names-in-different-files` |
| `s4-1` | A | `M1/nested-call-spread-does-not-change-the-enclosing-argument-offset` |
| `s4-2` | A | `M1/spread-repositioning-leaves-an-independent-refusal-path` |
| `s5-1` | A | `M1/same-receiver-chain-binds-two-offsets-to-one-method` |
| `s5-2` | A | `M1/equal-length-conditional-spread-literal-boundary` |
| `s5-3` | A | `M1/object-leaf-after-spread-intervening-positional-argument` |
| `s5-4` | A | `M1/user-defined-call-method-keeps-runtime-spread-offsets` |
| `x1-1` | A | `M1/comment-only-array-versus-an-elision` |
| `x1-2` | A | `M1/noniterable-spread-prevents-delivery-of-the-evaluated-prefix` |
| `x1-3` | A | `M1/constructor-expression-call-with-no-new-argument-list` |
| `x1-4` | A | `M1/spread-moves-a-fixed-slot-at-imported-and-global-calls` |
| `x2-1` | A | `M1/sequence-expression-contributes-only-its-final-value` |
| `x2-2` | A | `M1/super-lookup-and-virtual-dispatch-on-the-same-receiver` |
| `x3-1` | A | `M1/optional-chain-short-circuit-skips-a-nested-spread-call` |
| `x3-2` | A | `M1/repositioned-member-argument-keeps-name-only-confidence` |
| `x3-3` | A | `M1/same-line-shifted-calls-certify-return-flow-independently` |
| `x4-1` | A | `M1/callee-is-read-before-a-spread-element-reassigns-it` |
| `x4-2` | A | `M1/a-bound-use-cannot-certify-a-surplus-use-of-the-same-variable` |
| `x5-1` | A | `M1/same-name-chain-with-equal-ending-columns-on-different-lines` |
| `x5-2` | A | `M1/constructor-return-object-selects-the-outer-method` |
| `t1-5` | B | `M2/default-before-rest` |
| `t2-4` | B | `M2/generator-rest` |
| `t3-2` | B | `M2/unknown-spread-two-fixed-slots` |
| `t4-3` | B | `M2/constructor-rest` |
| `s1-6` | B | `M2/known-spread-raises-unknown-offset-floor` |
| `s1-7` | B | `M2/destructured-fixed-formal-consumes-one-slot` |
| `s1-8` | B | `M2/return-flow-certification-with-rest-inputs` |
| `s2-4` | B | `M2/parameter-property-and-tuple-rest` |
| `s2-5` | B | `M2/shadowed-rest-write-keeps-entry-binding` |
| `s2-6` | B | `M2/member-spread-begins-at-rest-after-literal-prefix` |
| `s3-3` | B | `M2/super-forwarding-cannot-bind-back-to-derived-constructor` |
| `s3-4` | B | `M2/independent-unknown-spreads-start-in-the-rest-range` |
| `s3-5` | B | `M2/distinct-uses-of-the-same-value-feed-one-rest-def` |
| `s3-6` | B | `M2/indexed-mutation-preserves-the-rest-binding` |
| `s4-3` | B | `M2/fixed-overload-signatures-with-a-rest-implementation` |
| `s4-4` | B | `M2/spread-straddling-the-fixed-slot-versus-one-wholly-in-rest` |
| `s4-5` | B | `M2/object-rest-pattern-index-and-synthesized-length` |
| `s5-5` | B | `M2/least-offset-rest-edge-retains-nameonly-trace-boundary` |
| `s5-6` | B | `M2/implicit-derived-constructor-forwards-into-base-rest` |
| `s5-7` | B | `M2/dense-literal-spread-element-enters-rest-after-fixed-slot` |
| `s5-8` | B | `M2/writing-unmapped-arguments-preserves-entry-rest-binding` |
| `x1-5` | B | `M2/one-dense-spread-straddles-the-fixed-and-rest-slots` |
| `x1-6` | B | `M2/rest-length-is-independent-of-element-value` |
| `x1-7` | B | `M2/adjacent-tag-substitutions-both-enter-rest` |
| `x2-3` | B | `M2/lexical-rest-parameter-named-arguments` |
| `x2-4` | B | `M2/default-reads-the-rest-binding-before-initialization` |
| `x2-5` | B | `M2/guaranteed-elements-inside-an-unknown-length-array-spread` |
| `x3-4` | B | `M2/fixed-slot-after-first-default-precedes-rest-despite-function-length-zero` |
| `x3-5` | B | `M2/default-derived-constructor-forwards-without-iterating-a-rest-array` |
| `x4-3` | B | `M2/shifted-fixed-argument-feeds-a-default-while-a-rest-argument-bypasses-it` |
| `x4-4` | B | `M2/twice-bound-prefixes-fill-fixed-and-rest-slots-in-binding-order` |
| `x5-3` | B | `M2/reflect-construct-new-target-supplies-only-the-prototype` |
| `x5-4` | B | `M2/rest-collection-and-fixed-slot-forwarding-certify-separately` |
| `t4-4` | C | `M3/bare-arrow-jsx-body` |
| `s1-9` | C | `M3/same-file-lexical-shadowing` |
| `s1-10` | C | `M3/renamed-reexport-with-local-import-alias` |
| `s1-11` | C | `M3/member-assignment-result-is-a-lexical-binding` |
| `s2-7` | C | `M3/same-file-property-name-is-not-a-free-binding` |
| `s2-8` | C | `M3/default-export-of-a-lexical-arrow` |
| `s3-7` | C | `M3/imported-default-arrow-with-no-lexical-name` |
| `s3-8` | C | `M3/namespace-import-proves-cross-file-member-binding` |
| `s3-9` | C | `M3/classic-script-global-at-the-cross-file-admission-boundary` |
| `s4-6` | C | `M3/async-as-a-bare-formal-is-not-an-async-modifier` |
| `s4-7` | C | `M3/commonjs-export-shorthand-and-module-scope-require-binding` |
| `s4-8` | C | `M3/type-only-import-cannot-admit-a-runtime-arrow-target` |
| `s5-9` | C | `M3/static-and-instance-arrow-fields-select-different-owners` |
| `s5-10` | C | `M3/conflicting-star-reexports-do-not-admit-either-arrow` |
| `s5-11` | C | `M3/escaped-free-call-identifier-denotes-the-lexical-arrow` |
| `s5-12` | C | `M3/enclosing-call-result-does-not-prove-inferred-arrow-identity` |
| `x1-8` | C | `M3/default-export-expression-and-live-export-diverge-after-reassignment` |
| `x1-9` | C | `M3/calling-a-getter-result-does-not-pass-the-argument-to-the-setter` |
| `x2-6` | C | `M3/immediately-invoked-bare-arrows-without-inferred-names` |
| `x2-7` | C | `M3/refusal-distinguishes-two-exact-calls-to-one-export` |
| `x3-6` | C | `M3/object-spread-overwrites-one-leaf-before-a-bare-formal-read` |
| `x3-7` | C | `M3/reopened-typescript-namespace-does-not-share-private-lexical-arrows` |
| `x4-5` | C | `M3/two-star-reexports-of-one-binding-admit-one-arrow-edge` |
| `x4-6` | C | `M3/a-local-type-alias-leaves-an-imported-arrow-in-value-scope` |
| `x5-5` | C | `M3/commonjs-exports-alias-after-replacing-module-exports` |
| `x5-6` | C | `M3/module-var-and-global-object-property-hold-different-arrows` |
| `t1-11` | D | `M4/set-duplicate-value-role` |
| `t1-12` | D | `M4/map-key-role` |
| `t2-5` | D | `M4/bound-callback-prefix-boundary` |
| `t2-6` | D | `M4/array-and-thisarg-roles` |
| `t3-3` | D | `M4/reducer-without-initializer` |
| `t3-4` | D | `M4/typed-array-receiver` |
| `t4-5` | D | `M4/thisarg-function-is-not-callback` |
| `t5-2` | D | `M4/callback-after-empty-spread` |
| `t5-3` | D | `M4/callback-after-unknown-spread` |
| `s1-12` | D | `M4/parenthesized-inline-callback-identity` |
| `s1-13` | D | `M4/member-callback-does-not-select-a-lexical-decoy` |
| `s1-14` | D | `M4/typescript-this-is-not-the-element-slot` |
| `s1-15` | D | `M4/find-last-is-outside-the-method-table` |
| `s1-16` | D | `M4/dynamic-property-identifier-is-not-a-method-name` |
| `s1-17` | D | `M4/reducer-element-slot-inside-rest` |
| `s2-9` | D | `M4/static-bracket-method-with-optional-invocation` |
| `s2-10` | D | `M4/private-method-is-not-a-table-method` |
| `s2-11` | D | `M4/same-name-call-chain-does-not-reuse-inner-receiver` |
| `s2-12` | D | `M4/named-function-expression-uses-inline-identity` |
| `s2-13` | D | `M4/destructured-accumulator-consumes-slot-zero` |
| `s2-14` | D | `M4/hole-visiting-and-hole-skipping-methods` |
| `s2-15` | D | `M4/thenable-invokes-rejection-slot-with-separate-source` |
| `s3-10` | D | `M4/generator-callback-owner-uses-byte-column-after-unicode` |
| `s3-11` | D | `M4/generic-builtin-invocation-through-call-is-excluded` |
| `s3-12` | D | `M4/rejection-only-then-does-not-select-first-callable` |
| `s3-13` | D | `M4/temporal-dead-zone-shadows-an-outer-callback` |
| `s3-14` | D | `M4+M5/two-doubt-kinds-on-one-line-preserve-both-boundaries` |
| `s4-9` | D | `M4/one-named-callback-has-different-element-slots-at-same-line-sites` |
| `s4-10` | D | `M4/escaped-property-identifier-denotes-the-table-method` |
| `s4-11` | D | `M4/discarded-sequence-operand-is-not-the-receiver` |
| `s4-12` | D | `M4+M5/imported-alias-is-excluded-from-the-table-but-admitted-through-a-holder` |
| `s4-13` | D | `M4+M5/user-method-table-edge-and-actual-in-body-callback-payload` |
| `s5-13` | D | `M4/borrowed-array-map-on-an-array-like-receiver` |
| `s5-14` | D | `M4/array-from-mapping-callback-is-outside-the-method-table` |
| `s5-15` | D | `M4/reassigned-function-declaration-is-not-the-current-callback` |
| `s5-16` | D | `M4/inline-callback-inside-rather-than-after-a-dense-spread` |
| `s5-17` | D | `M4/computed-receiver-index-is-not-the-element-source` |
| `s5-18` | D | `M4/erased-type-assertion-preserves-inline-callback-identity` |
| `x1-10` | D | `M4/parentheses-preserve-the-method-receiver-while-comma-detaches-it` |
| `x1-11` | D | `M4/one-function-in-both-promise-handler-slots` |
| `x1-12` | D | `M4/singleton-reducer-distinguishes-omission-from-an-undefined-initializer` |
| `x1-13` | D | `M4/constructing-a-method-is-not-a-builtin-callback-invocation` |
| `x1-14` | D | `M4/global-callback-name-matches-an-unimported-module-export` |
| `x2-8` | D | `M4/tag-substitution-is-not-the-first-map-argument` |
| `x2-9` | D | `M4/nested-array-iteration-reaches-through-two-name-only-edges` |
| `x2-10` | D | `M4/flatmap-returns-the-inner-function-without-invoking-it` |
| `x3-8` | D | `M4/same-line-roots-enter-one-formal-with-different-confidence` |
| `x3-9` | D | `M4/table-name-and-installed-reducer-disagree-on-the-element-slot` |
| `x3-10` | D | `M4/nested-then-returns-a-promise-in-the-outer-handler-slot` |
| `x4-7` | D | `M4/conditional-receiver-selects-either-array-but-never-the-condition` |
| `x4-8` | D | `M4/throwing-array-species-prevents-map-callbacks-but-allows-forEach` |
| `x5-7` | D | `M4/iterator-map-defers-the-callback-until-consumption` |
| `x5-8` | D | `M4/callback-factory-input-and-returned-callback-have-different-roles` |
| `t1-6` | E | `M5/explicit-undefined-default` |
| `t1-7` | E | `M5/supplied-callback-overrides-default` |
| `t1-8` | E | `M5/destructured-alias` |
| `t1-9` | E | `M5/logical-assignment-write` |
| `t1-10` | E | `M5/destructuring-write` |
| `t2-7` | E | `M5/captured-formal-write` |
| `t2-8` | E | `M5/loop-formal-write` |
| `t2-9` | E | `M5/sloppy-mapped-arguments-write` |
| `t2-10` | E | `M5/null-does-not-activate-default` |
| `t2-11` | E | `M5/shadowed-holder-formal` |
| `t2-12` | E | `M5/callback-payload-known-spread` |
| `t3-5` | E | `M5/destructured-property-default` |
| `t3-6` | E | `M5/destructured-property-overrides-default` |
| `t3-7` | E | `M5/direct-eval-formal-write` |
| `t3-8` | E | `M5/strict-unmapped-arguments-write` |
| `t3-9` | E | `M5/ts-this-does-not-consume-callback-slot` |
| `t3-10` | E | `M5/callback-payload-unknown-spread` |
| `t4-6` | E | `M5/sloppy-nonsimple-parameters-unmapped-arguments` |
| `t4-7` | E | `M5/default-and-explicit-callers-union` |
| `t4-8` | E | `M5/array-destructured-callback-slots` |
| `t4-9` | E | `M5/callback-target-rest-formal` |
| `t4-10` | E | `M5/recursive-holder-forwarding-cycle` |
| `t5-4` | E | `M5/optional-callback-invocation` |
| `t5-5` | E | `M5/duplicate-object-key-last-wins` |
| `s1-18` | E | `M5/holder-callback-after-known-spread` |
| `s1-19` | E | `M5/shadowed-undefined-is-a-supplied-callback` |
| `s1-20` | E | `M5/parameter-default-captures-earlier-formal` |
| `s1-21` | E | `M5/sloppy-duplicate-formal-uses-last-slot` |
| `s1-22` | E | `M5/nested-destructuring-keeps-full-key-path` |
| `s1-23` | E | `M5/fanout-counts-distinct-targets-at-the-cap` |
| `s1-24` | E | `M5/same-line-anonymous-callback-identities` |
| `s1-25` | E | `M5/constructor-is-the-callback-holder` |
| `s1-26` | E | `M5/forwarding-cycle-with-no-grounded-callback` |
| `s1-27` | E | `M5/jsx-explicit-callback-overrides-earlier-prop-spread` |
| `s1-28` | E | `M5/nameonly-callback-result-does-not-certify-return-flow` |
| `s1-29` | E | `M5/escaping-holder-retains-known-caller` |
| `s1-30` | E | `M5/imported-callback-alias-with-local-name-decoy` |
| `s1-31` | E | `M5/write-to-shadow-binding-keeps-holder-callback` |
| `s2-16` | E | `M5/var-redeclaration-without-initializer-preserves-formal` |
| `s2-17` | E | `M5/hoisted-body-function-replaces-formal-before-call` |
| `s2-18` | E | `M5/later-parameter-is-uninitialized-during-default` |
| `s2-19` | E | `M5/deleted-mapped-argument-does-not-rebind-formal` |
| `s2-20` | E | `M5/void-expression-activates-default-without-passing-operand` |
| `s2-21` | E | `M5/with-object-intercepts-formal-name` |
| `s2-22` | E | `M5/non-null-callee-with-erased-type-arguments` |
| `s2-23` | E | `M5/unqualified-cross-file-forwarding-hop-is-not-admitted` |
| `s2-24` | E | `M5/late-object-spread-overwrites-explicit-callback` |
| `s2-25` | E | `M5/inherited-callback-property-does-not-trigger-default` |
| `s2-26` | E | `M5/computed-numeric-key-matches-string-key` |
| `s2-27` | E | `M5/default-initializer-uses-parameter-before-body-function` |
| `s2-28` | E | `M5/arrow-callback-is-callable-but-not-constructable` |
| `s2-29` | E | `M5/inner-holder-uses-its-own-callback-in-same-name-chain` |
| `s3-15` | E | `M5/local-callee-alias-is-outside-formal-following` |
| `s3-16` | E | `M5/define-property-updates-mapped-argument-before-detaching-it` |
| `s3-17` | E | `M5/optional-eval-is-indirect-and-cannot-rebind-the-formal` |
| `s3-18` | E | `M5/anonymous-default-initializer-is-a-callback-target` |
| `s3-19` | E | `M5/array-pattern-elision-and-sparse-actual-activate-slot-default` |
| `s3-20` | E | `M5/later-accessor-replaces-an-own-callback-value` |
| `s3-21` | E | `M5/distinct-symbols-with-the-same-description-are-not-the-same-key` |
| `s3-22` | E | `M5/class-callback-is-constructable-but-not-callable` |
| `s3-23` | E | `M5/spread-payload-binds-the-callback-rest-with-derived-confidence` |
| `s4-14` | E | `M5/conditional-actual-selects-an-explicit-callback-or-its-default` |
| `s4-15` | E | `M5/truthy-left-and-operand-is-not-the-callback-value` |
| `s4-16` | E | `M5/derived-callee-edge-cannot-feed-another-holder` |
| `s4-17` | E | `M5/a-later-write-refuses-the-formal-at-earlier-and-later-calls` |
| `s4-18` | E | `M5/sloppy-block-function-sharing-a-parameter-name-stays-in-the-block` |
| `s4-19` | E | `M5/nested-arrow-and-ordinary-function-use-different-arguments-bindings` |
| `s4-20` | E | `M5/local-destructuring-of-a-parameter-selects-the-property-not-the-local-spelling` |
| `s4-21` | E | `M5/destructured-callback-used-as-a-jsx-component` |
| `s4-22` | E | `M5/jsx-boolean-shorthand-does-not-trigger-a-prop-default` |
| `s4-23` | E | `M5/rest-array-index-invocation-is-outside-formal-following` |
| `s4-24` | E | `M5/default-copies-earlier-callback-before-the-body-reassigns-it` |
| `s5-19` | E | `M5/later-default-writes-the-callback-before-the-body-runs` |
| `s5-20` | E | `M5/array-pattern-consumes-iterator-values-not-numeric-properties` |
| `s5-21` | E | `M5/distinct-overload-callers-feed-one-runtime-holder` |
| `s5-22` | E | `M5/bound-callback-payload-cannot-enter-the-prebound-prefix-slot` |
| `s5-23` | E | `M5/bare-arrow-formal-is-a-callback-holder` |
| `s5-24` | E | `M5/same-line-callback-calls-in-different-holders-keep-lexical-slots` |
| `s5-25` | E | `M5/only-rightmost-duplicate-formal-has-a-mapped-arguments-entry` |
| `s5-26` | E | `M5/lexically-shadowed-eval-cannot-replace-the-callback` |
| `s5-27` | E | `M5/setter-only-property-reads-as-undefined-and-selects-default` |
| `s5-28` | E | `M5/callback-object-literal-leaf-carries-derived-confidence` |
| `x1-15` | E | `M5/known-callback-precedes-an-unknown-spread-at-the-holder-call` |
| `x1-16` | E | `M5/recursive-forwarding-swaps-the-two-callback-slots` |
| `x1-17` | E | `M5/no-holder-call-versus-a-holder-call-with-an-omitted-argument` |
| `x1-18` | E | `M5/outer-object-default-supplies-a-property-before-its-own-default` |
| `x1-19` | E | `M5/later-argument-assignment-does-not-replace-an-already-read-callback` |
| `x1-20` | E | `M5/destructured-concise-method-versus-same-named-free-declaration` |
| `x1-21` | E | `M5/rejected-foreign-callers-do-not-consume-the-target-fanout-budget` |
| `x1-22` | E | `M5/one-jsx-expression-child-is-callable-while-siblings-form-an-array` |
| `x1-23` | E | `M5/a-helper-mutates-mapped-arguments-or-a-spread-copy` |
| `x1-24` | E | `M5/same-name-targets-at-the-same-line-in-distinct-caller-modules` |
| `x1-25` | E | `M5/setter-assignment-as-the-only-invocation-of-a-callback-holder` |
| `x2-11` | E | `M4+M5/reducer-initializer-and-direct-caller-supply-different-functions` |
| `x2-12` | E | `M5/callback-parameter-used-as-a-template-tag` |
| `x2-13` | E | `M5/computed-key-and-callback-object-must-come-from-the-same-caller` |
| `x2-14` | E | `M5/object-rest-forwarding-removes-the-key-that-would-suppress-a-default` |
| `x2-15` | E | `M5/default-created-closure-retains-the-parameter-environment` |
| `x2-16` | E | `M5/shadowed-arguments-aliases-the-outer-holders-mapped-arguments` |
| `x2-17` | E | `M5/fanout-drops-one-formal-and-preserves-the-other-on-the-same-line` |
| `x2-18` | E | `M5/global-callback-actual-and-import-alias-at-admitted-holder-calls` |
| `x2-19` | E | `M5/type-query-in-satisfies-is-not-a-second-callback-value` |
| `x2-20` | E | `M5/an-iterator-return-value-does-not-fill-a-destructured-callback-slot` |
| `x2-21` | E | `M5/jsx-element-actual-and-component-function-identity` |
| `x2-22` | E | `M5/forward-depth-is-applied-per-caller-path` |
| `x2-23` | E | `M5/module-namespace-object-and-its-callable-default-export` |
| `x3-11` | E | `M5/one-derived-target-has-fixed-slots-and-another-collects-both-payloads` |
| `x3-12` | E | `M5/unknown-spread-caller-cannot-add-a-second-callback-at-an-unproved-slot` |
| `x3-13` | E | `M5/computed-method-name-uses-the-enclosing-callback-before-method-formals-exist` |
| `x3-14` | E | `M5/destructuring-reads-the-property-after-all-caller-arguments-are-evaluated` |
| `x3-15` | E | `M5/callable-proxy-apply-trap-prevents-delegation-to-its-target` |
| `x3-16` | E | `M5/forwarded-value-is-the-relays-default-before-the-holder-applies-its-own` |
| `x3-17` | E | `M3+M5/cross-file-argument-follow-edge-does-not-itself-seed-refusal` |
| `x3-18` | E | `M5/jsx-intrinsic-tag-does-not-call-a-same-spelled-component-alias` |
| `x3-19` | E | `M5/escaped-closure-uses-the-creators-callback-and-the-later-callers-payload` |
| `x4-9` | E | `M4+M5/a-table-fed-callable-formal-also-has-an-exact-direct-caller` |
| `x4-10` | E | `M5/two-bindings-of-one-accessor-read-two-different-callbacks` |
| `x4-11` | E | `M5/an-empty-object-pattern-can-prevent-entry-to-a-callback-holder` |
| `x4-12` | E | `M5/a-yield-expression-passes-the-resume-value-to-the-holder` |
| `x4-13` | E | `M5/replacing-a-callbacks-call-property-preserves-its-direct-call-target` |
| `x4-14` | E | `M5/a-tagged-holder-receives-its-callback-after-the-template-object` |
| `x4-15` | E | `M5/destructuring-reads-nonenumerable-callbacks-that-object-spread-omits` |
| `x4-16` | E | `M5/omitted-and-explicit-undefined-create-different-mapped-arguments-entries` |
| `x4-17` | E | `M3+M5/one-unsafe-target-in-a-derived-set-refuses-a-bare-formal-source` |
| `x4-18` | E | `M5/jsx-factory-removes-a-callback-prop-before-calling-the-component` |
| `x4-19` | E | `M5/one-callback-target-receives-the-payload-at-two-different-offsets` |
| `x5-9` | E | `M5/partial-destructuring-closes-a-generator-before-the-second-formal` |
| `x5-10` | E | `M5/var-in-a-simple-catch-initializes-the-catch-binding` |
| `x5-11` | E | `M5/awaited-function-and-its-promise-are-different-actuals` |
| `x5-12` | E | `M5/reflect-set-on-mapped-arguments-respects-the-receiver` |
| `x5-13` | E | `M5/holder-passed-to-itself-rebinds-two-of-its-own-formals` |
| `x5-14` | E | `M5/method-decorator-replaces-the-function-passed-to-a-holder` |
| `x5-15` | E | `M5/callback-identity-test-narrows-two-derived-sites-in-one-holder` |
| `x5-16` | E | `M5/receiver-dependent-default-is-skipped-by-an-explicit-callback` |
| `x5-17` | E | `M5/jsx-child-overrides-spread-children-only-on-the-paired-tag` |
| `x5-18` | E | `M3+M5/fanout-drop-removes-the-only-bridge-to-a-refusal-seed` |

## Added row IDs by group

### Group A

`t1-1`, `t1-2`, `t1-3`, `t1-4`, `t2-1`, `t2-2`, `t2-3`, `t3-1`, `t4-1`, `t4-2`, `t5-1`,
`s1-1`, `s1-2`, `s1-3`, `s1-4`, `s1-5`, `s2-1`, `s2-2`, `s2-3`, `s3-1`, `s3-2`, `s4-1`,
`s4-2`, `s5-1`, `s5-2`, `s5-3`, `s5-4`, `x1-1`, `x1-2`, `x1-3`, `x1-4`, `x2-1`, `x2-2`,
`x3-1`, `x3-2`, `x3-3`, `x4-1`, `x4-2`, `x5-1`, `x5-2`.

### Group B

`t1-5`, `t2-4`, `t3-2`, `t4-3`, `s1-6`, `s1-7`, `s1-8`, `s2-4`, `s2-5`, `s2-6`, `s3-3`,
`s3-4`, `s3-5`, `s3-6`, `s4-3`, `s4-4`, `s4-5`, `s5-5`, `s5-6`, `s5-7`, `s5-8`, `x1-5`,
`x1-6`, `x1-7`, `x2-3`, `x2-4`, `x2-5`, `x3-4`, `x3-5`, `x4-3`, `x4-4`, `x5-3`, `x5-4`.

### Group C

`t4-4`, `s1-9`, `s1-10`, `s1-11`, `s2-7`, `s2-8`, `s3-7`, `s3-8`, `s3-9`, `s4-6`,
`s4-7`, `s4-8`, `s5-9`, `s5-10`, `s5-11`, `s5-12`, `x1-8`, `x1-9`, `x2-6`, `x2-7`,
`x3-6`, `x3-7`, `x4-5`, `x4-6`, `x5-5`, `x5-6`.

### Group D

`t1-11`, `t1-12`, `t2-5`, `t2-6`, `t3-3`, `t3-4`, `t4-5`, `t5-2`, `t5-3`, `s1-12`,
`s1-13`, `s1-14`, `s1-15`, `s1-16`, `s1-17`, `s2-9`, `s2-10`, `s2-11`, `s2-12`, `s2-13`,
`s2-14`, `s2-15`, `s3-10`, `s3-11`, `s3-12`, `s3-13`, `s3-14`, `s4-9`, `s4-10`, `s4-11`,
`s4-12`, `s4-13`, `s5-13`, `s5-14`, `s5-15`, `s5-16`, `s5-17`, `s5-18`, `x1-10`,
`x1-11`, `x1-12`, `x1-13`, `x1-14`, `x2-8`, `x2-9`, `x2-10`, `x3-8`, `x3-9`, `x3-10`,
`x4-7`, `x4-8`, `x5-7`, `x5-8`.

### Group E

`t1-6`, `t1-7`, `t1-8`, `t1-9`, `t1-10`, `t2-7`, `t2-8`, `t2-9`, `t2-10`, `t2-11`,
`t2-12`, `t3-5`, `t3-6`, `t3-7`, `t3-8`, `t3-9`, `t3-10`, `t4-6`, `t4-7`, `t4-8`,
`t4-9`, `t4-10`, `t5-4`, `t5-5`, `s1-18`, `s1-19`, `s1-20`, `s1-21`, `s1-22`, `s1-23`,
`s1-24`, `s1-25`, `s1-26`, `s1-27`, `s1-28`, `s1-29`, `s1-30`, `s1-31`, `s2-16`,
`s2-17`, `s2-18`, `s2-19`, `s2-20`, `s2-21`, `s2-22`, `s2-23`, `s2-24`, `s2-25`,
`s2-26`, `s2-27`, `s2-28`, `s2-29`, `s3-15`, `s3-16`, `s3-17`, `s3-18`, `s3-19`,
`s3-20`, `s3-21`, `s3-22`, `s3-23`, `s4-14`, `s4-15`, `s4-16`, `s4-17`, `s4-18`,
`s4-19`, `s4-20`, `s4-21`, `s4-22`, `s4-23`, `s4-24`, `s5-19`, `s5-20`, `s5-21`,
`s5-22`, `s5-23`, `s5-24`, `s5-25`, `s5-26`, `s5-27`, `s5-28`, `x1-15`, `x1-16`,
`x1-17`, `x1-18`, `x1-19`, `x1-20`, `x1-21`, `x1-22`, `x1-23`, `x1-24`, `x1-25`,
`x2-11`, `x2-12`, `x2-13`, `x2-14`, `x2-15`, `x2-16`, `x2-17`, `x2-18`, `x2-19`,
`x2-20`, `x2-21`, `x2-22`, `x2-23`, `x3-11`, `x3-12`, `x3-13`, `x3-14`, `x3-15`,
`x3-16`, `x3-17`, `x3-18`, `x3-19`, `x4-9`, `x4-10`, `x4-11`, `x4-12`, `x4-13`, `x4-14`,
`x4-15`, `x4-16`, `x4-17`, `x4-18`, `x4-19`, `x5-9`, `x5-10`, `x5-11`, `x5-12`, `x5-13`,
`x5-14`, `x5-15`, `x5-16`, `x5-17`, `x5-18`.

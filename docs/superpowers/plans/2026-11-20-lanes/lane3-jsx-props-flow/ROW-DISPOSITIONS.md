c-py-p01 | keep | 5.1.2
c-py-p05 | keep | 5.1.2
c-py-p19 | keep | 5.1.2
c-go-p01 | keep | 5.1.2
c-go-p05 | keep | 5.1.2
c-go-p19 | keep | 5.1.2
c-positional-stop | keep | 5.1.1
c-literal-read | keep | 5.3.2
c-literal-miss | keep | 5.3.2
c-jsx-named | keep | 5.3.4
c-jsx-alias | keep | 5.3.4
p01 | keep | 5.1.2
p02 | keep | 5.1.1
p03 | keep | 5.1.1
p04 | keep | 5.1.1
p05 | keep | 5.1.3
p06 | keep | 5.1.1
p07 | keep | 5.1.1
p08 | keep | 5.1.1
p09 | keep | 5.1.2
p10 | keep | 5.1.2
p11 | keep | 5.1.1
p12 | keep | 5.1.1
p13 | change: Keep the graph assertions. Set base to green: the earlier-formal alias already provides the pinned Defs (Stage T/E RED record). | 5.1.1
p14 | keep | 5.1.2
p15 | change: Keep the entry Def. Require the line-2 assignment Def → line-3 Use (exact) and no entry-Def → line-3 Use at any label. The straight-line identifier twin has no surviving entry edge. | 5.1.2
p16 | keep | 5.1.2
p17 | keep | 5.1.2
p18 | keep | 5.1.1
p19 | keep | 5.1.1
p20 | keep | 5.1.1
p21 | keep | 5.1.2
p22 | keep | 5.1.2
p23 | change: At the formal identifier bytes require entry → body read `cfg_incomplete` for js, ts and tsx. Distinguish the separate line-1 assignment Def by bytes. | 5.1.2
p24 | keep | 5.1.2
p25 | keep | 5.1.2
p26 | keep | 5.1.2
p27 | change: For js, ts and tsx require entry Def → line-3 Use `killed` and line-2 captured-write Def → the same Use `cfg_incomplete`. | 5.1.2
p28 | keep | 5.1.1
p29 | change: Keep the graph assertions. Set base to green: the outer-binding alias already provides the pinned Defs (Stage T/E RED record). | 5.1.1
p30 | change: Keep the graph assertions. Set base to green: the effectful whole-formal default already has no Def (Stage T/E RED record). | 5.1.1
p31 | change: Pin the earlier sibling's Def [15,16) → entire bare-default operand [24,25) at cfg_incomplete. Keep b absent. | 5.1.2
p32 | keep | 5.1.2
p33 | keep | 5.1.2
p34 | keep | 5.1.1
p35 | keep | 5.1.2
l01 | keep | 5.3.2
l02 | keep | 5.2.1
l03 | keep | 5.3.2
l04 | keep | 5.3.2
l05 | keep | 5.1.3
l06 | keep | 5.3.2
l07 | keep | 5.2.3
l08 | keep | 5.2.3
l09 | keep | 5.2.3
l10 | keep | 5.3.1
l11 | keep | 5.3.2
l12 | keep | 5.3.5
l13 | keep | 5.3.3
l14 | keep | 5.3.3
l15 | keep | 5.1.3
l16 | keep | 5.3.2
l17 | keep | 5.3.2
l18 | keep | 5.3.2
l19 | keep | 5.3.1
l20 | keep | 5.2.1
l21 | keep | 5.3.1
l22 | keep | 5.3.1
l23 | keep | 5.1.3
l24 | keep | 5.3.2
j01 | keep | 5.3.5
j02 | keep | 5.3.2
j03 | keep | 5.3.1
j04 | keep | 5.3.1
j05 | keep | 5.3.1
j06 | keep | 5.2.3
j07 | keep | 5.2.3
j08 | change: Keep the graph assertions. Set base to green: base emits no reserved-attribute edge (Stage T/E RED record). | 5.2.2
j09 | keep | 5.2.2
j10 | change: In member, bind once from the connected full user.name Use, with its path from user’s Def and trace. Assert no additional base user Use → a edge. Keep the three wrapper cases. | 5.3.4
j11 | keep | 5.2.2
j12 | keep | 5.3.2
j13 | keep | 5.2.2
j14 | change: Keep the graph assertions. Set base to green: base emits no namespaced-attribute edge (Stage T/E RED record). | 5.2.2
j15 | keep | 5.3.1
j16 | keep | 5.3.1
j17 | keep | 5.2.2
j18 | keep | 5.3.5
j19 | keep | 5.3.5
j20 | keep | 5.2.2
j21 | keep | 5.3.4
j22 | keep | 5.3.4
j23 | keep | 5.3.1
j24 | keep | 5.3.2
j25 | keep | 5.3.4
j26 | keep | 5.3.4
j27 | keep | 5.3.4
j28 | keep | 5.3.4
j29 | keep | 5.3.4
j30 | keep | 5.3.4
j31 | change: Keep One’s S1 title Def. No props edge into D:title, no path from One:title to D:title and no descent from that seed into D’s body. Count refused_source_not_found = 1. Do not require a caller Use for the negative. | 5.3.4
j32 | keep | 5.3.4
j33 | change: Both cases have no props edge into Row:onPick, no path from App’s onPick Def, and no trace descent. Expression counts refused_source_in_enclosing_owner = 1; block counts refused_source_unconnected = 1. Use the recorded distinct caller/enclosing nodes. | 5.3.4
j34 | keep | 5.3.1
j35 | keep | 5.3.5
j36 | change: In nested and default-export keep no edge and assert refused_not_exact_singleton = 1 and refused_unknown_wrapper = 0 in each. Keep the three resolved other-wrapper refusals and the React positive including its trace boundary. | 5.3.1
j37 | keep | 5.3.5
j38 | keep | 5.2.2
j39 | keep | 5.3.1
j40 | keep | 5.2.3
astra-1-01 | keep | 5.1.2
astra-1-02 | keep | 5.1.2
astra-1-03 | keep | 5.1.1
astra-1-04 | keep | 5.2.3
astra-1-05 | keep | 5.2.3
astra-1-06 | keep | 5.2.3
astra-1-07 | change: Keep graph edges, both paths, bytes, counter and negative control. Replace trace_reaches with SelfFunctionParam boundary NodeView:next@4(use) bytes [95,99) → NodeView:current@1(def) and trace_misses NodeView:current@2(use), both seeded at m:3. | 5.3.5
astra-1-08 | keep | 5.3.2
astra-1-09 | keep | 5.3.1
astra-1-10 | keep | 5.3.1
astra-1-11 | keep | 5.3.1
astra-1-12 | keep | 5.3.1
astra-1-13 | keep | 5.3.4
astra-1-14 | keep | 5.3.4
astra-1-15 | keep | 5.3.4
astra-1-16 | keep | 5.3.1
astra-1-17 | keep | 5.1.3
astra-1-18 | keep | 5.1.3
astra-1-19 | keep | 5.3.4
astra-1-20 | change: Keep all local-it bindings, their paths and local trace. Replace the positive shared path and shared trace with no edge into shared, no path from App:outside and no descent from that seed via the capture; refused_source_unconnected = 1. | 5.3.4
astra-1-21 | keep | 5.3.4
astra-1-22 | keep | 5.3.1
astra-1-23 | keep | 5.3.1
astra-1-24 | keep | 5.3.4
astra-1-25 | keep | 5.3.4
astra-1-26 | keep | 5.2.2
astra-2-01 | keep | 5.1.2
astra-2-02 | change: In captured, refuse the descendant formal's operand [61,65) once (refused_nested_param_list). Omit the sole-use loop Def [13,17). Keep the self-name negative and live's body edge in both cases. | 5.1.2
astra-2-03 | change: Pin seed [22,26) → bare sibling-default operand [44,48) at cfg_incomplete. Keep out absent and keep's body edge present. | 5.1.2
astra-2-04 | keep | 5.1.3
astra-2-05 | keep | 5.1.3
astra-2-06 | keep | 5.2.1
astra-2-07 | keep | 5.2.1
astra-2-08 | keep | 5.3.2
astra-2-09 | keep | 5.3.2
astra-2-10 | keep | 5.3.4
astra-2-11 | keep | 5.3.4
astra-2-12 | keep | 5.3.4
astra-2-13 | keep | 5.3.4
astra-2-14 | keep | 5.3.1
astra-2-15 | change: Keep star negative. In explicit also require no edge/path to Card, trace misses the body, and refused_not_exact_singleton = 1. Remove the trace boundary requiring a props edge: UI.default has no default-spelled callable candidate. | 5.3.1
astra-2-16 | keep | 5.3.1
astra-2-17 | keep | 5.2.2
astra-2-18 | keep | 5.3.4
astra-2-19 | keep | 5.2.2
astra-2-20 | keep | 5.3.2
astra-2-21 | keep | 5.2.3
astra-3-01 | keep | 5.1.2
astra-3-02 | change: Both cases refuse the nonfinal reader at [31,37) once (refused_param_list_shape). In capture omit the sole-use picked Def [43,49). In body keep that Def and its body edge to [66,72), with no entry edge to the reader. | 5.1.2
astra-3-03 | keep | 5.1.2
astra-3-04 | keep | 5.3.3
astra-3-05 | keep | 5.3.3
astra-3-06 | keep | 5.3.4
astra-3-07 | keep | 5.3.4
astra-3-08 | change: No edge into View or path/descent to its body, edges_to_def = 0 and refused_not_exact_singleton = 1. Both stars reach the same runtime binding in core, but the existing member rung has no View candidate in barrel. | 5.3.1
astra-3-09 | keep | 5.3.1
astra-3-10 | keep | 5.3.1
astra-3-11 | keep | 5.2.2
astra-3-12 | keep | 5.3.4
astra-3-13 | change: Direct only, keep edges and paths. Replace trace_reaches with SelfFunctionParam boundary Inner:next@4(use) bytes [70,74) → Inner:a@1(def) and trace_misses Inner:a@2(use), both seeded at m:3. Keep wrapped intact. | 5.3.5
astra-3-14 | keep | 5.3.2
astra-4-01 | keep | 5.1.2
astra-4-02 | keep | 5.1.2
astra-4-03 | keep | 5.2.1
astra-4-04 | keep | 5.3.2
astra-4-05 | keep | 5.3.1
astra-4-06 | keep | 5.3.1
astra-4-07 | change: Keep current’s binding, path, trace and the setter’s intra-owner edge. Replace the getter x → saved edge with no edge into Tile:saved and refused_source_unconnected = 1. The setter’s source stays ineligible. | 5.3.4
astra-4-08 | keep | 5.3.4
astra-4-09 | change: For ordinary and hosted keep the it → local path and App’s intra-owner edge. Replace paint:outside → shared with no edge into shared and refused_source_unconnected = 1 per case. Keep both connected method parameters and the isolation assertions. | 5.3.4
astra-4-10 | keep | 5.3.4
astra-4-11 | keep | 5.2.2
astra-4-12 | keep | 5.3.4
astra-5-01 | keep | 5.1.2
astra-5-02 | keep | 5.1.2
astra-5-03 | change: Both spreads precede the literal. In empty and one keep the pattern Def, assert no x edge or path into either pattern, trace misses both body uses, edges_to_def = 0 and refused_argument_after_spread = 1. Syntactic position is unsafe even when occupied. | 5.3.3
astra-5-04 | keep | 5.3.2
astra-5-05 | keep | 5.3.4
astra-5-06 | change: No incoming props edges into inherited or own and no path from either receiver Use to the corresponding body read. Count refused_source_unconnected = 2. Keep note → stamp, its path and trace. | 5.3.4
astra-5-07 | keep | 5.3.4
astra-5-08 | keep | 5.3.4
astra-5-09 | keep | 5.2.2
astra-5-10 | keep | 5.3.4
astra-5-11 | keep | 5.3.4
sol-1-01 | keep | 5.1.1
sol-1-02 | keep | 5.1.1
sol-1-03 | keep | 5.1.1
sol-1-04 | keep | 5.3.3
sol-1-05 | keep | 5.3.3
sol-1-06 | keep | 5.3.3
sol-1-07 | keep | 5.3.2
sol-1-08 | keep | 5.3.3
sol-1-09 | keep | 5.2.3
sol-1-10 | keep | 5.2.1
sol-1-11 | keep | 5.3.2
sol-1-12 | keep | 5.2.3
sol-1-13 | keep | 5.2.2
sol-1-14 | keep | 5.1.3
sol-1-15 | keep | 5.2.2
sol-1-16 | keep | 5.3.1
sol-1-17 | keep | 5.3.1
sol-1-18 | keep | 5.3.1
sol-1-19 | keep | 5.3.1
sol-1-20 | keep | 5.3.1
sol-1-21 | keep | 5.3.4
sol-1-22 | keep | 5.3.4
sol-1-23 | change: No props edge into D:a from the unconnected catch-body Use, no path from either the catch binding or the outer formal to D’s read; refused_source_unconnected = 1. | 5.3.4
sol-1-24 | keep | 5.2.2
sol-1-25 | keep | 5.2.2
sol-1-26 | keep | 5.3.1
sol-1-27 | keep | 5.2.2
sol-2-01 | change: Count refused_param_list_shape once for format(a) at [38,39). Omit the sole-use a Def [13,14) and its entry edge. Keep spare absent. The split bare default in d1-selected-endpoint is the positive control. | 5.1.2
sol-2-02 | keep | 5.1.2
sol-2-03 | keep | 5.1.1
sol-2-04 | keep | 5.2.1
sol-2-05 | keep | 5.3.3
sol-2-06 | keep | 5.2.1
sol-2-07 | keep | 5.3.4
sol-2-08 | keep | 5.3.4
sol-2-09 | keep | 5.3.4
sol-2-10 | keep | 5.3.4
sol-2-11 | keep | 5.3.4
sol-2-12 | keep | 5.3.4
sol-2-13 | keep | 5.3.4
sol-2-14 | change: Same-line: refuse the ambiguous leaf, no edge into D:a, no path from either x Def, refused_source_ambiguous = 1. Keep all split-line assertions. | 5.3.4
sol-2-15 | keep | 5.3.1
sol-2-16 | change: Both cases have no edge/path to Impl. In default require refused_not_exact_singleton = 1. In reexport the unrelated barrel View is also unbound; count refused_member_not_export = 1 on its spelling fallback. An export proof cannot add Impl to the resolver result. | 5.3.1
sol-2-17 | keep | 5.3.1
sol-2-18 | keep | 5.2.3
sol-2-19 | keep | 5.3.2
sol-3-01 | change: Pin the edge from key [15,18) to the whole computed-key operand [23,26) at cfg_incomplete. Keep value's path to its body read and spare absent. | 5.1.2
sol-3-02 | keep | 5.1.2
sol-3-03 | keep | 5.2.1
sol-3-04 | keep | 5.2.3
sol-3-05 | keep | 5.2.1
sol-3-06 | keep | 5.2.1
sol-3-07 | keep | 5.3.2
sol-3-08 | change: Same-line: no edge or path from either x version to D:a and refused_source_ambiguous = 1. Keep the connected replacement and the negative original-formal control in split-line. | 5.3.4
sol-3-09 | change: Keep both byte-specific local Def-to-Use controls. Neither same-name/same-range callback issues a JSX record (ambiguous_js_ts_call_owner_ids). Require no props edge or path into either D:a or E:a, sites_jsx = 0 and no source refusal. Never transfer the sites to App. | 5.3.4
sol-3-10 | keep | 5.3.4
sol-3-11 | keep | 5.3.4
sol-3-12 | keep | 5.3.4
sol-3-13 | keep | 5.3.4
sol-3-14 | keep | 5.3.4
sol-3-15 | keep | 5.3.1
sol-3-16 | keep | 5.3.1
sol-3-17 | keep | 5.3.2
sol-4-01 | keep | 5.1.1
sol-4-02 | keep | 5.1.2
sol-4-03 | change: The final literal reader admits Def [13,14) → Use [31,32) at cfg_incomplete only. Body var [42,43) reaches body use [59,60) at exact and cannot feed the reader. Keep the negative on an exact entry edge to the body use. | 5.1.2
sol-4-04 | keep | 5.2.3
sol-4-05 | keep | 5.3.3
sol-4-06 | keep | 5.3.4
sol-4-07 | keep | 5.3.4
sol-4-08 | keep | 5.3.4
sol-4-09 | keep | 5.2.2
sol-4-10 | change: Keep the plain alias binding. In instantiated require no edge or path from App:x to Render and refused_not_exact_singleton = 1; the alias walker does not unwrap the instantiation_expression. | 5.3.1
sol-4-11 | keep | 5.2.2
sol-4-12 | keep | 5.2.2
sol-4-13 | change: Keep ambiguous negative. In explicit require neither First nor Second receives an edge/path and refused_not_exact_singleton = 1. Neither function is a View candidate in barrel. | 5.3.1
sol-5-01 | keep | 5.1.2
sol-5-02 | keep | 5.1.2
sol-5-03 | keep | 5.2.1
sol-5-04 | keep | 5.3.2
sol-5-05 | keep | 5.3.4
sol-5-06 | keep | 5.3.4
sol-5-07 | keep | 5.3.1
sol-5-08 | keep | 5.3.1
sol-5-09 | keep | 5.3.1
terra-1-01 | keep | 5.3.4
terra-1-02 | keep | 5.3.4
terra-1-03 | keep | 5.3.4
terra-1-04 | keep | 5.3.4
terra-1-05 | keep | 5.3.4
terra-1-06 | keep | 5.3.4
terra-1-07 | keep | 5.3.4
terra-1-08 | keep | 5.3.4
terra-1-09 | change: Keep the graph assertions. Set base to green: base emits no accessor-field edge (Stage T/E RED record). | 5.2.1
terra-1-10 | change: Keep the graph assertions. Set base to green: base emits no prototype-key edge (Stage T/E RED record). | 5.2.1
terra-2-01 | keep | 5.3.4
terra-2-02 | keep | 5.3.4
terra-2-03 | keep | 5.3.4
terra-2-04 | keep | 5.3.4
terra-2-05 | keep | 5.1.1
terra-2-06 | keep | 5.1.3
terra-2-07 | keep | 5.2.1
terra-2-08 | keep | 5.1.3
terra-3-01 | keep | 5.3.4
terra-3-02 | keep | 5.3.4
terra-3-03 | keep | 5.3.4
terra-3-04 | keep | 5.3.4
terra-3-05 | keep | 5.3.4
terra-4-01 | keep | 5.2.3
terra-4-02 | keep | 5.1.3
terra-4-03 | keep | 5.3.4
terra-4-04 | keep | 5.1.1
terra-4-05 | keep | 5.2.1
terra-5-01 | keep | 5.1.3
terra-5-02 | keep | 5.1.3
terra-5-03 | keep | 5.1.3
terra-5-04 | keep | 5.1.3
d1-selected-endpoint | add: In parameter-body, refuse see(x) at [26,27) once (refused_param_list_shape). Omit the sole-use formal Def and both entry edges, including to the block local at [55,56). In parameter-bare require [12,13) → [22,23) at cfg_incomplete, exclude [50,51) and require a registered zero for refused_param_list_shape. Keep the block, loop, switch, catch and same-line-version controls. | 5.1.2
d1-rewritten-argument | add | 5.3.1
d1-debounce | add | 5.3.1
d1-pass-through | add | 5.3.1
d1-local-usecallback | add | 5.3.1
d1-shadowed-usecallback | add | 5.3.1
d1-unbound-singleton | add | 5.3.1
d1-import-alias | add | 5.3.1
d1-literal-member-source | add | 5.3.3
d1-jsx-bracket-family | add | 5.3.2
d1-bracket-beside-reads | add | 5.3.2
d1-multiline-selection | add: Full/base positives and two vetoes. Check the indexed full Use bytes, hidden-full base and broken fall-through in both negatives. | 5.3.4
d1-scoped-replacement | add | 5.2.3
d2-literal-selected-read | add: The inherited formal-to-shadow edges are fixture preconditions. Check the literal refusal and surviving formal reads. | 5.3.2
d2-guard-bare-arrow-literal | add | 5.3.5
d2-guard-rest-literal | add | 5.3.5
d2-forward-literal | add | 5.3.1
d2-jsx-selected-read | add: The inherited formal-to-shadow edges are fixture preconditions. Check the JSX refusal and surviving formal reads. | 5.3.2
d2-guard-bare-arrow-jsx | add | 5.3.5
d2-guard-rest-jsx | add | 5.3.5
d2-forward-jsx | add | 5.3.1
d2-default-capture | add: In m and body-write require exactly one [13,14) → [31,32) edge across all labels and require cfg_incomplete. Preserve m's separate body-var assertions. Keep closure-shadow and immediate-invocation negatives with their keep controls. In immediate-order refuse both call operands. In member-body-write omit the sole-use o Def and entry projection to [28,31), preserving the body write. In bare-order and opaque-sibling count one order and one shape refusal respectively, with keep's body edge in each. Read refusal counts through the S1 registry. | 5.1.2
d2-unknown-writer | add | 5.2.3
d2-unknown-target | add | 5.1.3
d2-multiline-shadow | add: Intentionally repeats d1-multiline-selection/shadow beside its source baseline. Witness the inner declaration at the indexed read even if disconnected. | 5.3.4
d2-multiline-version | add: Intentionally repeats d1-multiline-selection/stale beside its source baseline. Witness the intervening field write at the indexed read even if disconnected. | 5.3.4
d2-multiline-base-pairs | add: Intentionally repeats both d2 baselines together to check the two base paths independently of the conflict cases. | 5.3.4
d3-implicit-key-effects | add: Withhold the affected field projection with one write-effect refusal. Keep the disjoint bare body read and require zero write-effect refusals for that pair. | 5.1.2
d3-abrupt-key-order | add: A forward computed key blocks the final reader with one order refusal and no entry Def. Keep the same reader with the key omitted. | 5.1.2
d3-refusal-precedence | add: The bare outer-name sibling fails binding, the array sibling fails shape, and the nested default gives one nested-list refusal with a separate shadow verdict. Retain the nested control's outer body read. | 5.1.2
d3-mixed-line-fallback | add: Pin the raw write and alias twin at the condition bytes and exclude both signature endpoints. Preserve the wholly body-line anchors. The corresponding pre-S1 relocation requires the bounded §7 exception in DESIGN §5.9. | 5.1.2
d3-method-computed-name | add: Inventory the computed method key as other-binding for the method. Only the body operand receives the formal's edge. | 5.1.2
d3-method-decorator | add: Associate the sibling decorator with the method in TS and TSX. Require exactly one other-binding ledger entry at [15,16) for the formal at [21,22) in each grammar. Only the body operand receives the formal's edge. | 5.1.2
d3-signature-alias-omitted | add: With p omitted, require mixed-local base-read parity with the plain-formal twin and exclude all entry field projections. In shape-refused, count one shape refusal at [24,25) and require the body base read at [46,47) with no entry field projection. The call returns 1. | 5.1.2
d3-signature-alias-passed | add: With a distinct p supplied, require the same mixed-local base-read parity and absence of entry field projections. In shape-refused, count one shape refusal at [24,25) and require the body base read at [46,47) with no entry field projection. The call returns 7. | 5.1.2
d3-sibling-order | add: The later c formal causes order refusals at its own bare read and the downstream reader. In the reordered control the reader retains its cfg_incomplete edge. | 5.1.2

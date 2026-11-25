# Stage D row decisions

Applies to the four JSONL shards at `7ccb0eba04f1a575c0147fc6c2d70de9ab3d27b9`. There are **415 unique row IDs**, in shards of 92, 105, 98 and 120. All 18 `tbd` rows have a group below. This is the migration table for the implementing seats. The JSONL keeps the original observations until those slices apply their changes.

Read finding pairs as **default review / trace review** at default knobs. Explicit row knobs govern the walk and its catalog fan. A singleton catalog entry without a location qualifier uses the marked sink's file, projected line and operand, except that LineOnly has a null operand. Empty brackets mean zero decisions for the selected root. For BoundaryExited and NotReached source, terminal and descent depth are null. A Reached witness starts at the stated source and ends at the stated terminal. A missing finding at an ordinary observer has no matching sink rule (`unmodelled` at finding level if a reason is required). The truth column remains the semantic truth for the selected operand, including deliberate losses.

For a change to only one cell, inherit the other JSON fields. Keep existing `pending.levels` until the named producer or assertion is available. Apply the DESIGN §5.3 primary reason to the differing level, and retain additional candidates as evidence rather than asserting multiple primaries. A constructed edge test exercises the crossing contract; it cannot substitute for a missing real producer at the public entry point.

**CURRENT cells for materialisation (decision 6).** Keep `expect` and `reasons` as the target after all required admissions. For every row in a deferred group, and for dependent rows in any group, add `current: {until, expect, reasons}`. `until` lists the missing slice(s); `current.expect` has the same complete walk, review, trace_review and catalog fields as `expect`, and `current.reasons` uses the same per-level primary reasons. Record the active build and evidence in `note`. Measure the chosen build and assert CURRENT even when the target group is disabled or a cell already equals its target. Apply existing level-specific producer dependencies independently. Deferral alone never sets `pending` or skips a row. Install a cross-group CURRENT in the first slice that changes the answer, check it on every subsequent head, and switch to the target when its prerequisites ship. If only some ship, assert the resulting intermediate cell and its reason. Document the field and assertion selection in the fixture README when materialising the rows.

## Per-level prerequisites and intermediate cells

On materialisation add `prerequisites: {walk: [...], review: [...], trace_review: [...], catalog: [...], reasons: [...]}`. Each array is a conjunction. Use `binding`, `same_line`, `spelling` and `jsx_596` (the **specific** opening→target edge on the measured graph). Group remains ownership. An empty array asserts that level immediately once its harness exists. Track I-1/I-8 availability and other producer-specific tests separately. Record the sorted active prerequisites, graph/build and full CURRENT row for each encountered subset. For mixed levels select cells independently, then check reasons against the same active facts. A missing #596 edge has a measurable walk and ordinary review answer even while its future producer assertion is pending.

For example, materialise `s2-js-literal-alias-dot` with `walk: [binding, spelling]`, `review: []`, `trace_review: [binding, spelling]`, `catalog: [binding, spelling]` and `reasons: [binding, spelling]`. I-8 owns catalog execution. When only binding is present, its CURRENT has `until: [spelling]`, walk BoundaryExited with all three witness fields null, review exact, trace_review none and catalog []. Use `no_matching_site` for the proved trace loss. When only spelling is present, replace the missing prerequisite with binding and the cause with `target_not_binding`. A producer flag must identify the concrete emitted edge and grade, not just an upstream PR's merge.

The following map covers the four input shards after the amendments. Start a non-JSX admission with its single owning slice **only for the cells it changes**. Ordinary default review never depends on walker admissions. Independent negatives and already-satisfied levels have `[]`. In particular a `binding` group label cannot make a missing or overwritten leaf into a positive. The table overrides the single-slice rule and specifies which subsets have distinct answers. For a positive sink row with an existing Exact Use edge, the ordinary intermediate is BoundaryExited, the row's default grade / none, and []; its walk and trace loss is `target_not_binding` before binding, then the unresolved site reason below. For `observe` markers both findings are none and catalog is [] at every stage. All final Reached cells inherit the row's byte endpoints, depth, grades and catalog multiset unless amended here.

| Rows | Required facts for the target or change | CURRENT and activation |
|---|---|---|
| `s2-js-literal-alias-dot` | binding + spelling | Pinned BoundaryExited, exact / none, []; without binding the Use refuses `target_not_binding` even with spelling. With binding alone `go` fails the site filter (`no_matching_site`). With both, Reached d1, exact / unlabeled and the marked command hit. |
| `s3-tsx-alias-spread-restored`, `a2-tsx-memo-inline`, `a2-tsx-namespace-wrapper`, `a2-tsx-forwardref-inner-name` | jsx_596 + binding + spelling | No paired edge: NotReached, none / none, []. Exact edge alone or +spelling: BoundaryExited `target_not_binding`. Exact edge +binding alone: BoundaryExited `no_matching_site`. At the command sink default review becomes exact with the edge, trace none and [] until all three, then unlabeled and a Reached decision. The three wrapper observers stay none / none, []. A NameOnly edge refuses `not_exact` at Exact. In the first row the two opening spellings differ; prove the surviving alias leaf and exclude the fixed sibling. |
| `s1-tsx-same-line-sites`, `s2-tsx-overwrite-other-opening` | jsx_596 + binding + same_line | No edge: NotReached, none / none, []. On the Exact edge default review is exact. Without binding refuse the Use; with binding alone two same-spelling openings remain in the legacy count (`ambiguous_site`). Trace none, [] until both admissions, then Reached d1, exact / unlabeled with command hit. Overwriting the **other** opening preserves the selected leaf. |
| `b01`, `a1-tsx-attribute-name-is-value`, `a1-tsx-declarator-anchor`, `a2-tsx-callback-local`, `s3-tsx-competing-opening-prefix` | jsx_596 + binding | No edge: NotReached, none / none, []; Exact edge without binding: BoundaryExited `target_not_binding`; with binding: Reached d1, both findings none, []. In the prefix fixture both openings start together but the selected value is on the next line, beyond the closed first opening. Count at the minted value occurrence. Keep both token and zero-width anchor controls. |
| `s1-tsx-late-attribute`, `s1-tsx-continuation` | jsx_596 + binding | NotReached, none / none, [] without the edge; on the Exact edge, BoundaryExited, exact / none, [] until binding, then Reached d1, exact / unlabeled with command hit. One eligible opening at the value. |
| `s2-tsx-binding-third-hop`, `s3-tsx-competing-opening-computed`, `s2-tsx-formal-nested`, `s3-tsx-sibling-opening-nested-read` | jsx_596 for the conditional **refusal** on a demonstrated edge; no optional admission yields a positive | Without the relevant edge keep the pinned NotReached, none / none, []. For the third hop, with the edge the budget is exhausted first: BoundaryExited `depth_cap`, exact / none, []. For the computed read, `target_not_binding` applies with or without binding, before site ambiguity (exact / none, []). A paired nested read refuses `target_not_binding`; require evidence of a continuation before attributing it to the sink. Record actual producer-side review grades on that build. |
| `s1-tsx-late-spread`, `s2-tsx-overwrite-competing`, `s3-tsx-member-overwrite-literal-read`, `s3-tsx-write-after-spread`, `s3-tsx-rest-from-props` | [] for the pinned negative. jsx_596 for an assertion about the producer's suppression or refusal | Check the pinned cells at each subset. The selected leaf is overwritten, suppressed, outside the read contract or has no edge. Multiple openings or an alias name alone never requires a collision/spelling admission. A newly emitted edge triggers its actual guard or read test; do not replace the negative with Reached by group. |
| `tr2-tsx-child`, `a1-tsx-children-replaced`, `a1-tsx-key-is-metadata`, `a2-tsx-callback-capture`, `a2-tsx-concise-capture`, `a2-tsx-callback-shadow`, `a2-tsx-second-enclosing-owner`, `a2-tsx-signature-only`, `a2-tsx-forwardref-ref-slot` | [] for the negative | Assert the selected root's NotReached, none / none, []. Retain the missing child producer, replaced/metadata/other slot, shadow, absent capture or missing opening Use at its own level. No combination of walker admissions supplies it. |
| `a2-tsx-wrapper-local`, `a2-tsx-memo-identifier`, `a2-tsx-wrapper-let`, `a2-tsx-wrapper-nested`, `a2-tsx-memo-comparator-same-line`, `a2-tsx-wrapper-reexport`, `a2-tsx-anon-wrapper-fallback`, `a2-tsx-namespace-other-export`, `a2-tsx-callback-memo`, `a2-tsx-memo-in-factory`, `a2-tsx-wrapper-escaped-import`, `a2-tsx-default-object-member`, `a2-tsx-forwardref-empty-spread`, `a2-tsx-wrapper-satisfies`, `a2-tsx-wrapper-alias-default`, `a2-tsx-usecallback-element`, `a2-tsx-wrapper-shadowed-import` | [] for the pinned negative; #596-specific producer checks separately | NotReached, none / none, [] at every optional subset on the pinned graph. Bind the missing wrapper, export or lexical owner in the producer test. The word “alias”, a comparator on the declaration line, or a displayed wrapper name does not establish a second admissible opening. |
| `f03`; `m01`, `m01b` | binding for the changed reason or fan | f03 changes `target_not_binding` to `not_exact`. At the mixed-formal pair, prior to binding the selected p.a is BoundaryExited and p.b NotReached, exact / none, []; after binding use the two separate rows below, including the NotReached catalog operand at p.b. |
| `s2-js-alias-same-line`, `s2-js-alias-multiline-overlap`, `s3-ts-optional-default-competitor`; `s2-js-default-alias`, `s3-ts-optional-alias-rest-prefix` | spelling alone for the positive | From I-guard the first three require BoundaryExited, exact / none, [], `no_matching_site`; same_line cannot recover them. The latter two also need the alias site at I-spelling. A different-spelling fixed competitor cannot certify the actual. Namespace qualification that retains the member spelling (`s1-js-namespace-alias`) already works. |
| `b04`, `b04b`, `l07`, `b03`, `s2-ts-literal-refusal-far` | [] for the I-walk fence, version or already limiting refusal | No optional slice gates these assertions. Check the mandatory recursive controls, l07's zero consistent emissions, b03's third-hop depth, and the rejected literal subscript. A proved earlier cut still applies when a later optional admission is absent. |

For other non-JSX positives use binding alone on the literal/formal pairs (`js03`, `js04`, `b02`, `b08`, `g01`, `tr1-js-prefix`, `tr3-js-prefix-read`, `s1-js-formal-extension`, `s1-js-formal-write-sibling`); before binding require the Use fence. Use same_line alone on `js11`, `l02`, `l03`, `l05`, `tr2-js-rebind`, `tr5-js-multiline-two-sites`, `s2-go-dispatch-multiline-competitor`; retain the legacy collision before admission. Use spelling alone on `s01`–`s05` and `s1-js-optional-alias`; keep `no_matching_site` before spelling. Re-evaluate reason-only changes such as `w07` (alias lookup, then `unproven_slot`) even if the verdict holds. On `w02` keep NotReached, none / none and []. Before same_line the two `id` sites block entry. Only after the selected discarded call is entered can `return_not_entered_site` identify the cut to `separate`'s receiving Def. Keep the pinned missing-edge outcomes for `s3-js-nested-spread-alias-competitor`, `s3-js-mixed-rest-competitor` and `s3-ts-checked-branch-error-payload`. Catalog matcher aliases are independent of call-site spelling. Measure and record the complete active cells, including grades, before giving a slice credit.

## All formerly undecided rows

| Row | New group | Required cells | Why |
|---|---|---|---|
| `l05` | `same_line` | Reached d1 [45,49) → [19,20); none / none; [] | First call [72,79) contains the minted Use [74,78). The second call lacks its own argument Use. `sink` is an observer. |
| `w09` | `walk` | BoundaryExited; none / none; []; `not_exact` | The two local function expressions form a `local_def` pool without certified dispatch pedigree. The `observe` call has no catalog rule. |
| `w10` | `knobs` | Keep Reached d2 [177,178) → [68,69); exact / unlabeled; `structured-os-command` Reached | At (exact,3) the cheaper diamond wins. I-walk also repairs the shallow route for trace review at default depth 2. |
| `w10b` | `knobs` | Keep Reached d1 [136,137) → [61,62); exact / unlabeled; `structured-os-command` Reached | At (exact,3) the direct branch wins over the mutual-recursion detour. |
| `w10c` | `knobs` | Keep BoundaryExited; none / none; []; `depth_cap` | At (exact,0) the first descent is over budget. The observer supplies no finding. |
| `w10d` | `knobs` | BoundaryExited; none / none; []; `depth_cap` | Entering a then b spends two. The certified return to v would spend three. |
| `d02` | `walk` | Reached d1 [176,177) → [104,105); exact / unlabeled; `structured-os-command` Reached | One embedded method target; Go declares its receiver separately. Pin the promoted site and selected formal. |
| `d03` | `walk` | Reached d1 [164,165) → [65,68); nameonly / unlabeled; `structured-os-command` Reached | At [188,196) `r.run(t)` has one Exact `typed_param` target, `Base.run`. Set `axes.resolution` to `exact`. The bound instance supplies `self`, actual 0 supplies formal 1 `cmd`. |
| `tr2-js-wrapper` | `walk` | NotReached; unlabeled / none; []; walk and trace loss `unproven_return_input`, truth flows | Base used x [96,97) inside id(x) as wrap's synthetic return [93,98) to y [159,160). I-walk cuts that shortcut. There is no result assignment for the inner call. The default review path is unchanged. |
| `tr5-js-timeout` | `catalog` | NotReached; nameonly / nameonly; `structured-os-command` NotReached | Capture is `unmodelled`. The fan target t [87,87) on line 4 supports the name/line predicate. It is outside [113,114); record the decision with `operand_granularity` and no operand witness. |
| `s2-tsx-formal-nested` | `binding` | NotReached; none / none; []; `unmodelled` on base Keep #596-dependent direct-edge assertion in binding. | The opening has no base JSX edge. Never enter the invoked nested arrow via its parent's formal. If #596 supplies that exact Use edge, require BoundaryExited (`target_not_binding`); keep producer-specific checks distinct. |
| `s2-go-named-return-budget` | `control` | NotReached; none / none; []; `no_graph_edge` | Step 5c skips the naked return with zero value expressions. Two available ascents cannot supply the missing result. |
| `s2-js-computed-boundary-far` | `binding` | NotReached; none / none; []; `no_graph_edge` on base | The marked root has no refused crossing to propagate on the pinned graph. If a paired edge appears, reject p[k]; only a demonstrated continuation to leaf changes the verdict to BoundaryExited (`target_not_binding`). |
| `s2-ts-literal-refusal-far` | `reasons` | BoundaryExited; exact / none; []; `target_not_binding` | The pinned root stops at p [163,164). An independent probe from that exact Use reaches tail's x [108,109) in one descent. I-1 attributes it at a distance. Literal subscripts remain refused after I-binding. |
| `a2-tsx-callback-capture` | `control` | NotReached; none / none; []; `unmodelled` | The selected definition lies outside the executing opening owner. Neither an outer-name lookup nor a callback-local Use without that predecessor carries its value. |
| `a2-tsx-concise-capture` | `control` | NotReached; none / none; []; `unmodelled` | The selected definition lies outside the executing opening owner. Neither an outer-name lookup nor a callback-local Use without that predecessor carries its value. |
| `a2-tsx-signature-only` | `control` | NotReached; none / none; []; `no_graph_edge` | The expression-bodied signature supplies no reached opening-value Use. A parameter Def alone cannot emit this JSX edge. |
| `a2-tsx-second-enclosing-owner` | `control` | NotReached; none / none; []; `unmodelled` | The selected definition lies outside the executing opening owner. Neither an outer-name lookup nor a callback-local Use without that predecessor carries its value. |

## Other amendments and strengthened assertions

| Row | Group | Expectation change | Why |
|---|---|---|---|
| `b04` | `binding` → `walk` | BoundaryExited `self_param`; none / none; []. Pin on every I-walk entry with and without the view. | Base local DataFlow falsely reaches p.a at depth 0. Pair with b04b (control): NotReached; none / none; []; no sink warning, even if a recursive candidate is logged. |
| `js07` | `control` | Primary walk reason `unmodelled` | A reached array on the sink line is secondary projection evidence. The missing callback parameter route explains the operand loss. |
| `c04` | `catalog` | Keep the structured decision and operand; catalog verdict NotReached, no operand witness | The selected fan admits a candidate row, but the actual operand is reached only by a same-name line match or independent framework source. `operand_granularity`. |
| `l06` | `same_line` → `guard` | Keep BoundaryExited; primary `not_exact` | The edge-own guard ships independently and already determines this outcome before same-line widening. |
| `g01` | `control` → `binding` | Reached d1 [97,98) → [69,72); nameonly / unlabeled; `js-child-process` LineOnly Reached | Existing literal Use edge, singleton Exact call. `SameLine` is evidence about the read; it cannot by itself refuse descent or upgrade the finding. sinkA is the import-bound alias. |
| `f03` | `reasons` | CURRENT from I-1 while binding is absent: BoundaryExited, walk `target_not_binding`. Target after I-binding: BoundaryExited, walk `not_exact`. Both have nameonly / none and []. | I-binding certifies the first literal Use. The subsequent name-only method then blocks the route. If binding is deferred, assert the first reason and retain the second as data. |
| `m01` | `binding` | Keep walk/default; trace review unlabeled; catalog `structured-os-command` [88,91) NotReached on a.js:4. | In the mixed formal the fan reaches the base p Use in p.a. Review's identifier-on-line predicate can select exec(p.b) on that same line. Preserve its decision and `operand_granularity`; there is no witness from this root to the sibling p.b. |
| `m01b` | `binding` | Keep walk/default; trace review unlabeled; catalog `structured-os-command` [88,91) NotReached on a.js:4. | In the mixed formal the fan reaches the base p Use in p.a. Review's identifier-on-line predicate can select exec(p.b) on that same line. Preserve its decision and `operand_granularity`; there is no witness from this root to the sibling p.b. |
| `tr1-py-star` | `walk` | Walk boundary reason `unproven_slot`; keep default nameonly | A starred prefix prevents proof of the second formal. The actual refused hop has a specific slot cause. |
| `tr2-js-rebind` | `same_line` | Reached d1 [100,101) → [61,62); nameonly / unlabeled; `structured-os-command` Reached | The first actual has its own bytes before the intervening write. Only that occurrence has the incoming version; the later call cannot enter with it. |
| `tr5-js-two-roots` | `catalog` | Catalog [] | The selected t has no fan decision at exec(u). The review finding belongs to the other explicit line root. |
| `tr6-py-bound` | `control` | Keep BoundaryExited; nameonly / none and []; walk and trace-review primary `not_exact`. Set `axes.resolution` to `nameonly`. | `Runner().run(t)` resolves name-only at the default ceiling. Its plain method can bind actual 0 to `cmd` under (iv); the offset adds no independent refusal. |
| `tr6-py-decorator` | `walk` | Walk boundary reason `unproven_slot`; keep default nameonly | The wrapper can replace the argument. Resolution alone does not certify the original formal. |
| `a1-js-equal-budget-return-sites` | `walk` | Keep endpoint cells; witness actual [175,176), site [167,180), return Use [64,65), receiving Def [158,164); correct row.site from [135,148) to [167,180). | The seed [96,97) reaches the second call and terminal [189,195). Reject the first site's [143,144) actual even when endpoint bytes and depth agree. Use separate first/second result queries in I-walk. Run DESIGN 5.2's mixed-layout companion at I-same-line. |
| `a1-py-packed-result` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | Two return components and a single packed destination fail Step 5c's arity check. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `e2-js-inline-yaml` | `catalog` | Keep []; cover unsupported family 23 in the catalog diagnostic. | The source event, empty originating set, whole-file scan or cross-fan decision specified for this family in DESIGN 5.6 is unavailable from one fan. Preserve seed errors and any independently supported route. |
| `e2-python-inline-flask-positional` | `catalog` | Keep []; cover unsupported family 24 in the catalog diagnostic. | The source event, empty originating set, whole-file scan or cross-fan decision specified for this family in DESIGN 5.6 is unavailable from one fan. Preserve seed errors and any independently supported route. |
| `e2-python-inline-flask-keyword` | `catalog` | Keep []; cover unsupported family 25 in the catalog diagnostic. | The source event, empty originating set, whole-file scan or cross-fan decision specified for this family in DESIGN 5.6 is unavailable from one fan. Preserve seed errors and any independently supported route. |
| `e2-go-framework-table` | `catalog` | Catalog operand [114,115) (was [20,21)); keep Reached | Use the matched c.File(t) argument's file bytes. The earlier t at [20,21) belongs to the import keyword. |
| `e2-go-no-originating-path` | `catalog` | Keep []; cover unsupported family 27 in the catalog diagnostic. | The source event, empty originating set, whole-file scan or cross-fan decision specified for this family in DESIGN 5.6 is unavailable from one fan. Preserve seed errors and any independently supported route. |
| `e2-go-originating-path` | `catalog` | Catalog operand [105,106) (was [20,21)); keep Reached | Use the matched c.File(t) argument's file bytes. The earlier t at [20,21) belongs to the import keyword. |
| `e2-go-mixed-inline` | `catalog` | Keep []; cover unsupported family 29 in the catalog diagnostic. | The source event, empty originating set, whole-file scan or cross-fan decision specified for this family in DESIGN 5.6 is unavailable from one fan. Preserve seed errors and any independently supported route. |
| `s1-js-long-continuation` | `control` | Walk Reached d0, none / none, []. Default `review_cfg_filter`, trace review and catalog `matcher_admission`. | On line 26 default's 20-line CFG scan misses the call start at 4. Trace reaches t [708,709), but `call_arg_node(0)` selects the leading comment [99,123). The equal-width whitespace control restores trace only. Shorten to one continuation line to isolate the matcher in both modes. |
| `s2-catalog-js-inline-operand-other-fan` | `catalog` | Keep the structured decision and operand; catalog verdict NotReached, no operand witness | The selected fan admits a candidate row, but the actual operand is reached only by a same-name line match or independent framework source. `operand_granularity`. |
| `s3-js-nested-spread-alias-competitor` | `spelling` → `control` | NotReached; none / none; []; `no_graph_edge` | No binding into the selected inner formal past the spread. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-js-mixed-rest-competitor` | `guard` → `control` | NotReached; none / none; []; `no_graph_edge` | The selected receiver/rest edge is missing. No refused candidate is reachable. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-ts-nested-spread-default` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | No edge maps the spread-prefixed actual to the defaulted formal. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-js-rest-before-inner-call` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | The selected rest element has no positional formal edge. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-go-dispatch-variadic-tail` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | No edge supplies the named variadic element in the target method. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-go-variadic-fixed-defer-return` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | The nested inline return is missing before the fixed-slot call. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-tsx-sibling-opening-nested-read` | `binding` | Primary walk loss `unmodelled` on the absent capture path | If a producer subsequently emits a direct edge to the nested Use, the specific refusal is `target_not_binding`. Never assign both to the same existing hop. |
| `s3-ts-awaited-refused-ascent-two-more` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | The awaited assignment needs an existing ReturnFlow to its destination. The pinned walk reports no crossing boundary along this route. Exercise a wired return separately; the budget cannot repair the missing edge. |
| `s3-js-recursive-return-distal` | `walk` | BoundaryExited; primary `depth_cap` for **both** `walk` and `trace_review`; keep `self_param` only on the recursive hop. | Relaxing depth alone admits the route through `relay`, `finish`, `last`. Relaxing recursion alone still leaves the depth barrier. |
| `s3-py-escape-in-callee-other-fan` | `catalog` | Trace review unlabeled (was none); keep selected catalog Sanitized | The single-source harness still emits a trace finding. Preserve review's own cleansing and attribute the catalog decision to the selected fan. |
| `s3-py-two-escapes-categories` | `catalog` | Both findings unlabeled (were none); keep selected catalog Sanitized | Confirmed by the one-marked-line harness. The selected HTML and independent command operands require separate root/operand records. |
| `s3-ts-guard-return-destructure` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | Step 5c cannot supply the required destructured result. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-ts-await-branch-rejoin` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | The awaited assignment needs an existing ReturnFlow to its destination. The pinned walk reports no crossing boundary along this route. Exercise a wired return separately; the budget cannot repair the missing edge. |
| `s3-ts-checked-branch-error-payload` | `binding` → `control` | NotReached; none / none; []; `no_graph_edge` | The constructor/literal leaf path has no producer. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `s3-py-context-binding-to-next-manager` | `control` | Primary walk reason `no_graph_edge` | There is no producer for entering the next manager through this binding. Context-management syntax describes the missing operation. |
| `s3-py-receiver-keyword-forward-return` | `walk` | Default review unlabeled (was none); keep walk NotReached, trace none, catalog [] | The executable one-root harness reports the review finding. Absence of the bound keyword/result edge constrains the walk only. |
| `s3-go-fluent-overwrite-two-sites` | `same_line` → `control` | Reached d0 [221,222) → [307,308); unlabeled / unlabeled; `structured-os-command` Reached; retain known_wrong | The observed path is local assignment propagation. Certifying call sites cannot kill the false fluent-field path. Keep truth no_flow and file the local propagation defect. |
| `s3-go-if-init-nested-return` | `walk` → `control` | NotReached; none / none; []; `no_graph_edge` | The inline result supplied to apply lacks a modeled receiving assignment. Lane 8 adds no producer. Unit-test the rejection separately on a constructed edge where applicable. |
| `l07` | `same_line` | BoundaryExited. I-1 walk reason `ambiguous_site` for the baseline collision. Assert `no_matching_site` from I-walk, including when same-line admission is deferred. | The pre-write actual supplies only formal `b`. Version facts exclude the post-write actual for formal `a`. Zero consistent emissions remain. |
| `tr1-js-default` | `walk` | Reached d1 [114,115) → [73,74); exact / unlabeled; `structured-os-command` Reached. Clear `known_wrong` and the slot-loss reasons. | Actual 0 supplies `a` on the defined-value path. |
| `s1-py-default` | `walk` | Reached d1 [73,74) → [45,46); nameonly / unlabeled; `structured-os-command` Reached. Clear `known_wrong` and the slot-loss reasons. | Providing this positional argument binds `x` directly. |
| `s1-py-cls` | `walk` | Reached d1 [109,110) → [81,82); nameonly / unlabeled; `structured-os-command` Reached. Clear `known_wrong` and the slot-loss reasons. | `Runner.run(t)` binds `cls` through `@classmethod`. Actual 0 enters formal 1 `x`. |
| `s1-js-slot-far` | `walk` | Keep BoundaryExited; walk and trace-review primary `depth_cap`. | The supplied value crosses into the defaulted formal. `main → run → first → last` needs three descents at budget two. |
| `s2-js-default-alias` | `spelling` | Target Reached d1 [114,115) → [73,74); exact / unlabeled; `structured-os-command` Reached. Until I-spelling: BoundaryExited, exact / none, [], `no_matching_site`. | `send(t)` supplies the defaulted formal at a uniquely resolving renamed site. Remove the target `unproven_slot` reasons. |
| `s2-js-alias-same-line` | `same_line` → `spelling` | I-guard CURRENT: BoundaryExited, exact / none, [], `no_matching_site` for walk and trace review. I-spelling target: restore Reached d1 [104,105) → [63,64), exact / unlabeled, `structured-os-command` Reached. | Only `go(t)` [161,166) contains this actual. `run('fixed')` on the same line cannot certify it. |
| `s2-js-alias-multiline-overlap` | `spelling` | I-guard CURRENT: BoundaryExited, exact / none, [], `no_matching_site` for walk and trace review. I-spelling target: restore Reached d1 [67,68) → [70,71), exact / unlabeled, `structured-os-command` Reached. | The imported alias spans [92,102). The `run('fixed')` site on its continuation line excludes `t`. |
| `s3-ts-optional-default-competitor` | `same_line` → `spelling` | I-guard CURRENT: BoundaryExited, exact / none, [], `no_matching_site` for walk and trace review. I-spelling target: Reached d1 [123,124) → [81,82), exact / unlabeled, `structured-os-command` Reached. Clear `known_wrong` and the target slot-loss reasons. | Only `alias?.(t)` [191,201) supplies `t` to the defaulted `x`. On base, removing the adjacent `run('fixed')` alone changes the verdict to BoundaryExited. |

## Remaining rows

These 346 input rows retain their group. The prerequisite, reason and per-level amendments in this document take precedence over shard cells. Each ID occurs once in the disposition inventory above or below. The cross-cutting contracts still apply: root-specific verdicts, byte-verified paths, the five-family coverage diagnostic, projected continuation lines, and one primary reason per sink difference. In particular, retain the boundary-bypass verdict in `a1-js-sanitizer-boundary-bypass`, the matched-category paired proof in `s3-go-paired-clean-independent-fan`, and the comment-skipping data-slot test in `a1-js-sanitizer-comment-slot`. Line-based flow and a matcher decision can disagree about the same operand.

| Shard | Group | Count | IDs |
|---|---|---:|---|
| rows.jsonl | binding | 8 | `js03`, `js04`, `b01`, `b02`, `b03`, `b05`, `b06`, `b08` |
| rows.jsonl | catalog | 11 | `c01`, `c02`, `c03`, `c05`, `c06`, `c07`, `c08`, `c09`, `c10`, `c11`, `c12` |
| rows.jsonl | control | 33 | `js00`, `js00b`, `js01`, `js02`, `js05`, `js06`, `js08`, `js09`, `js10`, `js11b`, `js12`, `js13`, `js13b`, `js14`, `js14c`, `py01`, `py02`, `py03`, `py04`, `go01`, `go02`, `go03`, `go04`, `l01`, `w04`, `w08`, `b04b`, `b07`, `x01`, `m02`, `x02`, `d01`, `p01` |
| rows.jsonl | guard | 2 | `n01`, `n02` |
| rows.jsonl | knobs | 3 | `k01`, `k02`, `k03` |
| rows.jsonl | reasons | 2 | `f01`, `f02` |
| rows.jsonl | same_line | 5 | `js11`, `l02`, `l03`, `l04`, `w02` |
| rows.jsonl | spelling | 7 | `s01`, `s02`, `s03`, `s04`, `s05`, `s05b`, `w07` |
| rows.jsonl | walk | 4 | `w01`, `w03`, `w05`, `w06` |
| rows-t2a.jsonl | binding | 6 | `tr1-js-prefix`, `tr3-js-computed-match`, `tr3-js-dynamic-match`, `tr3-js-prefix-read`, `a1-tsx-attribute-name-is-value`, `a1-tsx-declarator-anchor` |
| rows-t2a.jsonl | catalog | 31 | `tr2-js-sanitize-callee`, `tr5-js-same-seed`, `a1-js-sanitizer-comment-slot`, `a1-js-sanitizer-parenthesized`, `a1-js-sanitizer-boundary-bypass`, `a1-js-named-handler`, `a1-js-shadowed-handler`, `e2-flat-js-call`, `e2-flat-js-module`, `e2-flat-js-construct`, `e2-flat-js-write`, `e2-flat-js-jsx-name`, `e2-flat-python-call`, `e2-flat-python-module`, `e2-flat-go-call`, `e2-flat-configured`, `e2-flat-weak`, `e2-structured-positional`, `e2-structured-semantic`, `e2-structured-client-binding`, `e2-structured-jsx`, `e2-python-template-source`, `e2-python-template-keyword`, `e2-js-request-flat-call`, `e2-js-request-flat-write`, `e2-js-inline-structured-arg`, `e2-target-seed-synthesis`, `e2-framework-source-guard`, `e2-path-category-guard`, `e2-path-call-guard`, `e2-python-source-line-cleanser` |
| rows-t2a.jsonl | control | 47 | `tr1-js-spread-after`, `tr1-js-rest`, `tr1-js-duplicate`, `tr1-py-kwargs`, `tr1-ts-destructure`, `tr1-go-variadic`, `tr2-py-unbound`, `tr2-go-methodexpr`, `tr2-tsx-child`, `tr3-js-member-write`, `tr3-js-spread-overwrite`, `tr3-js-multiline-descent`, `tr4-js-root-two-callers`, `tr4-js-return-then-descend`, `tr4-js-mutual-return`, `tr4-js-class-override`, `tr5-js-receiver-field`, `tr5-ts-generic`, `tr5-tsx-assert`, `tr6-py-dict-key`, `tr6-py-dict-sibling`, `tr6-py-kwonly`, `tr6-py-closure`, `tr6-go-struct`, `tr6-go-multireturn`, `tr6-go-multireturn-safe`, `tr6-go-defer`, `tr6-go-goroutine`, `tr6-go-closure`, `a1-js-site-other-caller`, `a1-js-literal-exact-pool`, `a1-ts-earlier-formal-hole`, `a1-js-bare-arrow-formal`, `a1-js-formal-computed-write`, `a1-js-shorthand-proto`, `a1-tsx-children-replaced`, `a1-tsx-key-is-metadata`, `a1-go-blank-live-result`, `a1-go-blank-source-result`, `a1-go-field-sibling-result`, `a1-go-forwarded-result-list`, `a1-go-partial-return-certificate`, `a1-py-nested-use-certificate`, `a1-py-nested-return-owner`, `a1-py-nondata-keyword`, `a1-js-sanitizer-sibling-def`, `a1-js-handler-response-name` |
| rows-t2a.jsonl | guard | 1 | `tr4-js-nameonly-far3` |
| rows-t2a.jsonl | same_line | 1 | `tr5-js-multiline-two-sites` |
| rows-t2a.jsonl | walk | 2 | `tr2-js-discard`, `a1-js-rejoin-after-return` |
| rows-t2b.jsonl | binding | 8 | `s1-tsx-late-spread`, `s1-tsx-late-attribute`, `s1-tsx-continuation`, `s1-js-formal-extension`, `s1-js-formal-write-sibling`, `s1-js-formal-invoked-nested`, `s2-js-literal-alias-dot`, `s2-tsx-binding-third-hop` |
| rows-t2b.jsonl | catalog | 18 | `s1-js-sanitize-two-sink-categories`, `s2-js-sanitized-two-roots`, `s2-js-sanitizer-through-return`, `s2-catalog-js-module-member-alias`, `s2-catalog-tsx-flat-spelling`, `s2-catalog-py-flat-attr`, `s2-catalog-py-instance-alias`, `s2-catalog-go-format-position`, `s2-catalog-js-weak-parameterized`, `s2-catalog-js-client-member`, `s2-catalog-tsx-structured-html-continuation`, `s2-catalog-py-template-argument-zero`, `s2-catalog-js-framework-flat-call`, `s2-catalog-js-framework-flat-write`, `s2-catalog-go-http-framework-slot`, `s2-catalog-js-configured-only`, `s2-catalog-go-originating-fan`, `s2-js-sanitized-separate-roots` |
| rows-t2b.jsonl | control | 41 | `s1-js-iife`, `s1-ts-chained`, `s1-ts-new`, `s1-js-namespace-alias`, `s1-js-nested-outside`, `s1-go-method-value`, `s1-js-formal-nested-sibling`, `s1-js-call`, `s1-js-apply`, `s1-js-bind`, `s1-py-posonly`, `s1-py-args`, `s1-py-double-star`, `s1-go-variadic-prefix`, `s1-go-variadic-slice`, `s1-py-return-root`, `s1-ts-await-return`, `s1-ts-depth-three`, `s1-go-dispatch-return`, `s1-js-unresolved-callable`, `s1-js-field-use-source`, `s1-js-sanitize-callee-html`, `s1-js-sanitize-bypass`, `s1-js-seed-literal`, `s1-py-framework-extra-root`, `s1-js-operand-two-uses`, `s2-ts-literal-dynamic-sibling`, `s2-py-bound-keyword`, `s2-py-unbound-star`, `s2-py-posonly-mapping-shadow`, `s2-py-star-before-keyword`, `s2-go-methodexpr-variadic`, `s2-ts-destructured-rebind`, `s2-js-callback-far`, `s2-js-wrong-category-bypass`, `s2-ts-field-argument`, `s2-py-field-argument`, `s2-go-receiver-field-method-value`, `s2-catalog-ts-constructor-operand`, `s2-catalog-ts-dom-assignment`, `s2-catalog-ts-yaml-schema-binding` |
| rows-t2b.jsonl | guard | 3 | `s1-js-nameonly-unrelated`, `s1-js-boundary-target`, `s2-js-mixed-bound-alias` |
| rows-t2b.jsonl | knobs | 1 | `s2-js-zero-budget-ascend` |
| rows-t2b.jsonl | reasons | 2 | `s1-js-boundary-unrelated-sink`, `s2-py-bound-boundary-far` |
| rows-t2b.jsonl | same_line | 4 | `s1-tsx-same-line-sites`, `s2-go-dispatch-multiline-competitor`, `s2-tsx-overwrite-competing`, `s2-tsx-overwrite-other-opening` |
| rows-t2b.jsonl | spelling | 1 | `s1-js-optional-alias` |
| rows-t2b.jsonl | walk | 8 | `s1-js-self-return`, `s1-js-ascent-two-descents`, `s1-py-return-expression-gain`, `s2-js-star-both-sides`, `s2-py-discard-multiline-return`, `s2-js-three-ascents`, `s2-js-competing-return-budget`, `s2-py-framework-bound-other-root` |
| rows-t2c.jsonl | binding | 24 | `s3-tsx-member-overwrite-literal-read`, `s3-tsx-competing-opening-computed`, `s3-tsx-competing-opening-prefix`, `s3-tsx-rest-from-props`, `s3-tsx-write-after-spread`, `a2-tsx-callback-local`, `a2-tsx-wrapper-local`, `a2-tsx-memo-identifier`, `a2-tsx-callback-shadow`, `a2-tsx-wrapper-let`, `a2-tsx-wrapper-nested`, `a2-tsx-memo-comparator-same-line`, `a2-tsx-wrapper-reexport`, `a2-tsx-anon-wrapper-fallback`, `a2-tsx-namespace-other-export`, `a2-tsx-forwardref-ref-slot`, `a2-tsx-memo-in-factory`, `a2-tsx-wrapper-escaped-import`, `a2-tsx-default-object-member`, `a2-tsx-forwardref-empty-spread`, `a2-tsx-wrapper-satisfies`, `a2-tsx-wrapper-alias-default`, `a2-tsx-usecallback-element`, `a2-tsx-wrapper-shadowed-import` |
| rows-t2c.jsonl | catalog | 28 | `s3-py-multiret-competing-budget-far`, `s3-ts-two-recognizers-shared-sink`, `s3-ts-offpath-other-root`, `s3-tsx-html-same-row-different-fans`, `s3-tsx-html-bypass-other-fan`, `s3-py-keyword-escape-other-fan`, `s3-go-paired-clean-independent-fan`, `s3-go-paired-offpath-independent-fan`, `s3-go-paired-guard-bypass-fan`, `s3-py-forward-star-mapping-wrapper`, `s3-go-error-reuse-slot`, `s3-go-guarded-fallback-return`, `a2-js-route-builder`, `a2-js-adapter-argument`, `a2-js-fastify-option-handler`, `a2-js-fastify-config-reference`, `a2-js-koa-context`, `a2-ts-nest-decorated`, `a2-ts-nest-header-neighbor`, `a2-py-fastapi-annotated`, `a2-py-django-view-file`, `a2-py-django-text-context`, `a2-go-gin-grouped`, `a2-go-mux-sibling-formal`, `a2-js-receiver-shadow`, `a2-go-gin-closure-owner`, `a2-py-flask-unpack-arity`, `a2-py-flask-named-expression` |
| rows-t2c.jsonl | control | 36 | `s3-js-apply-alias-competitor`, `s3-js-computed-method-literal-competitor`, `s3-py-bound-keyword-competitor`, `s3-py-mapping-wrapper-competitor`, `s3-py-unbound-explicit-star-return`, `s3-go-interface-methodexpr-tail`, `s3-js-wrong-return-distal-competitor`, `s3-js-return-into-unrelated-sink`, `s3-ts-constructor-options-receiver-read`, `s3-ts-next-middleware-locals`, `s3-ts-nonnull-object-leaf-member-await`, `s3-ts-await-object-return-sibling`, `s3-py-context-map-late-unpack`, `s3-py-factory-chain-receiver-state`, `s3-py-dynamic-key-chained-map`, `s3-go-decode-output-parameter`, `s3-go-fluent-intermediate-argument`, `s3-go-nested-pointer-struct-literal`, `s3-go-callback-write-outer-local`, `a2-js-stored-builder`, `a2-js-adapter-thunk`, `a2-ts-nest-param-alias`, `a2-py-fastapi-default`, `a2-py-fastapi-registration`, `a2-go-gin-type-alias`, `a2-py-fastapi-sibling-use`, `a2-js-framework-precedence`, `a2-js-koa-bare-arrow`, `a2-py-route-precedence`, `a2-py-flask-registration`, `a2-js-express-head`, `a2-js-inline-factory`, `a2-py-flask-sibling-target`, `a2-py-fastapi-model-alias`, `a2-py-fastapi-star-receiver`, `a2-js-express-error-middleware` |
| rows-t2c.jsonl | guard | 1 | `a2-tsx-callback-memo` |
| rows-t2c.jsonl | spelling | 5 | `s3-ts-optional-alias-rest-prefix`, `s3-tsx-alias-spread-restored`, `a2-tsx-memo-inline`, `a2-tsx-namespace-wrapper`, `a2-tsx-forwardref-inner-name` |
| rows-t2c.jsonl | walk | 3 | `s3-js-refused-ascent-one-more`, `s3-js-root-ascent-refusal-two-more`, `s3-go-named-explicit-ascent-far` |

Final groups for the 415 input rows: `binding` 52, `catalog` 101, `control` 179, `guard` 8, `knobs` 8, `reasons` 6, `same_line` 13, `spelling` 17, `walk` 31. Total 415. New rows specified below are additional to this historical population. The catalog and walk group assignments are implementation responsibility, not observations of a shipping producer.

## Reason-to-evidence audit for I-1

Pass over `rows.jsonl`, `rows-t2a.jsonl`, `rows-t2b.jsonl` and `rows-t2c.jsonl`: 415 IDs; 219 input rows with at least one reason, 487 individual level/reason entries. Read the reason at its own level against the marked root/operand, source, target cell, note and amendments above. An absent catalog command provides no matcher evidence. Keep historical reasons as candidates in the receipt; assert exactly the causal primary for the active build. The changes below supplement the earlier disposition tables.

| Rows and levels | Reason amendment and required evidence |
|---|---|
| `x01` (default review) | `uncertified_flow`. The value of t in `forget(t)` is transferred into z despite the fixed return. The use of z at [141,142) is the actual sink operand. Change only the actual to a literal: the default finding disappears. Walk NotReached, unlabeled / none, []. Keep `shortcut_suppressed` on the trace assignment cut. |
| `s1-js-self-return` | Walk BoundaryExited, none / none, []. Default review needs the omitted ReturnFlow (`review_no_return_flow`). Walk and trace review meet the recursive return to y [132,133) (`self_return`). Split the base case into a separate callee: the walk reaches via ReturnFlow and trace review reports, while default review still omits the sink. |
| `s2-js-mixed-bound-alias` (default review) | Replace `operand_granularity` with `uncertified_flow`. The tainted actual of `other.run(t)` reaches the selected Runner formal/exec operand through uncertain dispatch. The actual to the proven Runner is fixed. Changing only the first actual to a literal removes the base finding; the sink and exact sibling remain. Keep the edge-own `not_exact` on the trace refusal. |
| `s2-js-star-both-sides`, `tr1-py-star`, `tr6-py-decorator` (default review) | `uncertified_flow`: record the wrong positional formal after spread, absent second Python argument, or wrapper substituting a constant. Review reaches the marked operand through that binding. The stage-D trace hop refuses `unproven_slot`. Replacing the JS spread call with the effective `run(t, 'fixed')` removes both base findings. |
| `b06`, `s2-ts-literal-dynamic-sibling` (default review) | `uncertified_flow` for the existing leaf→Use into a computed read of the fixed sibling. The reached Use is **inside** the marked operand. Using the equivalent static clean-field read removes both the false binding and the base finding. Keep `target_not_binding` for the trace hop; a flat/line matcher alone does not explain the gain. |
| `s2-py-discard-multiline-return`, `tr6-go-multireturn-safe` (default review) | Replace projection or `unmodelled` with `uncertified_flow` for an assignment transfer into the actual fixed sink variable. Bind the same-line discarded input/independent assignment or the argument transferred into the fixed first result. Splitting the two Python calls removes the default finding (base trace separately returns through the wrong caller site). Retain `shortcut_suppressed` **on the trace cut** when its suppression certificate exists. |
| `a1-go-partial-return-certificate`, `a1-py-nested-use-certificate`, `a1-py-nondata-keyword`, `a1-js-sanitizer-sibling-def`; amended `s3-go-fluent-overwrite-two-sites` (positive cells against no_flow) | Replace `unmodelled` or missing reason with `uncertified_flow`. Base witnesses contain AssignmentPropagation into respectively the incomplete constant result, fixed outer result, fixed payload from a quote flag, fixed sibling Def, or overwritten fluent result. Pin the relation and receiver/slot/version to the reached operand. Splitting the JS sibling definitions removes its witness and findings. Keep known_wrong; these local transfers survive the crossing repair. |
| `e2-flat-configured`, `s2-catalog-js-configured-only` (review, trace review, catalog) | `matcher_admission` for an empty configured sink list. Record the reached operand and actual list. Require an I-8 API positive on the identical fixture with only the matched extra-sink rule added. The base CLI provides the empty-list negative. A built-in sink spelling control exercises recognition independently of configuration. |
| `e2-framework-source-guard` (same three levels) | `matcher_admission` for the live framework source-row mask. Record the matched pattern, seed row/range and applied predicate. Moving only `db.Query(t)` to the next line preserves reach and changes both base grades from none to exact. The two admission cases have distinct evidence even though both run with context off. |
| `e2-js-request-flat-call`, `e2-js-request-flat-write` | Replace the undifferentiated walk `unmodelled`: the line-keyed fan reaches the earlier occurrence on the sink row, while the later marked operand does not receive that flow. Use `operand_granularity` for the comparison with the flat matcher, recording both occurrences and the request-range predicate. Pin the missing def/use edge separately at the walk level. |
| `e2-go-mixed-inline`; `e2-target-seed-synthesis` (finding vs selected-root catalog) | For the exclusive cross-fan fallback use `unmodelled` + family 29 and its empty-matching-fan test. A different root alone does not establish exclusivity. For target synthesis use `operand_granularity` with the separate synthesized root ID, inventory entry 31. An explicit anchor outside the handler is not that root. |
| `a1-js-sanitizer-boundary-bypass` (default review) | Replace `unmodelled` with `sanitizer_model`: inspect review's cleansing on the same fan. Substituting an unrecognized helper for `escapeHtml` makes both base findings appear. Keep `depth_cap` on the too-deep raw trace branch. Attribute a lost finding to the first sufficient difference, retaining joint evidence. |
| `b01`, `tr5-js-two-roots`, `s3-py-receiver-keyword-forward-return`, `m01` | Clear stale reasons on the **corrected level**: b01's `no_graph_edge` describes producer-absent CURRENT, never a Reached target. tr5's extra-root reason describes ordinary review, never a selected-root hit in []; record the root difference on the comparison. The Python forwarder's confirmed default finding cannot retain a loss `no_graph_edge`. In m01 the post-binding trace finding is present; the remaining projection discrepancy is its sibling operand verdict. |
| `tr2-py-unbound`, `tr2-go-methodexpr`, `a1-go-field-sibling-result`, `a1-go-forwarded-result-list` | Use `no_graph_edge` for the specific missing argument/result route on the pinned graph. A method-expression spelling or a field destination describes the producer limit. The last two require the result-edge evidence (non-simple LHS, forwarded result list). Do not claim a refused crossing without an existing candidate. |
| `s3-ts-next-middleware-locals`, `s3-go-callback-write-outer-local` | For the missing framework callback/capture use `unmodelled`, with the missing callable relation. Keep missing local mutation edges as secondary candidates. Never count both labels against the same observation. |

The second pass enumerated every input (ID, level, reason), reapplied the row-specific overrides, and checked the mechanism against that client’s root/operand, cell, truth and fixture. Base navigation and both ordinary review projections were replayed on all 415 fixture rows. The following are **input** counts before amendments. A multi-reason cell contributes once per reason.

| Shard | Rows with a reason | Walk | Default review | Trace review | Catalog |
|---|---:|---:|---:|---:|---:|
| rows.jsonl | 40 | 34 | 8 | 5 | 3 |
| rows-t2a.jsonl | 55 | 35 | 45 | 44 | 10 |
| rows-t2b.jsonl | 48 | 32 | 39 | 40 | 4 |
| rows-t2c.jsonl | 76 | 53 | 64 | 70 | 1 |
| Total | 219 | 154 | 156 | 159 | 18 |

After applying the prior amendments, three rows required a change of mechanism at the reporting level: x01, s1-js-self-return and s1-js-long-continuation. No additional review/trace attribution swap was identified. Shared missing-producer reasons require the same missing relation on both clients. A real extra root or line projection retains `operand_granularity`. A genuine observer without a rule uses `unmodelled` for the missing finding. A trace boundary, wrong-site cut or budget can explain trace loss only when it connects to the operand. On a correct negative keep it as hop evidence. Catalog decisions, future producer/CURRENT cells and wire reasons have no executable base oracle. They retain the evidence obligations below.

Earlier corrections also apply during this pass. The slot-reason replacements are `tr1-js-default`, `s1-py-default`, `s1-py-cls`, `tr6-py-bound`, `s1-js-slot-far`, `s2-js-default-alias`, `s3-ts-optional-default-competitor`. Use the positive witnesses, actual resolution ceiling or depth in the row-specific decisions. For `l07`, `tr2-js-rebind`, `g01`, `f03`, `s3-js-recursive-return-distal`, `s3-tsx-sibling-opening-nested-read`, `js07` and `s1-js-long-continuation`, distinguish the slice, direction, refused edge and selected occurrence before dropping a secondary. For `a1-py-packed-result`, `s3-js-nested-spread-alias-competitor`, `s3-js-mixed-rest-competitor`, `s3-ts-nested-spread-default`, `s3-js-rest-before-inner-call`, `s3-go-dispatch-variadic-tail`, `s3-go-variadic-fixed-defer-return`, `s3-ts-awaited-refused-ascent-two-more`, `s3-ts-guard-return-destructure`, `s3-ts-await-branch-rejoin`, `s3-ts-checked-branch-error-payload`, `s3-go-if-init-nested-return` and `s3-py-context-binding-to-next-manager`, the specific missing producer takes precedence over a hypothetical slot, depth, or return-bearing path. Apply this to the affected level only. For b04b keep `self_param` on the recursive attempt, never project a sink boundary onto the CFG-impossible earlier local read.

Keep genuine line/fan/root comparisons under `operand_granularity` (for example c04, c09, tr5's independent explicit root, and the route-handler extra-root rows). An ordinary observer with no modeled sink family still uses `unmodelled` at finding level. A specifically identified unsupported construct can also use `unmodelled` at walk level. Do not assign a reason solely from a negative verdict. If I-1 cannot bind a required candidate to its mechanism, retain the evidence and report the attribution as unexplained.

Wire cost: two additional comparison reason strings, `uncertified_flow` and `matcher_admission`, and one local cut, `unproven_return_input`. Materialise the enum, validator, README vocabulary and serialization checks together in I-1. Existing reason names and meanings remain valid. Add the prerequisite/CURRENT schema and selection rule to the README, and apply this overlay to the four shards in the owning slices.

## New I-walk rows to materialise

These fixtures are additions, separate from the 415 input IDs. Use LF and a final newline. Bind the indicated Def or Use (including its access path), argument and returned component by their exact file bytes. No default knob overrides.

**Static methods.** Both Python rows use this `a.py` prefix and one suffix, with a blank line between. Both are `control`, checked again by I-walk. Truth flows. Assert a single CallDescent from actual 0 to declared formal 0 `cmd` [55,58), then the Use [79,82) on line 6. The grade pair is nameonly / unlabeled. Catalog target is one `structured-os-command` Reached decision on that operand.

```python
import os

class Runner:
    @staticmethod
    def run(cmd):
        os.system(cmd)  # @SINK
```

`d-py-static-class` uses this suffix. Required walk Reached d1, [111,112) → [79,82), site [135,148). The caller is a module-level *function*, so navigation has an enclosing callable for the seed.

```python
def entry():
    t = input()  # @SRC
    Runner.run(t)
```

`d-py-static-instance` uses this suffix inside the class. Required walk Reached d1, [123,124) → [79,82), site [151,162).

```python
    def entry(self):
        t = input()  # @SRC
        self.run(t)
```

**Callee-local negatives.** Materialise `d-return-call-local` and `d-return-comma-local` as independent copies of w05/a.js. Replace only `v + '!'` with `fixed(v)` and `(v, 'fixed')` respectively, and append `function fixed(v) { return 'fixed'; }` on its own line in both. Same source Def [62,63), same receiving variable `out`. Truth no_flow. Require NotReached, none / none, [], no witness. On the **head**, require `unproven_return_input` for the attempted local edge when the owning return is CFG-reachable. The base already reports NotReached on these seeds. A green endpoint alone would miss a broken fix. Existing w05 and `s1-py-return-expression-gain` are the positive local concatenations.

**Caller-seeded controls.** Each ID below is a separate `walk` row. Substitute the expression for `EXPR` in this source. All have the grade pair none / none and [] because `observe` is an ordinary observer. Reached d1 must use the actual `y`, formal `p`, exact selected returned value, and assignment to `out`; a NotReached target has no sink witness. On an existing synthetic edge require the logged local cut `unproven_return_input` when refused. Record this as the missing-flow reason on a feasible refused route, including true aggregate flows. Keep a demonstrated missing producer distinct.

```js
function fixed(z) { return 'fixed'; }
function f(p) {
  const v = p;
  return EXPR;
}
function entry() {
  const y = acquire(); // @SRC
  const out = f(y);
  observe(out); // @SINK
}
```

| Row ID | Expression and file | Truth | Required walk and discriminating assertion |
|---|---|---|---|
| `d-return-id` | `v`, a.js | flows | Reached d1, direct return Use |
| `d-return-binary-zero` | `v & 0`, a.js | no_flow | NotReached, the successful numeric operation yields zero |
| `d-return-call-caller` | `fixed(v)`, a.js | no_flow | NotReached, reject call argument even after entering f |
| `d-return-comma-caller` | `(v, 'fixed')`, a.js | no_flow | NotReached, reject discarded operand even after entering f |
| `d-return-comma-last` | `('fixed', v)`, a.js | flows | Reached d1, Use in the last value only |
| `d-return-array` | `[v]`, a.js | flows | NotReached, aggregate loss |
| `d-return-object` | `({key:v})`, a.js | flows | NotReached, aggregate loss |
| `d-return-condition` | `(v ? "fixed" : "fixed")`, a.js | no_flow | NotReached, test only |
| `d-return-arm` | `(true ? v : "fixed")`, a.js | flows | Reached d1, selected value in the feasible true arm |
| `d-return-template` | `` `text:${v}` ``, a.js | flows | Reached d1, ordinary untagged interpolation of a primitive string |
| `d-return-await` | `await v`, a.js; make both functions f and entry async and use `observe(await out)` | flows | NotReached, no proof of the fulfillment value |
| `d-return-spread` | `[...v]`, a.js | flows | NotReached, iterable elements unproved |
| `d-return-member-base` | `v.cmd`, a.js | flows | NotReached on this graph from the caller root. No Use→synthetic shortcut from the base. |
| `d-return-jsx` | `<span>{v}</span>`, a.tsx | flows | NotReached, no certified scalar component in the returned element |
| `d-return-member-exact` | `v.cmd`, a.js; move the source marker to the return line and select the **full member** Use | flows | Reached d0 from the selected member by direct ReturnFlow. The shorter base Use on that same line is NotReached. |

For the two Python `walk` rows below, substitute `(v, 'fixed')` in `d-return-py-tuple` and `f"text:{v}"` in `d-return-py-fstring`. Both have truth flows, none / none and []. The tuple requires NotReached: its element lacks a scalar slot proof. The f-string requires Reached d1 through the interpolated primitive string. Both were Reached d1 on the caller-seeded base.

```python
def f(p):
    v = p
    return EXPR

def entry():
    y = input()  # @SRC
    out = f(y)
    observe(out)  # @SINK
```

### Additional caller-seeded value-role rows

Each ID is a separate fixture. Substitute its expression in the matching template below. Use the original UTF-8 bytes with LF, including a final LF. The source is the `entry` Def of `y`, the entered site is `f(y)` at actual 0, and the terminal is the `out` Use in `observe(out)`. Bind the selected return Use, the ReturnValue component, the receiving Def of `out` and the matching ascent. For repeated names accept only an occurrence whose entire path through the expression is certified.

```ts
function f(v: string) {
  return EXPR;
}
function entry() {
  const y = acquire(); // @SRC
  const out = f(y);
  observe(out); // @SINK
}
```

Use the same text without `: string` in a.js. Prepend any helper on a separate line. For Python and Go use:

```python
def f(v: str):
    return EXPR

def entry():
    y = input()  # @SRC
    out = f(y)
    observe(out)  # @SINK
```

```go
package main
func f(v string) string {
    return EXPR
}
func entry() {
    y := acquire() // @SRC
    out := f(y)
    observe(out) // @SINK
}
```

For the `fmt` cases insert `import "fmt"` after `package main`. For the Go hook, insert `type fixedStringer struct{ s string }` and `func (fixedStringer) String() string { return "fixed" }` on separate lines before `f` and type its formal `fixedStringer`. For the Python hook prepend `class Fixed:` with method `def __format__(self, spec):` returning `"fixed"`, use `v: Fixed` and initialize `y = Fixed()`.

All rows have group `walk`, default knobs, findings **none / none** on the marked observer and catalog **[]**. Each of the following base queries reached the operand at depth 1 via ReturnInput and ReturnFlow. The expected **I-walk** cells are:

- **P** (propagate, truth flows) requires Reached d1 from the specified Def to Use through this returned component.
- **X** (excluded role, truth no_flow on these fixtures) requires NotReached, null witness fields and the `unproven_return_input` cut on the attempted synthetic input.
- **U** (undecided, truth flows on these fixtures) requires the same NotReached and cut, with the missing proof in the ledger and `unproven_return_input` as the walk loss. Where the missing observer finding differs from truth, apply the observer rule above.

| Row ID | File | Return expression | Expected cells | Discriminating evidence or fixture change |
|---|---|---|---|---|
| `d-return-ts-left` | a.ts | `'ls ' + v` | P | literal prefix |
| `d-return-ts-literal-number` | a.ts | `v + 42` | P | primitive literal beside a string |
| `d-return-ts-repeat` | a.ts | `v + v` | P | pin the actual selected occurrence |
| `d-return-ts-nested` | a.ts | `'ls ' + (v + v)` | P | recurse through both additions |
| `d-return-ts-opaque-sibling` | a.ts | `v + fixed(v)` | U | prepend `function fixed(z: string) { return 'fixed'; }` |
| `d-return-ts-as` | a.ts | `v as string` | P | value child |
| `d-return-ts-nonnull` | a.ts | `v!` | P | postfix non-null |
| `d-return-ts-satisfies` | a.ts | `v satisfies string` | P | value child |
| `d-return-ts-angle` | a.ts | `<string>v` | P | TS grammar |
| `d-return-ts-parens` | a.ts | `((v))` | P | two groupings |
| `d-return-ts-or` | a.ts | `v \|\| ''` | P | truthy string on left |
| `d-return-ts-coalesce` | a.ts | `v ?? ''` | P | non-nullish left |
| `d-return-ts-or-right` | a.ts | `'' \|\| v` | P | evaluated right |
| `d-return-ts-and-right` | a.ts | `true && v` | P | evaluated right |
| `d-return-ts-dead` | a.ts | `'fixed' \|\| v` | X | right operand never evaluated |
| `d-return-ts-else` | a.ts | `false ? 'fixed' : v` | P | false arm |
| `d-return-ts-constant-arm` | a.ts | `v === 'fixed' ? v : 'fixed'` | X | guard fixes even the value in the live arm |
| `d-return-ts-trim` | a.ts | `v.trim()` | P | native receiver |
| `d-return-ts-case` | a.ts | `v.toUpperCase()` | P | native receiver |
| `d-return-ts-slice` | a.ts | `v.slice(1)` | P | a suffix can survive |
| `d-return-ts-empty-slice` | a.ts | `v.slice(0, 0)` | X | no character survives |
| `d-return-ts-unknown-slice` | a.ts | `v.slice(position())` | U | unresolved bound |
| `d-return-js-own-trim` | a.js | `v.trim()` | X | prepend `class Fixed { trim() { return 'fixed'; } }` and initialize `y` with `new Fixed()` |
| `d-return-ts-tag` | a.ts | `` fixedTag`ls ${v}` `` | X | prepend `function fixedTag(strings: TemplateStringsArray, ...xs: unknown[]) { return "fixed"; }` |
| `d-return-py-left` | a.py | `"ls " + v` | P | literal prefix |
| `d-return-py-or` | a.py | `v or ''` | P | truthy string on left |
| `d-return-py-and` | a.py | `True and v` | P | evaluated right |
| `d-return-py-arm` | a.py | `v if True else 'fixed'` | P | value arm |
| `d-return-py-percent` | a.py | `"ls %s" % v` | P | consumed string field |
| `d-return-py-percent-zero` | a.py | `"%.0s" % v` | X | precision discards all characters |
| `d-return-py-percent-width` | a.py | `"ls%4s" % v` | P | fixed minimum width pads, no truncation (reviewer B round 3) |
| `d-return-js-composed-occurrence` | a.js | `("a" + v).slice(0, 1) + v` | P (second read) / X (first read) | two occurrences of `v` (two queries): seeded at the second read (inside `f`, ascending only), Reached with descent_depth 0 and the witness through the second read; seeded at the first read, a cut at the first edge (NotReached, `unproven_return_input`) (reviewer B round 3 smell) |
| `d-return-py-format` | a.py | `"ls {}".format(v)` | P | automatic positional field |
| `d-return-py-format-keyword` | a.py | `"ls {x}".format(x=v)` | P | named field consumes this value |
| `d-return-py-format-unused` | a.py | `"fixed".format(v)` | X | no consuming field |
| `d-return-py-format-spec` | a.py | `"{:.1}".format(v)` | P | positive literal precision retains a prefix |
| `d-return-py-format-precision` | a.py | `"{0:.{1}}".format(v, limit())` | U | unresolved precision can erase the value |
| `d-return-py-fstring-hook` | a.py | `f"{v}"` | X | use the Python `Fixed` setup above |
| `d-return-js-negate` | a.js | `-v` | P | feasible primitive number |
| `d-return-js-complement` | a.js | `~v` | P | numeric complement can vary with the number |
| `d-return-ts-void` | a.ts | `void v` | X | always undefined |
| `d-return-ts-index` | a.ts | `v[0]` | P | nonempty primitive string |
| `d-return-py-index` | a.py | `v[0]` | P | nonempty primitive string |
| `d-return-ts-dynamic-index` | a.ts | `v[position()]` | U | no certified selector |
| `d-return-go-concat` | a.go | `"ls " + v` | P | primitive string concatenation |
| `d-return-go-string` | a.go | `string(v)` | P | predeclared conversion |
| `d-return-go-bytes-string` | a.go | `string(v)` | P | change the formal type to `[]byte` |
| `d-return-go-sprintf` | a.go | `fmt.Sprintf("ls %s", v)` | P | standard `fmt`, bare string directive |
| `d-return-go-sprintf-v` | a.go | `fmt.Sprintf("ls %v", v)` | P | standard `fmt`, `%v` on a plain string (reviewer A R4) |
| `d-return-py-str` | a.py | `str(v)` | P | identity conversion of a primitive string (R4) |
| `d-return-js-String` | a.js | `String(v)` | P | identity conversion of a primitive string (R4) |
| `d-return-py-strip` | a.py | `v.strip()` | P | native trim; also `lower()` / `upper()` (R4) |
| `d-return-js-and-fixed` | a.ts | `v && 'fixed'` | X | the formal is a primitive string (TS `v: string`), where `''` is the only falsy value and carries no data; truth no_flow. An untyped formal would pass `0` or `-0` through, so the fixture must type it (R4 smell, reviewer B round 4) |
| `d-return-go-sprintf-zero` | a.go | `fmt.Sprintf("%.0s", v)` | X | zero-byte output |
| `d-return-go-sprintf-quote` | a.go | `fmt.Sprintf("%q", v)` | P | standard quoting emits the string’s data |
| `d-return-go-sprintf-precision` | a.go | `fmt.Sprintf("%.*s", limit(), v)` | U | the dynamic precision can select zero characters |
| `d-return-go-sprintf-hook` | a.go | `fmt.Sprintf("%s", v)` | X | use the Go `fixedStringer` setup above |

The paired fixture comparison (tally taken before the round-3 and round-4 return-row additions; the table now lists more return rows) covers 50 additional base-accepted witnesses. The design retains 34 true witnesses, rejects 11 false witnesses, and deliberately forgoes 5 true witnesses, one per U row. These are fixture counts against the proposed decisions. The owner’s pre-dispatch comparison in PLAN measures the cost in the frozen repositories, including alternative surviving paths.

### Finding and catalog controls to materialise

| ID or amendment | Fixture and expected cells |
|---|---|
| `d-js-long-continuation-space` (control, I-1) | Copy `s1-js-long-continuation/a.js`. Replace the 24 non-indent characters of each of the 21 `// argument continuation` comments with 24 ASCII spaces, preserving every newline and offset. Truth flows. Walk Reached d0 [64,65) → [708,709), default none (`review_cfg_filter`), trace exact at line 26, catalog one `structured-os-command` Reached there on [708,709). The long commented row keeps none / none and []. |
| `d-catalog-weak-fixed` (catalog, I-8) | Copy `e2-flat-weak/app.js`, prepend `import { exec } from 'child_process';` and insert ` exec('fixed');` after the SQL statement, before its marker. Truth flows. Walk Reached d0 [70,71) → [125,126), exact / exact. Catalog one weak `js-db-call`, LineOnly, Reached on line 4. No command decision. Base review severity on the mixed line is warning, on the SQL-only control suggestion. At the warning floor both line projections keep only the mixed fixture’s line. |
| `e2-target-seed-synthesis` (I-8 API pair) | Keep the shard’s anchor walk NotReached, unlabeled / unlabeled and []. Through the tagged comparison entry select only the anchor, then only the admitted synthesized target for `f(item: Item)`. Expected review/catalog line sets are empty / empty, then {app.py:8} / {app.py:8}. Preserve the graph seed error at app.py:7 in navigation. Neither ordinary review finding belongs to the anchor. |
| `e2-flat-configured` (I-8 API pair) | Keep Reached d0 [26,27) → [65,66), none / none, [] at empty configuration. Repeat on identical inputs with `["=customObserve"]`: require the configured identifier decision (family 11, original pattern ID, Strong, LineOnly/null operand, Reached on app.js:3, absent CWE/category) and the same review finding line. An empty adapter queried again returns []. |

## Known wrong results

The table below records the 70 input `known_wrong` annotations as history. On materialisation set `known_wrong` to null on `tr1-js-default`, `s1-py-default`, `s1-py-cls` and `s3-ts-optional-default-competitor`. Their base walk already matches `truth: flows` and the positive target. For the TS alias, assert the I-guard loss through CURRENT. Keep every truth label. A correction above can narrow a slice's responsibility to what an existing edge actually permits. Preserve the report/issue reference when moving a row into control to record a producer limit or a pinned false witness. A test-only green control is no soundness credit for a known false witness.

| Input annotation | Rows |
|---|---|
| walk | `w01`, `w03`, `w05`, `w10`, `tr1-js-default`, `tr1-py-star`, `tr2-js-discard`, `tr6-py-decorator`, `tr6-go-multireturn-safe`, `a1-go-partial-return-certificate`, `a1-py-nested-use-certificate`, `a1-js-equal-budget-return-sites`, `a1-py-packed-result`, `a1-js-rejoin-after-return`, `a1-py-nondata-keyword`, `a1-js-sanitizer-sibling-def`, `s1-py-cls`, `s1-py-default`, `s1-py-return-expression-gain`, `s2-js-star-both-sides`, `s2-py-framework-bound-other-root` |
| lane 9 slice A | `w06`, `w07` |
| binding | `b04`, `tr1-js-prefix`, `tr3-js-prefix-read`, `s1-js-formal-extension`, `s1-js-formal-write-sibling`, `s2-js-literal-alias-dot` |
| #594 computed-read edge | `b06` |
| constant-return review attribution | `x01` |
| flat review line attribution | `c09` |
| structured review operand attribution | `m01`, `m01b` |
| guard | `tr4-js-nameonly-far3` |
| same_line | `tr5-js-multiline-two-sites`, `s2-go-dispatch-multiline-competitor` |
| catalog | `a1-js-sanitizer-comment-slot`, `a1-js-sanitizer-parenthesized`, `s2-catalog-ts-constructor-operand` |
| spelling | `s1-js-optional-alias` |
| reasons | `s2-py-bound-boundary-far` |
| spelling: base walk, review differs from required behavior | `s3-js-nested-spread-alias-competitor` |
| guard: base walk, review differs from required behavior | `s3-js-mixed-rest-competitor` |
| spelling: base walk, trace_review differs from required behavior | `s3-ts-optional-alias-rest-prefix` |
| same_line: base walk, trace_review differs from required behavior | `s3-ts-optional-default-competitor` |
| walk: base walk, review differs from required behavior | `s3-ts-nested-spread-default`, `s3-js-rest-before-inner-call`, `s3-go-dispatch-variadic-tail`, `s3-go-variadic-fixed-defer-return`, `s3-go-if-init-nested-return` |
| binding: base walk, review differs from required behavior | `s3-tsx-competing-opening-computed` |
| binding: base walk differs from required behavior | `s3-tsx-competing-opening-prefix`, `a2-tsx-callback-local` |
| spelling: base walk, review, trace_review differs from required behavior | `s3-tsx-alias-spread-restored` |
| walk: base walk differs from required behavior | `s3-js-refused-ascent-one-more`, `s3-ts-awaited-refused-ascent-two-more`, `s3-js-root-ascent-refusal-two-more`, `s3-go-named-explicit-ascent-far`, `s3-js-recursive-return-distal` |
| catalog: base trace_review differs from required behavior | `s3-py-escape-in-callee-other-fan` |
| catalog: base review, trace_review differs from required behavior | `s3-py-two-escapes-categories` |
| walk: base walk, trace_review differs from required behavior | `s3-ts-guard-return-destructure`, `s3-ts-await-branch-rejoin` |
| binding: base walk, review, trace_review differs from required behavior | `s3-ts-checked-branch-error-payload` |
| walk: base review differs from required behavior | `s3-py-receiver-keyword-forward-return` |
| same_line: base walk, review, trace_review differs from required behavior | `s3-go-fluent-overwrite-two-sites` |
| spelling: base walk differs from required behavior | `a2-tsx-memo-inline`, `a2-tsx-namespace-wrapper`, `a2-tsx-forwardref-inner-name` |

## Verification receipt

The default command `CARGO_NET_OFFLINE=true cargo test --test algo_taxonomy taint_parity_test::` on this checkout passed four tests, with 440 filtered out. It validates the four input shards and runs engine checks only for the enabled control group. A second baseline run with `PARITY_ENABLED=binding,same_line,spelling,walk,guard,knobs,catalog,reasons` passed row well-formedness and failed the three engine comparisons (1 passed, 3 failed, 440 filtered out). These are the supplied pre-implementation contracts, including producer-conditional proposals. The differing default-review cells informed the amendments above; a missing producer cannot be repaired by a walker change. Neither run consumes this Markdown overlay or executes the future catalog.

Pinned `prism-base` reported `slicing 4.0.0 (9d1a7d06f653)`. No-cache fixture queries supplied the dispatch kinds, l05 source-node inventory, wrapper witness relations, timeout's separate line/operand verdicts and selected producer/boundary controls. Product-slice tests must execute the new byte, reason and catalog assertions. The census, inventory observations and population counts are supplied measurements.

Round-1 fold at `ec242f97`: no-cache `prism-base nav taint-reaches` on isolated Python fixtures showed `self.run(t)` and `r = Runner(); r.run(t)` each crossing from the `t` Use to the `cmd` Def of `run(self, cmd)` and reaching its Use at depth 1. The same base reached the marked operands of d03, s1-py-cls and the three supplied-default controls at depth 1. On `s3-ts-optional-default-competitor` a one-expression control replaced `run('fixed')` with `'fixed'`, keeping `alias?.(t)`: the base walk changed from Reached d1 to BoundaryExited. These probes verify baseline witnesses. The CURRENT transitions and catalog cells above are design requirements for the later slices.

## Review fold probe receipt (A round 2, B round 1)

On the clean input head `13ccf08b`, used the prebuilt `prism-base` reporting `9d1a7d06f653`. Navigation command: `prism-base nav --no-cache taint-reaches --repo DIR --source FILE:LINE --sink FILE:LINE --format json`; inspect the per-source/per-sink occurrences and graph edges. Review command: `prism-base --no-cache --repo DIR --diff DIFF.json --algorithm taint --resolution scoped --format sarif --taint-source FILE:LINE`, with and without `--taint-return-flow`; the temporary diff marks all fixture lines. Grades below are for the marked `taint_sink`. Future cut names, matched-root API results, catalog decisions and optional slice heads are requirements, not outputs of the base binary.

- **Static binding.** Class and instance fixtures both reached d1 with CallDescent [146,147) or [160,161) into formal [55,58), then [79,82). Both finding pairs were nameonly / unlabeled. A literal module-scope seed returned `NoEnclosingCallable`; the class-receiver control uses a module-level function.
- **Returned values.** Replayed all 21 proposed fixture rows from the source specifications above. The two w05 mutations, seeded locally, were NotReached with no findings. From a caller, both false results were Reached d1 over ReturnInput and ReturnFlow. In the `exec(out)` companion both review modes reported unlabeled. Every caller-seeded display, conditional, interpolation, await, spread, JSX and binary-zero control exhibited the synthetic relation; the direct identifier and exact member used ReturnFlow directly. Among 415 input-row navigation queries, 413 returned per-sink data and the two recorded SeedError rows (`e2-js-inline-yaml`, `s1-js-seed-literal`) returned `SymbolNotFound`. Of the emitted witness graphs, only `tr2-js-wrapper` included ReturnInput. These witness graphs do not enumerate every edge in each CPG.
- **Recursion and site ownership.** b04 reached p.a [92,95) via local DataFlow with zero descents; b04b did not reach the earlier read. Both reviews omitted the observer. l02/l03 were BoundaryExited; l01 reached via its second `t`. On the literal mixed fixture both result queries were Reached d1, and both selected actual [79,80). The chain to `a` returned to [53,54) through the other call. Separating the call-start lines permitted both queries without a legacy count collision. The I-walk assertions must still check the full chains.
- **Prerequisites.** The literal alias was BoundaryExited, exact / none. The six named JSX alias/wrapper/colliding positives were NotReached, none / none on the pinned producer. Also probed the nested, computed, prefix, callback-local, attribute, anchor, continuation and third-hop controls. No positive #596 edge was observed on these inputs.
- **Root and reason controls.** On tr5 the same sink Use [109,110) was NotReached from t [64,65) and Reached d0 from u [79,80); line review was exact in both modes. The mixed dispatch was BoundaryExited, nameonly / none; fixing its uncertain actual removed the finding. The configured and masked matcher inputs were Reached d0, none / none; unmasking the Go sink by moving it one line gave exact / exact. The actual configured-positive API run belongs to I-8. No real-code Catalog, Recovered or Sound gate was run for this documentation fold.

## Review fold probe receipt (A round 3, B round 2)

Input head `4c022e27`, branch `docs/lane8-stage-d`. Used the same no-cache navigation and all-lines review commands as the preceding receipt with `prism-base` at `9d1a7d06f653`. Counted source/terminal bytes, witness relations and the marked SARIF finding in each mode.

- **Return contribution.** Recreated all 50 additional return fixtures from the Markdown templates and reran the three amended positive rows. All 53 reached d1 through ReturnInput and ReturnFlow with no observer findings. A’s eight are among these. From a caller, both `fixed(v)` and the non-last comma operand also reached d1 on base. The value-role proof must distinguish those false witnesses. The original four-shard replay produced 413 navigation answers and the two recorded seed errors. Only `tr2-js-wrapper` displayed a ReturnInput in that replay. The six archived `frontier/relax-none.json` results contain no witness graph, and the descent logs record refusals. They cannot supply counts of all accepted synthetic inputs.
- **Root identity.** On `e2-target-seed-synthesis`, the graph anchor [212,213) reached neither the base nor the full member at line 8. Both review modes reported unlabeled. The signature query at line 7 returned `SymbolNotFound`. Removing only the framework decorator removed both findings. Entry 31’s producer builds a virtual Def from the admitted seed. The tagged singleton comparisons in I-8 will test it directly.
- **Line strength.** On `e2-flat-weak` and the fixed-command variant, both review modes reported exact. Trace review with `--format review --review-min-severity suggestion` retained both lines. The warning floor retained only the mixed line at app.js:4. As a mask control, `observe(t); exec('fixed');` had a reached t with neither finding. Changing the command actual to t produced exact / exact. I-8 must verify the weak decision and its distinct projected severity.
- **Reason attribution.** The 487-entry audit is itemised by shard and level above. On x01, fixing only the actual to forget eliminated the default finding. Splitting self-return across two functions changed the boundary into a ReturnFlow witness and gave none / unlabeled. On the long and short continuations the leading comment suppressed both findings. Equal-width whitespace gave none / exact on the long form and exact / exact on the short form. The original comment at [99,123) occupies named child zero, ahead of t at [708,709). Default review needs both a CFG-admitted row and an admitted argument.
- **Configured sinks.** On `e2-flat-configured`, t [26,27) reached [65,66), with neither review finding under empty configuration. Changing the sink spelling to `document.write` produced exact / exact. The base CLI exposes no extra-sink input. Only the I-8 API test can verify the positive configured decision while holding the spelling, graph and fan fixed.

Materialise the 50 return rows and amend the three earlier return targets at I-walk. Add the whitespace control and the three per-level reason corrections at I-1. Add the mixed-strength row and paired synthesized/configured API assertions at I-8. Document the extended `matcher_admission` evidence and the two catalog strength meanings in the fixture README when the harness consumes them. This Markdown fold changes none of the four JSONL inputs.

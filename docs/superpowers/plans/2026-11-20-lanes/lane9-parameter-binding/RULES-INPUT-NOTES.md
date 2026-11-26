# Matrix dispositions and census costs

This is the row ledger for [DESIGN §5](DESIGN.md). The design selects the required product behaviour. The supplied matrix preserves the original assertions, languages, span endpoints, multiplicity and truth. Where the model differs from the language, the reader needs both answers.

## Source receipts

| Artifact | Provenance | SHA-256 |
|---|---|---|
| `tests/fixtures/param_binding_matrix/rows.jsonl` | Reviewed worktree matrix `5767ceb84cbe77fc712f7dafa34a342021520dcb`, 425 distinct cases | `500284f1518dc0112b24466bff2fc108ad85faa8f90163305fe9b0fdea57b73d` |
| Matrix before review fixes | Integrated rows at `3975d8b0170c6d2c3c7283c84ae5053f1f16f5e1`, also 425 IDs | `da4d497a92b2be0d0ff117fe29d28f2305f62d328a2142a4b8ad8d21e1048ddd` |
| `matrix/RED-RECORD-new.md` | Fixture checkout `d8a9a67c1ef3490806c43852a4fc01a687ec1bc4`, parent `90353d1b1e55b4faa942f7908809dd75a9d00990` | `75b0749cd5230ccbac16d30b4ecdffd273b7fa03f917e231050e93eb4fe08aa1` |
| Original Stage M `CENSUS.md` (before the round 2 numerical note) | Census build `06157400a3be`, product-equivalent to `9d1a7d06f` | `4f2e0bbf42cb7c9bb42f58ee91034890df9ae4015a898f32e0f01db55738af5d` |
| `m3/T/graded-details.jsonl` | 68 probe rows | `0762ff4dcd025884ed5c9c0ebc107ac06107463791082eca2f300aeef3fe94e9` |
| `m3/SB/graded/verdicts.jsonl` | 127 probe rows, package-qualified row endpoints | `0212345fd48f65e7cb0279ab9d8b2c924446ee5cd2e140b419e53ebeeb699910` |
| `argfollow-PLAN.md` | #434 §3–5, research at `b593ad18` | `f9d9222535606a8e702bb5a4bc679ca24dbc153cde23e65fdb119ccfa12dc3db` |

The RED record reports **317 RED, 98 already correct and 10 unexecuted**. The 288 enumerated rows contributed 239 RED, 49 already correct and zero unexecuted. All nine controls passed. The ten matrix rows labelled `wrong` pin or exercise the known unimported-export defect. A passing pin gives no correctness credit to that resolution. Every tier stopped at round five. The per-row entries for D and E below list all 53 and 136 enumerated members of those two open classes.

## Reading a row

**Keep** requires every input assertion, including negative edges with their original label filters. The short description may omit controls or coordinates. **Set** gives an additional requirement or a replacement for the specified part. **Loss** records a known missing or excess result, the fixture shape and its cause. Keep all independent assertions and record the runtime truth even if a product assertion changes. Some negative controls already pass while a real transfer falls outside the model. Their Loss entry identifies both the passing assertion and the omitted transfer. **External** identifies a fixture checked by a separate census, oracle or witness ledger. **Sentinel** retains a hard correctness assertion that frozen rules or upstream resolution may fail. Compare the exact base and head. Never relabel a newly introduced checker-WRONG Exact as an accepted loss.

The input truth comes from the reviewed matrix. The base outcome is historical. Review commit `5767ceb8` marked 13 previously executed cases as model boundaries, removed one unsatisfiable reachability assertion apiece from x1-6 and x3-5, and fixed the matrix adapter's `call_nameonly` spelling and static vocabulary check. It did not issue the RED record. The current file has **23** rows carrying `unexecuted` (ten original and 13 reviewed skips). Preserve the negative witnesses and record missing/extra product arms without reporting skipped checks as passes. `Pass` below means “already correct” in the supplied RED record. A changed expectation needs a fresh same-base run before its feature group is enabled. Mixed D/E assertions run at their first applicable slice and together on the integrated head. A missing endpoint or fixture label-adapter mismatch is evidence about the fixture only.

## Controls

9 rows. 0 enumerated.

| Row | Shape | Input truth | Base | §5 rule | Required disposition |
|---|---|---|---|---|---|
| `c01` | fixed slot | correct | Pass | Controls | **Keep.** Bind main.v → f.a (exact). check Def. |
| `c02` | parenthesized arrow | correct | Pass | Controls | **Keep.** Bind main.v → g.x (exact). check Def. |
| `c03a` | go spread | correct | Pass | Controls | **Keep.** 1 prohibited edge. |
| `c03b` | python splat | correct | Pass | Controls | **Keep.** Bind 2 specified edges, including main.xs → f.a. |
| `c04` | literal after spread | correct | Pass | Controls | **Keep.** 1 prohibited edge. |
| `c05` | builtin drop | correct | Pass | Controls | **Keep.** Keep UnknownName drop. |
| `c06` | arguments | correct | Pass | Controls | **Keep.** 1 prohibited edge. |
| `c07` | unimported export | wrong | Pass | Admission | **Keep.** Resolve parseInt to parseInt@1 and check the other site set. bind 2 specified edges, including main.v → parseInt.vals (exact). |
| `c08` | unimported export | wrong | Pass | Admission | **Keep.** Resolve setTimeout to setTimeout@1. |

## A · Spreads and call identity

69 rows. 40 enumerated.

| Row | Shape | Input truth | Base | §5 rule | Required disposition |
|---|---|---|---|---|---|
| `a01` | unknown offset | correct | RED | A offsets | **Keep.** 1 prohibited edge. check Def. |
| `a02a` | spread source | correct | Pass | A offsets | **Keep.** 1 prohibited edge. |
| `a02b` | known prefix | correct | Pass | A offsets | **Keep.** Bind main.v → f.a (exact). 1 prohibited edge. |
| `a03a` | unknown offset | correct | RED | A offsets | **Keep.** Bind main.v → f.a (exact). 1 prohibited edge. |
| `a03b` | unknown offset | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `a03c` | unknown offset | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `a03d` | unknown offset | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `a04a` | constructor | correct | Pass | A offsets | **Keep.** 1 prohibited edge. |
| `a04b` | receiver | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `a04c` | optional | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `a05a` | chain identity | correct | RED | A identity | **Keep.** 5 prohibited edges. |
| `a05b` | chain identity | correct | RED | A identity | **Keep.** 5 prohibited edges. |
| `a05c` | chain identity | correct | RED | A identity | **Keep.** Bind main.x → m.a1 (exact). 4 prohibited edges. |
| `a05d` | call result | correct | Pass | A identity | **Keep.** Bind main.x → cb.p (exact). 1 prohibited edge. |
| `a05e` | constructor chain | correct | Pass | A identity | **Keep.** Bind main.w → m.q. 2 prohibited edges. |
| `a06a` | literal length | correct | Pass | A offsets | **Keep.** Bind main.v → f.b (exact). 2 prohibited edges. |
| `a06b` | literal length | correct | RED | A offsets | **Keep.** Bind main.v → f.a (exact). 2 prohibited edges. |
| `a06c` | literal length | correct | RED | A offsets | **Keep.** Bind main.v → f.c (exact). 2 prohibited edges. |
| `a06d` | literal length | correct | Pass | A offsets | **Keep.** Bind 2 specified edges, including main.v → f.a (exact). 4 prohibited edges. |
| `a06e` | unknown offset | correct | RED | A offsets | **Loss.** Keep the fixture negatives. String, tuple-typed, holey and directly spread-containing operands use the unknown-length rule even when this example has a recoverable length. |
| `a06f` | unknown offset | correct | RED | A offsets | **Loss.** Keep the fixture negatives. String, tuple-typed, holey and directly spread-containing operands use the unknown-length rule even when this example has a recoverable length. |
| `a06g` | literal length | correct | Pass | A offsets | **Keep.** Bind main.v → opt.b (exact). 1 prohibited edge. |
| `a06h` | overload | correct | Pass | A offsets | **Keep.** Bind main.v → f.b (exact). 1 prohibited edge. |
| `a07a` | two rows one site | correct | RED | A offsets | **Keep.** 3 prohibited edges. |
| `a07b` | two rows one site | correct | Unexecuted | A offsets | **External.** Keep the M1 fixture counter at one site and two pre-change rows. Run the enumerator on this fixture. The corpus has zero post-spread rows. |
| `a08a` | excluded call apply | correct | Pass | A offsets | **Loss.** Keep the UnknownName drop and prohibition on the argument edge. Builtin Function.call delivers v to f.b here. A keeps that adapter outside positional binding. |
| `a08b` | excluded call apply | correct | Pass | A offsets | **Loss.** Keep the UnknownName drop and prohibition on the argument edge. Builtin Function.apply delivers the second array element v to f.b. The ordinary call argument list contains only the array. |
| `a09a` | refusal reachability | correct | RED | Consumers | **Keep.** Bind 2 specified edges, including origin.x → mid.unused (exact). 1 prohibited edge. check reachability, unreachability. |
| `a09b` | refusal reachability | correct | Unexecuted | Consumers | **External.** Check the classified reachability change in the external refusal ledger with the seed and both witness paths. Keep the companion CPG assertions. |
| `t1-1` | multiple literal spreads | correct | Pass | A offsets | **Keep.** Bind main.v → f.c (exact). 1 prohibited edge. |
| `t1-2` | literal trailing comma | correct | Pass | A offsets | **Keep.** Bind main.v → f.b (exact). 1 prohibited edge. |
| `t1-3` | string spread boundary | correct | RED | A offsets | **Loss.** Keep the fixture negatives. String, tuple-typed, holey and directly spread-containing operands use the unknown-length rule even when this example has a recoverable length. |
| `t1-4` | nested known spread boundary | correct | RED | A offsets | **Loss.** Keep the fixture negatives. String, tuple-typed, holey and directly spread-containing operands use the unknown-length rule even when this example has a recoverable length. |
| `t2-1` | unknown then empty spread | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `t2-2` | known spread past arity | correct | RED | A offsets | **Keep.** 1 prohibited edge. |
| `t2-3` | typed tuple spread boundary | correct | RED | A offsets | **Loss.** Keep the fixture negatives. String, tuple-typed, holey and directly spread-containing operands use the unknown-length rule even when this example has a recoverable length. |
| `t3-1` | literal spread element boundary | ambiguous | Pass | A offsets | **Loss.** Keep the exact edge from the following w to b. A omits the real fixed-slot transfer from v inside the dense spread to a. |
| `t4-1` | tagged template implicit first argument | correct | Pass | A offsets | **Keep.** 1 prohibited edge. |
| `t4-2` | satisfies wrapped literal spread | correct | Pass | A offsets | **Keep.** Bind main.v → f.b (exact). 1 prohibited edge. |
| `t5-1` | overridden array iterator | correct | RED | A offsets | **Loss.** Runtime v enters c because the patched iterator yields twice. Frozen A assigns the literal length one. Preserve the runtime counterexample as a model-boundary skip. No corpus credit or exception to the Exact oracle gate. The reviewed matrix skips this row. |
| `s1-1` | literal leaf after empty spread | correct | RED | A offsets | **Loss.** #1 only, v → a.key. Lane 3 refuses a literal after any spread, including this empty one. Keep #2 (no edge to b.key). |
| `s1-2` | discarded array in sequence spread | correct | RED | A offsets | **Keep.** Bind main.first → f.b (exact). 1 prohibited edge. |
| `s1-3` | optional member call chain identity | correct | RED | A identity | **Keep.** Bind main.x → m.a1 (exact). 1 prohibited edge. |
| `s1-4` | same line distinct spread offsets | correct | RED | A identity | **Keep.** Bind 2 specified edges, including main.x → f.a (exact). 2 prohibited edges. |
| `s1-5` | spread inside array valued element | correct | Pass | A offsets | **Keep.** Bind main.v → f.b (exact). 1 prohibited edge. |
| `s2-1` | constructor and method share callee spelling | correct | RED | A identity | **Loss.** #1 only, inside → constructor.first. The call query emits the outer method site only. Require outside → C.second and both crossed negatives. |
| `s2-2` | known spread preserves base and projection | correct | RED | A offsets | **Keep.** Bind 2 specified edges, including main.box → record.third (exact). 4 prohibited edges. |
| `s2-3` | surplus after spread cannot certify return flow | correct | RED | Consumers | **Keep.** 1 prohibited edge. check unreachability, counter. |
| `s3-1` | type assertion cannot shorten runtime array | correct | RED | A offsets | **Keep.** Bind main.value → record.third (exact). 1 prohibited edge. |
| `s3-2` | equal call spans and callee names in different files | correct | RED | A identity | **Keep.** Bind 2 specified edges, including main.value → f.first (exact). 4 prohibited edges. |
| `s4-1` | nested call spread does not change the enclosing argument offset | correct | RED | A identity | **Keep.** Bind main.outside → outer.third (exact). 2 prohibited edges. |
| `s4-2` | spread repositioning leaves an independent refusal path | correct | RED | Consumers | **Keep.** Bind 2 specified edges, including origin.value → mid.first (exact). 1 prohibited edge. check reachability, unreachability. |
| `s5-1` | same receiver chain binds two offsets to one method | correct | RED | A identity | **Set.** Change #3 right → step.second to nameonly/call_nameonly, multiplicity one (outer r6_single_owner). Keep #1 inner Exact and both wrong-slot negatives. Prohibit Exact on the outer edge. |
| `s5-2` | equal length conditional spread literal boundary | ambiguous | RED | A offsets | **Loss.** No fixed edge from value. Both alternatives have length one at runtime, but the spread operand is a conditional expression. |
| `s5-3` | object leaf after spread intervening positional argument | correct | RED | A offsets | **Loss.** #1 only, value → fourth.item. Lane 3 stops leaf admission at the earlier spread. Keep #2–3 sibling negatives. |
| `s5-4` | user defined call method keeps runtime spread offsets | correct | RED | A offsets | **Keep.** Bind main.value → call.first (exact). 1 prohibited edge. |
| `x1-1` | comment only array versus an elision | correct | RED | A offsets | **Keep.** Bind 2 specified edges, including main.empty → f.first (exact). 2 prohibited edges. |
| `x1-2` | noniterable spread prevents delivery of the evaluated prefix | correct | RED | A offsets | **Loss.** The throw during ...null prevents delivery of lost. A computes slots without proving execution. Keep the reviewed dead-call boundary and preserve the ordinary kept arm as a control. The reviewed matrix skips this row. |
| `x1-3` | constructor expression call with no new argument list | correct | RED | A identity | **Loss.** #3 only, direct → constructor.item. The graph has the nested factory call but no constructor site for the positive new Product(direct) control. Require the Exact inner → factory.key edge and its constructor negative. |
| `x1-4` | spread moves a fixed slot at imported and global calls | correct | RED | Admission | **Sentinel.** All four assertions stay enabled and mandatory. The imported call binds admitted → text. The unrelated export wins foreign free_single and A moves the existing fixed binding to text, violating #3. Keep both foreign negatives and the imported wrong-slot negative. Report hard RED with c07/c08. |
| `x2-1` | sequence expression contributes only its final value | correct | RED | A offsets | **Loss.** #1 only, kept → take.first. Step 5b has no argument occurrence for the value of the sequence expression. Keep last → second and both negatives. |
| `x2-2` | super lookup and virtual dispatch on the same receiver | correct | RED | A identity | **Sentinel.** All four assertions stay enabled and mandatory. Both step sites have only r6_multi_owner_candidate targets, excluded from Step 5b. The two required dispatch edges are missing. Keep both crossed negatives and report hard RED. |
| `x3-1` | optional chain short circuit skips a nested spread call | correct | RED | A offsets | **Loss.** The optional chain skips the nested call, while grouping evaluates its argument. A computes slots without a short-circuit execution proof. Keep the reviewed dead-call boundary. The reviewed matrix skips this row. |
| `x3-2` | repositioned member argument keeps name only confidence | correct | RED | A offsets | **Keep.** Bind main.value → accept.third (nameonly/call_nameonly). 2 prohibited edges. |
| `x3-3` | same line shifted calls certify return flow independently | correct | RED | Consumers | **Keep.** Bind main.value → pick.item (exact). 1 prohibited edge. check counter. |
| `x4-1` | callee is read before a spread element reassigns it | correct | RED | A identity | **Loss.** Reading the callee precedes evaluating a spread that reassigns it. A cannot supply this temporal target proof. Preserve the reviewed evaluation-order boundary and both runtime target identities. The reviewed matrix skips this row. |
| `x4-2` | a bound use cannot certify a surplus use of the same variable | correct | RED | Consumers | **Keep.** Bind main.value → pick.item (exact). 1 prohibited edge. check counter. |
| `x5-1` | same name chain with equal ending columns on different lines | correct | RED | A identity | **Set.** Change #2 outer → step.second to nameonly/call_nameonly, multiplicity one (outer r6_single_owner). Keep the inner Exact and both wrong-slot negatives. Prohibit Exact on the outer edge. |
| `x5-2` | constructor return object selects the outer method | correct | RED | A identity | **Sentinel.** All five assertions stay enabled and mandatory. The constructor site is missing and both outer take targets are navigation-only multi-owner candidates. Keep the two positives, the decoy and both cross-site prohibitions. Report hard RED. |

## B · Rest parameters

54 rows. 33 enumerated.

| Row | Shape | Input truth | Base | §5 rule | Required disposition |
|---|---|---|---|---|---|
| `b01a` | rest | correct | RED | B rest | **Keep.** Bind main.v → f.args (exact). check Def. |
| `b01b` | arrow rest | correct | RED | B rest | **Keep.** Bind main.v → f.args (exact). |
| `b01c` | method rest | correct | RED | B rest | **Keep.** Bind main.v → f.args (exact). |
| `b01d` | rest | correct | RED | B rest | **Keep.** Bind main.v → f.args (exact). check Def. |
| `b02a` | rest | correct | RED | B rest | **Keep.** Bind 3 specified edges, including main.a → f.p (exact). 2 prohibited edges. |
| `b02b` | rest | correct | RED | B rest | **Set.** #1 withhold xs → rest. The operand starts at rest but the formal xs supplies no yield witness. |
| `b02c` | rest | correct | RED | B rest | **Set.** #2 withhold xs → rest. Keep #1 a → p and #3 no xs → p. |
| `b03` | ambiguous offset | ambiguous | Pass | B rest | **Loss.** For both xs and v require no fixed or rest edge. With an empty xs, v fills p. A longer xs can place v in rest. |
| `b04` | ts this | correct | RED | B rest | **Keep.** Bind 2 specified edges, including main.x → f.a (exact). |
| `b05a` | rest pattern | correct | Pass | B rest | **Loss.** Keep the negatives. A rest pattern exposes component bindings, not the single identifier rest Def to which B delivers elements. |
| `b05b` | rebound rest | correct | Pass | B rest | **Keep.** 1 prohibited edge. |
| `b06` | same-file NameOnly method | correct | RED | B rest | **Loss #2.** Preserve #1, the NameOnly `r6_single_owner` resolution to `f`. B withholds the same-file binding. Require no edge of any label from `main.v` to `f.rest`. |
| `b07` | rest object literal | correct | Pass | B rest | **Loss.** Keep q → q and no x → rest. Supplied object-literal fields are not the object element itself. B does not flatten object fields into rest. |
| `b08` | refusal reachability | correct | Pass | Consumers | **Keep.** Bind f.rest → freeSink.p (exact). check Def, unreachability. |
| `b09a` | least offset | ambiguous | RED | B rest | **Loss.** Require w → r, no v → p or r and no xs → r. The lower bounds of v and w are zero and one respectively. |
| `b09b` | literal length | correct | RED | B rest | **Keep.** Bind main.v → g.p (exact). 1 prohibited edge. |
| `b09c` | literal length | correct | RED | B rest | **Keep.** Bind main.v → g.r (exact). 1 prohibited edge. |
| `b10a` | no variable source | correct | Pass | B rest | **Keep.** 1 prohibited edge. |
| `b10b` | no variable source | correct | Pass | B rest | **Keep.** 1 prohibited edge. |
| `b10c` | no variable source | correct | Unexecuted | B rest | **External.** The external M2 fixture census must report zero sites. The CPG companion row prohibits an incoming variable edge. |
| `b10d` | no variable source | correct | Unexecuted | B rest | **External.** The external M2 fixture census must report zero sites. The CPG companion row prohibits an incoming variable edge. |
| `t1-5` | default before rest | correct | RED | B rest | **Keep.** Bind 2 specified edges, including main.v → f.rest (exact). 1 prohibited edge. |
| `t2-4` | generator rest | correct | RED | B rest | **Keep.** Bind main.v → collect.items (exact). |
| `t3-2` | unknown spread two fixed slots | correct | RED | B rest | **Loss #1.** The ledger required main.v → f.rest (exact), which holds only when xs yields at least one element. The least offset of v is 1 and rest starts at 2, so DESIGN §5 B supplies no binding. Prohibitions #2–3 stay mandatory. |
| `t4-3` | constructor rest | correct | RED | B rest | **Loss.** #1 x → first is the recorded fixed loss. #2 y → rest is blocked/RED without the constructor site. Require #3 no y → first. Exercise the supplied-site control below. |
| `s1-6` | known spread raises unknown offset floor | correct | RED | B rest | **Keep.** Bind main.value → pack.tail (exact). 2 prohibited edges. |
| `s1-7` | destructured fixed formal consumes one slot | correct | RED | B rest | **Loss.** Mark #1 a → fixed only. The fixed builder stops at the preceding destructure. Require b → tail and both crossed negatives. |
| `s1-8` | return flow certification with rest inputs | correct | RED | Consumers | **Keep.** Check counter, reachability. |
| `s2-4` | parameter property and tuple rest | correct | RED | B rest | **Loss.** #1 label → tag also lacks a parameter-property Def. #2–3 count/text → parts are blocked/RED without the constructor site. Require #4 no label → parts, including with a supplied site. |
| `s2-5` | shadowed rest write keeps entry binding | correct | RED | B rest | **Keep.** The round 1 fold restores the outer entry Def. Require #1 main.value → collect.items (exact) and #2 no main.value → reset.items. |
| `s2-6` | member spread begins at rest after literal prefix | correct | RED | B rest | **Set.** #1 withhold box.items → tail. Matching member bytes and a known start do not prove an element. Keep both fixed-slot prohibitions. |
| `s3-3` | super forwarding cannot bind back to derived constructor | correct | RED | B rest | **Set.** #1 last → Derived.constructor.items is blocked/RED without that constructor site. Require it with a supplied site. Keep all three prohibitions (#2–4). |
| `s3-4` | independent unknown spreads start in the rest range | correct | RED | B rest | **Set.** Withhold #2 left → tail and #4 right → tail. Neither opaque operand has a yield witness. Keep lead → first, last → tail and all three crossed negatives. |
| `s3-5` | distinct uses of the same value feed one rest def | correct | RED | B rest | **Keep.** Bind 2 specified edges, including main.value → collect.parts (exact). |
| `s3-6` | indexed mutation preserves the rest binding | correct | RED | B rest | **Keep.** Bind main.value → collect.parts (exact). check Def. |
| `s4-3` | fixed overload signatures with a rest implementation | correct | RED | B rest | **Keep.** Bind 3 specified edges, including main.value → pack.items (exact). 2 prohibited edges. check Def. |
| `s4-4` | spread straddling the fixed slot versus one wholly in rest | correct | RED | B rest | **Loss.** Keep #1–2 prohibiting left → tail/first for the straddle. For #3 withhold right → tail despite its trailing start because yield is unproved. Keep #4 no right → first. |
| `s4-5` | object rest pattern index and synthesized length | correct | Pass | B rest | **Loss.** Keep the negatives. A rest pattern exposes component bindings, not the single identifier rest Def to which B delivers elements. |
| `s5-5` | least offset reaches rest at a same-file NameOnly call | correct | RED | B rest | **Loss of original #1, #3, #4.** The offset reaches rest, but B withholds the same-file edge. Require no `main.value → pack.tail` edge at #1 and keep #2's Exact prohibition. At #3 require that the value Def cannot reach the tail read. The prerequisites module's `s5-5-foreign` exercises the positive edge, trace boundary and reachability. |
| `s5-6` | implicit derived constructor forwards into base rest | ambiguous | RED | B rest | **Loss.** #2 value → Base.tail is blocked/RED unless the extracted new Derived site identifies Base’s implementation. Exercise that binding with the supplied target, keep #1’s Def and check a wrong-target control. No synthetic forwarding. |
| `s5-7` | dense literal spread element enters rest after fixed slot | ambiguous | RED | B rest | **Set.** Bind the actual element value → tail once, exact. The preceding 0 occupies the fixed slot. |
| `s5-8` | writing unmapped arguments preserves entry rest binding | correct | RED | B rest | **Keep.** Bind main.value → pack.items (exact). check Def. |
| `x1-5` | one dense spread straddles the fixed and rest slots | ambiguous | RED | B rest | **Loss.** Bind extra and last to tail. No fixed edge from first inside the spread under A. Keep the prohibitions on crossing the head/rest boundary. |
| `x1-6` | rest length is independent of element value | correct | RED | Consumers | **Loss.** Keep value → items and count-Def → count-Use. Runtime length is independent of the element value. The coarse graph can still reach count via items. The review removed that unsatisfiable nonreachability requirement. |
| `x1-7` | adjacent tag substitutions both enter rest | correct | RED | B rest | **Loss.** Tagged substitutions reach rest at runtime after the template object. An ordinary argument list supplies no such operands. Keep the negative on strings and record both missing transfers. |
| `x2-3` | lexical rest parameter named arguments | correct | RED | B rest | **Keep.** Bind main.value → pack.arguments (exact). 1 prohibited edge. check Def. |
| `x2-4` | default reads the rest binding before initialization | correct | RED | B rest | **Loss.** At runtime the default reads uninitialised tail before the first body entry. Only kept enters a live invocation. Preserve the reviewed dead-call boundary. The ordinary rest rule supplies no exceptional-entry proof. The reviewed matrix skips this row. |
| `x2-5` | guaranteed elements inside an unknown length array spread | ambiguous | RED | B rest | **Loss.** No edge from after or uncertain. The top-level nested spread contributes a floor of zero, even though the two literal elements in the first operand guarantee rest membership at runtime. |
| `x3-4` | fixed slot after first default precedes rest despite function length zero | correct | RED | B rest | **Keep.** Bind 2 specified edges, including main.fixed → pack.middle (exact). 2 prohibited edges. |
| `x3-5` | default derived constructor forwards without iterating a rest array | correct | RED | B rest | **Loss.** #1 lost → Explicit.forward and #2 kept reaching Base.parts require their respective admitted constructor sites. Both are blocked/RED when missing. Supply each site separately and keep the two arms distinct. The review removed the negative walk through the explicitly spread array with a patched iterator. |
| `x4-3` | shifted fixed argument feeds a default while a rest argument bypasses it | correct | RED | Consumers | **Loss #5.** Reads of earlier formals inside parameter default initializers have no Use node (#916), outside slice B. Both exact edges (#1–2), both prohibitions (#3–4) and the nonreachability check (#6) stay mandatory. |
| `x4-4` | twice bound prefixes fill fixed and rest slots in binding order | correct | RED | B rest | **Loss.** Neither bound prefix nor the final payload has a proved adapter offset. No new head or tail transfer through the two bind results. The reviewed matrix skips this row. |
| `x5-3` | reflect construct new target supplies only the prototype | correct | RED | B rest | **Loss.** Preserve the reviewed whole-row skip for Reflect.construct. Its ordinary #3 direct → Shape.other independently needs a constructor site. Record the missing prerequisite, exercise #3 with a supplied site in a companion and keep its decoy negatives. The skipped reflection arm earns no pass. |
| `x5-4` | rest collection and fixed slot forwarding certify separately | correct | RED | Consumers | **Keep.** Bind main.value → relay.items (exact). 1 prohibited edge. check counter. |

### Ordinary B NameOnly rest edges

The independent `binding` verdicts for head `bd9c0945` give the following cost when the §5 B file comparison is applied to B-only graded `nameonly/call_nameonly` additions. The figures subtract 23 inherited foreign slice A edges from SB. Compare `row.from.file` and `row.to.file` with the SB package qualifier intact. Counts are edges. C, W and U mean CORRECT, WRONG and UNDECIDED. [X verdicts](/Users/wesleyjinks/prism-evidence/param-binding/sliceB/X/graded/verdicts.jsonl) · [T verdicts](/Users/wesleyjinks/prism-evidence/param-binding/sliceB/T/graded/verdicts.jsonl) · [SB verdicts](/Users/wesleyjinks/prism-evidence/param-binding/sliceB/SB/graded/verdicts.jsonl)

| Corpus | Withheld same-file | C / W / U | Kept foreign | C / W / U |
|---|---:|---|---:|---|
| X | 4 | 0 / 4 / 0 | 109 | 82 / 0 / 27 |
| T | 29 | 16 / 13 / 0 | 18 | 18 / 0 / 0 |
| SB | 36 | 0 / 3 / 33 | 177 | 1 / 0 / 176 |
| **Total** | **69** | **16 / 20 / 33** | **304** | **101 / 0 / 203** |

The unfiltered head failed the pooled gate at 20/373 (5.36%), with 20/137 WRONG among decided rows. All 20 errors matched a receiver method to an unrelated same-file declaration. Applying the file comparison to those verdicts gives **0/304** on the gate and **0/101** among decided rows. The 203 retained UNDECIDED rows earn zero credit. The 793 B-only Exact additions comprise 789 CORRECT, 0 WRONG and 4 UNDECIDED. These counts price the design on the recorded differential. Regrade the implementation head for activation. [Differential ledger](/Users/wesleyjinks/prism-evidence/param-binding/LEDGER.md)

Before this amendment, b06 and s5-5 were the only ordinary B same-file NameOnly rest positives across the 425 matrix IDs. Both now prohibit those edges. Three E rows require same-file `nameonly/call_nameonly` rest edges from independently derived `argument_follow` targets. Keep t4-9 #2, s3-23 #3 and x3-11 #4–5. D's s1-17 #2 also delivers into same-file rest, under `nameonly/builtin_callback`. Keep its role edge. Each producer retains its ledger disposition.

## C · Bare parameters

40 rows. 26 enumerated.

| Row | Shape | Input truth | Base | §5 rule | Required disposition |
|---|---|---|---|---|---|
| `k01a` | bare arrow | correct | RED | C bare | **Keep.** Bind main.v → g.x (exact). check Def. |
| `k01b` | bare arrow | correct | RED | C bare | **Keep.** Bind main.v → g.x (exact). check Def. |
| `k02` | inferred name | correct | Pass | Admission | **Keep.** 1 prohibited edge. |
| `k03` | inferred name | correct | Pass | Admission | **Keep.** 1 prohibited edge. |
| `k04` | inferred name | correct | Pass | Admission | **Keep.** 1 prohibited edge. |
| `k05a` | literal leaf | correct | RED | C bare | **Keep.** Bind main.x → f.p.a (exact). |
| `k05b` | rebound formal | correct | Pass | C bare | **Keep.** 1 prohibited edge. |
| `k06a` | cross file free single | wrong | Pass | Admission | **Set.** Stage #2 at B (no edge into vals). Keep #1 at C (parseInt resolves to parseInt@1). |
| `k06b` | cross file free single | wrong | Pass | Admission | **Keep.** Resolve escape to escape@4. 2 prohibited edges. |
| `k06c` | cross file free single | wrong | Pass | Admission | **Keep.** Resolve escape to escape@4. 2 prohibited edges. |
| `k07a` | imported | correct | RED | Admission | **Set.** Stage #1 v → parseInt.vals at B and #2 v → escape.s at C. Require both Exact. |
| `k07b` | same file | correct | RED | Admission | **Set.** Stage #1 v → parseInt.vals at B and #2 v → escape.s at C. Require both Exact. |
| `k08a` | return flow | correct | RED | Consumers | **Keep.** Check counter. |
| `k08b` | return flow | correct | Pass | Consumers | **Keep.** Check counter. |
| `t4-4` | bare arrow jsx body | correct | RED | C bare | **Keep.** Bind main.value → render.item (exact). check Def. |
| `s1-9` | same file lexical shadowing | correct | RED | C bare | **Keep.** Bind main.input → id.inner (exact). 1 prohibited edge. |
| `s1-10` | renamed reexport with local import alias | correct | RED | C bare | **Keep.** Bind main.value → project.item (exact). |
| `s1-11` | member assignment result is a lexical binding | correct | RED | C bare | **Keep.** Bind main.value → project.item (exact). |
| `s2-7` | same file property name is not a free binding | correct | Pass | Admission | **Keep.** 1 prohibited edge. check Def. |
| `s2-8` | default export of a lexical arrow | correct | RED | C bare | **Keep.** Bind main.value → project.item (exact). |
| `s3-7` | imported default arrow with no lexical name | correct | RED | C bare | **Loss.** Mark #2 value → anonymous item. The unnamed default arrow has no admitted export target. Keep its Def. |
| `s3-8` | namespace import proves cross file member binding | correct | RED | C bare | **Keep.** Bind main.value → project.item (exact). check Def. |
| `s3-9` | classic script global at the cross file admission boundary | ambiguous | Pass | Admission | **Loss.** A legitimate cross-script global loses its input. Keep the Def and require no new binding without lexical admission. |
| `s4-6` | async as a bare formal is not an async modifier | correct | RED | C bare | **Keep.** Bind 2 specified edges, including main.first → sync.async (exact). 1 prohibited edge. check Def. |
| `s4-7` | commonjs export shorthand and module scope require binding | correct | RED | C bare | **Keep.** Bind main.value → project.item (exact). check Def. |
| `s4-8` | type only import cannot admit a runtime arrow target | correct | Pass | Admission | **Keep.** 1 prohibited edge. check Def. |
| `s5-9` | static and instance arrow fields select different owners | correct | RED | C bare | **Loss.** #3–4 request Exact while self_receiver and qualifier_owner each return two same-file NameOnly targets. Preserve both Defs. Record excess NameOnly arms against the crossed negatives #5–6 when the ordinary binder emits them. Never certify an Exact edge at either site. |
| `s5-10` | conflicting star reexports do not admit either arrow | correct | Pass | Admission | **Keep.** 2 prohibited edges. check Def. |
| `s5-11` | escaped free call identifier denotes the lexical arrow | correct | RED | C bare | **Loss.** Mark #2 value → project.item. The raw escaped callee spelling yields UnknownName. Keep the bare Def. |
| `s5-12` | enclosing call result does not prove inferred arrow identity | correct | Pass | Admission | **Keep.** 1 prohibited edge. check Def. |
| `x1-8` | default export expression and live export diverge after reassignment | correct | RED | C bare | **Loss.** A snapshot export and a live mutable export have different targets. Keep the decoy negatives and any already proved binding. C cannot add the module-state resolution needed to recover a missing one. The reviewed matrix skips this row. |
| `x1-9` | calling a getter result does not pass the argument to the setter | correct | RED | C bare | **Loss.** Bind the direct call to apply. A getter returning apply needs a separate returned-value proof. Without it, omit that arm and keep the setter exclusion. The reviewed matrix skips this row. |
| `x2-6` | immediately invoked bare arrows without inferred names | correct | RED | C bare | **Loss.** Mark #1–2. Both parenthesized literal-arrow call sites drop UnknownName rather than resolving to the distinct anonymous owners. Keep both crossed negatives. |
| `x2-7` | refusal distinguishes two exact calls to one export | wrong | Pass | Consumers | **Keep.** Resolve parseInt to parseInt@1. bind 2 specified edges, including admitted.item → parseInt.text (exact). check reachability, unreachability. |
| `x3-6` | object spread overwrites one leaf before a bare formal read | correct | RED | C bare | **Set.** Use lane 3's literal-leaf producer and require the independent b leaf only. Keep the overwritten a negative. A missing precondition in that producer is reported against its base. |
| `x3-7` | reopened typescript namespace does not share private lexical arrows | correct | RED | C bare | **Loss.** Mark #2 only. The second namespace call has two local_def Exact entries and fails exact_singleton. Keep the first call’s singleton positive and both decoy negatives. |
| `x4-5` | two star reexports of one binding admit one arrow edge | correct | RED | C bare | **Keep.** Bind main.value → visit.item (exact). |
| `x4-6` | a local type alias leaves an imported arrow in value scope | correct | RED | C bare | **Keep.** Bind main.value → visit.item (exact). |
| `x5-5` | commonjs exports alias after replacing module exports | correct | RED | C bare | **Loss.** After module.exports is replaced, its old exports alias receives stale, while require obtains live. C does not simulate mutable module exports. Retain the reviewed skip and prohibit a new Exact to stale. The reviewed matrix skips this row. |
| `x5-6` | module var and global object property hold different arrows | correct | RED | C bare | **Loss.** The module-local var and globalThis property hold different arrows. Preserve the admitted lexical-call control separately. Dynamic host-global lookup remains a reviewed loss. The reviewed matrix skips this row. |

## D · Built-in callbacks

92 rows. 53 enumerated.

| Row | Shape | Input truth | Base | §5 rule | Required disposition |
|---|---|---|---|---|---|
| `h01a` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@4:5>.e (nameonly/builtin_callback). |
| `h01b` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:11>.e (nameonly/builtin_callback). |
| `h01c` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:14>.e (nameonly/builtin_callback). |
| `h01d` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:12>.e (nameonly/builtin_callback). |
| `h01e` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:17>.e (nameonly/builtin_callback). |
| `h01f` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:12>.e (nameonly/builtin_callback). |
| `h01g` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:13>.e (nameonly/builtin_callback). |
| `h01h` | array method | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:15>.e (nameonly/builtin_callback). |
| `h02a` | reducer | correct | RED | D role | **Loss.** Require the element → second formal. The explicit initializer is the real seed but D does not trace that distinct source to the accumulator. |
| `h02b` | reducer | correct | RED | D role | **Loss.** Require the element → second formal. The explicit initializer is the real seed but D does not trace that distinct source to the accumulator. |
| `h03a` | promise | correct | RED | D role | **Keep.** Bind main.p → &lt;anon@4:5>.r (nameonly/builtin_callback). |
| `h03b` | promise | correct | RED | D role | **Keep.** Bind main.p → ok.first (nameonly/builtin_callback). 2 prohibited edges. |
| `h04` | named local | correct | RED | D callback | **Keep.** Bind main.arr → handler.e (nameonly/builtin_callback). |
| `h05a` | named imported | correct | Pass | D callback | **Loss.** No D edge. A runtime callback exists but D's named admission requires a same-file immutable callable declaration or direct const callable at the value occurrence. |
| `h05b` | shadowed | correct | Pass | D callback | **Keep.** 2 prohibited edges. |
| `h05c` | local let | correct | Pass | D callback | **Loss.** No D edge. A runtime callback exists but D's named admission requires a same-file immutable callable declaration or direct const callable at the value occurrence. |
| `h06a` | callback role | correct | RED | D role | **Keep.** Bind main.arr → &lt;anon@3:15>.e (nameonly/builtin_callback). 1 prohibited edge. |
| `h06b` | no formal | correct | Pass | D role | **Keep.** 1 prohibited edge. |
| `h06c` | destructured formal | correct | Pass | D role | **Loss.** No D edge to a destructured element leaf. D needs a supported entry Def for the delivered role. |
| `h07a` | member only formal | correct | RED | D receiver | **Keep.** Bind simple.items.forEach → &lt;anon@2:17>.v (nameonly/builtin_callback). check reachability. |
| `h07b` | mixed formal | correct | RED | D receiver | **Keep.** Bind simple.items → &lt;anon@3:17>.v (nameonly/builtin_callback). check reachability. |
| `h07c` | member receiver | correct | RED | D receiver | **Keep.** Bind main.o.list.forEach → &lt;anon@2:18>.v (nameonly/builtin_callback). check reachability. |
| `h07d` | optional receiver | correct | RED | D receiver | **Keep.** Bind main.arr → &lt;anon@3:16>.v (nameonly/builtin_callback). check reachability. |
| `h07e` | this receiver | correct | RED | D receiver | **Set.** Bind this.items only if one of the byte-matched receiver nodes in §5 exists. No substitute from a method or receiver on another occurrence. |
| `h07f` | shared line | correct | RED | D receiver | **Keep.** Bind 2 specified edges, including main.arr → &lt;anon@4:15>.v (nameonly/builtin_callback). 2 prohibited edges. check reachability. |
| `h07g` | no receiver node | correct | Pass | D receiver | **Loss.** Keep the negative into v. The literal [1] delivers an element, but the call has no eligible receiver Use from which D could emit the edge. |
| `h08` | in repo receiver | ambiguous | RED | D role | **Loss.** Emit the name-only table candidate for Box.map. The body supplies no witnessed element delivery. Checker evidence decides WRONG or UNDECIDED. No conversion credit for an unwitnessed candidate. |
| `h09a` | call result receiver | ambiguous | Unexecuted | D receiver | **Loss.** No D edge into k from Object.keys(o), Object, or o. The result lacks its own receiver value Use. Replace the skipped positive with an enabled negative. |
| `h09b` | call result receiver | ambiguous | Unexecuted | D receiver | **Loss.** No D edge into y from the intermediate result, arr or f. Preserve any independent inner map edge. Enable the outer negative. |
| `h09c` | call result receiver | ambiguous | Unexecuted | D receiver | **Loss.** No D edge into r from fetch(u), fetch or u. The argument is not the promise result. Enable the negative. |
| `h10a` | excluded method | correct | Pass | D receiver | **Keep.** 2 prohibited edges. |
| `h10b` | excluded method | correct | Pass | D receiver | **Keep.** 2 prohibited edges. |
| `h10c` | excluded method | correct | Pass | D receiver | **Keep.** 2 prohibited edges. |
| `h11` | array method | correct | RED | D label | **Keep.** Check reachability, trace boundary. |
| `h12a` | instance override | ambiguous | RED | D role | **Loss.** Emit the name-only table candidate. The installed method delivers 99 and the receiver holds 0. Grade that emitted transfer WRONG. |
| `h12b` | instance override | ambiguous | Unexecuted | D role | **External.** Replace the historical UNDECIDED counter and its skip explanation with the independent builtin grade for h12a. Require WRONG = 1, CORRECT = 0, UNDECIDED = 0. Keep the row external. Use an opaque override for the UNDECIDED control. |
| `h13` | same line exact call | correct | RED | D label | **Set.** The original receiver lacks a matching Use. Repair the fixture per the byte receipt below. Assert the doubtful edge and not_exact boundary; run the variable-root x3-8 companion as the integrated D gate. |
| `h14a` | refusal reachability | correct | RED | Consumers | **Keep.** Bind 2 specified edges, including origin.arr → handler.v (nameonly/builtin_callback). check Def, unreachability. |
| `h14b` | refusal reachability | correct | Unexecuted | Consumers | **External.** Check the classified reachability change in the external refusal ledger with the seed and both witness paths. Keep the companion CPG assertions. |
| `t1-11` | set duplicate value role | correct | RED | D role | **Loss.** Bind the value at slot zero only. The Set repeats that value at slot one. The Map key is a separate role. Both second slots are outside the table contract. |
| `t1-12` | map key role | correct | RED | D role | **Loss.** Bind the value at slot zero only. The Set repeats that value at slot one. The Map key is a separate role. Both second slots are outside the table contract. |
| `t2-5` | bound callback prefix boundary | correct | Pass | D callback | **Loss.** No D edge through bind. The callback receives a prepended argument and the element would enter a different formal. |
| `t2-6` | array and thisarg roles | correct | RED | D role | **Loss.** Bind the element only. The index, whole collection and supplied thisArg have different roles. |
| `t3-3` | reducer without initializer | correct | RED | D role | **Set.** Uphold the recording. Without an initializer the first visited element seeds acc, and a subsequent element feeds value. Require both receiver edges as builtin_callback. See h02a for an explicit seed. |
| `t3-4` | typed array receiver | correct | RED | D role | **Keep.** Bind main.values → &lt;anon@3:14>.value (nameonly/builtin_callback). 1 prohibited edge. |
| `t4-5` | thisarg function is not callback | correct | Pass | D callback | **Keep.** 1 prohibited edge. |
| `t5-2` | callback after empty spread | correct | RED | D callback | **Keep.** Bind main.arr → handler.value (nameonly/builtin_callback). 1 prohibited edge. |
| `t5-3` | callback after unknown spread | ambiguous | Pass | D callback | **Loss.** No D edge to handler after an unknown spread. It is the callback only if the preceding spread is empty. |
| `s1-12` | parenthesized inline callback identity | correct | RED | D callback | **Keep.** Bind main.arr → &lt;anon@3:16>.value (nameonly/builtin_callback). |
| `s1-13` | member callback does not select a lexical decoy | correct | Pass | D callback | **Loss.** No D edge through the member-valued callback, including to the free-name decoy. Its unbound reference has no supported D callback identity. |
| `s1-14` | typescript this is not the element slot | correct | RED | D role | **Keep.** Bind main.arr → handler.value (nameonly/builtin_callback). 1 prohibited edge. |
| `s1-15` | find last is outside the method table | correct | RED | D receiver | **Loss.** Keep the find transfer to inside and the negative into outside. The concrete arr also calls outside with its element through findLast, which is outside the closed table. |
| `s1-16` | dynamic property identifier is not a method name | correct | Pass | D receiver | **Keep.** 1 prohibited edge. |
| `s1-17` | reducer element slot inside rest | ambiguous | Pass | D role | **Set.** Uphold the recording. Element role one lies inside rest at zero. Replace the negative with one builtin_callback edge arr → parts. No separate edges for the index, initializer or array role. |
| `s2-9` | static bracket method with optional invocation | correct | RED | D receiver | **Keep.** Bind main.arr → handler.value (nameonly/builtin_callback). 1 prohibited edge. |
| `s2-10` | private method is not a table method | correct | Pass | D receiver | **Keep.** 1 prohibited edge. check Def. |
| `s2-11` | same name call chain does not reuse inner receiver | correct | RED | D receiver | **Loss.** Bind arr only to transform at the inner site. The outer call consumes the intermediate result and loses its transfer to consume. |
| `s2-12` | named function expression uses inline identity | correct | RED | D callback | **Keep.** Bind main.arr → handler.value (nameonly/builtin_callback). 1 prohibited edge. |
| `s2-13` | destructured accumulator consumes slot zero | correct | RED | D role | **Keep.** Bind main.arr → step.value (nameonly/builtin_callback). 1 prohibited edge. |
| `s2-14` | hole visiting and hole skipping methods | ambiguous | RED | D role | **Loss.** Uphold the recording. Runtime find invokes visits(undefined) and map never invokes skips. D projects both element candidates. Record the map candidate as a false positive, no credit. |
| `s2-15` | thenable invokes rejection slot with separate source | ambiguous | RED | D role | **Loss.** D #1–3: the table proposes fulfillment; runtime calls fail(reason) only. At E2 add the conditional target/payload case. Pin the outer caller outcome independently. With an admitted Exact holder caller require reason → rejected.reason, otherwise require absence from this caller and record the missed runtime transfer. Scope D’s negative to its label. |
| `s3-10` | generator callback owner uses byte column after unicode | correct | RED | D callback | **Keep.** Bind main.arr → &lt;anon@3:35>.value (nameonly/builtin_callback). 1 prohibited edge. check Def. |
| `s3-11` | generic builtin invocation through call is excluded | correct | Pass | D receiver | **Loss.** Keep the Def and negative from arr. The borrowed map delivered through Function.call has no attached table method on arr at this call site. |
| `s3-12` | rejection only then does not select first callable | correct | Pass | D role | **Loss.** Keep the Def and negative into reject. The rejected promise delivers reason. D has no rejection role and cannot use the first callable it happens to see at argument one. |
| `s3-13` | temporal dead zone shadows an outer callback | correct | Pass | D callback | **Keep.** 2 prohibited edges. check Def. |
| `s3-14` | two doubt kinds on one line preserve both boundaries | correct | RED | D label | **Set.** Stage D #1,3,5 and E2 #2,4 (rerun all at E2). Separate the D receiver occurrence from the E payload Use as in the byte receipt, retaining the common root, both barriers and reachability. |
| `s4-9` | one named callback has different element slots at same line sites | correct | RED | D role | **Loss.** Repair the first receiver endpoint and keep #1 and #3–6. Mark only #2: no receiver Use at the second arr occurrence for reduce. Keep the same-line layout. |
| `s4-10` | escaped property identifier denotes the table method | correct | RED | D receiver | **Keep.** Bind main.arr → handler.item (nameonly/builtin_callback). 1 prohibited edge. |
| `s4-11` | discarded sequence operand is not the receiver | correct | Pass | D receiver | **Loss.** No edge from discarded. The comma expression is outside the receiver-path grammar, so the selected arr also supplies no D edge. |
| `s4-12` | imported alias is excluded from the table but admitted through a holder | correct | RED | D callback | **Loss.** No D edge through the imported alias. Require E's independently proved alias target and payload edge, name-only. |
| `s4-13` | user method table edge and actual in body callback payload | ambiguous | RED | D role | **Loss.** D #3–4: the box → handler.item builtin candidate is a known excess, and Exact is forbidden. E2 #1–2 require the handler target and payload → handler.item only with an admitted Exact holder caller. Pin Box.map’s constructor_local Exact singleton on this fixture; a changed prerequisite must be reported. |
| `s5-13` | borrowed array map on an array like receiver | correct | RED | D role | **Keep.** Bind main.list → handler.item (nameonly/builtin_callback). 1 prohibited edge. |
| `s5-14` | array from mapping callback is outside the method table | correct | Pass | D receiver | **Loss.** Keep the Def and negative from input. For an iterable such as [1], Array.from delivers an element to handler. This static function and its second-argument mapper fall outside the D table. |
| `s5-15` | reassigned function declaration is not the current callback | correct | Pass | D callback | **Loss.** Keep the Def and negative into handler.original. The assignment makes the old declaration stale. The element actually enters replacement.current, but D cannot recover a named callback by tracking mutable assignments. |
| `s5-16` | inline callback inside rather than after a dense spread | ambiguous | RED | D callback | **Set.** For callback identity read the inline function at exact runtime slot zero inside the dense literal. Require one builtin_callback edge from arr. This creates no A edge from a spread leaf. |
| `s5-17` | computed receiver index is not the element source | correct | RED | D receiver | **Loss.** Keep the plain arr control. No edge from lists, index or a different arr occurrence into handler. The computed receiver value has no supported static path. |
| `s5-18` | erased type assertion preserves inline callback identity | correct | RED | D callback | **Keep.** Bind main.arr → &lt;anon@3:12>.item (nameonly/builtin_callback). 1 prohibited edge. |
| `x1-10` | parentheses preserve the method receiver while comma detaches it | correct | RED | D receiver | **Keep.** Bind main.items → kept.item (nameonly/builtin_callback). 2 prohibited edges. check Def. |
| `x1-11` | one function in both promise handler slots | correct | RED | D role | **Keep.** Bind main.promise → settle.item (nameonly/builtin_callback). 1 prohibited edge. |
| `x1-12` | singleton reducer distinguishes omission from an undefined initializer | ambiguous | RED | D role | **Loss.** One element and no initializer produce zero reducer invocations. With explicit undefined there is an invocation and only the ordinary element role. D models potential delivery and may over-report the first call. |
| `x1-13` | constructing a method is not a builtin callback invocation | correct | RED | D receiver | **Keep.** Bind main.items → mapped.item (nameonly/builtin_callback). 2 prohibited edges. check Def. |
| `x1-14` | global callback name matches an unimported module export | correct | RED | D callback | **Keep.** Bind main.items → local.text (nameonly/builtin_callback). 2 prohibited edges. check Def. |
| `x2-8` | tag substitution is not the first map argument | correct | RED | D receiver | **Keep.** Bind main.items → ordinary.item (nameonly/builtin_callback). 2 prohibited edges. check Def. |
| `x2-9` | nested array iteration reaches through two name only edges | correct | RED | D label | **Keep.** Bind 2 specified edges, including main.batches → visit.items (nameonly/builtin_callback). 2 prohibited edges. check reachability, trace boundary. |
| `x2-10` | flatmap returns the inner function without invoking it | correct | RED | D callback | **Keep.** Bind main.items → &lt;anon@3:17>.outer (nameonly/builtin_callback). 2 prohibited edges. check Def, unreachability. |
| `x3-8` | same line roots enter one formal with different confidence | correct | RED | D label | **Set.** Use the represented receiver and distinct variable roots in the byte receipt. Require the builtin edge to stop for not_exact, and the direct Exact argument to descend from its own root. |
| `x3-9` | table name and installed reducer disagree on the element slot | ambiguous | RED | D role | **Loss.** Installed reduce delivers seed 0 to acc and receiver element 7 to value. Grade the emitted items → acc WRONG. Record the missing items → value independently. Neither is Exact. |
| `x3-10` | nested then returns a promise in the outer handler slot | correct | RED | D callback | **Keep.** Bind main.inner → visit.value (nameonly/builtin_callback). 2 prohibited edges. |
| `x4-7` | conditional receiver selects either array but never the condition | ambiguous | RED | D receiver | **Loss.** No table edge from flag, left or right. The receiver expression denotes a selected value, but neither branch is a Use of the entire receiver path. |
| `x4-8` | throwing array species prevents map callbacks but allows forEach | ambiguous | RED | D role | **Loss.** Keep the forEach transfer. The throwing species getter prevents map invocation. The name-only map candidate is a known false positive. |
| `x5-7` | iterator map defers the callback until consumption | ambiguous | RED | D role | **Loss.** D proposes both name-only candidates. Only the consumed iterator invokes active. The idle candidate is a known false positive. |
| `x5-8` | callback factory input and returned callback have different roles | ambiguous | RED | D callback | **Loss.** Uphold the recording. Both calls deliver to visit, never to make.seed. Keep input → make.seed and the second D edge. The first D edge is missing because its callback is a factory result. |

## E · Calls through a parameter

161 rows. 136 enumerated.

| Row | Shape | Input truth | Base | §5 rule | Required disposition |
|---|---|---|---|---|---|
| `e01` | named target | correct | RED | E source | **Keep.** Resolve cb to handler@5. bind run.input → handler.e (nameonly/call_nameonly). |
| `e02a` | anonymous target | ambiguous | RED | E payload | **Set.** Add the payload → w edge to the byte-identified inline bare arrow with nameonly/call_nameonly, including when ordinary slice C is disabled. |
| `e02b` | anonymous target | correct | RED | E payload | **Keep.** Resolve cb to &lt;anon@6:7>@6. bind run.input → &lt;anon@6:7>.w (nameonly/call_nameonly). |
| `e03a` | mixed targets | correct | RED | E cap | **Keep.** Resolve cb to h@5, k@8. |
| `e03b` | mixed targets | correct | RED | E cap | **Keep.** Resolve cb to h@5. |
| `e04a` | payload arity | correct | RED | E payload | **Keep.** Bind 2 specified edges, including run.a → handler.x (nameonly/call_nameonly). |
| `e04b` | payload arity | correct | RED | E payload | **Set.** Keep the first payload edge only. The surplus argument contributes no edge on this target. |
| `e05` | fanout | correct | RED | E cap | **Loss.** Nine distinct functions exceed eight. Drop the entire slot as ArgFanout, increment once per affected site and emit no payload edge. |
| `e06a` | written formal | correct | Pass | E stability | **Keep.** Keep UnknownName drop. 1 prohibited edge. |
| `e06b` | nameonly caller | correct | Pass | E source | **Loss.** A name-only call to the holder cannot ground the parameter. Keep the original drop even if a runtime receiver could select this method. |
| `e06c` | spread caller | correct | Pass | E value | **Keep.** Keep UnknownName drop. 1 prohibited edge. |
| `e06d` | excluded call apply | correct | Pass | E source | **Loss.** Keep the original drop and the negative. cb.call(this, x) delivers x to handler.e here. This member use is outside #434's direct parameter-call rule. |
| `e07` | imported holder | correct | RED | E source | **Keep.** Resolve cb to handler@2. bind run.input → handler.e (nameonly/call_nameonly). |
| `e08a` | forward depth | correct | RED | E forwarding | **Keep.** Resolve cb to handler@5. |
| `e08b` | forward depth | correct | Pass | E forwarding | **Loss.** The only grounding caller is at depth three, counting the direct holder caller as one. Keep the original drop. |
| `e09a` | existing name search | correct | RED | E source | **Set.** Preserve the existing target and its CallNameOnly label. The old receipt printed call_name_only. Matrix revision 5767ceb8 fixed the adapter spelling. Obtain a new baseline outcome before interpreting this historical RED. |
| `e09b` | default formal | correct | RED | E value | **Keep.** Resolve cb to noop@1. bind run.input → noop.e (nameonly/call_nameonly). |
| `e10` | engine boundary | correct | RED | D label | **Keep.** Check reachability, trace boundary. |
| `e11` | call result | correct | RED | E source | **Loss.** Resolve the inner cb to provider and require x → p with call_nameonly. Keep the outer UnknownName drop and the negative from y. The returned anonymous function receives y at runtime, but its identity requires following a call result. |
| `e12` | cross file free single | wrong | RED | E source | **Keep.** Resolve setTimeout to setTimeout@1 and check the other site set. bind setTimeout.input → real.v (nameonly/call_nameonly). 1 prohibited edge. |
| `e13` | anonymous caller | correct | RED | E source | **Keep.** Resolve f to h@6, k@9. |
| `e14` | same line exact call | correct | RED | D label | **Set.** Keep both assertions. Assert BoundaryExited/not_exact on the exact root/from/to tuple, with the call_nameonly edge. Use a same-line direct variable argument and a separate root to prove Exact descent. |
| `e15` | destructured slot | correct | RED | E value | **Keep.** Resolve a to h@6 and check the other site set. |
| `e16a` | refusal reachability | correct | RED | Consumers | **Keep.** Bind 3 specified edges, including origin.x → run.y (exact). check Def, unreachability. |
| `e16b` | refusal reachability | correct | Unexecuted | Consumers | **External.** Check the classified reachability change in the external refusal ledger with the seed and both witness paths. Keep the companion CPG assertions. |
| `t1-6` | explicit undefined default | correct | RED | E value | **Keep.** Resolve cb to fallback@1. bind run.input → fallback.e (nameonly/call_nameonly). |
| `t1-7` | supplied callback overrides default | correct | RED | E value | **Keep.** Resolve cb to handler@4. 1 prohibited edge. |
| `t1-8` | destructured alias | correct | RED | E value | **Keep.** Resolve renamed to handler@5. bind run.input → handler.e (nameonly/call_nameonly). |
| `t1-9` | logical assignment write | ambiguous | Pass | E stability | **Loss.** The logical assignment may write the declaration. With this callable it short-circuits, yet the holder-wide stability rule removes its entry witness. |
| `t1-10` | destructuring write | correct | Pass | E stability | **Keep.** 1 prohibited edge. |
| `t2-7` | captured formal write | correct | Pass | E stability | **Keep.** 1 prohibited edge. |
| `t2-8` | loop formal write | correct | Pass | E stability | **Keep.** 1 prohibited edge. |
| `t2-9` | sloppy mapped arguments write | correct | Pass | E stability | **Keep.** 1 prohibited edge. |
| `t2-10` | null does not activate default | correct | Pass | E value | **Keep.** 1 prohibited edge. |
| `t2-11` | shadowed holder formal | correct | Pass | E source | **Keep.** 1 prohibited edge. |
| `t2-12` | callback payload known spread | correct | RED | E payload | **Set.** Resolve the grounded target independently of the payload. Apply A's exact-offset tests to its fixed slots and inherit the derived label. |
| `t3-5` | destructured property default | correct | RED | E value | **Keep.** Resolve cb to fallback@5. bind run.input → fallback.e (nameonly/call_nameonly). |
| `t3-6` | destructured property overrides default | correct | RED | E value | **Keep.** Resolve cb to handler@8. 1 prohibited edge. |
| `t3-7` | direct eval formal write | correct | Pass | E stability | **Keep.** 1 prohibited edge. |
| `t3-8` | strict unmapped arguments write | correct | RED | E stability | **Keep.** Resolve cb to handler@7. bind run.input → handler.e (nameonly/call_nameonly). |
| `t3-9` | ts this does not consume callback slot | correct | RED | E value | **Keep.** Resolve cb to handler@4. bind run.x → handler.p (nameonly/call_nameonly). |
| `t3-10` | callback payload unknown spread | correct | RED | E payload | **Set.** Resolve the grounded target independently of the payload. Apply A's exact-offset tests to its fixed slots and inherit the derived label. |
| `t4-6` | sloppy nonsimple parameters unmapped arguments | correct | RED | E stability | **Keep.** Resolve cb to handler@6. bind run.input → handler.e (nameonly/call_nameonly). |
| `t4-7` | default and explicit callers union | correct | RED | E value | **Keep.** Resolve cb to fallback@1, handler@4. bind 2 specified edges, including run.input → fallback.e (nameonly/call_nameonly). |
| `t4-8` | array destructured callback slots | correct | RED | E value | **Keep.** Resolve cb to h@6 and check the other site set. 2 prohibited edges. |
| `t4-9` | callback target rest formal | correct | RED | E payload | **Keep.** Resolve cb to handler@4. bind run.input → handler.values (nameonly/call_nameonly). 1 prohibited edge. |
| `t4-10` | recursive holder forwarding cycle | correct | RED | E forwarding | **Keep.** Resolve cb to handler@7. bind run.input → handler.p (nameonly/call_nameonly). |
| `t5-4` | optional callback invocation | correct | RED | E source | **Keep.** Resolve cb to handler@4. bind run.input → handler.p (nameonly/call_nameonly). 1 prohibited edge. |
| `t5-5` | duplicate object key last wins | correct | RED | E value | **Keep.** Resolve cb to last@8. bind run.input → last.p (nameonly/call_nameonly). 1 prohibited edge. |
| `s1-18` | holder callback after known spread | correct | RED | E value | **Keep.** Resolve cb to selected@4. bind run.input → selected.p (nameonly/call_nameonly). 1 prohibited edge. |
| `s1-19` | shadowed undefined is a supplied callback | correct | RED | E value | **Keep.** Resolve cb to selected@4. bind run.input → selected.p (nameonly/call_nameonly). 1 prohibited edge. |
| `s1-20` | parameter default captures earlier formal | correct | RED | E value | **Keep.** Resolve cb to selected@5. bind run.input → selected.p (nameonly/call_nameonly). |
| `s1-21` | sloppy duplicate formal uses last slot | correct | RED | E stability | **Keep.** Resolve cb to late@7. bind run.input → late.p (nameonly/call_nameonly). 1 prohibited edge. |
| `s1-22` | nested destructuring keeps full key path | correct | RED | E value | **Set.** Use two typed paths, left.cb and right.cb. Require first and second independently with no crossed payloads. |
| `s1-23` | fanout counts distinct targets at the cap | correct | RED | E cap | **Set.** Nine calls naming eight distinct functions keep all eight. The duplicate h0 consumes no ninth position. |
| `s1-24` | same line anonymous callback identities | correct | RED | E cap | **Set.** Count both inline callable identities, including their distinct start-byte columns. Require both payloads. |
| `s1-25` | constructor is the callback holder | correct | RED | E source | **Set.** #1 cb → handler and #2 constructor.input → handler.p (nameonly/call_nameonly) are blocked/RED without the new Runner Exact Source caller. Test both with that caller supplied. Omitted and wrong-holder grounds contribute neither. |
| `s1-26` | forwarding cycle with no grounded callback | correct | Pass | E forwarding | **Keep.** Keep UnknownName drop. 1 prohibited edge. |
| `s1-27` | jsx explicit callback overrides earlier prop spread | correct | RED | E value | **Set.** The explicit cb after the JSX spread is the last assignment. Admit handler through that key. |
| `s1-28` | nameonly callback result does not certify return flow | correct | RED | E payload | **Keep.** Resolve cb to handler@5. bind run.input → handler.p (nameonly/call_nameonly). check counter. |
| `s1-29` | escaping holder retains known caller | correct | RED | E source | **Keep.** Resolve cb to handler@4. bind run.input → handler.p (nameonly/call_nameonly). 1 prohibited edge. |
| `s1-30` | imported callback alias with local name decoy | correct | RED | E value | **Keep.** Resolve cb to handler@1. bind run.input → handler.p (nameonly/call_nameonly). 1 prohibited edge. |
| `s1-31` | write to shadow binding keeps holder callback | correct | RED | E stability | **Keep.** Resolve cb to handler@10. bind run.input → handler.p (nameonly/call_nameonly). |
| `s2-16` | var redeclaration without initializer preserves formal | correct | RED | E stability | **Keep.** Resolve cb to handler@5. bind run.input → handler.value (nameonly/call_nameonly). |
| `s2-17` | hoisted body function replaces formal before call | correct | Pass | E stability | **Keep.** 1 prohibited edge. |
| `s2-18` | later parameter is uninitialized during default | correct | Pass | E value | **Keep.** 2 prohibited edges. |
| `s2-19` | deleted mapped argument does not rebind formal | correct | RED | E stability | **Set.** Unconditional deletion before the later element write severs that arguments mapping. Keep kept and exclude other. |
| `s2-20` | void expression activates default without passing operand | correct | RED | E value | **Keep.** Resolve cb to fallback@1. bind run.input → fallback.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s2-21` | with object intercepts formal name | correct | Pass | E stability | **Loss.** Keep the negative into original. The with object intercepts cb and supplies replacement. A dynamic property lookup supplies no entry-formal identity from which E can bind the real transfer. |
| `s2-22` | non null callee with erased type arguments | correct | RED | E source | **Keep.** Bind run.input → handler.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s2-23` | unqualified cross file forwarding hop is not admitted | correct | RED | E source | **Keep.** Resolve cb to selected@2. bind setTimeout.input → selected.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s2-24` | late object spread overwrites explicit callback | correct | RED | E value | **Set.** Fold the explicit nested object literal in source order. The later own cb wins. An unknown spread at that point would leave the key opaque. |
| `s2-25` | inherited callback property does not trigger default | correct | Pass | E value | **Loss.** Keep the negative into fallback. The custom prototype supplies inherited. The supported own-data path proves neither that inherited callable nor the absence required by the default. |
| `s2-26` | computed numeric key matches string key | correct | RED | E value | **Keep.** Resolve cb to selected@4. bind run.input → selected.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s2-27` | default initializer uses parameter before body function | correct | RED | E stability | **Keep.** Resolve cb to selected@7. bind run.input → selected.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s2-28` | arrow callback is callable but not constructable | correct | RED | E source | **Keep.** Resolve cb to handler@5. bind run.input → handler.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s2-29` | inner holder uses its own callback in same name chain | correct | RED | E source | **Keep.** Resolve left to first@15. bind run.input → first.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s3-15` | local callee alias is outside formal following | correct | RED | E source | **Loss.** Require the direct cb site. The local scalar alias site has no declaration-backed formal slot under #434, even though alias calls the same function. |
| `s3-16` | define property updates mapped argument before detaching it | correct | Pass | E stability | **Loss.** Keep the original Def and negative. defineProperty writes replacement through the live map before detaching it. The entry slot is stale and E does not derive a new target from that write. |
| `s3-17` | optional eval is indirect and cannot rebind the formal | correct | RED | E stability | **Keep.** Resolve cb to handler@5. bind run.input → handler.value (nameonly/call_nameonly). |
| `s3-18` | anonymous default initializer is a callback target | correct | RED | E value | **Set.** The inline default is grounded only by the caller supplying undefined. Resolve its byte-identified function and bind the payload name-only. |
| `s3-19` | array pattern elision and sparse actual activate slot default | correct | RED | E value | **Keep.** Resolve cb to fallback@4. bind run.input → fallback.value (nameonly/call_nameonly). 1 prohibited edge. |
| `s3-20` | later accessor replaces an own callback value | correct | Pass | E value | **Loss.** Keep the earlier Def and the negative into it. The final getter returns undefined so fallback runs. Invoking a getter to establish the default exceeds the static literal proof. |
| `s3-21` | distinct symbols with the same description are not the same key | correct | Pass | E value | **Loss.** Keep the decoy Def and negative. The first symbol selects selected at runtime. The string/index path model cannot represent that symbol identity, and equal descriptions do not equate the keys. |
| `s3-22` | class callback is constructable but not callable | correct | RED | E source | **Loss.** A class is not a callable target of cb(input). new cb(input) needs a construct route, which #434 parks. Require no new constructor payload at either site. |
| `s3-23` | spread payload binds the callback rest with derived confidence | correct | RED | E payload | **Keep.** Resolve cb to handler@4. bind 2 specified edges, including run.first → handler.first (nameonly/call_nameonly). 2 prohibited edges. |
| `s4-14` | conditional actual selects an explicit callback or its default | correct | RED | E value | **Keep.** Resolve cb to fallback@1, chosen@4. bind 2 specified edges, including run.input → fallback.item (nameonly/call_nameonly). |
| `s4-15` | truthy left and operand is not the callback value | correct | RED | E value | **Keep.** Resolve cb to chosen@4. bind run.input → chosen.item (nameonly/call_nameonly). 1 prohibited edge. |
| `s4-16` | derived callee edge cannot feed another holder | ambiguous | RED | E source | **Loss.** Keep the first argument-follow target run. Its name-only invocation cannot ground a second holder. No derived edge from run.input to handler.item. |
| `s4-17` | a later write refuses the formal at earlier and later calls | ambiguous | Pass | E stability | **Loss.** No entry-based edge at either call. The later assignment invalidates the holder slot everywhere, including the real earlier invocation. |
| `s4-18` | sloppy block function sharing a parameter name stays in the block | correct | RED | E stability | **Keep.** Resolve cb to handler@9. bind run.input → handler.item (nameonly/call_nameonly). 1 prohibited edge. |
| `s4-19` | nested arrow and ordinary function use different arguments bindings | correct | RED | E stability | **Keep.** Resolve cb to handler@12. bind own.input → handler.item (nameonly/call_nameonly). 1 prohibited edge. |
| `s4-20` | local destructuring of a parameter selects the property not the local spelling | correct | RED | E value | **Keep.** Resolve cb to chosen@5. bind run.input → chosen.item (nameonly/call_nameonly). 1 prohibited edge. |
| `s4-21` | destructured callback used as a jsx component | correct | RED | E source | **Keep.** Resolve Render to Actual@4. |
| `s4-22` | jsx boolean shorthand does not trigger a prop default | correct | Pass | E value | **Keep.** 1 prohibited edge. check Def. |
| `s4-23` | rest array index invocation is outside formal following | ambiguous | Pass | E source | **Loss.** No new target for callbacks[0](input). The callee is an element access on a rest array, not the bound formal identifier. |
| `s4-24` | default copies earlier callback before the body reassigns it | correct | RED | E stability | **Set.** Select source's entry value for cb's default at initialization. A later write to the distinct source binding cannot invalidate cb. |
| `s5-19` | later default writes the callback before the body runs | correct | Pass | E stability | **Loss.** Keep the original Def and negative. The omitted later actual runs an initializer that assigns replacement to cb. Refusing the stale entry value also leaves the replacement outside this argument-follow route. |
| `s5-20` | array pattern consumes iterator values not numeric properties | correct | Pass | E value | **Loss.** Keep the decoy Def and negative. The object's iterator yields selected. A numeric own property is not an array-pattern iterator witness and E does not execute the generator. |
| `s5-21` | distinct overload callers feed one runtime holder | correct | RED | E source | **Keep.** Resolve cb to first@7, second@10. bind 2 specified edges, including run.input → first.item (nameonly/call_nameonly). |
| `s5-22` | bound callback payload cannot enter the prebound prefix slot | correct | Pass | E value | **Loss.** Keep both slot prohibitions and omit the bound target. A bind result carries a prefix adapter outside #434. |
| `s5-23` | bare arrow formal is a callback holder | correct | RED | E source | **Keep.** Resolve cb to handler@5. bind run.input → handler.item (nameonly/call_nameonly). 1 prohibited edge. check Def. |
| `s5-24` | same line callback calls in different holders keep lexical slots | correct | RED | E source | **Keep.** Resolve cb to first@6 and check the other site set. bind 2 specified edges, including left.input → first.item (nameonly/call_nameonly). 2 prohibited edges. |
| `s5-25` | only rightmost duplicate formal has a mapped arguments entry | correct | RED | E stability | **Set.** Map only the rightmost duplicate in a sloppy simple parameter list. Refuse the holder whose live mapped slot is actually written. |
| `s5-26` | lexically shadowed eval cannot replace the callback | correct | RED | E stability | **Keep.** Resolve cb to handler@5. bind run.input → handler.item (nameonly/call_nameonly). |
| `s5-27` | setter only property reads as undefined and selects default | correct | RED | E value | **Set.** The final own accessor has only a setter, so its read selects fallback. Add getter-then-setter and setter-then-getter controls, both retaining the getter, and data/accessor replacements (§5 E). |
| `s5-28` | callback object literal leaf carries derived confidence | correct | RED | E payload | **Loss.** The object leaf can reach handler.box.item at runtime. Lane 3 admits literal-leaf transfers at Exact-singleton sites only. E resolves the callback name-only and emits no literal-leaf edge at either grade. |
| `x1-15` | known callback precedes an unknown spread at the holder call | correct | RED | E value | **Keep.** Resolve cb to handle@5. bind run.input → handle.item (nameonly/call_nameonly). 1 prohibited edge. |
| `x1-16` | recursive forwarding swaps the two callback slots | correct | RED | E forwarding | **Set.** Follow the two distinct slot identities to their grounded external callers within budget. Keep left and right once each. |
| `x1-17` | no holder call versus a holder call with an omitted argument | correct | RED | E value | **Keep.** Resolve cb to fallback@1. bind active.input → fallback.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x1-18` | outer object default supplies a property before its own default | correct | RED | E value | **Keep.** Resolve cb to outer@1. bind run.input → outer.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x1-19` | later argument assignment does not replace an already read callback | correct | RED | E value | **Loss.** A mutable let actual is outside the terminal grammar. No first or second target from this caller. At runtime the already-read argument is first. |
| `x1-20` | destructured concise method versus same named free declaration | correct | RED | E value | **Set.** Resolve the concise method by its own callable node and span. Do not resolve the same-spelled free declaration. |
| `x1-21` | rejected foreign callers do not consume the target fanout budget | wrong | RED | E source | **Keep.** Resolve setTimeout to setTimeout@1 and check the other site set. bind setTimeout.input → real.item (nameonly/call_nameonly). 1 prohibited edge. check counter. |
| `x1-22` | one jsx expression child is callable while siblings form an array | correct | RED | E value | **Loss.** No target from JSX children under the named-attribute rule. The single expression is callable at runtime and the siblings form an array. Never credit sibling. |
| `x1-23` | a helper mutates mapped arguments or a spread copy | correct | RED | E stability | **Loss.** Keep copied.input → original and the separate borrowed and copied negatives. The helper mutates a live mapped index only on the borrowed arm. E refuses that stale entry without deriving replacement from the helper write. |
| `x1-24` | same name targets at the same line in distinct caller modules | correct | RED | E cap | **Keep.** Resolve cb to handler@2, handler@2. bind 2 specified edges, including run.input → handler.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x1-25` | setter assignment as the only invocation of a callback holder | ambiguous | Pass | E source | **Loss.** The setter invocation has no admitted Source holder call. Keep the drop and the negative payload assertion. |
| `x2-11` | reducer initializer and direct caller supply different functions | ambiguous | RED | E source | **Loss.** Bind D's reducer element to step.value. E gets direct from the Exact step call only. The reducer's real initializer callable never enters E as a caller. |
| `x2-12` | callback parameter used as a template tag | correct | RED | E source | **Loss.** Retain the implicit template-object slot in runtime truth. Tags do not supply an ordinary call argument list to #434. No new target or payload from the tagged invocation. Preserve decoy negatives. |
| `x2-13` | computed key and callback object must come from the same caller | correct | RED | E value | **Loss.** No target from the dynamic binding key. Pairing that key with another caller's object would select decoy. Suppress both contributions, including the true chosen one. |
| `x2-14` | object rest forwarding removes the key that would suppress a default | correct | RED | E value | **Loss.** The surviving object after rest exclusion is not a literal path witness. No fallback or removed target from this forwarding arm. |
| `x2-15` | default created closure retains the parameter environment | correct | RED | E stability | **Set.** Keep the closure's captured parameter-environment declaration. The body var in the separate environment does not rewrite it. |
| `x2-16` | shadowed arguments aliases the outer holders mapped arguments | correct | RED | E stability | **Set.** A borrowed or escaping live mapped arguments object can invalidate the owning holder. A spread copy and a different function's shadow binding cannot write the original formal. |
| `x2-17` | fanout drops one formal and preserves the other on the same line | ambiguous | RED | E cap | **Loss.** Only wide drops as ArgFanout. Keep the stable target and its payload at narrow, one fanout site. Its nine distinct siblings get no payload. |
| `x2-18` | global callback actual and import alias at admitted holder calls | correct | RED | E value | **Keep.** Resolve cb to parseInt@1. keep UnknownName drop. bind imported.input → parseInt.text (nameonly/call_nameonly). 2 prohibited edges. |
| `x2-19` | type query in satisfies is not a second callback value | correct | RED | E value | **Keep.** Resolve cb to chosen@4. bind run.input → chosen.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x2-20` | an iterator return value does not fill a destructured callback slot | correct | RED | E value | **Loss.** An executed generator's iterator protocol and completion result are outside literal selection. No fallback proof and no target decoy. |
| `x2-21` | jsx element actual and component function identity | correct | RED | E value | **Keep.** Resolve cb to passed@9. bind run.input → passed.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x2-22` | forward depth is applied per caller path | ambiguous | RED | E forwarding | **Loss.** Keep near through the two admissible Source callers. Lose deep on its depth-three path. Visiting the deep path cannot suppress the near one. |
| `x2-23` | module namespace object and its callable default export | correct | RED | E value | **Keep.** Resolve cb to direct@5 and check the other site set. bind 2 specified edges, including run.input → direct.item (nameonly/call_nameonly). 3 prohibited edges. |
| `x3-11` | one derived target has fixed slots and another collects both payloads | correct | RED | E payload | **Keep.** Resolve cb to fixed@4, collect@7. bind 4 specified edges, including run.a → fixed.left (nameonly/call_nameonly). 4 prohibited edges. |
| `x3-12` | unknown spread caller cannot add a second callback at an unproved slot | ambiguous | RED | E value | **Loss.** Keep known from the independent direct caller. An unknown earlier spread can put possible in this slot but supplies no slot proof. |
| `x3-13` | computed method name uses the enclosing callback before method formals exist | correct | RED | E source | **Keep.** Resolve cb to nameOf@8. keep UnknownName drop. bind make.input → nameOf.value (nameonly/call_nameonly). 1 prohibited edge. |
| `x3-14` | destructuring reads the property after all caller arguments are evaluated | correct | RED | E value | **Loss.** The first actual reads a member and the second reads a mutated object after argument evaluation. Neither is a static literal path at delivery. No stale early/late target on either arm. |
| `x3-15` | callable proxy apply trap prevents delegation to its target | correct | RED | E value | **Keep.** Bind run.input → direct.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x3-16` | forwarded value is the relays default before the holder applies its own | correct | RED | E forwarding | **Keep.** Resolve cb to chosen@10. bind run.input → chosen.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x3-17` | cross file argument follow edge does not itself seed refusal | correct | RED | Consumers | **Keep.** Resolve cb to handler@6. bind 2 specified edges, including origin.value → run.input (exact). 1 prohibited edge. check reachability. |
| `x3-18` | jsx intrinsic tag does not call a same spelled component alias | correct | RED | E source | **Keep.** Resolve cb to local@11. bind Panel.value → local.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x3-19` | escaped closure uses the creators callback and the later callers payload | correct | RED | E source | **Keep.** Resolve cb to selected@6. bind later.payload → selected.item (nameonly/call_nameonly). 2 prohibited edges. |
| `x4-9` | a table fed callable formal also has an exact direct caller | ambiguous | RED | E source | **Loss.** D feeds visit.cb as an element. Only the direct Exact visit call contributes direct to E. The callable element remains a real unmodelled target. |
| `x4-10` | two bindings of one accessor read two different callbacks | correct | RED | E value | **Loss.** Both reads invoke stateful getter code. No static value or grounded target for either local. Never collapse the two reads into a union of guessed getter results. |
| `x4-11` | an empty object pattern can prevent entry to a callback holder | correct | RED | E value | **Loss.** The holder-wide set can contain skipped and kept. Only kept can reach the body because a sibling empty pattern throws on null. Preserve the runtime negative and record the extra name-only target. |
| `x4-12` | a yield expression passes the resume value to the holder | correct | RED | E value | **Loss.** A yield expression receives its value from later generator resumption. No target from resumed, outward or priming under the static value rule. |
| `x4-13` | replacing a callbacks call property preserves its direct call target | correct | RED | E stability | **Set.** An assignment to cb.call changes a property, not the cb binding. Require the direct handler payload and no borrowed handler payload from cb.call(via). |
| `x4-14` | a tagged holder receives its callback after the template object | correct | RED | E source | **Loss.** Retain the implicit template-object slot in runtime truth. Tags do not supply an ordinary call argument list to #434. No new target or payload from the tagged invocation. Preserve decoy negatives. |
| `x4-15` | destructuring reads nonenumerable callbacks that object spread omits | correct | RED | E value | **Loss.** Property descriptors and enumerability affect direct and copied objects differently. No static literal witness for direct or fallback in this fixture. Never infer hidden. The reviewed matrix skips this row. |
| `x4-16` | omitted and explicit undefined create different mapped arguments entries | correct | RED | E stability | **Loss.** Both actuals are opaque pick(...) results. Neither a missing argument nor explicit undefined is established for the receiving holder. No new changed/fallback targets. |
| `x4-17` | one unsafe target in a derived set refuses a bare formal source | wrong | RED | Consumers | **Keep.** Resolve cb to safe@8, unsafe@11 and check the other site set. bind 2 specified edges, including run.input → safe.item (nameonly/call_nameonly). 2 prohibited edges. check unreachability, reachability. |
| `x4-18` | jsx factory removes a callback prop before calling the component | correct | RED | E value | **Loss.** Runtime targets are fallback and direct. An admitted JSX Source record can project attribute by name, although h strips key. Keep direct, record any attribute candidate as wrong, and record the missed fallback from the opaque props forward. |
| `x4-19` | one callback target receives the payload at two different offsets | ambiguous | RED | E payload | **Loss.** Deduplicate handler once. The ordinary caller gives input → first only. The bound caller cannot supply its runtime offset one. No edge into second or third. |
| `x5-9` | partial destructuring closes a generator before the second formal | correct | RED | E value | **Loss.** Partial consumption and IteratorClose determine both values. No target from the iterator actuals, including head and fallback. Preserve the negative on tail. |
| `x5-10` | var in a simple catch initializes the catch binding | correct | RED | E stability | **Keep.** Resolve cb to supplied@8. bind run.input → supplied.item (nameonly/call_nameonly). 3 prohibited edges. |
| `x5-11` | awaited function and its promise are different actuals | correct | RED | E value | **Loss.** A Promise object is not supplied's callable identity. The awaited call result is also opaque. Neither supplied nor direct is derivable in this fixture. |
| `x5-12` | reflect set on mapped arguments respects the receiver | correct | RED | E stability | **Loss.** A Reflect.set write to the live mapped object can change cb. A distinct data-write receiver preserves it. Without a proved receiver the caller cannot justify keeping the original. Retain the reviewed reflection skip, costing the separate arm. The reviewed matrix skips this row. |
| `x5-13` | holder passed to itself rebinds two of its own formals | correct | RED | E forwarding | **Set.** The direct Source caller passes run itself as a concrete callable. Keep it once and bind swapped payloads with the derived label. The recursive derived call contributes no new grounding. |
| `x5-14` | method decorator replaces the function passed to a holder | correct | RED | E value | **Loss.** The decorator can replace Registry.handle. No target from the class member or its decorator result. The undecorated method is a decoy. |
| `x5-15` | callback identity test narrows two derived sites in one holder | ambiguous | RED | E cap | **Loss.** Both sites use the same holder slot. Require the NameOnly set {first, second} at each and record the two extra arms. The runtime guards separate them. |
| `x5-16` | receiver dependent default is skipped by an explicit callback | correct | RED | E value | **Loss.** Keep explicit only. An omitted callback with this.handler as its default requires instance state. No target chosen or skipped from guessing receiver identity. |
| `x5-17` | jsx child overrides spread children only on the paired tag | correct | RED | E value | **Loss.** The two concrete targets come from spread-sourced children and a factory-consumed child. Neither is an explicit winning JSX attribute. No JSX-derived kept, child or replaced target under #434. |
| `x5-18` | fanout drop removes the only bridge to a refusal seed | wrong | RED | Consumers | **Loss.** Nine distinct targets remove the whole derived set. Keep the fanout counter, the existing foreign seed and the corresponding loss of the bridge. Reason about reachability after the drop. |

## Round 1 fixture evidence

The reviewed matrix SHA above was read as input. Native probes used its in-memory build path, `ParsedFile::parse` and `CodePropertyGraph::build`, with the local checkout libraries on design base `3b02a47e` and A `2d67a0e0`. Offsets are half-open UTF-8 bytes. These are prerequisite observations; no B–E product head exists in this receipt.

**Receiver byte receipt (design base).** JS, TS and TSX agreed. For each receiver spelling below, the sole Use with that base path on the line is zero-width at line start; no complete-path or callee projection qualifies at the indented receiver. The miss persists even before any D label or trace test runs.

| Original | Expected token bytes | Actual base Use | Fixture action |
|---|---|---|---|
| h13 | arr 76..79 | main.arr 74..74 | Put the call at column 1. Use the existing spanned endpoint with path arr and empty text at 74. The observed arr Def 60..63 reaches this node. |
| x3-8 | items 91..96 | main.items 89..89 | Give items and direct separate const definition lines, put the call at column 1, keep both calls on that line. This replay has items Def 64..69 → Use 97..97 and direct Use 125..131 → visit.item Def 15..19 Exact. On the base, the separate direct Def 85..91 reaches the formal and its body Use 29..33. Select the zero-width Use by its actual bytes. |
| s4-9 | arr 122..125 and 137..140 | main.arr 120..120 only | Move the first call to column 1 to obtain the represented map receiver at 120..120; select it with empty text. The second arr still has no node. Keep its runtime reduce edge as #2 loss. |

**Indented receiver companion (D, expected loss).** Pair the repaired column-1 x3-8 with the same fixture and independent variable roots, inserting two ASCII spaces before `items.forEach(visit); visit(direct);`. Keep the original h13 byte pin (line start 74, receiver 76..79) as the corresponding literal-neighbour control. At the indented site pin the existing line-start Use, the distinct receiver span, the absence of both a full receiver Use and a callee projection, and the callback Def. The missing `items → visit.item` D edge is the expected loss. A missing fixture endpoint is a setup error. Check the absence from the enumerated same-owner Uses and prohibit a borrowed Exact edge. Require the same-line direct argument’s independent Exact edge and descent. Only the column-1 represented edge can exercise the D `not_exact` boundary. This companion is additional to the 425 original IDs.

At D and E2 use an integrated trace test for the reason and both root-specific frontiers. Assert the labelled edge first, then its boundary reason and the direct variable’s descent to the same formal and body Use. For e14 keep `cb(x)` and replace the literal neighbour with `handler(direct)`, sourcing direct from `const direct = read();` on the preceding line. On the base, direct Def 30..36 reaches handler.p Def 92..93 and its body Use 103..104, while x Def 17..18 reaches only its own payload Use 52..53 in that arm. For s3-14 use `const payload = input;` on its own line and `input.forEach(handler); cb(payload);` starting at column 1. The repaired base has receiver 65..65 and payload 92..99. The base trace from input Def 27..32 reaches both occurrences. Repeat this check on the feature head. Pin actual receiver bytes for every mixed-row fixture before enabling a receiver assertion. For first-on-line receivers the same column-1 repair and a spanned empty-text endpoint represent s2-15, s4-13, x2-11 and x4-9. A syntax/endpoint mismatch cannot satisfy an expected loss. The negative at a missing receiver cannot borrow an edge from the neighbouring occurrence.

**Runtime controls.** In this fold Node v26.0.0 matched all 12 listed Object.prototype keys on an ordinary object, an own undefined override, a null-prototype miss and a nested ordinary lookup (48 reads). It also matched seven descriptor/prototype controls, six empty/nonempty array, string and template spreads and mutation of an empty array through an alias. The independent checker and production gates remain pending.

**Mixed fixed/rest positives (B).** Scanning the group-B edge destinations gives b02a, b02c, b04, t4-3, s1-7, s2-4, s3-4, x1-5, x3-4 and x4-3 (the last uses two call sites). On A the fixed edge already exists in b02a/c, b04, s3-4, x3-4 and x4-3. x1-5 loses the fixed literal element, s1-7 loses the fixed slot past a destructure, and s2-4 independently lacks a parameter-property target. The public call-site dump on both bases also lacks a caller site for t4-3 and s2-4.

**Constructor prerequisites (B/E).** The round 2 reviews found zero sites resolving to a constructor FunctionId in X and T. Of 510 `new X(` occurrences counted in X, 499 lacked a site. Pin the integration-base Source origin, file, full new-expression span, caller, sole Exact target and constructor implementation for each ordinary new below. Without that prerequisite record each required end-to-end positive as blocked/RED. Exercise the same arguments and declarations independently by supplying the byte-bound site and target to the binding or E caller-index test. Check the independent negatives there as well as on the real graph. No synthetic caller enters the product. A helper pass supplies no extraction or resolution credit.

| Row | Real-site positive ordinals | Supplied-site checks and independent controls |
|---|---|---|
| t4-3 | #2 rest. #1 fixed shares the missing site and retains its recorded loss. | Supply C. Require x → first, y → rest and #3 no y → first. |
| s2-4 | #2–3 rest. Fixed #1 also lacks a parameter-property Def. | Supply Packet. Require count and text → parts and #4 no label → parts. Preserve #1's separate Def loss even with a site. |
| s3-3 | #1 last → Derived.items | Supply Derived. Require #1 and all three negatives (#2–4), including no last → Base.parts and no super arguments flowing back to Derived. |
| s1-25 | #1 target, #2 payload | Supply new Runner(handler, input) as an Exact Source caller of Runner’s constructor. Require the handler target and constructor.input → handler.p. With absent or different holder evidence require neither contribution. |
| s5-6 | #2 implicit new Derived needs an admitted Base constructor implementation | Supply that Base target at this site and require value → tail. Require #1’s Def independently and prohibit binding to a different implementation. |
| x3-5 | #1 explicit new Explicit and #2 implicit new Implicit, with separate implementation pins | Supply Explicit and Base respectively. Require lost → Explicit.forward and kept reaching Base.parts. Check each site’s actual against the other site’s formal at the binding step. Preserve the reviewed removal of the whole-walk negative. |
| x5-3 | #3 ordinary new Shape, inside an existing whole-row skip | Exercise direct → Shape.other in a focused supplied-site test with #4 no direct → Target.items. Preserve the #2 reflection decoy prohibition and the existing skip. |

The active rows contain **9 required positive ordinals in 6 IDs** dependent on these pins (21 assertions across the specified languages), plus **2 fixed-slot loss ordinals** sharing the missing site (11 affected positive ordinals). The x5-3 #3 prerequisite is inside a seventh, skipped ID. Report actual blocked counts on the integration base by ID/ordinal and language, alongside fixed-loss overlaps, skips, supplied-site results and independently run negatives. Only a dump proving that site and implementation can unblock its real-site requirement. s3-22’s `new cb(input)` has its own parked construct-route loss. A constructor spelling in setup, a decoy or a prohibition needs no ordinary-constructor positive. **Supplied-site seam (round 3, B):** every supplied-site test supplies, together with the call site and its Exact target, the call's argument nodes and byte spans: the pinned JS, TS and TSX grammars expose a `new` expression's callee as `constructor`, so the argument index of slice A's call-identity lookup records no arguments for it and Step 5b would skip even a supplied site. The seam verifies B's rest binding and E's caller contribution independently of extraction; the end-to-end REDs stay.

**Opaque rest operands (B).** For the unproved formals and member above, replace b02b #1, b02c #2, s2-6 #1, s3-4 #2/#4 and s4-4 #3 with same-endpoint prohibitions on the withheld operand edge. Preserve the scalar, fixed and crossed siblings. Require the Exact edge for a stable, demonstrably yielding operand at the same rest offset and admitted singleton callee. Use the zero/one-yield generator, empty Set, empty array iterator and sliced-empty controls in §5. Record independently witnessed zero delivery as WRONG if a transfer was emitted. Price all withheld operand edges, including those at sites that still have a scalar edge.

**Resolver route inventory (C).** Joined each positive edge's source occurrence to its call and inspected the complete candidate pool on A, for every advertised language. These entries cover all group-C rows containing positive edge assertions and the two counter controls. Existing Defs, prohibitions and externally checked outcomes retain their own assertions.

| Route / prerequisite | Matrix members and disposition |
|---|---|
| Singleton local_def | k01a/b, k05a (literal read), k07b (rest and bare), t4-4, s1-9, s1-11, s4-6, x3-6 (literal read), first x3-7. k08a/b exercise the local call’s return-flow counters. |
| Singleton value-space import | k07a, s1-10 (renamed re-export), s2-8 (named lexical default), s4-7 (CommonJS), x4-5/6 are import_member. s3-8 is import_qualified. Pin the namespace export `project` to the selected bare arrow’s declaration. Use this genuine export as the positive for the two private-decoy controls in DESIGN §5. |
| Missing route | s3-7 anonymous default export and s5-11 raw escaped spelling drop UnknownName. Both x2-6 literal IIFEs drop UnknownName with the parenthesized expression as callee. Keep independent Def and decoy assertions. |
| Multiple candidates | s5-9 has two same-file NameOnly targets per call; x3-7’s second local_def call has two Exact entries. Record the missing Exact and use the actual pool for the negatives. |
| Previously accepted boundaries and controls | x1-8 snapshot/live both unresolved; x1-9 direct local succeeds, getter arm lacks the returned target; x5-5 CommonJS replacement unresolved; x5-6 local succeeds, global receiver unresolved. Preserve the reviewed skips. x2-7 pins existing outgoing import_member and foreign free_single edges plus their independent refusal pathways. |

For B/C singleton `import_qualified`, pair a private `function readFile(...parts)` and a private `const readFile = item => item` (separate fixtures) with `export { live as readFile }` in the same module. Pin the namespace import, selected pool, independent export declaration and both declaration spans. Withhold every new edge to the private declaration, including bare literal leaves. Keep the actually exported rest-function and bare-arrow controls. Enumerate canonical-path fallback singletons on the §7 M2/M3 inputs by these dispositions before activating B or C.

**Per-assertion ledger for the test-only matrix PR.** Use 1-based ordinals in the digest above. A `loss` is a nonempty reason attached to a particular semantic assertion. Run it at its active slice and report expected semantic mismatch with observed endpoints/labels/targets. An unexpected pass is an error (review the loss); malformed input, missing required setup, panic or wrong discrepancy is a test error. A zero edge count at a valid, documented unsupported endpoint can be the expected result. Keep strict schema validation and all mandatory sibling assertions. A passing negative with a separate runtime omission needs a ledger entry only. Existing reviewed whole-row skips remain explicit. Hard sentinels never take a loss marker.

In the three group-C rows below, record explicit first-active B ordinals and keep the remaining ordinal at C. Verify B alone and B+C against the same assertion objects.

| ID | B | C |
|---|---|---|
| k06a | #2 prohibit an edge into parseInt.vals | #1 pin the historical free_single resolution |
| k07a | #1 imported v → parseInt.vals | #2 imported v → escape.s |
| k07b | #1 local v → parseInt.vals | #2 local v → escape.s |

| Mixed ID | D assertions (ordinal) | E2 assertions (ordinal) |
|---|---|---|
| s3-14 | 1, 3, 5 (reachability from receiver) | 2, 4; repeat 5 with both routes |
| s4-12 | 1 | 2–4 |
| s4-13 | 3–4 | 1–2 with grounded Box.map |
| x2-11 | 1–3 | 4–7 |
| x4-9 | 1–2 | 3–6 |
| s2-15 | 1–3 | appended #4 target/drop and #5 payload assertion, fixed from the independent admission pin |

Pin holder admission from an independent base site record. On this base s4-13's map is singleton Exact; s2-15's then drops UnknownName. Assert the appropriate fixed E2 expectation for each pinned fixture and include an admitted thenable-method companion requiring actual reason delivery. Changing the prerequisite fails its pin and requires regrading. For the existing s2-15 use the in-body fail site's original UnknownName drop and no call_nameonly payload from that site. In the companion place the in-body call on a uniquely admitted holder with a represented local reason Use; require both the argument_follow target and the matching payload. A callback target produced by E is never its own precondition.

Besides the row-specific additions above, the fixture PR uses the following expected mismatches from the runtime ledger, after binding the endpoints on the integration base. Ordinals specify individual assertions, including single-target `resolves` and crossed prohibitions where the model emits a union.

| Expected discrepancy | Original row and assertion |
|---|---|
| Missing slot, receiver or construct payload | x1-5 #1, x1-7 #1–2, x2-5 #1, x4-7 #1–2, s3-22 #2, s5-28 #1 |
| Table candidate violates a runtime negative | x1-12 #2, x4-8 #2, x5-7 #2. Require the supported neighbour and preserve Exact prohibitions. |
| Absent target and payload | x1-19, x1-22, x2-12/13/14/20, x4-12, x5-11/14 each #1–2; x3-14 and x4-10/16 each #1–4; x4-14 #1–3; x5-9 #1–3,5; x5-17 #1,2,4 |
| Different target set or adapter slot | x4-11 #1,4; x5-15 #1,2,7,8. Assert the actual allowed union separately. x4-18 #1,2 and #6 if the JSX attribute ground exists; x4-19 #3; x5-16 #1,2. Preserve the independent supported target and payload checks. |
| Corrected product assertion | t3-3 #2 becomes the omitted-seed positive, s1-17 #2 becomes the single rest positive. Neither takes a loss marker. Apply the other Set and seven runtime rulings by endpoint and label. |

Use the complete Loss/Set table, including documented exclusions without a contradictory assertion, to generate the changed-assertion manifest in the test-only PR. Reconcile by row, ordinal, language and original assertion contents; classify every assertion as required (with blocked/RED prerequisite status where applicable), corrected, staged, expected loss or existing explicit skip, and record the observed first-applicable result for every changed member. Never convert a newly failing assertion into a loss based solely on its failure. For a conditionally witnessed product excess, pin the actual pool/representation first; an absent prerequisite requires an explicit disposition and a supported positive control.

## Census costs and the limits of each count

Triples are X / T / SB, as recorded by `census/M{1,2,4,5}/<corpus>/aggregate.json`. “Open reasons” in M5 can overlap or coexist with a successful target. Counting a reason does not count a lost CORRECT site.

| Decision | Measured population and unit | Interpretation for the differential |
|---|---|---|
| A offsets and call identity | M1 trailing-spread sites 3/3/4, all dropped, zero known-length and zero post-spread rows. SB 26 outside-span rows at 7 sites. The grader called 18 unproved and 8 chain, with zero correct or proved-wrong. Builtin call/apply trailing-spread sites 0/0/0. | Use the census category and grader class separately. Fifteen SB attributions were ambiguous. At the census's resolved trailing-spread sites the existing post-spread literal-leaf cutoff in s1-1/s5-3 cost zero observed conversions (all ten sites dropped). The ten sites do not enumerate terminal spreads or every method called `call`. |
| B rest | M2 rest Def sites 56/1,146/272, without a rest Def 0/0/0 in its identifier-rest population. No eligible Use 50/831/110. Cross-file with a Use 0/1/0. Admitted 6/314/162, including 1/40/12 spread-operand-only. | The eligible and excluded counts sum to 1,474 identifier-rest sites. A rest pattern or synthetic call is outside this denominator. No admitted site had a spread before its rest index or its eligible argument. M2 does not measure a marginal count for dense terminal leaves, nested floors or exceptional initializers. |
| C lexical and label admission | M3 0/68/127 added. Disjoint ordinary exclusions 0/14/0 cross-file FreeSingle Exact, 0/48/2 cross-file NameOnly and 0/3/0 multi-Exact (local_def and import_qualified). | 14 UNDECIDED in the first class. In the second, T 6 WRONG + 42 UNDECIDED, SB 2 UNDECIDED. The third contains T#5/#58/#59, all UNDECIDED. The three predicates withhold zero CORRECT. They retain 128 rows (116 CORRECT, 12 UNDECIDED) before the separate declaration-witness check. The original SB 104 all-CORRECT sites is a forecast. |
| D receiver | M4 receiver absent 903/748/8,463. Missing identifier 377/296/2,479, member 199/204/1,652, this_path 18/50/1,071 (6,346 sites together), call-result form 163/106/2,208, chain 89/32/801, literal 47/54/199, other 10/6/53. Conversions by receiver form identifier 390/258/2,610, member 31/63/558. | 2,455 ordinary path Uses, 1,452 projection Uses and 3 zero-width Uses supply all 3,910 conversion candidates. Receiver-absent is counted after a read Def exists. Do not infer a node from an argument inside the receiver expression. |
| D callback identity and roles | M4 callback not admitted 33/88/1,586, read Def unobserved 120/114/908, same-file named conversions 4/21/63, bare conversions 1/253/155. Whole-array reads 7/6/129, thisArg reads 0/0/115, reducer names 93/11/638. | No separate counts for imported vs mutable vs factory callbacks, for omitted initializer vs supplied, or for Map/Set second-role reads. Compare the named live-binding proof on M4's 88 candidate keys and report the delta. Missing graph nodes, invocation timing and unknown receiver types are different facts. No new runtime-type, species, sparse-array or factory blacklist. |
| D method heuristic and descent | M4 library/repo/no-declaration 339/0/82, 316/5/0, 681/15/2,472. Role sample C/W/U 150/0/50, 3/0/197, 47/0/153. D 3,910 candidate bindings and E 3,782 proposed binding rows supply the two doubtful label populations. | 98.5% library among typed covers declarations. Preserve the historical sample. The head oracle counts a witnessed contrary override delivery as WRONG even under a library signature. An opaque override is UNDECIDED. Lane 8 I-guard reports actual trace losses. M4/M5 contain no count of paths an Exact walk would have borrowed from a neighbour. |
| E Source callers and gates | M5 refused foreign holder callers 1/49/234, sites affected 2/1/9. Existing targets 0/149/356. No admitted target 363/626/18,633. Conversion dispositions lacking payload after target selection 127/206/440. Def unobserved 7/14/68. | E may resolve a Source site without a payload edge. M5's independent `no_payload_use` tally 440/364/13,660 includes other dispositions. The two tallies have different denominators. Exported-or-escaping 357/400/5,099 carries no blanket exclusion. |
| E formal stability | M5 parameter sites with reassigned formal 1/4/1,455. Open-reason `param_reassigned` 3/4/1,943 includes follow/forward paths. | Bound and report the cost of the existing #434 no-write rule at this grain. The aggregate does not split direct, captured, eval, mapped arguments or delegated writes. In E1 count these under stability with actual lost caller contributions. A narrower subtype has no independent census total. |
| E paths and values | M5 destructured sites 359/89/1,326, with default 1/5/4 in the separate `destructured_default` class, body destructures 8/152/6. Open reasons include `pattern_mismatch` 3/6/0, `object_arg_not_literal` 0/31/0, `spread_object` 1/2/0, `destructured_local` 14/27/2, `jsx_spread` 4/0/0 and `jsx_children` 1/0/0. | Static literal paths and explicit winning JSX attributes set the proof boundary. An absent own key and an opaque value have different default behaviour. M5 has no marginal count for a new factory-specific refusal or a new heap/iterator analysis. |
| E opaque terminals and adapters | M5 open reasons `member` 10/63/112, `call_result` 13/99/588, `bind_result` 0/22/17, `let_var_reassigned` 0/9/197, `let_var` 0/0/376. Parameter-dot-call sites 3/4/1,100, dot-apply 2/3/953 and element/arguments sites 0/0/2. | These are the #434 model boundaries used by the named losses, including e06d and e11. None establishes the number of genuine transfers an adapter, getter, decorator or coroutine would recover. M6 counted 3 implicit arguments-object first breaks. |
| E forwarding, cap and identity | Open reasons cycle 0/14/251, depth 0/2/0. Fanout 12/48/75, with payload 6/37/62. Sites with anonymous callers 16/73/279 and bare target Defs 0/162/14 among conversions. | Costs are counted at the slot/site, after deduplication. A rejected caller consumes zero slots. A no-ground cycle and a cycle with an external ground have different outcomes. |

M4 counted only the closed method table. It gives no standalone yield for adding findLast or Array.from (s1-15, s5-14). Its receiver, callback and read-Def counts measure the eligible population. Neither fixture asks the product to remove an existing edge. For shared literal leaves, the producer records 44 wrong NameOnly singletons in its earlier census. M5's 3,782 proposed rows describe Use → Def only.

The counts above measure the rules at the grain stage M recorded. No subtype inherits an invented zero. Keep existing evidence boundaries for holes outside those populations. A proposed further product refusal needs a before/after cost on the pinned corpora, eligible keys and lost credited rows before adoption. The lack of such a cell is why the design records named losses for runtime-sensitive D/E examples instead of installing a new family of filters.

## M3 UNDECIDED rows

Each key below is the 1-based record in the named grade file, followed by file and half-open UTF-8 byte range of both endpoints. Each listed row earns **zero** credit. T has 19 Exact and 43 NameOnly with `runtime_callee_unwitnessed`. SB has 9 Exact with `argument_value_unwitnessed` and 2 NameOnly with `runtime_callee_unwitnessed`. Six additional T rows, records 40–43 and 52–53, are WRONG with `callee_declaration_mismatch` and fall under C's cross-file NameOnly refusal.

| Grade key | Source bytes | Target bytes | Probe label | Product disposition |
|---|---|---|---|---|
| T#1 | `compiler/program.ts@105215..105229` | `compiler/resolutionCache.ts@32311..32315` | nameonly | Refuse ordinary foreign NameOnly |
| T#2 | `compiler/program.ts@105215..105234` | `compiler/resolutionCache.ts@32311..32315` | nameonly | Refuse ordinary foreign NameOnly |
| T#3 | `compiler/program.ts@107040..107060` | `compiler/resolutionCache.ts@32311..32315` | nameonly | Refuse ordinary foreign NameOnly |
| T#4 | `compiler/program.ts@107040..107065` | `compiler/resolutionCache.ts@32311..32315` | nameonly | Refuse ordinary foreign NameOnly |
| T#5 | `compiler/transformers/declarations.ts@11779..11804` | `compiler/transformers/declarations.ts@17928..17929` | exact | Withhold multi-Exact local_def (three targets) |
| T#6 | `compiler/program.ts@122449..122462` | `compiler/resolutionCache.ts@32311..32315` | nameonly | Refuse ordinary foreign NameOnly |
| T#7 | `compiler/program.ts@122449..122467` | `compiler/resolutionCache.ts@32311..32315` | nameonly | Refuse ordinary foreign NameOnly |
| T#8 | `compiler/emitter.ts@122617..122621` | `services/textChanges.ts@69186..69190` | exact | Apply the recorded FreeSingle site flag¹ |
| T#9 | `compiler/emitter.ts@125804..125808` | `services/textChanges.ts@69349..69353` | exact | Apply the recorded FreeSingle site flag¹ |
| T#10 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/moduleResolution.ts@20769..20777` | nameonly | Refuse ordinary foreign NameOnly |
| T#11 | `compiler/watchUtilities.ts@13346..13354` | `harness/evaluatorImpl.ts@2320..2324` | nameonly | Refuse ordinary foreign NameOnly |
| T#12 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/tsserver/watchEnvironment.ts@27377..27378` | nameonly | Refuse ordinary foreign NameOnly |
| T#13 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/moduleResolution.ts@51590..51591` | nameonly | Refuse ordinary foreign NameOnly |
| T#14 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/moduleResolution.ts@53487..53488` | nameonly | Refuse ordinary foreign NameOnly |
| T#15 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/moduleResolution.ts@54769..54770` | nameonly | Refuse ordinary foreign NameOnly |
| T#16 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/programApi.ts@5753..5761` | nameonly | Refuse ordinary foreign NameOnly |
| T#17 | `compiler/watchUtilities.ts@13346..13354` | `services/transpile.ts@6510..6518` | nameonly | Refuse ordinary foreign NameOnly |
| T#18 | `compiler/watchUtilities.ts@13346..13354` | `testRunner/unittests/customTransforms.ts@894..902` | nameonly | Refuse ordinary foreign NameOnly |
| T#19 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/moduleResolution.ts@20769..20777` | nameonly | Refuse ordinary foreign NameOnly |
| T#20 | `compiler/watchUtilities.ts@14552..14560` | `harness/evaluatorImpl.ts@2320..2324` | nameonly | Refuse ordinary foreign NameOnly |
| T#21 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/tsserver/watchEnvironment.ts@27377..27378` | nameonly | Refuse ordinary foreign NameOnly |
| T#22 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/moduleResolution.ts@51590..51591` | nameonly | Refuse ordinary foreign NameOnly |
| T#23 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/moduleResolution.ts@53487..53488` | nameonly | Refuse ordinary foreign NameOnly |
| T#24 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/moduleResolution.ts@54769..54770` | nameonly | Refuse ordinary foreign NameOnly |
| T#25 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/programApi.ts@5753..5761` | nameonly | Refuse ordinary foreign NameOnly |
| T#26 | `compiler/watchUtilities.ts@14552..14560` | `services/transpile.ts@6510..6518` | nameonly | Refuse ordinary foreign NameOnly |
| T#27 | `compiler/watchUtilities.ts@14552..14560` | `testRunner/unittests/customTransforms.ts@894..902` | nameonly | Refuse ordinary foreign NameOnly |
| T#28 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/moduleResolution.ts@20769..20777` | nameonly | Refuse ordinary foreign NameOnly |
| T#29 | `compiler/watchUtilities.ts@15253..15258` | `harness/evaluatorImpl.ts@2320..2324` | nameonly | Refuse ordinary foreign NameOnly |
| T#30 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/tsserver/watchEnvironment.ts@27377..27378` | nameonly | Refuse ordinary foreign NameOnly |
| T#31 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/moduleResolution.ts@51590..51591` | nameonly | Refuse ordinary foreign NameOnly |
| T#32 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/moduleResolution.ts@53487..53488` | nameonly | Refuse ordinary foreign NameOnly |
| T#33 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/moduleResolution.ts@54769..54770` | nameonly | Refuse ordinary foreign NameOnly |
| T#34 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/programApi.ts@5753..5761` | nameonly | Refuse ordinary foreign NameOnly |
| T#35 | `compiler/watchUtilities.ts@15253..15258` | `services/transpile.ts@6510..6518` | nameonly | Refuse ordinary foreign NameOnly |
| T#36 | `compiler/watchUtilities.ts@15253..15258` | `testRunner/unittests/customTransforms.ts@894..902` | nameonly | Refuse ordinary foreign NameOnly |
| T#37 | `compiler/emitter.ts@153374..153378` | `services/textChanges.ts@69186..69190` | exact | Apply the recorded FreeSingle site flag¹ |
| T#38 | `compiler/emitter.ts@153880..153884` | `services/textChanges.ts@69349..69353` | exact | Apply the recorded FreeSingle site flag¹ |
| T#39 | `compiler/utilities.ts@17408..17409` | `compiler/utilities.ts@16961..16965` | exact | Retain, zero credit |
| T#44 | `compiler/emitter.ts@188638..188647` | `services/textChanges.ts@69524..69529` | exact | Apply the recorded FreeSingle site flag¹ |
| T#45 | `compiler/emitter.ts@190408..190417` | `services/textChanges.ts@69700..69705` | exact | Apply the recorded FreeSingle site flag¹ |
| T#46 | `compiler/emitter.ts@198521..198529` | `services/textChanges.ts@69524..69529` | exact | Apply the recorded FreeSingle site flag¹ |
| T#47 | `compiler/emitter.ts@198568..198576` | `services/textChanges.ts@69700..69705` | exact | Apply the recorded FreeSingle site flag¹ |
| T#48 | `compiler/emitter.ts@198952..198960` | `services/textChanges.ts@69524..69529` | exact | Apply the recorded FreeSingle site flag¹ |
| T#49 | `compiler/emitter.ts@199640..199648` | `services/textChanges.ts@69700..69705` | exact | Apply the recorded FreeSingle site flag¹ |
| T#50 | `compiler/emitter.ts@208435..208439` | `services/textChanges.ts@69870..69874` | exact | Apply the recorded FreeSingle site flag¹ |
| T#51 | `compiler/emitter.ts@208560..208564` | `services/textChanges.ts@70035..70039` | exact | Apply the recorded FreeSingle site flag¹ |
| T#54 | `testRunner/unittests/tscWatch/watchApi.ts@24496..24503` | `testRunner/unittests/helpers/baseline.ts@1012..1019` | exact | Retain singleton local_def, zero credit |
| T#55 | `testRunner/unittests/helpers/virtualFileSystemWithWatch.ts@46302..46311` | `testRunner/unittests/helpers/virtualFileSystemWithWatch.ts@5851..5860` | nameonly | Retain, zero credit |
| T#56 | `compiler/emitter.ts@61982..61986` | `services/textChanges.ts@69186..69190` | exact | Apply the recorded FreeSingle site flag¹ |
| T#57 | `compiler/emitter.ts@62364..62368` | `services/textChanges.ts@69349..69353` | exact | Apply the recorded FreeSingle site flag¹ |
| T#58 | `harness/tsserverLogger.ts@6598..6605` | `harness/harnessIO.ts@4511..4515` | exact | Withhold multi-Exact import_qualified |
| T#59 | `harness/tsserverLogger.ts@6598..6605` | `harness/harnessIO.ts@65519..65523` | exact | Withhold multi-Exact import_qualified; runtime decoy |
| T#60 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/moduleResolution.ts@20769..20777` | nameonly | Refuse ordinary foreign NameOnly |
| T#61 | `compiler/watchUtilities.ts@9104..9112` | `harness/evaluatorImpl.ts@2320..2324` | nameonly | Refuse ordinary foreign NameOnly |
| T#62 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/tsserver/watchEnvironment.ts@27377..27378` | nameonly | Refuse ordinary foreign NameOnly |
| T#63 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/moduleResolution.ts@51590..51591` | nameonly | Refuse ordinary foreign NameOnly |
| T#64 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/moduleResolution.ts@53487..53488` | nameonly | Refuse ordinary foreign NameOnly |
| T#65 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/moduleResolution.ts@54769..54770` | nameonly | Refuse ordinary foreign NameOnly |
| T#66 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/programApi.ts@5753..5761` | nameonly | Refuse ordinary foreign NameOnly |
| T#67 | `compiler/watchUtilities.ts@9104..9112` | `services/transpile.ts@6510..6518` | nameonly | Refuse ordinary foreign NameOnly |
| T#68 | `compiler/watchUtilities.ts@9104..9112` | `testRunner/unittests/customTransforms.ts@894..902` | nameonly | Refuse ordinary foreign NameOnly |
| SB#36 | `prototype-pollution/ion-parser_0.5.2:parser.js@8734..8737` | `prototype-pollution/ion-parser_0.5.2:parser.js@9489..9492` | exact | Retain, zero credit |
| SB#37 | `prototype-pollution/ion-parser_0.5.2:parser.js@9133..9138` | `prototype-pollution/ion-parser_0.5.2:parser.js@9489..9492` | exact | Retain, zero credit |
| SB#82 | `prototype-pollution/think-helper_1.1.0:index.js@6025..6026` | `prototype-pollution/think-helper_1.1.0:index.js@5894..5895` | exact | Retain, zero credit |
| SB#83 | `prototype-pollution/think-helper_1.1.0:index.js@6055..6056` | `prototype-pollution/think-helper_1.1.0:index.js@5894..5895` | exact | Retain, zero credit |
| SB#84 | `prototype-pollution/think-helper_1.1.0:index.js@6080..6081` | `prototype-pollution/think-helper_1.1.0:index.js@5894..5895` | exact | Retain, zero credit |
| SB#85 | `prototype-pollution/think-helper_1.1.0:index.js@6106..6107` | `prototype-pollution/think-helper_1.1.0:index.js@5894..5895` | exact | Retain, zero credit |
| SB#86 | `prototype-pollution/think-helper_1.1.0:index.js@6134..6135` | `prototype-pollution/think-helper_1.1.0:index.js@5894..5895` | exact | Retain, zero credit |
| SB#95 | `redos/is-svg_4.2.2:index.js@908..913` | `redos/is-svg_4.2.2:index.js@619..622` | exact | Retain, zero credit |
| SB#104 | `redos/react-native_0.63.0-rc.0:Libraries/Components/Picker/PickerIOS.ios.js@4038..4064` | `redos/react-native_0.63.0-rc.0:ReactAndroid/src/androidTest/js/PickerAndroidTestModule.js@1723..1728` | nameonly | Refuse ordinary foreign NameOnly |
| SB#105 | `redos/react-native_0.63.0-rc.0:Libraries/Components/Picker/PickerIOS.ios.js@4038..4043` | `redos/react-native_0.63.0-rc.0:ReactAndroid/src/androidTest/js/PickerAndroidTestModule.js@1723..1728` | nameonly | Refuse ordinary foreign NameOnly |
| SB#126 | `redos/terminal-kit_2.1.0:lib/inputField.js@11507..11513` | `redos/terminal-kit_2.1.0:lib/inputField.js@11668..11675` | exact | Retain, zero credit |

¹ The T grade file records edge confidence and endpoints. It has no site resolution kind. Of 19 T Exact rows, two are same-file. Stage M identifies 14 of the remaining 17 as cross-file FreeSingle, leaving three foreign Exact from other routes. Rejoin the product site flag by full identity on replay. T#5 at declarations.ts:355 resolves local_def to the three arrows at lines 449, 1328 and 1668. Withhold #5 and the import_qualified pair #58/59. Retain same-file #39 and singleton local_def #54 at zero credit. The other 14 foreign Exact have the recorded FreeSingle flag. All 19 retain their historical UNDECIDED grades.

## Fixture repairs and distinct near neighbours

The coordinate repair in t4-2 selects `v` at 5:34. Retain all 12 pairs from the receipt. No pair is an exact duplicate. The cases distinguish one spread vs two (a06c/t1-1), unknown vs literal nested operand (a06f/t1-4), TS wrapper forms (a06g/t4-2), trailing argument vs spread element (b09c/s5-7), omission vs undefined (e09b/t1-6), supplied vs omitted reducer seed (h02a/t3-3), trace-only vs explicit direct edge (h13/x3-8), ordinary vs known-spread floor contribution (t3-2/s1-6), strict vs non-simple parameters (t3-8/t4-6), distinct same-file vs cross-file spans (s1-4/s3-2), same-name chain layout (s5-1/x5-1), and distinct sites vs repeated Uses within one site (x3-3/x4-2). Use the round 1 receiver byte receipt for h13 and x3-8. The label repair in 5767ceb8 addresses e09a's old adapter failure. Obtain a same-base replay before crediting a product change.

Reviewed the slice at `cd457e31` and merged head `b6bf3aa7`. Two material correctness defects require changes. These are source-traced findings; I ran no builds or tests, made no writes or GitHub comments, and did not open `eval/secbench`. Cloud results remain supplied evidence.

1. **WRONG / MATERIAL(a,c) — D deletes a live coalesced same-line definition. Confidence: 100%.**

   ```python
   def f():
       x = 1; x = 2
       sink(x)
   ```

   The removed pair is **`x`, def line 2 → use line 3**. It carries the value `2`.

   For named Python owners, the producer keys definition locations by path and line, retaining the **first** occurrence’s span ([file.rs:160](src/data_flow/build/file.rs:160), [file.rs:253](src/data_flow/build/file.rs:253)). Both raw candidates then reach D with that first span ([file.rs:563](src/data_flow/build/file.rs:563)). D selects the second assignment as the dominating write; its window excludes the first span, so **both candidates disappear**, including the live second assignment.

   This also explains why adding `global` to the two same-line store witnesses makes them pass: it disables D rather than establishing that their original removed edges were false positives.

   **Fix options:** evaluate every raw definition occurrence before coalescing and retain the emitted group whenever any member survives; or conservatively exempt multiply defined Python path/line groups from D. Preserve emitted identities and labels. Restore the original local same-line witnesses and assert the live pair survives. A separate exemption is unnecessary once pruning accounts correctly for every member of the coalesced group.

2. **WRONG / MATERIAL(a) — Attribute spelling does not establish identical storage or an unconditional overwrite. Confidence: 100%.**

   ```python
   def f(o, other):
       o.x = source()
       saved = o
       o = other
       o.x = 0
       o = saved
       sink(o.x)
   ```

   With two distinct ordinary objects, the removed pair is **`o.x`, def line 2 → use line 7**. The write on line 5 affects `other`; the original object still contains the source value.

   Python’s new classifier accepts attribute paths, and D selects writes solely by equal `AccessPath` and lexical position ([lexical_prune.rs:125](src/data_flow/build/lexical_prune.rs:125), [lexical_prune.rs:238](src/data_flow/build/lexical_prune.rs:238)). It checks neither receiver identity nor intervening receiver rebinding.

   Stable receivers are insufficient too: for `o.x = source(); o.x = None; sink(o.x)`, a property setter that ignores `None` leaves the first value intact. D nevertheless removes that first definition’s pair.

   **Fix options:** conservatively disable Python attribute D proofs, or require explicit proofs of receiver stability and ordinary, non-intercepted storage writes. A rebinding-only guard does not fix descriptors; a descriptor-only guard does not fix the example above. These restrictions need not apply to plain local-name assignments. Add both counterexamples as preservation obligations.

3. **SMELL / MATERIAL(c), nonblocking — Python cap/cache claims lack direct regression coverage. Confidence: 99%.**

   The new Python matrix checks uncached producer output and ordinary counter accounting. The cache/incremental custody test uses Go ([lexical_prune_tests.rs:235](src/data_flow/build/lexical_prune_tests.rs:235)); the survivor-count/replay tests likewise use Go ([prune_cap_tests.rs:27](src/data_flow/build/prune_cap_tests.rs:27)).

   The shared machinery looks consistent with the claimed behavior; this is not a demonstrated cache or cap bug. Parameterize those tests for Python, or add focused Python cases with positive U/D counts, cold/full/partial-hit equality, and below/equal/above survivor-cap transitions. Existing language-independent codec tests already cover codec invariants; those need no duplication.

D **can soundly remove an RD `exact` pair**. For example:

```python
def f():
    x = 1
    x = (
        2); consume_the_value_after_assignment(x)
```

Source tracing predicts `x@2 → x@4` can receive `exact`: RD maps a use’s **line** to the shortest statement spanning that line ([statement_lookup.rs:34](src/cpg/reaching/statement_lookup.rs:34)). Here the short multiline assignment wins over the longer call, placing the use at the assignment’s incoming state, where the old definition reaches. The classifier then returns `Exact` ([reaching.rs:788](src/cpg/reaching.rs:788)).

D uses byte ordering and correctly sees the completed overwrite before the call. That removal is sound; RD’s `exact` was false. This explains a possible mechanism, **not the pending corpus population**. Rule-splitting the cloud removals remains necessary, and neither the label nor the 80 sampled RIGHT removals resolves the two defects above.

The changed-oracle judgments are:

| Changed witness | Judgment |
|---|---|
| `dfg_label_test.rs` | Still tests killed/exact label attachment. The independent overwrite makes the original killed shortcut intentionally removable; `global` supplies a surviving witness. |
| Parity `MIXED` | Still tests labeled traversal and folding over exact/killed edges. Retargeting is reasonable. |
| `tests/common/mod.rs` | Still tests delivery of a selected killed hop. It no longer covers the ordinary local `clean(query)` case; retain that case separately to check continued flow. |
| `dfg_label_store_test.rs` | Still exercises conservative payload assertions, but the globals mask finding 1. The original local coalesced-flow cases must remain covered. |

Allowing Python to shrink in `lexical_prune_tests` is appropriate for this slice. Its subset and unchanged-label assertions remain useful, but cannot establish that removals preserve real flows.

For the four reported CI failures:

- **Provenance and taint grades:** D removes the direct `query@2 → sink@4` shortcut across `query = clean(query)`. The incoming read edge to line 3 and the new definition’s edge to line 4 remain. Source does not establish a lost real flow here; changing the label witness is reasonable while retaining the local consumer case.
- **Label-counter test:** zero killed edges is intended for `x = source(); x = clean(); sink(x)`. However, the loop stops at that assertion. Its later `dfg_reaching_same_line` fixture also loses its live coalesced pair through finding 1. Adding a global there would conceal the defect.
- **MCP eager-default:** U removes the artificial `user` Use on assignment-only line 2, eliminating its occurrence-order warning. The real chain `user Def@2 → user Use@3 → value Def@3 → value Use@4` remains. A golden update preserving that witness and `Reached` is justified.

I found no additional demonstrated output regression in non-Python admission, surviving-label handling, counter replay, or cache version activation. Wall-time compliance remains supported only by the supplied timing measurements. Round 2 should review bounded fixes on this artifact, especially the restored local same-line witnesses.

VERDICT: REQUEST_CHANGES



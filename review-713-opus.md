# Review of PR #713: Python rules U and D (#630 slice 5), round 1 of 2

Reviewer: Opus 5.5. This was a read-only review of `<local> at head b6bf3aa7, covering the slice diff `a2c9c9f4...cd457e31`. I ran no builds and no tests. Every claim below comes from reading the source, and none was executed.

**Goal (from the brief):** extend the removal-only rules U and D from Go to Python `function_definition` owners. No real flow may be removed. Surviving labels and other languages must stay unchanged, and the change must add no wall time.

**Summary:** U is sound as written. D has one provable lost-flow defect: same-line defs share one def location, and D judges that shared location instead of each def's own position. It also has a contract gap: an attribute path is treated as fixed storage. That holds up in Go, but in Python it breaks when the base is rebound or the attribute is a property. The four CI failures are not lost flows. A fifth case is a lost flow, and the first CI failure in the same test hides it.

---

## WRONG findings

### F1. D drops the only edge out of a `;`-joined line that holds two writes of the same path (WRONG, MATERIAL)

**Input:**
```python
def same():
    value = 1; value = 2
    use(value)
```
- **Main** has the edge line 2 → line 3 for `value`, labelled `NameOnly(SameLine)`.
- **Head** has no DataFlow edge into `use(value)` at all. The real flow `value = 2` → `use(value)` is lost.

The same happens with `eval/fixtures/python/dfg_reaching_same_line/a.py` (`a = first(); a = second()` then `sink(a)`). Its expected edge `a.py:2:a → a.py:3:a` no longer exists.

**Mechanism:**
- For non-synthetic owners, `occurrence_byte` is the constant `|byte| if synthetic { byte } else { 0 }` (`src/data_flow/build/file.rs:160`). As a result, `def_locs_by_occurrence` keeps one `VarLocation` per `(path, line)`, which is the first def on that line (`or_insert`, file.rs:253).
- In the lvalue edge loop, `keep_def` receives those coalesced bytes, not the span's own bytes (file.rs:563-568, and the same for twins at file.rs:624-627).
- `LexicalFacts::window` (lexical_prune.rs:231) picks `value = 2` as the dominating write W. The window starts at W's statement.
- The coalesced location is `value` in `value = 1`, which lies before W. So the candidate from the first span is dropped, and the candidate from the second span is dropped as well, because it is judged at the same coalesced bytes. Both drop, and the edge is gone.

**Corroboration in the PR itself:**
- `tests/integration/dfg_label_store_test.rs` needed `global value` / `global a` added to its two `same.py` fixtures. Without it, `collapsed_same_line_and_capture_groups_have_no_exact_occurrence` fails with "did not produce its conservative label".
- Those fixtures are SameLine witnesses, not Killed witnesses. The PR body describes all the `global` additions as "straight-line Python witnesses of `NameOnly(Killed)`", which is not accurate for this file. Here `global` hides the defect instead of preserving the witness.
- The measured "sameline" removals (django 19, mypy 2) are very likely this class. The cloud probe should list them.
- `tests/cli/min_confidence_test.rs::dfg_stats_each_label_shape_increments_its_counter_and_matches_call_stats` also runs `dfg_reaching_same_line` (`dfg_label_nameonly_sameline > 0`). CI does not show that failure because the loop panics first on the earlier `dfg_reaching_killed_def` case. Once the killed case is folded, this case will fail too, and it should. The fold must not add `global` to `dfg_reaching_same_line`.

**Go note:** Go D shares these call sites, so the same defect is latent for non-gofmt Go such as `x = 1; x = 2`. gofmt normally splits those lines, which is why Go never hit it. This is a pre-existing issue and is IMMATERIAL for this PR. A fix in the shared code would restore those Go edges, an additive change that should be disclosed.

**Fix options:**
- **(a) Python-only, Go byte-identical.** Have D keep a pair whenever the def's line holds more than one lvalue span of the path. Count these in `LexicalFacts::new` from `lvalues`, in the same spirit as rule S's "all coalesced same-line defs" clause. This is the smallest change. It costs the genuinely dominated same-line pairs, a two-digit yield per corpus.
- **(b) Precise.** Pass each span's own `(span.start_byte, span.end_byte)` to `lexical.keep_def`, for both the direct and twin sites, while still emitting the coalesced location. The coalesced edge then survives whenever any of its same-line defs is inside a window. The duplicate candidate that is dropped still increments the counter, which matches the documented "skipped candidates before dedup" meaning. This also changes Go output (restored edges) and needs disclosure.
- **Regression tests, required either way:**
  - Remove `global` from both `dfg_label_store_test.rs` fixtures. They are RED on the current head and become the witness.
  - Add a matrix row `x = 1; x = 2` / `use(x)` with `keep: true` from line 2 to line 3.
  - Let `dfg_reaching_same_line` keep running through `min_confidence_test`.
- **Not applicable to:** synthetic JS owners. They key def locations per byte, and D is not enabled for them.
- **Structural invariant worth adding:** in `python_prune_matrix` and the fixture-tree walk, for every pair D removes, assert that the surviving edges still include an edge into the same use from a def located in W's statement. That one assertion catches any future mismatch between the def location and the window.

### F2. D treats Python attribute paths as fixed storage: a rebound base and property setters both remove real flows (WRONG, MATERIAL against the stated goal; resolvable as an owner contract decision)

**Input A (base rebound twice):**
```python
def f(a, b):
    o = a
    o.val = 1        # A: writes a.val
    o = b
    o.val = 2        # W: writes b.val
    o = a
    return o.val     # reads a.val == 1
```
- **What happens:** `window` for the `o` occurrence on the return line finds W, the latest write of path `o.val`, as a direct child of the same block. A lies before W, so `A → return` is dropped. The writes `o = b` and `o = a` are never consulted.
- **Main:** has this edge, probably labelled `Killed` because RD's `affects` kills field defs on a base write.
- **Real flow:** yes, A → return.

**Input B (property setter that ignores some values):**
```python
def g(self):
    self.value = 5       # A: setter stores 5
    self.value = None    # W: setter does `if v is not None: self._v = v`
    return self.value    # getter returns 5
```
- **What happens:** `A → return` is dropped.

`classify_write` accepts any `attribute` chain as a write (via `pure_path`, lexical_prune.rs:320). Nothing checks writes to a prefix of the path, and nothing models attribute semantics. The brief lists both shapes as MATERIAL. CLAUDE.md's Go text accepts this explicitly ("Field paths are compared as fixed storage; pointer identity across reassignment of a path's base is out of model"). The new Python paragraph states no such contract. The 80/80 sample would not exercise either shape, because both are rare.

**Fix options:**
- **(a) Contract.** The owner accepts the Go fixed-storage contract for Python, and the CLAUDE.md Python paragraph says so explicitly: base rebinding, aliasing through rebinding, properties, descriptors and `__setattr__` are out of model. No code change. Choose this if the attribute share of the dominated removals is large and the owner agrees.
- **(b) Base-stable fields only.** D applies to a field path only when its base is a parameter of the owner that is never written in the owner. This covers `self.x` and `cls.x`, where almost all of the yield sits. Properties then remain the only out-of-model risk, which (a) can document. Bases written anywhere, including `for`, `with` and walrus targets, keep their pairs.
- **(c) Identifiers only.** Python D applies to simple identifier paths and keeps every field pair. This is the most conservative option. Measure the yield loss first by splitting `dfg_pruned_dominated` by path shape on django and mypy.
- **Not applicable to:** U. A pure write line has no read of the path whatever the attribute semantics, apart from a setter reading the old value, which is out of model in every option.

### F3. CI is red, and the witnesses outside the slice were not updated (WRONG: failing tests; MATERIAL: merge gate). Verdict on each of the four

The brief asks whether any of the four CI failures is a lost real flow. From source, none of them is.

1. **`evidence_delivery_provenance_reaches_sarif_with_all_three_grades`**, killed case in `flow_source(true)`. D drops `query`@2 → line 4 because `query = clean(query)` dominates, which is intended. The flow survives as 2 → use@3 (kept, since the window for both line-3 occurrences contains def 2) and then def@3 → 4. The finding is still emitted. Its grade falls from nameonly to unlabeled because the route now crosses the unlabeled intra-statement step from use@3 to def@3. This is a grade change for consumers of `--min-confidence`, not a lost flow, and it should be disclosed.
2. **`evidence_delivery_taint_reaches_sarif_with_exact_and_nameonly`**, killed case. Same mechanism as item 1, and also not a lost flow.
3. **`dfg_stats_each_label_shape...`**: `dfg_label_nameonly_killed` is 0 for `eval/fixtures/python/dfg_reaching_killed_def`. The fixture's only Killed edge (2 → 4) is dominated, which is an intended removal. As described in F1, this failure hides the next case in the loop, `dfg_reaching_same_line`, which is a lost flow.
4. **MCP `eager-default` golden**, `taint_reaches` on `app.py`.
   - **Source:** `user = input(); value = user; sink(value)`.
   - **Main:** `value = user` creates an alias twin def of `user` at line 3. Its refs include line 2, so main emitted twin@3 → line 2 and minted a zero-width anchor Use of `user` at line 2. That anchor appears in the golden graph as `use` at bytes 9..9. It caused the `OrderingUnavailable / OccurrenceMismatch` warning at `app.py:2:user`.
   - **Head:** U proves line 2 write-only for `user`, so the backward edge into a pure write is dropped and the anchor is never minted. The warning disappears and the graph changes.
   - **Verdict:** this is not a lost flow. The real chain `user`@2 → @3 → `value`@3 → @4 is untouched, and the change is an improvement. Re-bless the golden and give this reason in the commit, as CLAUDE.md requires.

**Not yet failing but affected: tier-A matrix.** CLAUDE.md asks for `tier-a --matrix-only` when a change touches this area. The PR does not report that run. Two Python `probe = "dfg"` cases would now fail:
- `eval/fixtures/python/dfg_reaching_killed_def`: an intended removal. Update the fixture, or move it to a shape D cannot prove (see F4).
- `eval/fixtures/python/dfg_reaching_same_line`: F1, so fix the code, not the fixture.

Paste both into the PR description rather than re-baselining.

**Fix:** the test-only fold in flight. Add `global` (or a shape D cannot prove) to `flow_source(true)` and to `dfg_reaching_killed_def`, re-bless the golden with the reason above, and leave `dfg_reaching_same_line` unchanged so that it acts as F1's gate.

---

## SMELL findings

### F4. Judgement on the four `global <name>` witness additions and the relaxed oracle (SMELL, IMMATERIAL apart from the part tied to F1)

- **Still testing what they tested:** `tests/ast/dfg_label_test.rs`, `tests/integration/dfg_label_parity_test.rs` (MIXED), and `tests/common/mod.rs` (provenance, taint, cross-file killed).
  - RD still computes `Killed{kill_line}` and `Exact` on the shifted lines, and the consumers still carry those labels.
  - Main used straight-line kills as the natural witness, and D now legitimately removes them, so moving the witness is right.
  - **Weakness:** the witnesses now depend on one D implementation detail, the `global` exemption. If D ever becomes finer about globals (for example, refusing only when a call sits between W and the use), they will break silently again.
  - **Option:** use a kill shape D cannot prove by construction, so the witness is independent of D's exemptions. Examples are a kill in both arms of an `if`/`else`, or a `with clean() as x:` rebinding (`with` targets are never a D write). Check first that RD reports `Killed` with a single `kill_line` for the chosen shape. If not, keep `global` and add a comment naming the dependency.
- **`dfg_label_store_test.rs`:** not a Killed witness. Here `global` hides F1. Remove it once F1 is fixed.
- **`lexical_prune_tests` subset oracle:** Python now gets the same oracle Go already has: subset, equal `labels`, equal `exact_labels`, and no `uses` equality, because U removes anchor uses. That is consistent with Go and acceptable, but it cannot see a lost flow. F1 passed it.
  - **Strengthen with:** counter equality per Python file (removed edges equal `dfg_pruned_write_only + dfg_pruned_dominated`, as `python_prune_matrix` already asserts for its four fixtures), `after.uses ⊆ before.uses`, and the "dominator edge survives" invariant from F1.

### F5. Decorated functions are silently excluded (SMELL, IMMATERIAL: conservative, yield only)

`all_functions` returns the `decorated_definition` node as the owner for a decorated function (`tree_access.rs::all_functions_via_tree` drops the inner definition). `enabled` requires `owner.kind() == "function_definition"`, so every `@property`, `@staticmethod`, `@classmethod`, route handler and fixture gets neither U nor D. Neither the CLAUDE.md wording ("a `function_definition` owner") nor the PR's "Known accepted costs" says so.

**Options:**
- Document the exclusion.
- Or enable U and D on `parsed.unwrap_decorated(owner)`. Decorator expressions sit outside the inner function and are already outside the walk. This would need a matrix row with a decorated owner.

### F6. Removal-only is not strict for Python owners that were previously capped (SMELL, IMMATERIAL: owner rule #634 applied to a new language)

The fallback cap counts survivors after U/D (pass 0 evaluates `lexical`). A Python owner over 1,000,000 prospective emissions on main that drops under the cap after pruning will now emit edges it did not emit on main. That is the owner-accepted Go rule from #634. The Python paragraph and the PR's "0 added" claim should mention it, because the corpora do not exercise it.

### F7. Hazard rows missing from the matrix (SMELL, IMMATERIAL: by reading, each is sound except the same-line case, which is F1)

The positive claims without a RED row are starred targets, annotated assignment with a value, and `except ... as` as a non-dominating target. The hazards the brief names that have no row:
- same-line `;` (F1);
- `try`/`finally`, `with suppress(...)`;
- `for`/`while` with `else`, `break`/`continue`;
- generators and `yield`;
- base rebinding (F2);
- a multi-line W.

My reading of the region logic: a write that is a direct child of a `block` either completes or leaves that block abnormally, and a block has a single entry. `with`/`try`/loop arms are separate blocks. So the rows other than F1 and F2 should pass as `keep`. They are cheap insurance against future changes to `is_region`.

---

## Open evidence question: can D remove a pair that RD labels `exact`, and is that sound?

D removes a pair purely syntactically: W is an unconditional write that is a direct child of a block, between the def's position and the use's statement. RD labels a pair `exact` when its statement-node CFG finds a route from the def's node to the use's node on which W does not kill the def. Both can be true only when RD's model contains a route that bypasses W but that Python cannot actually take. From the code, these are the mechanisms:

1. **Line granularity.** The use line shares a statement node with W. Example: a multi-line W whose last line also starts the use's statement (`x = clean(\n    y); sink(x)`). RD places the use at a node whose IN set still holds the def, while D sees W end before the use's child. This removal is sound, because RD's `exact` is false.
2. **W has no mapped statement node.** If `StatementLookup` maps W's line to no node, W gens and kills nothing in RD. This removal is sound for the same reason.
3. **A binding-relation mismatch.** W does not kill under `same_def_binding` (`kill` versus `flat_kill`) while the lookup is not uncertain. Python function-scoped names should share one binding, so I expect few or none of these. The probe should confirm.
4. **U removals.** These are pure write lines and may legitimately carry `exact`.

Neither unsound mechanism shows up as `exact` by code:
- F1's coalesced edges carry `SameLine`, as `dfg_label_store_test.rs` expects.
- F2's attribute cases carry `Killed`, because RD's `affects` lets a base write such as `o = b` kill `o.val`, and W kills by an equal path.

So for D removals labelled `exact`, D itself is probably right and RD's `exact` was false.

**Bucketing for the cloud probe:**
- Rule (U or D).
- For D removals that RD labelled `exact`:
  - does the def line hold more than one def of the path (that would be F1, and unsound);
  - does the path have fields (that would be F2);
  - do W and the use's statement share a line or node (mechanism 1, sound);
  - is W's line unmapped (mechanism 2, sound).
- Separately: every `sameline` removal, to quantify F1.

---

## Other checks

- **Non-Python languages:** `enabled` is unchanged for Go. Python-only branches are guarded by `parsed.language == Language::Python`. The `labeled_statement` check in `window` is now gated on Go and is otherwise unchanged. `pure_path` gains an `attribute` arm, which only Python reaches. Go behaviour is unchanged, apart from the latent F1 defect it already had. This is consistent with the vitess control.
- **Cache:** v156, bumped, with pins updated. Pruning runs in the per-file producer, so cold, full-hit and partial-hit builds behave the same as for Go.
- **`dfg_pruned_*` accounting:** uses the same machinery as Go. `python_prune_matrix` asserts that counters equal removed edges.
- **Wall time:** the Python owner walk now runs for every `function_definition` owner, and nested owners re-walk their subtree as in Go. The interleaved django timing shows no difference.
- **U soundness:** write-only requires every occurrence of the base identifier on the line to lie inside a plain-`=` lvalue span of exactly this path, outside nested scopes, and the line to have no recorded rvalue for the path. This covers attribute-name identifiers, keyword arguments, augmented assignment (its LHS is recorded as an rvalue), chained inner targets, `del`, `global` and walrus, all conservatively. I found no counterexample.

---

## Tally

- **MATERIAL WRONG: 3.**
  - F1: the same-line coalesced def location loses a real flow. Code fix required.
  - F2: attribute fixed storage. A code restriction, or an explicit owner contract decision.
  - F3: CI red, plus tier-A fixtures. The test fold is in flight, and the `dfg_reaching_same_line` case must stay as F1's gate.
- **SMELL, IMMATERIAL: 4.** F4, F5, F6, F7.

VERDICT: REQUEST_CHANGES

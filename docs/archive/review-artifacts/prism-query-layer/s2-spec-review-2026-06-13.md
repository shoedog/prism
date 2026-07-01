# Merged Spec Review — S2 Node-Identity (rev 1 → rev 2)

**Process:** dual spec-review via a2a-bridge (`a2a-bridge.s2-spec-review.toml`): codex gpt-5.5 **xhigh** (rigor) + claude **opus/max** (soundness) → refine → synth. **Model caveat:** the bridge run log does not record the agent model and no session log was findable, so the claude lens running opus could NOT be verified (the documented bridge claude model-override → sonnet[1m] defect may have applied); the operator (claude-opus-4-8, max) independently validated every finding — all are correct and load-bearing — so the review is acted on regardless.

**Verdict:** not ready to plan — re-scope the identity model (owner-approved 2026-06-13: byte ADDITIVE not key; defer occurrence node-splitting). Folded into spec rev 2.

## Triage (owner fix-vs-defer)
- **B1–B4 (blockers) FIXED in rev 2 via the additive re-scope:** byte is a non-key span field (range preserved on every node) used only for same-line ordering + witness anchor; var/statement keys + CFG line-addressing retained; function identity re-keyed `(file, start_line)` (FunctionId already has it → zero resolver change, B2). Operator verified `same_line_same_path_uses` needs only same-line *order* (sort-by-byte), so additive still deletes Plan B's ordering oracle.
- **M1–M7, m1–m3 FIXED in rev 2:** param span rule; half-open `[start,end)` + MISSING/zero-width; `VarLocation::Ord` excludes byte (manual); function `name` retained as display + `defs`/`uses` re-keyed to `function_start_line`; ordinal domain+tie-break; call-site explicitly out-of-scope; `reconstruct_cpg`/`from_parts` named as parallel edits; per-pattern anchor table; parse-span invariants; before/after DFG/CPG characterization fixtures pulled into acceptance.
- **DEFERRED:** occurrence-level node-splitting → priced into Plan B only if its witness must point at a specific repeated occurrence (additive ordering suffices for all known consumers).

## Raw merged review

Both lenses produced full reviews (no node failed), so this merges Rigor (completeness/ambiguity) and Soundness (design/decomposition) and resolves their disagreements inline.

**Root cause both lenses converge on:** the spec promotes the byte from an *additive field* to the *primary identity key* — re-keying `var_index`, the statement index, and `func_index`, and dropping `path`/`name` from those keys. Two of §8's own assurances — "(file, start_byte, access) is occurrence-unique" and "the resolver/matrix are unaffected" — are contradicted by the code. Every blocker below traces to that single choice, and the cleanest fix for most of them is to keep the byte additive rather than make it the key.

---

## BLOCKER

**B1 — §4/§8: `var_index` key `(file, start_byte, access)` is not occurrence-unique.**
*Issue:* `collect_identifier_paths` (`ast.rs:1640–1658`) emits, for one occurrence of `self.config.timeout`, **both** the full path and the base `self` at the same line — §3 gives both the same `start_byte` (first byte of `self`) and same `access = Use`. They collide; the first-wins guard drops one, silently collapsing the field-sensitive/insensitive distinction that exists to prevent the `dev.name`→`dev.id` leak. This degrades the very F1 fix S2 centers on (`same_line_same_path_uses` filters by `path`). §8's "occurrence-unique" claim is its own counterexample.
*Resolution:* keep `path` in the key → `(file, start_byte, path, access)`, or specify that these alternates stop becoming separate `Variable` nodes.
*Both lenses agree; Soundness supplies the airtight proof and the §8 refutation, Rigor adds the nuance that lvalue field writes do **not** emit a base def (so the collision is a read/alias phenomenon, not symmetric).*

**B2 — §2/§4/§7.6: byte-keyed `func_index` collides with the byte-free `FunctionId`; function-identity migration is incomplete.**
*Issue:* §4 re-keys `func_index` to `(file, start_byte)` and §7.6 asserts the resolver is unaffected — both cannot hold. The CPG is assembled from `CallGraph::FunctionId`, which carries `{file, name, start_line, end_line}` and **no `start_byte`** (`call_graph.rs:15–19`); Step-5 call edges, Step-5b param binding, and Step-9 dispatch all map resolved callees in by name. `function_node(file,name)` (`query.rs:20–24`) breaks; `callers_of`/`callees_of` read the name from the key.
*Resolution:* **key `func_index` by `(file, start_line)`** — `FunctionId` already supplies it, so it de-conflates same-name functions with **zero resolver change** and honors §7.6; carry `start_byte`/`end_byte` as additive span fields on the Function node for the witness. Include cache and public-query migration.
*Disagreement resolved → Soundness is right: Rigor proposed adding a byte to `FunctionId`/resolution outputs or building a bridge; that is more invasive and contradicts §7.6, whereas `start_line` is already present and wire-stable.*

**B3 — §4/§7.3: byte-keying the statement index severs the line-keyed CFG.**
*Issue:* `CfgEdge` is `{file, from_line, to_line}` (`cfg.rs:19–22`) and Step-8 wires control-flow edges via `stmt_index.get(&(file, from_line))` (`build.rs:435–436`). §4 simultaneously re-keys the statement index to `(file, start_byte)` and stops the line-dedup in `statements_in_function`. With a byte-keyed index the line-based lookup cannot be formed → **every ControlFlow edge drops**; and the now-un-deduped multiple statements per line cannot be disambiguated by a line-granular edge. §7.3's acceptance ("multi-statement line keeps both statements") cannot be met as specified.
*Resolution:* keep the statement index line-addressable for CFG wiring (`(file, line) → Vec<NodeIndex>`), or give `CfgEdge` byte/node-id endpoints — a `cfg.rs` change the spec does not budget.
*Both lenses agree; Soundness contributes the precise drop mechanism, Rigor the §7.3 acceptance contradiction.*

**B4 — §4: the extraction/edge update is mis-scoped — edges are reconstructed line-only and will miss byte-keyed lookups, and occurrences come from more than the two named helpers.**
*Issue:* most DFG edges flow from `find_path_references_scoped`, which returns `BTreeSet<usize>` (lines, collapsed). A def *node* gets a real byte from `lvalue_paths`, but the edge's `from` is reconstructed from `*def_line` with no byte (`data_flow.rs:338`), so Step-4's `var_index.get(from_key)` keys on a placeholder ≠ the node's byte, misses, and the edge vanishes. §4 names only `assignment_lvalue_paths_on_lines`/`rvalue_identifier_paths_on_lines`, but locations also originate from parameter defs, raw and resolved aliases, and call-argument text — none updated.
*Resolution:* introduce a shared `VarOccurrence { path, line, start_byte, end_byte, function_start, kind }` emitted by **every** source, and thread bytes through the four edge-loc reconstruction sites — **or** keep the byte additive (non-key) so edges still map by the existing `(file, function, line, path, access)`.
*Merged from Soundness BLOCKER (edge-drop correctness) + Rigor MAJOR (breadth of sources); elevated to BLOCKER because the edge-drop is a silent correctness regression in the load-bearing F1 path. Soundness credits that §8's promised "characterization tests on DFG edge sets" would catch it — meaning the spec as written cannot ship, which is exactly blocker-grade.*

---

## MAJOR

**M1 — §3/§5: parameter-occurrence identity is under-specified.**
*Issue:* DFG pins parameter defs to the function-start line, and `is_parameter_binding` detects them by `Variable Def` line == function start (`trace.rs:188–199`). §3's span rule would put a parameter's `start_byte` at the parameter token (possibly a different physical line in a multi-line signature) while its `line` stays at function start — `line` and `start_byte` then disagree, and the param node sits in the function-start line bucket with an out-of-line byte.
*Resolution:* define parameters structurally — keep `line` = function start, specify `start_byte`, add a parameter flag; add multi-line-signature tests.
*Disagreement resolved → split decision: Soundness is right that this is **not a hard break** (the `line` field is retained, so `is_parameter_binding` survives), but Rigor is right that the spec **leaves the byte/line rule undefined**, which the §3 wire contract and same-line sorting need. Net: MAJOR, not BLOCKER.*

**M2 — §3: byte-range notation is ambiguous (inclusive vs half-open).**
*Issue:* `[start_byte, end_byte]` reads inclusive, but tree-sitter byte ranges are conventionally half-open. This is a witness/wire contract consumers will hard-code against.
*Resolution:* state explicitly whether `end_byte` is exclusive, and define empty/MISSING-node handling. *(Rigor, unique.)*

**M3 — §5/§8: `VarLocation`'s `derive(Ord)`/`Eq` change ripples into the reachability layer the spec doesn't list.**
*Issue:* `VarLocation` derives `Ord` (so the new byte fields enter identity automatically) and is a `BTreeMap` key (`forward`/`backward`) and `BTreeSet` member returned by `forward_reachable`/`backward_reachable`, taint, and `cfg_queries` — consumed by `taint`/`full_flow`/`left_flow`/`provenance_slice`. None appear in §5's blast radius or §8's `var_index`-only test scope.
*Resolution:* state the DFG-identity change explicitly and gate it with reachability/edge-set characterization tests; **or** hand-write `VarLocation::Ord` to exclude the byte (the spec must call this out — `derive(Ord)` includes all fields).
*Calibration → Soundness corrects Rigor: §5's "~26 algorithms read `node.line()` — untouched" is **accurate** (the `line` field stays; only ~5 files read `.line()`); the real gap is the reachability consumers, not the line-readers. Rigor's "changes every reachability result" was too broad.*

**M4 — §2/§5: dropping the node's `function` name leaves a fallible resolution and a half-done F2 fix.**
*Issue:* (a) `function_of()`'s behavior for **orphan variables** — a `Variable` whose enclosing function has no Function node, which Step-6's `if let Some` guard (`build.rs:410`) proves can occur — is unspecified and won't surface as a test failure; (b) `defs`/`uses` stay keyed by function **name** (`data_flow.rs:67–69`), so same-name functions still co-mingle *in the DFG index* — F2 de-conflation lands at the node layer but not the index feeding it.
*Resolution:* keep `name` as a non-identity display field on the node (eliminates the resolution), or define `function_of()`'s `None` semantics; decide explicitly whether `defs`/`uses` re-key to `function_start`; and state which public structs (`VarLocation`, `FlowEdge`, `SymbolRef::Variable`, `var_node`) keep display names vs. add `function_start` vs. become shims.
*Calibration → Soundness credits that §5 does add `function_of()` and §8 does flag the ripple (Rigor implied both were unacknowledged); the two surviving failure modes above are the precise, narrower finding.*

**M5 — §5: navigation `ordinal` semantics are undefined, and it is not "for free."**
*Issue:* "populated from the same-line byte rank" never says the ranked set (all nodes? one symbol kind? same path? emitted evidence?), and ties when a full path and base path share `start_byte` (the B1 collision). Separately, it touches 8 hardcoded `ordinal: 0` sites plus per-bucket byte-sorting plus the M4 name resolution.
*Resolution:* define the ordinal domain and tie-breaker, or add byte spans directly to `SymbolRef`. *(Rigor MAJOR 8 + Soundness MINOR 7 merged.)*

**M6 — §4/§5: call-site identity is omitted.**
*Issue:* S2 hardens Function/Statement/Variable nodes, but `CallSite` is line-only and nav call evidence carries only `call_site_line`; same-line repeated calls still collapse.
*Resolution:* add call-site spans/ordinals if same-line call identity is a nav goal, **or** explicitly mark it out of scope. *(Rigor, unique.)*

**M7 — §5/§7.5: cache migration is more than a version bump — `reconstruct_cpg` is a duplicate rebuild that won't compile.**
*Issue:* `reconstruct_cpg` (`cpg_cache.rs:324`) is a second copy of build.rs's index logic; it destructures `function` and builds the old `(file,name)`/5-tuple keys, so it won't compile under §2, and it must byte-sort the `location_index` buckets it rebuilds. The index key **types** change, not just values. §5 mentions only the `CACHE_VERSION` 4→5 bump and "SerializedCpg shape unchanged."
*Resolution:* name `reconstruct_cpg` (and `from_parts`) as a required parallel edit and state the rebuilt index invariants for `func_index`/`var_index`/`location_index`; §7.5's round-trip test guards it.
*Disagreement resolved → MAJOR: Soundness rated this MINOR, but its own evidence (won't compile, key-type change, byte-sort on reconstruct) is MAJOR-grade and the spec actively understates it; Rigor's MAJOR is the right call.*

---

## MINOR

**m1 — §3/§6: fallback anchor rules need a per-pattern table.** "Each language's extraction defines its anchor" is not precise enough for destructuring projections, pointer deref, array/index paths, alias-resolved paths, parameters, and synthesized paths. Add per-pattern anchor rules and tests. *(Rigor; Soundness corroborates — its B1 alias case is "fallback-dependent" on this under-specified choice.)*

**m2 — §6: parse-degraded span invariants are not observable.** Turn "approximate but monotonic" into concrete invariants: bounds within file length, `start_byte ≤ end_byte`, zero-width policy, warning behavior. *(Rigor, unique.)*

**m3 — §7/§8: acceptance should require the DFG/CPG characterization fixtures §8 already promises.** §8's risk table promises before/after DFG edge-set tests, but §7 acceptance only requires determinism + no-regression. Pull explicit before/after DFG edge-set and CPG-dump fixtures into acceptance — they are the guard for B3, B4, and M3. *(Rigor M13 + both lenses' repeated reliance on characterization testing.)*

---

**Verdict: not ready to plan — re-scope the identity model first.** Both lenses agree (Rigor "needs changes," Soundness "reconsider"). The required change is to split the one decision the spec conflates: make the byte **additive** on the variable and statement axes (keep `path` in `var_index`, keep `VarLocation`'s identity, keep the statement index line-addressable; use the byte only to order same-line buckets and as the serialized anchor — this dissolves B1, B3, B4, and the variable side of M3), and key the **function** axis by `(file, start_line)` for de-conflation (dissolves B2, honors §7.6, no resolver change). Then resolve M1–M2 and the call-site/cache/ordinal ambiguities (M5–M7) and fold the characterization tests (m3) into acceptance. If occurrence-level *node-splitting* is genuinely required by Plan B's witness, that is the larger change and must be priced separately — including byte-threading through `find_path_references_scoped`, the edge-loc reconstruction sites, the CFG wiring, and the reachability characterization tests.
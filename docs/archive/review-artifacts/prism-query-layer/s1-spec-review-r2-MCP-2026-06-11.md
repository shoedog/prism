# Merged Spec Review — Prism S1 (revision 2)

Both lenses returned full reviews; no missing lens. Both also retracted several first-draft concerns after code verification (the "all 28 call sites" worry, the `--no-cache` CLI-shape objection, `ParsedFile` literal-construction migration risk, and the B1 comments/strings concern — closed by §4's B2 deferral). Those are excluded below. The soundness lens additionally verified the B1 "quirky code is the extractor" construction end-to-end against `ast.rs:3333-3398` and confirmed it complete — the spec's central bet holds.

## BLOCKER

1. **§3 Slice A — the reconstruction walk is specified in the wrong direction** *(Soundness)*. `descendant_for_byte_range` returns the *deepest* node spanning the range, so the mandated "walk-down" to the stored `(range, kind_id)` can never reach a same-span *ancestor* — the only case where recovery is needed. It is dead code today (the prior review round verified no current grammar has a same-span ambiguity), but under grammar drift it silently flips an entire language to the per-file fallback with zero test signal, since the §7 gate samples only tokio. **Resolution:** change "walk-down" to walking **up** through same-span ancestors (or descend from root via `goto_first_child_for_byte`), and add a fallback-fire counter asserted ≈0 in the per-language equivalence tests. The spec as written mandates a provably non-functional recovery step, so this blocks even though the impact is latent.

## MAJOR

2. **§2 Correctness contract — call-edge equality projection is too weak** *(Rigor)*. `(caller, callee_name, line)` omits `qualifier` (present on `CallSite`, consumed by resolution), and doesn't say whether `caller` means the full `FunctionId {file, name, start_line, end_line}`. **Resolution:** require equality over the full `CallSite`, or spell out the exact tuple including full caller identity and `qualifier`.

3. **§2a/§5 — exact-order parity must cover serialized `CallGraph` and `DataFlowGraph`, not only CPG node/edge vectors** *(Rigor)*. `cpg_cache.rs` serializes `call_graph` and `dfg` alongside the CPG, and `DataFlowGraph.edges` is an order-sensitive `Vec`. **Resolution:** extend the serial-canonical-merge-order requirement and the §2.2 in-order equality tests (or a byte-level cache parity test) to CG and DFG.

4. **§4 B1 — the "identifier" grammar is unpinned** *(Rigor)*. "Every identifier immediately preceded by `->` or `.`" can diverge between two raw scanners on Unicode, digit-leading tokens, and boundary cases; existing Level-4 filters use `char::is_alphanumeric() || '_'`. **Resolution:** pin the exact predicate for both candidate enumeration and the oracle universe, explicitly over raw source including comments/strings (B1 semantics).

5. **§4 B1 — the differential oracle's pairing rule is unstated, and the natural reading re-runs the removed hotspot inside `cargo test`** *(Soundness)*. Universe × all-files is the original O(fields × files × lines) blow-up relocated into the test suite (clause (a) alone is large on the prism corpus) — the predictable end state is a multi-minute test that gets `#[ignore]`d, eroding the gate that makes "byte-identical by construction" trustworthy. **Resolution:** state the pairing rule in two halves — *excess*: iterate the index's own `(field, file)` keys against the legacy scan; *misses*: for each universe field, legacy-scan only files containing the `->field`/`.field` substring. The prefilter is provably outcome-preserving (it is the legacy scanner's own `has_field` check hoisted to file level) and must scope the **whole** universe, including clause (a).

6. **§7 — both hotspot acceptance gates have validity defects.** (a) The C gate's "during the parse phase" interval is unobservable from `/usr/bin/time` on the whole command *(Rigor)* — define instrumentation/profiler markers or restate the gate as full-command user/wall with caveats. (b) The B1 gate can **spuriously fail**: the index builder *is* the legacy per-field logic refactored to a per-line callable, and post-S1 hugo's total shrinks several-fold while the builder still sweeps 234k LOC, so legitimate builder frames can exceed the 1% threshold *(Soundness — upgraded here from its MINOR ranking: a gate that fails with the design working as intended blocks merge, the same severity class as (a))*. **Resolution:** give the per-line callable a distinct symbol; the gate then asserts the *legacy* symbol at ≈0% in production builds — stricter and unambiguous.

7. **§3 Slice A — the required synthetic reconstruction-mismatch test has no seam** *(Rigor)*. `functions()` returns an immutable slice and construction happens inside `parse()`, so implementors will either expose mutation ad hoc or skip the test. **Resolution:** specify an intended `#[cfg(test)]` helper or local unit-test hook in `ast.rs`. Pairs naturally with the BLOCKER's fallback-fire counter.

8. **§6 — the bench script contract lacks executable command templates and default-repo resolution** *(Rigor)*. **Resolution:** specify the cold template (`prism nav --no-cache repo-map --repo <path>` is a valid cold/no-write shape) and the warm template (`--cache-dir` + an immediately repeated identical command), output format/disposal, how `prism/tokio/hugo/django/rust-analyzer` paths resolve (env vars or config), and behavior when a default repo is absent.

## MINOR

9. **§3 `FunctionInfo`** — document `start_line`/`end_line` as 1-indexed inclusive (matching `node_line_range()`), and add at least `Clone` to the derive list since `ParsedFile` derives `Clone` *(Rigor)*.

10. **§3 Step 5b** — write the comparand as `callee_id.name` explicitly. Both `site.callee_name` and `callee_id.name` are in scope at that block and they differ for Level-4-resolved calls; the §2.2 tests would catch the mistake, but for the contracted agent-TDD loop this one-token ambiguity is a guaranteed wasted iteration *(Soundness)*.

11. **§3 eager-vs-lazy** — the decision is correct but the rationale is absent. Record it: eager construction is load-bearing because under C1 a lazy table first-touches every file inside the *serial* CPG build, surrendering the parallelism eager buys; lazy `OnceLock` is the designated fallback if the 10% warm gate ever fails *(Soundness)*.

12. **§6 portability** — pin required tools (`timeout` vs `gtimeout`, `/usr/bin/time -l` availability) and define RSS unit conversion and failure behavior so results reproduce across macOS setups *(Rigor)*.

13. **§9/§10 plan carry-overs** — the plan should carry two priced-in risks verbatim: the B2 trigger ("scheduled only once the Tier-A harness is live"), because B1 relocates the quirky scanner somewhere *less* visible than today's hot loop; and a note that lifting §2a's insertion-order cap on C2 is S2-adjacent `NodeIndex`-identity work, not a C2 option *(Soundness)*.

## Disagreement resolutions

- **Verdict (Rigor "needs changes" vs Soundness "sound to plan"):** no substantive conflict — Soundness validated the architecture and decomposition (which Rigor did not contest), while Rigor's surviving findings are contract-text gaps; both demand the same thing: a revision 3 before `writing-plans`.
- **B1 gate severity (Soundness MINOR vs merged MAJOR):** merged as MAJOR — Soundness itself sharpened the finding to "can spuriously fail," and a falsely-failing acceptance gate is the same severity class as Rigor's MAJOR on the unobservable C gate.
- **Walk-direction severity (latent vs blocking):** Soundness is right that current impact is zero, but the spec text mandates a mechanism that provably cannot work; since the fix is one word plus one assertion, it blocks the revision rather than being deferred to the plan.

**Verdict:** Not ready to plan as-is — fold items 1–8 into a spec revision 3 (the design and A/B/C decomposition are sound and need no rework; items 9–13 may land as revision edits or plan-level notes), then proceed to `writing-plans`.
# MERGED SPEC REVIEW — Plan A: Substrate Hardening (Tier 2)

Both lenses returned full reviews (no node failed), and they converged more than they diverged. The design's *shape* is sound — the engine's home is pinned (§3 + §7 place A3 in `src/cpg/` substrate), the A3 invariant matches the existing `query.rs` traverse-in-NodeIndex / convert-at-boundary idiom, and the A2/A5 deferrals are correctly justified by "no intraprocedural v1 consumer." What remains are Rust-level *contracts* that must be pinned before A3 is coded deterministically and before Plan B locks `shape.rs`. Disagreements are resolved inline.

---

## BLOCKER

**B1 — §3 (A3): the CFG predicate is not executable as written.**
*Issue:* The spec says enqueue a same-function neighbor whose line is CFG-reachable via `cfg_reachable_including_continuation`, but never pins source-line inclusion or the no-CFG fallback. Verified landmines: `cfg_reachable_lines` *excludes the start line* (`cfg/cfg_queries.rs:99-105`), so a same-line `x = source` assignment-propagation step (the very edge A3 traverses, `query.rs:535-562`) can be pruned; and `taint_forward_cfg` falls back to pure taint when no CFG exists and always includes same-line targets (`:133-135,171-185`). Both lenses flag this; RIGOR grounds it most precisely.
*Resolution:* Define `cfg_valid(source, target)` exactly — including same-line/source-line behavior and the absent-CFG fallback — so two implementations cannot diverge.

**B2 — §3/§5/§8: the interprocedural boundary is under-specified at the engine level AND missing as an output shape.** *(RIGOR BLOCKER "boundary records" + SOUNDNESS MAJOR "A" — same defect, two altitudes; merged and kept at BLOCKER because it is the exact misleading-Evidence failure Plan A exists to prevent.)*
*Issue:* §3 records cross-function edges only as a "boundary set = cross-function targets." A target-only set cannot name the dropped `(from, to)` edge or distinguish multiple sources reaching the same boundary target — yet Plan B wants sink-located `InterproceduralBoundary` warnings naming the dropped edge/source, and the constraints brief makes boundary-visibility a *hard* requirement (#4: make the boundary visible, never a silent false negative). Separately, §5 — the section that becomes the locked `shape.rs` contract — enumerates only `reached:false` for "unreachable" under "one-shape-per-response," with no third shape reserved. A taint that *exits to a callee* would then render identically to one with *no path*, reading as "safe" when the honest answer is "indeterminate — stopped at a call boundary."
*Resolution:* (a) Define an `InterproceduralBoundary`/`BoundaryEdge` struct with ordering, de-dupe key, and whether it is part of A3's `Trace`; (b) reserve a first-class boundary-exited output shape (e.g. `reached: indeterminate`) in §5. Both before Plan B locks `shape.rs`.

**B3 — §4/§5: `cleansed_for` is function-body presence, not per-path evidence, and that caveat lives only in prose.** *(SOUNDNESS "B", strongest unique point; RIGOR independently asks to "keep the warning." Elevated to BLOCKER: both lenses call it a false-negative-on-a-real-vuln risk, the same misleading-Evidence class as B2.)*
*Issue:* §4 correctly states `cleansed_for` is "function-body sanitizer presence keyed on the source function, not per-path evidence," but §5 — what the LLM actually consumes — says nothing about labeling it. With a real, non-trivial recognizer set (python 5 / js_ts 3 / path 2, verified), a consumer reads "a sanitizer exists in this function" as "this path is neutralized" when the sanitizer sits on a different branch.
*Resolution:* `shape.rs` must label this "function-level sanitizer present (not path-proven)," or withhold `cleansed_for` from the witness until per-path sanitization exists. Before Plan B locks `shape.rs`.

---

## MAJOR

**M1 — §3/§5: witness determinism does not define relation-merge semantics, and "first enqueue wins" is unimplementable on the existing engines.** *(RIGOR #4 + SOUNDNESS "F" reinforcement, merged.)*
*Issue:* "Neighbors sorted by `NodeIndex::index()`; first enqueue wins" is insufficient when DataFlow and same-line assignment-propagation edges produce overlapping candidates for one target — the spec never says which relation wins the parent slot, how duplicate targets are handled, or which edge label the parent carries (it must be `"DataFlow"`/`"AssignmentPropagation"`, never `"TaintFlow"`/`"ControlFlow"`). Critically, the existing engines dedup at *pop* (`query.rs:530`, `data_flow.rs:479`) — "first *pop* wins" — so A3 cannot reuse them for "first *enqueue* wins"; it needs push-time parent assignment + a push guard intrinsically.
*Resolution:* Specify unified neighbor construction with relation priority, duplicate-target handling, and parent-label choice; state that A3 carries its own push-time guard (this also covers the multi-source-sink case where Plan B picks the shortest witness's source for cleansing).

**M2 — §4/§5/A7: the typed-output seam lacks the Rust-level contracts a mechanical implementation needs.** *(RIGOR #3; both lenses agree the draft "SeedSet undefined" claim is RETRACTED — Plan B defines `SeedSpec`/`ResolvedSeed`/`SeedSet`/`resolve_seed_set`/`taint_reaches`. RIGOR was right to retract.)*
*Issue:* Current `Evidence` has only `query/items/truncated/warnings/graph` and *no* summary field, with closed `Reason`/`WarningKind` enums (`navigation/types.rs:42-88,118-127`); Plan B sketches `ReasoningSummary` but not the additive serde schema or text/JSON rendering. Still unpinned across the two docs: the exact `Trace` type, parent-map key/value types, root/source representation, `shape.rs` function signatures, and the `Evidence` field name for the summary.
*Resolution:* Pin these in Plan A's hand-off (§8) or normatively defer each to Plan B by name — but enumerate them, since they hit implementation, not just wording.

**M3 — §4 (A4): expose one reasoning adapter, not two raw taint internals.** *(RIGOR #5 adapter-shape; SOUNDNESS "D"/"G5" supplies the resolution to the lenses' only real disagreement.)*
*Issue:* `apply_cleansers` is shaped for production `FlowPath` fans and bails on empty paths (`taint.rs:10645-10651`); A3 produces parent-trace witnesses, not `FlowPath`s. *Disagreement resolved:* RIGOR's draft suggested doing the relocation-into-`src/sanitizers/` "now inside A4"; SOUNDNESS is correct that this is A2-sized work, because `apply_cleansers`/`function_body_cleansed_for` transitively depend on taint-local `collect_calls`/`call_path_*`/`is_js_ts_language` — so §4/§9's pairing of the relocation with A2 stands.
*Resolution:* Specify a single reasoning-facing adapter (e.g. "cleansed categories for a source `VarLocation`"), defining source==sink and empty-witness behavior; keep the physical relocation deferred to A2.

**M4 — §7: the byte-identity proof is not mapped to the surfaces A6 actually touches.**
*Issue:* A6 can affect LeftFlow, FullFlow, Taint target-seed synthesis, `taint_forward(_cfg)`, CFG reachability, and chop paths (generic `reachable_forward/backward` feed all of these, `query.rs:86-138`). `cli_nav_compat` locks LeftFlow + nav goldens but not all diff-review behavior, and its own comment says the aggregate `review` preset is *not* byte-stable (`nav_compat_test.rs:17-22`).
*Resolution:* Add a call-site/proof matrix, or narrow the byte-identity claim to the surfaces actually covered.

---

## MINOR

**m1 — §4 (A6): scope the push-guard out of the gate.** *(Disagreement: RIGOR #6 rated this MAJOR "specify it"; SOUNDNESS "F" rated it MINOR "remove it." SOUNDNESS is right — see M1: A3 needs its own push-time guard regardless, so A6 buys the gate nothing while expanding the golden-proof surface onto production functions.)* *Resolution:* Move A6 out of the gate as separable, opportunistic hygiene; RIGOR's demand to name every affected function and whether the guard is `visited`-only vs a separate `enqueued` set applies only if it is retained.

**m2 — §4 (A4)/§9: the deferred A4→A2 unwind has no firing condition.** A2 ships "when its Phase-IP consumer lands" — no date, no issue — so a "temporary" layering inversion becomes load-bearing. *Resolution:* Attach the unwind to a dated/issue-tracked obligation on A2.

**m3 — §3/§7 (A7): doc wording + pin the witness representation.** A7's "`taint_reaches.rs` (the query + A3's BFS)" should read "*calls* A3's BFS" — §3/§7 already place the engine in substrate. Pin that A3 reconstructs the witness in `NodeIndex` space and converts to `VarLocation`/`file:line` only at the `shape.rs` boundary (the `to_var_location` idiom, `query.rs:568`); a `VarLocation`-keyed parent map is collision-free intraprocedurally but lossy if it drops the access path.

**m4 — §5: frame the witness honestly.** The CFG check is per-node ("N reachable from source"), not pairwise along the path, and Step-5b base-path truncation makes v1 field-insensitive — both correct safe over-approximations. *Resolution:* §5's summary should say "data-flow path," not "the path the taint takes," and add a one-line "field-insensitive / over-approximate" Evidence caveat.

**m5 — §7/§9 (A5): the Rust `?` deferral is sound — log only the gap.** A5's subject is the synthetic error-channel-to-caller, inherently interprocedural and outside v1 scope; intraprocedural `?`-unwrap of a tainted local is already caught by same-line assignment propagation. *Resolution:* Document that on a `?`-laden Rust flow crossing the error channel, a clean `reached:false` reflects scope, not proof of safety — relevant given the Rust-heavy dogfood target.

**m6 — §7: make the test targets mechanically exact.** `tests/reasoning_*` and the reasoning `[[test]]` targets do not exist yet, and `src/cpg/tests.rs` is a unit-test module, not a Cargo test target. *Resolution:* List the exact files, target names, and commands to add/run (mirror the `cli_nav_compat` style at `Cargo.toml:494-496`).

---

**Verdict:** Design is **sound to plan** (decomposition, engine home, and A3 invariant all hold) — but **B1, B2, and B3 must be resolved before A3 is implemented and before Plan B locks `shape.rs`**, since all three are the misleading-Evidence failures Plan A exists to prevent. RIGOR's "needs changes" and SOUNDNESS's "sound to plan" are reconciled: the architecture is sound, the contracts are not yet pinned.
# Review of `2026-04-25-phase1-cwe-go-design.md` — iteration 3

**Date:** 2026-04-25
**Reviewer:** Prism agent (independent pass; did not author the design)
**Subject:** [`2026-04-25-phase1-cwe-go-design.md`](2026-04-25-phase1-cwe-go-design.md)
**Scope:** Verification of all iteration-2 §3 items + first pass on §4. Spec now covers §1–§4 (closing note at line 587).

**Prior reviews:**
- [iteration 1](2026-04-25-phase1-cwe-go-design-review.md) — §1 + §2
- [iteration 2](2026-04-25-phase1-cwe-go-design-review-2.md) — §1+§2 verification + §3

---

## Verdict

**Ready to ship after three small §4 cleanups.** All seven iteration-2 §3 items have been addressed cleanly — most with substantial rewrites that improve the spec beyond what the original feedback asked for. §4 is procedurally complete; only minor enumeration/convention items remain.

> **Correction to chat review:** my verbal review stated "R1 is still unresolved in §3.5." That was wrong — I didn't re-read §3.5 before stating that and was working from iteration-2 memory. On fresh read, §3.5 has been rewritten to enumerate the three options, pick option (1) → `FlowPath` augmentation in `data_flow.rs`, and explain the reasoning (cleansing is path-sensitive). §4.3 / §4.5's `FlowPath` references match §3.5's pick — there is no terminology drift. Apologies for the noise.

---

## §3 — iteration-2 items verified ✅

All seven items resolved:

| # | Iteration-2 item | Resolution in current spec | Status |
|---|---|---|---|
| R1 | §3.5: `TaintValue` integration shape unspecified | Rewritten lines 373–388. Three options enumerated; picks **option (1) — `FlowPath` augmentation in `data_flow.rs`** with `cleansed_for: BTreeSet<SanitizerCategory>` field. Reasoning given (cleansing is path-sensitive; option (2) loses precision; option (3) achieves it but with call-site clutter). | ✅ |
| R2 | §3.9: Shell-escape cleanser appears redundant | Section retitled "Shell-escape — no cleanser needed in Phase 1." Cleanser dropped explicitly. Reasoning preserved (sink `semantic_check` already excludes safe form). Forward-looking justification kept ("future phase consuming `*exec.Cmd` would reintroduce"). `SHELL_RECOGNIZERS` is `&[]` for symmetry. | ✅ |
| R3 | §3.2: `syscall.Exec` slice-element taint behavior unspecified | New paragraph at line 300: "Prism's existing DFG models slices conservatively — any tainted element taints the slice as a whole..." Per-element tracking explicitly out of scope. | ✅ |
| S1 | §3.8: Rel-form paired_check direction ambiguity | New paragraph at line 454: "Rel-form direction ambiguity (Phase 1 limitation)..." with both correct and incorrect guard shapes spelled out, false-negative explicitly documented, CFG-aware enhancement deferred. | ✅ |
| S2 | §3.10: C1 fixture validation external, not in Phase 1 tests | New closing paragraph at line 499: "C1 fixture validation is external." Validation cadence reference, in-tree vs eval-team scope cleanly separated. | ✅ |
| S3 | §3.1: Two sink storage shapes coexist; migration footnote | Inline footnote at line 238: "Future migration: existing `SINK_PATTERNS` could be promoted to `SinkPattern` shape... deferred to keep this PR scoped; Phases 2/3 will revisit." | ✅ |
| S4 | §3.4: `paired_check` resolution textual co-occurrence | New paragraph at line 371: "Resolution model: `paired_check` is a string name resolved at suppression time by textual co-occurrence in the same function body — not a type-safe binding..." | ✅ |

The R1 rewrite is particularly strong — option (1) was my recommended path and the reasoning given (path-sensitivity) is the right justification, not just adoption-by-suggestion. R2's drop with forward-looking note is the cleaner of the two options I gave. R3, S1, S2, S3, S4 are clean adoptions of the suggested wording.

Two stylistic nits from iteration-2 also addressed:
- **PowerShell / exotic shell paths:** §3.2 paragraph at line 302 now has explicit scope note. ✅
- **Stale §1 Commit 1 references** (`FrameworkRegistry`, "the detection trait"): not yet corrected at line 19 — still says "defining `FrameworkSpec`, `FrameworkRegistry`, and the detection trait." Carry-forward — see §4 cleanups below.

---

## §4 — review

### Small items

#### S1 — §4.1: Cargo.toml additions enumeration is missing

§2.9 enumerates 4 new `[[test]]` entries (`frameworks_nethttp`, `frameworks_gin`, `frameworks_gorilla_mux`, `frameworks_registry`). §3.10 doesn't mention Cargo.toml. §4.1 introduces two new test locations (`tests/algo/taxonomy/sanitizers_test.rs`, `tests/integration/cwe_phase1_suppression_test.rs`) but doesn't list the Cargo.toml additions for them.

Add a one-liner to §4.1:

> Cargo.toml `[[test]]` additions: 4 from §2.9 + `algo_taxonomy_sanitizers` + `integration_cwe_phase1_suppression` = 6 new entries total.

This catches the kind of completeness drift that produces "test passes locally but `cargo test --test name` fails" issues during CI.

#### S2 — §4.1: `tests/integration/coverage_test.rs` 3-copy update missing

CLAUDE.md flags that `coverage_test.rs` has the `all_test_files` array in **three places** (in `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and `test_coverage_matrix_validation`) and all three need updating when test files are added or renamed. PR #71 handled this correctly. §4.1 should call it out as an explicit step:

> Update `tests/integration/coverage_test.rs` `all_test_files` arrays — all three copies — with the new test file paths. Run `cargo test --test integration_coverage` to verify the matrix doesn't under-report.

The kind of step that gets missed in fresh sessions; explicit mention saves a CI cycle.

#### S3 — §4.4: `RE-*.md` reply convention asserted without grounding

> the reply doc established `~/code/agent-eval/analysis/RE-*.md` as their reply convention

I don't have evidence in this conversation that this convention is established. The ACK §6 mentions filing replies but doesn't specify the `RE-` prefix. If it's real (from a prior session), fine — but the ACK doesn't surface it, so a reader who only has ACK + spec wouldn't know. Two options:

- **Verify** the convention exists (presumably from a prior session) and add a one-line citation.
- **Drop** the specific naming claim and leave it as "reply mechanism per ACK §6 — likely a `RE-*.md` doc by analogy with the existing reply convention, but TBD."

The latter is honest; the former is more useful if true.

### Carry-forward from iteration-2

#### S4 — Stale §1 Commit 1 references

Iteration-1 flagged two stale phrases on line 19 of §1 Commit 1:

> New `src/frameworks/mod.rs` defining `FrameworkSpec`, **`FrameworkRegistry`**, and **the detection trait**.

Both bolded phrases were already stale at iteration-1 time (§2.4 dropped the enum; there is no detection *trait*, just a function pointer). Suggested fix unchanged from iteration-1:

> New `src/frameworks/mod.rs` defining `FrameworkSpec`, `ALL_FRAMEWORKS`, and the `detect_for` dispatch function.

Trivial cleanup; appropriate for the self-review pass.

### Strengths

- **§4.3 risk register** is the standout addition. Five concrete risks, each with a trigger and an action. Many specs skip risk pre-decision entirely; this section's existence catches contingencies the implementer would otherwise hit cold. The `FlowPath` row is correctly framed as a contingency for the *implementation* of the (now decided) integration choice — not as the "decide later" punt I incorrectly read it as in chat.
- **§4.5 per-commit time breakdown** with explicit buffer aligns with the lower end of the ACK §2 estimate. The "faster if `FlowPath` augmentation goes cleanly; slower if path-validation needs tuning" hedge is calibrated to the actual variance sources.
- **§4.2 branch / PR / sub-agent dispatch** matches the Phase 0 pattern verbatim. No surprises for the implementer; one-step lookup against PR #71's shape.
- **§4.4 acceptance ladder** ("shipped" on PR merge / "accepted" on eval-team confirmation) makes the ownership boundary explicit. Useful for downstream coordination.
- **§4.6 out-of-scope recap** correctly absorbs the iteration-2 nits (PowerShell, exotic shell paths) and §3 deferrals (CFG-aware path-validation). Internally consistent with §1's out-of-scope list.
- **§4.1 three-layer test taxonomy** (unit detection / unit sinks-and-recognizers / integration suppression-rate) maps cleanly to the three Phase 1 acceptance criteria. Pinning ≥80% suppression via an integration test against a 10+10 fixture suite is the right shape.

---

## Recommendation

The spec is essentially shippable. Three §4 cleanups (S1 Cargo.toml enumeration, S2 coverage_test.rs 3-copy, S3 RE-*.md grounding) plus the carry-forward iteration-1 stale-references cleanup. All four are wording fixes appropriate for the author's self-review pass — no further reviewer iteration needed.

**Next steps (per the brainstorming skill flow noted earlier):**

1. Author self-review pass — apply S1–S4 above; check internal consistency and placeholder cleanup across all four sections.
2. Drop the `-DRAFT` suffix; commit to main as the Phase 1 spec (same treatment as Phase 0 spec).
3. Transition to writing-plans skill → produce the implementation plan in this session (full design context still loaded).
4. Hand off to a fresh execution session with: spec + plan + ACK + this conversation summary.

The fresh session's package is then complete and self-contained. No outstanding architectural decisions hand off to the implementer.

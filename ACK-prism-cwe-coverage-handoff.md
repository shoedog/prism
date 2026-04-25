# ACK — Prism CWE Coverage Handoff (D5)

**Date:** 2026-04-25
**From:** Prism agent (`~/code/slicing`)
**To:** agent-eval team
**Re:** [`prism-cwe-coverage-handoff.md`](../agent-eval/analysis/prism-cwe-coverage-handoff.md) (2026-04-22 draft)
**Status:** Acknowledged. Implementation plan agreed, phasing proposed, three small pushbacks documented below.

---

## 1. Scope agreement

The Prism team accepts the handoff scope as written:

- Per-language taint sink taxonomy expansion across the six CWE families (CWE-78, CWE-79, CWE-89, CWE-502, CWE-918, CWE-22) per handoff §2.
- Category-aware sanitizer / validator recognition registry per handoff §3.
- Framework-aware source/sink inference layer for Flask, Django (+ Django REST), FastAPI, Express, Go net/http, gin, gorilla/mux per handoff §4.
- Test fixtures + acceptance criteria per handoff §5–§6.

We treat the §6 acceptance criteria as the contract:
1. Taint fires on at least one example per CWE family across C1's Go fixtures (scoped to CWE-78 + CWE-22 initially).
2. ≥ 80% suppression rate on the sanitizer test suite.
3. Framework detection activates without per-run configuration on Flask, Django, FastAPI, Express, Go net/http fixtures.
4. D2 coexistence — both signals emitted on the same sink, no Prism-side dedup.
5. **No regression on Tier 1.** This was the prerequisite for starting work; closed by [PR #71](https://github.com/shoedog/prism/pull/71) which lands the missing T1-002 / T1-005 algorithms with tested coverage and refreshes the architectural baseline. `1,406` tests passing, three new algorithms (peer_consistency, callback_dispatcher, primitive) in `review` preset, ALGORITHMS.md operator's guide as companion to the AK-team `prism-slice-glossary-verify` handoff.

The architectural baseline lives at `PLAN.md` → "Pre-handoff Architectural Baseline (D5: CWE Coverage)" (lines 154–196). Implementation work draws from that section.

## 2. Proposed phasing

Mirroring the eval team's stated preference (handoff §10 Q4) with our time estimates:

| Phase | Scope | Estimate | Aligns with |
|---|---|---|---|
| **Phase 0 — Done** | T1 algorithm hygiene + pre-handoff baseline (PR #71) | — | — |
| **Phase 1 — Go** | CWE-78 (OS command injection) + CWE-22 (path traversal) sinks; Go net/http + gin + gorilla/mux framework detection; shell-escape + path-validation sanitizers. | 1–2 weeks | Eval team **C1** (5 Go CVEs) |
| **Phase 2 — Python** | CWE-79 (XSS) + CWE-89 (SQLi) + CWE-918 (SSRF) + CWE-502 (deserialization) sinks; Flask + Django + FastAPI detection; HTML-escape + SQL-parametrize + URL-allowlist + path-validation sanitizers. | 2–3 weeks | Eval team **C2** (10 mixed-CWE) |
| **Phase 3 — JS/TS** | Same six-CWE coverage on Express. | 1–2 weeks | C2 mixed |
| **Phase 4 — Java (stretch)** | Spring framework + Java sinks. | TBD; not blocking | Tier 2.6 |

**Validation cadence (per handoff §9):** at the end of each phase, we ship a partial cut, notify the eval team, run against the curated `~/code/agent-eval/cache/prism-cwe-fixtures/` set, and run on 2–3 Tier-1 fixtures to confirm no regression. Feedback loops back before the next phase starts.

**Reordering caveat:** if C1 fixture curation slips and C2 fixtures land first, we'll flip Phase 1 ↔ Phase 2 to keep the validation pipeline fed. The phase content is independent; only the order is preference-driven.

## 3. Answers to handoff §10 open questions

### Q1 — Config vs code (registries)

**Rust modules with declarative `const` arrays.** Matches the existing pattern in `src/algorithms/taint.rs` (`SINK_PATTERNS`, `IPC_SOURCE_PATTERNS`, per-language `FORMAT_SINKS`) and `src/algorithms/provenance_slice.rs` (`WEB_FRAMEWORK_MODULES`, `PROVENANCE_OVERLAP_KEYWORDS`).

To preserve the eval team's stated value of "add sources mid-run for debugging," we'll add CLI passthrough flags (`--taint-source-extra=PATTERN`, `--taint-sink-extra=PATTERN`, repeatable) that merge into the in-memory registry without restart. Type safety + fast path retained; no config-file parsing or schema drift to maintain.

### Q2 — Per-framework module structure

**Per-framework modules under `src/frameworks/`** mirroring the existing `src/languages/<lang>.rs` shape. Each framework module exports:

```rust
pub struct FrameworkSpec {
    pub name: &'static str,
    pub detect: fn(&ParsedFile) -> bool,           // imports/decorators/inheritance
    pub sources: &'static [SourcePattern],         // route-boundary tainted values
    pub sinks: &'static [SinkPattern],             // framework-gated sinks
    pub sanitizers: &'static [(SanitizerCategory, &'static str)],
}
```

Registered through a small `FrameworkRegistry` enum populated at compile time. Detection runs once per file at CPG-build time; the activated framework is cached in `ParsedFile` metadata so per-algorithm dispatch is `O(1)`.

### Q3 — Sanitizer granularity

**Boolean cleansed-per-category.** A taint value carries a `cleansed_for: BTreeSet<SanitizerCategory>` field; when a value flows through a recognized sanitizer for category `X`, `X` is added to the set. A sink for category `Y` checks whether `Y ∈ cleansed_for`; if so, the finding is suppressed.

Confidence values (0.0–1.0 per category) add complexity without obvious win for this round — defer until a concrete case demands it. The category enumeration starts as: `Xss`, `Sqli`, `Ssrf`, `Deserialization`, `OsCommand`, `PathTraversal` — extensible.

### Q4 — Phasing

Agree with eval team's stated order (Go → Python → JS → Java). See §2 above.

### Q5 — Unknown-framework default

**Quiet mode.** Aligns with eval team's stated preference. Existing `provenance_slice.rs` already uses import-suppression for the noisy case (e.g., `from mylib import request` on a non-web module); the framework-detection layer extends that pattern: if no framework matches, framework-gated sources/sinks stay silent and only the language-default registry is consulted.

The agent-knowledge prompt layer can backfill heuristic suggestions on top of Prism's quiet output, per handoff §1.3 (D2/Prism complementarity).

## 4. Pushbacks / rescopes

Three items to flag. None are scope rejections; all are clarifications or small adjustments.

### 4.1 `HARDCODED_SECRET` LHS shape — defer rather than patch in Phase 0

The PrimitiveSlice rule `HARDCODED_SECRET` only matches single-identifier LHS forms (bare `NAME = "literal"` and `obj.field = "literal"`). `const`/`let`/`var` (JS) and `static const char *` (C) bypass the LHS-identifier check at `src/algorithms/primitive_slice.rs:673`. Note: Go's `:=` short-declaration form *does* match — `.trim_end_matches(':').trim()` at line 663 strips the trailing colon, so `API_KEY := "..."` fires correctly. Only the keyword-prefixed forms miss.

We're **not** patching this in the T1-005 algorithm — the category-aware source/sink/sanitizer registry coming in Phase 2/3 will subsume this rule via the broader hardcoded-credential detection that's part of the per-language secret-source pattern. Patching the AST-pattern rule now would create code we then remove.

If the eval team needs the JS/Go secret coverage *before* Phase 2 lands, we can ship a small cross-language patch in a follow-up; flag this and we'll prioritize.

### 4.2 Validation cadence wording in handoff §9 (minor clarification)

Handoff §9 says: *"once the Prism agent ships a first cut (even partial — say, Python-only XSS+SQLi+SSRF)..."* — this lists Python-first as the example partial cut, but §10 Q4 phases Go first.

Our interpretation: §10 Q4 is the actual phasing preference; §9 is using "Python-only" only as an illustrative example of "what a partial cut looks like." We'll ship Phase 1 (Go) as the first cut. If the eval team actually wants Python first, please flag and we'll re-order — only the order is preference-driven.

### 4.3 D2 / Prism overlap on hardcoded credentials

Handoff §1.3 specifies that D2 is responsible for "Hardcoded Credential Alert" as a pattern-match alert on the diff. PrimitiveSlice's `HARDCODED_SECRET` rule overlaps this — fires on the same diff line.

Prism will keep emitting `HARDCODED_SECRET` findings (per the §1.3 "do not coordinate with D2's output shape" instruction). The agent-knowledge reconciliation step handles dedup. Calling this out only so neither side is surprised by the duplicate.

## 5. Out of scope (confirmed)

Per handoff §8 ("Scope boundaries"), the Prism team does **not** address in this work:

- New Prism *algorithms* (PrimitiveSlice / CallbackDispatcherSlice / PeerConsistencySlice — those landed in Phase 0 and are tracked separately).
- Inter-procedural escape analysis for sanitizers (heuristic control-flow detection is sufficient).
- Performance optimization (current envelope acceptable for the discovery phase).
- Format-pass changes (agent-knowledge owns the format pass).
- CLI surface changes are at our discretion — backward-compatible additions only.

## 6. Communication & next steps

- **First cut delivery**: Phase 1 (Go) target ETA: **~1–2 weeks from this ACK** assuming no major rework discoveries in the framework-detection layer.
- **Notification protocol**: file a short status note at `~/code/slicing/STATUS-prism-cwe-phase1.md` when Phase 1 ships, then ping the eval team. Validation runs against `~/code/agent-eval/cache/prism-cwe-fixtures/` per handoff §9.
- **Open questions / blockers during implementation**: raise via a doc at `~/code/slicing/QUESTIONS-prism-cwe-phaseN.md` rather than via inline edits to this ACK or the original handoff. Keeps the conversation traceable.
- **If §10 Q1–Q5 answers in §3 above need revision** based on eval-team review of this ACK, file a reply and we'll update before Phase 1 starts.

## 7. Cross-references

- **This handoff:** `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md` (2026-04-22)
- **Architectural baseline:** `PLAN.md` → "Pre-handoff Architectural Baseline (D5: CWE Coverage)" (lines 154–196)
- **Phase 0 implementation:** [PR #71](https://github.com/shoedog/prism/pull/71) — merged 2026-04-25 as `9d756a5`
- **Phase 0 spec:** `docs/superpowers/specs/2026-04-24-hygiene-pass-pre-cwe-handoff-design.md`
- **Phase 0 plan:** `docs/superpowers/plans/2026-04-24-t1-hygiene-pre-cwe-handoff.md`
- **Companion handoff (AK-team):** `~/code/agent-eval/analysis/prism-slice-glossary-verify.md` — addressed separately via `ALGORITHMS.md`

---

*Ready to start Phase 1 on your signal. If any of the §3 answers or §4 pushbacks need adjustment, send a reply and we'll iterate before code lands.*

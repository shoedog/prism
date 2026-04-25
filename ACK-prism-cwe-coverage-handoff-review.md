# Review of `ACK-prism-cwe-coverage-handoff.md`

**Date:** 2026-04-25
**Reviewer:** Prism agent (independent pass; did not author the ACK)
**Subject:** [`ACK-prism-cwe-coverage-handoff.md`](ACK-prism-cwe-coverage-handoff.md) (155 lines, uncommitted draft)
**Source-of-truth:** [`~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`](../agent-eval/analysis/prism-cwe-coverage-handoff.md) (632 lines, 2026-04-22)

---

## Verdict: **B — one wording fix needed before commit**

The ACK is structurally and substantively sound. Every claim except one in §4.1 maps cleanly to the handoff. After fixing §4.1 (one-line edit), it's ready to commit.

---

## What's accurate (verified against handoff + code + git)

| ACK section | Validates against | Verdict |
|---|---|---|
| §1 scope | handoff §0 + §1 + §2 + §6 acceptance criteria | ✅ Framework list (Flask, Django + DRF, FastAPI, Express, Go net/http, gin, gorilla/mux) is verbatim from handoff §4.1. The five §6 acceptance criteria are restated faithfully, including the "scope to CWE-78 + CWE-22 initially" caveat from handoff §6.1. |
| §2 phasing | handoff §10 Q4 | ✅ Phase 1 Go / Phase 2 Python / Phase 3 JS / Phase 4 Java order matches. Reordering caveat ("flip Phase 1 ↔ Phase 2 if C2 lands first") matches handoff §10 Q4's "Prism agent may reorder if a different sequence is easier." Adding gin + gorilla/mux to Phase 1 is technically beyond §10 Q4 (which names only "Go net/http") but is the smallest defensible expansion — those frameworks are listed in handoff §4.1 and are Go-specific, so phasing them with Go is natural. |
| §3 Q1 | handoff §10 Q1 | ✅ Rust modules + CLI passthrough flags addresses the handoff's "config enables mid-run debugging" value with a concrete alternative. The flag names (`--taint-source-extra`, `--taint-sink-extra`, repeatable) are concrete enough for eval-team validation. |
| §3 Q2 | handoff §10 Q2 | ✅ `src/frameworks/` per-framework modules with `FrameworkSpec` struct. Concrete architecture; eval team can push back if shape is wrong. |
| §3 Q3 | handoff §10 Q3 | ✅ Boolean `cleansed_for: BTreeSet<SanitizerCategory>` matches handoff's "boolean is simpler and probably sufficient." Category enum (`Xss`, `Sqli`, `Ssrf`, `Deserialization`, `OsCommand`, `PathTraversal`) is concrete and extensible. Good positioning. |
| §3 Q4 | handoff §10 Q4 | ✅ Direct agreement, refers to §2 table. |
| §3 Q5 | handoff §10 Q5 | ✅ Quiet mode matches handoff's stated preference; existing `provenance_slice.rs` import-suppression cited as precedent. |
| §4.2 wording clarification | handoff §9 vs §10 Q4 | ✅ The §9/§10 Q4 inconsistency is real — handoff §9 says "Python-only XSS+SQLi+SSRF" as the example partial cut while §10 Q4 lists Go first. ACK's interpretation (§10 Q4 wins as preference; §9 is illustrative) is the right read; offering to flip is the right safety net. |
| §4.3 D2 overlap | handoff §1.3 | ✅ Handoff §1.3 explicitly says "do not coordinate with D2's output shape" and §7 lists "Hardcoded Credential Alert" as a D2 alert. ACK correctly extends the §1.3 deserialization-overlap pattern to hardcoded creds. |
| §5 out of scope | handoff §8 | ✅ All five items captured: new algorithms, inter-procedural escape analysis, perf, format-pass, CLI surface. |
| §6 communication | handoff §9 | ✅ STATUS-prism-cwe-phase1.md and QUESTIONS-prism-cwe-phaseN.md are reasonable extensions of the handoff's notification protocol. |
| §7 cross-references | git log + PLAN.md | ✅ Verified: PR #71 merged as `9d756a5` (matches `git log main`); `PLAN.md:154` is "Pre-handoff Architectural Baseline (D5: CWE Coverage)"; line range 154–196 plausible (file is 254 lines). |

---

## What needs fixing — §4.1 has a factual error

**The claim (ACK §4.1, paragraph 1):**

> The PrimitiveSlice rule `HARDCODED_SECRET` only matches bare `NAME = "literal"` and `obj.field = "literal"` LHS forms. `const`/`let`/`var` (JS), `:=` (Go), and `static const char *` (C) all bypass the LHS-identifier check.

**Reality:** Go's `:=` short-declaration form **does match**. The other three forms (JS `const`/`let`/`var`, C `static const char *`) genuinely do bypass.

**Why** — `src/algorithms/primitive_slice.rs:663`:

```rust
let lhs = trimmed[..eq_pos].trim().trim_end_matches(':').trim();
```

For `API_KEY := "value"`:

- `eq_pos` = position of `=` (the second char of `:=`).
- `trimmed[..eq_pos]` = `"API_KEY :"`.
- `.trim().trim_end_matches(':').trim()` strips the trailing `:` → `"API_KEY"`.
- Passes the alphanumeric check at line 673; matches `secret_tokens` (`api_key`); fires.

For `const API_KEY = "value"` (JS) or `var API_KEY = "value"` (Go's verbose form) or `static const char *API_KEY = "value"` (C):

- The LHS contains a space (`const `, `var `, `static const char *`).
- The alphanumeric check at line 673 (`name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$')`) fails on the space.
- The rule doesn't fire.
- Those three are correctly missed.

### Suggested fix

Drop `:=` from the list and (optionally) add Go's keyword forms:

> The PrimitiveSlice rule `HARDCODED_SECRET` only matches single-identifier LHS forms (bare `NAME = "literal"` and `obj.field = "literal"`). Multi-token LHS bypass the alphanumeric-only `name` check at `src/algorithms/primitive_slice.rs:673`: `const`/`let`/`var NAME = ...` (JS), `var`/`const NAME = ...` (Go's keyword forms), and `static const char *NAME = ...` (C). Note: Go's `:=` short-declaration form *does* match — `.trim_end_matches(':').trim()` at line 663 strips the trailing colon. Only the keyword-prefixed forms miss.

Or more compact:

> ...`const`/`let`/`var` (JS) and `static const char *` (C) bypass the LHS-identifier check. (Go's `:=` short-form matches; the keyword forms `var X = ...` / `const X = ...` do not.)

Either is fine. Goal: don't claim Go is broken when it isn't.

---

## Notable positioning choices — all defensible

1. **Phase 1 = Go (not Python).** Right read of the §9 / §10 Q4 mismatch. Offering to flip is the safety valve. Keep.
2. **HARDCODED_SECRET LHS gap deferred.** The "patching now creates code we then remove" argument is strong **once §4.1 is corrected**. The Go `:=` claim being wrong slightly weakens the "scope of the gap" framing. Recommendation: fix the wording and keep the deferral. (Alternative: if the `const`/`let`/`var` forms are more limited than initially thought, ship a 5-line patch that handles `^(const|let|var|static|...)\s+IDENT\s*=` and call the deferral done. My read is the deferral is still right — the registry will subsume.)
3. **Sanitizer category enum concrete in ACK.** Right — invites push-back before code lands. Keep.
4. **CLI passthrough flag names explicit.** Right — invites validation before implementation. Keep.

---

## Verified facts (cited in the ACK)

- ✅ `9d756a5` is the merge commit for PR #71 on `main` (verified via `git log --oneline main`).
- ✅ `PLAN.md:154` is the "Pre-handoff Architectural Baseline (D5: CWE Coverage)" section.
- ✅ Handoff §4.1's framework list matches the ACK §1 framework list verbatim (Flask, Django + DRF, FastAPI, Express, Go net/http, gin, gorilla/mux).
- ✅ Handoff §6.1 contains the "scope the criterion to CWE-78 and CWE-22 initially" caveat that ACK §1 acceptance-criterion 1 cites.
- ✅ Handoff §9's "Python-only XSS+SQLi+SSRF" example genuinely conflicts with §10 Q4's Go-first preference — the §4.2 clarification is justified.
- ✅ Handoff §1.3's D2/Prism complementarity reasoning extends naturally to hardcoded creds (D2 has "Hardcoded Credential Alert" per §7 cross-references). ACK §4.3 extension is consistent.
- ⚠️ ACK §4.1 claim about Go `:=` is incorrect (see "What needs fixing" above).

---

## Recommended action

**Option B** (adjust first): fix §4.1 wording per the suggested rewrite above, then commit. One-line edit.

If you'd rather skip the precision fix and commit as-is (Option A): the ACK is still 95% correct. The eval team is unlikely to grep `primitive_slice.rs:663` to test the claim — but if they do, they'll find the inconsistency. Better to fix.

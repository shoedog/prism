# Review of `2026-04-25-phase1-cwe-go-design.md` (§1–§2)

**Date:** 2026-04-25
**Reviewer:** Prism agent (independent pass; did not author the design)
**Subject:** [`2026-04-25-phase1-cwe-go-design.md`](2026-04-25-phase1-cwe-go-design.md) (235 lines, mid-brainstorm)
**Scope:** §1 Scope & deliverables · §2 Framework detection layer design (Commit 1). §3–§4 still pending; not reviewed.

---

## Verdict

**Solid foundation.** One real internal inconsistency to resolve, plus four small clarity / completeness items. **Signal to continue to §3** once these are addressed (or after a quick consensus call — none are scope-altering).

---

## Real issue — §1 Commit 1 contradicts §2.5/§2.8

§1 Commit 1 says:

> Wire framework detection into `CpgContext::build()` — runs once per file at CPG construction time.

But §2.5 says:

> Populated lazily on first call to `parsed.framework()`. `OnceCell` keeps it cheap and thread-safe.
> Alternative considered: compute eagerly at `CpgContext::build()`. Eager is fine but lazy is simpler.

And §2.8 says:

> Pull model: the framework data is queried at analysis time, not pre-tagged.

These pick opposite models. §2.5/§2.8 explicitly chose lazy + pull and gave reasoning; §1 reads as if eager + push were the plan. Recommend keeping the lazy choice and rewording the §1 Commit 1 bullet:

> Add `parsed.framework()` accessor backed by `OnceCell<Option<&'static FrameworkSpec>>` — populated lazily on first call. No integration with `CpgContext::build()` (lazy avoids build-pipeline coupling).

---

## Small items

### S1 — §1 Commit 2: argument-shape refinement only addresses the shell-wrapper case

The bullet enumerates `exec.Command("sh","-c",X)` / `bash` / `cmd.exe` and notes that other forms shouldn't fire. But handoff §2.5 explicitly calls out **two** CWE-78 cases:

1. **Shell-wrapped:** literal `"sh"`/`"bash"`/`"cmd.exe"` + literal `-c`/`/c` + tainted command string. ✅ covered.
2. **Tainted binary:** `exec.Command(taintedPath, args...)` where the first arg itself is tainted (a CWE-73 / CWE-78 hybrid, less common but still in the handoff). Not mentioned.

Either explicitly fold (2) into Commit 2 (one extra `SinkPattern` with `tainted_arg_indices: &[0]` and no shell-string check) or add a sentence to §1 saying "tainted-binary case is deliberately deferred — handoff §2.5 lists it but C1 fixtures don't exercise it." Otherwise it'll get caught in Phase 1 acceptance.

### S2 — §2.4 `FrameworkRegistry` enum is shown but unused

The `enum FrameworkRegistry { NetHttp, Gin, GorillaMux }` appears alongside `ALL_FRAMEWORKS: &[&'static FrameworkSpec]` and `detect_for() -> Option<&'static FrameworkSpec>`. Dispatch goes through the `&'static FrameworkSpec` reference — the enum variants aren't read anywhere in the snippet. Either:

- **Drop the enum** — the static-ref is sufficient for everything shown.
- **Use it** — e.g., add `FrameworkSpec.kind: FrameworkRegistry` for telemetry / logging (`"detected framework: Gin"`), or `FrameworkRegistry::all() -> &[Self]` as the canonical iteration.

Not blocking, but the snippet as written has dead code that'll prompt the reviewer ("what's this enum for?").

### S3 — §2.2 `SourcePattern.tainted_arg` field name is ambiguous

`tainted_arg: Option<usize>` reads two ways:

- "this arg is tainted on input" (the source consumes a tainted arg)
- "this arg becomes tainted on output" (the source produces taint into a passed-by-reference arg)

The doc-comment clarifies it's the latter (`c.BindJSON(&v)` taints `&v`). A rename to `taints_arg: Option<usize>` (verb form) or `taints_output_arg: Option<usize>` removes the ambiguity. One-line fix; worth doing before code lands and other readers have to reach for the doc-comment.

### S4 — §2.6 unstated edge case: multiple `*http.Request` params

The "capture the parameter name (commonly `r`, sometimes `req`)" logic implies one. Edge cases:

- Middleware combiners: `func chain(r1, r2 *http.Request)` — pathological but possible.
- Test utilities: `func compareRequests(want, got *http.Request)` — common in tests.

Not a Phase 1 deliverable, but two options: (a) bind **all** matching parameter names (each one becomes a candidate prefix), or (b) note the limitation explicitly ("first `*http.Request` param wins; subsequent params not modeled"). Pick one and add a sentence.

### S5 — §2.9 test plan missing the "no framework" baseline

The four test cases cover positive, two negatives, and disambiguation. Missing: a plain Go file with **no web imports** → `detect_for` returns `None`. This is the quiet-mode default per ACK §3 Q5 — should be pinned by a test, otherwise a future regression that auto-detects every Go file as net/http would slip past the suite.

---

## Optional polish — §2.3 framing

> First-match wins with the registry ordered `[gin, gorilla_mux, nethttp]` — more specific frameworks (gin builds on net/http) take precedence over the lower-level layer they're built on.

"gin builds on net/http" is imprecise — gin uses its own httprouter and only touches net/http at the lowest level. The intent ("router frameworks take precedence over the base layer") is right; suggested rewording:

> First-match wins with the registry ordered `[gin, gorilla_mux, nethttp]`. Router frameworks (gin, gorilla/mux) take precedence; net/http is the fallback for any file with `*http.Request` parameters that isn't already claimed by a more-specific router.

Optional — no design impact.

---

## Strengths (worth keeping verbatim)

- **Three-commit decomposition** maps cleanly to the three Phase 1 acceptance criteria (commit 2 → criterion 1, commit 3 → criterion 2, commit 1 → infrastructure for both). Easy to land incrementally.
- **Concrete struct definitions** in §2.2 are reviewable as code, not just shape sketches. The `SourcePattern` / `SinkPattern` separation is right (sources have `origin` + `tainted_arg`; sinks have `category` + `tainted_arg_indices` + `semantic_check`).
- **Out-of-scope section** explicit — particularly the HARDCODED_SECRET LHS deferral (now correctly grounded in the corrected ACK §4.1).
- **Mirrors `src/languages/<lang>.rs`** structure — consistent with existing patterns the team will recognize.
- **Lazy OnceCell + pull-model** are the right architectural choices (contradicts §1 only because §1 wasn't updated; the §2 reasoning is solid).
- **Test plan negatives are right shape** — wrong-import, vendored, disambiguation. Just missing the baseline (S5).

---

## Recommendation

Address the §1/§2 inconsistency (the real issue) and decide on S1 (CWE-78 tainted-binary case — fold in or explicitly defer). The rest are wording / completeness polish that can land alongside §3 or in a final pass. Then signal and the reviewer will move to §3 (sink + sanitizer registries).

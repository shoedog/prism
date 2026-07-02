# Phase-IP PR-1 — dual code-review follow-ups (deferred / pushed-back)

> **Status:** Archived query-layer note. See `docs/features/query-layer/README.md` for current docs and the local archive README for routing.

Whole-branch codex + claude review of `phase-ip-interface` (13 task commits, `e03f547..HEAD`),
2026-06-15. **Fixed-in-branch** findings (BLOCKER embedded-interface fail-closed; MAJOR
generic-gate-signature-scope; MAJOR `var *T` liveness; MINOR `parenthesized_type`; NIT dead
trim) landed in commit `682a340`. This doc records the findings **deferred** or **pushed back**,
with rationale, so a future implementor doesn't have to re-derive them.

claude verdict: APPROVE-WITH-NITS. codex verdict: CHANGES-REQUESTED (2 BLOCKER, 4 MAJOR). The
two real bugs codex surfaced (and claude missed) are fixed; the rest are below.

---

## Deferred (out of PR-1 scope) — each with priority / why / impact / fix-sketch

### D1 — CHA candidate identity is name-based, not record/method source identity (codex BLOCKER 2)
- **Priority:** Important (precision), but **out of §17 scope** and **not introduced by PR-1**.
- **Why deferred:** spec §17 / Task 12 scope is *cross-language* leakage only. The Step-9 CHA
  candidate index has *always* matched `func_index` by method **name** (pre-PR-1, globally).
  Task 12 strictly *narrows* this (name **+** `RecordInfo.file` ownership), so it can only
  *drop* candidate edges, never add a wrong one — it is an improvement, not a regression.
- **Impact:** a C++ free function or unrelated class method sharing a virtual method's name,
  **in an owned file**, can still receive a CHA Exact edge. Pre-existing; no current corpus is
  C/C++, so unmeasured.
- **Fix-sketch:** carry clang source-owner identity (record + method) through `TypeDatabase`
  and match CHA candidates on that identity instead of `(name, file)`.

### D2 — repo-global bare-name keys collide across packages (codex MAJOR 2)
- **Priority:** Important — but the **documented, owner-accepted** `CrossPackageBareName`
  over-approximation (spec §15) + the owner-locked "uncapped Exact" decision.
- **Why deferred:** interface/concrete/admission keys are bare names by design in PR-1; precise
  package keying is already on the **spec PR-2 work-list**. The over-approximation is telemetered
  (`interface_overapprox["CrossPackageBareName"]`).
- **Impact:** in a multi-package repo, `a.Runner` vs `b.Runner` (or two same-named satisfiers)
  can combine/overwrite and mint a wrong Exact edge. Bounded by the §13.7 barrier-precision
  guard; the (deferred) corpus precision gate is the eventual control.
- **Fix-sketch:** key interfaces, concrete types, and satisfiers by package/import identity
  (Go package dir), as in the spec's PR-2 receiver-expansion + package-precise-keys work.

### D3 — `DispatchProvider::resolve_dispatch` inconsistent with promoted-method satisfaction (codex MAJOR 4)
- **Priority:** Low.
- **Why deferred:** `resolve_dispatch` is an **unused legacy path** for Phase-IP — the dispatch
  flows through `CallGraph::interface_impls` + the resolution seam, not this `DispatchProvider`
  method. claude confirmed **zero in-tree non-test consumers** (only a doc comment references it).
- **Impact:** none on Phase-IP. The method would under-report promoted-method targets *if* a
  future consumer used it for Go interface dispatch.
- **Fix-sketch:** either reuse `sat_keys` / `compute_interface_dispatch` in `resolve_dispatch`,
  or document it as explicitly non-Exact / unsupported for promoted Go interface dispatch.

### D4 — broader RTA liveness gaps (codex MAJOR 1, residual)
- **Priority:** Low (recall; never precision).
- **Why deferred:** accepted spec §8 ("partial liveness, kept-Exact fallback covers"). The sharp
  `var p *T` admission-key sub-case **is fixed** (commit `682a340`). Remaining: unexported
  (lowercase-first-filtered) in-repo types never go live; address-taken locals
  (`p := P{}; _ = &p`) don't record `*P`.
- **Impact:** for an interface with ≥1 *other* live satisfier (so the fallback doesn't fire), a
  satisfier reachable only via an un-tracked construction form is dropped — a recall miss, never
  a wrong edge.
- **Fix-sketch:** prefer a known-concrete-types set over the uppercase-first filter; track
  `&local` address-of and short-var-decl construction in `scan_go_node`.

### D5 — pointer-EMBED promoted admission `Wrap struct{ *Base }` (pre-existing Task-4 deferral)
- **Priority:** Low (recall). Already noted at Task 4.
- **Impact:** a value wrapper embedding `*Base` may admit only as `*Wrap` (not `Wrap`); a
  `Wrap{}` value construction wouldn't mark it live. Recall conservativeness.
- **Fix-sketch:** thread embed-field pointer-ness through `promoted_struct_methods` so promoted
  methods land in `set_value`/`set_ptr` per the embedded field kind.

### D6 — CHA absolute-vs-repo-relative path canonicalization (pre-existing Task-12 deferral)
- **Priority:** Important for real C++ CHA recall; **safe** (conservative — drops, never mints).
- **Impact:** with absolute `compile_commands.json` paths vs repo-relative CPG keys, the exact
  `owned.contains(file)` match misses → CHA enrichment disabled for real C++ (no current corpus).
- **Fix-sketch:** canonicalize both sides to repo-relative before the ownership set test.

---

## Pushed back (verified NOT a bug — no change made)

### P1 — clear `dispatch_overapprox` in `compute_satisfaction` (claude M3)
`dispatch_overapprox` is populated at **extract time** (`go.rs:322`, `CrossPackageBareName`),
whereas `dispatch_gaps` is cleared+repopulated **solely inside** `compute_satisfaction`
(`go.rs:895/905/919`). Adding `data.dispatch_overapprox.clear()` there would **wipe** the
extract-time telemetry. The clear-asymmetry is correct; `compute_satisfaction` runs exactly once
per fresh `GoTypeData`, so there is no double-count footgun. **No change.**

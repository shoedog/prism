# Python/JS Self-Receiver Same-File Owner Narrowing — Design (2026-06-22)

> Slice **1a** of the Python/JS resolution-maturity initiative ("finish the self story"). A precision +
> soundness fix for `self`/`this`/`cls` method calls. Inheritance MRO is the separate **1b** follow-on
> (out of scope here). Supersedes the slice-1 framing in
> `handoffs/2026-06-23-python-js-receiver-typing-handoff.md` — see **§1 Premise correction**.

## 1. Premise correction (what verify-first found)

The handoff scoped slice 1 as "open the `Rust|Go` language gates so Python/JS `self.method()` can resolve
at all (`self_receiver≈0`)." **A live end-to-end trace of the production `build` path refuted that
premise.** Same-class implicit-self/this resolution **already works**, and is already language-neutral:

- `Language::call_function_qualifier` (`src/languages/mod.rs:701`) already extracts the receiver for
  Python `attribute`→`object` and JS/TS `member_expression`→`object`, so `self.method()` arrives at
  resolution with `qualifier = Some("self")` in the production path.
- The self arm (`src/resolution.rs:918-936`) is **already language-neutral**: `qualifier ∈
  {self,this,cls}` → `method_owners.get(caller)` → `owner_lookup` → relabel `SelfReceiver`.
  `Language::method_owner` covers Python + JS/TS/Tsx.
- The handoff's named gate (`recover_self_receiver_qualifier`, `src/call_graph.rs:1827`, `Rust`-only) is
  a **textual fallback reached only when the AST qualifier is `None`** — which never happens for
  Python/JS dot-calls. Opening it is a **no-op** for the production path.

**Empirical confirmation** (release `prism nav --no-cache call-stats`, this build):

| Corpus | total calls | `self_receiver` Exact | direct `self.x()` in source | `self_receiver` NameOnly | unresolved |
|---|--:|--:|--:|--:|--:|
| fastapi | 19,919 | 75 | 85 (→ **88% resolve**) | — | 59% |
| pydantic | 65,645 | 631 | — | **1,316** | 54% |

Fixtures: same-class `self.helper()` / `this.helper()` → `self_receiver` Exact; inherited
`self.base_method()` → unresolved.

So self-resolution is **not** the gap. The two real gaps in the self story are:

1. **Precision (this slice, 1a):** the **1,316** pydantic `self_receiver` NameOnly are cross-module
   **owner-key collisions** — pydantic defines **142 class names in >1 file** (`Model` ×72, `Foo` ×24,
   `Response` ×19, …). `owner_lookup` keys on the *bare* class name, so `self.method()` matches the
   method on *every* same-named class and demotes to NameOnly.
2. **Recall (follow-on, 1b):** inherited `self.method()` (method on a base class) — needs an MRO walk.
   Out of scope here.

### 1.1 The latent false-positive (the soundness lever)

Today, when the caller's own class does **not** define the method but a same-named class in **another
file** does, `owner_lookup` returns that single unrelated candidate as a **full-confidence Exact**.
Demonstrated:

```
# a.py
class Widget:
    def render(self): ...        # the only Widget.render in the repo

# b.py
class Widget:                    # SAME bare name, different file, NO render
    def draw(self):
        return self.render()     # binds Exact (score=1.00, self_receiver) to a.py Widget.render — WRONG
```

`prism nav callees --symbol draw` reports `draw@b.py` → `a.py:2-3 render` `score=1.00`. An LLM consumer
is told `b.Widget.draw` calls a method in an unrelated class. This slice closes that FP.

## 2. Goal

For `self`/`this`/`cls` method calls in **single-file-class languages (Python, JavaScript, TypeScript,
Tsx)**, narrow the owner-index lookup to candidates defined in the **caller's own file**. `self`/`this`/
`cls` provably denotes the caller's lexically-enclosing class, whose body is contiguous in one file.
Result: cross-module same-class-name collisions resolve to the caller's actual class (**Exact** instead
of NameOnly), and an inherited/absent method never binds to an unrelated same-named class (**FP closed**;
left unresolved for the 1b MRO follow-on).

## 3. Mechanism

### 3.1 The single change site

`resolve_call_site` (legacy) delegates to `resolve_call_site_full` (`src/resolution.rs:1288-1290`), so the
**only** site to change is the self arm at `src/resolution.rs:918-936`. (Lines 843, 1629, 1652 are Rust
`::`-path parsing, not the dot-receiver arm.)

### 3.2 New helper (owner narrowing for self-calls)

```rust
/// `self`/`this`/`cls` provably denotes the caller's own enclosing class, whose
/// body is contiguous in a single file (Python/JS/TS). Narrow the owner index to
/// candidates in `caller_file`: resolves cross-module same-class-name collisions to
/// the caller's actual class, and refuses to bind to an unrelated same-named class
/// in another file. Returns None when no same-file candidate exists (inherited or
/// external — drop here; the 1b MRO follow-on resolves the inherited case). Kind is
/// QualifiedOwner so the caller's existing SelfReceiver relabel fires unchanged.
fn self_owner_lookup_same_file(
    &self,
    owner: &str,
    name: &str,
    caller_file: &str,
) -> Option<Vec<ResolvedCallee<'_>>> {
    let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
    let same_file: Vec<&FunctionId> =
        ids.iter().filter(|fid| fid.file == caller_file).collect();
    match same_file.len() {
        0 => None,                                              // inherited/external → drop (1b)
        1 => Some(exact(same_file, ResolutionKind::QualifiedOwner)),
        _ => Some(demoted(same_file, ResolutionKind::QualifiedOwner)), // same-file dup class/overloads → NameOnly
    }
}
```

### 3.3 The arm, gated

```rust
Some(q)
    if q == "self" || q == "this" || q == "cls"
        || self.receiver_vars.get(caller).map(String::as_str) == Some(q) =>
{
    if let Some(owner) = self.method_owners.get(caller) {
        let narrow = matches!(
            crate::languages::Language::from_path(&caller.file),
            Some(crate::languages::Language::Python)
                | Some(crate::languages::Language::JavaScript)
                | Some(crate::languages::Language::TypeScript)
                | Some(crate::languages::Language::Tsx)
        );
        let looked_up = if narrow {
            self.self_owner_lookup_same_file(owner, name, &caller.file)
        } else {
            self.owner_lookup(owner, name)   // Go receiver-vars, others: UNCHANGED
        };
        if let Some(mut resolved) = looked_up {
            for callee in &mut resolved {
                if callee.kind == ResolutionKind::QualifiedOwner {
                    callee.kind = ResolutionKind::SelfReceiver;
                }
            }
            return ResolutionOutcome::hit(resolved);
        }
    }
    ResolutionOutcome::dropped(DropReason::UnknownName)
}
```

`receiver_vars` is **Go-only** (populated from `go_receiver_var`), so for Python/JS/TS the arm is only
ever entered via `self`/`this`/`cls`; the language gate cleanly separates the narrowed path (Py/JS/TS)
from the unchanged path (Go receiver-vars, anything else). `promoted_aliases` (Go embedding) is not
consulted on the narrowed path because Go is excluded — no behavior loss.

### 3.4 Why same-file is sound for Py/JS/TS (and wrong for Go/Rust)

A Python/JS/TS class body is a single contiguous block in one file, so every method of the caller's class
is in `caller.file`. `method_owners.get(caller)` derives the owner from the caller's own function node
(in `caller.file`), so the authoritative candidate is necessarily same-file. **Go** methods (`func (r T)
M()`) and **Rust** `impl T` blocks legitimately span multiple files within a package/crate — same-file
narrowing would wrongly drop real edges there, so they keep the existing `owner_lookup`.

### 3.5 Case table (Python/JS/TS self-calls)

| Case | candidates `methods[(C,m)]` | same-file | today | after 1a |
|---|---|---|---|---|
| same-class, no collision | 1 (caller's file) | 1 | Exact | **Exact** (unchanged) |
| same-class, cross-file collision | N (incl. caller's) | 1 | **NameOnly** | **Exact** ✅ |
| inherited, cross-file same-name class defines m | ≥1 (none caller's) | 0 | **Exact/NameOnly to wrong class (FP)** | **drop** ✅ |
| inherited, no same-name class defines m | 0 | 0 | drop | drop (1b resolves) |
| same-file dup class / `@overload` | ≥2 (caller's file) | ≥2 | NameOnly | NameOnly (unchanged) |

No legitimate recall is lost: every row that resolved correctly before still resolves; only wrong-class
FPs flip to drop and cross-file-collision NameOnly flips to Exact.

### 3.6 Canary safety

`multi_target_exact_sites` (the wrong-singleton soundness canary) counts sites with **>1 Exact** target.
This slice only ever produces a **single** same-file Exact (len 1) or a same-file **demoted/NameOnly**
(len ≥2) or a drop — it never mints a multi-target Exact. Canary must stay byte-flat.

## 4. Scope

**In:** the self arm narrowing for Python/JS/TS; the helper; tests; `CACHE_VERSION` bump.

**Out (explicit):**
- **1b — inheritance MRO** (base-class walk; needs `class_hierarchy` plumbed from `type_providers` into
  `CallGraph`). Separate spec.
- **self.field.method() attribute chains** (`self.app.get()`) — qualifier extracts as `self.app`, needs
  field typing. Separate.
- **Typed receivers** `x.method()` (slice 2). Separate.
- **Go/Rust self behavior** — unchanged by construction.

## 5. Files

- `src/resolution.rs` — add `self_owner_lookup_same_file`; gate the self arm (§3.2–3.3). The behavior
  change.
- `src/cpg_cache.rs` — `CACHE_VERSION` 20 → 21 (resolution behavior changes; cached CPGs are stale).
- Tests (exact files pinned in the plan): Python + JS/TS discriminating fixtures + a Go non-regression
  fixture. Candidate homes: `tests/lang/python/` (create if absent), `tests/lang/javascript/`,
  `tests/lang/go/`; or `resolve_call_site_full` unit tests with hand-built `CallGraph`s (the
  cross-file-collision and same-file-dup cases are awkward to express from real source in one fixture
  file, so prefer hand-built `CallGraph` unit tests for those two, real-source fixtures for the rest).

## 6. Acceptance

- **pydantic** call-stats: `self_receiver` Exact **rises** (~+1,300 from NameOnly→Exact),
  `self_receiver` NameOnly **falls** correspondingly; `multi_target_exact_sites` **byte-flat**; total
  resolved not reduced (FP closures are small and correct).
- **fastapi** call-stats: `self_receiver` Exact stable or slightly up; no regressions.
- **FP fixture** (`b.Widget.draw` → `self.render()`): now **drops** (was wrong Exact to `a.Widget`).
- **Rust/Go corpora** (ripgrep, caddy): call-stats **byte-identical** (gate excludes them).
- **Tier-A** `--matrix-only --allow-stale-sut`: 0 regressions; `--quick` M2 dogfood P/fp unchanged.
- `cargo test` green; `cargo fmt --check` clean.
- Record the pydantic before/after `self_receiver` Exact/NameOnly split + the canary in the PR.

## 7. Risks

- **Over-narrowing a legitimately split class.** Python/JS/TS classes are single-file; the only way to get
  a same-named class's method into another file is a *different* class (the collision we're fixing) — so
  this is sound. Documented; the `≥2 same-file → NameOnly` arm absorbs the rare same-file-dup edge.
- **A real corpus where pydantic-style collisions are test-only.** pydantic's collisions are concentrated
  in its test suite (`class Model(BaseModel)` repeated across test files). The mechanism is still a
  correctness/precision fix for all code; report the prod-vs-test split if call-stats can distinguish, but
  do not gate on it.
- **Other resolvers.** Confirmed `resolve_call_site` delegates to `resolve_call_site_full`; nav `callees`
  shows the `self_receiver` kind from this arm. No parallel self-arm exists.

## 8. Pipeline

Spec (this doc) → codex xhigh spec-review (fold) → writing-plans → codex xhigh plan-review (fold) →
subagent-driven TDD → host acceptance (§6) → **final codex xhigh diff-review** → PR (owner-gated). Branch:
`self-receiver-samefile-narrowing` (off `origin/main` `10572e3`).

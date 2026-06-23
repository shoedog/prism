# Python/JS Self-Receiver Same-Class Owner Narrowing — Design (2026-06-22, rev 2)

> Slice **1a** of the Python/JS resolution-maturity initiative ("finish the self story"). A precision +
> soundness fix for `self`/`this`/`cls` method calls. Inheritance MRO is the separate **1b** follow-on
> (out of scope). Supersedes the slice-1 framing in
> `handoffs/2026-06-23-python-js-receiver-typing-handoff.md` — see **§1 Premise correction**.
>
> **Rev 2 — codex xhigh spec-review fold (verdict REWORK, all findings verified + folded):** the rev-1
> mechanism narrowed by `caller.file`, which is **not class identity** — a single file can hold two
> same-named classes (top-level redefinition, or — common in pydantic test code — `class Model(BaseModel)`
> nested in different test functions). With a cross-file `C.m` also present, file-narrowing would collapse
> a today-honest **NameOnly into a single *wrong* Exact** (empirically reproduced: `outer1.C.f`'s
> `self.m()` → both `outer2.C.m` same-file + `b.C.m` cross-file at NameOnly today; file-filter would pick
> the wrong same-file singleton). The canary (multi-Exact only) would not catch it. **Rev 2 narrows by the
> caller's class identity (file + owner-class span), which is both sound and strictly more precise** — it
> drops the absent-method case correctly and still resolves genuine same-class calls even when same-named
> classes share a file. Other folds: §3.4 soundness scoped to prism's *indexed* class-body methods (dynamic
> forms out-of-model); static/classmethod conflation subsumed by class identity (§3.5); identity-aware
> acceptance + nested-dup/static fixtures (§5–6); Java a documented follow-up (§4); separate helper, not the
> fail-open `owner_lookup_in_modules` (§3.2).

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
  a **textual fallback reached only when the AST qualifier is `None`** (production `build` → 
  `build_with_receiver_config_and_scope_graph_inputs` → `function_calls_with_qualifier_and_spans_on_lines`
  always supplies it for Python/JS dot-calls; only `build_skeleton` passes `None`). Opening it is a
  **no-op** for the production path.

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

Today, when the caller's own class does **not** define the method but a same-named class **elsewhere**
does, `owner_lookup` can return that unrelated candidate as a **full-confidence Exact**. Demonstrated:

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
is told `b.Widget.draw` calls a method in an unrelated class. This slice closes that FP — and, per rev 2,
closes the *same-file* variant of it too.

## 2. Goal

For `self`/`this`/`cls` method calls in **single-file-class languages (Python, JavaScript, TypeScript,
Tsx)**, narrow the owner-index lookup to candidates that belong to the **caller's own class** — identified
by `(file, owner-class span)`, not the bare class name. `self`/`this`/`cls` provably denotes the caller's
lexically-enclosing class. Result: cross-module same-class-name collisions resolve to the caller's actual
class (**Exact** instead of NameOnly); an inherited/absent method never binds to an unrelated same-named
class, **whether in another file or the same file** (**FP closed**; left unresolved for the 1b MRO
follow-on).

## 3. Mechanism

### 3.1 The single change site

`resolve_call_site` (legacy) delegates to `resolve_call_site_full` (`src/resolution.rs:1288-1290`), so the
**only** site to change is the self arm at `src/resolution.rs:918-936`. (Lines 843, 1629, 1652 are Rust
`::`-path parsing, not the dot-receiver arm.)

### 3.2 New build-time data: per-method owner-class identity

`self`/`this`/`cls` denotes the caller's enclosing class. Two methods belong to the same class iff they
share the **same class definition node**. Identify a class within the index by `(file, class-start-line)`
— two distinct class definitions in one file have distinct start lines (nested-in-function classes
included). Add:

```rust
// FunctionId (method) -> the start line of its owner class definition node.
// Populated wherever `method_owners` is populated, from the same ancestor walk.
pub method_class_line: BTreeMap<FunctionId, usize>,
```

Population: `method_metadata` (`src/call_graph.rs:1799`) already locates the owner via
`Language::method_owner`; extend it (or add a sibling helper `method_owner_class_node`) to also return the
**class definition node's** start line, and insert into `method_class_line` at each `method_owners.insert`
site: `build_skeleton` (`:286-292`), `build_with_receiver_config…` (`:525-530`), `build_direct_subset…`
(`:1479`), and the merge paths (`:1039`, `:1079`). `caller.file` + `method_class_line[caller]` is the
caller's class identity; `fid.file` + `method_class_line[fid]` is a candidate's.

### 3.3 New helper (owner narrowing by class identity)

```rust
/// `self`/`this`/`cls` provably denotes the caller's own enclosing class. Narrow the
/// owner index to candidates that belong to that exact class — same file AND same
/// owner-class start line — not merely the same file (two same-named classes can share
/// a file) and not merely the same bare owner name (collisions across files). Returns
/// None when no candidate belongs to the caller's class (inherited or external — drop
/// here; the 1b MRO follow-on resolves the inherited case). Kind is QualifiedOwner so
/// the caller's existing SelfReceiver relabel fires unchanged.
fn self_owner_lookup_same_class(
    &self,
    owner: &str,
    name: &str,
    caller: &FunctionId,
) -> Option<Vec<ResolvedCallee<'_>>> {
    let caller_class = *self.method_class_line.get(caller)?;
    let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
    let same_class: Vec<&FunctionId> = ids
        .iter()
        .filter(|fid| {
            fid.file == caller.file
                && self.method_class_line.get(*fid) == Some(&caller_class)
        })
        .collect();
    match same_class.len() {
        0 => None,                                                // not on caller's class → drop (1b)
        1 => Some(exact(same_class, ResolutionKind::QualifiedOwner)),
        _ => Some(demoted(same_class, ResolutionKind::QualifiedOwner)), // same-class same-name dup → NameOnly
    }
}
```

The `≥2` arm is reached only by a genuine same-class same-name duplicate — Python `@overload` stubs +
impl, or a JS/TS class carrying both `static m()` and instance `m()`. Demoting (NameOnly) is the safe,
correct outcome there (we cannot pick between them without method-kind facts — explicitly out of scope).

### 3.4 The arm, gated

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
            self.self_owner_lookup_same_class(owner, name, caller)
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

`receiver_vars` is **Go-only** (populated from `go_receiver_var`, which returns `None` unless the language
is Go — `src/languages/mod.rs:1247`), so for Python/JS/TS the arm is only ever entered via
`self`/`this`/`cls`; the language gate cleanly separates the narrowed path (Py/JS/TS) from the unchanged
path (Go receiver-vars, anything else). `promoted_aliases` (Go embedding) is intentionally not consulted
on the narrowed path because Go is excluded — no behavior loss. We keep a **separate helper** rather than
routing through `owner_lookup_in_modules`, which **fail-opens** when its narrowing empties
(`src/resolution.rs:732-735`); this design needs an empty same-class result to **drop** (to close the FP),
the opposite of fail-open.

### 3.5 Soundness — scoped to prism's indexed method model

`self_owner_lookup_same_class` is sound **relative to prism's current method index**, which contains only
**directly-defined class-body methods**: `Language::method_owner` (`src/languages/mod.rs:1056`) recognizes
Python `class`-block `def`s and JS/TS `class_body` methods. Dynamic method-definition forms — Python
monkeypatching / `setattr` / metaclass injection / decorator-moved methods; JS prototype assignment /
mixins / `Object.assign` — **never enter `methods[(C, m)]`**, so the narrowing cannot regress them (they
were never resolved here). Within that model:

- A Python/JS/TS class body is one contiguous span in one file, so every directly-defined method of the
  caller's class shares `(caller.file, method_class_line[caller])`. The candidate that belongs to the
  caller's class is therefore exactly identified — `caller.file` alone is **not** sufficient (the same-file
  duplicate-class case), but `(file, class-start-line)` is.
- Go (`func (r T) M()` across files in a package) and Rust (`impl T` blocks across files) genuinely spread
  one type's methods over multiple class-less spans — class-identity narrowing is meaningless/harmful
  there, so they are **excluded by the gate** and keep `owner_lookup`.

### 3.6 Case table (Python/JS/TS self-calls)

| Case | `methods[(C,m)]` candidates | belong to caller's class | today | after 1a |
|---|---|---|---|---|
| same-class, no same-name collision anywhere | 1 (caller's class) | 1 | Exact | **Exact** (unchanged) |
| same-class, cross-file same-name class also defines m | N (1 is caller's) | 1 | **NameOnly** | **Exact** ✅ |
| absent on caller's class; same-name class in **another file** defines m | ≥1 (none caller's) | 0 | **wrong Exact/NameOnly (FP)** | **drop** ✅ |
| absent on caller's class; same-name class in the **same file** defines m | ≥1 (none caller's) | 0 | **wrong NameOnly/Exact (FP)** | **drop** ✅ (rev-2) |
| absent everywhere (inherited) | 0 | 0 | drop | drop (1b resolves) |
| same-class same-name dup (`@overload`, static+instance) | ≥2 (all caller's class) | ≥2 | NameOnly | NameOnly (unchanged) |

No legitimate recall is lost: every row that resolved *correctly* before still resolves; only wrong-class
FPs flip to drop and cross-file-collision NameOnly flips to Exact. **Static/classmethod** (`this` in a JS
`static` method, `cls` in a Python `@classmethod`) is handled by the same identity test: it binds to the
caller's-own-class method (correct reference) or, when the class carries both static and instance `m`,
falls to the `≥2 → NameOnly` arm. prism resolves *references*; it does not validate static-vs-instance call
legality (out of scope).

### 3.7 Canary note

`multi_target_exact_sites` (the wrong-singleton canary, `src/navigation/queries.rs:245`) counts sites with
**>1 Exact** target — it is **blind to a single wrong Exact**, which is precisely the failure mode
file-narrowing (rev 1) would have introduced. Class-identity narrowing (rev 2) prevents singleton wrong
Exacts by construction; acceptance (§6) verifies this directly by **sampling new `self_receiver` Exact
sites by caller-class/target-class identity**, not by trusting the canary alone.

## 4. Scope

**In:** the `method_class_line` map + population; the `self_owner_lookup_same_class` helper; the Py/JS/TS
gate on the self arm; tests; `CACHE_VERSION` bump.

**Out (explicit):**
- **1b — inheritance MRO** (base-class walk; needs `class_hierarchy` plumbed from `type_providers` into
  `CallGraph`). Separate spec.
- **self.field.method() attribute chains** (`self.app.get()`) — qualifier extracts as `self.app`, needs
  field typing. Separate.
- **Typed receivers** `x.method()` (slice 2). Separate.
- **Method-kind (static/instance/classmethod) resolution** — references only; no call-legality typing.
- **Java** — `this.m()` has the same bare-owner collision (`method_invocation` qualifier,
  `src/languages/mod.rs:707`; class-body owners) and the **same class-identity mechanism would make it
  sound**; deferred as a clean follow-up to keep this slice on the team's Python/JS priority. Lua stays
  excluded (table/metatable methods legitimately span files).
- **Go/Rust self behavior** — unchanged by construction.

## 5. Files

- `src/resolution.rs` — add `method_class_line` field to the resolver struct; add
  `self_owner_lookup_same_class`; gate the self arm (§3.3–3.4). The behavior change.
- `src/call_graph.rs` — capture the owner-class start line in `method_metadata`/a sibling helper; populate
  `method_class_line` at every `method_owners.insert` + merge site (`:292`, `:530`, `:1039`, `:1079`,
  `:1479`). Thread the field through `CallGraph` construction.
- `src/cpg_cache.rs` — `CACHE_VERSION` 20 → 21 (new cached field + resolution behavior change).
- Tests (exact files pinned in the plan): prefer **hand-built `CallGraph` / `resolve_call_site_full` unit
  tests** for the multi-class cases (cross-file collision, same-file duplicate class, static+instance),
  which are awkward to express as single real-source fixtures, plus **real-source fixtures** for the basic
  same-class and cross-file-FP cases. Add a **Go fixture** asserting receiver-var calls are unchanged.

## 6. Acceptance

- **pydantic** call-stats: `self_receiver` Exact **rises** (target ≈ +1,300 from NameOnly→Exact, minus
  same-file/inherited FP drops), `self_receiver` NameOnly **falls**; `multi_target_exact_sites`
  **byte-flat**; total resolved not materially reduced.
- **Identity-aware regression check (folds MAJOR 4):** dump the set of `self_receiver` **Exact** sites
  before/after and confirm every *newly-Exact* site has `target` class identity == caller class identity
  (no singleton wrong Exact). The nested-duplicate fixture (`outer1.C.f` → `self.m()`) must resolve to
  **drop**, not a same-file singleton Exact.
- **fastapi** call-stats: `self_receiver` Exact stable/slightly up; no regressions.
- **FP fixtures**: cross-file `b.Widget.draw`→`self.render()` and same-file nested-duplicate both **drop**.
- **Rust/Go corpora** (ripgrep, caddy): call-stats **byte-identical** (gate excludes them).
- **Tier-A** `--matrix-only --allow-stale-sut`: 0 regressions; `--quick` M2 dogfood P/fp unchanged.
- `cargo test` green; `cargo fmt --check` clean.
- Record the pydantic before/after `self_receiver` Exact/NameOnly split + the identity-aware sample in the
  PR.

## 7. Risks

- **Class identity via start-line.** Two distinct class definitions in one file always have distinct start
  lines (nested classes included), so `(file, class-start-line)` is a sound within-file class key. If a
  candidate lacks a `method_class_line` entry it is excluded (conservative drop), never promoted.
- **pydantic collisions are test-concentrated.** Many same-named `Model` classes live in pydantic's test
  suite. The mechanism is still a correctness/precision fix for all code; report the prod-vs-test split if
  call-stats can distinguish, but do not gate on it.
- **Population coverage.** `method_class_line` must be populated at *every* site `method_owners` is
  (including merge paths) or a method's narrowing silently drops; the plan enumerates all six sites and a
  test asserts a merged graph still narrows.

## 8. Pipeline

Spec (this doc, rev 2) → codex xhigh spec re-review (fold) → writing-plans → codex xhigh plan-review (fold)
→ subagent-driven TDD → host acceptance (§6) → **final codex xhigh diff-review** → PR (owner-gated).
Branch: `self-receiver-samefile-narrowing` (off `origin/main` `10572e3`).

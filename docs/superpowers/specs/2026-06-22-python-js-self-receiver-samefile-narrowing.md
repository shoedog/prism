# Python/JS Self-Receiver Same-Class Owner Narrowing — Design (2026-06-22, rev 3)

> Slice **1a** of the Python/JS resolution-maturity initiative ("finish the self story"). A precision +
> soundness fix for `self`/`this`/`cls` method calls. Inheritance MRO is the separate **1b** follow-on
> (out of scope). Supersedes the slice-1 framing in
> `handoffs/2026-06-23-python-js-receiver-typing-handoff.md` — see **§1 Premise correction**.
>
> **Rev 2 — codex spec-review fold (REWORK):** rev-1 narrowed by `caller.file`, which is **not class
> identity** (a file can hold two same-named classes — top-level redefinition, or `class Model(BaseModel)`
> nested in different test functions, common in pydantic). With a cross-file `C.m` also present,
> file-narrowing collapses a today-honest NameOnly into a single **wrong** Exact (reproduced). Rev 2
> narrows by the caller's **class identity** instead.
>
> **Rev 3 — codex spec RE-review fold (REWORK):** the class key must be the class node's **byte span**, not
> its start *line* — legal JS/TS one-line/nested classes or named class expressions can share a start line
> (`(file, start_line)` collides). Rev 3 keys on `(file, (start_byte, end_byte))`. Also folded: the full
> population/literal/cache-test checklist (§5); honest acceptance that **measures** the buy rather than
> promising a number (§6); the `≥2`-candidate arm explicitly names the decorated wrapper+inner
> double-capture (§3.3); and **§9 records the decorated-method double-capture as the next (bigger) Python
> precision lever** — discovered while sizing this slice.

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

1. **Precision (this slice, 1a):** the **1,316** pydantic `self_receiver` NameOnly are dominated by
   cross-module **owner-key collisions** — pydantic defines **142 class names in >1 file** (`Model` ×72,
   `Foo` ×24, …). `owner_lookup` keys on the *bare* class name, so `self.method()` matches the method on
   *every* same-named class and demotes to NameOnly. (A second contributor — decorated-method
   double-capture — is **not** fixed by this slice; see §3.3 and §9.)
2. **Recall (follow-on, 1b):** inherited `self.method()` (method on a base class) — needs an MRO walk.
   Out of scope here.

### 1.1 The latent false-positive (the soundness lever — unconditional value)

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

`prism nav callees --symbol draw` reports `draw@b.py` → `a.py:2-3 render` `score=1.00`. An LLM consumer is
told `b.Widget.draw` calls a method in an unrelated class. This slice closes that FP — cross-file **and**
the same-file nested-class variant (also reproduced). This soundness fix holds regardless of the precision
buy's size.

## 2. Goal

For `self`/`this`/`cls` method calls in **single-file-class languages (Python, JavaScript, TypeScript,
Tsx)**, narrow the owner-index lookup to candidates that belong to the **caller's own class** — identified
by `(file, owner-class byte span)`, not the bare class name and not merely the file. `self`/`this`/`cls`
provably denotes the caller's lexically-enclosing class. Result: cross-module same-class-name collisions
resolve to the caller's actual class (**Exact** instead of NameOnly); an inherited/absent method never
binds to an unrelated same-named class, whether in another file **or the same file** (**FP closed**; left
unresolved for the 1b MRO follow-on).

## 3. Mechanism

### 3.1 The single change site

`resolve_call_site` (legacy) delegates to `resolve_call_site_full` (`src/resolution.rs:1288-1290`), so the
**only** site to change is the self arm at `src/resolution.rs:918-936`. (Lines 843, 1629, 1652 are Rust
`::`-path parsing, not the dot-receiver arm.)

### 3.2 New build-time data: per-method owner-class identity

`self`/`this`/`cls` denotes the caller's enclosing class. Two methods belong to the same class iff they
share the **same class definition node**. Identify a class by `(file, class-node byte span)` — byte spans
are unique per node, so two distinct class definitions in one file (top-level redefinition, nested in
different functions, one-line, or named class expressions) always differ, closing the start-line-collision
hole the re-review flagged. Add to `CallGraph`:

```rust
/// FunctionId (method) -> its owner class definition node's byte span (start, end).
/// Populated wherever `method_owners` is. `(caller.file, span)` is the class identity.
#[serde(default)]
pub method_class_span: BTreeMap<FunctionId, (usize, usize)>,
```

`#[serde(default)]` keeps cache-format safety (the field is empty on an old blob; a `CACHE_VERSION` bump
forces a rebuild anyway — see §5). Populate via a sibling helper that mirrors `method_owner`'s walk but
returns the **enclosing class definition node** (Python `class_definition`; JS/TS `class_declaration` /
`class`), not the name node:

```rust
// in Language: parallels method_owner but returns the class DEFINITION node.
pub fn method_owner_class_node<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> { /* same walk; return `cls` */ }
```

Then at each method-collection site: `let span = lang.method_owner_class_node(&func_node)
.map(|c| (c.start_byte(), c.end_byte()));` and insert into `method_class_span` alongside the existing
`method_owners.insert`. Using `method_owner_class_node` (the same walk that yields the owner) guarantees
every method of a class records the **identical** span — the linchpin for same-class siblings comparing
equal (no false drops).

### 3.3 New helper (owner narrowing by class identity)

```rust
/// `self`/`this`/`cls` provably denotes the caller's own enclosing class. Narrow the
/// owner index to candidates that belong to that exact class — same file AND same
/// owner-class byte span. Returns None when no candidate belongs to the caller's class
/// (inherited or external — drop here; the 1b MRO follow-on resolves the inherited
/// case). Kind is QualifiedOwner so the caller's existing SelfReceiver relabel fires.
fn self_owner_lookup_same_class(
    &self,
    owner: &str,
    name: &str,
    caller: &FunctionId,
) -> Option<Vec<ResolvedCallee<'_>>> {
    let caller_span = *self.method_class_span.get(caller)?;
    let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
    let same_class: Vec<&FunctionId> = ids
        .iter()
        .filter(|fid| {
            fid.file == caller.file && self.method_class_span.get(*fid) == Some(&caller_span)
        })
        .collect();
    match same_class.len() {
        0 => None,                                                // not on caller's class → drop (1b)
        1 => Some(exact(same_class, ResolutionKind::QualifiedOwner)),
        _ => Some(demoted(same_class, ResolutionKind::QualifiedOwner)), // same-class same-name dup → NameOnly
    }
}
```

The `≥2` arm is reached by a genuine same-class same-name duplicate: (a) Python `@overload` stubs + impl;
(b) a JS/TS class carrying both `static m()` and instance `m()`; **(c) decorated methods**, which prism
captures as *two* FunctionIds — the `decorated_definition` wrapper and the inner `function_definition`
(reproduced: a `@staticmethod` self-call yields two same-class candidates → NameOnly). Demoting (NameOnly)
is the safe, correct outcome — we cannot pick between them without method-kind facts (out of scope). Case
(c) is **pre-existing** (today's `owner_lookup` also returns both → NameOnly), so this slice does not
regress it; it is the buy-limiter quantified in §6 and the subject of the §9 follow-on.

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

`receiver_vars` is **Go-only** (`go_receiver_var` returns `None` unless Go — `src/languages/mod.rs:1246`),
so for Python/JS/TS the arm is only ever entered via `self`/`this`/`cls`; the gate cleanly separates the
narrowed path (Py/JS/TS) from the unchanged path (Go receiver-vars, anything else). We keep a **separate
helper** rather than `owner_lookup_in_modules`, which **fail-opens** when narrowing empties
(`src/resolution.rs:732-735`); this design needs an empty same-class result to **drop** (to close the FP).

### 3.5 Soundness — scoped to prism's indexed method model

Sound **relative to prism's current method index**, which contains only **directly-defined class-body
methods** (`Language::method_owner`, `src/languages/mod.rs:1056`: Python `class`-block `def`s, JS/TS
`class_body` methods). Dynamic forms — Python monkeypatch/`setattr`/metaclass injection; JS prototype
assignment/mixins/`Object.assign` — **never enter `methods[(C, m)]`**, so narrowing cannot regress them.
Within that model, a Python/JS/TS class body is one contiguous node in one file, so every directly-defined
method of the caller's class shares `(caller.file, class-span)`. Go (`func (r T) M()`) and Rust (`impl T`)
spread one type's methods over multiple files/nodes — class-span identity is meaningless there, so they are
**excluded by the gate** and keep `owner_lookup`. The existing `FunctionId` is line-based
(`src/call_graph.rs:20-24`); that is irrelevant here because the **class** key is byte-span — but it is the
remaining theoretical limit for two same-line same-name *methods* (out of scope; flagged in §9).

### 3.6 Case table (Python/JS/TS self-calls, true span identity)

| Case | `methods[(C,m)]` candidates | belong to caller's class | today | after 1a |
|---|---|---|---|---|
| same-class, no same-name collision anywhere | 1 | 1 | Exact | **Exact** (unchanged) |
| same-class, cross-file same-name class also defines m | N (1 is caller's) | 1 | **NameOnly** | **Exact** ✅ |
| absent on caller's class; same-name class in **another file** defines m | ≥1 (none caller's) | 0 | **wrong Exact/NameOnly (FP)** | **drop** ✅ |
| absent on caller's class; same-name class in the **same file** defines m | ≥1 (none caller's) | 0 | **wrong (FP)** | **drop** ✅ |
| absent everywhere (inherited) | 0 | 0 | drop | drop (1b resolves) |
| same-class same-name dup (`@overload`, static+instance, **decorated wrapper+inner**) | ≥2 (all caller's class) | ≥2 | NameOnly | NameOnly (unchanged) |

No legitimate recall is lost: every row that resolved *correctly* before still resolves; only wrong-class
FPs flip to drop and cross-file-collision NameOnly flips to Exact. **Static/classmethod** (`this` in a JS
`static` method, `cls` in a Python `@classmethod`) is subsumed: it binds to the caller's-own-class method
(correct reference) or, with both static+instance `m`, falls to the `≥2 → NameOnly` arm. prism resolves
*references*, not static-vs-instance call legality (out of scope).

### 3.7 Canary note

`multi_target_exact_sites` (`src/navigation/queries.rs:245`) counts sites with **>1 Exact** — it is blind
to a single wrong Exact, the exact failure mode rev-1 risked. Span-identity narrowing prevents singleton
wrong Exacts by construction; acceptance (§6) verifies directly by sampling new `self_receiver` Exact sites
by `(caller class span, target class span)`, not by trusting the canary alone.

## 4. Scope

**In:** `method_class_span` + `method_owner_class_node` + population; `self_owner_lookup_same_class`; the
Py/JS/TS gate on the self arm; tests; `CACHE_VERSION` bump.

**Out (explicit):** **1b** inheritance MRO (separate); `self.field.method()` attribute chains (field
typing); typed receivers (slice 2); method-kind resolution; **decorated double-capture** (§9 — the next
precision slice); **Java** (`this.m()` has the same collision; the same class-span mechanism would make it
sound — documented follow-up; kept out for the team's Python/JS priority); Lua (table methods span files);
Go/Rust self behavior (unchanged by construction).

## 5. Files & population checklist (from the re-review)

- `src/resolution.rs` — add the `method_class_span` field to `CallGraph`; add
  `self_owner_lookup_same_class`; gate the self arm (§3.3–3.4).
- `src/languages/mod.rs` — add `method_owner_class_node` (Python/JS/TS; mirror `method_owner`'s walk,
  return the class node).
- `src/call_graph.rs` — populate `method_class_span` at **every** `method_owners` write + thread the field
  through **every** `CallGraph` construction/merge. Enumerated:
  - field declaration (next to `method_owners`, `:164`);
  - `CallGraph::empty` (`:227`); build literals: `build_skeleton` (`:368`), full build (`:970`),
    `build_direct_subset` (`:1581`);
  - inserts: `build_skeleton` (`:292`), full build (`:530`), `build_direct_subset` (`:1479`);
  - merge/prune: `extend` paths (`:1079` and the `:1039` block), and `remove_files`/retain (drop entries
    for removed files);
  - the per-file extraction structure (the parallel `FileFunctions`-style tuple that feeds `method_owners`
    under rayon) must carry the span so the merge preserves it — the plan pins the exact type.
- `src/cpg_cache.rs` — `CACHE_VERSION` 20 → 21; update the version assertion test (`~:570-572`).
- Tests — prefer **hand-built `CallGraph` / `resolve_call_site_full` unit tests** for the multi-class cases
  (cross-file collision, same-file nested duplicate, **same-line** JS/TS duplicate, static+instance,
  decorated wrapper+inner), plus real-source fixtures for the basic same-class and cross-file-FP cases, and
  a **Go fixture** asserting receiver-var calls are unchanged. A test must assert a **merged** graph
  (`extend`) still narrows (population-coverage guard).

## 6. Acceptance

- **pydantic** call-stats: **measure** the `self_receiver` Exact↑ / NameOnly↓ delta (do **not** assert a
  fixed number — the buy is the cross-file-collision subset of the 1,316; decorated double-captures and
  inherited calls stay NameOnly/drop). Report the actual split in the PR. `multi_target_exact_sites`
  **byte-flat**.
- **Identity-aware regression check:** dump `self_receiver` **Exact** sites before/after; every
  *newly-Exact* site must have `target` class span == caller class span (no singleton wrong Exact). The
  nested-duplicate **and same-line** fixtures (`outer1.C.f` → `self.m()` with `outer2.C.m`) must **drop**.
- **fastapi**: `self_receiver` Exact stable/slightly up; no regressions.
- **FP fixtures**: cross-file (`b.Widget.draw`→`self.render()`) and same-file nested both **drop**.
- **Rust/Go corpora** (ripgrep, caddy): call-stats **byte-identical** (gate excludes them).
- **Tier-A** `--matrix-only --allow-stale-sut`: 0 regressions; `--quick` M2 dogfood P/fp unchanged.
- `cargo test` green; `cargo fmt --check` clean.

## 7. Risks

- **Class identity via byte span** is collision-free (distinct nodes → distinct spans); a missing
  `method_class_span` entry excludes that candidate (conservative drop), so population coverage (§5) is the
  real risk — guarded by the merged-graph test.
- **pydantic collisions are test-concentrated** (many `class Model` in tests). The mechanism is a
  correctness/precision fix for all code; report prod-vs-test split if measurable, don't gate on it.
- **Modest precision buy.** With ~20% of pydantic methods decorated (double-captured → NameOnly), the
  Exact upgrade is bounded; the unconditional value is the FP closure. Accepted (§9 is the larger lever).

## 8. Pipeline

Spec (rev 3, all spec-review findings folded) → writing-plans → codex xhigh plan-review (fold) →
subagent-driven TDD → host acceptance (§6) → **final codex xhigh diff-review** → PR (owner-gated). The
remaining adversarial gates are plan-review + the final diff-review (the start_byte fold is mechanical and
codex-prescribed, so no 3rd spec round). Branch: `self-receiver-samefile-narrowing` (off `origin/main`
`10572e3`).

## 9. Discovered follow-on — decorated-method double-capture (the next Python precision lever)

While sizing 1a, call-stats + a fixture showed prism captures every **decorated** method as **two**
FunctionIds (the `decorated_definition` wrapper + the inner `function_definition`), so **every call to a
decorated method** — `self.validator()`, `obj.prop()`, `C.helper()` — collapses to **NameOnly** (≥2
same-class candidates). pydantic: **~418 of ~2,066 class methods (~20%) are decorated**
(`@property`/`@*validator`/`@staticmethod`/…); fastapi/Django/JS decorators (and TS) are similar. This
degrades resolution **across the whole dynamic stack**, not just self-calls, and is plausibly a **larger
precision buy than 1a**. It is out of scope here because the fix lives in the **function-extraction /
FunctionId-identity layer** (dedup wrapper+inner into one logical method, or prefer the inner node) —
broader blast radius, needs its own verify-first (what consumes both nodes: `all_functions`, the inventory
at `src/navigation/inventory.rs`, the calls index). **Recommended as the next slice after 1a.**

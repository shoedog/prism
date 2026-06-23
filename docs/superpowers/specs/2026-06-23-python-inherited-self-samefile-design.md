# Slice 1b — Inherited `self`/`this` Resolution, SAME-FILE bases (Python/JS/TS) — Design

> Spec-of-record, Python-maturity loop slice 1b (last mandate slice). Recall companion to 1a
> (#131). Scoped to **same-file base classes** (cross-file bases deferred to slice 3's parked
> import model + its owner decision). **Front-loads the slice-3 review lessons** (occurrence
> hygiene, wildcard-poison, the bounded-static contract, canary-is-blind). Pairs with the loop
> handoff + `[[project_prism_measurement_maturity]]`.

## Goal

Resolve `self.method()` / `this.method()` where `method` is not on the caller's own class but
**is** defined on a **same-file base class** — currently dropped as `UnknownName` by 1a's
same-class narrowing. Pure-recall win, no precision risk (these are drops today; the design
mints Exact only on a single unambiguous same-file provider). Buy: most of the measured 16
in-repo inherited sites (FastAPI 12, Pydantic 4) whose base is same-file; cross-file bases
(e.g. Pydantic's imported `BaseConfig`) are **deferred** (need slice 3's import resolution).

## Framing: this is the inheritance analogue of R4 (no new soundness contract)

prism's **R4 same-file `LocalDef`** (`resolution.rs:1248`) already resolves a same-file free
function under prism's standing **bounded-static assumption** (it does not disprove
`globals()`/`exec` rebinding — no prism rung does; same as pyright/mypy/all IDEs). Slice 1b is
the same move for the class hierarchy: resolve a `self.m()` call to a method on a **same-file**
base class, found by class **identity** (file + byte-span, preserving 1a). It relies ONLY on
prism's existing contract — **no new owner decision** (unlike slice 3's cross-file imports,
which are higher dynamic-risk and parked on that decision). The one syntactically-detectable
hole — a `from x import *` that could supply a base name shadowing the same-file class — is
closed by **wildcard-poison** (below). `globals()`/`exec` class rebinding is out of scope =
prism's standing assumption.

**Canary caveat (slice-3 lesson):** `multi_target_exact_sites` only counts ≥2-Exact sites, so
it CANNOT catch a single wrong inherited Exact. Soundness here is gated by **design + the
adversarial diff-review**, not the canary. Every gate fails open (→ the existing `UnknownName`
drop), never to a wrong Exact.

## Architecture (span-keyed same-file class hierarchy)

### Data model

```rust
// Ordered base links for one class. Order = textual base order (MRO left-to-right approx).
pub enum ClassBaseLink {
    SameFile((usize, usize)),   // base resolved to a same-file class by byte-span (its method_class_span)
    ExternalOrAmbiguous,        // base name not a unique same-file class (external, 0, or >1) — an MRO BARRIER
}

// on CallGraph, serialized:
pub class_bases: BTreeMap<FunctionId, Vec<ClassBaseLink>>,
```

Keyed by the class's representative `FunctionId` — reuse the **same identity 1a uses**: a class
is identified by `(file, method_class_span)`. `class_bases` is keyed per the class-owning span;
in practice we store it keyed by the same `FunctionId`→span mapping `method_class_span` uses, so
the resolver can go caller `FunctionId` → caller class span → bases. (Implementation detail for
the plan: a parallel `BTreeMap<(file, span), Vec<ClassBaseLink>>`, or fold into the
`method_metadata` tuple 1a threads — the plan picks the cleanest; the span identity is
non-negotiable.)

**Base-name → SameFile resolution (build time):** for `class Child(Base): …`, resolve `Base` to
a **same-file** class named `Base`. If exactly **one** same-file class is named `Base` →
`SameFile(its_span)`. If **0** (base is external/imported) or **>1** (same-name collision in
file) → `ExternalOrAmbiguous`. Cross-file/imported bases are **always** `ExternalOrAmbiguous`
in this slice (no import resolution) → a barrier (so an imported base never mints an edge — the
recall is deferred, soundly).

**Wildcard-poison:** if the caller's file contains any `from x import *` (Python) /
`import * as`/`export *` shadow risk (JS/TS), a base name could be supplied by the wildcard
rather than the same-file class → mark **all** of that file's `ClassBaseLink`s
`ExternalOrAmbiguous` (the whole file's inherited resolution falls open). Detected
syntactically at extraction (the import walk already sees wildcard nodes — cf. slice-2's `"*"`
sentinel).

### Extraction helper

Add `Language::class_base_names(class_node) -> Vec<String>` beside 1a's
`method_owner_class_node` (`languages/mod.rs`), Python/JS/TS only: Python `class C(A, B)` →
`["A","B"]` from the `argument_list` (skip keyword args like `metaclass=`); JS/TS
`class C extends A` → `["A"]` from `class_heritage`. Lives in the languages/build layer, not
resolution.

### Resolution hook

In the self-arm, after `self_owner_lookup_same_class` returns `None` (`resolution.rs:733`/the
self-arm at ~`:944`), before the `UnknownName` drop, gated Python/JS/TS:

```
self_owner_lookup_inherited(caller, name):
    caller_span = method_class_span[caller]               // 1a identity; None -> give up (drop)
    visited = {caller_span}
    return walk(caller.file, caller_span, name, visited)

walk(file, class_span, name, visited):
    for link in class_bases[(file, class_span)] (IN ORDER):   // MRO left-to-right
        match link:
          ExternalOrAmbiguous => return None                  // BARRIER: a base we can't see may define `name`; stop
          SameFile(base_span):
              if base_span in visited { return None }          // cycle guard
              visited.insert(base_span)
              hits = methods[(base_owner, name)] filtered to (file, base_span)   // span-exact, never bare owner_lookup
              match hits.len():
                1 => return Exact SelfReceiver([hit])           // single unambiguous provider
                >1 => return None                               // ambiguous provider -> drop (conservative)
                0 => { r = walk(file, base_span, name, visited); if r.is_some() { return r } }  // not here -> deeper base
    return None
```

(`base_owner` = the base class's owner key, available from the span→owner mapping the build
records.) **Soundness invariants:**
- Span-exact filtering only — **never** bare `owner_lookup(base_name, name)` (would bind an
  external base to a same-named in-repo class — the exact 1a FP class).
- **`ExternalOrAmbiguous` is a hard MRO barrier:** if encountered before an in-repo hit, return
  `None` (drop). We do NOT look past a base whose contents we can't see (it might define `name`).
- Exact only on a **single** same-file provider → cannot raise `multi_target_exact_sites`.
- Cycle-guarded; same-file only (bounded recursion within one file's class graph).
- Python/JS/TS-gated → Rust/Go byte-identical. Every uncertainty → the existing `UnknownName`
  drop (never a new wrong Exact).

## Scope guards (first merge)
- **Same-file bases only.** Cross-file/imported bases = `ExternalOrAmbiguous` (deferred to
  slice 3's import model + owner decision).
- Single + multiple inheritance both handled, but only via the ordered same-file walk with the
  barrier rule; no C3 (a `>1 provider` or a barrier → drop).
- Wildcard-poison per file (above).
- Preserve 1a canaries: cross-file same-name-class stays Exact-only-for-caller-class; absent
  same-class method without an in-repo base stays dropped.

## Plumbing + cache
`class_bases` threaded through `empty`/full/skeleton/subset builds + `remove_files` (drop
entries whose file is excluded) + `merge` (extend), mirroring 1a's `method_class_span`
(`call_graph.rs` template). **Bump `CACHE_VERSION` 23→24** + assertion test.

## Test plan (TDD — soundness decoys mandatory)
- Python same-file `class Child(Base)`, `Base.m` defined, `Child` lacks `m`,
  `self.m()` in Child → Exact `SelfReceiver`.
- JS/TS `class Child extends Base { … this.m() }`, same-file `Base.m()` → Exact.
- **External base barrier:** `class Child(ExternalBase)` (no same-file `ExternalBase`),
  `self.m()` → `ExternalOrAmbiguous` → drop (NOT a guess).
- **Same-name unrelated class (1a FP class):** `a.py` has `class Widget(Base)` and the repo has
  an unrelated `Base` elsewhere — same-file resolution only, span-exact → no cross-file bind.
- **Ambiguous base** (two same-file `class Base`) → `ExternalOrAmbiguous` → drop.
- **Multiple inheritance**, two same-file bases each defining `m` → ambiguous provider → drop.
- **Barrier ordering:** `class C(External, InRepoBase)` where `InRepoBase.m` exists → External
  is first → barrier → drop (don't reach InRepoBase). (Conservative; documents the MRO rule.)
- **Wildcard-poison:** caller file has `from x import *` → inherited resolution falls open.
- **Cycle:** `class A(B)` / `class B(A)` (pathological) → terminates, drops.
- 1a regressions preserved (same-class Exact; absent-method drop).

CACHE asserts 24; `class_bases` serde round-trip + merge/remove_files.

## Acceptance
- `unresolved_unknown_name` **down** by the same-file inherited buy (≈ up to 12 fastapi / 4
  pydantic — whichever bases are same-file); `kind_exact.self_receiver` **up** by the same.
- **Canary `multi_target_exact_sites` byte-FLAT** (necessary, not sufficient — design + the
  diff-review are the soundness gate).
- **Rust/Go (ripgrep, caddy) call-stats byte-identical** (Python/JS/TS-gated); JS inert if
  excalidraw has 0 same-file inherited sites.
- Tier-A: replace the mislabeled `python/inherited_override` fixture (currently `c.go()` on an
  untyped param) with a real same-file inherited-self fixture; `--matrix-only` 0-regression;
  suite green; fmt clean.
- Build both binaries via git worktree.

## Risks / unknowns
- **Span-identity reuse** must exactly mirror 1a (`method_class_span`) or it reintroduces the
  cross-file same-name-class FP 1a fixed. Non-negotiable.
- **Base-owner key lookup** from a span: the build must record span→owner so the walk can form
  `methods[(base_owner, name)]`. If unavailable, fall open (drop), never guess.
- **Realized buy may be < 16** (only same-file bases; cross-file deferred). Honest: a smaller
  SOUND same-file buy now, the cross-file remainder waiting on slice 3's owner decision.
- `globals()`/`exec`/dynamic class rebinding out of scope = prism's standing bounded-static
  contract (same as every existing rung).

# Slice 1b — Inherited `self`/`this` Resolution, SAME-FILE bases (Python/JS/TS) — Design rev 2

> Spec-of-record, Python-maturity loop slice 1b (last mandate slice). **rev2 folds the codex
> spec-review of rev1** (3 BLOCKERs — all FIXABLE syntactic gaps, NOT the dynamic-binding
> boundary that parked slice 3): MRO C3 ordering, barrier non-propagation, and base-name
> rebinding. Recall companion to 1a (#131). Pairs with the loop handoff +
> `[[project_prism_measurement_maturity]]`.

## Goal

Resolve `self.method()` / `this.method()` where `method` is on a **same-file base class** —
currently dropped as `UnknownName` by 1a's same-class narrowing. Pure-recall, no precision risk
(drops today; Exact only on a single unambiguous same-file provider via a sound walk). Buy: the
same-file **single-inheritance** subset of the measured 16 in-repo sites (FastAPI's
`OAuth2PasswordBearer(OAuth2)`, `APIKeyBase` chain; Pydantic's `Secret(_SecretBase)` — all
same-file single-inheritance). Cross-file bases deferred to slice 3's parked import model.

## Framing: the inheritance analogue of R4 (no new soundness contract)

R4 same-file `LocalDef` (`resolution.rs:1248`) resolves a same-file free function under prism's
standing **bounded-static assumption** (no rung disproves `globals()`/`exec`). 1b is that move
for the class hierarchy, via class **identity** (file + byte-span, preserving 1a). It relies
ONLY on prism's existing contract. **rev1's BLOCKER-3 (base-name rebinding) is closed MORE
strictly than R4** — we syntactically poison a base name that has any other same-file top-level
binding — so 1b does not even bless R4's latent weakness; no new owner decision. `globals()`/
`exec` class rebinding is out of scope = prism's standing assumption.

**Canary caveat (slice-3 lesson):** `multi_target_exact_sites` (≥2-Exact only) CANNOT catch a
single wrong inherited Exact. Soundness is gated by **design + the adversarial diff-review**.
Every gate fails open to the existing `UnknownName` drop, never to a wrong Exact.

## Architecture (span-keyed, single-inheritance, tri-state walk)

### Data model

```rust
pub enum ClassBaseLink {
    SameFile { span: (usize, usize), owner: String }, // base = a clean sole same-file class
    Barrier,                                          // external / rebound / ambiguous — MRO BARRIER
}

// on CallGraph, serialized, keyed by class identity (file, class byte-span):
pub class_bases: BTreeMap<(String, (usize, usize)), Vec<ClassBaseLink>>,
```

`(file, span)` key (per review: a `FunctionId` key would lose methodless intermediate classes =
recall loss). `SameFile` carries the base's `owner` key so the walk forms `methods[(owner,name)]`
directly. **Never** a global span-only / bare-`owner_lookup` path (that is the cross-file
same-name FP 1a fixed).

### Base-name resolution (build time) — occurrence-clean or Barrier

For `class Child(Base): …`, resolve each base name. `Base` → `SameFile{span,owner}` **iff ALL**:
1. exactly **one** top-level `class Base` in the caller's file (→ its span+owner), AND
2. `Base` has **no other top-level binding occurrence** in the file — no `import Base` / `from x
   import Base` / `from x import y as Base`, no top-level `Base = …`, no top-level `def Base`, no
   second `class Base`, AND
3. the file has **no `from x import *`** (wildcard could supply `Base`).
Otherwise → `Barrier`. (This is the slice-3 occurrence-completeness rule applied to base names:
any rebinding of `Base`, in any syntactic form, adds a non-`class`-`Base` occurrence → Barrier.
Closes rev1 BLOCKER-3; `globals()`-style invisible rebind is out of scope = prism's contract.)
Cross-file/imported bases are thus **always** `Barrier` in this slice (no import resolution).

Subscripted bases (`class C(Base[int])`, `class C(Generic[T])`) → take the head identifier
(`Base`/`Generic`) and apply the same occurrence rule (`Generic` is external → Barrier).
Attribute/call bases (`class C(mod.Base)`, `class C(make_base())`) → not a simple name →
`Barrier`. Decorated classes (`@dataclass class C(Base)`) → unwrap to the `class_definition`.
`metaclass=`/keyword args skipped.

### Extraction helper
`Language::class_base_names(class_node) -> Vec<String>` beside `method_owner_class_node`
(`languages/mod.rs:1182`), Python/JS/TS only: Python `class C(A, B)` → base exprs from
`argument_list` (strict per above); JS/TS `class C extends A` → `["A"]` from `class_heritage`
(single parent). Build layer, not resolution.

### Resolution hook — single-inheritance, tri-state (closes BLOCKERs 1 & 2)

After `self_owner_lookup_same_class` returns `None` (`resolution.rs:733`/self-arm ~`:944`),
before the `UnknownName` drop, gated Python/JS/TS:

```
enum Walk { Hit(FunctionId), Absent, Blocked }

fn inherited(file, class_span, name, visited) -> Walk:
    let bases = class_bases[(file, class_span)]            // None => Absent
    if bases.len() > 1 { return Blocked }                 // BLOCKER-1: multiple inheritance — no C3, bail (sound)
    match bases.first():
        None              => Absent                        // no base
        Some(Barrier)     => Blocked                       // BLOCKER-2: external/rebound base may define `name` — STOP
        Some(SameFile{span, owner}):
            if visited.contains(span) { return Blocked }   // cycle
            visited.insert(span)
            let hits = methods[(owner, name)] filtered to (file, span)   // span-exact
            match hits.len():
                1 => Hit(hits[0])
                n if n > 1 => Blocked                       // ambiguous provider
                0 => inherited(file, span, name, visited)   // recurse the SINGLE chain; Blocked propagates up

// caller:
match inherited(caller.file, caller_span, name, {caller_span}):
    Hit(fid) => Exact SelfReceiver([fid])
    _        => None    // -> existing UnknownName drop
```

**Soundness invariants:**
- **Single-inheritance only:** any class in the chain with >1 base → `Blocked` (no C3 guessing).
  Linear chains are C3-correct. The measured buy is single-inheritance.
- **Tri-state propagation:** a `Barrier`/ambiguous/cycle/MI anywhere in the chain → `Blocked`,
  which propagates up (recurse result is returned directly — no "try a sibling" path, and with
  single-inheritance there are no siblings). Only `Absent` continues deeper.
- **Span-exact** `methods[(owner,name)]` filter by `(file, span)`; never bare `owner_lookup`.
- Single provider → cannot raise `multi_target_exact_sites`.
- Python/JS/TS-gated → Rust/Go byte-identical. Every non-`Hit` → existing `UnknownName` drop.

## Scope guards (first merge)
- Same-file bases only; single-inheritance only (MI → Blocked); cross-file deferred to slice 3.
- Occurrence-clean base resolution + wildcard-poison (above).
- Preserve 1a canaries (same-class Exact; absent-method drop; cross-file same-name no-bind).

## Plumbing + cache
`class_bases` threaded through `empty`/full/skeleton/subset builds + `remove_files` (drop
entries whose file is excluded) + `merge` (extend), mirroring 1a's `method_class_span`
template. **Bump `CACHE_VERSION` 23→24** + assertion test.

## Test plan (TDD — soundness decoys mandatory)
- **Single-inheritance Exact:** Python `class Child(Base)`, same-file `Base.m`, `self.m()` in
  Child → Exact `SelfReceiver`. JS/TS `extends` analogue.
- **Linear chain:** `C(B)`, `B(A)`, `A.m` same-file, `self.m()` in C → Exact (recurse).
- **MI bail (BLOCKER-1):** `class C(B, D)`, `B(A)`/`D` same-file, `D.m` + `A.m` → `Blocked` →
  drop (NOT a C3 guess).
- **Barrier-in-chain (BLOCKER-2):** `C(B)`, `B(External)` (External not same-file), `self.m()`
  → `Blocked` → drop (don't reach anything past External).
- **Named-import-shadow base (BLOCKER-3):** same-file `class Base` + `from ext import Base` →
  `Base` rebound → `Barrier` → drop.
- **Assignment-rebind base (BLOCKER-3):** `class Base` + `Base = Other` → `Barrier` → drop.
- **Wildcard-poison:** file has `from x import *` → all bases `Barrier` → drop.
- **Ambiguous same-name base:** two same-file `class Base` → `Barrier`.
- **>1 provider:** single base whose class defines `m` twice (overload-ish) → `Blocked`.
- **Cycle:** `A(B)`/`B(A)` → terminates, drops.
- **Subscript/attribute base:** `class C(Generic[T])` / `class C(mod.Base)` → `Barrier`.
- 1a regressions preserved (same-class Exact; absent-method drop; cross-file same-name no-bind).

CACHE asserts 24; `class_bases` serde round-trip + merge/remove_files.

## Acceptance
- `unresolved_unknown_name` **down** by the same-file single-inheritance inherited buy;
  `kind_exact.self_receiver` **up** by the same.
- **Canary `multi_target_exact_sites` byte-FLAT** (necessary, not sufficient — design + the 1b
  diff-review are the soundness gate; it must trace every non-`Hit` → drop).
- **Rust/Go (ripgrep, caddy) call-stats byte-identical** (Python/JS/TS-gated); JS inert if
  excalidraw has 0 same-file single-inheritance inherited sites.
- Tier-A: replace the mislabeled `python/inherited_override` fixture (currently `c.go()` on an
  untyped param) with a real same-file single-inheritance inherited-self fixture;
  `--matrix-only` 0-regression; suite green; fmt clean.
- Build both binaries via git worktree.

## Risks / unknowns
- **Span-identity reuse** must mirror 1a exactly (key `(file, span)`, never global) or it
  reintroduces the cross-file same-name FP. Non-negotiable.
- **Realized buy may be < 16** (same-file single-inheritance subset). Honest: a smaller SOUND
  buy now; cross-file + MI remainder deferred (slice 3 / C3 follow-on).
- **Base occurrence scan** must itself be correct (top-level only; every binding form of the
  name → Barrier). The one syntactic judgment; test it (named import, alias import, assignment,
  2nd class, def, wildcard).
- `globals()`/`exec`/dynamic class rebinding out of scope = prism's standing bounded-static
  contract (same as every existing rung).

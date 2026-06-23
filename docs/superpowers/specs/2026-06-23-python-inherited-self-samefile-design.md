# Slice 1b — Inherited `self`/`this` Resolution, SAME-FILE DIRECT base (Python/JS/TS) — Design rev 3

> Spec-of-record, Python-maturity loop slice 1b (last mandate slice). **rev3 narrows rev2 to
> DEPTH-1 (direct base only)** after the rev2 re-review (`/tmp/slice1b-rereview-out.md`) confirmed
> rev1's 3 BLOCKERs closed but found a recursion-only **member-shadow** BLOCKER + 2 MAJORs. The
> measured buy is entirely depth-1, so dropping recursion removes the trap class with ~0 buy
> loss. Recall companion to 1a (#131). Pairs with the loop handoff + `[[project_prism_measurement_maturity]]`.

## Goal

Resolve `self.method()` / `this.method()` to a method on the caller class's **direct same-file
base class** — currently dropped as `UnknownName` by 1a's same-class narrowing. Pure-recall, no
precision risk. Buy: the measured in-repo inherited sites, which are all **depth-1 single
direct-base** (FastAPI `OAuth2PasswordBearer(OAuth2)`→`OAuth2.make_not_authenticated_error`,
`APIKey*(APIKeyBase)`→`check_api_key`; Pydantic `Secret(_SecretBase)`→`get_secret_value`).
Deeper inheritance, multiple inheritance, and cross-file bases are out of scope (deferred).

## Framing: depth-1 = the inheritance analogue of R4 + 1a, sound by simplification

The rev2 re-review's member-shadow BLOCKER (`class B(A): m = 0; class C(B): self.m()` — recursing
past B's attribute to A's method) arises **only from recursion**. **rev3 does NOT recurse**: it
resolves only the **single direct base**. Consequences:
- No recursion → **no recurse-past-shadow** (the BLOCKER is structurally impossible).
- No multi-level walk → **no MRO/C3** concern (only a single direct base is ever consulted; >1
  base → drop).
- Consistent with **1a**: 1a assumes a class with `def m` provides method `m` (ignoring a rare
  same-name attribute shadow in that same class); rev3 makes the identical assumption for the
  direct base — no NEW assumption beyond 1a's existing one. `globals()`/`exec`/dynamic-object-
  model (`__mro_entries__`) out of scope = prism's standing bounded-static contract.

**Canary caveat (slice-3 lesson):** `multi_target_exact_sites` cannot catch a single wrong
inherited Exact. Soundness is gated by **design + the adversarial diff-review**. Every non-Hit
path → the existing `UnknownName` drop.

## Architecture (depth-1, span-keyed, per-base links)

### Data model

```rust
pub enum ClassBaseLink {
    SameFile { span: (usize, usize), owner: String }, // base = a clean sole same-file class
    Barrier,                                          // external / rebound / ambiguous / non-simple
}

// on CallGraph, serialized, keyed by class identity (file, class byte-span):
pub class_bases: BTreeMap<(String, (usize, usize)), Vec<ClassBaseLink>>,
```

`(file, span)` key (1a identity). **One `ClassBaseLink` per base slot, in order, count
preserved** — a non-simple base (call/attribute/subscript/string) emits `Barrier`, **never**
disappears (closes rev2 MAJOR: `class C(make_base(), A)` stays 2 bases → MI → drop, not
collapsed to single-base `A`). `SameFile` carries the base's `owner` so the walk forms
`methods[(owner,name)]` directly. **Never** bare `owner_lookup` / global span map.

### Base-name resolution (build time) — occurrence-clean or Barrier

For each base slot of `class Child(...)`:
- **Simple identifier base** `Base` → `SameFile{span,owner}` **iff ALL**: (1) exactly one
  top-level `class Base` in the caller's file; (2) `Base` has **no other top-level binding
  occurrence** in the file (no `import`/alias-import/`Base = …`/`def Base`/2nd `class Base`,
  including inside top-level `if`/`try`/`for`/`with` bodies — a module-scope identifier-
  occurrence scan, NOT enumerated binder forms); (3) no `from x import *` in the file. Else
  `Barrier`.
- **Non-simple base** — subscript (`Base[int]`, `Generic[T]`), attribute (`mod.Base`), call
  (`make_base()`), string forward-ref (`"Base"`), PEP-695 type params — → **`Barrier`** (closes
  rev2 MAJOR on `__class_getitem__`/`__mro_entries__`; we do not model the dynamic object model).
- Cross-file/imported bases → always `Barrier` (no import resolution in this slice).

### Extraction helper
`Language::class_base_names(class_node) -> Vec<ClassBaseLink>` (NOT `Vec<String>` — preserve
count) beside `method_owner_class_node` (`languages/mod.rs:1182`), Python/JS/TS: Python
`argument_list` base slots (skip `metaclass=`/keyword args), each → simple-name-or-`Barrier`;
JS/TS `class_heritage` `extends` (single parent). Decorated classes unwrap to `class_definition`.
Build layer, not resolution.

### Resolution hook — depth-1, no recursion (closes the member-shadow BLOCKER)

After `self_owner_lookup_same_class` returns `None` (`resolution.rs:733`/self-arm ~`:944`),
before the `UnknownName` drop, gated Python/JS/TS:

```
fn inherited_direct(caller, name) -> Option<FunctionId>:
    caller_span = method_class_span[caller]            // None -> None (drop)
    let bases = class_bases[(caller.file, caller_span)]
    if bases.len() != 1 { return None }                // 0 bases, or >1 (MI) -> drop (no C3)
    match bases[0]:
        Barrier            => None                      // external/rebound/non-simple base -> drop
        SameFile{span,owner} =>
            let hits = methods[(owner, name)] filtered to (caller.file, span)   // span-exact
            if hits.len() == 1 { Some(hits[0]) } else { None }   // single method provider only

// caller: Some(fid) => Exact SelfReceiver([fid]); None => existing UnknownName drop.
```

**Soundness invariants:**
- **Depth-1 only:** exactly one direct base, resolved `SameFile`, providing exactly one method
  `m` (span-exact). No recursion → no member-shadow, no MRO, no cycle.
- `>1` base (MI) → drop; `Barrier` base → drop; `0` or `>1` method hits → drop.
- Span-exact `methods[(owner,name)]` filter by `(caller.file, base_span)` → cannot bind a
  different-file class.
- Single provider → cannot raise `multi_target_exact_sites`.
- Python/JS/TS-gated → Rust/Go byte-identical. Every non-`Some` → existing `UnknownName` drop.

## Scope guards (first merge)
Same-file + **direct (depth-1)** + **single** base only. MI, deeper inheritance, cross-file
bases, non-simple bases → drop (deferred). Occurrence-clean base resolution + wildcard-poison.
Preserve 1a canaries.

## Plumbing + cache
`class_bases` threaded through `empty`/full/skeleton/subset builds + `remove_files` (drop
file-excluded entries) + `merge` (extend), mirroring 1a's `method_class_span` template.
**Bump `CACHE_VERSION` 23→24** + assertion test.

## Test plan (TDD — soundness decoys mandatory)
- **Depth-1 Exact:** Python `class Child(Base)`, same-file `Base.m` (method), `self.m()` in
  Child (no `m` on Child) → Exact `SelfReceiver`. JS/TS `extends` analogue.
- **Grandparent NOT resolved:** `class A: def m`, `class B(A): pass`, `class C(B): self.m()` →
  drop (depth-1 only; B has no `m`, no recursion). (Documents the depth-1 scope.)
- **Member-shadow safe (the rev2 BLOCKER):** `class A: def m`, `class B(A): m = 0`,
  `class C(B): self.m()` → drop (B is direct base, `methods[(B,m)]`=0 → drop; never reaches A).
- **MI drop:** `class C(B, D)` → 2 bases → drop.
- **Non-simple base preserved → MI/Barrier:** `class C(make_base(), A)` → 2 base slots
  (Barrier + A) → `len != 1` → drop (not collapsed to single-base A).
- **Subscript base:** `class C(Base[int])` / `class C(Generic[T])` → `Barrier` → drop.
- **Named-import / assignment rebind base:** same-file `class Base` + `from ext import Base`
  (or `Base = Other`) → `Barrier` → drop.
- **Wildcard-poison:** file has `from x import *` → `Barrier` → drop.
- **Ambiguous same-name base:** two same-file `class Base` → `Barrier` → drop.
- **>1 method provider:** direct base defines `m` twice → drop.
- 1a regressions preserved (same-class Exact; absent-method drop; cross-file same-name no-bind).

CACHE asserts 24; `class_bases` serde round-trip + merge/remove_files.

## Acceptance
- `unresolved_unknown_name` **down** by the depth-1 direct-base inherited buy;
  `kind_exact.self_receiver` **up** by the same.
- **Canary `multi_target_exact_sites` byte-FLAT** (necessary, not sufficient — design + diff-
  review are the gate; the 1b diff-review must trace every non-`Some` → drop).
- **Rust/Go (ripgrep, caddy) call-stats byte-identical**; JS inert if excalidraw has 0 depth-1
  inherited sites.
- Tier-A: replace the mislabeled `python/inherited_override` fixture with a real depth-1
  inherited-self fixture; `--matrix-only` 0-regression; suite green; fmt clean.
- Build both binaries via git worktree.

## Risks / unknowns
- **Span-identity reuse** must mirror 1a exactly (key `(file, span)`, never global). Non-negotiable.
- **Realized buy** is the depth-1 single-direct-base subset — but that IS the measured buy
  (all cited sites are depth-1). Deeper/MI/cross-file deferred.
- **Base occurrence scan** correctness (top-level only; any binding form of the name → Barrier;
  module-scope identifier-occurrence, not enumerated forms). The one syntactic judgment — test it.
- Same-name attribute shadowing the providing base's own method (`def m` + `m = 0` in the SAME
  class) is the same rare assumption 1a already makes (a class with `def m` provides `m`); no new
  unsoundness. `globals()`/`exec`/`__mro_entries__` out of scope = prism's standing contract.

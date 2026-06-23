# Slice 3 — Sound Imported-Member Free-Call Resolution (Python) — Design rev 3

> Spec-of-record, Python-maturity loop slice 3. **rev3 folds two codex design reviews**
> (`/tmp/slice3-specreview-out.md` rev1: 5 BLOCKER; `/tmp/slice3-rereview-out.md` rev2: B3/B5/
> MAJOR1/MAJOR2 CLOSED, B1/B2/B4 open). Both reviews converged on: **syntactic soundness over
> Python's dynamic binding is a fragile form-enumeration**, and the `multi_target_exact_sites`
> canary CANNOT catch a single wrong `import_member` Exact — so a missed binder form = a SILENT
> wrong edge in production. rev3 therefore replaces enumeration with **complete-by-construction
> occurrence rules**. Pairs with the loop handoff + `[[project_prism_measurement_maturity]]`.

## Goal

Resolve **bare imported free-function calls** in Python — `from mod import foo; foo()` and
`from mod import foo as f; f()` — to the imported definition, minting **Exact**
(`ImportMember`) **only when provably unambiguous**. Drains part of the `free_multi` NameOnly
bucket (multiple repo-wide same-name defs pinned to one file) and recovers aliased calls that
currently **drop** as `UnknownName`. The realized buy is the subset that passes every gate
(est. ~40-70% of the ~305 pydantic+fastapi import-singletons = low hundreds; that the gates
shrink it is correct, not a shortfall). JS deferred (named-import soundness needs an export
table for re-exports — its own slice).

## Design principle: complete-by-construction, not enumerated

A wrong Exact here is invisible to the canary, so every gate must be **sound by construction
and fail open on ANY uncertainty**. We never enumerate "which Python statements bind a name"
(params, `for`, `with as`, `except as`, `match`, comprehension, walrus, `del`, `global`,
augmented/tuple assign, conditional def, local/conditional import — a long, version-drifting
tail). Instead we use **occurrence rules**: a name is safe only when its textual occurrences
are exhaustively accounted for as import-or-call (caller side) / a sole top-level def (module
side). Any binder, known or not, introduces an extra occurrence → fall open. This closes
rev2's B1/B2 without form-enumeration.

## Decomposition

- **3a — inert foundation (resolution/call-stats byte-identical):** three new serialized
  structures + extraction + build/merge/cache plumbing. NO resolution change. (NB "byte-
  identical" = resolution/call-stats output; the serialized cache blob necessarily changes —
  hence the CACHE bump.)
- **3b — the rung (behavior):** the Python-only R4c rung consuming 3a.
3a ships first; 3b off merged 3a. Each is its own plan → review → PR → merge.

---

## Sub-slice 3a — foundation (3 structures)

### 1. `ImportBinding` with kind, member, and eligibility

```rust
pub enum ImportKind { Named, Default, Namespace, CommonJs, Wildcard, Module }

pub struct ImportBinding {
    pub local: String,            // name as used at call sites
    pub module_path: String,      // raw import source (dotted Python module / JS specifier)
    pub member: Option<String>,   // original imported symbol; Some for Named, None otherwise
    pub kind: ImportKind,
    pub eligible: bool,           // import-pure (see below); R4c fires only when true
}

// on CallGraph, serialized, parallel to the unchanged `imports` (alias->module, drives R3):
pub import_bindings: BTreeMap<String /*file*/, BTreeMap<String /*local*/, ImportBinding>>,
```

New extraction in `ParsedFile` (member from `aliased_import.name` Py `ast.rs:625` / JS
`import_specifier.name` `ast.rs:747`; do NOT reuse the lossy `imports` map — MAJOR-1).
- Python `from x import f` → `Named{member:Some("f")}`; `from x import f as g` →
  `Named{local:"g", member:Some("f")}`; `import x[.y][ as z]` → `Module{member:None}`;
  `from x import *` → `Wildcard`.
- JS kinds captured for completeness; 3b reads Python only.

**`eligible` (the import-pure rule — closes B1):** for a Python `Named` binding of `local`,
`eligible = true` iff, across the WHOLE caller file, **every** identifier occurrence textually
equal to `local` is either (i) the binding occurrence of a **single module-scope** `from …
import local` (exactly one such import; not function-local, not inside a compound statement),
or (ii) a **call-function position** (`local(...)`). Any other occurrence — assignment LHS,
parameter, `for`/`with`/`except`/`match`/comprehension target, nested `def`/`class` name,
`del`/`global`/`nonlocal`, a second import of `local` (function-local or conditional) — makes
`eligible = false`. Computed at extraction (full tree available) by collecting all occurrences
of each imported `local` and classifying them. This is **complete**: any binding of `local`,
in any syntactic form, adds a non-import-non-call occurrence → not eligible → R4c falls open.
Conservative (a file that imports `f` and also uses `f` as anything but a call loses the buy),
but sound regardless of which binder forms Python adds.

### 2. `module_bindings` — top-level binding kind per file

```rust
pub enum BindingKind { FuncDef(FunctionId), ClassDef, Ambiguous, Other }

pub module_bindings: BTreeMap<String /*file*/, BTreeMap<String /*name*/, BindingKind>>,
```

`module_bindings[file][name] = FuncDef(fid)` iff the name's **sole** top-level binding is a
single `function_definition`/`decorated_definition` AND the name has **no other top-level
occurrence in a binding position** anywhere at module scope — including inside top-level
`if`/`try`/`for`/`while`/`with` compound statements (closes B2: a conditional rebind, a
top-level `for f in …`, `f = …`, `del f`, or `from y import f` re-export all add a top-level
occurrence → `Ambiguous`). `class name` (sole) → `ClassDef`. Anything ambiguous/mixed →
`Ambiguous`. Detection over-approximates toward `Ambiguous` (occurrence-based, same principle
as eligibility): if the name appears at top level anywhere other than as the single def's
name, it is not a clean `FuncDef`. Nested defs/methods are excluded (not module-scope).
`FuncDef` carries the `FunctionId` so the rung returns the target with no second lookup.

### 3. `indexed_files` — authoritative file set

```rust
pub indexed_files: BTreeSet<String>,   // every repo file prism parsed (serialized)
```

Needed for source-root-aware singleton module resolution (MAJOR-2 + B4). Populate from build
inputs in `empty`/full/skeleton/subset builds; maintain in `remove_files`/`merge`. **Must be
complete even in subset/scoped builds** (rev2 Q3): if a subset build stored only subset files,
3b could miss a duplicate module candidate and mint a wrong Exact — so `indexed_files` records
the full indexed set the graph was built against, and 3b treats an incomplete set
conservatively (see 3b: a non-singleton OR an out-of-scope module → fall open).

### 3a plumbing + cache

Three structures threaded through `empty` (`call_graph.rs:234`), full build (`:277`/`:514`),
skeleton/subset builds (`:466`/`:1494`), `remove_files` (`:1048`, filter file-keyed maps by
excluded set; `indexed_files` set-difference), `merge` (`:1107`, extend; `indexed_files`
union) — mirroring the `method_class_span` field pattern. **No `CallSite` change** (eligibility
lives on `ImportBinding`, not per-call) → call-site identity untouched. **Bump `CACHE_VERSION`
23→24** + assertion test. 3a changes NO resolution code → `resolve_call_site_full`
byte-identical → **all corpora call-stats byte-identical** (3a acceptance gate).

---

## Sub-slice 3b — the R4c rung (Python-only)

After R4b implicit-this (`resolution.rs:1311`), before R5 free-fn pool (`:1313`). **R4c**
(R4.5 is Go SamePackage). Python callers only (Rust/Go/JS byte-identical).

```
// R4c: sound imported-member free-call resolution (Python).
if caller.file is .py:
    let Some(b) = import_bindings[caller.file].get(name) else fall through to R5
    if !(b.kind == Named && b.member.is_some() && b.eligible) { fall through }  // B5 + B1
    let member = b.member.unwrap()
    let Some(modfile) = resolve_module_to_file(caller.file, b.module_path) else fall through  // B4
    match module_bindings[modfile].get(member) {
        Some(FuncDef(fid)) => return Exact ImportMember([fid])   // the ONLY Exact path
        _ => fall through to R5                                  // class/ambiguous/absent/nested
    }
```

`resolve_module_to_file(caller_file, module_path) -> Option<String>` (B4 + MAJOR-2,
source-root robust):
- Map the dotted/relative `module_path` to a path **suffix** (`a.b.c` → `a/b/c`; relative
  `.mod`/`..pkg.mod` resolved against the caller's directory to a dotted suffix).
- Collect every `f ∈ indexed_files` whose path **ends with** `<suffix>.py` or
  `<suffix>/__init__.py`.
- Return `Some(f)` **iff exactly one** such file exists; `None` otherwise (0, or ≥2 across
  source roots like `pkg/util.py` vs `src/pkg/util.py`, or `.py`-and-`__init__.py` both
  present). No filesystem I/O — purely `indexed_files` suffix intersection.

New `ResolutionKind::ImportMember` → `"import_member"`.

**Soundness invariants (the design IS the guarantee — the canary is blind):**
- Caller side: `eligible` proves the name is import-pure in the file → no local/param/loop/
  with/except/comprehension/del/global/2nd-import shadow, by occurrence-completeness.
- Module side: `FuncDef(fid)` proves a sole top-level def → no nested/class/reassign/re-export
  shadow.
- Module path: unique suffix match → no source-root ambiguity.
- Named-with-member only → no default/namespace/CommonJS/wildcard.
- One candidate → cannot raise `multi_target_exact_sites`.
- Python-gated → Rust/Go/JS byte-identical.
- Every uncertainty → fall through to R5 (current behavior): never a new drop, never a wrong
  Exact.

---

## Test plan (TDD — soundness decoys mandatory, per rev2 MAJOR-3)

3a (extraction/plumbing units):
- `ImportBinding`: member recovery (`from x import f as g` → member `f`); kinds
  (named/default/namespace/CommonJS/wildcard/module).
- **eligibility:** `from .m import f; f()` only → eligible; `+ f = g()` anywhere → ineligible;
  `def run(f): f()` → ineligible (param occurrence); `for f in xs: f()` → ineligible;
  `from .m import f` + a 2nd `from .n import f` (any scope) → ineligible; `from .m import f`
  used only as `f()` ×N → eligible.
- `module_bindings`: sole `def f` → `FuncDef`; `def f` + top-level `if c: f = x` →
  `Ambiguous`; `def f` + `del f` → `Ambiguous`; `class F` → `ClassDef`; nested `def wrap():
  def f()` → `f` absent from module table; `from y import f` (re-export) → not `FuncDef`.
- `indexed_files`: complete after full build; survives merge (union) / remove_files (diff);
  cache asserts 24; `ImportBinding`/`module_bindings` serde round-trip.

3b (resolution, Python):
- `from .mod import f; f()`, one top-level `def f` in `mod.py`, ≥1 other repo `f` → Exact.
- `from .mod import f as g; g()` → Exact (was a drop).
- **param/local/loop/with/except shadow** (each) → fall-through, NOT Exact.
- **module reassign / conditional rebind / del in target** → `Ambiguous` → fall-through.
- **nested-fn decoy** in target module → fall-through.
- **class shadow** (`def Foo`+`class Foo`) → `Ambiguous` → fall-through.
- **later/function-local re-import** of the name → ineligible → fall-through.
- **source-root duplicate** (`pkg/util.py` + `src/pkg/util.py`) → `None` → fall-through.
- **`.py` + `__init__.py` both present** → `None` → fall-through.
- external package → not indexed → fall-through.
- non-Python caller → byte-identical.

## Acceptance

- **3a:** ALL corpora (ripgrep/caddy/express/excalidraw/fastapi/pydantic) call-stats
  **byte-identical** to base. Suite green; fmt clean; cache 24 asserted; serde/cmp-key test.
- **3b:** pydantic/fastapi `import_member` Exact up, `free_multi`+`unresolved` down by the
  same; **canary `multi_target_exact_sites` byte-FLAT** (necessary, NOT sufficient); Rust/Go/JS
  **byte-identical**; Tier-A `--matrix-only` 0-regression; suite green.
- **Soundness gated by DESIGN + adversarial 3b diff-review** (canary is blind) — the diff-review
  must trace every fall-open path and confirm no provenance gap mints a wrong Exact.
- Build both binaries via git worktree; never swap the binary mid-measurement.

## Risks / unknowns

- **Conservatism may shrink the buy** (import-pure is strict). Acceptable: a smaller SOUND buy
  beats an unsound one (canary can't protect us). Measure post-3b; if near-zero, reassess
  whether 3b is worth shipping (3a foundation still useful for 1b regardless).
- **Occurrence classification** ("call-function position" vs other) must itself be correct —
  the one piece of syntactic judgment left; test it directly (attribute calls `x.f()`,
  `f` as an argument `g(f)`, `f` in a decorator `@f`).
- **`indexed_files` completeness in subset/scoped builds** — 3b must treat an out-of-scope or
  non-singleton module as fall-open (never assume absence = unique).
- JS deferred. pydantic mixed-language: call-stats deltas are the source of truth.

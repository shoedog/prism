# Slice 3 — Sound Imported-Member Free-Call Resolution (Python) — Design rev 2

> Spec-of-record, Python-maturity loop slice 3. **rev2 folds the codex spec-review of rev1
> (5 BLOCKER + 3 MAJOR, `/tmp/slice3-specreview-out.md`).** rev1 ("single free fn named
> `member` in the resolved file → Exact") was UNSOUND: prism's function inventory has no
> provenance (export/kind/top-level/scope), and the `multi_target_exact_sites` canary CANNOT
> catch a single wrong `import_member` Exact — so **the design must be sound by construction**,
> acceptance cannot backstop it. Pairs with the loop handoff + `[[project_prism_measurement_maturity]]`.

## Goal

Resolve **bare imported free-function calls** in Python — `from mod import foo; foo()` and
`from mod import foo as f; f()` — to the imported definition, minting **Exact**
(`ImportMember`) **only when provably unambiguous**. Drains part of the `free_multi` NameOnly
bucket (non-aliased, multiple repo-wide same-name defs pinned to one file) and recovers
aliased calls that currently **drop** as `UnknownName`. Buy ≈ 241 pydantic + 64 fastapi
import-singletons — but **only the subset that passes every soundness gate** (realized buy
will be lower; that is correct, not a shortfall). JS deferred (named-import soundness needs an
export table for re-exports — its own slice).

## Why this is bigger than "a rung" (the rev1 → rev2 lesson)

A sound rung must PROVE, before Exact: (1) the call name is bound by a **named import** with a
concrete member; (2) the module resolves to **exactly one** indexed file; (3) the member is a
**top-level function definition** in that file (not nested, not a class, not an assignment,
not a re-export); (4) the call name is **not shadowed** by a param/local/nested-def in the
caller's function, nor reassigned at the caller's module scope. prism stores **none** of this
today. So slice 3 = build that provenance (sub-slice **3a**, inert) + consume it (sub-slice
**3b**, the rung). The provenance is **reusable**: slice 1b's cross-file base-class linking
and a future imported-receiver-type slice need the same module/import model.

## Decomposition

- **3a — inert foundation (behavior-identical, all corpora byte-identical):** four new
  serialized structures + their extraction + build/merge/cache plumbing. NO resolution change.
- **3b — the rung (behavior):** the Python-only R4c rung consuming 3a. Each sub-slice is its
  own spec-derived plan → review → PR → merge (3a first; 3b off merged 3a).

---

## Sub-slice 3a — foundation

### 1. `ImportBinding` with kind + member (replaces rev1's lossy reuse of `imports`)

```rust
pub enum ImportKind { Named, Default, Namespace, CommonJs, Wildcard, Module }

pub struct ImportBinding {
    pub local: String,            // name as used at call sites
    pub module_path: String,      // raw import source (dotted Python module / JS specifier)
    pub member: Option<String>,   // original imported symbol; Some for Named, None otherwise
    pub kind: ImportKind,
}

// on CallGraph, serialized, parallel to the unchanged `imports` (alias->module, drives R3):
pub import_bindings: BTreeMap<String /*file*/, BTreeMap<String /*local*/, ImportBinding>>,
```

New extraction in `ParsedFile` (do NOT derive from `imports` — MAJOR-1; the member is lost
there). Recover the member from the AST fields already present:
- Python `from x import f` → `Named{local:"f", member:Some("f"), module_path:"x"}`.
- Python `from x import f as g` → `Named{local:"g", member:Some("f"), ...}` (member =
  `aliased_import` **`name`** field, `ast.rs:625` shows it's available; local = `alias`).
- Python `import x` / `import x.y` / `import x as y` → `Module{member:None}`.
- Python `from x import *` → `Wildcard{member:None}`.
- JS `import {f} from "./x"` / `{f as g}` → `Named` (member from `import_specifier.name`,
  `ast.rs:747`); `import f from "./x"` → `Default`; `import * as ns` → `Namespace`;
  `const {f}=require()` → `CommonJs`. (Extraction captures all kinds in 3a; 3b's rung is
  Python-only and fires only on `Named`.)
- **Last-binding-wins** within a file (BTreeMap insert order = source order via the existing
  recursive walk): matches Python/JS "last import of a name wins" — so `from .a import f;
  from external import f` leaves the binding = the external one (→ 3b fails open). Sound.

Two `local`s can never collide except by re-import (handled by last-wins). The existing
`imports` map is **unchanged** (byte-compat for R3).

### 2. `module_bindings` — top-level binding kind per file

```rust
pub enum BindingKind {
    FuncDef(FunctionId),  // exactly one top-level `def name`/decorated def, no other top-level binding of name
    ClassDef,
    Assignment,           // top-level `name = ...`
    ImportReexport,       // top-level `import`/`from import` binding name (a re-export)
    Ambiguous,            // >1 top-level binding of name, or conflicting kinds
    Other,
}

pub module_bindings: BTreeMap<String /*file*/, BTreeMap<String /*name*/, BindingKind>>,
```

Extraction walks **module-scope direct children only** (NOT inside functions/classes — that
excludes nested fns and methods, closing BLOCKER-2). Per top-level name: a lone
`function_definition`/`decorated_definition` → `FuncDef(fid)`; a lone `class_definition` →
`ClassDef`; a top-level assignment → `Assignment`; an import binding the name →
`ImportReexport`; **if a name has more than one top-level binding (e.g. `def f` then `f=…`,
or conditional defs), or mixed kinds → `Ambiguous`** (the rung Exacts only on `FuncDef`, so
ambiguity fails open). This is the provenance rev1 lacked (closes BLOCKERs 2 [nested], 3
[class vs fn], and re-export shadowing).

`FuncDef` carries the `FunctionId` so the rung returns the exact target without a second
lookup. Python-first; JS/TS module_bindings can be populated in 3a (inert) but 3b only reads
Python files' tables.

### 3. `indexed_files` — authoritative file set for singleton module resolution

```rust
pub indexed_files: BTreeSet<String>,   // every repo file prism parsed (serialized)
```

`CallGraph` today stores only files that have functions/calls/imports (MAJOR-2). To detect
module-path ambiguity (`mod.py` vs `mod/__init__.py`; a module with no functions), 3b needs
the full parsed-file set. Populate from build inputs in `empty`/full/skeleton/subset builds;
maintain in `remove_files`/`merge`.

### 4. `CallSite.name_shadowed: bool` — build-time caller-scope shadow bit

```rust
// CallSite, serde(default), excluded from cmp_key (like receiver_materialized):
pub name_shadowed: bool,
```

Set at extraction (tree available) when the call's own function name is bound in the caller's
enclosing-function scope — a **parameter, a local assignment target, or a nested `def`/`function`
of the same name** before/anywhere in that function. This is presence-detection only (no type
recovery), reusing the scope-walk shape from prior slices. 3b fails open when `name_shadowed`
(closes BLOCKER-1's param/local case). Module-level reassignment of an imported name is caught
separately by `module_bindings[caller][name] == Assignment/Ambiguous` (see 3b).

### 3a plumbing + cache

All four structures threaded through `empty` (`call_graph.rs:234`), full build (`:277`/`:514`),
skeleton/subset builds (`:466`/`:1494`), `remove_files` (`:1048`), `merge` (`:1107`); **bump
`CACHE_VERSION` 23→24** (`cpg_cache.rs:68`) + the assertion test. 3a changes NO resolution
code → `resolve_call_site_full` byte-identical → **all corpora call-stats byte-identical**
(the 3a acceptance gate).

---

## Sub-slice 3b — the R4c rung (Python-only)

Inserted **after R4b implicit-this (`resolution.rs:1311`), before R5 free-fn pool (`:1313`)**.
("R4.5" is taken by Go `SamePackage` — this is **R4c**.) Gated to Python callers (Rust/Go/JS
byte-identical).

```
// R4c: sound imported-member free-call resolution (Python).
if caller.file is .py:
    let Some(b) = import_bindings[caller.file].get(name) else fall through to R5
    if b.kind != Named || b.member.is_none() { fall through }      // BLOCKER-5: named only
    let member = b.member.unwrap()
    if site.name_shadowed { fall through }                          // BLOCKER-1: param/local/nested shadow
    match module_bindings[caller.file].get(name) {                  // BLOCKER-1: module-level reassign
        Some(Assignment | Ambiguous) => fall through, _ => {}
    }
    let Some(modfile) = resolve_module_to_file(caller.file, b.module_path) else fall through  // BLOCKER-4: singleton
    match module_bindings[modfile].get(member) {
        Some(FuncDef(fid)) => return Exact ImportMember([fid])      // the ONLY Exact path
        _ => fall through to R5                                     // class/assign/reexport/nested/ambiguous/absent
    }
```

`resolve_module_to_file(caller_file, module_path) -> Option<String>` (BLOCKER-4 + MAJOR-2):
- Python dotted-absolute `a.b.c` → candidate set {`a/b/c.py`, `a/b/c/__init__.py`} ∩
  `indexed_files`; relative `.mod`/`..pkg.mod` → anchor at caller's package dir then same.
- Return `Some(f)` **iff exactly one** candidate is in `indexed_files`; `None` (fail open) on
  0 or >1 (ambiguous source roots / `.py`-vs-`__init__.py` both present).
- No filesystem I/O — purely `indexed_files` intersection.

New `ResolutionKind::ImportMember` → `"import_member"` (isolated bucket; visible buy).

**Soundness invariants (the design IS the guarantee — the canary is blind here):**
- Exact only on `FuncDef(fid)` from the resolved module: excludes nested fns (not top-level),
  classes, assignments, re-exports, and ambiguous top-level names.
- Named imports with a concrete member only: default/namespace/CommonJS/wildcard fall open.
- Singleton authoritative module resolution only: multi-file/ambiguous → fall open.
- Caller-scope shadow (`name_shadowed`) or module-level reassign → fall open.
- Single candidate (one `FunctionId`) → cannot raise `multi_target_exact_sites`; the rung
  never emits >1 Exact target.
- Python-gated → Rust/Go/JS byte-identical.
- Every uncertainty **falls through to R5** (current behavior) — never a new drop, never a
  wrong Exact.

---

## Test plan (TDD — soundness decoys are mandatory, per MAJOR-3)

3a (extraction/plumbing unit tests):
- `ImportBinding` member recovery: `from x import f as g` → `Named{local:g, member:f}`;
  `import x` → `Module`; `import {f as g} from "./x"` → `Named{local:g, member:f}`;
  default/namespace/CommonJS → correct kinds.
- `module_bindings`: top-level `def f` → `FuncDef`; `class F` → `ClassDef`; `f = …` →
  `Assignment`; `from y import f` → `ImportReexport`; `def f` + `f = …` → `Ambiguous`; a
  **nested** `def wrap(): def f()` → `f` NOT in the module table.
- `indexed_files` survives merge/remove_files; cache version asserts 24.
- `name_shadowed`: `def run(f): f()` → true; `def run(): f = x(); f()` → true; `def run():
  def f(): ...; f()` → true; plain `f()` with no local `f` → false.

3b (resolution, Python):
- `from .mod import f; f()` with one top-level `def f` in `mod.py` (and ≥1 other repo `f`) →
  Exact `ImportMember` (the free_multi→Exact buy).
- `from .mod import f as g; g()` → Exact (member `f`); was a drop before (recall recovery).
- **Param shadow:** `from .mod import f; def run(f): f()` → R5/fall-through, NOT Exact.
- **Local shadow:** `from .mod import f; def run(): f = h(); f()` → fall-through.
- **Module reassign:** `from .mod import f; f = x; f()` → fall-through (`Assignment`).
- **Nested-fn decoy:** member resolves to a `def f` nested inside another fn in mod → NOT in
  module_bindings → fall-through.
- **Class shadow:** `mod.py: def Foo; class Foo`; `from .mod import Foo; Foo()` → `Ambiguous`
  → fall-through (no Exact-to-fn).
- **Later external re-import:** `from .a import f; from external import f; f()` → binding =
  external → unresolved module → fall-through (NOT Exact to `.a`).
- **Module ambiguity:** `mod.py` AND `mod/__init__.py` both indexed → `resolve_module_to_file`
  None → fall-through.
- **External package:** `from requests import get; get()` → not indexed → fall-through.
- Non-Python caller unaffected (Rust/Go fixture → byte-identical).

## Acceptance

- **3a:** ALL corpora (ripgrep/caddy/express/excalidraw/fastapi/pydantic) call-stats
  **byte-identical** to base (inert). Suite green; fmt clean; cache 24 asserted.
- **3b:** pydantic/fastapi `import_member` Exact **up**, `free_multi` NameOnly + `unresolved`
  **down** by the same; **canary `multi_target_exact_sites` byte-FLAT** (necessary but NOT
  sufficient — see below); Rust/Go/JS **byte-identical** (Python-gated). Tier-A `--matrix-only`
  0-regression (touches resolution); suite green.
- **Soundness is gated by DESIGN + adversarial diff-review, NOT the canary** (the review proved
  a single wrong `import_member` leaves `multi_target_exact_sites` flat). The 3b diff-review
  must explicitly trace each fall-open path and confirm no provenance gap mints a wrong Exact.
- Build both binaries via git worktree; never swap the binary mid-measurement.

## Risks / unknowns

- **Scope:** 4 new serialized structures + 2 extraction passes + shadow detection. Largest
  slice of the loop. Mitigated by 3a-inert decomposition + design re-review before build.
- **`module_bindings` top-level detection** must exclude nested/conditional/`__all__`-rebind
  cases → `Ambiguous` (fail open) on any doubt.
- **Python relative-import anchoring** (package dir, namespace packages) — fail open (None)
  on any non-trivial layout; recall-only risk, never a wrong Exact.
- JS deferred (re-export export-table). pydantic mixed-language: the `free_multi`/`unresolved`
  call-stats deltas are the source of truth, not the rev1 site-estimate.

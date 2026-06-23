# Slice 3 — Bare-Import-Qualified Free-Call Narrowing (Python/JS) — Design

> Spec-of-record draft for the Python-maturity loop, slice 3. Formalizes architect
> memo `/tmp/slice3-architect-out.md` (Option B). Pairs with handoff
> `docs/superpowers/handoffs/2026-06-23-python-maturity-autonomous-loop.md` and memory
> `[[project_prism_measurement_maturity]]`.

## Goal

Resolve **bare imported free-function calls** — `from mod import foo; foo()` (Python),
`import { foo } from "./mod"; foo()` (JS/TS) — by narrowing the repo-wide free-function
candidate set to the **imported module's resolved file** and minting an **Exact** edge
on a single surviving candidate. This soundly drains part of the `free_multi` NameOnly
bucket (the R5 repo-wide bare-call fallback) where an import binding pins which file's
`foo` is meant.

## Background — the current ladder (verified)

The bare-call (unqualified `foo()`) branch in `CallGraph::resolve_call_site_full`
(`src/resolution.rs`) filters out methods, then:
1. **R4 same-file `LocalDef`** — a free `foo` defined in the caller's own file → Exact.
   (Python/JS same-file calls already land here, so a current `free_multi` site has **no**
   caller-file candidate — the gap is *not* "same file".)
2. **Go-only same-directory `SamePackage`** — Go siblings share a package namespace.
3. **Repo-wide non-static free functions**: `len == 1` → `free_single` (Exact);
   `len > 1` → **`FreeMulti`** demoted to `NameOnly` (`src/resolution.rs:1238`).

Python/JS have **no** Go-style implicit same-directory namespace: a sibling file's `foo`
is **not** in scope unless imported. So the sound lever is **import binding**, not same-dir.

Imports are already collected as `alias -> module_path` in `CallGraph.imports`
(`src/call_graph.rs:155`, populated from `ParsedFile::extract_imports` at
`src/ast.rs:566`), raw-string (not filesystem-resolved). Today that map feeds only the
**qualified** rung `pkg.f()` (R3, `src/resolution.rs:993`), **not** bare `f()`.

### Measured shape (`nav --no-cache call-stats`, exact edge telemetry)

| corpus | total call sites | `free_multi` NameOnly edges | `multi_target_exact_sites` (canary) |
|---|---:|---:|---:|
| pydantic | 65,645 | 25,293 | 439 |
| fastapi | 19,919 | 488 | 70 |
| express | 949 | 72 | 0 |

Read-only site-split estimate (classifier over prism's function inventory + ASTs):

| corpus | **import singleton** (the buy) | same-dir-singleton-no-import | same-dir multi | external import shadow | residual genuinely-ambiguous |
|---|---:|---:|---:|---:|---:|
| pydantic | **241** | 11 | 159 | 76 | 2,647 |
| fastapi | **64** | 0 | 2 | 0 | 13 |
| express | **1** | 0 | 6 | 1 | 14 |

Of same-dir singletons, almost all overlap an import binding (pydantic 172/183, fastapi
49/49) — confirming **import resolution must run before any same-dir heuristic**, and
that blind same-dir Exact would mint wrong edges on import-shadow decoys.

**Buy:** ~241 pydantic + 64 fastapi + 1 express import-singletons flip `free_multi`
(NameOnly) → Exact. The residual ~2,647 pydantic stays NameOnly (genuinely ambiguous —
correct). This is the cross-module lever the slice-2 strategic finding identified.

## Architecture (Option B)

A new resolution rung, **R4.5**, placed **after R4 same-file `LocalDef`** and **before
R5 repo-wide free-multi**, gated to Python/JS/TS callers. Plus a richer import-binding
data model and module-path→repo-file resolution.

### Data model — `ImportBinding`

Today `extract_imports` returns `BTreeMap<String, String>` = `local_name -> module_path`.
For `from x import foo as bar` it stores `bar -> x` and **loses `foo`** (the imported
member). The new rung needs the member name to match the right free function in the
target file.

Introduce a richer binding **additively**, preserving the existing `alias -> module_path`
map for the R3 qualified rung (byte-compat):

```rust
/// One imported name binding. `local` is the name as used at call sites;
/// `module_path` is the raw import source string (dotted Python module or JS
/// specifier); `member` is the original imported symbol when it differs from
/// `local` (aliases) or is a named import, else None for whole-module imports.
pub struct ImportBinding {
    pub local: String,
    pub module_path: String,
    pub member: Option<String>,
}
```

Today `imports` is **per-file nested**: `BTreeMap<file, BTreeMap<alias, module_path>>`
(`call_graph.rs:157`). Mirror that: `CallGraph.import_bindings: BTreeMap<file,
BTreeMap<local, Vec<ImportBinding>>>` keyed by caller file then local name (the rung
needs per-file, by-local-name lookup; a `Vec` tolerates duplicate/re-imported locals —
last-wins shadowing handled at lookup, see Scope guards). The existing `imports`
(alias->module) map is **unchanged** and still drives R3 (byte-compat).

### Module-path → repo-file resolution

A helper `resolve_module_to_files(caller_file, module_path, lang) -> Vec<RepoFile>`:
- **Python:** dotted absolute (`a.b.c` → `a/b/c.py` or `a/b/c/__init__.py`) and relative
  (`.mod` / `..pkg.mod`) anchored at the caller's directory; resolve against the set of
  indexed repo files (no filesystem stat beyond what prism already indexes). Multiple
  matches (e.g. package `__init__` re-exports) → keep all candidates (rung demotes if >1
  survive after member match).
- **JS/TS:** relative specifiers (`./mod`, `../mod`) with extension resolution
  (`.js/.ts/.tsx/.jsx`) and directory-index (`mod/index.*`). Bare specifiers
  (`"react"`, `"./node_modules"...`) = external → no repo file → fail open.
- **External / unresolved** (bare package, missing file): return empty → rung fails open
  to R5 (no behavior change for those sites).

Resolution consults only prism's already-parsed file set (`CallGraph` has the file map);
no new I/O. Deterministic ordering (BTree/sorted) for cache stability.

### The R4.5 rung

In the bare-call branch, after same-file `LocalDef` misses and before the repo-wide pool:

```
if caller_lang in {Python, JS, TS, Tsx}:
    if let Some(bindings) = import_bindings[caller_file].get(callee_name):
        target_files = union(resolve_module_to_files(caller_file, b.module_path) for b in bindings)
        member_names = { b.member.unwrap_or(b.local) for b in bindings }   # what to match in target
        candidates = free functions named `member_name` defined in any target_file
        match candidates.len():
            1 => Exact, ResolutionKind::ImportMember
            >1 => demote NameOnly (ImportMember-multi) OR fall through to R5  # see Open Decision 1
            0 => fall through to R5 (member not found in resolved file → external/re-export)
```

New `ResolutionKind::ImportMember` (serializes `"import_member"`) so the buy is visible
in call-stats and isolated from `free_single` / `import_qualified` / `free_multi`.

**Soundness invariants:**
- Exact **only** on a single candidate. This cannot increase `multi_target_exact_sites`
  (the wrong-singleton canary) because a site gets at most one Exact target here.
- A bare call with **no** import binding for its name → rung is a no-op → R5 unchanged.
- Member name match uses the **imported member** (`from x import foo as bar; bar()` looks
  for `foo` in `x`, not `bar`).
- External/unresolved module → fail open to R5 (preserve current NameOnly, no new drop).
- Non-Python/JS/TS callers: rung never runs (Rust/Go byte-identical).

## Scope guards (first merge — keep the slice thin & sound)

1. **Named imports only.** Python `from x import f` / `from x import f as g`. JS/TS
   `import { f } from "./x"` / `import { f as g } from "./x"`. **Defer**: JS default
   imports (`import f from "./x"`), namespace (`import * as ns`), CommonJS
   `const { f } = require("./x")` — these need export-shape modeling (their own slice).
2. **Relative + absolute repo paths only.** External bare specifiers fail open.
3. **Free functions only** (methods already excluded by the bare-call branch). Imported
   **classes** used as bare calls (`Foo()` constructor) are out of scope here.
4. **Last-wins shadowing:** if the same local name has both a same-file `LocalDef` (R4)
   and an import binding, R4 already won (rung runs only after R4 miss). If a name has
   multiple import bindings (re-import), union the targets; >1 surviving candidate demotes.
5. **Preserve `r5_cross_file_free_multi_kept_demoted`** (`tests/integration/resolution_test.rs:1475`)
   and `tests/integration/resolution_test.rs:1431` (same-file LocalDef) byte-for-byte.

## Open decisions (best-judgment defaults; owner may revisit)

1. **Multi-candidate after member match → demote-NameOnly(`import_member`) vs fall-through
   to R5(`free_multi`).** Default: **fall through to R5** (`free_multi`), so the canary and
   existing NameOnly telemetry are unchanged and the rung *only ever adds Exact*. This is
   the most conservative (zero NameOnly churn) and keeps the buy attributable purely to
   `import_member`. (Demote-as-import_member is deferrable telemetry.)
2. **Python package `__init__.py` re-exports.** A `from pkg import f` where `pkg/__init__.py`
   re-exports `f` from `pkg.impl`. Default: resolve to `pkg/__init__.py`; if `f` is not a
   free def there (only a re-export), candidates=0 → fall through to R5 (no wrong edge,
   no buy). Following re-export chains is deferred (needs export modeling).

## Test plan (TDD — each is a discriminating fixture)

Python (`tests/lang/python/`, new `import_binding_test.rs` or extend resolver tests):
- `from .mod import f; f()` with one `f` in `mod.py` → Exact `import_member`.
- `from .mod import f as g; g()` → resolves member `f` in `mod.py` → Exact.
- **External shadow:** `from external_pkg import f; f()` (no repo file) → stays `free_multi`
  / R5 (fail open, no Exact).
- **Same-dir decoy:** two sibling files each define `f`, caller imports from exactly one →
  Exact to the imported one (NOT the sibling) — the soundness-critical case.
- **Multi-candidate:** member name defined in two resolved target files → default
  fall-through to R5 (`free_multi`), no Exact (Open Decision 1).
- **No-import bare call** with >1 repo-wide def → unchanged `free_multi`.

JS/TS (`tests/lang/javascript/`, `tests/lang/typescript/`):
- `import { f } from "./mod"; f()` → Exact (extension + index resolution).
- `import { f as g } from "./mod"; g()` → member `f` matched → Exact.
- **Default/CommonJS deferred:** `import f from "./mod"` and `const {f}=require("./mod")`
  assert **no** `import_member` (out of scope; stays R5) — non-regression guards.
- External package `import { f } from "react"` → fail open.

Cache: `CallGraph` gains `import_bindings` (serialized) → **bump `CACHE_VERSION`** (23→24,
slice 2 shelved so base is main) + the cache-version assertion test.

## Acceptance (gates)

- **Buy:** pydantic/fastapi/express `free_multi` NameOnly **down** by ≈ the import-singleton
  count and `import_member` Exact **up** by the same (≈241 pydantic / 64 fastapi / 1 express).
- **Canary `multi_target_exact_sites` byte-FLAT** on every corpus.
- **Rust/Go (ripgrep, caddy) call-stats BYTE-IDENTICAL** (rung is Python/JS/TS-gated).
- **JS inert until JS named-imports land**; if JS named-import resolution ships in this
  slice, express buy is +1 and excalidraw stays sound (verify byte-delta is only `import_member`).
- Tier-A `--matrix-only` 0-regression (touches resolution); suite green; fmt clean.
- Build both binaries via git worktree; never swap the binary mid-measurement.

## Risks / unknowns

- **Aliases currently lose the imported member** — fixing `extract_imports` to carry
  `member` is mandatory and is the main data-model change. Must not perturb the existing
  `imports` (alias->module) map that R3 depends on (keep it; add `import_bindings` beside it).
- **JS default/CommonJS export shapes unmodeled** → named imports first; default/CommonJS
  deferred with explicit non-regression guards.
- **pydantic is mixed-language**; the 25,293 `free_multi` edge bucket is the source of
  truth, the site-split is a coverage-limited estimate — acceptance keys on the call-stats
  delta, not the estimate.
- **Module-path resolution false-negatives** (unusual layouts, namespace packages) → fail
  open to R5, never a wrong Exact. Recall-only risk, not soundness.

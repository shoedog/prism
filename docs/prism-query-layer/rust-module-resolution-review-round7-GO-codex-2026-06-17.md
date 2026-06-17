I’ll keep this read-only and bounded. I’m going to sample the current resolver/module code and repo history enough to verify whether the proposed architecture aligns with the existing failure modes, then I’ll pressure-test the spec against Rust and C++ semantics directly.The repo currently still centers resolution in `src/resolution.rs` and cache invalidation in `src/cpg_cache.rs`; the recent commits are exactly the spec revisions leading to rev6. I’m going to read the resolver hotspots and the spec commit diff to check whether the spec’s claimed fixes match the actual architecture pressures.The existing call resolver confirms the spec is addressing a real recall-safety issue: qualified Rust paths currently have stem/owner fallback behavior, so the rev6 “graph-or-fall-through” requirement is not cosmetic. I’m now checking the AST/import extraction boundaries and cache behavior so I can separate architecture issues from implementation chores.The spec’s cache section also matches a current gap: today’s cache hashes source content and binary identity, but not manifests, module file existence, `#[path]` outcomes, or cfg inputs as first-class graph inputs. That is a valid Phase 1 implementation requirement, not a new model problem.I have enough source context for implementation pressure: current import extraction excludes Rust entirely, current qualified resolution is heuristic, and current cache key would need widening. I’m doing the semantic pass now: Rust module/use/macros/cfg and the C++ stress cases, looking specifically for “cannot be represented without changing core” versus “policy/populator must be precise.”**VERDICT**: SOUND-WITH-CONCERNS.

No remaining core-shape [BLOCKER] found. The `Scope` + namespace-qualified `Binding` + conditioned/open-kind `Edge` + policy-driven resolver is now comprehensive enough to proceed to Rust Phase 1 slicing, provided the Phase 1 plan preserves the spec’s fall-through/poison rules exactly.

**FINDINGS**

1. [MAJOR] §4.3b / §7: Macro poison is architecturally representable, but under-specified for implementation.
   A flat binding set can model `macro_rules!` textual scope only if the populator computes precise `vis_extents`: later same-name macros shadow earlier ones, and older macros may become visible again after an inner shadowing scope ends. For unexpanded item/proc/attribute macros, “affected names” is not knowable; Phase 1 must treat an item-position unexpanded macro as wildcard poison for the relevant namespace/range, effectively like a deferred glob. Otherwise common code with macro-generated `fn f` can wrongly resolve `f()` to an outer in-repo `fn f`.

2. [MAJOR] §1 / §5: C++ occurrence visibility is plausible but the contract needs one more sentence.
   The core can represent reopened namespaces, anonymous namespace per TU, inline namespaces, overload sets, using directives, and ADL without a struct change. But header inclusion correctness relies on `visible()` being allowed to consult occurrence/include context beyond `Binding.vis_extents`, because `Binding` itself has no `occ`. That is acceptable through `ResolveQuery.ctx` + `ScopeExtent.occ`, but the spec should explicitly say binding visibility may be occurrence-qualified by policy, not only edge visibility.

3. [MINOR] §1: `visible(binding, q)` is too narrow as written.
   Rust glob re-export visibility and C++ access checks sometimes need traversal provenance/current edge/current lookup scope, not just the binding and original query. The surrounding prose says “query/traversal context”; make the hook signature reflect that. No data-model change required.

4. [MINOR] §9 / §10: Qualified Rust `::` phasing is internally easy to misread.
   §10 correctly requires Phase 1 to graph-resolve qualified paths or fall through, disabling the current legacy heuristic in [src/resolution.rs](/Users/wesleyjinks/code/slicing/src/resolution.rs). §9 still says “qualified `::`-call resolution via the graph” is Phase 3. Clarify: full precision is Phase 3; Phase 1 safety is graph-or-disable.

5. [MINOR] §8: Stable IDs need stronger keys for hard cases.
   `(crate, module-path, item-name, ns, ordinal)` is not enough by itself for cfg-duplicate modules, block locals, anonymous C++ namespaces, macro-generated/reopened extents, or same-name items in the same module. Add source file/byte, condition fingerprint, and TU/occurrence where applicable.

**Representability Checks**

Rust §2 rows are representable without core changes:
- crate roots/workspaces/dep renames: `Root` scopes + extern/member bindings.
- `mod`, inline mod, `#[path]`, cfg duplicate mods: `Module` scopes + conditioned bindings.
- block-local `use` and locals: `Block` scopes + `vis_extents` + `Target::Local`.
- `use a::{self, b}`: separate pending bindings; `self` binds the prefix’s final name.
- explicit/glob/re-export chains: named imports as `Binding(Pending)`, globs as edges, cycle-guarded fixpoint.
- glob-of-glob and re-export cycles: recursive edge expansion with cycle guard; poison if deferred.
- glob-vs-explicit: §3.4’s per-rib explicit-before-glob rule is correct. Current-scope glob before lexical parent is also correct for Rust block ribs.
- cfg alternatives vs ambiguity: per-candidate `cond` is enough; non-exclusive uncertainty falls through.
- `pub(in path)`: `Vis` + policy visibility works.
- macro textual scope: representable via multi-region extents, with the macro poison caveat above.
- 2015/2018 anchors: the split is correct: 2015 `use`/`::` crate-root, expression bare lookup lexical; 2018+ bare lookup with extern-prelude participation after local names.

C++ §2.5 is also representable as data:
- reopened namespaces: multi-extent `Module`.
- anonymous namespace: TU-owned/internal `Module`.
- inline namespace: transparent open-kind edges.
- using declaration/directive: binding vs open edge.
- overload set: multiple value bindings combined by policy into `ResolvedSet`.
- ADL: policy candidate injection from `ResolveQuery.ctx`.
- header inclusion/order: adequate if the occurrence-visibility clarification above is made.

**§11 Answers**

1. Rust Phase 1 is sound enough to slice if macro poison is wildcard-safe and qualified legacy resolution is disabled where graph authority applies.
2. Round-6 fixes are correct: module-boundary stop is necessary; `Target::Local`/`callable` fixes local shadowing; qualified graph-or-fall-through fixes a real current wrong-edge path.
3. Engine/policy seam is clean enough. No Rust-only concept has to live in core.
4. Go/Java/Python/TS/JS still fit the core. `Target::Local` helps them; `visible()` handles capitalization/access/package/protected rules.
5. GO for Rust Phase 1. Minimal surface: core graph types, Rust crate/module/block populator, pending import/re-export fixpoint, Rust anchor policy, local/value shadow bindings, visibility enforce-or-fall-through, cfg condition carry, glob/macro/pending poison, and consumer replacement for unqualified plus qualified-safe fall-through.
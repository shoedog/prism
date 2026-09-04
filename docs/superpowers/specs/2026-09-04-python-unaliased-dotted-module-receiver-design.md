# Python unaliased dotted-module receiver ownership

**Status:** accepted for implementation
**Recorded:** 2026-09-04
**Exact base:** `4298e548003cbb59cf506531142d177169a7a28e` (PR #227 merge)
**Scope:** Python `import pkg.models` plus `pkg.models.Class` annotations and constructor locals, direct methods only

## 1. Decision and boundary

Implement the first bounded increment of the roadmap's Python authoritative module/scope item:

```python
import pkg.models

def run(client: pkg.models.Client):
    client.send()
```

`client.send()` may resolve `Exact` to `pkg/models.py::Client.send` only when the complete proof chain below succeeds. The receiver qualifier must equal the complete imported dotted module path; the name bound in the caller remains its first component (`pkg`).

Excluded: shortened `models.Client`, a different or deeper qualifier than the imported path, `from pkg import models`, explicit aliases pretending to be an unaliased root, function/class-local imports, wildcard/duplicate/rebound imports, function-local root shadowing, inherited-only methods, re-exports, dynamic imports, forward-reference strings, generic/subscripted annotations, and a general Python scope graph. Existing `models.Client` alias behavior from PR #227 remains unchanged.

## 2. Import-shape authority

The current `ImportBinding` tuple cannot distinguish `import pkg.models` from `import pkg.models as pkg`; both record local `pkg`, module path `pkg.models`, and `ModuleImport`. Those statements have different member-access semantics, so route logic alone is insufficient.

Add `ImportBindingKind::AliasedModuleImport` for Python `import module as alias`. Retain `ModuleImport` for unaliased imports and existing JS/TS forms. This fact is serialized; CPG cache version advances 55→56. The new Exact topology also advances the navigation call-edge sidecar 24→25.

## 3. Exact proof chain

For recovered type text `Q.C` in caller file `F`, where `Q` may contain dots:

1. Split at the final dot. Every qualifier and class component is a valid Python identifier, and the class begins uppercase for constructor recovery.
2. Use the qualifier's first component as the caller-local import root.
3. Exactly one structured binding in `F` has that local root and is eligible after wildcard, duplicate-local, and module-scope rebound screening.
4. If the binding is `ModuleImport`, require the complete qualifier to equal `binding.module_path`; this admits `import pkg.models` plus `pkg.models.Client` and rejects `pkg.Client` or `pkg.other.Client`.
5. If the binding is `AliasedModuleImport`, require the qualifier to equal `binding.local`; this preserves PR #227's `import pkg.models as models` plus `models.Client` and rejects `import pkg.models as pkg` plus `pkg.models.Client`.
6. No function-local value or import binding shadows the qualifier's root.
7. Across indexed Python files, module matching selects exactly one defining file before class lookup.
8. That file contains one occurrence-clean module-scope class `C`, and existing class-span-filtered lookup finds one non-ambiguous direct method.

Any failed relevant import proof returns `Blocked`; it cannot enter same-file or global bare-owner Exact fallback.

## 4. Shared route, classification, and incremental parity

Generalize `python_qualified_receiver_parts` to return the complete qualifier prefix and final owner for two-or-more valid components. The classifier derives the local root from that qualifier, applies the existing whole-function shadow census, and retains only proof-key-authorized receiver text. Constructor extraction uses the same parser and still requires an uppercase final owner.

The shared route handles bare member imports, aliased module imports, simple unaliased module imports, and dotted unaliased module imports under the mutually exclusive rules above. Proof keys remain `(caller file, recovered type spelling, defining file, owner)`; unaliased module keys use the full module path while aliased keys use the local alias. Absent-to-present and present-to-absent proof changes trigger the existing incremental full-rebuild seam; method-body-only edits preserve the key.

## 5. RED/GREEN acceptance

RED first on exact base:

- positive: `import pkg.models` resolves typed-param and constructor-local `pkg.models.Client` receivers to the exact direct method;
- parity: direct-subset construction matches full construction;
- extraction: unaliased and explicitly aliased dotted imports have distinct kinds;
- negative: `import pkg.models as pkg` cannot authorize `pkg.models.Client`;
- negative: shortened, mismatched/deeper, unimported, duplicate-root, module-rebound, local-import, local-root-shadow, and inherited-only forms never produce recovered-owner Exact;
- incremental: absent-to-present and present-to-absent class authority match fresh builds; a method-body-only edit preserves proof keys and behavior;
- cache: CPG and navigation cache versions are pinned to 56 and 25, with existing round-trip suites covering the new enum/topology.

The positive, extraction distinction, subset, and two authority transitions must fail on the exact base. Negative controls must pass before and after implementation.

## 6. Files and verification

- `src/ast.rs`: emit `AliasedModuleImport` for Python explicit aliases; accept qualified-chain constructors through the shared parser.
- `src/call_graph.rs`: add the import kind and generalize route/proof-key generation.
- `src/resolution.rs`: derive and shadow-check the root of a multi-component qualifier.
- `src/cpg_cache.rs`, `src/navigation/call_edge_cache.rs`: version the serialized fact and changed resolution topology.
- `tests/lang/python/typed_receiver_test.rs`: focused full/subset behavior and proof barriers.
- `tests/integration/import_binding_test.rs`: extraction distinction.
- `tests/integration/resolution_test.rs`: authority transitions and stable-proof parity.

Required gates: focused RED/GREEN targets; cache round trips; complete Python/import-binding/incremental targets; `cargo fmt --all -- --check`; `cargo check --all-targets`; configured Clippy; full default and `mcp` suites with totals; release build; Tier-A matrix-only; Tier-A quick. Tier-A may use `--allow-stale-sut` only immediately after a release rebuild in this worktree; corpus drift is reported, never re-baselined.

Review cap: two rounds. At the cap, recurring/open-class qualifier, alias-shape, or scope-proof gaps park the artifact; a closed declining list may be fixed only with a disclosed extension.

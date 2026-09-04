# Python module-qualified typed-receiver ownership

**Status:** accepted for implementation
**Recorded:** 2026-09-04
**Exact base:** `5e54d48381f329cae370557eeac35bc00ff7b801` (PR #226 merge)
**Scope:** Python `import module [as alias]` plus `alias.Class` annotations and constructor locals, direct methods only

## 1. Decision and boundary

Extend the imported typed-receiver proof to the adjacent module-qualified form:

```python
import pkg.models as models

def run(client: models.Client):
    client.send()
```

`client.send()` may resolve `Exact` to `pkg/models.py::Client.send` only when every proof below succeeds. This increment supports exactly two identifier components in the recovered type: one local module alias and one class name.

Excluded: unaliased multi-component access such as `pkg.models.Client`, `from pkg import models`, function/class-local imports as authority, wildcard/duplicate/rebound imports, function-local alias shadowing, imported inheritance, re-exports, dynamic imports, forward-reference strings, generic/subscripted qualified annotations, JS/TS, and a general Python scope graph. Failed or excluded proof remains materialized residue and never enters a bare-owner Exact fallback.

## 2. Exact proof chain

For recovered type text `Q.C` in caller file `F`:

1. `Q.C` consists of exactly two valid Python identifier components.
2. `F` has exactly one structured import binding whose local name is `Q`.
3. That binding is an eligible `ModuleImport`; wildcard, duplicate-local, and module-scope rebound checks make it ineligible otherwise.
4. No function-local value/import binding named `Q` appears in the enclosing function.
5. Across indexed Python files, module matching for the binding path selects exactly one file `D`; cardinality is checked before class lookup.
6. `clean_class_spans[(D, C)]` exists, proving one occurrence-clean module-scope class.
7. Existing class-span-filtered direct-method lookup finds exactly one non-ambiguous `C.method` in `D`.

The persisted receiver type remains `Q.C`; resolution kind remains `TypedParam` or `ConstructorLocal`. Bare member-import behavior from PR #226 remains unchanged.

## 3. Eligibility and extraction

`ImportBinding.eligible` becomes kind-neutral: true means the binding survived wildcard, duplicate-local, and module-scope rebound checks. Every existing unqualified-call consumer continues to require `MemberImport`, so clean `ModuleImport` eligibility cannot enable R4c function-call resolution.

For unaliased dotted Python imports, structured extraction records Python's actual bound root (`import pkg.models` binds `pkg`, not `models`). This prevents the excluded invalid spelling `models.Client` from acquiring authority. Aliased imports retain the explicit alias.

The classifier treats the qualifier prefix of `Q.C` as the imported name. It retains receiver recovery only when the complete qualified proof key is present and the enclosing function does not bind `Q`; otherwise it emits materialized-only state.

## 4. Shared route and incremental parity

The existing three-way imported-class route is extended without changing its outcome contract:

- `NotImported`: no relevant structured binding; existing same-file and residue behavior continues.
- `Proven { defining_file, owner }`: direct-method lookup is restricted to that exact file/class span.
- `Blocked`: relevant import syntax exists but any proof failed; no same-file/global Exact owner fallback.

The shared proof-key set includes both bare member-import keys and module-qualified keys as `(caller file, recovered type spelling, defining file, owner)`. Incremental CPG construction already compares this set before and after merge; extending the set makes absent-to-present and present-to-absent qualified authority changes trigger the existing full-rebuild seam. Method-body-only changes keep the same key.

## 5. RED/GREEN acceptance

RED first on the exact base:

- positive: aliased dotted module import resolves typed-param and constructor-local receivers to the exact direct method;
- positive: a single-component module import resolves `models.Client`;
- negative: duplicate alias, module-scope rebound, missing/external module with a same-name decoy, wildcard import, `from pkg import models`, and inherited-only method never produce recovered-owner Exact;
- edge: function-local module import is not authority, and a function-local `Q` binding blocks a module-level qualified proof;
- edge: unaliased dotted import does not authorize the invalid shortened qualifier;
- parity: direct-subset construction matches full construction;
- incremental: absent-to-present and present-to-absent authority transitions match fresh builds; a method-body-only edit preserves the proof key and behavior.

The positive must fail on base for the missing Exact edge. Negative controls must pass before and after implementation.

## 6. Files and verification

- `src/ast.rs`: correct structured local binding for unaliased dotted Python imports.
- `src/call_graph.rs`: kind-neutral eligibility, shared qualified route/proof keys, and proven receiver projection.
- `src/resolution.rs`: qualifier-aware classifier proof/shadow gate.
- `tests/lang/python/typed_receiver_test.rs`: focused full/subset behavior.
- `tests/integration/incremental_cpg_test.rs`: both authority-transition directions and stable-proof control.

Required gates: focused RED/GREEN targets, complete Python target, import-binding integration target, `cargo fmt --all -- --check`, `cargo check --all-targets`, configured Clippy, full default and `mcp` suites with totals, release build, Tier-A matrix-only, and Tier-A quick. Tier-A may use `--allow-stale-sut` only immediately after a release rebuild in this worktree; corpus drift is reported, never re-baselined.

Review cap: two rounds. At the cap, recurring/open-class proof gaps park the artifact; a closed declining list may be fixed only with a disclosed extension.

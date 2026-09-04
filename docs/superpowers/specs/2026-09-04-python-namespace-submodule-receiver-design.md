# Python namespace-package submodule receiver ownership

**Status:** merged in PR #230 as `5051918f61c99fda83eb18936992fb62025b7669`
**Recorded:** 2026-09-04
**Exact base:** `7488bb64f333bbc93f21c31c1104a551649467f4` (PR #228 merge)
**Scope:** Python `from pkg import models [as m]` plus `models.Class`/`m.Class`, where `pkg` is proven to be an indexed namespace package and `pkg.models` is one exact indexed submodule

## 1. Decision and boundary

Implement item 2b of the owner-selected Python authoritative module/scope queue:

```python
from pkg import models

def run(client: models.Client):
    client.send()
```

The qualified receiver may resolve `Exact` to `pkg/models.py::Client.send` only when `pkg.models` is proven as an indexed namespace-package submodule. This increment deliberately does not interpret exports from a regular package initializer.

Excluded: relative `from . import models`; any indexed parent `pkg.py` or `pkg/__init__.py`; package attributes, `__getattr__`, and reexports; source-root inference; function/class-local imports; wildcard/duplicate/rebound imports; function-local alias shadowing; mismatched/deeper qualifiers; ambiguous submodule files or classes; inherited-only methods; dynamic imports; forward-reference strings; generic/subscripted annotations; and a general Python scope graph.

## 2. Existing facts and discriminator

Extraction already records `from pkg import models` as `MemberImport { local: "models", module_path: "pkg", member: Some("models") }`; an explicit alias changes only `local`. No new syntax fact is required.

For a qualified receiver `Q.C` and an eligible `MemberImport`, require `Q == binding.local`, then compose the candidate submodule identity from `binding.module_path + "." + binding.member`. Every component must be an identifier and the parent path must be absolute. Bare `from pkg import Client` remains the existing class-import route and never enters this qualified branch.

## 3. Namespace-package authority

The composed submodule is authoritative only when all of these hold:

1. Exactly one eligible caller binding has local `Q`.
2. `binding.kind` is `MemberImport`, `binding.member` is present, and `Q == binding.local`.
3. The parent path is absolute and every parent/member component is an identifier.
4. Neither the exact parent module file (`pkg.py`) nor parent package initializer (`pkg/__init__.py`) is indexed. Their presence blocks rather than attempting export analysis.
5. Existing exact dotted-module matching selects exactly one Python file for the composed identity (`pkg/models.py` or `pkg/models/__init__.py`) before class lookup; stem fallback is forbidden.
6. That file has one occurrence-clean module-scope class `C`, and existing class-span-filtered lookup finds one non-ambiguous direct method.
7. No function-local value/import binding shadows `Q`.

Any failed relevant import proof returns `Blocked`; it cannot enter same-file or global bare-owner Exact fallback.

## 4. Shared proof and incremental parity

Extend the existing `python_imported_class_route` and proof-key generation; do not add a parallel resolver. Qualified `MemberImport` proof keys remain `(caller file, recovered spelling, defining file, owner)` and use `local.Class` spelling.

Adding/removing a parent module/initializer, submodule file, or clean class changes the proof-key set and therefore takes the existing incremental full-rebuild seam. Method-body-only edits preserve the key. Because cached call-site classification and resolved navigation topology change, advance CPG cache 56→57 and navigation sidecar 25→26 with targeted round-trip tests.

## 5. RED/GREEN acceptance

RED first on exact base:

- positive: unaliased and explicitly aliased namespace submodule imports resolve typed-parameter and constructor-local receivers to the exact direct method;
- parity: direct-subset construction matches full construction;
- negative: indexed `pkg.py` or `pkg/__init__.py`, relative imports, mismatched/deeper qualifiers, duplicate/rebound/wildcard/local imports, local alias shadowing, ambiguous submodules/classes, and inherited-only methods never produce recovered-owner Exact;
- compatibility: bare `from pkg.models import Client` keeps its existing class-import Exact behavior, while `from pkg import models` cannot be treated as a bare class import;
- incremental: parent blocker and submodule/class authority transitions match fresh builds; method-body-only edits preserve proof keys and behavior;
- cache: CPG/navigation versions are pinned to 57/26, with targeted CPG and sidecar round trips for the new Exact topology.

The positive/subset/incremental authority tests and cache pins must fail on exact base. Negative and compatibility controls must pass before and after implementation.

## 6. Files and verification

- `src/call_graph.rs`: compose and prove the namespace submodule route and proof keys.
- `src/resolution.rs`, `src/cpg/build.rs`: thread the complete stored import/index authority into the shared route/proof-key consumers if required.
- `src/cpg_cache.rs`, `src/navigation/call_edge_cache.rs`: version the changed derived state/topology.
- `tests/lang/python/typed_receiver_test.rs`: focused behavior, parity, compatibility, and proof barriers.
- `tests/integration/resolution_test.rs`: authority transitions and stable-proof parity.
- `tests/ast/cpg_cache_test.rs`, `src/navigation/call_edge_cache.rs`: targeted round trips.

Required gates: focused RED/GREEN; complete Python/import-binding/incremental targets; cache round trips; format/diff/check/configured Clippy; full default and `mcp` suites with totals; release build; Tier-A matrix-only; Tier-A quick. Tier-A pin drift is reported, never re-baselined.

Review cap: two rounds. At the cap, recurring/open-class package-export or scope-proof findings park the artifact; a closed non-repeating list may be fixed within the cap.

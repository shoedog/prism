# Python imported typed-receiver ownership

**Status:** bounded design for implementation
**Recorded:** 2026-09-04
**Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9` (`origin/main`, PR #225 merge)
**Scope:** Python `from module import Class [as Alias]` receiver annotations and constructor locals, direct methods only

## 1. Decision and boundary

Extend the existing Python typed-receiver recovery only when a module-scope member import proves the receiver class's defining file and declaration. For:

```python
from pkg.models import Client as ImportedClient

def run(client: ImportedClient):
    client.send()
```

`client.send()` may resolve `Exact` to `pkg/models.py::Client.send` only if every proof in section 2 succeeds.

This slice deliberately excludes:

- module-qualified types such as `models.Client`;
- function- or class-local imports;
- wildcard, duplicate, or rebound imports;
- imported-class inheritance fallback;
- re-exports and dynamic import behavior;
- JS/TS receiver recovery; and
- a general Python scope graph.

An excluded or failed proof preserves the existing materialized-receiver residue path. It must not enter the global bare-owner Exact fallback.

## 2. Exact proof chain

For a recovered bare type spelling `T` in caller file `F`:

1. `T` has exactly one structured module-scope `ImportBinding` in `F`.
2. The binding is an eligible `MemberImport`; wildcard, duplicate-local, and module-scope rebound checks have already marked other cases ineligible.
3. The binding supplies one original member name `C` and module path `M`.
4. Across indexed Python files, `file_matches_module(file, M, F, indexed_files)` selects exactly one file `D`. Cardinality is checked before inspecting the member so a single-component stem collision cannot become Exact merely because only one candidate declares `C`.
5. `clean_class_spans[(D, C)]` exists, proving one occurrence-clean module-scope class declaration.
6. The existing class-span-filtered direct-method lookup finds exactly one non-ambiguous `C.method` in `D`.

The resolution kind remains the recovery evidence (`TypedParam` or `ConstructorLocal`). No new serialized owner field or cache version is needed: the resolver has the caller file, import index, indexed-file set, and clean class/method indexes at query time.

## 3. Extraction safety

The legacy flat import map recursively sees function-local imports, while structured `ImportBinding`s intentionally include only module-scope imports. Receiver classification therefore distinguishes:

- wildcard file: materialized only;
- bare type present in the flat map and in the structured module-scope member-import set: retain the recovered type for query-time proof;
- bare type present only in the flat map: materialized only; and
- non-imported bare type: unchanged existing behavior.

Structured import bindings and their eligibility must be built before call-site classification in both full and subset builds. The same computed maps are then stored on `CallGraph`; do not maintain two independent eligibility calculations.

## 4. Resolver behavior

Classify imported receiver ownership as one of:

- `NotImported`: run the existing same-file clean-class/direct-base logic and legacy fallback unchanged;
- `Proven { defining_file, owner }`: consult only the direct method in that exact class span; a hit returns Exact, while a miss or ambiguity falls through to residue; or
- `Blocked`: fall through to residue without same-file or global bare-owner Exact lookup.

This three-way state is required because `receiver_materialized` suppresses R3/R3b but does not by itself prevent the later R6 NameOnly residue. NameOnly residue is allowed; imported ownership must never manufacture an unrelated Exact edge.

## 5. RED/GREEN acceptance

RED first:

- positive: dotted module member import with alias resolves an annotated receiver to the exact direct method in the imported class;
- second positive path: constructor local using the imported alias resolves with `ConstructorLocal`;
- negative: two indexed files matching a single-component module name block Exact even if one has the requested class;
- negative: an eligible external/missing module plus an unrelated same-named in-repo class does not produce `TypedParam`/`ConstructorLocal` Exact;
- edge: a function-local member import remains unrecovered/materialized-only;
- edge: an imported class whose method exists only on a base class does not gain an inherited Exact edge in this slice.

The positive test must fail on the exact base for the expected missing edge. Negative/edge controls must pass before and after the implementation.

## 6. Files and verification

- `src/call_graph.rs`: build structured imports before classification in full/subset builds; pass them into receiver context.
- `src/resolution.rs`: classification gate, imported-owner proof, exact-file direct-method lookup, and three-way routing.
- `tests/lang/python/typed_receiver_test.rs`: RED regression plus negative/edge cases.
- `src/cpg_cache.rs`: unchanged unless implementation introduces serialized state (not planned).

Required gates: focused Python typed-receiver tests, `cargo fmt --check`, `cargo test` with totals, `cargo build --release`, Tier-A matrix-only, and Tier-A quick. The Tier-A commands may use `--allow-stale-sut` only immediately after the release rebuild in this worktree.

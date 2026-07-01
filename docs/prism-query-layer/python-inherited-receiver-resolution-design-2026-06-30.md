# Python Inherited Receiver Resolution Plan

Status: revised after Codex + Claude a2a-bridge review
Date: 2026-06-30
Primary goal: add a precision-preserving Python inherited receiver recall slice without flipping the intentionally unsafe `python/inherited_override` matrix guard.

## Current Signal

After PR #144, the Tier-A matrix has one remaining Python gap:

- `python/inherited_override: expected_gap`

The current fixture is intentionally unsafe:

```python
class Base:
    def go(self):
        pass

class Child(Base):
    def go(self):
        pass

def run(c):
    c.go()
```

`c` is untyped. Both `Base.go` and `Child.go` exist. Resolving that call to `Child.go` would be a guess, so the current R6 collision drop is correct. This fixture should stay `known_fail` as a negative precision guard until Prism has a sound source of receiver type information for this call.

The implementable recall gap is narrower and safer:

```python
class Base:
    def go(self):
        pass

class Child(Base):
    pass

class Other:
    def go(self):
        pass

def run(c: Child):
    c.go()
```

Today Prism recovers `receiver_type = Child`, fails `owner_lookup("Child", "go")`, then falls through to the residue pool. With another `go` owner present, that becomes a collision instead of the exact direct-base method. The missing step is direct same-file base lookup for recovered Python receiver types.

## Existing Architecture

Relevant facts already exist:

- `CallSite.receiver_type` and `receiver_recovery` are populated for Python typed parameters and constructor locals.
- `class_bases` records direct same-file base links for module-scope Python/JS/TS classes, keyed by `(file, class_span)`.
- `inherited_direct_base(caller, name)` uses `class_bases` for `self.m()` inside a class.
- `resolve_call_site_full` already gives recovered Python receiver misses a fallthrough instead of `ExternalReceiver`.

The important limitation: recovered receiver dispatch starts from an owner name such as `Child`, not from a caller method inside `Child`. `class_bases` is keyed by class span and does not let the resolver safely find the `Child` span when `Child` has no methods.

## Design

Add one graph fact:

```rust
pub clean_class_spans: BTreeMap<(String, String), (usize, usize)>
```

Key: `(file_path, owner_name)`.

Value: the byte span of a top-level class definition.

Populate it only when the class owner is occurrence-clean:

- exactly one top-level class with that name in the file
- top-level binding count for that name is exactly one
- no competing import, assignment, type alias, delete target, function definition, loop target, `match`/`case` capture, or compound-suite module-scope binding for that name

This must be produced by shared class-fact construction, not by duplicating a similar-but-separate policy. Refactor `build_class_bases` into a `build_class_facts`-style helper that returns both:

```rust
(
    BTreeMap<(String, (usize, usize)), Vec<ClassBaseLink>>,
    BTreeMap<(String, String), (usize, usize)>,
)
```

The helper should perform one top-level class scan and reuse the same module-scope binding counts, wildcard/import barriers, compound-suite binding treatment, type-alias/delete treatment, and duplicate-class checks that already protect `ClassBaseLink::SameFile`. For this slice, module-scope Python `match_statement` is an unconditional conservative barrier for both `clean_class_spans` and Python `class_bases`; this is precision-safe but can reduce inherited-self recall in files with unrelated module-scope `match` statements, so add an unrelated-match regression test. Add negative tests for `case Child:` and `case Base:` rebinding. This allows `Child` to be used as a class identity only when the existing base resolver would also treat the file as clean.

Then add:

```rust
fn recovered_receiver_direct_method(
    &self,
    caller_file: &str,
    receiver_owner: &str,
    method_name: &str,
    recovered_kind: ResolutionKind,
) -> RecoveredDirectMethod<'_>

fn inherited_recovered_receiver_direct_base(
    &self,
    caller_file: &str,
    receiver_owner: &str,
    method_name: &str,
    recovered_kind: ResolutionKind,
) -> Option<Vec<ResolvedCallee<'_>>>
```

`recovered_receiver_direct_method` is only for clean same-file class identity. It looks up `(caller_file, receiver_owner)` in `clean_class_spans`, then filters `(receiver_owner, method_name)` candidates to:

- same file
- exact receiver class byte span via `method_class_span`
- not present in `method_class_span_ambiguous`

It should return a tri-state result:

- exact hit: one candidate after filtering
- blocked: same-file receiver class identity exists and direct child candidates are present but ambiguous or dirty
- miss: receiver identity exists and no direct child method candidate exists

The blocked state prevents Prism from falling through to the inherited base helper or legacy repo-wide owner lookup when the local child method exists but cannot be represented exactly.

Concrete shape:

```rust
enum RecoveredDirectMethod<'a> {
    Hit(Vec<ResolvedCallee<'a>>),
    Blocked,
    Miss,
}
```

`inherited_recovered_receiver_direct_base` algorithm:

1. Look up `(caller_file, receiver_owner)` in `clean_class_spans`.
2. Look up `(caller_file, receiver_span)` in `class_bases`.
3. Require exactly one base slot.
4. Require that base slot to be `ClassBaseLink::SameFile`.
5. Look up `(base_owner, method_name)` in `methods`.
6. Filter candidates to the same file, the exact base class byte span via `method_class_span`, and not present in `method_class_span_ambiguous`.
7. Require `len == 1` after that filter.
8. Return `Exact` with the recovered receiver kind:
   - typed param remains `ResolutionKind::TypedParam`
   - constructor local remains `ResolutionKind::ConstructorLocal`

Do not recurse. Do not handle multiple inheritance. Do not cross files. Do not use stem or name-only fallback inside the helper.

## Resolver Integration

In the recovered receiver branch:

1. If the caller language is Python and `(caller_file, recv_ty)` exists in `clean_class_spans`, use span-scoped same-file class identity instead of repo-wide `owner_lookup`.
2. Try `recovered_receiver_direct_method` first, so a local `Child.go` override wins over an inherited base method only when it belongs to the exact recovered same-file `Child` class span.
3. If that direct method is blocked/ambiguous, do not try the base helper and do not fall through to legacy repo-wide owner lookup; preserve precision by falling through to the existing residue behavior.
4. If the direct method misses, try `inherited_recovered_receiver_direct_base`.
5. If inherited lookup hits, return it.
6. If there is no clean same-file class identity for `(caller_file, recv_ty)`, preserve existing behavior: `owner_lookup(recv_ty, name)` remains the legacy path for imported or otherwise non-local recovered receiver types.
7. If all Python recovered-receiver paths miss, Python falls through to the existing residue behavior. Non-Python recovered receiver misses keep their existing `ExternalReceiver` behavior.

This keeps untyped calls unchanged and keeps the precision floor intact.

## Fixture Strategy

Keep `eval/fixtures/python/inherited_override` as the unsafe untyped fixture:

- status remains `known_fail`
- it continues to signal that `c.go()` with untyped `c`, `Base.go`, and `Child.go` must not be promoted to an exact edge
- `eval/tests/test_matrix.py` can continue using it as the canonical `expected_gap` / `flip_candidate` known-fail exemplar
- because Tier-A `known_fail` becoming a `flip_candidate` is a reportable signal rather than a hard matrix failure, the Rust negative test is the hard precision guard and validation must explicitly check that `python/inherited_override` did not become an unexpected flip candidate

Add a new Tier-A fixture, for example `eval/fixtures/python/inherited_direct_base_typed`:

- `Base.go` is the seed.
- `Child(Base)` does not override `go`.
- `Other.go` is a collision decoy.
- `run(c: Child)` calls `c.go()`.
- status is `pass`.
- expectation includes `resolution_kind = "typed_param"`.

This adds the sound recall capability under a truthful matrix name while preserving the G5 precision guard. Tier-A `exact = true` checks caller-set equality, not `ResolutionConfidence::Exact`, so the Rust integration/unit tests must assert exact confidence directly.

## Tests

Add focused Rust tests in a new Python module to avoid further growing the existing large inheritance and typed-receiver files:

- `tests/lang/python/inherited_receiver_test.rs`
  - typed receiver `Child` inherits `Base.go` with `Other.go` decoy -> exact `TypedParam` to `Base.go`
  - constructor-local `c = Child(); c.go()` inherits `Base.go` with decoy -> exact `ConstructorLocal` to `Base.go`
  - child override wins over inherited base when `Child.go` exists
  - duplicate `go` definitions inside the exact same `Child(Base)` class block inherited base fallback
  - duplicate `go` definitions inside `Base` do not produce inherited exact
  - a different-file `class Child: def go` does not preempt same-file `Base.go` for `run(c: Child)`
  - untyped receiver collision remains dropped or non-exact
  - duplicate, imported, assigned, type-aliased, deleted, function-defined, or compound-suite-bound `Child` class owner does not produce inherited exact
  - duplicate, imported, assigned, type-aliased, deleted, function-defined, or compound-suite-bound `Base` class owner does not produce inherited exact
  - module-scope Python `match`/`case` capture rebinding of `Child` does not produce inherited exact
  - module-scope Python `match`/`case` capture rebinding of `Base` does not produce inherited exact
  - multiple inheritance does not produce inherited exact
  - register the new module in `tests/lang/python/main.rs` and update all three `coverage_test.rs` `all_test_files` arrays
- `tests/integration/resolution_test.rs`
  - add `py_inherited_base_typed_param_exact`
  - assert through `resolve_call_site_full` that the exposed `ResolutionKind` is `TypedParam` and `ResolutionConfidence::Exact`
  - this makes `cargo test --test integration py_` exercise new feature coverage instead of only existing Python receiver tests
- `tests/ast/cpg_cache_test.rs`
  - add full-vs-incremental parity for changed-file reconstruction
  - add retained-cache/serde parity for an unrelated-file rebuild
  - include direct assertions that `clean_class_spans` matches between full and incremental behavior

## Cache And Incremental Plumbing

Because the new class-span map is serialized on `CallGraph`:

- bump `CACHE_VERSION` from v29 to v30
- update the cache history comment
- rename `cache_version_is_29_for_js_ts_import_member_facts` to `cache_version_is_30_for_python_inherited_receiver_class_spans`
- update the cache-version assertion, including an explicit lib-test validation command
- thread `clean_class_spans` through:
  - empty graph construction
  - full build
  - direct subset build
  - `remove_files`
  - `merge`
  - unconditional `#[serde(default)]`, matching existing `CallGraph` fields, because cache deserialization happens before version checking

Skeleton construction should leave `clean_class_spans` empty, matching the existing `class_bases` behavior. The skeleton path is for cheap direct-call scoping and currently omits receiver recovery; extracting class inheritance facts there would add cost without improving this slice.

Production call sites:

- full build currently calls `Self::build_class_bases(files)` in Phase 5; replace it with `build_class_facts(files)` and assign both returned facts.
- direct-subset build currently calls `Self::build_class_bases(files)` before constructing `subset_files`; replace it with `build_class_facts(files)` using the complete `files` map, not `subset_files`, so unchanged class owners remain available during partial rebuild.
- skeleton/empty construction only add `clean_class_spans: BTreeMap::new()` entries and do not call the fact builder.

Add incremental parity tests for the new fact:

- changed-file reconstruction: the changed file owns `Child(Base)` and `run(c: Child)`, and a partial rebuild must still resolve to `Base.go` with `TypedParam`
- retained-cache/serde: an unrelated-file rebuild must preserve the inherited receiver edge and the `clean_class_spans` contents

Add `clean_class_spans` to `normalized_cpg_behavior` next to `class_bases`, or explicitly assert identical `clean_class_spans` in every new incremental test. Prefer normalizing it so future parity tests catch missing plumbing automatically.

Add a cache round-trip assertion showing `clean_class_spans` survives serialize/deserialize.

## Implementation Sequence

1. Add failing tests and fixtures first:
   - new Tier-A fixture under `eval/fixtures/python/inherited_direct_base_typed`
   - focused Python receiver/inheritance tests
   - one `py_` wire-level integration test
   - one CPG cache/incremental parity test
2. Add the `CallGraph.clean_class_spans` field with unconditional `#[serde(default)]` and empty initialization in skeleton/empty constructors.
3. Refactor `build_class_bases` into shared `build_class_facts` that returns both `class_bases` and `clean_class_spans` from the same scan and occurrence-clean policy.
4. Extend Python module-scope binding treatment for the shared class facts:
   - unconditional file-level barrier for module-scope Python `match_statement`
   - count/barrier PEP 695 `type_alias_statement` left names
   - count/barrier `delete_statement` targets
5. Wire the full build and direct-subset build call sites, with direct-subset using the complete `files` map.
6. Thread the new field through `remove_files`, `merge`, normalization, and cache round-trip coverage.
7. Add the span-scoped Python recovered receiver path:
   - try exact same-file child method for the recovered class span
   - suppress base lookup and legacy repo-wide lookup when the local child method is ambiguous/dirty
   - try direct-base inherited lookup only after same-file child method miss
   - use legacy repo-wide `owner_lookup` only when no clean same-file class identity exists
8. Bump cache version v29 -> v30, rename the cache-version test, and update the history comment.
9. Run targeted tests, rebuild release, then run the Tier-A matrix and quick gates.

## Concrete Tier-A Fixture

Add `eval/fixtures/python/inherited_direct_base_typed/`.

`app.py`:

```python
class Base:
    def go(self):
        pass

class Child(Base):
    pass

class Other:
    def go(self):
        pass

def run(c: Child):
    c.go()
```

`expected.toml`:

```toml
[case]
language = "python"
capability = "inherited_direct_base_typed"
status = "pass"

[seed]
file = "app.py"
symbol = "go"
line = 2

[[expect.callers]]
file = "app.py"
line = 13

[expect]
exact = true
resolution_kind = "typed_param"
```

## Validation

Required before PR:

```bash
cargo fmt
cargo test --test lang_python inherited
cargo test --test lang_python typed_receiver
cargo test --test integration py_
cargo test --test ast cpg_cache
cargo test --lib cache_version_is_30_for_python_inherited_receiver_class_spans
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
git diff --check
```

For `tier-a --quick`, exit `2` is acceptable only if the generated report matches the known harness/oracle condition: `baseline_invalid: true`, `sut_error_rate: 0.0`, and no unexpected matrix regression. Also check matrix output for `python/inherited_override`; it should remain `expected_gap`, not an unexpected `flip_candidate`.

## Non-Goals

- No inference for untyped parameters.
- No cross-file base resolution.
- No imported base resolution.
- No recursive inheritance walk.
- No C3/MRO modeling.
- No JS/TS receiver-type recovery in this slice.

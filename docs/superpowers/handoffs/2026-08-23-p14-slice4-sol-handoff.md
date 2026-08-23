# Handoff — roadmap #14 slice 4 alias identity and promoted-selector snapshot

**Written:** 2026-08-23T17:09:44Z · **By:** Codex `/root` · **Provider:** codex  
**Workspace:** `/Users/wesleyjinks/code/slicing-p14s4-sol` · branch `go-alias-aware-local-local-sol` · base `18b585a` · **Measured code checkpoint:** `ee98e98d3c9cb38d77942061b31b16647417b61c` · tree clean before this handoff  
**Authority:** owner brief > `docs/superpowers/specs/2026-08-22-go-nested-module-import-identity-design.md` §5/§6 > `docs/superpowers/specs/2026-08-22-p17-narrow-concrete-receiver-direct-design.md` §2 R1(b)  
**Provenance:** `[FRESH]` means measured in this worktree; `[SUPPLIED]` means owner-provided controls/expectations. The installed `bootstrap/handoff-template.md` named by steering was not present, so this follows the repository's existing handoff layout and includes every field requested in the brief.

## 0. Gating facts

- **Lane ownership:** `[FRESH]` no subagents or other implementer worktrees were inspected or coordinated. This is the independent sol implementation.
- **Custody:** all implementation/test stable points through `ee98e98` are committed and unpushed. This handoff is the only new file after that checkpoint and will receive its own commit. Controller owns push; do not amend.
- **In flight:** no build, test, call-stats, or eval process remains running.
- **Review cap:** three Part B self-review rounds were declared. Round 1 found missing direct fixture coverage; round 2 found two bounded non-struct embedded-owner WRONGs; round 3 found no new defect. No extension was taken.

## 1. Outcome

Part A is complete: Go aliases are expanded transitively as full canonical type expressions before Exact signature comparison, profile/arity/cycle/provenance failures produce `AliasUnresolved`, and `Local↔Local` now requires equal effective paths while `Bare↔Bare` retains the name rule.

Part B is complete as a foundation only: `CallGraph` serializes a read-only, owner/profile-keyed promoted-selector snapshot with resolved embed identity, pointer bit, source selector, ordinary fields, own methods, promoted target/depth/shadow/value-method-set facts, and `ProfileConflict`. It is built during `apply_go_interface_dispatch_with_scope_inputs`, cleared with interface dispatch, restored on cache hits, surfaced in call-stats and the interface manifest, and is not consumed by any resolution route.

## 2. Commit ledger

| Commit | Role |
|---|---|
| `9069fb6` | Part A red: alias-aware local identity poles and P10 Local/Bare split |
| `58c33ef` | Part A red: parameterized, profile, cycle, and provenance failures |
| `b36336e` | Cache red: CPG 48 / sidecar 17 pins |
| `6e226a6` | Part A red: variadic function shape |
| `cf53e50` | Part A red: unsupported alias constraint reason |
| `6cb3851` | Part A green: profile-aware full-expression alias expansion and Local path comparator |
| `e874bc1` | Part B red: snapshot axes, stored facts, telemetry, manifest, non-Go empty state |
| `2db5b16` | Part B red: no-cache / CPG-hit / sidecar-hit cache parity |
| `0c7e47f` | Cache-test admissibility correction: bincode rather than JSON map-key encoding |
| `fb8b2cc` | Part B coverage completion: direct target, unknown target, shadow/value/depth facts |
| `9b5591e` | Part B red: embedded interface and defined-type provenance |
| `ee98e98` | Part B green: serialized owner/profile snapshot and additive diagnostics |

No commit was amended and nothing was pushed.

## 3. Tests by name

### Part A alias poles

- `alias_to_local_substitutes_the_complete_rhs`
- `alias_to_qualified_substitutes_the_import_path_identity`
- `alias_to_instantiated_generic_preserves_arguments`
- `parameterized_alias_binds_each_occurrence_capture_safely`
- `parameterized_alias_wrong_arity_fails_closed`
- `unsupported_parameterized_alias_constraint_is_unresolvable`
- `predeclared_aliases_normalize_byte_and_rune`
- `alias_to_composite_substitutes_nested_pointer_slice_map_and_func`
- `aliases_in_two_packages_expand_to_one_base_type`
- `nested_aliases_expand_transitively`
- `generic_instantiation_wrapping_an_alias_keeps_shape`
- `agreeing_build_profile_alias_variants_expand`
- `alias_and_defined_build_variants_fail_closed`
- `uncertain_alias_profile_fails_closed`
- `alias_cycle_fails_closed`
- `unresolvable_alias_target_fails_closed`
- `external_test_clause_alias_is_invisible_to_production`
- `alias_function_variadic_shape_never_matches_nonvariadic`
- `bare_to_bare_without_module_keeps_the_name_rule`
- `s4_unqualified_named_types_with_two_proven_paths_do_not_match`

Resolver and manifest assertions pin exact target files in the alias suites.

### Part B snapshot poles

- `package_qualifier_is_part_of_profile_uniqueness`
- `resolved_embedded_owner_identity_is_part_of_profile_uniqueness`
- `ordinary_field_selector_names_are_part_of_profile_uniqueness`
- `own_method_names_are_part_of_profile_uniqueness`
- `embedded_alias_selector_name_is_preserved_separately_from_target_identity`
- `anonymous_struct_embed_is_an_explicit_profile_conflict`
- `unresolvable_embedded_identity_is_an_explicit_profile_conflict`
- `resolved_embedded_interface_is_not_an_unresolvable_identity`
- `embedded_defined_type_contributes_its_promoted_method`
- `depth_two_profile_conflict_taints_the_outer_owner`
- `duplicate_identical_profile_declarations_are_not_a_conflict`
- `promoted_method_facts_preserve_shadow_and_value_method_set`
- `promoted_method_facts_preserve_depth_two_target_identity`
- `receiver_method_set_shape_is_a_fifth_profile_safety_axis`
- `snapshot_counts_reach_call_stats_and_the_manifest_diagnostic`
- `non_go_graph_has_an_empty_snapshot`
- `go_promoted_snapshot_is_byte_equal_without_cache_on_exact_hit_and_sidecar_hit`

The cache test compares bincode bytes for a no-cache build, cache miss, exact CPG hit, and sidecar hit.

## 4. Verification

| Check | Fresh result |
|---|---|
| `cargo fmt --check` | pass |
| `git diff --check` | pass |
| `cargo check` | pass |
| focused Part A Go target before Part B | 153 passed / 0 failed / 0 ignored |
| Part B snapshot focus after final fixes | 16 passed / 0 failed / 0 ignored (within the final 169-test Go target) |
| full Go test binary | 169 passed / 0 failed / 0 ignored |
| full navigation test binary | 108 passed / 0 failed / 0 ignored |
| final `cargo test --quiet` | 3,326 passed / 0 failed / 1 ignored across 28 summaries |
| `cargo build --release` after Part A | pass |
| `cargo build --release` after Part B | pass; binary `slicing 3.1.2 (ee98e98d3c9c)` |
| cache pins | CPG 48 / sidecar 17 |
| max changed implementation file | `src/go_promoted_snapshot.rs`, 581 lines |

### Tier-A exclusion

`eval/.venv/bin/tier-a --matrix-only --allow-stale-sut` could not start because this fresh clone has no `eval/.venv`. The repository harness exists, but system Python lacks `tomllib` and is below the declared Python 3.12 floor. `uv run --offline ...` was then blocked from `~/.cache/uv`; the escalated offline cache-read request was rejected. Those probes are inadmissible and no matrix result is claimed. Tier-A quick was also not run for the same missing environment. Controller must run the expected 104-case matrix (and quick, if required) in an enabled environment.

Per owner instruction, the gopls oracle was not run. Therefore real-corpus 1→0 zero-fanout oracle scoring is not measured here. The fail-closed unit poles demonstrate the mechanism; all five fresh corpora reported `go_alias_unresolved={}`.

## 5. Five-corpus same-base evidence

Controls are `[SUPPLIED]` `ctrl514-{ripgrep,caddy,prometheus,etcd,hugo}.txt` from main `514cfe3`, stated resolution-equivalent to base `18b585a`. Candidate runs are `[FRESH]` `target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/<corpus>`.

### Part A versus control

| Corpus | `interface_dispatch` Exact | Interface fanout leaf diff | `go_alias_expanded` | `go_alias_unresolved` | Other leaf result |
|---|---:|---|---:|---|---|
| ripgrep | `0→0` | `{}` unchanged | 0 | `{}` | pre-existing JSON byte-identical; only two additive zero alias keys |
| caddy | `1766→1766` | byte-identical | 0 | `{}` | no other change |
| prometheus | `2461→2498` | bucket `7: 9→12`; all others unchanged | 25 | `{}` | only downstream interface/multi-target counters changed |
| etcd | `2002→2062` | `1:445→420`, `2:196→202`, `3:93→112`; all others unchanged | 33 | `{}` | only downstream interface/multi-target counters changed |
| hugo | `625→625` | byte-identical | 8 | `{}` | no other change |

Prometheus's downstream changes include interface multi-target sites `3855→3862`; etcd's `515→548`. No unrelated language leaf changed.

### Part B versus retained Part A output

| Corpus | Exact / fanout | Snapshot owners | Profile conflicts | Promoted methods | Isolation result |
|---|---|---:|---:|---:|---|
| ripgrep | byte-identical | 0 | 0 | 0 | only three additive zero snapshot keys |
| caddy | byte-identical | 367 | 59 | 181 | only three additive snapshot keys |
| prometheus | byte-identical | 1,190 | 37 | 416 | only three additive snapshot keys |
| etcd | byte-identical | 974 | 57 | 966 | only three additive snapshot keys |
| hugo | byte-identical | 972 | 59 | 866 | only three additive snapshot keys |

Part B changed no pre-existing call-stats leaf on any corpus. This is the measured proof that it added no edge, removed no edge, and altered no resolution route.

## 6. Design discrepancies and fifth axis

- The authoritative slice-4 spec still names cache versions 47/16, while the direct owner brief sequences this work after #17 and mandates 48/17. The clone base did not contain #17 when the bump landed; 48/17 was used exactly as instructed. Controller must reconcile stacking before PR.
- **Fifth profile-safety axis:** receiver method-set shape. Profile variants can retain the same own-method name while switching `func (B) M()` to `func (*B) M()`, changing `value_method_set`. The snapshot therefore carries and compares `(method name, pointer_receiver)` in addition to the four required axes. The test `receiver_method_set_shape_is_a_fifth_profile_safety_axis` makes both `B` and an outer `S{B}` conflict.
- Resolved embedded interfaces are terminal identities, not conflicts. Defined non-struct embedded types are internal hop profiles and can contribute promoted methods. Both were found in capped self-review and pinned red before their fix.
- Anonymous invalid struct-embed syntax is retained by tree-sitter below a top-level `ERROR`. Snapshot-local recovery records it as conflict without admitting the malformed declaration to provider routing.

## 7. Resume order and stop conditions

1. Controller runs Tier-A matrix (expected 104) and any required quick check in a Python 3.12+ eval environment.
2. Controller compares this independent implementation with the other implementer, reconciles #17 stacking/cache pins, and performs the independent review.
3. Controller pushes the existing commits without amend if accepted.

Do not consume the snapshot for routing in this slice. Do not rebaseline corpora or Tier-A. Do not infer an oracle verdict from call-stats. Do not push from this lane.

## 8. Verdict

**Implementer verdict:** SELF-PASS, not independent approval. Part A and Part B are committed, full Rust tests and both release builds are green, cache parity is byte-pinned, and Part B is corpus-inert outside additive telemetry. Acceptance remains gated on the controller's 104-case Tier-A run, independent comparison/review, and sequencing reconciliation.

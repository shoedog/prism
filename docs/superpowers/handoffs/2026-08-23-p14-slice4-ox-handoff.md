# Handoff — roadmap #14 slice 4, implementer Ox (branch `go-alias-aware-local-local-ox`)

Date: 2026-08-23 · Base: main@18b585a (docs-only descendant of 514cfe3 control base — resolution-identical) · HEAD: `a8e7d18`

## Commits (in order; red-first per pole group)
1. `3ca0a31` test: 14 RED fixtures for alias-aware `Local↔Local` by path (spec §5 list, resolver + manifest parity, target-file identities).
2. `059ef5e` feat: Part A — profile/clause-scoped alias index (`src/go_alias_index.rs`), whole-RHS canonical expansion in `canon_type`, fail-closed `AliasUnresolved{defined_variant|profile_uncertain|cycle|arity|unresolvable}`, byte→uint8 / rune→int32 normalization, `Local↔Local ⇒ path equality` comparator, telemetry.
3. `45c3b30` test: 11 RED fixtures for the owner/profile-keyed promoted-selector snapshot (four axes, depth-2 poisoning, cache byte-parity, foundation no-routing-change).
4. `3c70081` feat: Part B — `src/go_promoted_snapshot.rs`, serialized on `CallGraph` (`#[serde(default)]`), read accessor + `_mut` test seam, call-stats + additive manifest diagnostic. NOT consulted by any route.
5. `a8e7d18` build(cache): CPG 46→**48**, sidecar 15→**17** + pin renames. **#17-narrow has NOT merged on main when I reached the bump** (main is still at 18b585a; its branch carries no cache change I could see), so per brief I used 48/17 directly and flag it here for controller reconciliation before the PR.

## Tests (all green; full suite `cargo test`: **3315 passed / 0 failed / 1 ignored**)
Part A — `tests/lang/go/alias_local_local_test.rs`: `s4_alias_to_same_package_local_type_expands_to_the_rhs`,
`s4_alias_to_qualified_type_expands_through_the_import_map`, `s4_alias_to_instantiated_generic_keeps_exact_against_direct_instantiation`,
`s4_parameterized_alias_expands_with_arity_checked_binding`, `s4_parameterized_alias_wrong_arity_fails_closed`,
`s4_byte_and_rune_aliases_normalize_to_uint8_and_int32`, `s4_alias_to_composite_predeclared_type_expands`,
`s4_aliases_in_two_packages_to_one_base_type_keep_exact`, `s4_disagreeing_build_profile_variants_fail_closed`,
`s4_agreeing_build_profile_variants_expand`, `s4_distinct_defined_types_with_the_same_name_in_two_proven_packages_no_longer_match`,
`s4_bare_bare_without_gomod_still_keeps_the_name_rule`, `s4_alias_cycle_fails_closed`,
`s4_test_clause_alias_is_invisible_to_production_consumers`, `s4_generic_instantiation_wrapping_an_alias_keeps_shape`.
Renamed P10 fixture in `owner_partition_fix_wave_test.rs`:
`s4_unqualified_named_types_keep_the_existing_bare_name_rule` → `s4_unqualified_named_types_in_proven_paths_no_longer_match_by_name`
(Bare↔Bare variant lives in the new file).

Part B — `tests/lang/go/promoted_snapshot_test.rs`: embed-identity axis, ordinary-field axis, own-method axis,
embedded-alias SELECTOR-vs-resolved-identity axis (`type A = B` in `S{A}` vs `S{B}`), package-qualifier embed
(`q.B` vs `r.B` resolves to different owners), anonymous inline struct embed fails closed, depth-2 path-owner
poisoning, duplicate identical declarations → NOT a conflict, shadowing + value_method_set bits,
foundation/no-routing-change + bincode round-trip byte-equality.

Cache pins: `cache_versions_are_pinned_for_slice4_alias_aware_local_local` (48), `sidecar_version_is_pinned_for_slice4_alias_aware_local_local` (17).

## Implementation notes
- Parameterized aliases: tree-sitter-go has NO generic-alias production — `type Twice[T any] = Pair[T,T]` parses as
  `type_spec` with a `type_parameter_list` and an ERROR node holding `=`. Detection keys on that shape; RHS params are
  rewritten to `%N%` placeholders at index build; consumption arity-checks then substitutes then transitively expands.
  Constraints other than `any`/`interface{}` make the variant unresolvable (fail closed).
- Expansion is transitive on canonical strings with a per-alias-key cycle guard; visibility via
  `exact_declaration_visibility` (own package) / `exact_cross_package_visibility` (qualified); all-exactly-visible
  variants must be `Alias` AND canonically identical. All-`Defined` names keep existing leaf behavior (no gap).
- Qualified embeds in the Part B snapshot resolve through import map + module-graph path→dir; ambiguous dirs or
  multiple ordinary clauses fail closed. Embedded INTERFACES are recorded but deferred to interface dispatch
  (not a conflict). Anonymous inline struct embeds (grammar ERROR sibling `{ struct…`) are recorded as unresolvable
  → conflict. Own-method axis compares name sets contributed PER BUILD PROFILE of ordinary-clause files.
- ERROR-recovery added to top-level Go extraction so an anonymous-embed ERROR cannot erase its whole owner.

## Verification totals
- `cargo fmt` clean; clippy warning count == pre-branch baseline (**319** lines, no new warnings).
- `cargo test`: 3315 passed / 0 failed / 1 ignored.
- `cargo build --release` + `eval: uv run tier-a --matrix-only --allow-stale-sut`: **104 matrix-ok, 0 failures**.

## 5-corpus same-base leaf diff (`nav --no-cache call-stats` vs scratchpad `ctrl514-*.txt`)
ripgrep: **BYTE-IDENTICAL** to control after BOTH parts (non-Go untouched).

### PART A only (vs ctrl514)
| corpus | interface_dispatch Exact | fanout shifts | go_alias_expanded | go_alias_unresolved | AliasUnresolved site-gaps |
|---|---|---|---|---|---|
| caddy   | 1766→1753 (−13) | hist "18"−1, new "5":+1 | 0 | defined_variant 25 | — |
| prometheus | 2461→2183 (−278) | fanout-1 sites −6 etc.; qualifier_field/multi_target_exact_sites −85 | 1 | defined_variant 319 | 13 |
| etcd    | 2002→1616 (−386) | broad histogram shift; multi_target_exact_sites −29 | 0 | defined_variant 875 | 114 |
| hugo    | total unchanged (fanout 3: 71→68) | — | 1 | defined_variant 14 | 6 |

Recall loss is visible via the oracle's zero-fanout scoring (1→0 transitions): prometheus fanout-1 bucket dropped
121→127? — precisely: `"1"` 121→127 GAINED sites while Exact fell; every loss is a mixed Alias/Defined-profile leaf,
i.e. the spec-mandated fail-closed class. The hardened oracle delta run (gate_ok) is the controller's step; I did not
run gopls.

### PART B only (vs PART A outputs — resolution leaves IDENTICAL)
Only three new telemetry leaves differ, per corpus:
caddy owners 367 / conflicts 61 / promoted 119 · prometheus 1190/67/256 · etcd 974/102/574 · hugo 972/120/162 ·
ripgrep: absent (non-Go gate) → byte-identical.

## Not done / limitations
- Did NOT run the gopls oracle (controller does). The etcd/caddy/prometheus Exact losses above need the delta-mode
  attribution pass; every loss I inspected traces to `defined_variant`.
- Cache-parity is proven via whole-graph bincode round-trip byte-equality plus stripped-snapshot resolution parity;
  I did not exercise a literal on-disk sidecar HIT end-to-end for the new field (the field rides `CallGraph` serde,
  which the CPG-cache roundtrip covers).
- #17-narrow not merged ⇒ cache numbers are 48/17 with a skipped 47/16 step; if the controller lands #17 first, add
  one intermediate bump comment (no code change needed beyond history).
- Fifth profile-safety axis: none found beyond the four carried; the closest candidates I hit (embedded interfaces,
  test-clause methods) are handled as explicit non-conflict deferrals inside the four axes. If a reviewer wants extra
  safety, `conflict_axes` strings ("embed_identity", "ordinary_fields", "own_methods", "anonymous_embed",
  "unresolved_embed") are where a fifth would be inserted.

## Controller checklist
- [ ] Push wave commits (done locally, not pushed).
- [ ] Reconcile cache 47/16 vs 48/17 ordering with #17-narrow merge.
- [ ] Oracle delta runs per corpus (gate_ok) for Part A losses.

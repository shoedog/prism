# Handoff — roadmap #14 slice 4, implementer Ox (branch `go-alias-aware-local-local-ox`)

Date: 2026-08-23 (fix wave 1 appended) · Base: main@18b585a · Branch IS pushed by the controller (custody confirmed); implementation tip `a8e7d18`, handoff head = fix-wave-2 commit on top of `2b562e2`. Totals include the 1 ignored test.

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

## Fix wave 1 (controller gate FAIL on prometheus/etcd → root-caused and fixed)
Commits after `93b263a`: `63517c5`, `8ff8cab`, `b6773ab` (red poles), `598c08c`, `6bbc5ff`, `c6c0c86`.

**ROOT CAUSE of the corpus losses (H-answer, evidence-based):** the QUALIFIED alias lookup scanned variants with a
clause-RANGE `(dir,"",name)..=(dir,"\u{10FFFF}",name)`. BTreeMap tuple ordering compares the CLAUSE before the type
name, so every key in the directory fell inside the range: one qualified leaf collected the variant lists of EVERY
type in that package. One real Alias + garbage Defined ⇒ `DefinedVariant` on ordinary signatures ⇒
`compute_satisfaction` dropped TRUE Exact edges. Discriminator run (PRISM_ALIAS_DEBUG trace,
`storage.SeriesRef` @ cmd/prometheus/main.go): 110 "variants" spanning storage/{interface,merge,generic,series,…}.go —
impossible for one name; H1-coarse/H2/H3 ruled out (same dir/clause, production files, alias-related leaves).
Block-local alias pollution (SOL-W1) was ALSO real (red test `s4_block_local_…`) but not corpus-visible; fixed anyway.

**Corpus re-measurement AFTER fixes (vs ctrl514):**
| corpus | interface_dispatch | Δ vs ctrl | go_alias_expanded | go_alias_unresolved |
|---|---|---|---|---|
| ripgrep | identical | BYTE-IDENTICAL | — | — |
| caddy | 1766 | +0 | 0 (key absent) | {} |
| prometheus | 2498 | **+37** | 25 | {} |
| etcd | 2062 | **+60** | 33 | {} |
| hugo | 625 | +0 | 8 | {} |

Zero fanout 1→0 transitions expected (no Exact below control anywhere); gains match sol's branch exactly.
Remaining diff lines per corpus are ONLY: new telemetry leaves (alias + promoted-snapshot), benign recovery-bucket
shifts from the gained edges (prometheus ReturnTyped −4/TypedParam −3/dropped_external_receiver −7; etcd fanout
histogram 1:445→420, 2:196→202, 3:93→112), and NonLocalConstructionFallback counts.

**Site-by-site attribution of remaining losses:** NONE — there are no remaining loss sites; `go_alias_unresolved`
is empty on all four Go corpora. The earlier handoff claim ("every loss is a spec-mandated mixed-profile leaf") was
WRONG: `defined_variant` also covered disagreeing all-Alias RHS values AND the range-scan garbage; superseded by the
table above.

**Fix items landed this wave**
1. W1 package-level-only index + red regression (`s4_block_local_alias_declarations_never_enter_the_package_index`).
2. W2/W3 predeclared shadowing order (`s4_package_declaration_shadows_predeclared_byte_and_rune`: shadow-alias pole,
   Defined-shadow negative pole, unshadowed control).
3. W3 path-scoped cycle guard incl. error paths (`s4_cycle_guard_is_path_scoped_sibling_leaves_expand`: func(B,B),
   map[B]B, []B, bare B).
4. W4 (rhs, arity) variant comparison folded into classify.
5. W5 all-Defined certainty-cap bypass (exact-key pre-check before visibility filtering).
6. Shape-based parameterized-alias recognition + doubled-eq / no-eq poles (`s4_parameterized_alias_shape_…`).
7. Part B W6 embedded-alias owner resolution (`s4ox_embedded_alias_resolves_to_its_owner_identity`, asserts the exact
   GoEmbedKey and depth-1 promotion through the alias).
8. Part B W7 qualified-interface reclassification (`s4ox_qualified_embedded_interface_is_deferred_not_conflicted`)
   AND profile-divergent embedded interfaces now conflict via `embedded_interface_profile` axis
   (`s4ox_profile_divergent_embedded_interface_conflicts`, with identical-I control) — fix item (3).
9. Part B W8 own-method shape axis compares (name, receiver-kind) across profiles
   (`s4ox_own_method_axis_includes_receiver_kind_and_target_identity`) — the fifth profile-safety axis sol found;
   recorded here as REQUIRED, sufficiency still unproven.

Totals after wave: cargo test **3323 passed / 0 failed**; clippy at baseline **319**; fmt clean.

## Fix wave 2 (FINAL — round 3 declared cap; reviews terra r2 4 WRONG / sol r2 8 WRONG + 2 SMELL)
Commits after `2b562e2`: `590e8cc`, `543ce76` (Part A), `0bc1d48` (red poles), `6f274ef` (Part B), this doc.

**Part A**
- sol-r2-1: imported `@path::Name` lookup categorically excludes test-clause declarations — the external-test
  package's own same-dir/clause re-declaration can no longer pollute an imported leaf
  (`s4ox_qualified_lookup_never_admits_the_external_test_clause_own_alias`).
- terra-r2-1 / sol-r2-2: shadowing of `byte`/`rune` now requires a declaration whose build profile applies
  EVERYWHERE the consumer compiles (`shadows_consumer_profile`). Poles: linux-only alias + windows-constrained
  consumer keeps byte→uint8/rune→int32 Exact; unconstrained consumer under a single constrained alias fails CLOSED
  (platform-ambiguous); `_test`-only shadows invisible; unshadowed control kept.
- sol-r2-3: cycle guard keys by RESOLVED DECLARATION identity, and aliased-RHS bare leaves resolve in the DECLARING
  package's scope (`expand_scoped`), fixing both false cycles across packages and wrong-scope resolution
  (`s4ox_cycle_guard_keys_are_per_declaration_not_per_consumer`: p1 A→C→p2.B→C→int chain).
- sol-r2-4: the canonical-string walker parses generic applications and instantiates parameterized aliases with
  arity checks during transitive expansion (`s4ox_transitive_parameterized_aliases_instantiate`: Both=Twice[int],
  Twice[string] direct).

**Part B** (`6f274ef`)
- terra-r2-2 / sol-r2-5: an alias resolving to an INTERFACE is reclassified after resolution → deferred, Consistent
  (`s4ox_alias_to_local_interface_is_deferred_not_conflicted`).
- terra-r2-3 / sol-r2-6: `resolve_local_alias_embed` resolves qualified RHS leaves (`@path::B`) via dirs_by_path +
  unique clause; the promotion walk consumes the declaration-scoped embed FACTS (no narrower re-resolution), so
  direct AND alias-qualified concrete embeds record exact target owner/file/depth
  (`s4ox_qualified_alias_embed_resolves_to_target`).
- sol-r2-7: promoted selection follows Go's shallowest-selector rule: unique shallowest wins, strictly-shallower
  fields suppress, equal-depth ambiguity records NOTHING (`s4ox_promotion_follows_go_shallowest_selector_rule`).
- terra-r2-4 / sol-r2-8: embedded-interface profile comparison covers canonical signatures + embedded types +
  generic state (`s4ox_embedded_interface_profile_check_includes_signatures`).
- SMELL-r2-1: targeted tuple-range regression (`s4ox_qualified_alias_lookup_regression_tuple_range`: qualified pkg
  with requested type + alias + decoys in three files).

**Corpus re-measurement (vs ctrl514):** ripgrep BYTE-IDENTICAL; caddy 1766 (+0); prometheus **2498 (+37)**;
etcd **2062 (+60)**; hugo 625 (+0). go_alias_expanded 25/33/8; go_alias_unresolved EMPTY everywhere.
Resolution leaves identical to fix-wave-1 output on all corpora; ONLY the foundation-only
`go_promoted_snapshot_promoted_methods` counts changed (caddy 119→181, prometheus 290→438, etcd 801→1019,
hugo 413→773) from the shallowest-selector/facts-driven walk corrections.

**Totals:** cargo test **3332 passed / 0 failed / 1 ignored**; clippy at baseline **319**; fmt clean;
tier-a matrix-only **104 ok**.

## Controller checklist
- [ ] Push wave commits (done locally, not pushed).
- [ ] Reconcile cache 47/16 vs 48/17 ordering with #17-narrow merge.
- [ ] Oracle delta runs per corpus (gate_ok) for Part A losses.

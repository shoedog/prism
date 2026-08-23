# #17-narrow — PROVEN concrete-recovered receivers must not reach the bare-name interface-dispatch fallback (design v7, 2026-08-23)

Status: v7 — OWNER DECISION 2026-08-23: R1(b) (promoted-from-embedded-concrete direct routing) REMOVED from this slice after its comparator failed four consecutive scoped confirmations on independent axes (package qualifier → ordinary fields → own-method axis → embedded-alias selector names) — open-class by the convergence test; promoted-concrete selectors on a proven-concrete receiver now take the fail-closed terminal drop (true edge deferred to #14 slice 4's owner/profile-keyed promoted snapshot, which must carry ALL four axes). Everything else was sol-confirmed in the scoped round (F2–F4). v7 = one-pass edit + controller cross-section self-review; committed as design-of-record on branch `p17-narrow-spec` (no further review round per owner). Previously v6 — sol r4 (FIX, "park for owner escalation": W1 profile-safe promotion unprovable with the restricted snapshot — `struct_embeds` last-file-wins across build profiles, go.rs:222/:618 — folded FAIL-CLOSED; W2 = Ox W-1 etcd baseline (already v5); W3 §8 call-stats formula vs route-specific audit — folded per route; SMELLs: `struct{ *B }` compiling non-addressable positive pole (wording was too broad); synthetic alias populates interface satisfaction; §1 ReturnTyped nit) folded. PARKED FOR OWNER ACCEPTANCE per the disclosed r4 rule. Previously v5 — Ox r4 (FIX: 1 WRONG — §8 etcd baseline/counts not resequenced vs §0, a mis-gate scenario; + kind-list nit; 2 SMELL: compiling-source premise explicit; synthetic-decl extraction widens provider diff surface) folded; Ox confirmed the R1(b) applicability simplification (no compiling counterexample). Awaiting sol r4; per the disclosed rule the spec now goes to the OWNER for acceptance (no further self-extension). Previously v4 — sol r3 (5 WRONG: addressability-blind promotion pole; ambiguous-promotion route missing; test-1 not a constructor-local input; test-9 contradicted the #17b carve-out; audit rejected valid R1(d)/(e)) + 3 SMELL (snapshot naming; unnamed interface alias; telemetry units) folded; 3 of the 5 were introduced by the controller's r2 folds → controller cross-section self-review added before dispatch. Round 4 = DISCLOSED ONE-ROUND EXTENSION of the 3-round cap (converging: bounded textual fixes, no new mechanism); a WRONG in r4 parks the spec for owner escalation. Previously v3 — sol r2 (5 WRONG / 1 SMELL: cache rehydration, R3 wording/#17b carve-out, alias canonical target + `type D I`, promoted-evidence carrier, route enum) and Ox r2 (APPROVE, 4 SMELL: shared-consult mandate, receiver_owner_identity consumers, index build-once, test poles) folded; for round 3 (sol ∥ Ox, DECLARED CAP = 3 rounds). Previously v2 — sol r1 (5 WRONG / 2 SMELL) and Ox r1 (3 WRONG / 7 SMELL) folded; for round 2 (sol ∥ Ox). Scope: a small precision PR on main,
sequenced AFTER #14 slice 3 (owner decision 2026-08-22: slice 3 merges with the 5 etcd #17-class sites documented; this slice's acceptance must turn them to 0 against `oracle-s3b-etcd.json`).

## 1. Problem (measured, hardened oracle)
Call sites whose receiver static type prism recovered as a concrete named type (`receiver_class` ∈ {constructor_local, typed_param, var_local, type_assertion}) reach
`resolve_call_site_full` WITHOUT a proven owner identity (`classify_simple_ident` emits `owner_identity: None`, resolution.rs:745–750; S1-proven
return_typed receivers DO carry the identity and are a CONTROL here, not a member of the problem class — sol r4 nit). They miss `own_method_partition` / `legacy_direct` / `go_embedded_interface_route` and land in the final bare-name ladder
`iface_key(recv_ty)` → `interface_impls` (resolution.rs:2498–2527), which mints `InterfaceDispatch` edges from ANY same-bare-name interface: caddy
`caddyfile.Adapter.Adapt` (interface `caddyconfig.Adapter`), prometheus `storage.Close` (14), etcd `integration.Cluster.Client/Endpoints` vs interface
`framework.Cluster`, `cache.Get` → `RecordingClient`. gopls `definition` = the concrete method; prism mints other types → over_approx.

## 2. Routing decomposition (the rule — replaces v1's blanket skip)
For a Go call site `recv.name(...)` with a recovered receiver static type `T` (pointer peeled):
- R1 **proven concrete owner** (declaration-kind index says `T` resolves, in the caller's package/profile view, to a struct or defined non-interface
  type with identity `(dir, clause, name)`): (a) OWN method of `T` → direct Exact to that declaration (own-method partition / QualifiedOwner lane;
  use the owner returned by a SUCCESSFUL partition selection even when it was resolved on demand rather than carried on the `CallSite` — the
  line-2321 match today only uses carried identities); (b) method PROMOTED from an embedded CONCRETE struct `B` → NOT ROUTED DIRECT IN THIS SLICE (owner decision 2026-08-23): a proven-concrete
  receiver whose selector `name` is not an own method but IS supplied by an embedded concrete struct takes the terminal drop of (e) with DropReason
  `ConcreteReceiverPromotedDeferred` (route `concrete_promoted_deferred_drop`, telemetry `go_concrete_receiver_promoted_deferred`, sites) — a
  fail-closed, precision-safe conversion of today's false interface Exact into a drop; the TRUE `B.name` edge is a pinned recall gap until #14
  slice 4 ships an owner/profile-keyed promoted-selector snapshot. WHY deferred (record for slice 4): the live provider collapses build profiles —
  `struct_embeds`/`struct_embed_files` last-file-wins per `GoOwnerIdentity` (type_providers/go.rs:222–226/:618–619), `method_declarations` per
  owner with profiles unioned (:190), `promoted_struct_method_candidates_from_data` refuses duplicate-bare outer names (:~2356) and the public
  projection drops `value_method_set` (:~2345/~2444) — and a profile-safe equality comparator for "several declarations of `S`" must carry, at EVERY
  embedding hop, the unordered set of (pointer-ness, RESOLVED embedded owner identity, embedded field SELECTOR name — `type A = B` in `S{A}` exposes
  selector `A`) ∪ the declaration's ordinary field names, PLUS the own-method axis (a profile-specific `func (S) M()` flips R1(a) vs promotion),
  with anonymous struct embeds and unresolvable identities as conflicts; four scoped confirmations each found a new axis (sol r5 fields, Ox r6
  methods, sol r6 selector names, Ox r5 qualifier) — treat the list as necessary, not proven sufficient. Applicability note kept for slice 4:
  on compiling source, promoted-selector existence ⇒ call applicability (value-embed `struct{ B }` + pointer-receiver `M` on a non-addressable
  value is a compile error; pointer-embed `struct{ *B }` puts `(*B).M` in `S`'s value method set so `makeS().M()` compiles) — `value_method_set`
  is an S4 input, not a direct-routing input. Poles for THIS slice: `type S struct{ B }; func (B) M(); func f(s S){ s.M() }` → `concrete_promoted_
  deferred_drop` (asserted: zero interface fanout, zero targets); `type S struct{ *B }; func (*B) M(); makeS().M()` → same drop; duplicate
  build-tagged `S` declarations → `Ambiguous(profile conflict)` ⇒ R3 legacy output (pinned, see §3), NOT an R1 route; (c) method supplied by an embedded INTERFACE field → S4 `go_visible_s4_implementers` via
  `go_embedded_interface_route` exactly as today (legitimate dynamic dispatch; empty-live fallback stays available there); (d) `name` is a func-valued
  field → P5 lane as today; (e) none of the above → terminal drop (DropReason: `ConcreteReceiverNoSelector`). Soundness of (e): a receiver
  proven concrete in the caller's view cannot share its bare name with an interface in that same package (Go forbids duplicate top-level names),
  so the bare-name ladder's only remaining hit would be a cross-package same-bare-name interface — never a true dispatch for a statically concrete
  receiver; embedded-interface supply is exactly (c). (Ox r2 argument, verified.)
- R2 **proven interface owner** (receiver typed as an interface, incl. alias-to-interface) → interface-owner S4 lane as today.
- R3 **unproven / ambiguous** (no declaration found for `T` in the view; multi-owner bare name; generic instantiation `Box[int]` — see §6; external type
  with no in-repo declaration) → existing behavior UNCHANGED in this slice. Honest statement (sol r2 W2): today's R3 path is NOT fail-closed —
  the bare `iface_key(recv_ty)` ladder (resolution.rs:~2498) still mints in-repo same-bare-name interface implementers for an EXTERNAL receiver
  (e.g. external concrete `q.A{}` + unrelated in-repo `p.A{M()}` → false Exact; external interface `http.Handler` + in-repo `Handler` → wrong
  implementer set). This slice forbids the bare fallback ONLY when R1 applies (proven concrete); `var w Writer` with a caller-package interface
  `Writer` is R2 (proven interface), not an R3 exception. Making R3 terminal (retiring the bare ladder for unproven receivers) is precision-correct
  but changes interface_dispatch counts on every corpus beyond the #17 sites and sacrifices unverified external-interface recall — it is carved out
  as **#17b — OWNER DECISION 2026-08-22: separate, measured first (telemetry population per corpus before #17b's design); own same-base control + oracle delta**, NOT folded here, so this slice stays narrow and its
  acceptance counts stay attributable. R3 telemetry (units explicit): `go_unproven_receiver_bare_fallback_sites` (R3 sites where the ladder was attempted),
`go_unproven_receiver_bare_fallback_hits` (sites where it minted ≥1 Exact) and `..._edges` (edges minted) so #17b's population is measured
before it ships.
- ReturnTyped sites lacking `receiver_owner_identity` are already hard-dropped at resolution.rs:2147–2153 (ExternalReceiver) — R1 applies to
  ReturnTyped only when S1 recovery proved the identity; the new logic must not double-handle that pre-drop.

## 3. Evidence sources (where "proven concrete" comes from)
A **declaration-kind index** keyed by P10 owner identity `(package_dir, package_clause, name)` + declaring profile, built from the provider snapshot:
`Struct | DefinedNonInterface | Interface | AliasToInterface{target} | AliasToConcrete{target} | AliasCyclicOrUnresolved | Ambiguous(profile conflict)`.
`Ambiguous(profile conflict)` is DEFINED as: the owner identity has >1 declaring file (`struct_declaration_files` / `type_declaration_files`,
type_providers/go.rs:170/:174, size >1) — REGARDLESS of whether the declarations agree — because the provider's method/embed/field evidence is
profile-collapsed (`method_declarations` unioned per owner, `struct_embeds` last-file-wins; see R1(b)), so no R1 route (not even R1(a) own-method)
can be proven for the caller's profile; such owners are R3 (legacy output unchanged, pinned by test (17)). This is the fail-closed closure of the
own-method axis (Ox r6: a profile-specific `func (S) M()` would otherwise flip R1(a) for the other profile).
`target` is the CANONICAL owner identity `(package_dir, package_clause, name)` + defining file/profile + pointer/value target form, resolved in the
ALIAS DECLARATION's import environment (not the call site's — `p/types.go: type A = q.S` + `p/run.go: a.M()` without importing `q` must still route
to `q.S`'s methods/interface snapshot; sol r2 W3). Both the alias graph (`type A = X`) AND the defined-underlying-type graph (`type D I` where `I`
is an interface ⇒ `D` is Interface — its method set is `I`'s; `type D S` where `S` is a struct ⇒ `DefinedNonInterface` with its OWN method set,
NOT `S`'s) resolve transitively, fail closed on cycles/unresolved; the bare global `aliases` map (type_providers/go.rs:~691/:~806, raw bare
strings) is NOT admissible for Exact routing. The index stores `interface_of: target` for `Interface`-kind entries reached through a defined type (`type D I` → `D`'s method-set snapshot is
`I`'s, followed through the graph). Unnamed interface aliases `type A = interface{ M() }` have no named target: extraction records a synthetic
alias-owned interface declaration named `A` in the declaring package (its literal method set), so the receiver is R2 with its own snapshot
— the synthetic declaration MUST also feed interface satisfaction / `interface_impls` (implementers of the literal method set), not only the
declaration snapshot (sol r4 SMELL) (sol r3 SMELL; NOTE this is new PROVIDER-layer extraction behavior, not resolution-only — the implementation plan's blast-radius estimate must
include `type_providers/go.rs`, Ox r4); pole added. Tests: `type D I`, `type A = interface{ M() }`, cross-file alias without call-site import, alias-to-pointer,
transitive alias. "Interface-presence false" is not
"concrete proven": absent entry ⇒ R3. Identity proof for constructor_local/typed_param/var_local receivers is computed at RESOLVE time from
`CallSite.receiver_type` + caller file via `resolve_go_owner_identity` + this index (on demand), or at extraction time by populating
`receiver_owner_identity` when the type is declared in the caller's own package view — pick one and state it in the implementation. MANDATE (either
choice): resolver and manifest MUST consult ONE shared function (`go_concrete_receiver_route`, the `go_embedded_interface_route` pattern,
resolution.rs:1235–1259 / queries.rs:604) — no second inline implementation of the proof. If extraction-time population is chosen: three consumers
already read `receiver_owner_identity` as a "possibly-interface owner" (interface-owner lane resolution.rs:~2214, manifest `proven_concrete_owner`
queries.rs:~639, `func_value_field_or_external_drop`); populating concrete identities interacts correctly there (interface-presence → `Some(false)` →
falls to the direct lane) and those branches must NOT be "fixed" — state this in the implementation note and pin with test (14). The index is built
ONCE per build, after `GoTypeData` collection completes and before any resolve (profile-conflict → `Ambiguous` computed at build, never per-resolve).
Placement of the shared route (sol r2): beside `go_receiver_owner`/`go_embedded_interface_route`, invoked in the recovered-receiver branch AFTER
the terminal `pre_resolved_target` lane and the ReturnTyped-without-owner / FieldTyped (`GoFieldTarget`) pre-drops, BEFORE the interface-owner
block (resolution.rs:~2213) and therefore before own/direct, embedded S4, bare-interface and P5 fallbacks; terminal before the bare ladder for
`ConcreteReceiverNoSelector`; it returns the on-demand selected owner so the direct lane drops its carried-identity-only condition
(resolution.rs:~2323). The manifest invokes the same route immediately after its interface-method denominator predicate (queries.rs:~585).
`pre_resolved_target` stays authoritative; P10 profile selection stays inside the shared route. Return contract (Ox r3): `go_concrete_receiver_route`
returns the FULL route verdict (R1 a–e / R2 / R3) and the resolver consumes that verdict — it must NOT fall into the carried-`receiver_owner_identity`-
gated interface-owner block (~2214) when no identity is carried but the route proved R2 on demand. The serialized index/snapshot types carry `serde`
derives (covered by the 46→47 / 15→16 bumps).

## 4. Manifest / oracle
`prism nav interface-manifest` emits an additive diagnostic `dispatch_route` ∈ {`concrete_direct`, `concrete_promoted_deferred_drop` (R1(b),
owner-deferred), `interface_dispatch`,
`embedded_interface_dispatch`, `func_value_field`, `concrete_no_selector_drop` (R1(e) — a PROVEN receiver must never be reported as
unproven; sol r2 W5), `unproven_drop`}; resolver drop reason, telemetry counter and manifest route are pinned TOGETHER per route (pinned-string test like `interface_manifest_receiver_class_strings`). The oracle
does NOT consume it for classification (gopls stays authoritative: `definition_kind` from gopls); it is a parity diagnostic.

## 5. Telemetry / cache
`go_concrete_receiver_direct`, `go_concrete_receiver_promoted_deferred` (sites), `go_concrete_receiver_no_selector_drop`,
`go_unproven_receiver_bare_fallback_{sites,hits,edges}` (R3, for #17b); derived call edges change → CPG **46→47** and sidecar **15→16**
(this slice is sequenced AFTER #14 slice 3 = 46/15 per the owner decision; one transition), pin tests updated;
the declaration-kind index (and the boolean "selector is promoted from an embedded concrete" evidence R1(b)'s deferred-drop needs — derived from
the existing promoted walk, owner-keyed; LABEL-ONLY: both R1(b)-deferred and R1(e) are drops, so a mis-classification between them can only
mislabel the diagnostic/telemetry, never mint an edge; no promoted-selector snapshot ships in this slice; the owner/profile-keyed one is #14 slice 4) is
SERIALIZED ON `CallGraph` (sol r2 W1): the CPG cache restores the whole
`CallGraph` (cpg_cache.rs:~185/:~315) and `build_with_cached_cpg` (cpg/context.rs:~107) rebuilds only the type registry/live types — a
"derived, not serialized" index would be ABSENT on an exact CPG hit with a cold sidecar, making the same site R1/direct cold and R3 warm (and a
freshly generated sidecar would persist the wrong answer). Populate/clear them alongside the existing declaration snapshots in
`apply_go_interface_dispatch_with_scope_inputs` (call_graph.rs: definition ~2871, callers ~1355/~2868; `clear_interface_dispatch` ~2707). Acceptance requires byte-equal resolver + manifest output across
no-cache / cold-cache-create / exact-CPG-hit / exact-sidecar-hit (a behavioral parity test, not just version pins).

## 6. Scope decisions
Generic concrete receivers (`Box[int].M()`): pre-existing drop (`resolve_go_owner_identity` rejects `[`; `iface_key` returns None on instantiation) —
OUT OF SCOPE here; pin the unchanged drop in tests and acceptance; tracked under roadmap #16/#14 slice 4 (alias/generic canonicalization).

## 7. Tests (resolver + manifest parity; assert TARGET FILES)
(1) interface `p.A` + struct `q.A` with method `M`; `a := q.A{}; a.M()` and `a := q.NewA(); a.M()` (constructor_local — the live classifier
needs a simple local qualifier whose binding is scanned, resolution.rs:~699; a bare composite-literal receiver `q.A{}.M()` is NOT recovered
today and is OUT OF SCOPE, sol r3 W3) → Exact `q.A.M` only (red today); pointer-receiver variant;
(2) typed param `c *q.C` with interface `p.C` → Exact `q.C.M` only; (3) S1-proven return-typed receiver → same; syntactic-only return-typed → pre-drop
unchanged; (4) receiver typed as interface `p.A` → implementers as today; (5) `type S struct{ I }` (embedded interface) `s.M()` → S4 implementers as
today (W1 control); (6) `type S struct{ B }` (embedded concrete) `s.M()` → `concrete_promoted_deferred_drop` (zero interface fanout, zero targets; pinned recall gap; was the promotion-lane control before the owner deferred R1(b)); (7) `type Command struct{ Run func() }`
factory-returned receiver `c.Run()` → P5 edge preserved; (8) `type A = I; func f(a A){ a.M() }` → interface dispatch preserved (alias-to-interface);
(9) `type A = q.S` alias-to-concrete → direct; plus the cross-package pole where import basename `q` is ambiguous → R3, which in THIS slice
means the LEGACY output: the fixture MUST include an unrelated same-bare interface `S` with an implementer so the pole is discriminating — assert
the legacy (false) Exact via the bare ladder AND `go_unproven_receiver_bare_fallback_*` increments; #17b flips this pole (sol r3 W4 — a fixture
without a same-bare interface does not pin the carve-out); (10) unproven multi-owner bare type → pin the ACTUAL unchanged output of today's ladder for that shape (qualified, ambiguous import basename — it may reach the ladder rather than a clean drop; Ox r3), no change under this slice; (11) `var w Writer;
w.Write()` with caller-package interface `Writer` → unchanged; (12) generic receiver → unchanged drop (pinned); (13) cache pins; non-Go byte-identical;
(14) manifest `dispatch_route` strings pinned; (15) concrete `var_local` (`var c q.C; c.M()`) and `type_assertion` (`x.(q.C).M()`) receivers →
R1 direct (every Go receiver class covered); (16) cache-parity behavioral test (cold vs exact CPG hit vs sidecar hit byte-equal); (17) owner declared in two build-tagged files (identical
or not) → `Ambiguous(profile conflict)` → R3 legacy output pinned (with and without a same-bare interface in the fixture).

## 8. Acceptance (controller)
Same-base `call-stats --no-cache` vs main, edge-counted PER ROUTE (sol r4 W3): at a changed site with old interface fanout N — R1(a):
`interface_dispatch` Exact −N, the direct receiver kind (`constructor_local`/`typed_param`/`var_local`/`type_assertion`/`return_typed`) Exact +1,
total −(N−1); R1(d) P5: `interface_dispatch` −N, `FuncValueField` +1 (NOT a direct kind), total −(N−1); R1(e) and the R1(b) deferred drop:
`interface_dispatch` −N, new edges 0, total −N; R1(c)/R2/R3: unchanged. `kinds` totals and site counts reported separately, summed over routes.
Hardened oracle in `--baseline` delta mode vs the identity-aware baselines — caddy/prometheus/hugo: `oracle-s1ebase-{caddy,prometheus,hugo}.json`
(slice 3 left their resolution leaves unchanged, 0 newly-exact); **etcd: `oracle-s3b-etcd.json`** (the post-slice-3 state that CONTAINS the 5
#17-class false Exacts — Ox r4 W-1: comparing against s1e after slice 3 merges would mis-gate or invite an ad-hoc re-baseline that silently
blesses those 5 sites); env pins corpus SHA / go1.26.2 / gopls v0.22 / darwin-arm64 / GOWORK repo-root-or-off — PLUS a **subtraction audit** (the delta gate only sees newly-Exact
sites): enumerate every changed site by `(file, start_byte, end_byte, method)` and prove before/after: oracle `definition_kind` concrete and prism
listed the false interface identities BEFORE; AFTER the audit is ROUTE-SPECIFIC (sol r3 W5): `concrete_direct` → zero interface fanout and the resolver target equals
exactly the gopls method-definition identity (full target identities/FunctionIds, never owner names or counts); `func_value_field` (R1(d)) → the
expected P5 registration target and zero interface fanout (gopls's field declaration is NOT the comparator); `concrete_no_selector_drop` (R1(e)) →
zero targets and no gopls method definition; `concrete_promoted_deferred_drop` (R1(b) deferred) → zero targets, zero interface fanout, route + telemetry pinned, and the site is listed in the PR as a known recall gap for slice 4; embedded-interface
S4 → unchanged visible implementer identities; the
changed-site population is the FULL OUTER JOIN of before/after site keys (a site that disappears from the manifest is an audited change, never a
vanished row); no oracle-INTERFACE site lost an implementer; controls (5)–(12) and (16) hold; telemetry reported in stated units (sites vs affected edges);
cache parity: no-cache / cold-create / exact-CPG-hit / exact-sidecar-hit byte-equal. Pin exact expected
counts per corpus from the baselines BEFORE implementation (prometheus concrete over_approx 17 → 0 on those sites; etcd: s1e's 11 PLUS slice 3's 5
(cache_test.go:1383/1559 Get; revision_test.go:114/126 Client; v3_failover_test.go:93 Endpoints) → 0 against `oracle-s3b-etcd.json`; hugo 3 → 0; caddy
`metrics.go:56` is external-interface-ambiguous and must remain unresolved, not newly Exact); report all four corpora even when a delta is zero. (The former "re-run slice 3's delta" step is obsolete: slice 3 is merged before this slice and
its etcd state IS the baseline.)

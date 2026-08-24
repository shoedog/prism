# A — Go receiver type-origin binding (design v3)

**Status:** design, owner-approved 2026-08-24 ("A first"). **Base:** current `main` (post-#190, post-#193; CPG 50, nav sidecar 18). **Supersedes the sequencing of** #16, which is parked (PR #203 §7-PARK) pending this work.

**Citation policy:** cite symbols; line numbers are advisory as of this writing and must be re-verified before use.

## 1. Problem

A Go receiver's *type text* is routinely recovered in one file and then re-interpreted in another file's import namespace. Exactness of the caller's import map proves that an alias maps to one directory; it does **not** prove the recovered text originated in that alias's scope. Constructible in valid Go:

```go
// p/a.go   import ext "external.example/api";  var V ext.I
// p/b.go   import ext "example.com/app/decoy"; func F() { V.M() }
```

`V`'s type text `ext.I` is transported into `b.go`, resolved there, and yields the decoy interface — which then validates and mints wrong targets. This is the defect class that refuted six successive #16 rules (`dispatchable` filter → universe membership → receiver-text qualifier → absence-of-hazards → identity establishment → exact-import-path evidence). **It is not introduced by #16** — with a single decoy declaration it already mints through the existing concrete-receiver route on `main`.

## 2. Key insight — mostly plumbing, but plumbing alone is NOT sufficient (sol r1 W1/W2)

Carrying origin-correct owners is necessary and is most of the work, but two things must land **first** or the plumbing will confidently carry *wrong* owners:
- **The shared resolver's basename fallback must not be admissible for provenance-bearing facts.** Resolving `ext.I` against its true defining file still falls from an absent exact import key to a unique directory basename, so an external `outside.example/decoy` can bind to an in-repo `decoy`. Origin-correctness does not save it. A **strict receiver-owner resolution mode** — exact import-path / effective-module identity only — is a prerequisite, not a refinement. (The shared resolver is *not* changed globally.)
- **A bare name is converted straight into a same-package owner without proving it denotes a package type.** So `var V I` in a dot-import file would be "proven" as `p.I`, and a type-parameter receiver `T` as `p.T` — both then carry `Some(owner)`, which means the fail-closed guard can no longer catch them. Poison and binder facts must therefore precede owner population, not follow it.

With those in place, the rest genuinely is plumbing:

The provenance already exists and the correct pattern is already implemented twice. `GoTypedFact` carries `{ty, defining_file}`; three sibling consults live in `go_receiver_index_visibility.rs`:

| function | resolves against | returns |
|---|---|---|
| `unique_visible_return_owner` (S1) | `fact.defining_file` ✅ | `GoPartitionSelection<GoOwnerIdentity>` |
| `resolve_go_struct_field_owner` (S2) | `declaration.defining_file` ✅ | owner **and** raw text |
| `unique_visible_type` (S3) | reads `defining_file` for the visibility test, then **discards it** ❌ | naked `Option<String>` |

`unique_visible_type` is the only one that returns text. Its single caller — package-variable recovery in `go_receiver_index.rs` — therefore stores `owner_identity: None`, even though `facts.imports`, `facts.package_basenames`, and `facts.go_file_profiles` are all in scope at that exact point. **The owner is not computed because the return type cannot express it.** Downstream, `CallGraph::go_receiver_owner` falls back to re-binding the text against the *caller's* file.

Correspondingly, `ReturnTyped` and `FieldTyped` — the two kinds that *do* carry owners — are hard-dropped when the owner is missing, but **`VarDecl` has no equivalent guard**. That asymmetry is the hole.

## 3. Design — four slices, in order (restructured per sol r1 W2 and its recommended order)

**Slice 0 — prerequisites: strict resolution + poison/binder facts.** These must precede any owner population.
- **Every populated owner must be declaration-backed (sol r2 W1).** It is not enough to poison dot-imports, type parameters, and local-type shadows: the bare branch of the resolver *synthesizes* a same-package owner without checking that a declaration exists, and the downstream owner check validates only that the package exists. A **predeclared** identifier is the surviving case — `func F(e error) { e.Error() }` would be stamped `Some(p.error)`, the route becomes `Unproven`, and the resolver deliberately falls through to the bare `interface_impls[("error","Error")]` ladder, which can mint an unrelated source-declared `q.error`'s implementer. **Rule: an owner is populated only when a visible declaration for that name exists in the resolved package; predeclared and declaration-less names stay `None` and drop terminally.** Red-first: the `error` negative above, asserted across resolver, manifest, and sidecar.
- **Strict receiver-owner resolution mode:** provenance-bearing receiver facts resolve only via exact import-path / effective-module identity; the directory-basename fallback is inadmissible. Red-first: same-basename external negative (`outside.example/decoy` vs in-repo `decoy`) ⇒ no owner, no mint.
- **Dot-import markers:** record that a file had `import . "pkg"` (today it is erased with no marker). **Poison only bare names lacking a proven lexical or package declaration — not every bare receiver in the file (sol r1 W5).** Positive control that must keep minting: `package p; import . "example/ext"; type Local struct{}; func (Local) M() {}; func F(v Local) { v.M() }`.
- **Go type-parameter binders:** collect them (the existing collector matches tree-sitter-Rust kinds and is unreachable for Go) and treat a receiver bound by one as unprovable.
- **Terminal local-type poisoning:** a locally-declared type shadowing the receiver type name is terminal, not merely a flag consumed downstream.

**Slice 1 — make provenance expressible (seams 1 + 2), using slice 0's strict mode.** Replace `unique_visible_type` with an owner-returning consult mirroring `unique_visible_return_owner`: resolve each fact's text against **its own `defining_file`**, return the resolved `GoOwnerIdentity` alongside the raw text, and emit real `GoPartitionEvidence` instead of `Default::default()` (S3 is currently invisible to owner-partition telemetry). Package-variable recovery then carries `owner_identity: Some(..)`. Two facts whose texts are *textually* identical but resolve to different owners must be treated as ambiguous and fail closed — today they collapse into one string and look unique.

**Slice 2 — resolve eagerly where the file is already known (seam 3).** At the post-merge local-binding path, `defining_file == caller_file`, so the owner is derivable with no new plumbing. Resolving there makes `proven_owner.is_none()` mean *genuinely unprovable* rather than *not yet computed*, which is the precondition for slice 3. **Exclude `proof_shadowed` sites (sol r1 W3):** origin identity is not proof that the binding is still live. A live test pins a shadowed `Iterator` with two package owners dropping *because* `owner_identity` is `None`; attaching the first binding's owner flips `on_demand` to false, so the shadow branch stops returning its collision bail and falls through to the legacy bare ladder. Either exclude shadowed sites here, or make shadowing a terminal drop before owner-based routing.

**Slice 3 — fail closed on absent provenance, through ONE shared terminal predicate.** With slices 0–2 landed, a Go receiver reaching resolution without a proven owner drops rather than re-binding against the caller's imports. **The guard must be a single predicate consumed by resolution, manifest generation, and the sidecar (sol r1 W4):** the existing `ReturnTyped` guard lives only in the resolver, while the interface manifest reaches its legacy bare-name lookup independently — so today a site can drop in the resolver and still appear as an interface dispatch in the manifest. Negative tests assert resolver / manifest / sidecar **parity**. **Order matters** — doing this before slices 0–2 would forfeit recall those slices are about to make provable.

*(The former slice 4 has been promoted into slice 0 — its facts are prerequisites, not follow-ups.)*

## 4. Non-goals

No general Go type inference. No change to the shared `resolve_go_owner_identity` signature or its basename fallback for other consumers (a global change needs its own blast-radius review). No new interface-dispatch behaviour — #16 remains parked and becomes a follow-on that consumes proven identities.

## 5. Risks

- **`CallSite` identity (corrected — sol r1 S6; the v1 statement was wrong).** Populating a field that `cmp_key` omits **cannot** merge or split the `BTreeSet`: the ordering key is unchanged, and a source occurrence is already identified by caller/callee/span/qualifier/type. The real hazard is the opposite: **adding** the owner to the key would let `None` and `Some(owner)` revisions of the same occurrence coexist across an overlapping merge, producing duplicate site and resolution counts — and it would not repair the existing `Eq`/`Ord` mismatch, since other derived fields remain excluded either way. **Decision: keep the occurrence key and add a competing-owner invariant, scoped to a single fresh classification epoch (sol r2 W2).** Two different proven owners for one occurrence are an ambiguity that drops — but *only* among candidates produced by the same fresh whole-program classification, evaluated **before** projection into `CallSite`. **Persisted owners are replacement inputs, never competing evidence:** rematerialization computes fresh classifications and then replaces each retained site, so comparing a cached owner against a freshly proven one would wrongly drop the correct new answer when a defining file's import changes (`ext "a.example/api"` → `ext "b.example/api"`) while the caller file stays cached. Applying the invariant only after set merging would instead hide same-key conflicts. Required fixture: an incremental A→B owner change with unchanged raw type and caller, asserting full-build parity. Do not add this field alone; redesigning equality would require reworking all merge/rematerialization logic together.
- **Cache (claim narrowed — sol r1 S8).** `receiver_owner_identity` already exists in the serialized schema and caches are additionally fenced by binary build identity, so `#[serde(default)]` alone does **not** establish that current rows silently degrade. A CPG/sidecar bump is still warranted as a topology-release fence, and the gate below asserts a deserialized site carries its owner — but the design must not claim a defect it has not demonstrated.
- **S3 index keying (sol r1 S8).** The package-var index is keyed by `(dir, name)` with no package clause, so ordinary `p` and external-test `p_test` vars of the same name share a bucket. **Contract (sol r2 S4 — stated, not deferred): a bare package-var reference selects facts whose defining file carries the caller's exact package clause** — an ordinary `p` caller selects `p` facts, an external-test `p_test` caller selects `p_test` facts. **A missing or unparsed build profile yields `uncertain` and drops** (today the consult-time filter *fails open* for such files). Both outcomes are named in the fixture matrix.
- **Recall.** Slice 3 forfeits sites whose provenance is genuinely absent. Size it in the census before enabling, and report the decrement rather than discovering it in the oracle.

## 6. Gates

1. **Per-slice fixture matrix — not one combined list (sol r1 S7).** Each slice ships only fixtures it can actually satisfy; slice 1 cannot satisfy type-parameter or dot-import expectations, which belong to slice 0. Slice 0: same-basename external negative; **predeclared-identifier negative (`func F(e error)` with an unrelated `q.error`) ⇒ no owner, terminal drop, asserted across resolver/manifest/sidecar**; type-parameter receiver ⇒ drop; dot-import bare name lacking a declaration ⇒ drop **plus** the locally-declared `Local.M` positive that must keep minting. Slice 1: the cross-file alias counterexample ⇒ no mint (currently mints); a **positive** S3 case asserting the exact real owner and target (not merely "no wrong mint"); two same-dir facts with textually identical types resolving to different owners **within one classification epoch** ⇒ ambiguous, fail closed; an **incremental A→B owner change** (defining file's import alias re-pointed, caller cached) ⇒ B replaces A, full-build parity; `p` caller selects `p` facts and `p_test` caller selects `p_test` facts; a profile-less file ⇒ uncertain/drop. Slice 2: shadowed-site exclusion preserves the existing collision-bail drop. Slice 3: resolver / manifest / sidecar parity negatives. Throughout: positive controls that S1/S2 behaviour is unchanged.
1b. **Slice 1 is behavioural, not merely plumbing (sol r1 S7):** moving `owner_identity` from `None` to `Some` flips the `on_demand` routing branch, so its gates must include resolver/manifest/cache parity for the populated owner.
2. Same-base 5-corpus control cut at the implementation branch's **actual** base (`mainD`/`mainE` are stale). Deltas confined to receiver-provenance counters plus census-predicted site changes.
3. Oracle join with baselines recut at the same base; report `sound`/`recall_gap`/`over_approx` movement per slice, not just at the end.
4. Site-count parity check for the `cmp_key` question, with the delta explained.
5. Cache version bump plus the four-path cache-parity battery; assert a deserialized site carries its owner.
6. Full suite green, `cargo fmt`, tier-a `--matrix-only` at every wave.

## 7. Sequencing

Slices land in order **0 → 1 → 2 → 3** (sol r2 W3 — the previous text still named the obsolete order and the removed slice 4), each its own PR with its own review round. **The competing-owner/occupancy-key decision belongs to slice 1**, where owners first become populated. **Confirmed viable (sol r2):** the shared terminal predicate of slice 3 has exactly two production consumers — the resolver and the manifest — while the navigation sidecar derives its edges through the resolver. **#16 follow-on:** once owners are proven, #16's bare consult reduces to "consume the proven identity, walk, live-select, arity-filter" with no text rule, no membership proxy, and no fallback — and the wins it was chasing (the 8 over-approx kills, the etcd-24 recovery, caddy's +6 identities) come back into reach without a proxy.

## 8. Open questions

- Is `indirect_call_site` dropping receiver provenance intentional? The source site is in scope but unused for receiver fields.
- (Resolved into the design: the S3 `(dir, name)` keying question is now a slice-1 requirement, §5; the cache-degradation question is narrowed in §5 to a topology fence with an explicit assertion rather than an assumed defect.)

**Producer census (sol r1, confirming scope is bounded):** S1 return facts, S2 field declarations, and S3 package variables are the *only* cross-file receiver-text producers. Typed parameters, constructor locals, local `var`, and type assertions all originate in the caller file and merely lack owners today; `indirect_call_site` produces no receiver text at all. The remaining defects are bounded to the strict resolver, the prerequisite poison/binder facts, shadow validity, and the second manifest consumer — **not another open class.**

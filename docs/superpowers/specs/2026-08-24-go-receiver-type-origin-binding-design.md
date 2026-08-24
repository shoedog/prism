# A — Go receiver type-origin binding (design v1)

**Status:** design, owner-approved 2026-08-24 ("A first"). **Base:** current `main` (post-#190, post-#193; CPG 50, nav sidecar 18). **Supersedes the sequencing of** #16, which is parked (PR #203 §7-PARK) pending this work.

**Citation policy:** cite symbols; line numbers are advisory as of this writing and must be re-verified before use.

## 1. Problem

A Go receiver's *type text* is routinely recovered in one file and then re-interpreted in another file's import namespace. Exactness of the caller's import map proves that an alias maps to one directory; it does **not** prove the recovered text originated in that alias's scope. Constructible in valid Go:

```go
// p/a.go   import ext "external.example/api";  var V ext.I
// p/b.go   import ext "example.com/app/decoy"; func F() { V.M() }
```

`V`'s type text `ext.I` is transported into `b.go`, resolved there, and yields the decoy interface — which then validates and mints wrong targets. This is the defect class that refuted six successive #16 rules (`dispatchable` filter → universe membership → receiver-text qualifier → absence-of-hazards → identity establishment → exact-import-path evidence). **It is not introduced by #16** — with a single decoy declaration it already mints through the existing concrete-receiver route on `main`.

## 2. Key insight — this is plumbing, not type inference

The provenance already exists and the correct pattern is already implemented twice. `GoTypedFact` carries `{ty, defining_file}`; three sibling consults live in `go_receiver_index_visibility.rs`:

| function | resolves against | returns |
|---|---|---|
| `unique_visible_return_owner` (S1) | `fact.defining_file` ✅ | `GoPartitionSelection<GoOwnerIdentity>` |
| `resolve_go_struct_field_owner` (S2) | `declaration.defining_file` ✅ | owner **and** raw text |
| `unique_visible_type` (S3) | reads `defining_file` for the visibility test, then **discards it** ❌ | naked `Option<String>` |

`unique_visible_type` is the only one that returns text. Its single caller — package-variable recovery in `go_receiver_index.rs` — therefore stores `owner_identity: None`, even though `facts.imports`, `facts.package_basenames`, and `facts.go_file_profiles` are all in scope at that exact point. **The owner is not computed because the return type cannot express it.** Downstream, `CallGraph::go_receiver_owner` falls back to re-binding the text against the *caller's* file.

Correspondingly, `ReturnTyped` and `FieldTyped` — the two kinds that *do* carry owners — are hard-dropped when the owner is missing, but **`VarDecl` has no equivalent guard**. That asymmetry is the hole.

## 3. Design — four slices, in order

**Slice 1 — make provenance expressible (seams 1 + 2).** Replace `unique_visible_type` with an owner-returning consult mirroring `unique_visible_return_owner`: resolve each fact's text against **its own `defining_file`**, return the resolved `GoOwnerIdentity` alongside the raw text, and emit real `GoPartitionEvidence` instead of `Default::default()` (S3 is currently invisible to owner-partition telemetry). Package-variable recovery then carries `owner_identity: Some(..)`. Two facts whose texts are *textually* identical but resolve to different owners must be treated as ambiguous and fail closed — today they collapse into one string and look unique.

**Slice 2 — resolve eagerly where the file is already known (seam 3).** At the post-merge local-binding path, `defining_file == caller_file`, so the owner is derivable with no new plumbing. Resolving there makes `proven_owner.is_none()` mean *genuinely unprovable* rather than *not yet computed*, which is the precondition for slice 3.

**Slice 3 — fail closed on absent provenance.** With slices 1–2 landed, extend the existing `ReturnTyped` no-owner hard-drop to the remaining recovery kinds: a Go receiver reaching resolution without a proven owner drops rather than re-binding against the caller's imports. **Order matters** — doing this before slices 1–2 would forfeit recall that those slices are about to make provable.

**Slice 4 — scope holes provenance cannot fix.** These need new facts, not carried ones:
- **Go type parameters have no binder set anywhere.** `func (s *Store[T]) f() { var v T; v.M() }` yields receiver text `T`, which binds to a package-level `T` if one exists — no gate catches it, because `T` contains no `[`. Collect Go type-parameter binders (the existing collector matches tree-sitter-Rust node kinds and is unreachable for Go) and drop receivers bound by one.
- **Dot imports are erased from the import map** with no marker, so a file with `import . "pkg"` is indistinguishable from a file with none. Record the marker and treat any bare receiver in such a file as unprovable.
- **Local type declarations** are already handled as a poison flag; confirm coverage and keep it.

## 4. Non-goals

No general Go type inference. No change to the shared `resolve_go_owner_identity` signature or its basename fallback for other consumers (a global change needs its own blast-radius review). No new interface-dispatch behaviour — #16 remains parked and becomes a follow-on that consumes proven identities.

## 5. Risks

- **`CallSite::cmp_key` includes `receiver_type` but not `receiver_owner_identity`.** Two sites differing only in proven owner currently compare **equal** in the `BTreeSet<CallSite>`. Populating owners can therefore merge or split sites. This must be settled explicitly — either add the owner to the key (and measure the site-count delta) or prove the collision is unreachable. **Treat as a blocking design question for slice 1.**
- **Cache.** `receiver_owner_identity` is `#[serde(default)]`, so old rows deserialize to `None` — silently reintroducing the defect for retained files. A CPG/sidecar version bump is required so stale rows rebuild rather than degrade.
- **Recall.** Slice 3 forfeits sites whose provenance is genuinely absent. Size it in the census before enabling, and report the decrement rather than discovering it in the oracle.

## 6. Gates

1. Red-first fixtures: the cross-file alias counterexample above ⇒ no mint (currently mints); two same-dir facts with textually identical types resolving to different owners ⇒ ambiguous, fail closed; Go type-parameter receiver ⇒ drop; dot-import file bare receiver ⇒ drop; and positive controls that S1/S2 behaviour is unchanged.
2. Same-base 5-corpus control cut at the implementation branch's **actual** base (`mainD`/`mainE` are stale). Deltas confined to receiver-provenance counters plus census-predicted site changes.
3. Oracle join with baselines recut at the same base; report `sound`/`recall_gap`/`over_approx` movement per slice, not just at the end.
4. Site-count parity check for the `cmp_key` question, with the delta explained.
5. Cache version bump plus the four-path cache-parity battery; assert a deserialized site carries its owner.
6. Full suite green, `cargo fmt`, tier-a `--matrix-only` at every wave.

## 7. Sequencing

Slices land in order 1 → 2 → 3 → 4, each its own PR with its own review round; slice 1 carries the `cmp_key` decision. **#16 follow-on:** once owners are proven, #16's bare consult reduces to "consume the proven identity, walk, live-select, arity-filter" with no text rule, no membership proxy, and no fallback — and the wins it was chasing (the 8 over-approx kills, the etcd-24 recovery, caddy's +6 identities) come back into reach without a proxy.

## 8. Open questions carried from the mapping

- Does `receiver_owner_identity` survive the CPG cache round-trip today? (`#[serde(default)]` says old rows degrade to `None`; the cache-bump gate above assumes it must not.)
- Is the S3 `(dir, name)` key collision between `foo` and `foo_test` packages reachable when a file lacks a build profile? The visibility test **fails open** for profile-less files.
- Is `indirect_call_site` dropping receiver provenance intentional? The source site is in scope but unused for receiver fields.

# #16 — Go package-qualified interface identity (design **v12 — B1**)

**Status:** design-of-record. Clean rewrite, owner-approved 2026-08-24 (the v1→v11 document accumulated eleven layered amendment records whose gates contradicted across five sections and whose line citations had drifted wholesale; it was parked twice — see §8). **Base:** current `main` (post-R1(b) #190, post-#193; CPG 50, nav sidecar 18). **Roadmap:** row 16.

**Citation policy (the failure mode this rewrite fixes):** cite **symbols**, not line numbers. Line numbers below are advisory as of this writing and MUST be re-verified before use.

## 1. Problem

The Go bare-interface consult (the `iface_key(recv_ty)` arm in `resolution.rs`, ~2800; mirrored in `navigation/queries.rs`, ~847) resolves through `CallGraph::interface_impls`, a table keyed by **bare interface name** whose satisfaction is computed over bare-collapsed last-writer maps. Same-named interfaces in different packages collapse together, so the consult can mint implementers of an unrelated package's interface.

## 2. Rule (B1) — normative

Owner decision 2026-08-24: **"ship B, then A"**, adopting **B1**. At the bare consult, in order:

1. **Establish identity.** Call `resolve_go_owner_identity(receiver_text, caller_file, …)`. It is total over `Some(identity)` / `None`, and returns `None` for *every* case where the type cannot be placed: generic instantiation `T[X]` (rejected before the bare/qualified split), a qualifier whose import cannot be bound or binds ambiguously, and bare text in a file with no package clause. **`None` ⇒ DROP**, counted `go_bare_identity_unresolved_drop`.
2. **Validate.** The identity must be a key of `go_interface_declarations` — **all recorded identities, regardless of `dispatchable`/`generic`** (the extractor marks real interfaces non-dispatchable when generic or when a method signature is unsupported; see `record_interface_type` in `type_providers/go.rs`). **Absent ⇒ DROP**, counted `go_bare_identity_invalid_drop`.
3. **Walk** (unchanged from the settled C1 design): caller-scoped satisfaction walk over a shared fn extracted from the proven lane (`go_visible_s4_implementers` in `resolution.rs`, `select_interface_signatures_with_mode` in `go_owner_partition_s4.rs`). Satisfier method sets come from two caller-profile-safe sources — **direct** `go_method_declarations`, and **promoted** entries from the R1(b) selector snapshot consulted under exactly its rules (ProfileUnique, single declaration, exact variant coherence, not field-shadowed, plus the `FunctionId` join requiring `!generic && signature.is_some()` and, here additionally, a canonical signature match against the interface method). Ambiguity / `uncertain` / `conflict` ⇒ **DROP**.
4. **Live selection.** Intersect satisfier admission keys with `CallGraph::go_interface_live_types` (`call_graph.rs`, ~681; already serialized). Non-empty ⇒ that subset; empty ⇒ the full step-3 satisfier set (receiver-kind-aware fallback, semantics unchanged).
5. **Arity filter** — the existing shared helper in its existing position; an arity-emptied set routes through `func_value_field_or_external_drop` exactly as today.
6. **Mint** Exact `InterfaceDispatch` from the survivors.

**There is no global-index consult anywhere in this design.** The `bare_name → BTreeSet<GoOwnerIdentity>` index survives **only** as the step-2 membership oracle; it never selects an identity. This is the point of B1: every earlier revision (v7 `dispatchable` filter → v8 universe membership → v9 receiver-text qualifier → v10 absence-of-hazards) tried to make an identity *fallback* safe and was refuted, because each substituted a cheap proxy for what the receiver actually **binds** to, and membership proves a declaration exists, not that the receiver binds to it. B1 removes the fallback rather than approximating it, and is total by construction.

**Validation is diagnostic, not load-bearing** (reviewer-confirmed): an identity absent from `go_interface_declarations` makes the signature walk return `Uncertain` before implementers are consulted, and step 3 drops on uncertainty — so validate-then-drop and walk-then-drop mint identically. Step 2 buys a clean counter and a census key. It must never be described as a correctness mechanism.

## 3. What does not change

The static table build (`compute_interface_dispatch`) still runs untouched: stats/gaps/fanout/`fallback_fired` telemetry views stay byte-stable. Proven (non-bare) resolution arms are untouched and gated by per-site byte comparison. Table retirement is a follow-up once its telemetry views derive elsewhere. The nav manifest arm mirrors §2 via the same shared consult fn so oracle rows mirror resolver mints.

## 4. Slice-0 census (pre-implementation gate; TEMP probe, worktree-only)

Over **every** R3-eligible bare consult attempt, compute today's minted set vs the B1 set, and oracle-join every newly minted set. The probe must share the extracted consult code path (or be byte-diffed against its output on the full attempt set) — a diverging simulation invalidates the gate.

- **Route partition — exhaustive and mutually exclusive:** `mint` | `unresolved_drop` | `invalid_drop` | `walk_drop` | `arity_drop` | `missing`. (B1 has no global index, hence no collision route.)
- **Expected-route oracle.** Expected routes are produced by a **reference classifier that cannot import or call production routing code**, run over the committed v8 keyed census artifact bound **by path and SHA-256** in the census report. The gate asserts exact key-set equality with that artifact plus `actual == expected` per key, and includes a **route-mutation red**: deliberately mislabel one site and the gate must fail. Reported dispositions plus counts are not an oracle.
- **Preservation floor** (implementer-level, per site, not per site-set): caddy `CaddyModule` and `CertMagicStorage` non-test implementer sets must survive implementer-for-implementer — else STOP and escalate. An implementer is excluded from the floor only if **every** file defining part of its method set matches `*_test.go` (Go's intrinsic rule only; build tags never exclude). Exclusions are reported, never silently dropped.
- **Named forfeits, not failures.** B1 drops the etcd-24 `AuthBackend` recovery and the former `unique_index` population. Both are expected; the census records the exact site count and the resulting `sound`/`recall_gap` decrement (see `classify` in `eval/tools/dispatch_oracle.py`, ~159). **All population counts in this document are provisional until the census re-derives them.**
- **Recall-debt inventory.** Inventory every non-minting route whose *correct* binding would have minted (etcd-24 is the known seed). This does not gate B1 — B1 forfeits recall by design — and becomes A's acceptance backlog.
- **Precision residual audit.** Enumerate every mint-bearing site and disposition it `binding-provable` | `binding-unprovable, mint preserved` | `binding-unprovable, mint suspect`. **`binding-provable` requires source-level type-origin evidence — the declaration the receiver actually binds to, cited by file:line. Universe membership is expressly inadmissible: it is the rejected proxy.** Any `mint suspect` is an explicit owner disposition before merge.

## 5. Acceptance gates

1. Census (§4) passed and attached to the PR.
2. Red-first fixtures (§6) all failing before the change.
3. **Same-base 5-corpus control**: cut fresh `mainF` call-stats at the implementation branch's **actual base** (`mainD`/`mainE` are stale — a control must match the branch's base). Deltas confined to bare-arm counters/kinds plus census-predicted site changes; ripgrep byte-identical; static-table telemetry views byte-stable; proven-route manifest rows byte-identical per site.
4. **Oracle delta** vs baselines **recut at the same base**: gate_ok TRUE; census kill-list realized; no transitions outside the census's predicted set.
5. `#17b` audit re-run: `over_approx` census 8 → (8 − killed); the `sound` tally moves by exactly the §4-predicted decrement (it does **not** stay unchanged — B1 forfeits mints by design).
6. **Cache: CPG stays 50** (`CACHE_VERSION`, `cpg_cache.rs` ~153 — this design adds no new serialized state); **nav sidecar 18 → 19** (`NAV_CALL_EDGE_CACHE_VERSION`, `navigation/call_edge_cache.rs` ~55 — resolved outcomes change). Four-path cache-parity battery plus a cache-hit assertion that the deserialized live set is non-empty on Go corpora.
7. Full suite green; `cargo fmt` clean; tier-a `--matrix-only` at every wave.
8. **Controller-owed and previously environment-blocked:** Tier-A and the gopls oracle join, including adjudication of any newly-added qualified identities.

## 6. Red-first fixtures

- **Identity establishment:** generic instantiation `T[X]` ⇒ `unresolved_drop`; qualified receiver whose import cannot be bound ⇒ `unresolved_drop`; bare receiver in a file with no package clause ⇒ `unresolved_drop`. **No global-index consult in any of these** — a mint here means the removed fallback has reappeared under another label.
- **Validation:** phantom identity (constructor in `p` returns qualified `q.I`; caller in `p` dispatches through the constructor-local receiver; `p` declares no `I`) ⇒ `invalid_drop` **and no `q.I` edge even though `q.I` is globally unique** — this pins the forfeited etcd-24 shape so A flips it deliberately; declared-but-non-dispatchable `p.I` alongside an unrelated unique `q.I` ⇒ identity validates, walk fails closed, no `q.I` edge; external receiver (`import "time"`, `t.Stop()` on `*time.Timer`) with an unrelated unique in-repo `debug.Timer` ⇒ drop, no edge (the hugo `warpc.go` regression).
- **Walk:** promoted-satisfier preservation (live `T struct{ Base }` + `Base.M` + live direct `U.M` on `I{M()}` ⇒ both mint); promoted supply fail-closed on ProfileConflict; promoted signature mismatch (`Base.M(int)` vs `I{M(string)}` ⇒ `T` contributes nothing); walk `uncertain`/`conflict` ⇒ drop; unexported-method namespace restriction; embedded-flattened requirements.
- **Live/profile ordering:** Windows-only `W.M` live, Linux-only `L.M` not, Linux caller on `I.M` ⇒ profile filter first, live selection second, fallback to `L`, never `W`.
- **Binding characterization (documents A's scope, does not assert correctness):** generic type-parameter receiver `func f[T interface{ M(); N() }](x T) { x.M() }` with an unrelated recorded `a.T`; a local type declaration shadowing a package-level interface name. Each records the current outcome with a comment that closing it is A's scope, so A's change to these tests is visible in review.

## 7. Delivery

Implementation proceeds directly on current main (R1(b) landed; no serialization needed). Implementer via bridge clone; code review terra ∥ parallel seat; controller runs corpus/oracle batteries and pushes. **Follow-on A** — positive, scope-aware receiver type-origin binding (predeclared identifiers, type parameters, local declarations and shadowing, aliases, carried cross-file identities), resolved *before* validation — is its own design and plan. B1 is shaped so A is additive: A replaces only identity establishment, while the walk, live selection, arity filter, and telemetry here are downstream and unchanged. **A's acceptance target:** recover the sites B1 forfeits (etcd-24 plus the former `unique_index` population), with zero false edges, and clear both §4 inventories.

## 8. History

v1–v3 (static-table shapes) parked at cap for an open profile-witness class. C1 chosen by the owner 2026-08-23; v4–v8 folded successive review rounds and **v8 merged as the then design-of-record (#195)**. Its re-census passed the amended floor on all four corpora but surfaced a stdlib-receiver false edge; v9/v10 tried to make the identity-invalid fallback safe and were **parked as open-class** (#201). v11 removed the fallback but retained contradictory gates and stale citations and was **parked as an artifact defect** (#202), with the review finding one real hole — an unresolvable-identity path that re-entered the global index and could resurrect the very false edge. This v12 adopts B1 and is a clean rewrite; full history is in git and in PRs #195, #201, #202.

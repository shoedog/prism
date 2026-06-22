# Glob Re-Export Member Expansion — Design Spec (2026-06-22)

**Status:** design-of-record (pre codex xhigh review).
**Arc:** prism owner-key-collision precision arc, successor to the cross-crate `use`
resolution slice (PR #124, merged to `main` `e98250f`).
**Memory:** `~/.claude/.../memory/project_prism_owner_key_collision.md` (authoritative arc log).
**Handoff that greenlit this:** `docs/superpowers/handoffs/2026-06-22-glob-expansion-and-roadmap-handoff.md`.

---

## 1. Problem

prism's Rust name-resolution engine does **not** expand glob re-exports. A `pub use mod::*`
is populated as a *deferred-glob poison edge* (`src/name_resolution/rust_populator/walk/items.rs:237`
— `UseItem::Glob` → `add_glob_edge(BindTarget::Pending(path, anchor), …)` with the comment
"no member expansion in Phase 1"). When a queried `(name, ns)` reaches such an edge, the engine
returns `poisoned()` (`src/name_resolution/engine.rs:274`, the
`BindTarget::Pending(_, _) => return GlobOutcome::Poison` arm) — it fails *closed* (never falls
through to a wrong outer same-name) but resolves *nothing*.

This is the dominant residual after #124. Many crates expose their public API through **crate-root
glob re-export facades**: ruff's crate roots carry 29 `pub use …::*` lines; `ruff_python_ast/src/lib.rs`
opens with seven (`pub use nodes::*; pub use expression::*; …`). So `use ruff_python_ast::Stmt`
(where `Stmt` reaches the crate root via `pub use nodes::*`) now resolves the **crate** segment
(`ruff_python_ast` → the crate Root, via #124's `extern_crate_root` fallback) but the **final type
segment** `Stmt` hits the deferred root glob → `Poison` → unresolved.

The two precision buckets are **coupled, not independent** (handoff §4): a cross-crate facade
collision needs the leading crate segment (**#124**, now on `main`) *and* the final type segment
through the glob (**this slice**). Neither alone recovers it. A throwaway spike on a pre-#124 base
measured **+985 `kind_exact` / −910 `unresolved_unknown_name`** on ruff from glob expansion *alone*,
sound (`multi_target_exact_sites` byte-identical), and proved via synthetic probes that the cross-
crate facade collision recovers only with **both** #124 and glob expansion present. The combined
magnitude on a #124 base is the number this slice's acceptance will measure (we are not re-spiking;
acceptance measures the real implementation, which is strictly better than a throwaway hack).

---

## 2. Goal & non-goals

**Goal.** Expand deferred glob re-export edges during name resolution, **bounded to two glob hops**,
so a queried name reachable through a `pub use mod::*` facade resolves to the real item instead of
poisoning — while never minting a wrong full-confidence edge, and while **instrumenting every fail-
closed bail** so the recall left on the table is measured, not guessed.

**Non-goals (YAGNI).**
- **No unbounded glob recursion.** Stop at depth 2; *count* (do not resolve) anything that would need
  a 3rd hop, so a future slice can size the deeper tail from data.
- **No new language coverage.** Rust only (the glob facade pattern lives in `rust_policy`/Rust
  populator). Other languages' glob edges are unaffected — the engine change is generic but only
  Rust populates deferred glob edges today.
- **No re-architecture** of the engine ⇄ policy seam, the `recovery_typepath` classifier, the #124
  cross-crate machinery, or the call-stats outcome buckets. This slice is additive on the existing
  glob seam plus one new telemetry field.
- **No silent disambiguation, ever.** Multiple distinct candidates → not-a-single (fail closed),
  never a pick.

---

## 3. Design

### 3.1 Where the change lives

One engine function changes behavior: `glob_lookup` (`engine.rs:252`). Today its deferred-glob arm
(`:274`) returns `Poison` immediately. The change makes that arm **expand the glob one hop** (resolve
the glob's target scope, then look the queried name up there), recursing through the existing
`scope_member_lookup_probed` so a target that is itself a facade is followed for a second hop — all
under a depth bound and an extended cycle guard, with each glob edge first gated by a policy
visibility check on its own `vis` (§3.4). The success and every fail-closed reason increment a
process-global, per-measurement `glob_stats` counter (§3.5).

`GlobOutcome` (`engine.rs:243`) keeps its three variants (`Poison` / `Hit(Vec<Candidate>)` / `Empty`)
unchanged — a bail still returns `Poison` (fail closed); a clean expansion accumulates into the
existing `candidates` vector and returns `Hit`, so `policy.combine` at the call site
(`engine.rs:134`, `:441`) handles cross-glob distinct-target ambiguity exactly as it does today.

### 3.2 The expansion algorithm (deferred-glob arm of `glob_lookup`)

For each glob edge `e` in the scope (the existing loop, `engine.rs:261`):

- **Edge-visibility gate (step 0 — BLOCKER-fix).** Before an edge may contribute *or* poison, gate it
  by a new **tri-state** `policy.glob_edge_visible(e, q, trav)` hook (§3.4) → `Visible` / `Hidden` /
  `Unknown`:
  - `Visible` (e.g. a `pub use m::*`, or a private `use m::*` queried from *within* its module
    subtree) → proceed to expand.
  - `Hidden` (a *known* visibility that provably does not reach the query origin — e.g. a private
    `use m::*` queried from *outside* its subtree) → **skip** (`continue`); for that vantage it brings
    nothing, so falling past it is sound.
  - `Unknown` (a visibility the policy cannot decide — e.g. `pub(in path)` whose restrict scope is not
    resolved; `resolve_restrict` is a Phase-1 stub returning `None`, `walk/mod.rs:271`, and the
    populator currently discards the `pub(in)` path, `items.rs:192`) → **`Poison`** +
    `glob_stats::vis_unknown()`. Skipping an `Unknown` edge would let the name fall through to a wrong
    outer same-name (the glob *might* be visible-and-containing) — so it poisons, never skips.
  (Today the only edge gate is the byte-range `vis_range`, `engine.rs:265`; the edge's `vis` was never
  consulted because a deferred glob poisoned before exposing anything.)

- **Resolved-scope glob edge** (`engine.rs:275`, the existing arm): unchanged. The Rust populator
  emits **only** deferred `Pending` glob edges (items.rs:241), so this arm is not exercised by Rust
  today; the change below lives entirely in the deferred arm.

- **Deferred `Pending(path, anchor)` glob edge** (the changed arm): in this exact order, **fail closed
  on any doubt**. The glob-edge guard is held **across the whole expansion** — both the target-path
  resolution AND the member lookup — via an RAII scope-guard, so every early return leaves exactly
  once and the depth/cycle state stays live for the recursion (BLOCKER-fix: the rev-1 "leave
  immediately after path resolution" let a facade chain exceed depth 2 and `a::* ↔ b::*` recurse):
  1. **Depth gate.** If `guard.glob_depth() >= MAX_GLOB_DEPTH` (= 2): `glob_stats::depth_exceeded()`,
     `return GlobOutcome::Poison`. (An unexpanded glob *might* contain the name; we cannot prove
     absence, so poison — never fall through.)
  2. **Cycle gate.** `if !guard.enter_glob(edge_idx)` (this glob edge already active on the chain →
     mutually-recursive / self glob): `glob_stats::cycle()`, `return Poison`. `enter_glob` records
     `edge_idx` in a glob-edge-keyed set **and** increments `glob_depth`; the RAII guard's `Drop`
     runs `leave_glob` (removes + decrements) on every exit path.
  3. **Resolve the glob's target scope** via `resolve_path_guarded(graph, path, NS_TYPE, anchor,
     from = scope_id, prefix_ns = policy.namespaces().first(), &q.at, policy, guard)` — the same
     anchored resolver the named-`Pending` arm uses (`engine.rs:206`), so `crate::`/`super::`/`self::`/
     multi-segment glob paths (`pub use a::b::*`) and #124's leading-crate fallback all work. The guard
     stays held. Classify the target:
     - **Exactly one in-repo `Target::Scope(T)`** (a single `Resolved` scope candidate with cond `tc`)**:**
       look the queried name up in `T` via `scope_member_lookup_probed(graph, T, q, policy, guard)`
       (it sees `glob_depth + 1`). On its result:
       - `Resolved` with **exactly one** candidate `mc` → contribute it, conjoining conditions
         (MAJOR-fix, mirroring `:217`/`:291`): push `Candidate { target: mc.target, cond:
         conjoin(cond_of(&e.cond), conjoin(tc, mc.cond)), .. }`, where `tc` is the target-path
         candidate's cond. `glob_stats::resolved(glob_depth)`. **Caveat:** `resolve_path_guarded`
         drops *prefix*-segment conds when advancing scopes (`engine.rs:383`, `scope = s`), so for a
         cfg-gated prefix in a multi-segment glob path (`#[cfg(x)] mod a; pub use a::b::*`) `tc`
         carries only the final segment's cond — exactly as the existing named-import chase does. A
         pre-existing engine limitation (precision, not a wrong-single; rare for re-export paths); the
         proper fix (accumulate prefix conds, benefiting named imports too) is deferred (§9).
       - `ResolvedSet`, `Ambiguous`, or any non-single (MAJOR-fix — `scope_member_lookup_probed` can
         return `ResolvedSet` for cfg-exclusive worlds via `combine`, `rust_policy.rs:207`–`:211`):
         not-a-single → `glob_stats::ambiguous()`, `return Poison`. (Conservative: a cfg-exclusive set
         is a legitimate multi-world resolution that *could* be propagated — a measured recall
         follow-on, §9 — but the first slice fails closed.)
       - `Poisoned` → `return Poison` (a deeper over-depth/cyclic/external glob inside `T` already
         poisoned and counted its own bucket; propagate, no new count).
       - `Unresolved` → **contribute nothing, continue** to the next glob edge. The recall-enabling
         case: `T` *provably* lacks the name (no rib, and all of `T`'s own globs resolved to nothing —
         a deeper *unresolvable* glob would have returned `Poisoned`, not `Unresolved`, verified at
         `engine.rs:439`–`:442`), so this facade is irrelevant to the query — not a poison.
     - **>1 scope / `Ambiguous` target:** `glob_stats::multi_target()`, `return Poison`.
     - **`Unresolved` / external / non-scope / `Poisoned` target:** `glob_stats::external()`,
       `return Poison`. (We could not establish what the glob brings in; it *might* contain the name.)

After the loop (`engine.rs:306`): `!saw_glob || candidates.is_empty()` → `Empty`; else
`Hit(candidates)`. The caller `combine`s (`engine.rs:134`): a re-export **diamond** (two glob edges to
the *same* item) is **deduped to a single `Resolved`** (`rust_policy.rs:177`–`:188`); two glob edges to
*distinct* targets become `ResolvedSet` (pairwise cfg-exclusive) or `Ambiguous` (compatible-cfg
conflict) — never a silent pick (MAJOR-fix: the rev-1 "always `Ambiguous`" claim was too strong).

**Cardinal invariant preserved (`engine.rs:11` §7):** every bail returns `Poison` (fail closed); the
only `→ Resolved` contribution is a *single clean visible in-repo member hit* in a *single cleanly-
resolved visible target scope*. The worst outcome is a missed edge, never a wrong one. A scope poisons
the whole lookup the instant any *visible* glob edge cannot be fully resolved to a single (or proven to
lack the name), because an unresolved glob could hide the real target and invalidate an otherwise-
single answer.

### 3.3 Bounding & termination

- **Depth bound `MAX_GLOB_DEPTH = 2`** (a named `const` in `engine.rs`): at most two deferred-glob
  expansions on any resolution chain. Re-counting: outer call `glob_depth = 0` → expand (→ 1) →
  inner glob at 1 → expand (→ 2) → inner glob at 2 → `2 >= 2` → `depth_exceeded`, no 3rd expansion.
  So levels 1 and 2 resolve; level 3+ is counted and blocked.
- **Cycle guard, glob-edge keyed.** `CycleGuard` today tracks `graph.bindings` indices for the
  `Pending`-import fixpoint (`engine.rs:75`, `enter`/`leave` on a `BTreeSet<usize>`). Glob edges are
  `graph.edges`, **not** bindings — mixing edge indices into the binding set would collide and could
  miss a `a::* ↔ b::*` cycle. Add a **second** set `active_globs: BTreeSet<usize>` keyed by glob-edge
  index, with `enter_glob`/`leave_glob` (which also adjust `glob_depth`). The depth bound alone
  already forces termination (a 2-cycle hits `depth_exceeded` at the 3rd hop); the edge-keyed guard
  exists to **attribute a true cycle to the `cycle` bucket** (accurate telemetry) and to stay correct
  if `MAX_GLOB_DEPTH` is ever raised. Cycle check is **after** the depth check, so a depth-blocked
  chain is `depth_exceeded`, and a genuine re-entry within budget is `cycle`. Concretely: a
  **self-glob** (`pub use self::*`, or an edge whose target re-enters the same edge at depth 1) trips
  `cycle` (re-entry before the cap); a 2-cycle `a::* ↔ b::*` instead trips `depth_exceeded` at the 3rd
  hop (the depth check fires first). Both terminate; the §6 `cycle` test uses a self-glob accordingly.

### 3.4 Visibility & soundness reuse

The expansion reuses the engine's existing visibility discipline and adds **one** new check — the
glob edge's own visibility (BLOCKER-fix):
- **Glob-edge visibility (new, tri-state — BLOCKER-fix).** `glob_lookup` today gates an edge only by
  its byte-range `vis_range` (`engine.rs:265`); it never checks the edge's `vis: Vis` (`types.rs:427`,
  populated from the `use`/`pub use` visibility at `items.rs:191`,`:239`) — safe only because deferred
  globs poisoned before exposing members. Once expanded, a private `use m::*` must **not** behave like
  a public re-export. Add `ResolutionPolicy::glob_edge_visible(&self, edge, q, trav) -> GlobEdgeVis`
  returning **`Visible` / `Hidden` / `Unknown`** (default `Visible` for non-Rust policies). The Rust
  impl reuses a shared `vis_reaches(vis, def_scope, from) -> Option<bool>` helper (factored out of
  `visible()`, `rust_policy.rs:220`–`:268`) applied to `edge.vis.kind` with `edge.from` as the defining
  scope: `Some(true)` ⇒ `Visible`, `Some(false)` ⇒ `Hidden`, `None` ⇒ `Unknown`. Crucially `visible()`
  *folds* "unknown" into `false` (fine for a rib, which fails closed by *not contributing*), but a glob
  edge must distinguish them: a `pub(in path)` whose `restrict` is unresolved (`resolve_restrict` stub
  → `None`, `walk/mod.rs:271`; the `pub(in)` path is also discarded by the populator, `items.rs:192`)
  is **`Unknown` → poison** (§3.2), NOT skipped. Recovering `pub(in)`-glob recall (populate the
  restrict) is deferred (§9).
- **Member visibility (unchanged).** The member lookup goes through `scope_member_lookup_probed →
  resolve_rib`, enforcing `policy.visible(...)` per member from `from` (the query origin). A `pub use`
  does not launder *member* privacy, mirroring the named-`Pending` chase (`engine.rs:195`–`:228`).
- **Prefix visibility (unchanged).** `resolve_path_guarded` judges each glob-path prefix segment from
  `from`, so a sibling-private module on the glob's path falls through.
- With these gates, no candidate is produced that the named-import path could not also produce — this
  slice only removes the blanket "deferred ⇒ poison" short-circuit, now gated (edge-vis + depth +
  cycle) and counted.

### 3.5 Telemetry — process-global `glob_stats` (per-measurement)

Call-site resolution runs under **rayon** (`call_graph.rs:472`,`:557` → `resolve_call_site` →
`resolve_path`), so counters must be thread-safe; and the per-bail reason is *internal* to
`glob_lookup`, not recoverable from the call-stats outcome classifier (`queries.rs:102`,`:156`) —
genuinely new plumbing.

**Mechanism (revised per codex review).** The rev-1 design (a `GlobExpandStats` field on `CallGraph`,
carried by reference on the guard, with the param added to the public engine entries) was rejected for
three reasons:
1. `CallGraph` derives `Clone`/`Serialize`/`Deserialize` (`call_graph.rs:144`); `AtomicUsize` fields
   are not trivially any of those.
2. The public `resolve`/`resolve_path` entries are shared by many callers (`consumer.rs:85`,`:123`,
   `:166`; `resolution_identity.rs:70`,`:80`; `resolution_receiver.rs:397`,`:414`; tests), so a
   required `stats` param ripples far beyond the two `resolution.rs` sites.
3. A cumulative atomic on `CallGraph` is **not a clean per-measurement number** — `resolve_call_site`
   runs during the CPG build (`cpg/build.rs:653`–`:666`) and every consumer, so the count would depend
   on prior calls and repeated `call-stats` invocations.

Use a **process-global, per-measurement counter** instead (codex's offered alternative):

- A `glob_stats` module (`src/name_resolution/glob_stats.rs`) holds a `static` set of `AtomicUsize`
  buckets — `resolved_l1`, `resolved_l2`, `depth_exceeded`, `cycle`, `external`, `multi_target`,
  `ambiguous`, `vis_unknown` — with per-bucket increment helpers (e.g. `glob_stats::depth_exceeded()`,
  `glob_stats::resolved(depth)`; Relaxed), `reset()`, and `snapshot() -> GlobExpandSnapshot`:

```text
resolved_l1   name resolved via 1 glob hop          cycle         mutually-recursive / self glob edge
resolved_l2   name resolved via 2 glob hops         external      glob target unresolvable/external
depth_exceeded would need a 3rd+ hop (blocked)      multi_target  glob path resolves to >1 scope
ambiguous     non-single member (incl. ResolvedSet) in the single target
vis_unknown   glob edge with undecidable visibility (e.g. unresolved pub(in)) → poison
```

- `glob_lookup` records via a sink resolved as `guard.stats.unwrap_or(&GLOBAL)`: production leaves
  `guard.stats = None` so it writes the process-global static; a `#[cfg(test)]` engine entry can
  inject a **local** `&GlobExpandStats` for isolated assertions (see Test isolation). **No signature
  change to the public `resolve`/`resolve_path` and no `CallGraph` field** — they construct the guard
  with `stats: None`; the only engine-signature change is threading `&mut CycleGuard` into `glob_lookup`
  (call sites `engine.rs:132`,`:439`), needed anyway for the recursion/depth/cycle. `CycleGuard` gains
  an `Option<&GlobExpandStats>` (lifetime), defaulted to `None` by the public entries (no caller
  ripple).
- `call_stats` (`queries.rs:156`) calls `glob_stats::reset()` at entry, runs the existing re-resolution
  loop (`:198`–`:201`, which resolves every site — discarding any build-time counts), then reads
  `glob_stats::snapshot()` after the loop and emits a top-level JSON object:

```json
"glob_expand": { "resolved_l1": N, "resolved_l2": N, "depth_exceeded": N, "cycle": N,
                 "external": N, "multi_target": N, "ambiguous": N, "vis_unknown": N }
```

Reset-at-entry + snapshot-after-the-loop is a clean per-`call-stats` measurement even though the global
is shared: `call-stats` runs serially after the build, and the loop's parallel increments are atomic
and joined before the snapshot. (A target *member* that is itself an unresolvable `Pending` import
surfaces as a `Poisoned` member handled by the propagate-`Poison` arm in §3.2 — pre-existing poison,
not a glob-expansion bail, so no bucket.)

The `resolved_l*` counters are *expansion-event* counts (a single clean member hit at that depth), not
final-edge counts — the realized edge buy is read from `kind_exact` / `unresolved_unknown_name`. The
fail-closed buckets are the slice's decision data: they size how much recall each closed door leaves on
the table (driving whether a follow-up raises the depth bound, propagates cfg-exclusive sets, or
attacks `external`).

**Test isolation.** The unit tests do NOT rely on the process-global. A `#[cfg(test)]` engine entry
(e.g. `resolve_path_with_stats(.., stats: &GlobExpandStats)`) constructs the guard with
`stats: Some(&local)`, so each test asserts exact bucket counts on its OWN `GlobExpandStats` instance —
fully isolated, parallel-safe, no shared-counter races and no lock. Production (`call_stats`) leaves
`stats = None` and uses the reset-at-entry global.

### 3.6 Cache

This changes resolution behavior → **`CACHE_VERSION` 18 → 19** (`src/cpg_cache.rs:63`) and update the
version assertion test (`cpg_cache.rs:568`–`:570`). No serialized schema field is added (the stats are
runtime-only, never cached), but the resolved edges differ, so the bump is required. The spike relied
on `--no-cache`; production must invalidate.

---

## 4. Files touched

| File | Change |
|------|--------|
| `src/name_resolution/engine.rs` | `glob_lookup`: expand the deferred-glob arm (tri-state edge-vis gate, `Unknown` → `vis_unknown` poison; RAII glob-guard held across path-resolution **and** member-lookup; depth/cycle gates; conjoin edge+path+member conds; `ResolvedSet`/`Ambiguous` member → poison; `glob_stats` buckets via `guard.stats.unwrap_or(&GLOBAL)`); gains a `&mut CycleGuard` param (call sites `:132`,`:439`). `CycleGuard`: add `glob_depth`, `active_globs`, `enter_glob`/`leave_glob` + an RAII drop-guard + an `Option<&GlobExpandStats>` sink (defaulted `None` by the public entries). Add `MAX_GLOB_DEPTH` const. **`resolve`/`resolve_path` signatures UNCHANGED.** |
| `src/name_resolution/glob_stats.rs` **(new)** | process-global `AtomicUsize` buckets + per-bucket increment helpers / `reset()` / `snapshot()`. |
| `src/name_resolution/types.rs` | add `ResolutionPolicy::glob_edge_visible(&self, edge, q, trav) -> GlobEdgeVis` (tri-state `Visible`/`Hidden`/`Unknown`, default `Visible`). |
| `src/name_resolution/rust_policy.rs` | implement `glob_edge_visible`; extract a shared `vis_reaches(vis, def_scope, from) -> Option<bool>` helper used by it **and** `visible()`. |
| `src/navigation/queries.rs` | `call_stats`: `glob_stats::reset()` at entry, `snapshot()` after the re-resolution loop, emit the `glob_expand` JSON object. |
| `src/cpg_cache.rs` | `CACHE_VERSION` 18 → 19; update the version assertion test (`:568`–`:570`). |
| `tests/name_resolution/` | New unit tests (each bucket incl. `vis_unknown`, depth levels, edge-vis skip, `ResolvedSet`, cond preservation, diamond) + a glob-member-workspace fixture; bucket assertions go through the `#[cfg(test)]` local-sink engine entry (own `GlobExpandStats`, no `TEST_LOCK`). |
| `tests/integration/` | e2e: cross-crate facade collision recovers to one Exact (depends on #124 + this). |

No change to `call_graph.rs`, `resolution.rs` (the telemetry rework keeps the shared engine entries
untouched), the `recovery_typepath` classifier, `multi_target_exact_sites` counting, or the #124
`crate_deps_by_root` machinery.

---

## 5. Soundness argument (the review bar)

1. **Never a wrong single.** The only `Unresolved → Resolved` contribution is a member lookup that
   returns *exactly one* visible in-repo candidate in a *single* cleanly-resolved *visible* target
   scope, with conditions preserved. Every other case (multi-target, `ResolvedSet`/`Ambiguous`/multi
   member, external, cycle, over-depth) returns `Poison`; an invisible edge is skipped. Cross-glob
   candidates fold through the existing `policy.combine` — a diamond (same target) dedups to one
   `Resolved`; distinct targets become `ResolvedSet` (cfg-exclusive) or `Ambiguous`, never a silent
   pick. ⇒ `multi_target_exact_sites` must stay **byte-identical** before/after on ruff and prism
   (the canary; any increase = a wrong Exact = BLOCKER).
2. **No fall-through past a glob.** Today a deferred glob poisons so the engine never skips it to an
   outer same-name (§7). The expansion preserves that: a *visible* glob that *might* contain the name
   but cannot be fully resolved still poisons (the depth/cycle gates, the `external`/`multi_target`
   arms, and the `ResolvedSet`/`Ambiguous`/`Poisoned` member arms). The lookup only proceeds past a
   glob when it is invisible for this vantage (brings nothing) or *provably* lacks the name (the
   `Unresolved` member arm — a deeper *unresolvable* glob returns `Poisoned`, not `Unresolved`).
3. **Termination.** `MAX_GLOB_DEPTH` bounds expansions per chain; `active_globs` rejects re-entry; the
   RAII guard holds across path-resolution + member-lookup and leaves exactly once on every exit.
   Depth and cycle guards independently terminate; together they classify cycles vs over-depth.
4. **Visibility.** Member and prefix visibility flow through the existing `policy.visible` /
   `resolve_path_guarded` vantage logic; the glob **edge** is additionally gated by the new
   `glob_edge_visible` hook, which applies the *same* rule (`vis_reaches`) to `edge.vis` (§3.4). No
   privacy laundering and no new privacy *rule* — one new *application* of the existing rule.
5. **Determinism.** Resolution order is unchanged; the only new state is per-chain guard fields and
   process-global atomic counters (commutative; reset per measurement). Cached output differs only by
   the now-resolved edges (hence the `CACHE_VERSION` bump). Counter totals are order-independent.

---

## 6. Testing strategy

TDD, unit-first. Every test uses a synthetic `ScopeGraph` (or a fixture crate) exercising one
behavior. Discriminating fixtures only (a test that also passes with the *old* poison behavior is not
a test of this slice). The bucket-asserting tests use the `#[cfg(test)]` local-sink engine entry
(§3.5), asserting exact counts on their own `GlobExpandStats` instance — parallel-safe, no
`TEST_LOCK`.

**Resolution unit tests (`tests/name_resolution/`):**
- `glob_expand_single_hop_resolves` — `mod m { pub struct S; } pub use m::*;` query `S` at root →
  resolves to `m::S` (was `Poison`). Asserts `resolved_l1 == 1`.
- `glob_expand_two_hops_resolves` — root `pub use a::*`; `a` has `pub use b::*`; `b` defines `S` →
  resolves at depth 2. Asserts `resolved_l2 == 1`.
- `glob_expand_third_hop_blocked` — three nested facades; query the depth-3 name → `Poison`,
  `depth_exceeded == 1`, edge **not** resolved (the measure-don't-resolve requirement).
- `glob_expand_cycle_fails_closed` — a **self-glob** (`pub use self::*`, or an edge whose target
  re-enters the same edge at depth 1) with no real definition of the name → `Poison`, `cycle >= 1`.
  (A 2-cycle `a::* ↔ b::*` instead trips `depth_exceeded` — covered by `glob_expand_third_hop_blocked`;
  add an explicit assertion there that an `a ↔ b` cycle increments `depth_exceeded`, not `cycle`.)
- `glob_expand_ambiguous_member_fails_closed` — target scope defines the name twice under
  *compatible* cfg (a genuine conflict → `Ambiguous`) → `Poison`, `ambiguous == 1`, no Exact.
- `glob_expand_resolved_set_member_fails_closed` — target defines the name twice under *cfg-exclusive*
  worlds (`combine` → `ResolvedSet`) → `Poison`, `ambiguous == 1` (the conservative not-a-single rule;
  the deferred-propagation follow-on, §9). Distinguishes `ResolvedSet` handling from a true conflict.
- `glob_expand_external_target_fails_closed` — glob path resolves outside the repo / unresolvable →
  `Poison`, `external == 1`.
- `glob_expand_multi_target_fails_closed` — the globbed module path is itself ambiguous → `Poison`,
  `multi_target == 1`.
- `glob_expand_target_lacks_name_continues` — **two glob edges** in one scope: the first target lacks
  the name, the second provides it → resolves to the second (the `Unresolved` member arm on the first
  glob must *continue* to the second, not poison). NOTE: must use two GLOB edges, not a sibling
  non-glob binding — a same-scope explicit binding is found by rib step 1 (`engine.rs:101`) and would
  bypass `glob_lookup` entirely, so it would not exercise this arm. Guards against over-poisoning.
- `glob_expand_respects_member_visibility` (**discriminating** — MINOR-fix) — under one `pub use m::*`,
  `m` has a `pub` `S` and a private `Hidden`. Querying `S` from outside resolves (`resolved_l1 == 1`);
  querying `Hidden` from outside does **not** (stays unresolved, never the private item). A paired
  must-resolve + must-not-resolve so the test fails if expansion is broken either way.
- `glob_expand_skips_private_glob_edge` (**BLOCKER-fix**) — a **private** `use m::*` (non-`pub`) edge
  in module `mid`, with `m::S` public. A query for `S` from *outside* `mid`'s subtree must NOT resolve
  through the private glob (edge-visibility gate skips it); a query for `S` from *within* `mid` does
  resolve. Asserts the edge-vis hook, not just member visibility.
- `glob_expand_pub_in_unknown_fails_closed` (**BLOCKER-fix**) — a `pub(in some::path) use m::*` edge
  (restrict unresolved → `Unknown` visibility) → `Poison` + `vis_unknown == 1` from BOTH an inside and
  an outside vantage; never skipped, never fallen-through to an outer same-name.
- `glob_expand_preserves_conditions` — a `#[cfg(feature="x")]`-gated `pub use m::*` whose `m::S`
  resolves → the resulting candidate carries the conjoined `e.cond ∧ member cond` (MAJOR-fix); a
  cfg-incompatible query does not select it.
- `glob_expand_diamond_resolves_single` — two glob paths to the **same** `S` (a re-export diamond) →
  `combine` dedups → a single `Resolved` (not `Ambiguous`); no Exact inflation.
- `glob_expand_distinct_targets_two_globs` — two glob edges in one scope yielding *different* `S` →
  `combine` → `ResolvedSet` (cfg-exclusive) or `Ambiguous` (compatible cfg) — never a silent pick; no
  wrong Exact.

**Workspace / cross-crate fixture (the §8/handoff lesson — always add a glob-workspace fixture):**
- `tests/name_resolution/build_wiring_test.rs` (or a new file): a glob-member workspace
  (`members = ["crates/*"]` with concrete `crates/foo`, `crates/bar`) where crate `bar` does
  `use foo::SomeType` and `foo`'s root re-exports `SomeType` via `pub use inner::*`. Resolves only
  with #124 (leading `foo`) **and** this slice (final `SomeType` through the glob) — the coupled
  case. Confirms the prerequisite wiring.

**e2e (`tests/integration/`):** cross-crate facade collision (two crates each defining a same-named
type behind a facade) recovers to one Exact for the *dependent* crate and stays dropped for the
non-dependent one (depends on #124's per-crate dep gate + this expansion).

**Acceptance (host, post-implementation):**
- `cargo fmt --check`; `cargo test --lib`; **`cargo test --test name_resolution`** (the seam that the
  §3 CI regression proved must not be skipped); `cargo test --test integration`.
- `cargo build --release` then call-stats deltas vs a `main` worktree on **ruff** and **prism**:
  report the full breakdown — `kind_exact` (per-kind), `unresolved_unknown_name`, `dropped_multi_owner`,
  `recovery_typepath.*`, and **`multi_target_exact_sites` (must be flat)** — plus the new
  `glob_expand` bucket histogram. This is the combined-base measurement the spike could not get.
- Tier-A: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (0 regressions), and ruff M2
  `uv run tier-a --corpus ruff --allow-stale-sut` (`baseline_invalid=false`, 0 regressions — the buy
  lands outside the adjudicated sample, so M2 shows no-harm, not scored gains).

---

## 7. Acceptance criteria

- All new unit tests + the workspace fixture + the e2e pass; full `--test name_resolution` green.
- `multi_target_exact_sites` byte-identical pre/post on ruff **and** prism (hard gate).
- `kind_exact` increases / `unresolved_unknown_name` decreases on ruff (the general buy; compare to
  the spike's pre-#124 +985/−910 — report whether it held, shrank, or grew on the #124 base).
- The `glob_expand` histogram is populated and self-consistent (`resolved_l1 + resolved_l2 > 0` on
  ruff; the fail-closed buckets sized for the roadmap).
- Tier-A `--matrix-only` and ruff M2 both 0-regression.
- codex xhigh final diff review: SHIP (sound, recall-safe, faithful).

---

## 8. Resolved design decisions (codex review folded — rev 3)

The rev-1 codex xhigh review (CHANGES-REQUIRED, 2 BLOCKER + 4 MAJOR + 1 MINOR) is folded:

- **Telemetry carrier → process-global, per-measurement `glob_stats`** (§3.5), reversing rev-1's
  guard-carried `CallGraph`-field sink. codex's three telemetry findings — `CallGraph` derives
  `Serialize`/`Clone`; the public `resolve`/`resolve_path` entries are shared by many callers (ripple);
  a cumulative `CallGraph` atomic is not a clean per-measurement number — make the global the cleaner
  choice. It also keeps `resolve`/`resolve_path` and `CallGraph` untouched. (Test isolation was
  revised again in the rev-2 re-review — see the local-sink bullet below.)
- **Glob-edge visibility** → new `glob_edge_visible` policy hook gating each edge by its own `vis`
  (§3.4, BLOCKER-fix).
- **Guard lifetime** → RAII glob-guard held across path-resolution + member-lookup, single leave on
  every exit (§3.2, BLOCKER-fix).
- **`ResolvedSet` member** → not-a-single → poison + `ambiguous` bucket (conservative; propagation
  deferred, §9, MAJOR-fix).
- **Condition preservation** → conjoin edge ∧ path ∧ member conds (§3.2, MAJOR-fix).
- **Discriminating visibility tests** → paired must-resolve/must-not + a private-glob-edge test
  (§6, MINOR + BLOCKER-fix).
- **Glob-edge visibility is tri-state** (rev-2 re-review BLOCKER): `Visible`/`Hidden`/`Unknown`; an
  undecidable `pub(in)` edge poisons (`vis_unknown`), never skips (§3.2, §3.4).
- **Prefix-segment conditions** are inherited-as-dropped from `resolve_path_guarded` (same as named
  imports); the contributed cond preserves edge ∧ final-segment ∧ member (§3.2); the accumulate-prefix
  fix is deferred (§9, rev-2 re-review MAJOR).
- **`cycle` test uses a self-glob**; a 2-cycle is `depth_exceeded` (§3.3, §6, rev-2 re-review MAJOR).
- **Telemetry test isolation** via a `#[cfg(test)]` local-sink entry + guard-carried `Option` sink
  (not a global `TEST_LOCK`), keeping the public entries ripple-free (§3.5, rev-2 re-review MAJOR).
- **`target_lacks_name` test uses two glob edges** (a sibling rib would bypass `glob_lookup`) (§6,
  rev-2 re-review MINOR).

**Remaining judgment for the plan/impl reviewer.** The `glob_depth` counter is shared with the glob's
own target-PATH resolution, so an exotic multi-segment `pub use a::b::*` whose prefix itself traverses
a glob consumes budget toward the depth-2 cap. The spec takes the conservative single counter (sound;
at worst slightly under-resolves a glob-in-prefix chain, which `depth_exceeded` surfaces) rather than a
separate path-vs-name depth. Confirm this is acceptable or split the counter.

---

## 9. Deferred / follow-ons

- **Glob depth ≥ 3.** Sized by `glob_expand.depth_exceeded` after this lands; raise `MAX_GLOB_DEPTH`
  only if the data justifies it.
- **Propagate cfg-exclusive `ResolvedSet` through globs.** The first slice poisons a `ResolvedSet`
  member (counted under `ambiguous`). If that bucket is large on ruff, a follow-on can propagate the
  cfg-exclusive candidates (each conjoined) the way the resolved-scope arm already does — sound, more
  recall, no wrong single.
- **Accumulate prefix-segment conditions in `resolve_path_guarded`.** Today it drops non-final
  segment conds (`engine.rs:383`); a cfg-gated prefix module in a re-export/glob path resolves
  under-conditioned. Fixing it benefits the named-import chase too. Deferred (precision, rare).
- **Populate `pub(in path)` restrict for `use`/glob edges** (`resolve_restrict`, `walk/mod.rs:271`;
  keep the parsed `pub(in)` path in `items.rs:192`) to recover `pub(in)`-glob recall that this slice
  currently poisons as `vis_unknown`.
- **Attacking a specific fail-closed bucket** (`external` cross-crate facades, `ambiguous`) — sized by
  the new histogram; likely diminishing per §4's facade-mediation lesson.
- **Other-language glob edges** — only Rust populates deferred glob edges today; revisit if another
  language's populator starts emitting them.
- **Re-characterize the ruff residue** after this lands (the buckets will shift; remaining same-name
  collisions are likely correct-keep-all shadows / poison / downstream-method — diminishing returns).

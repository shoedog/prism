# Phase-IP spec review — claude opus (SOUNDNESS lens) — 2026-06-14

Operator subagent (read-only, prism nav). Artifact:
`docs/superpowers/specs/2026-06-14-prism-phase-ip-type-confirmed-dispatch-design.md` (draft).
Codex rigor-lens companion: `phase-ip-spec-review-codex-2026-06-14.md`.

## Claim verification

**Claim 1 — Build order forces CallGraph-internal dispatch: CONFIRMED.** `build_impl`
(src/cpg/build.rs:134-137) does `cg = CallGraph::build(files)` then `assemble_graph`, whose Step 5
(build.rs:349-379) materializes Call/Return edges via `cg.resolve_call_site(site)`. The
`TypeRegistry` is built only later in `CpgContext::build` (src/cpg/context.rs:63-65). The sole
resolution entry point `resolve_call_site_full` (src/resolution.rs:247) takes only `&self,
&CallSite` — no registry. No second pass; no path where the registry is reachable at resolution
time. Inference correct.

**Claim 2 — The resolution seam + multi-callee compose: CONFIRMED.** src/resolution.rs:404-424 is
the P6-lite block; on `owner_lookup` miss it returns `dropped(ExternalReceiver)` at line 422 — the
exact insertion point. `exact(...)` (resolution.rs:167-178) takes `impl IntoIterator<Item=&FunctionId>`
and already returns N callees (used for R1 trait dual-key, resolution.rs:240). Composes with Step-5
(build.rs:362 loops `for resolved in cg.resolve_call_site(site)` → one edge per callee) and nav
re-resolution (call_resolve.rs:15-26 maps every callee to a `NavCallEdge`).

**Claim 3 — Embedding == existing trait dual-key: CONFIRMED.** call_graph.rs:61-70. Aliasing
`(S,m)→Base::Ping` while leaving `method_owners[fid]=Base` is the same mechanism; existing seam
returns the promoted hit with no change.

**Claim 4 — Empty-live-set RTA fallback is load-bearing: CONFIRMED (keystone holds).** (a)
interface_dispatch/main.go has no composite literal — only `type Fast struct{}` + `func (f Fast)
Go()`. (b) live_types.rs `scan_go_node` (158-172) only catches `composite_literal`. (c) Step-9 CHA
prunes only when live set non-empty (build.rs:559). Global live set = ∅ → without the "keep all"
fallback `Fast` is pruned and the fixture can't flip. *(embedded_method also has ∅ live set, but
embedding is unconditional / not RTA-pruned — spec correctly scopes the fallback to interface only.)*

**Claim 5 — Nav bypasses CPG edges: CONFIRMED.** `nav callers`→`direct_callers` (queries.rs:247-262)
and `nav callees`→`direct_callees` (queries.rs:362-389) re-resolve via `resolve_call_site`
(call_resolve.rs:16); neither touches `cpg.graph` (only `ego_graph` reads the petgraph,
queries.rs:692). A CPG-only post-pass cannot move nav recall.

**Claim 6 — No CACHE_VERSION bump needed: CONFIRMED.** `CACHE_VERSION=7` + `git_sha` in key
(cpg_cache.rs:47,60-63; mismatch→rebuild 297-302). `CallGraph` fields already `#[serde(default)]`
(call_graph.rs:63-70). Nav cache delegates to `cpg_cache::load_cache` (navigation/cache.rs:45). There
is a `wrong_git_sha_misses` test naming Phase-IP as the use case (cpg_cache.rs:693-695). `CpgEdge`
unchanged. GIT_SHA invalidation alone is safe.

## Findings (prioritized)

**[MAJOR] §6 — unbounded N-way interface fan-out as Exact under name+arity-only satisfaction is the
decision most likely to erode EFT's precision win, and the guardrail is under-specified.** EFT's win
(`target-c-method` P 0.208→1.00, baseline.md:36) came from `ExactOnly` excluding unconfirmed same-name
edges from barrier/vertical/threed/spiral BFS (barrier_slice.rs:90,139; query.rs:408-409). Phase-IP
injects N type-confirmed `Call(Exact)` edges into that same BFS. The may-analysis argument (§6) is
correct, but satisfaction is name+arity only (§5.3), not signature — on a popular method (`Close()`,
`Read(p []byte)`) name+arity admits every same-named/same-arity type as a "satisfier," and with a
coarse/empty live set (the §5 fallback) the Exact fan-out balloons. caddy U-method callers precision
is already 0.81 with collision FPs on unique names (baseline.md:35); a barrier slice seeded at a
popular interface method could pull in dozens of Exact-confidence false peers — and `ExactOnly`, the
filter meant to keep them out, admits them because they're labeled Exact. Invisible to the matrix
(fixtures are 1-2 implementers); only caught in the human-triggered caddy rerun. Fix: state an explicit
ExactOnly fan-out interaction + a containment lever (cap/parameterize fan-out, or demote to NameOnly
above a fan-out threshold, or signature matching for interfaces with >K implementers) and add a
*multi-implementer barrier-slice precision* assertion to §10.

**[MINOR] §2 (line ~79) — present-tense factual error.** "`CallGraph::build` calls this free function
[`collect_live_types`] directly (it predates the registry)" is false today: `collect_live_types` is
called only from context.rs:65,89,110,170,192 + type_provider.rs:303, never from `CallGraph::build`.
The task (§7) is to *add* `live_types::collect_live_types(files,&∅)` inside `build_interface_impls`,
which works (free `pub fn` over files, no registry dep, live_types.rs:29). Fix: reword to future tense
("can call it"), matching §7.

**[MINOR] §7 — overstates maintenance surface + misframes confidence.** (a) The only exhaustive
`ResolutionKind` match is `as_str()` (resolution.rs:36-54); reason/stat helpers delegate to it, so
adding 2 variants + 2 `as_str()` arms is the whole surface (histogram auto-updates). (b) Confidence is
set at construction in `exact()`/`demoted()` (resolution.rs:174,187), not via a kind→confidence match
— so "map both → Exact" rides `exact()` for free. Fix: drop the separate mapping step.

**[MINOR] §7 — incremental `build_direct_subset` computes dispatch over the FRESH SUBSET, but
embedding/satisfaction is a WHOLE-PROGRAM relation.** If the subset has `run(r Runner){r.Go()}` but the
file defining `Fast` is not in `only_files`, subset `build_interface_impls` won't see `Fast` → edge
missed. build_incremental (build.rs:166) merges cached+fresh CG; `interface_impls` must be computed
over the MERGED graph, not the fresh subset, or diff-scoped reviews silently miss interface/embedded
edges a full build resolves (path-dependent results). Fix: compute over the merged/whole indexed set,
or document incremental Go dispatch as best-effort (mirroring build.rs:160-165 indirect-call caveat).

**[MINOR] §5/§12 — qualified interface names (`io.Writer`, `pkg.Reader`) unaddressed.** P6-lite recovers
the syntactic param type; `owner_key` (resolution.rs:75-84) strips `::` segments but not `.`-qualified
Go package prefixes. If interface decls parse under bare `Runner` but the receiver arrives `io.Reader`,
`interface_impls.get(&(recv_ty,name))` misses → cross-package interface receivers (common in caddy, the
recall target) silently won't resolve. Fix: normalize the key on both sides (same package-prefix
stripping); add a qualified-interface fixture to §10.

**[MINOR] §4 — pointer vs value receiver method-set union propagates into satisfaction-as-Exact.** Go's
rule is asymmetric (`*T` set ⊇ `T` set). The §4 union means a value type `T` may be judged to satisfy an
interface it does not (a pointer-receiver method got counted) → another Exact FP source. Same family as
the MAJOR, different axis. Fix: acknowledge in §5 and fold into the caddy precision gate.

## Soundness assessment

- **Decomposition / home — SOUND.** CallGraph-internal dispatch is forced by build order (Claim 1) and
  is the only home flowing through the one ladder nav+CPG+metric consult (Claims 2,5). No materially
  simpler design hits the goal; the rejected alternatives provably fail (Claims 5,1).
- **Confidence=Exact — sound in principle, risky in practice.** Single-implementer is a clean win; the
  risk is the unbounded multi-implementer fan-out under name+arity feeding ExactOnly (the MAJOR). The
  architecture is right; the guardrail is missing.
- **Hidden coupling — RTA computed twice** (CallGraph::build with `&∅` vs registry with real
  `known_classes`, context.rs:65) over the same files — duplicated scan + drift hazard (the two pass
  different `known_classes` legitimately for Python; a maintainer may not realize the resolver uses ∅).
  Worth a §7 note.
- **Risk — most likely regretted:** (1) Exact for unbounded fan-out under name+arity (MAJOR) — bites in
  the caddy rerun after the matrix says green. (2) Dispatch over the fresh subset in `build_direct_subset`
  (incremental MINOR) — path-dependent diff-review results.
- **Scope / absence checks.** Nothing obviously cuttable. MUST-know items the spec omits: (a) incremental
  whole-program-vs-subset hazard; (b) qualified interface key normalization; (c) explicit ExactOnly
  fan-out interaction + multi-implementer precision gate. (a),(b) surface as silent misses on caddy — the
  exact corpus the work targets.

**Verdict: sound to plan** — home, seam, and all six load-bearing claims hold (Claim 4, the keystone, is
correct); resolve the MAJOR (bound/guard the multi-implementer Exact fan-out + add a multi-implementer
precision gate) and the two silent-miss absence checks (incremental whole-program scope; qualified
interface keys) before building.

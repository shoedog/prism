# Phase-IP spec review — ROUND 3 (deep full-spec analysis) — claude opus — 2026-06-15

Operator subagent, MAXIMUM-EFFORT exhaustive pass (read all 689 lines of `src/type_providers/go.rs`
+ resolution.rs, call_graph.rs, cpg/build.rs, cpg/context.rs, cpg/query.rs, live_types.rs,
navigation/{queries,call_resolve}.rs, cpg_cache.rs, navigation/cache.rs, ast.rs receiver/constructor
paths, the EFT spec, both round-2 records, the Tier-A baseline + re-anchor adjudication, the eval
harness, and the two Go fixtures). Codex round-3 companion: `phase-ip-spec-review3-codex-2026-06-15.md`.

## PHASE A — round-2 fold verification

**A1. §6 `canon_type`/`canon_sig` — PARTIALLY RESOLVED.** Recursive grammar now covers the node kinds
the extractors enumerate (go.rs:369-371); channel direction, `interface{}`≡`any`, grouping,
single/zero returns, generics/type-sets + anon→recorded-gap all specified + byte-equality tested
(§13.3). Residual: (a) **variadic** (`variadic_parameter_declaration`) is never defined in the
grammar — a naive recursion makes `f(int)` falsely satisfy `f(...int)` or diverge across the two
walks; add `...→"..."+canon_type(inner)` + a fixture. (b) **named multiple results** `(n int, err
error)` vs `(int, error)`: §6 invokes group-expansion only for params; state the identical
name-drop+group-expansion for the result `parameter_list` or named-return methods canonicalize
asymmetrically.

**A2. §7 three-key encoding — RESOLVED.** Invariant stated precisely (admission `{T,*T}` for
satisfaction map+live set+intersection; method-body bare `T` after admission). FunctionId-identity
verified: `extract_method` row+1 (go.rs:399-400) == `node_line_range` row+1 (ast.rs:2640) for the same
`method_declaration` node; `data.methods` bare-keyed (go.rs:425) so `*T` never reaches the FunctionId
lookup (go.rs:660-672). Producers/consumers cannot diverge if the invariant holds. (Recovered-receiver
encoding relative to the alphabet → A6.)

**A3. §8 comprehensive `scan_go` — PARTIALLY RESOLVED.** `T{}`/`&T{}`/`new(T)`/`var x T` sources are
implementable against scan_go (live_types.rs:158-173); residual-gap honesty correct. **Factory rule is
not resolved and the citation is wrong:** `constructor_type` (ast.rs:3899-3932) recognizes call-site
`NewX()`/`X{}` to type a *local* (invoked from short_var_declaration ast.rs:3868), it does NOT scan
func defs/result types. The §8 factory-return rule is genuinely new analysis and under-specified
("constructs it" = literal? transitive? any return of T?) — each choice changes the caddy live set.
Fix: either drop factory-return to a recorded gap (fallback covers recall per owner decision) and
delete the citation, OR define it precisely + note it's new code.

**A4. RTA fallback "kept Exact, receiver-kind-aware-empty" — RESOLVED as a contradiction-fix.** §3/§5/
§12 fire the fallback only when no admission key of any kind is live (current code go.rs:646-657 is the
contradictory version → real required change). Closes codex r2-B1. BUT the precision risk of keeping
the fallback Exact is moved entirely onto the §14 gate → raises A5 to load-bearing.

**A5. §14 precision gate — UNRESOLVED (the blocking cluster, see B1).** Three defects: (1) the "57
caddy sites" are `x.(Module).CaddyModule()` **type-assertion** dispatch (re-anchor-adjudication
:84, baseline.md:61) — P6-lite recovers only typed-params + constructor-locals, NOT type assertions,
so Phase-IP mints zero edges there and the gate is **vacuously true**; (2) the harness has no
interface-dispatch attribution primitive (`CallEdge` model.py:36-42, `Adjudication` adjudication.py:27-42
carry no dispatch-kind; verdicts/strata have no interface category); (3) "57" exists in no committed
artifact except a reviewer count of `ambiguous` diffs. The success metric (caddy recall lift) may
barely move because caddy's dominant interface shape is the out-of-scope type assertion.

**A6. one `normalize_go_key` + replace-not-merge — PARTIALLY RESOLVED.** Replace-not-merge correct
(merge call_graph.rs:753-770 omits the new maps; remove_files prunes by fid.file only :736-740 →
stale-alias hazard real; §11 clear+recompute from full merged files closes it; build_incremental gets
full files cpg/build.rs:170). Residual: (a) **recovered-receiver normalization point** — the receiver
is normalized at *extraction* by recover_receiver (peel_type+owner_key, call_graph.rs:1263) and
serialized into `CallSite.receiver_type`; neither strips `pkg.`/`[…]` (resolution.rs:75-84,88-120).
Spec must pin: Go extraction switches to `normalize_go_key` (changes serialized form → cache concern)
OR the seam re-normalizes the already-owner_key'd string. (b) **build_scoped** builds the CallGraph via
`build_enriched(&filtered)` (context.rs:167) → Go dispatch over the subset unless full `files` is
threaded in (B6).

## PHASE B — new findings

### BLOCKER

**B1. §14 acceptance does not prove the success metric; the gate is mis-targeted and unmeasurable.**
The whole "keep-fallback-Exact" decision (A4) depends on this gate, and (per A5) it targets out-of-scope
type-assertion sites, the harness can't attribute interface FPs, and "57" is ungrounded. Phase-IP can
ship a wide-fan-out Exact interface edge into the four ExactOnly slices (barrier_slice.rs:84-109,
threed/vertical/spiral verified) and the gate passes vacuously. **Fix:** redefine the gate over
in-scope (typed-param/constructor-local) interface sites + specify the harness attribution primitive
(surface `ResolutionKind::InterfaceDispatch` on the SUT `CallEdge` so accounting can filter), OR demote
the fallback to NameOnly (removes the exposure, makes the gate non-load-bearing). Also: consider whether
P6-lite should recover the **type-assertion** receiver (`x.(Module)` syntactically names the type) —
that is where caddy's interface recall actually lives, and it's currently out of scope.

**B2. Cache safety rests on bincode-error→Miss + GIT_SHA, not on `#[serde(default)]`.** CallGraph is
bincode-serialized whole inside SerializedCpg (cpg_cache.rs:89,183,251). bincode is non-self-describing
and does NOT honor `#[serde(default)]` for missing trailing fields — a pre-IP blob deserialized into the
new struct errors→`CacheResult::Miss` (cpg_cache.rs:252) *before* the git_sha check (:297), or worse
mis-reads trailing bytes. Safety is real but comes from Miss+GIT_SHA, not serde defaults (the spec's
stated mechanism is wrong for bincode). Same-sha dirty iteration (comments at cpg_cache.rs:295 rely on
`--no-cache`) could serve a garbage/empty-map CallGraph. §13.10 only tests within-version round-trip.
**Fix:** bump `CACHE_VERSION` (one line, removes the GIT_SHA dependency for format safety) OR document
the real mechanism + mandatory `--no-cache` for same-sha dirty iteration; add a cross-version test.

### MAJOR

**B3. Interface fan-out multiplies interprocedural DataFlow edges (Step 5b), not just Call edges.**
build.rs:382-485 binds each call's args to EACH resolved callee's params as `CpgEdge::DataFlow`. N
satisfiers → N× arg→param DFG edges into taint/chop/delta (which use `All` confidence). Unstated,
uncapped, no telemetry (§10 width telemetry is Call-only). Wide `error`-class interface → taint
over-propagation + DFG blow-up on factory-heavy Go. **Fix:** decide whether satisfiers participate in
arg→param binding (a wide may-edge set may be what taint wants — but make it a decision) + extend
telemetry/caps or document as accepted.

**B4. "Step-9 CHA no shared code path" is imprecise.** Step 9 is type_db-gated (cpg/build.rs:526, never
runs pure-Go ✓), but in a **mixed Go+C/C++ repo** the CHA seed scan (build.rs:544-551) reads ALL
function nodes' `Call(Exact)` edges — including newly minted Go interface edges — and matches callee
names against C++ `virtual_method_nodes`. Isolation is by cross-language name-collision improbability,
not construction. **Fix:** correct the claim, or gate the seed-scan callee match by owner language.

**B5. §4.2 embedded-pointer-kind retention is under-scoped.** `GoStruct.embedded: Vec<String>` holds
*stripped* names (go.rs:291), read bare-keyed by `collect_promoted_methods_from` (go.rs:539-548:
`data.structs.get(embedded_name)`, `data.methods.get(embedded_name)`). "Stop stripping at go.rs:291"
(the spec's literal citation) would corrupt those bare lookups. Retaining kind requires changing the
`embedded` field type (carry kind *alongside* the bare name) + every reader. **Fix:** specify the
`GoStruct.embedded` type change + enumerate readers; keep lookups bare-keyed, consult kind only in
satisfaction/selector rules.

**B6. `build_scoped` full-repo Go-dispatch wiring is impossible as-implied.** build_scoped builds the
CallGraph via `build_enriched(&filtered)` (context.rs:167); `CallGraph::build` (where §10 consumes the
provider) sees only the file map it's called with. The full `files` exists in build_scoped's scope but
isn't passed down. The obvious implementation yields scoped Go dispatch (silent recall holes for
diff-scoped Go reviews). **Fix:** specify build_scoped constructs Go dispatch from full `files` and
injects it (new entrypoint or `populate_go_dispatch(&full_files)` post-step) + a §13 test that scoped
== full for in-scope functions.

### MINOR
- **B7** §10 "call-stats auto-updates" conflates the kind histogram (does auto-update via `as_str`)
  with **fan-out width** (genuinely new accumulator in call_stats, queries.rs:12-47).
- **B8** Both committed fixtures are single-implementer/single-file; §13.2b/c/d, §13.5/6/8 need NEW
  fixtures. §13.8 (multi-implementer barrier precision) is the ONLY test exercising wide fan-out into
  an ExactOnly slice — make it a gating fixture, not optional.
- **B9** Confirmed: `ResolutionKind` not Serialize/Deserialize (resolution.rs:16-17); one exhaustive
  `as_str` match (resolution.rs:36); call_stats consumes `as_str` → new kinds auto-surface. Spec accurate.
- **B10** §6 cross-pkg over-approx (`io.Reader`≡`bufio.Reader`) is correctly noted but compounds B1 (lands
  as unmeasured Exact-FP while the gate is vacuous). Fixing B1 covers it.

### Absence checks
- The signature-confirmed `compute_satisfaction` rewrite changes the **registered** provider's
  `subtypes_of` behavior (context.rs:253); no in-tree consumer today (unwired), safe — state it so a
  future registered consumer isn't surprised.
- No edge dedup (EFT F11; build.rs:362-377): N satisfiers → N parallel Call edges, deduped at traversal
  by node (query.rs:444 visited). Correct by construction — note it so no one "fixes" it and breaks
  FunctionId identity.

## Verdict
**needs changes** — BLOCKER cluster B1/A5 (the §14 gate targets out-of-scope type-assertion sites, "57"
is ungrounded, harness can't attribute interface FPs → the gate the keep-fallback-Exact decision rests
on is vacuous) + B2 (bincode ≠ serde-default; safety mis-described). §6/§7 (A1/A2) + replace-not-merge
(A6) substantially resolved; MAJORs (A3 factory citation, A6 normalization point, B3 DFG fan-out, B4 CHA
seed scan, B5 embedded-kind scope, B6 build_scoped wiring) are concrete edits, no redesign. Fix B1+B2,
tighten the rest → ready to plan.

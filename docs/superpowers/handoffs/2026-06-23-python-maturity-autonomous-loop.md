# Python Maturity — Autonomous Loop Handoff (2026-06-23)

**Live continuation tracker** for the overnight autonomous loop. Updated at each milestone (after each
spec, each plan, slice midpoint, slice end). If the session compacts or the owner returns, THIS is the
source of truth for loop state. Pairs with memory `[[project_prism_measurement_maturity]]`.

## Mandate (owner, 2026-06-23, owner asleep)
Loop to **complete**: the **decorated double-capture** slice (in flight) + **1b** (inheritance MRO) +
**2** (typed receivers) + **3** (import-scoping/free_multi). **Sub-slices allowed.** **Authorized to open
PRs after the review pipeline settles, and merge when CI passes (may merge before coverage settles).**
No owner questions while asleep — a genuine design fork gets a best-judgment call + a flag here for morning,
never a block. One stuck slice gets parked (documented here), not allowed to block the rest.

## Pipeline per slice (the loop)
spec → codex spec-review (xhigh) → **fold to sound** (re-review until no BLOCKER/MAJOR) → `writing-plans`
→ codex plan-review → **fold to sound** → **codex-implement** (effort=high, workspace-write; it CANNOT
write `.git` → orchestrator commits per-task after verifying) → **verify** (`cargo test` + `cargo fmt
--check` + acceptance) → codex diff-review (xhigh) → **fold to sound** → **open PR** → **merge on green CI**
(rebase-merge; coverage may be unsettled) → sync main → next slice off fresh main.

**Acceptance per slice (the gates):** the per-corpus buy (call-stats bucket rises) + canary
`multi_target_exact_sites` byte-flat + **Rust/Go (ripgrep, caddy) call-stats byte-identical** (owner
accepts this in lieu of `--quick`) + Tier-A `--matrix-only` 0-regr + suite green. Build both binaries via a
git WORKTREE (never swap the binary mid-measurement).

**Standing constraints:** explicit `git add <paths>` (never `-a`); NEVER stage `eval/` or `docs/eval/`;
commit trailer `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`; PR body ends
`🤖 Generated with [Claude Code](https://claude.com/claude-code)`; verify codex's work before committing
(its output is not safety-classified). See `[[feedback_workflow_preferences]]`.

## Slice status

| Slice | Stage | Branch / artifacts |
|---|---|---|
| **1a** self same-class | **MERGED** (#131, rebase, main `184208a`) | — |
| **decorated** double-capture | **✅ MERGED (#132, rebase, main `2b4641a`)** — all CI green incl. Coverage | — |

**Decorated acceptance (deco vs current-main, both have #131):** pydantic `self_receiver` +79 / `qualifier_owner`
+20 / `free_single` +178 Exact (precision buy); large correct **duplicate-edge collapse** (decorated body
double-scan removed: `local_def` 9825→5304, `free_multi` 25293→13579, `unresolved` 35687→30914 DOWN =
no recall loss; `total_call_sites` unchanged = byte-deduped sites); `multi_target_exact_sites` 439→316 DOWN
(dup free-fn Exact collapsed). **Rust(ripgrep)/Go(caddy) byte-identical; Tier-A matrix 40 ok; suite
2470/2563 pass; fmt clean.** Codex caught+fixed a real blast-radius bug: decorated DFG is wrapper-canonical
but `enclosing_function()` returns inner → taint seed identity mismatch broke decorated Flask taint (a
pre-existing sanitizer test failed) → fixed in `synthesize_target_seed_paths` + regression. **Diff-review
SHIP-WITH-FIXES (no BLOCKER):** folded MAJOR-2 (`return_value_nodes` recursion fence by node identity not
kind — `f874812`); **DEFERRED MAJOR-1 = pre-existing GENERAL nested-function/decorator-call ownership** (a
function's byte-range call/DFG scan attributes a nested `def`'s calls + the `@decorator` expr to the
enclosing fn — VERIFIED on main for an undecorated `outer`/`inner`, so NOT introduced here; needs a
cross-cutting "belongs-to-this-body" predicate across `function_calls_*`/DFG/callees = its own slice; PR
#132 body documents it). **NEXT after #132 merges: slice 2 (typed receivers) off merged main.**
| **2** typed receivers | **🅿️ SHELVED `22deb40`** (re-review REWORK; pre-declared "last fix cycle"). Branch preserved (local, unpushed). +18 fully recoverable. **⚠️ OWNER DECISION FLAGGED below.** | `slice2-typed-receivers` (wt `/tmp/prism-slice2`, tip `22deb40`) |

**Slice-2 ACCEPTANCE (branch `22deb40` vs main `08f019d`, `--no-cache call-stats`, `/tmp/slice2-accept/`):**
**Soundness gate — Rust/Go/JS BYTE-IDENTICAL:** ripgrep (Rust), caddy (Go), express + excalidraw (JS) all `diff -q` empty ✓ (the `call_start_byte` byte-scan did NOT perturb Rust/Go; JS/TS fully inert post-narrowing).
**Python buy:** fastapi `constructor_local +1` / `typed_param +1` (+2); pydantic `constructor_local 28→43` / `typed_param 286→287` (+16) = **~+18 Exact total**. **Canary `multi_target_exact_sites` FLAT** (fastapi 10→10, pydantic 316→316) — no wrong-singleton FP. **`dropped_external_receiver` FLAT** (0→0, 1228→1228) — no recall loss. All other Exact buckets unchanged. Precision-neutral + small sound recall buy. (~+18 matches the strategic finding; owner may reconsider/prioritize slice 3 in the morning — slice 3 is the actual cross-module lever and is next regardless.)

### ⚠️ Slice-2 SHELVE decision + OWNER DECISION (autonomous, owner asleep — `/tmp/slice2-rereview-out.md`)
The codex xhigh re-review (`bkeo4ttai`) returned **VERDICT: REWORK** — 2 BLOCKERs + 1 MINOR:
- **BLOCKER A** (`resolution.rs:371`, `ast.rs:4114`, `resolution.rs:1052`): an **untyped** Python local shadow of an import — `import api` + `api = other(); api.m()` — increments the binding counter but recovers **no type** (`other()` lowercase → `constructor_type`=None → `found`=None), so `classify_simple_ident` returns `none()` (NOT materialized) → R3 still mints false `ImportQualified` Exact. Same for `class api` → R3b `QualifierOwner`.
- **BLOCKER B** (`ast.rs:470`, `ast.rs:4114`): non-`assignment` Python binders (`for api in …`, `with … as api`, walrus, lambda params, comprehension targets) are **not walked at all** → never increment the counter → never materialize → import-shadow leaks the same way.
- **MINOR** (`ast.rs:4022`): the `call_start_byte` byte-scan is **not** Python-gated, so Rust/Go aren't *theoretically* byte-identical (a same-line `x.m(); let x = …` case main counted, branch prunes — actually a correctness improvement).

**VERIFIED pre-existing, NOT a slice-2 regression:** on fastapi/pydantic the acceptance buckets `import_qualified` (16→16, 5636→5636), `qualifier_owner` (2→2, 20→20), `qualified_owner` (816→816) are **FLAT** — slice 2 mints **zero** new R3/R3b Exacts. Main has the identical false Exacts (main has no Python receiver recovery), so these BLOCKERs are pre-existing prism precision limits the slice's *type-driven* materialization only half-closes (typed shadows suppress R3/R3b; untyped/loop-bound shadows leak). The MINOR is empirically inert (ripgrep/caddy byte-identical).

**Why SHELVED (not merged, not fixed-now):** (1) pre-declared guardrail "Last fix cycle — shelve if another BLOCKER"; (2) buy is marginal **+18 Exact**; (3) completing the mechanism is a soundness-risk rabbit hole (enumerating for/with/walrus/lambda/comprehension/except-as binders + an attribute/subscript tail codex flagged) — a new sub-slice, not a tweak; (4) **slice 3 is the actual cross-module lever** and is next regardless; (5) mandate explicitly allows parking a stuck slice.

**⚠️ OWNER, pick one in the morning** (branch `slice2-typed-receivers`@`22deb40` preserved, all reversible):
- **(a) Accept shelve** (default; slice 3 carries the value) — recommended.
- **(b) Merge as-is** — it's a clean no-regression +18 (canary/buckets FLAT, Rust/Go/JS byte-identical); the BLOCKERs are pre-existing and don't manifest on the corpora. Just say "merge slice 2 as-is" and I'll push+PR+merge.
- **(c) Complete the mechanism properly** as sub-slice 2b: **binding-PRESENCE suppression** (not type-driven). The fix is principled and the key data is *already there* — `receiver_type_in_fn` already counts `bindings`; map `bindings>=1 && found=None` → `materialized_only()` (closes BLOCKER A in ~5 lines). BLOCKER B needs the for/with/walrus/lambda/comprehension binders added to the walk to increment the counter (no type recovery needed — presence is enough). This makes materialization binding-driven (syntactic, decidable) instead of type-driven, and is strictly more correct. Estimated 1 spec→impl→review cycle.

**Slice-2 diff-review (REWORK) — the fix IMPROVES the value story:** the 2 BLOCKERs are PRE-EXISTING
false-Exacts (verified on main: `def run(x: Foo): x.m()` + a `class x` → false `qualifier_owner` to
`class x.m`; an import-shadowing typed param → false `import_qualified`) — NOT slice-2 regressions. Root: a
*materialized* receiver binding must suppress R3/R3b **even when the type is poisoned** (import/wildcard),
mirroring Rust `rust_recv_materialized`. The fix (in flight) suppresses R3/R3b for any recovered Python/JS/TS
receiver binding → **closes these pre-existing false-Exacts** + the +17 Exact buy → slice 2 becomes a
precision+soundness win (not just +17). Plus scope-aware recovery (call start byte + skip nested class
bodies). Then re-review → PR → merge.

**⚠️ Slice 2 STRATEGIC FINDING (read this):** sound (Rust/Go byte-identical, `dropped_external_receiver`
FLAT 1228→1228, canary flat, Tier-A 40 ok, suite 2486/2579) **but the realized buy is ~+17 Exact total**
(pydantic `constructor_local` +15 / `typed_param` +1; fastapi +1/+1) — NOT the ~700 owner-hit headline. The
soundness guard (skip imported + wildcard-file types) removes ~98% because **Python typed receivers are
overwhelmingly IMPORTED types** — which are **slice 3/4's cross-module/import territory**, not slice 2's
same-module-local recovery. **Decision (autonomous, mandate = complete 2 + merge-on-green): completing it**
(sound, +17, sets up the receiver-typing infra slice 3/4 extends), flagged here — owner may reconsider /
revert in the morning, OR prioritize slice 3 (the actual lever) which is next anyway. 5 review rounds
(3 spec + 2 plan REWORKs) hardened the guard (collision→wildcard→order) + R3b pre-emption + ordering.
| **3** import-scoping/free_multi | **🅿️ PARKED at rev3 (4th REWORK = hard-stop honored)** — sound under prism's existing bounded-static contract EXCEPT fixable items; needs **1-line owner ratification** (⚠️ below). Branch+designs preserved. | branch `slice3-import-binding-rung` (wt `/tmp/prism-slice3`, rev3 `77c0aff`) |

### ⚠️⚠️ CONSOLIDATED OWNER DECISION — the Python-maturity slices converge on ONE question (read this first)
The overnight loop rigorously attempted slices 2/3/1b. **Slice 3 (rev3) hit, after 4 design reviews, a fundamental boundary** (`/tmp/slice3-rev3review-out.md`): a SOUND imported-member rung must prove the call name isn't dynamically rebound, but `from x import *` (with `__all__`) and `globals()[…]`/`exec` rebind with **no textual occurrence**, defeating any syntactic proof. **THE REFRAME (decisive):** prism's EXISTING rungs (R3 import-qualified, R4 local, R5 free, R6 receiver) **already** resolve names without disproving `globals()`/`exec` rebinding — prism's resolver contract already embeds a **bounded-static assumption** (same as pyright/mypy/all IDEs). So the question is NOT "can we make it sound" (we can't disprove `globals()`, and neither does any existing prism rung) but **"do we hold the new Python import/inheritance rungs to prism's EXISTING bounded-static bar, or a stricter one?"** This single decision gates slice 3 AND slice 1b (base-class resolution shares the boundary).
- **OWNER: ratify "hold to prism's existing bounded-static contract"** → I build **rev4** = rev3 + (a) **wildcard-poison** (`from x import *` in caller-file → all Named imports ineligible; in target module → all `module_bindings` Ambiguous — wildcard is syntactically detectable, cheap, closes B1/B2's only *detectable* hole), (b) **relative-imports-FIRST** (exact normalized sibling/package paths, not ends-with suffix → closes B4; absolute-import source-roots deferred/fall-open), (c) **`indexed_files` authority flag** (thread all-file-paths + `module_resolution_authoritative` bool; disable R4c on scoped/incomplete builds → closes MAJOR). `globals()`/`exec` stays out of scope = prism's standing assumption. Buy est low-hundreds (sound subset).
- **OWNER: decline (require stricter-than-existing soundness)** → sound Python import-member + inherited-self resolution is NOT achievable syntactically; drop slices 3 + 1b (the receiver-typing maturity story ends at what shipped: 1a self-same-class + decorated). 
- Either way: **slice 2's proper fix (sub-slice 2b binding-presence) shares the same bounded-static framing** — its untyped-shadow "holes" are also just prism's standing assumption.

### Slice-3 rev3 robust design (folding rev2 re-review `/tmp/slice3-rereview-out.md`)
3 REWORKs converged on: syntactic soundness over Python's dynamic binding is a FRAGILE enumeration (params→+nested→+local-import→+loop/with/except/match/comprehension→+del/global…), and the canary is BLIND to a wrong `import_member` so a missed form = silent wrong Exact. **rev3 switches to COMPLETE-by-construction occurrence rules (no form-enumeration):**
- **Eligibility = "import-pure"**: a Named import is eligible iff EVERY occurrence of the name in the caller FILE is either the single module-scope import binding or a call-function-position. Any other occurrence (assign-LHS/param/for/with/except target/nested def/2nd import) → fail open. Complete (any binder mentions the name textually). **Drops `CallSite.name_shadowed`** → 3 structures, cleaner-inert 3a.
- **`module_bindings` FuncDef** only if the name's SOLE top-level binding is that def; any other top-level occurrence → `Ambiguous`.
- **`resolve_module_to_file`**: suffix-match across ALL indexed files (`…/a/b/c.py` or `…/__init__.py`), unique-or-`None` (closes source-root B4).

### ⚠️ Slice-3 spec-review REWORK — the slice is BIGGER than "a thin rung" (autonomous decision: fold+decompose, NOT shelve)
Root cause across all 5 BLOCKERs: **prism's `functions`/`FunctionId` inventory carries NO provenance** (no export table, binding-kind, top-level-vs-nested, scope/order). A sound import-member rung needs that model built. BLOCKERs: (B1) file-level import map ignores caller-scope shadowing — `def run(f): f()` param or later `from external import f` → wrong Exact; (B2) "free" includes **nested** fns / JS private locals → false singleton; (B3) `def Foo`+`class Foo` in one module → rung Exacts to the fn when the class was imported; (B4) module-path ambiguity (`mod.js` vs `mod.ts`, `pkg.py` vs `pkg/__init__.py`) filtered too late; (B5) `member.unwrap_or(local)` leaks default/namespace/CommonJS into the named rung. **META (critical): the `multi_target_exact_sites` canary CANNOT catch a single wrong `import_member` Exact** (counts only ≥2-Exact sites) — acceptance is BLIND to this rung's soundness, so the DESIGN must be sound (same lesson as slice 2 / #127).
**Why fold+decompose (not shelve like slice 2):** slice 3 is THE cross-module value lever (not marginal), owner explicitly wants it + authorized sub-slices, and the foundation (module-binding table + indexed-file set) is **reusable by slice 1b** (cross-file base classes) + a future imported-receiver-type slice. The review was **prescriptive** (handed the sound design). **rev2 = Python-first, conservative-fail-open**, decomposed:
- **Sub-slice 3a (INERT foundation, byte-identical):** `module_bindings: BTreeMap<file,BTreeMap<name,BindingKind>>` (TOP-LEVEL only; kind∈{FuncDef(fid),ClassDef,Assignment,ImportReexport,Other}) + `indexed_files: BTreeSet<String>` (authoritative singleton module resolution) + `ImportBinding{local,module_path,member:Option,kind:ImportKind}` new extraction (member recoverable from `aliased_import.name`/JS `import_specifier`). Plumbing (full/skeleton/subset builds, empty/remove_files/merge) + CACHE 23→24. NO rung → behavior-identical → safe PR.
- **Sub-slice 3b (behavior):** the R4c rung (after R4b `:1311`, before R5 `:1313`), Python-only first. Exact ONLY when: import kind==Named w/ concrete member; module resolves to EXACTLY ONE indexed file; that file's `module_bindings[member]==FuncDef(fid)` (excludes nested/class/assign/re-export); NO same-name shadow in caller's enclosing fn or a later module-level same-name binding. Else fail open to R5. JS deferred (needs an export table for re-exports).
| **1b** inheritance MRO | **SPEC drafted** `/tmp/slice1b-spec-draft.md` (queued; branch off post-slice-3 main; reuses slice-3 ImportBinding for cross-file bases) | memo `/tmp/slice1b-architect-out.md` |

### Slice-3 impl surface (studied, ready to plan)
- New rung goes **after R4b implicit-this (`resolution.rs:1311`), before R5 free-fn pool (`:1313`)**. Rename — "R4.5" is TAKEN by Go SamePackage (`:1258`); call it **R4c / import-member**.
- **KEY subtlety (verified):** the candidate pool `ids = self.functions.get(name)` is keyed by the **CALL name** (the local). For an alias `from x import f as g; g()` the def is named `f`, so `functions.get("g")` won't contain it — the rung must do a **fresh `self.functions.get(member)`** lookup, not narrow the existing `free` set. Non-aliased: local==member so the buy is `free_multi`→Exact; aliased calls **currently DROP UnknownName** (no repo `g`) → rung is pure recall recovery.
- **`extract_imports` member-loss CONFIRMED** (`ast.rs:664-671`): `aliased_import` stores only `alias`→module, drops the `name` (member) field — but `child_by_field_name("name")` IS available (used at `:625`), so the member is recoverable with a small extraction change.
- `imports` is `BTreeMap<file, BTreeMap<alias, module_path>>` (per-file nested) — `import_bindings` mirrors it. CACHE 23→24.

### Architect results + execution order (all 3 done; measured buys are SMALLER than headlines)
**ORDER: decorated (in flight) → 2 → 3 → 1b** (by measured Python buy; all sequential off fresh main).
- **2 typed receivers (~700 Python sites):** owner-lookup hits ~171 FastAPI + ~542 pydantic; **Express ≈0**
  (CommonJS Router, no in-repo ES classes — defer JS). Currently in `dropped_multi_owner` + `r6_single_owner`
  NameOnly. **Design = Option B "hit-or-fallthrough":** open the `recover_simple_ident` (`resolution.rs:320`)
  + `receiver_type_in_fn` (`ast.rs:403`) Rust|Go gates for Python/JS/TS; recover typed params + constructor
  locals + annotations; feed R6 `owner_lookup`; **on MISS fall through to R6 residue, do NOT drop-to-
  ExternalReceiver** (FastAPI has 1,416 syntactic recoveries with no owner hit → a drop would spike).
  Rust/Go byte-identical. First-merge guard: constructor-locals + explicit annotations only (skip
  import-qualified/attribute type syntax, TS structural, CommonJS). Bare owner-key (demote-on-multi safety).
- **3 import-scoping/free_multi (~300 sites; 25k is EDGES not sites):** same-dir is UNSOUND for Python/JS
  (siblings not in scope w/o import); same-file already R4. **Design = Option B import-binding rung:**
  richer `ImportBinding` (local name, module path, **imported member** — aliases currently lose it), resolve
  module path → repo file, add a rung after R4-local/before R5-free-multi; Exact on single candidate, multi
  demotes, external/unresolved fails open to R5. Buy: ~241 pydantic + 64 fastapi import-singletons; residual
  ~2,647 pydantic genuinely ambiguous (stays NameOnly, correct). Named imports first (JS default/CommonJS
  deferred).
- **1b inheritance MRO (16 sites — smallest, LAST):** 12 FastAPI + 4 pydantic + 0 excalidraw in-repo
  inherited self/this; external bases dominate (SCIP). Option A span-keyed `class_bases` (preserve 1a's
  `(file,class_span)` identity), walk bases after same-class miss, external/ambiguous = MRO barriers,
  conservative single-provider Exact. New Tier-A fixture needed (`inherited_override` is mislabeled).

## Decorated slice — design + open review findings (folding to spec rev 2)
**Design:** wrapper-canonical — at extraction, skip the inner `function_definition` when its parent is a
`decorated_definition`; keep the wrapper as the single `FunctionId`. Removes the duplicate id / CPG node /
double body-scan; fixes free-fn duplicate-Exact + decorated-method NameOnly demotion (~20% pydantic
methods). **Spec-review (SHIP-WITH-FIXES) findings being folded into rev 2:**
- **BLOCKER:** the unwrap companion is incomplete — a single `unwrap_decorated(node)` helper must be used by
  `find_parameters_node` (`ast.rs:3922`), `function_body_node` (`:2607`), `statements_in_function`
  (`:3097`), `statement_spans_in_function` (`:3112`), `return_value_nodes` (`:2828-2893`). Centralize.
- **MAJOR:** NOT Python-only — C++ has the same wrapper/inner shape (`template_declaration` +
  `function_definition`, `queries.rs:129-133`). Reword to "Python decorator wrapper"; defer C++ template
  canonicalization; add a C++ template **canary (no-change)**.
- **MAJOR:** inventory currently drops the wrapper / keeps the inner (`navigation/inventory.rs:34-56`);
  wrapper-canonical **inverts** that — decide the contract, update its test, note start-line/kind churn.
- **MAJOR:** the manual fallback collector `collect_functions_manual` (`ast.rs:466-474`, reachable via
  `:286-288`) reintroduces the duplicate — apply the canonical filter there too, or centralize before
  `FunctionInfo`.
- MINOR: start-line shifts `def`→decorator (nav `nodes_at` churn); acceptance add helper + C++ + free-fn
  `LocalDef` + inventory tests.

## Environment / ops
- a2a-bridge: `~/code/a2a-bridge/target/release/a2a-bridge run-workflow <id> --input /tmp/sr-input.md
  --config <abs.toml> --session-cwd <repo> --out <abs> 2>err`, wrap `timeout`. `--input` does NOT reach
  codex (task in `prompt_file`). codex implement config = effort `high` + `sandbox_mode="workspace-write"`;
  review/architect = `xhigh` + `read-only`. Ports used this loop: 8210-8221 → **next ≥8222**.
- call-stats: `./target/release/prism nav --no-cache call-stats --repo <ABS>` → JSON. C/C++/large-TS DON'T
  complete. Acceptance corpora: fastapi, pydantic, express, excalidraw (Python/JS that complete); Rust =
  ripgrep, Go = caddy (byte-identical inertness check).
- Worktrees: `/tmp/prism-decorated` (decorated slice). Main tree `/Users/wesleyjinks/code/slicing` on main.
- Architect memos (raw codex output): `/tmp/{slice2,slice3,slice1b}-architect-out.md` (ephemeral —
  formalize into specs before relying on them).

## Next action (live)
1. **Slice 2:** ✅ SHELVED (REWORK; see ⚠️ block above). Branch preserved. Owner decision flagged for morning. Moving on.
2. **Slice 3:** 🅿️ PARKED at rev3 (see ⚠️⚠️ CONSOLIDATED OWNER DECISION above). Blocked on 1-line owner ratification → rev4 ready. Branch+designs preserved.
3. **Slice 1b (NOW ACTIVE — last mandate slice, smallest ~16 sites, most bounded):** spec drafted `/tmp/slice1b-spec-draft.md`. **Attempting with slice-3 lessons FRONT-LOADED** (same-file bases only [independent of parked slice 3], occurrence-rule eligibility, **wildcard-poison**, bounded-static contract stated explicitly, conservative MRO barriers). TIGHT stop: 1 spec-review → SHIP→plan→build→merge (clean win); fixable REWORK→1 fold→re-review; contract-wall REWORK→PARK with slice 3 under the SAME owner decision. Note: 1b relies only on prism's EXISTING bounded-static contract (same-file class hierarchy = lower dynamic-risk than slice 3's cross-file imports), so it may be cleanly shippable without a NEW decision.

Ports used this loop: 8210-8221, 8245 → **next ≥8250**. Update this handoff at each milestone.

# Arity-Disambiguation for Go Interface Dispatch — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop Go interface-dispatch from minting same-named methods of the wrong arity — filter `interface_impls` candidates by the call site's argument count so a 3-arg `handler.ServeHTTP(rr, req, next)` (caddyhttp.MiddlewareHandler) no longer mints the 2-arg `HandlerFunc` (caddyhttp.Handler).

**Architecture:** Capture arity on both sides at extraction time — the **method param count** in the GoTypeProvider (which already extracts each method's signature) surfaced as a new `CallGraph.method_arity` map (parallel to `method_owners`), and the **call argument count + spread flag** on `CallSite` (populated in the Go Calls extraction). Then add an arity filter at the single interface-dispatch mint in `resolution.rs`, guarded to never lose recall when arity is unknown (call uses spread, candidate is variadic, or either count is unavailable). Re-run the Slice-E dispatch oracle on caddy as the regression gate (the one `over_approx` site must flip to `sound`, `dispatch_precision` → 1.0).

**Tech Stack:** Rust, tree-sitter-go, the existing CPG/resolution layer; the Python `eval/tools/dispatch_oracle.py` as the acceptance gate.

**Scope guard (no new scope):** the *filter* applies **only** at the Go `interface_impls` mint (`resolution.rs:678`) — the single measured FP locus. It does **not** touch the `owner_lookup` same-owner overload mint (`resolution.rs:486`), other-language resolution, or any other rung of the R1–R7 ladder. But the work is **structured for reuse** (see "Generalize for reuse" below) so the `owner_lookup:486` C++/Java overload locus can adopt the same filter later without rework. Arity *capture* is general where cheap (the `CallSite.arg_count`/`arg_spread` fields, language-agnostic) and Go-scoped where it costs extraction (the `method_arity` map is Go-populated now; its *shape* is language-agnostic). The filter is conservative — unknown arity (either side) → **keep**.

See **"Cross-language scope"** below for why Go-only is the correct line and which other languages are (and are not) susceptible.

> **Rev 2 — codex (gpt-5.5 xhigh) review folded (2026-06-16).** Record: `docs/prism-query-layer/arity-plan-review-codex-2026-06-16.md`. Folded: **[BLOCKER]** the filter must also run in `interface_dispatch_manifest` (the oracle reads the manifest, not CPG edges) — Task 3 now applies one shared helper at **both** loci; **[MAJOR]** the cross-language immunity is prism-*model*-dependent, not purely language-fundamental (the `owner_lookup:486` locus can affect TS/Python/Rust too) — Cross-language + Backlog corrected; **[do-now]** a manifest-level test (Task 3 Step 4) + a fanout/recall-delta gate (Task 4 Step 3). Confirmed by codex: FunctionId alignment (Task 1) and the recall-safety rule (no Go recall-loss case found). Tasks 1 & 2 (data capture) were already implemented + committed (`cf550f2`, `d0eb6be`) in parallel with the review and are unaffected.

---

## Background: the live FP (κ-confirmed, dispatch_precision 0.9994 → target 1.0)

`modules/caddyhttp/headers/headers_test.go:366` — `err = handler.ServeHTTP(rr, req, next)` (3 args, the `MiddlewareHandler` shape). prism recovered `handler` as a `constructor_local`, `owner_lookup` missed, and the interface consult minted `HandlerFunc` (whose `ServeHTTP` is 2-arg `(w, r)`). Name-based: `interface_impls[(iface_key, "ServeHTTP")]` returns every impl regardless of arity. Recorded in `eval/adjudications.jsonl` (`measurement=interface_dispatch`, fingerprint `498353d980a73060`).

## Key code anchors (verified)

- **The mint to filter:** `src/resolution.rs:677` — `self.interface_impls.get(&(k, name.to_string()))` → `exact(ids.iter(), ResolutionKind::InterfaceDispatch)`. The `site: &CallSite` is in scope here.
- **`CallSite`** `src/call_graph.rs:24` — fields: caller, callee_name, line, start_byte, end_byte, qualifier, receiver_type, receiver_recovery. **No arg count.**
- **`CallGraph.method_owners`** `src/call_graph.rs:67` (`BTreeMap<FunctionId, String>`) — the population pattern to mirror for `method_arity`.
- **`interface_impls` built** `src/call_graph.rs:939` (`self.interface_impls = table.impls`); the GoTypeProvider computes it.
- **GoTypeProvider method extraction** `src/type_providers/go.rs:811` `extract_method` builds `GoMethod { name, receiver_type, is_pointer_receiver, signature, generic, file, start_line, end_line }`. The receiver is a separate `receiver` field (`extract_receiver`, go.rs:854) — **not** in the param list. `find_parameters_node` + the `parameter_declaration` / `variadic_parameter_declaration` node kinds give the param count.
- **`insert_sat_keys`** `src/type_providers/go.rs:1184` pushes `(admission_key, method.func_id)` into `sat_keys[(iface, method)]` — the per-impl record where arity can be carried.
- **Cache:** `CACHE_VERSION = 10` `src/cpg_cache.rs:51`; pin test `cache_version_is_10_for_phase_ip_pr2`. A `CallSite` field add ⇒ bump to **11**.
- **Tests:** `tests/integration/resolution_test.rs` — `build()` / `build_with_receiver_config()` helpers, `go_iface_src()` fixture (interface `Runner { Go() }`, impls `Fast`/`Slow`). Interface-dispatch tests: `type_assertion_interface_receiver_dispatches_exact`, `var_local_interface_receiver_dispatches_exact`, `interface_manifest_implementers_set`.

---

## Cross-language scope (why Go-only is correct)

prism's resolution ladder mints a **set** of same-named candidates at `Exact` confidence in exactly **two** places; everywhere else multiple same-named candidates are **demoted** to `NameOnly` (`StemMulti :556`, `FreeMulti :815`, dyn-trait CHA `:484`) or **dropped** (`MultiOwnerCollision :760`):

1. **`interface_impls` (`resolution.rs:678`) — Go-only.** A set of *different-type* implementers, populated solely by the GoTypeProvider (`go.rs:247`). **This is the measured FP and the only locus this PR fixes.**
2. **`owner_lookup` (`resolution.rs:486`) — same-owner overloads.** `Exact` when one type has ≥2 same-named methods (`pool.len()>1 && primary_owners.len()==1`); cross-owner multiples demote (`:483/:484`).

| Language | Susceptible? | Why (language fundamental, not prism policy) |
|---|---|---|
| **Go** | ✅ — **this PR** | No method overloading, but prism's name-based `interface_impls` mixes same-named methods of *different interfaces* (`Handler.ServeHTTP/2` vs `MiddlewareHandler.ServeHTTP/3`). A prism imprecision, arity-fixable. |
| **C / C++** | ⚠️ latent — `owner_lookup:486` | Genuine in-language overloading → a class can hold `f(int)` **and** `f(int,int)`; `obj.f(1)` mints both `Exact`. Different locus; **unmeasured** (no corpus); arity only *partial* (overloads also differ by type/default-args/templates). |
| **Java** | ⚠️ latent — `owner_lookup:486` | Same as C++ — overloading by signature. |
| **Rust** | ❌ | No overloading within an impl (compiler-forbidden). The only same-name multiplicity is one name across two *traits* on a type — Rust forces explicit `Trait::foo(x)` disambiguation, and the right discriminator is trait identity, not arity; prism already demotes cross-trait multiples. **Language-fundamental immunity** — holds even without demote/drop. |
| **Python** | ❌ | Last-def-wins: a `(class, name)` has exactly one definition. No arity set can exist. **Language-fundamental** — demote/drop irrelevant. |
| **TS / JS** | ❌ | JS last-def-wins; TS collapses overload *signatures* into one implementation. One runtime method per name. **Language-fundamental.** |

**Conclusion:** Go-only is correct for the *fix*. But the immunity framing needs a correction (codex MAJOR 2): the table's rightmost column is the *language* reality — at the **language** level, Python/JS/TS/Rust have one logical definition per `(type, name)`. prism's **extraction model does not enforce that**: it buckets every syntactic same-`(owner, name)` method (`call_graph.rs:367-378` / `languages/mod.rs` owner indexing), with no last-def-wins collapse and Rust trait methods dual-keyed by concrete owner *and* trait. So `owner_lookup:486` can mint a same-owner **set** at `Exact` for **any** language where such a set exists in the model — e.g. **TS overload signatures** (`f(a)` + `f(a,b)`), a **redefined Python method**, or a **Rust cross-trait same-name** pair. The arity-conflation class is therefore **not** exclusive to C++/Java; C++/Java are merely where in-language overloading makes it *common*. **For this PR none of that matters** — the fix is `interface_impls`-only and the `owner_lookup:486` locus is out of scope, **unmeasured** (no C/C++/Java corpus — see the backlog), and only **partially** arity-addressable (overloads also differ by parameter type / default args / templates). Deferred and broadened (see "Backlog / related deferred work").

## Generalize for reuse

So the C++/Java `owner_lookup:486` locus can adopt this later without rework:
- **Task 1** — `MethodArity` and `CallGraph.method_arity` are **language-agnostic in shape**; only the *population* (GoTypeProvider) is Go-specific now. A future C++/Java arity pass populates the same map.
- **Task 2** — `CallSite.arg_count`/`arg_spread` are **language-agnostic** (counting `argument_list` children works for every grammar); populate them wherever cheap, not just Go.
- **Task 3** — extract the predicate as a **standalone, language-neutral helper**, e.g. `fn arity_admits(arg_count: Option<usize>, arg_spread: bool, m: Option<&MethodArity>) -> bool` (the conservative unknown→keep + variadic/spread rule), so both the `interface_impls` mint **and** a future `owner_lookup:486` overload filter call the *same* function. The PR wires it only at `:678`.

## Task 1: Capture Go method param-arity (provider → `CallGraph.method_arity`)

**Files:**
- Modify: `src/type_providers/go.rs` (capture param count + variadic on `GoMethod`; surface it on the interface table)
- Modify: `src/call_graph.rs` (new `method_arity` field; populate at `apply_go_interface_dispatch` / table consume site ~`:939`)
- Test: `tests/integration/resolution_test.rs`

**Design:** `MethodArity { params: usize, variadic: bool }` (`params` excludes the Go receiver). Captured at `extract_method` (the `node` is available; count `find_parameters_node` children of kind `parameter_declaration`, treat a `variadic_parameter_declaration` as `variadic: true`). Carried to `CallGraph.method_arity: BTreeMap<FunctionId, MethodArity>` so `resolution.rs` can look up each candidate FunctionId. Cleared in `clear_interface_dispatch` alongside `interface_impls`.

- [ ] **Step 1: Failing test** — a Go fixture with two same-named methods of different arity; assert `method_arity` records each. (TDD: write the assert against `cg.method_arity` for `Fast.Go`/`Slow.Go`-style impls extended with a 1-param variant.)

```rust
#[test]
fn method_arity_records_param_count_excluding_receiver_and_variadic() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type H struct{}\nfunc (h H) Do(a int, b int) {}\n\
         type V struct{}\nfunc (v V) Do(xs ...int) {}\n",
        Go,
    )]);
    let do_h = cg.method_arity.iter().find(|(f, _)| f.name == "Do" && f.file == "main.go" && f.start_line <= 2 && f.end_line >= 2);
    // H.Do: 2 params, not variadic (receiver excluded)
    let (_, a) = do_h.expect("H.Do arity recorded");
    assert_eq!(a.params, 2);
    assert!(!a.variadic);
    // V.Do: variadic
    let v = cg.method_arity.iter().find(|(_, ar)| ar.variadic).expect("variadic recorded");
    assert_eq!(v.1.params, 1);
}
```

- [ ] **Step 2: Run → fails** (`method_arity` field doesn't exist). `cargo test --test integration resolution_test::method_arity_records_param_count_excluding_receiver_and_variadic`
- [ ] **Step 3: Implement** — add `MethodArity` (in `resolution.rs` or `call_graph.rs`, `Serialize/Deserialize/Clone`), a `params`+`variadic` capture in `extract_method`, plumb through `GoTypeData` → the interface `table` → `CallGraph.method_arity`. Mirror `method_owners` population + `clear_interface_dispatch` reset. **Verify the receiver is excluded** (count params under `find_parameters_node`, not the `receiver` field).
- [ ] **Step 4: Run → passes**; full `cargo test --test integration resolution_test::` green.
- [ ] **Step 5: Commit** `feat(go): capture method param-arity for dispatch (method_arity map)`

## Task 2: Capture call-site argument count + spread flag on `CallSite`

**Files:**
- Modify: `src/call_graph.rs` (`CallSite` — add `arg_count: Option<usize>`, `arg_spread: bool`, both `#[serde(default)]`)
- Modify: `src/ast.rs` (Go Calls extraction — count `argument_list` children; set `arg_spread` if a `spread_element`/variadic-call argument is present)
- Modify: `src/cpg_cache.rs` (`CACHE_VERSION` 10 → 11; update the pin test)
- Test: `tests/integration/resolution_test.rs`, `tests/ast/cpg_cache_test.rs`

**Design:** `arg_count = None` means "not captured / unknown" (the conservative value — never filtered). `arg_spread = true` for `f(xs...)` (Go) — also never filtered. Populate at the Go Calls extraction where the `CallSite` is built (the `argument_list` node is reachable from the `call_expression`). Keep non-Go languages emitting `None` unless trivially free (the filter is Go-only, so other languages need not populate it).

- [ ] **Step 1: Failing test** — assert a recovered Go call site carries the right `arg_count`.

```rust
#[test]
fn callsite_records_go_argument_count() {
    use prism::languages::Language::Go;
    let (cg, _) = build(&[(
        "main.go",
        "package main\nfunc f(a int, b int, c int) {}\nfunc g() { f(1, 2, 3) }\n",
        Go,
    )]);
    let site = cg.calls.values().flatten().find(|s| s.callee_name == "f").expect("call to f");
    assert_eq!(site.arg_count, Some(3));
    assert!(!site.arg_spread);
}
```

- [ ] **Step 2: Run → fails** (no `arg_count` field).
- [ ] **Step 3: Implement** the field + extraction + `CACHE_VERSION` → 11 (rename the pin test to `cache_version_is_11_for_arity`). Confirm `arg_count`/`arg_spread` are excluded from any `CallSite` cmp/identity key that must stay byte-stable (mirror the `receiver_recovery` `#[serde(default)]` + cmp-exclusion treatment).
- [ ] **Step 4: Run → passes**; `cargo test --test integration resolution_test:: && cargo test --test ast cpg_cache_test::` green.
- [ ] **Step 5: Commit** `feat(go): capture call argument count + spread on CallSite (cache v11)`

## Task 3: The arity filter at the interface-dispatch mint

**Files:**
- Modify: `src/resolution.rs` (the `arity_admits` helper + the filter at the `interface_impls` mint, ~:677)
- Modify: `src/navigation/queries.rs` (**apply the SAME filter** in `interface_dispatch_manifest`, ~:116-119)
- Test: `tests/integration/resolution_test.rs`

> **⚠ BLOCKER from codex review (folded):** the filter must be applied at **two** consumers of `cg.interface_impls`, not one. `resolve_call_site` (`resolution.rs:677`) feeds CPG edges / M2; but `nav interface-manifest` emits `implementers` **directly** from `cg.interface_impls` (`navigation/queries.rs:116-119`), and the Slice-E **dispatch oracle reads that manifest** (`dispatch_oracle.py:563`) — not CPG edges. Filtering only at `:677` would leave the manifest (and therefore Task 4's oracle gate) showing the same `HandlerFunc` FP. **Both call sites must run the same arity filter via one shared helper.**

**Design (the precise rule):** a shared helper drops a candidate FunctionId only on a **confident exact mismatch** — keep unless **all** of: the call's `arg_count` is `Some(n)`, `!site.arg_spread`, the candidate's `method_arity` is `Some(a)`, `!a.variadic`, and `a.params != n`. Any unknown (no call count, spread call, no recorded arity, variadic candidate) → **keep** (recall-safe; codex independently verified no Go recall-loss case under this rule). At `resolution.rs:677`, if filtering empties the set, drop with `DropReason::ExternalReceiver` (the no-impl path) — do not fall through. At the manifest, filtering empties → the site emits `implementers: []` / `fanout: 0` (a concrete/dropped site), same as a no-impl receiver.

**Note (precision of the premise, codex MINOR 4):** Go interface *satisfaction* is already signature-aware (`type_providers/go.rs:1235` `method_set_satisfies`). The FP is **not** at satisfaction — it's the final dispatch-table lookup `interface_impls[(iface_key, method_name)]`, which is keyed per `(interface-key, method-name)` and carries **no call-site arity**. The recovered receiver/interface-key maps the 3-arg call to an interface whose impl set includes the 2-arg `HandlerFunc`. The arity filter operates on the already-minted candidate set, so it composes cleanly with satisfaction.

- [ ] **Step 1: Failing test** — the 2-arg-vs-3-arg `ServeHTTP` split (mirrors caddy). A 3-arg call dispatches to only the 3-arg impl.

```rust
#[test]
fn interface_dispatch_filters_candidates_by_call_arity() {
    use prism::languages::Language::Go;
    // Two interfaces share the method name `Serve` at different arity; the 3-arg
    // call site must mint only the 3-arg implementer.
    let (cg, _) = build(&[(
        "main.go",
        "package main\n\
         type H struct{}\nfunc (h H) Serve(w int, r int) {}\n\
         type M struct{}\nfunc (m M) Serve(w int, r int, next int) {}\n\
         type Mid interface { Serve(w int, r int, next int) }\n\
         func use() { _ = H{}; _ = M{} }\n\
         func call(x Mid) { x.Serve(1, 2, 3) }\n",
        Go,
    )]);
    let callees = resolve_callees(&cg, "main.go", /* the x.Serve(1,2,3) line */ 9);
    let names: Vec<&str> = callees.iter().map(|c| owner_of(&cg, c)).collect();
    assert!(names.contains(&"M"), "3-arg call must mint M.Serve");
    assert!(!names.contains(&"H"), "3-arg call must NOT mint 2-arg H.Serve");
}
```
*(Use the resolution-test idiom actually present in `resolution_test.rs` — `resolve_call_site` + `method_owners`/`implementers` — to express “the minted set is exactly {M}”. The fixture shape is what matters: two same-named methods, arities 2 and 3, a 3-arg dispatch.)*

- [ ] **Step 2: Run → fails** (today both `H` and `M` are minted).
- [ ] **Step 3: Implement the shared helper + filter at BOTH loci.** Write a standalone language-neutral helper in `resolution.rs`, e.g. `fn arity_filter<'a>(impls: &'a [FunctionId], arg_count: Option<usize>, arg_spread: bool, method_arity: &BTreeMap<FunctionId, MethodArity>) -> Vec<&'a FunctionId>` built on the inner predicate `fn arity_admits(arg_count: Option<usize>, arg_spread: bool, m: Option<&MethodArity>) -> bool` (the conservative unknown→keep + variadic rule). Call it from **(a)** the `interface_impls` mint at `resolution.rs:677` (over `self.method_arity`), and **(b)** `interface_dispatch_manifest` in `src/navigation/queries.rs` (~:116-119) where `implementers`/`fanout` are computed from `cg.interface_impls.get(...)` — using the manifest site's `arg_count`/`arg_spread` and `cg.method_arity`. **Both must use the one helper** (the BLOCKER). This is also the "Generalize for reuse" contract so a future `owner_lookup:486` locus reuses it. Add a `DropReason`/telemetry note only if it falls out naturally; do not wire it anywhere else.
- [ ] **Step 4: Manifest-level test (codex do-now).** A resolver test alone does NOT cover the oracle path. Add a test that builds the 2-arg-vs-3-arg `Serve` fixture, calls `interface_dispatch_manifest(&cg)`, and asserts the 3-arg dispatch site emits `implementers == ["M"]` (not `["H","M"]`) and `fanout == 1`. (Mirror `interface_manifest_implementers_set` in `resolution_test.rs`.)
- [ ] **Step 5: Recall-guard tests** — add/confirm: (a) a **variadic** candidate is kept against any call arity; (b) a **spread call** `x.Serve(args...)` keeps all candidates; (c) an unknown-`arg_count` site keeps all candidates; (d) the existing `type_assertion_interface_receiver_dispatches_exact` / `var_local_interface_receiver_dispatches_exact` / `interface_manifest_implementers_set` still pass unchanged (same-arity dispatch unaffected, at both loci).
- [ ] **Step 6: Run → all pass**; full `cargo test` green; `cargo fmt`.
- [ ] **Step 7: Commit** `fix(go): arity-disambiguate same-named interface-dispatch candidates (resolver + manifest)`

## Task 4: Regression gate — dispatch oracle confirms the FP flips to sound

**Files:**
- Verify only (no source change): `eval/tools/dispatch_oracle.py`, `docs/eval/tier-a/slice-e-caddy-dispatch-baseline.json`

- [ ] **Step 1:** `cargo build --release`
- [ ] **Step 2:** regenerate the caddy manifest + run the oracle:
```bash
target/release/prism nav interface-manifest --repo ~/code/bench-repos/caddy > /tmp/caddy-manifest.json
cd eval && uv run python tools/dispatch_oracle.py --manifest /tmp/caddy-manifest.json \
  --repo ~/code/bench-repos/caddy --corpus caddy --out /tmp/caddy-dispatch-oracle.json
```
- [ ] **Step 3:** confirm `over_approx` `1 → 0`, `dispatch_precision 0.9994 → 1.0`, the `headers_test.go:366` site classifies `sound`. **Recall-delta check (codex do-now #3):** `over_approx == 0` alone is insufficient — the oracle's `load_dispatch_sites` ignores `fanout == 0` sites (`dispatch_oracle.py:303-306`), so a filter that wrongly empties a dispatch set (drop-to-empty recall bug) would silently *leave the gate* rather than fail it. So also diff the manifest before/after: the **dispatch-site count must not fall** and **no previously-fanned-out site may drop to `fanout 0`** except the one fixed FP site. Compare `prism nav interface-manifest` site counts + per-site fanout pre/post; paste the before/after (precision **and** the dispatch-site/fanout delta) into the PR.
- [ ] **Step 4:** update `docs/eval/tier-a/slice-e-caddy-dispatch-baseline.json` `gate.dispatch_precision_floor` → 1.0, `confirmed_prism_fp` → 0, with a note referencing this fix; flip the `eval/adjudications.jsonl` site-1 record handling per the gate (the prism_fp is now resolved — record the resolution, do not silently delete). Mark the deferred-doc precision follow-up **DONE**.
- [ ] **Step 5: Commit** `eval(tier-a): dispatch precision 1.0 on caddy after arity-disambiguation`

## Final verification (before PR)

- [ ] `cargo fmt --check` · `cargo test` (all suites) · `cargo build --release`
- [ ] `cd eval && uv run tier-a --matrix-only --allow-stale-sut` — no regressions (the `go/interface_dispatch*` cases stay ok)
- [ ] `cd eval && uv run pytest -q` — green
- [ ] dispatch oracle: caddy `dispatch_precision` 1.0, `over_approx` 0
- [ ] PR description carries the oracle before/after + confirmation of no recall loss (the recall-guard tests + no new `over_approx`)

## Risks / watch-items

- **Recall loss masquerading as precision.** The whole point is dropping *wrong* edges; a buggy filter could drop *right* ones. Mitigations: the conservative unknown→keep rule, the variadic/spread guards, and Task 4 step 3's "no new over_approx" check.
- **Variadic call-vs-def.** A variadic def (`...T`) accepts n≥(params−1) args; never filter a variadic candidate by exact count (Task 1 captures `variadic`, Task 3 honours it).
- **Cache.** v10→v11 invalidates stale caches; confirm the warm/cold paths and the pin test (Task 2).
- **Non-Go languages.** `arg_count=None` for them ⇒ filter is a no-op; the Go-language gate at `resolution.rs:673` already scopes the mint.

## Backlog / related deferred work

- **`owner_lookup:486` same-owner overload arity (deferred, broadened per codex MAJOR 2).** The same arity-conflation class has a second, *unmeasured* locus: `owner_lookup` mints same-owner method **sets** `Exact` (`resolution.rs:486`). Because prism's model buckets every syntactic same-`(owner, name)` method (no last-def-wins / trait-disambiguation collapse), this is **not** C++/Java-only — it can bite **TS overload signatures, redefined Python methods, and Rust cross-trait same-name** pairs too. C++/Java are simply where in-language overloading makes it common. Adopting the Task-3 helper there is the fix, **but** it requires (a) **method-arity capture per language** (this PR populates `method_arity` Go-only), (b) a **corpus** to measure + gate it (no C/C++/Java corpus exists; the multi-language exposure also wants TS/Python/Rust evidence — see below), and (c) recognition that arity is **partial** (overloads also differ by parameter type / default args / templates). Alternatively, document why prism deliberately tolerates these same-`(owner, name)` sets. Do **not** build speculatively; gate it on a corpus.
- **Corpus-matrix expansion (backlog).** Pull + baseline new Tier-A corpora to (i) measure the C++/Java overload locus above, and (ii) broaden language coverage: **2 C, 2 C++, 2 Java, +1 Go, +2 Rust** libraries. Rationale: the `prism` self-corpus drifts every commit (always SHA-`baseline_invalid` until re-pinned at a merge) and `tokio` is large/slow + oracle-floor-failing (0.22) — so the Rust anchor leans on a moving target + a noisy one; 1–2 *smaller, stable* Rust libs give a steadier Rust anchor. Full spec: `docs/eval/tier-a/corpus-expansion-backlog.md`.

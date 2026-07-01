## Merged Spec Review: Prism Navigation Layer (Tier 1) v2

---

### BLOCKER

**B1 — `func_index` re-key: spec must explicitly evaluate and reject Option C before planning** *(both lenses)*

The spec chose re-keying to a 3-tuple `(file, name, start_line)` (§3), cascading compile errors across `cpg.rs`, `cpg_cache.rs`, `data_flow.rs`, `call_graph.rs`, and all review algorithms. Soundness identified a third path the spec did not consider:

**Option C (additive, zero-breakage):** Keep `func_index` as `BTreeMap<(String,String),NodeIndex>` unchanged. Add `line_range_index` as a second, independent index built from the already-present `CpgNode::Function { start_line, end_line }` data (`cpg.rs:394-399`). Navigation queries use `line_range_index`; all existing algorithms, the Contains-edge build, `from_parts()`, and `reconstruct_cpg()` (`cpg_cache.rs:325`) remain untouched. No `CACHE_VERSION` bump. No golden re-baseline.

Option C is viable because the data is already there. The spec must make one of these explicit arguments before planning:
- "Option C is sufficient — adopt it" (blocker dissolves), or
- "Option C cannot be used because navigation queries must share the same index as review queries for consistency" (blocker stands, and `cpg_cache.rs:325` must be added to the change inventory the spec omitted).

**Disagreement resolution:** Soundness is right. The spec's framing as a binary choice was an oversight. The choice is three-way and materially changes implementation scope.

---

**B2 — Parse-failure enforcement gap: spec vs. current code conflict** *(Rigor, code-verified by both)*

§5 specifies that `ParseFailed` files are excluded from the graph. Current code (`src/algorithms/mod.rs:63-101`, `src/main.rs:384-413`) already emits "Skipping analysis" in the warning string for `>30%` error rate, but then builds the CPG from the same unfiltered `files` map. The spec must clarify:

1. Is this enforcement fix part of Step 3 (repo_loader) or Step 1 (CPG core)?
2. What is returned when a `callers`/`nodes-at` query references a symbol in a `ParseFailed`-excluded file — `UnsupportedFile` error, or `SkippedPath` warning with empty items?
3. Does the existing diff-review path adopt the same exclusion, or does it keep current lenient behavior?

Without these answers the repo_loader contract (§5) and the query error model (§8) are inconsistent with each other.

---

### MAJOR

**M1 — Score decay formula is unspecified** *(Soundness)*

§8 says `score` encodes proximity "decaying per hop" but pins no formula. `1.0/(1+hop)` vs `0.5^hop` produce different orderings at hop≥2 and different truncation behavior under `--max-results`. One sentence closes this: e.g., `score = 1.0 / (1 + hop_distance)`, integer hops, ties broken by `(file, start_line, ordinal)`. Required for deterministic output and reproducible goldens.

---

**M2 — `NavigationIndex` omits `TypeRegistry` and `live_types`** *(Soundness)*

§3 defines `NavigationIndex { cpg, profile, parse_quality }`. `CpgContext` carries `types: TypeRegistry` (owned, `cpg.rs:62`) and `live_types: BTreeSet<String>` (RTA dispatch pruning). Without these in `NavigationIndex`, callee resolution for the 9 non-C/C++ languages loses language type information, and RTA-based dispatch pruning is unavailable to nav queries. `CodePropertyGraph.type_db` is C/C++-only and does not substitute. Add both fields; they are read-only at query time and cheap to carry.

---

**M3 — `line_range_index` lookup semantics underspecified** *(Soundness)*

§3 and §7 name `line_range_index` without specifying containment semantics. For a query line inside a closure or lambda (common in JS and Rust), "first function with `start_line ≤ query`" returns the outer function; "innermost enclosing function" (smallest `end_line - start_line` among all matches with `start_line ≤ query ≤ end_line`) is unambiguous and what callers expect. One sentence naming the semantics prevents a future regression when closures appear in fixtures.

---

**M4 — Output contract goldens are incomplete** *(Rigor, partially addressed by spec)*

§8 provides goldens for `callers` (success), `nodes-at` (enclosing function), empty result, and `AmbiguousSymbol`. Missing: `callees`, `ego-graph` (full graph shape), `module_deps`, `repo_map`, and at least one example for each `WarningKind` (`ParseQuality`, `IndirectCallApprox`, `UnresolvedModule`, `Collision`, `SkippedPath`). §16 names the fixture *scenarios* but does not include the expected JSON. Add goldens before implementation; they are the executable contract.

---

**M5 — Review compatibility fixture list is underspecified** *(Rigor)*

§12 states that stdout bytes, stderr bytes, exit codes, and `--format` variants are regression-locked, and §16 mentions a "CLI legacy compatibility" fixture. Neither section lists which flag combinations constitute the lock or what the accepted delta is. Specify: which `--algorithm`, `--format`, and flag combinations are in the golden set; how the func_index re-baseline delta (§11) is marked as the sole sanctioned diff; and which CI step fails on unapproved deviation.

---

**M6 — Localization (M4 disposition) — DISMISSED**

Rigor flagged NL/BM25 localization scoring as a MAJOR. The v2 spec explicitly defers it (`M4 disposition: "Out of scope — reasoning-layer concern, not Tier-1"`). Not a finding against this spec.

---

### MINOR

**m1 — Coverage test count: two arrays, not three** *(both lenses)*

`tests/integration/coverage_test.rs` has two `all_test_files` definitions (at lines `:106` and `:325`), not three as stated in §16 ("Update the three `all_test_files` copies"). Update §16 guidance and CLAUDE.md to say "two copies."

---

**m2 — Grammar fingerprint: build-script embed, not runtime read** *(Soundness)*

§9 says to hash `Cargo.lock` entries for `tree-sitter-*` crates. The spec should name the mechanism: a build-script-generated compile-time constant (matching the existing `env!("CARGO_PKG_VERSION")` pattern in `cpg_cache.rs:181,249`), not a runtime Cargo.lock read. Runtime reads fail in release artifacts and break reproducibility.

---

**m3 — `SymbolResolver` trait object-safety** *(Soundness)*

§14's `SymbolResolver` trait takes `ResolverContext<'a>` where `'a` appears in method arguments. This is not trivially box-erasable without an HRTB bound. The spec must choose before writing the trait: (a) trait parameter lifetime (verbose but explicit), or (b) `Arc<NavigationSession>` in `ResolverContext` (removes the lifetime at one allocation). Either is fine; the choice ripples through all implementations.

---

**m4 — `resolve_callers` qualifier gap: scope as known limitation** *(Soundness)*

§7 correctly says to make `resolve_callers` qualifier-aware. The spec should note that this fixes an existing call-graph precision gap (`call_graph.rs:801` ignores `site.qualifier`), not a nav-layer regression. The fix is a one-line forwarding change to `resolve_callees_qualified`; if it reveals behavioral complexity, it should be tracked as a separate item rather than silently widening §7 scope.

---

**m5 — Binary name / clap app name mismatch** *(Soundness, code-verified)*

`Cargo.toml:9` names the binary `prism`; `main.rs:38` names the clap app `slicing`. §11 says "left unchanged." If intentional (gradual rename), document it in §11. If an oversight, fixing it is a one-line change and should be included in Step 1.

---

**Verdict:** Needs changes before planning. B1 (Option C evaluation) and B2 (parse-failure enforcement order and error model) must be resolved in the spec text — both are single-paragraph clarifications. M1–M3 are single-sentence fixes. M4–M5 require enumerating goldens and fixture combos. Resolve the two BLOCKERs and the three one-sentence MAJORs first; the plan is ready to execute after that.
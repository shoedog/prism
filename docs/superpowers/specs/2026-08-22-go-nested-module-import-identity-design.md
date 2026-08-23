# Go nested-module import identity — design v2 (roadmap #14, folds #15b) — **four slices**

Date: 2026-08-22 · Status: **v4 — sol round 3 (declared cap): slices 1–2 APPROVE (implementing); slices 3–4 FIX → folded here (§4/§5); the adversarial diff review is the next gate for slices 3–4 (convergence 7→7→5, narrowing)** · Owner decision 2026-08-22: **full scope as sol's round-1 review specified, delivered as slices**
(each slice = its own spec section here → implementer → controller gate → adversarial diff review → PR). Sol round-1 findings (7 WRONG + test
corrections) are folded below; the slice that closes each finding is marked **[R1-n]**.

## 0. Slices, order, and what each closes
| Slice | Scope | Closes | Gate | Cache |
|---|---|---|---|---|
| **1 oracle hardening** | `interface-manifest` package-qualified identities `(package_dir, package_clause, type_name)` + target method file/span; `dispatch_oracle.py` qualified compare, `oracle_unresolved`, zero-fanout scoring, `--baseline` delta mode (over_approx / timeout / unresolved in the delta block), no vacuous 1.0, pinned environment; re-baseline main | [R1-3] [R1-4] | its own pytest + synthetic collision fixtures (`good.Impl`/`bad.Impl`; `p`/`p_test`; build-tagged twins) red on today's tool | none (additive manifest fields) |
| **2 loader/parser/cache hygiene** | Go `testdata/` excluded from Go inputs; go.mod tokenizer + `module` directive grammar (incl. parenthesized) + semantic module-path validation; symlinked `go.mod`/`go.work` = hashed terminal boundary (`symlink_refused` topology entry); `go.work` topology-hashed; one immutable manifest snapshot shared by hashing and parsing; `SKIP_POLICY_VERSION` 1→2; cache transition | [R1-5] [R1-6] [R1-7] | full suite, tier-a, same-base control (5 corpora); hardened oracle ≥ baselines (run after slice 1 merges) | CPG 45 / sidecar 14 |
| **3 effective module identity** | `GoModuleGraph`: declared (nearest `go.mod`) + active set (`go.work` `use` — relative or absolute in-repo — or root) + **applicable** local `replace` (workspace version of an active main module always wins; `go.work` replaces first; then the union of active modules' replaces, conflicts fail closed; a replace applies only when its LHS path is required by an active main module; version-specific replaces fail closed) → **effective** import path per dir, memoized; whole-workspace fail-closed on malformed `go.work`/`use`/active `go.mod`; nested `Local` tokens; telemetry | [R1-1] (+ #15b) | same-base control + hardened oracle in delta mode (`gate_ok`) | CPG 46 / sidecar 15 |
| **4 alias-aware step 2** | clause/profile-scoped type-alias expansion substituting the ENTIRE canonical RHS type expression (generics, pointers, slices, maps, funcs, predeclared, nested aliases; profile variants must agree) before `Local`/`Qualified` tokens; then `Local↔Local` by path | [R1-2] | same as 3 | CPG 47 / sidecar 16 |
Order: 1 ‖ 2 for IMPLEMENTATION (independent files; 2 bumps cache, 1 does not), but slice 2's oracle ACCEPTANCE runs only after slice 1 has merged and main is re-baselined with the hardened oracle → 3 (needs 1's gate and 2's parser/symlink/snapshot rules) → 4. If slices merge back-to-back,
the cache transitions stay one-per-PR by convention (no mid-branch multi-bumps).

## 1. Problem (unchanged from v1, corrected)
P10 (#176) made S4 signature identity import-aware, but a file's own bare types get a `Local` (`~path::T`) token only when the ROOT `go.mod`
proves the path (`CallGraph::go_package_import_paths`); files below a nested `go.mod` stay `Bare` and fail closed against `Qualified`/`Local`
(etcd 1788→1742 interface-dispatch Exact; etcd has 13 modules + `go.work`, prometheus 5 + `go.work`, hugo 4, no `go.work`). Root+dir derivation
would be WRONG for nested modules; and — per sol r1 — the nearest `go.mod`'s DECLARED path is not the EFFECTIVE import identity either: under
`replace original.example/mod => ./fork` the directory `fork/` (declaring `fork.example/mod`) is addressed as `original.example/mod/...`, while an
inactive `decoy/` declaring `original.example/mod` must not be. Identity therefore needs the active module graph (slice 3). Independently, today's
precision gate compares bare receiver NAMES (`manifest` + `dispatch_oracle.py`), so a wrong-package `bad.Impl` scores sound against `good.Impl`
(slice 1), and `testdata/` Go files can donate Exact edges through `Bare↔Bare` + the empty-live fallback (slice 2).

## 2. Slice 1 — dispatch-oracle hardening (tooling only; no resolution change)
- **Manifest** (`src/navigation/queries.rs` interface-manifest site emission, today `implementers: Vec<String>` = owner type names via
  `cg.method_owners` / file stem): ADD an additive per-site field `implementer_identities: [{ "name": <owner type name>, "file": <repo-relative
  method file>, "span": [start_line, end_line], "package_dir": <dir of that file>, "package_clause": <clause of that file; null if unknown> }]`
  (one per kept `FunctionId`, sorted, **deduped on the FULL tuple (package_dir, package_clause, name, file, span)** — never on (dir, name): the target
  method file/span is the Exact-edge evidence for build-tagged twins such as `impl_darwin.go`/`impl_linux.go`); keep `implementers` byte-identical for existing consumers. ALSO list in-scope sites with `fanout == 0` (already
  present in the manifest? — confirm; if the manifest omits them, emit them with `implementers: []` so the oracle can score recall) **[R1-4]**.
- **Oracle** (`eval/tools/dispatch_oracle.py`): (a) satisfier identity = `(package_dir, package_clause, type_name)` on BOTH sides — prism from `implementer_identities`, gopls from each
  `textDocument/implementation` location's file → repo-relative dir + that file's package clause + the receiver type at that line (existing `_type_at`
  extended); a gopls location that cannot be mapped to (dir, clause, receiver, declaration) → `oracle_unresolved` (counted; BLOCKS delta acceptance);
  an identity with unknown clause is unscorable (blocks in delta mode). The oracle is a satisfier-TYPE gate; the Exact-edge TARGET check compares
  prism's `(file, span)` against gopls' implementation location for the same (dir, clause, type) — a mismatch (wrong build-tagged twin) is
  `target_mismatch` and blocks. Name-only fallback ONLY for manifests without identities (old binaries), flagged `identity_mode: "name_only"`
  **[R1-3]**; (b) `load_dispatch_sites` keeps in-scope `fanout == 0` sites (the live manifest already emits them; removing the Python filter suffices);
  classification adds `recall_gap` for them when gopls returns satisfiers (reported, non-gating, but VISIBLE) **[R1-4]**; (c) `--baseline <prior out.json>` **delta mode**: `delta.newly_exact_sites`
  (fanout 0→>0 or new implementer identities) each with its classification; `gate_ok = no over_approx AND no oracle_timeout AND no oracle_unresolved AND no target_mismatch among delta sites AND fanout-positive SITE coverage ≥ 0.90 AND fanout-positive EDGE coverage ≥ 0.90` (the dual coverage floors were added at sol's slice-1 implementation review r2 Q3 and shipped in #180; `not_dispatch` / `interface_zero_fanout` / `external_definition` / `unknown_definition` are reported exclusions; `sound_site_rate` is site-weighted, `dispatch_precision` edge-weighted);
  summary prints `gate_ok` and the offending sites; the output pins the environment: corpus SHA, `go version`, `gopls version`, GOOS/GOARCH/tags,
  effective GOWORK (the tool forces the repo-root `go.work` when present, else `GOWORK=off`, so a parent/environment workspace cannot change the
  universe); a baseline/branch pair with different pins is refused **[R1-4]**; (d) empty scored denominator → `dispatch_precision: null` + `scored_sites` count,
  never a vacuous 1.0; timeouts still excluded from precision but counted and (in delta mode) blocking; (d') gopls locations are normalized to the ENCLOSING METHOD DECLARATION span (not the raw selection/name range) before comparing with prism's
  full-node `(file, span)` (sol r3 implementation condition); (e) collision fixtures (pytest, red→green on today's tool): `good.Impl` vs `bad.Impl` (different dirs); `p.Impl` vs `p_test.Impl` (same dir,
  different clause); `impl_darwin.go` vs `impl_linux.go` (same dir/clause/type, different target file → `target_mismatch`).
- Re-baseline main (264c8ef) with the hardened oracle for caddy/prometheus/etcd (+hugo) and record the identity-aware numbers; they replace the
  name-only baselines (caddy 1.0000/70, prometheus 0.9715/649, etcd 0.9931/1481, 2026-08-22 16:19) as the gate.
- Tests: Rust — manifest field serialization + byte-compat of the existing fields + fanout-0 site emission; Python — oracle unit tests for
  (a)–(e) with fake gopls adapters (no live gopls in pytest).

## 3. Slice 2 — loader / parser / cache hygiene
- **`testdata/`** **[R1-5]**: Go ignores `testdata` directories; today `repo_loader::BUILTIN_SKIP_DIRS` excludes `vendor` (so the v1 vendor pole
  is unreachable — correct, test the loader exclusion instead) but not `testdata`, so `testdata/**/*.go` methods enter the S4 satisfier population
  and can be minted Exact via `Bare↔Bare` (predeclared-typed signatures) + the empty-live fallback. Fix: **language-aware** skip — Go files under a
  `testdata` path segment are skipped at load (other languages unaffected: a Python repo's `testdata/` is real code). Measure: same-base control on
  the 5 corpora (expect small Exact/NameOnly removals; non-Go byte-identical); telemetry `skipped_go_testdata_files`.
- **`module` directive** **[R1-6]**: replace `parse_go_module_path` with a small go.mod TOKENIZER following the documented grammar
  (https://go.dev/ref/mod#go-mod-file-grammar): line-oriented; `//` comments; bare and quoted (interpreted, with unescape) and raw-backtick strings;
  **parenthesized directive blocks** (`module (\n example.com/m\n)` is legal); `/* */` is NOT legal → whole file malformed; exactly one `module`
  directive with exactly one path token, else malformed; then **semantic module-path validation** equivalent to Go's `module.CheckPath`
  (non-empty, ASCII, each element non-empty, allowed characters `A-Za-z0-9.~_-`, no element starting/ending with `.`, no `..`/`.` elements, no
  leading/trailing `/`, no `//`, the major-version suffix rule for `/vN` with N ≥ 2, Windows reserved names rejected) → else malformed (fail
  closed; `module bad!path` must NOT prove an identity). Unit-test poles: plain; quoted; raw-quoted; trailing comment; trailing junk; duplicate;
  empty; parenthesized; `/* */` → malformed; `bad!path`; `example.com/m/v2` ok; `example.com/m/v1` (invalid major) malformed.
- **Symlinked `go.mod` / `go.work`** **[R1-7]**: resolution currently follows symlinks while `collect_manifest_hashes` hashes only `is_file()`
  entries → stale-cache hole. Fix: (i) a symlinked `go.mod`/`go.work` is REFUSED for identity AND recorded in the topology key as an entry
  `<rel> = symlink_refused` (hash its presence and path-kind, not its target bytes) so add/remove/regular↔symlink transitions invalidate; (ii) a
  refused OR malformed nearest `go.mod` is a **terminal unproven boundary** — resolution does NOT continue upward to the parent module
  (otherwise a nested symlink would inherit the root identity); (iii) **one immutable manifest snapshot**: the loader reads the bytes of every
  `go.mod`/`go.work` (and Cargo.toml) once into `LoadedRepo` (content + hash); the module graph/parsers consume THOSE bytes — never a second disk
  read — so graph B can never be keyed under hash A (sol SMELL, folded); telemetry reason `symlink`.
- **Cache**: derived data changes for identical inputs (testdata removal) → **CPG 45 / sidecar 14** with ancestry comment; `SKIP_POLICY_VERSION`
  1 → 2 (loader skip policy changed); pin tests for both round-trip and incremental; a go.mod/go.work add/remove/edit/malformed-transition/
  module-path-change/symlink-transition test matrix on the cache key.

## 4. Slice 3 — effective module identity (memoized)
- **`GoModuleGraph`** (new module, e.g. `src/go_module_graph.rs`, built once per call-graph build from `repo_root`; all reads memoized; `go.mod`
  files read once): (i) every non-symlinked `go.mod` under the repo (vendor/testdata excluded) → `(module_dir, declared_path)` (validated parser);
  (ii) **active main modules** = when a regular, fully valid `go.work` exists at repo root: its `use` directories — relative OR absolute, canonicalized
  (normalized `..` allowed) and lexically proven to lie inside the repo; each must contain a regular, valid `go.mod` — a malformed `go.work`, any
  `use` outside the repo / unprovable / missing or malformed target `go.mod`, or a malformed active-module `go.mod` → the WHOLE workspace graph
  fails closed (all Go identities unproven; reason `workspace_invalid`); without `go.work`: the root `go.mod` module if present, else none; (iii) **applicable local replacements** **[R1-1]**: FIRST parse ALL `replace` directives (local-dir AND module-version RHS, in-repo or not,
  usable or not) from `go.work` and from every ACTIVE main module's `go.mod`; THEN determine the precedence WINNER per LHS (`go.work` replaces
  override module replaces; among active main modules the union applies, and two different replacements of the same LHS without a `go.work`
  override = conflict); ONLY THEN test the winner for usability: it must be a local directory RHS (relative RHS resolved against the directory of
  the file containing the directive), inside the repo, lexically proven, containing a regular valid `go.mod`, and applicable per (4)–(5). An
  unusable/unprovable WINNER (version RHS, outside the repo, missing/malformed/symlinked `go.mod`, version-specific) → the LHS is
  `replace_unproven`: NEVER fall through to a lower-precedence local replacement (sol r3-1). Precedence and applicability: (1) an active main module's WORKSPACE version always wins for its own module path — a
  replace whose LHS is an active main module's path is ignored (the etcd shape: root `go.mod` replaces `go.etcd.io/etcd/api/v3 => ./api` while
  `api` is a `use`d main module — `api/**` stays `go.etcd.io/etcd/api/v3/...`); (2) `go.work` replaces are consulted first; (3) then the UNION of
  the active modules' replaces — two active main modules replacing the same LHS to different targets without a `go.work` override is a
  WORKSPACE error in Go → the whole workspace graph is `workspace_invalid` (all Go identities unproven), not just that LHS (sol r3-2); likewise
  two active main modules (or an active main module and an applicable replace target) claiming the SAME effective path → `workspace_invalid`; (4) a wildcard replace (no LHS version) applies only if its LHS module path is `require`d by at least one active main
  module (a replace alone never enters the module graph); (5) a version-specific replace (`A vX => dir`) applies only with PROOF that vX is the
  selected version — prism does not compute MVS, so version-specific replaces FAIL CLOSED (documented recall stance; telemetry `replace_unproven`).
  Packages under an applicable target dir are addressed as `A + rel(pkg_dir, target_dir)` — the REPLACED path, regardless of the target's declared
  path. Version replacements (`=> other/module v1.2.3`) are ignored. (iv) **Effective identity of a package dir** = from its nearest enclosing provider after the precedence above: an active main module (declared
  path + rel) or an applicable replace target (replaced path + rel); a dir under an inactive, unreplaced module only → **no identity (fail closed)** with a reason (`inactive_module`, `replace_unproven`,
  `workspace_invalid`, `no_go_mod`, `malformed`, `symlink`); duplicate effective paths among active providers are a workspace error (above). A nested `go.mod` excludes its
  subtree from the parent module (nearest wins) — inside an ACTIVE parent, a nested INACTIVE module's packages get no identity.
- `go_package_import_paths` → consults the graph (per-dir memo); `local_import_paths` rule unchanged (single ordinary clause) → nested `Local`
  tokens for proven effective identities. Comparator unchanged in this slice (`Local↔Local` still by name until slice 4).
- Telemetry: `go_module_graph { modules, active, replaces, duplicate_providers }`, `go_import_path_proven_files`, `go_import_path_unproven_files`
  + reason histogram; conservation test: `proven + unproven == loaded Go files` and `Σ reasons == unproven`.
- Cache: **CPG 46 / sidecar 15**; `go.work` + all `go.mod` already in the key (slice 2).
- Tests (all resolver + manifest parity, asserting TARGET FILES/identities, not only owner names **[R1-3]**): go.work with `use .` AND
  `use ./nested` → root interface bare `Context` ↔ nested implementer `root.Context` (Qualified, same path) → Exact `Impl` at `nested/impl.go`;
  the same fixture WITHOUT go.work → nested inactive → fail closed (empty); `require original.example/mod v0.0.0` + `replace original.example/mod
  => ./fork` (fork declaring `fork.example/mod`) + inactive `decoy/` declaring `original.example/mod`: a signature `original.example/mod/p.T` matches
  `fork/p` (Exact) and NOT `decoy/p`; the SAME without the `require` → no identity for `fork/p` (unrequired replace is inert); version-specific
  replace → fail closed; the etcd shape (active main module also named by another main module's replace) → workspace version wins; conflicting
  replaces in two active modules WITHOUT a `go.work` override → whole workspace unproven (no Exact edges from m1/m2 at all) and WITH the override →
  override wins; a `go.work` replace with a non-local/version RHS over a module-file local replace of the same LHS → `replace_unproven` (no fall-through);
  duplicate effective paths among active mains → workspace unproven; replace target missing/malformed/symlinked `go.mod` → inert; valid absolute
  in-repo `use`; missing root `go.mod` + valid root `go.work`; malformed whole `go.work` → all unproven; `use` target missing `go.mod` → all
  unproven; duplicate providers → both dropped; nested-in-nested (depth 2); relative `../` replace inside repo; replace target outside repo
  ignored; memoization: each `go.mod`/`go.work` parsed once from the loader snapshot; full == incremental == cached (46) parity; non-Go byte-identical.
- Gate: same-base control (5 corpora; every Exact loss attributed; every Exact gain scored by the hardened oracle in delta mode: `gate_ok` required)
  + precision ≥ slice-1 baselines per corpus.

Oracle delta gate: `gate_ok` required per corpus, OR a documented exception where every blocking site is attributed to a tracked pre-existing class with owner sign-off. 2026-08-22 owner decision: etcd's 5 blocking sites (cache_test.go:1383/1559 Get → RecordingClient; revision_test.go:114/126 Client; v3_failover_test.go:93 Endpoints) are the roadmap-#17 concrete-receiver class surfaced by identity recovery (0 lost-exact, 370 sound); slice 3 merges with this exception and #17-narrow's acceptance must turn them to 0 (`oracle-s3b-etcd.json` is that baseline).

## 5. Slice 4 — alias-aware `Local↔Local` by path
- Alias expansion **[R1-2]**: record every `type A = <type expr>` per declaring (dir, clause, profile) (P10 identity; `_test` clause aliases
  invisible to production), keeping the RHS as a canonicalized TYPE EXPRESSION (not a named leaf). `canon_type`, on a `type_identifier`/
  `qualified_type` leaf that names an alias visible to the file (own package with a visible declaring profile, or via the file's import map for
  qualified), SUBSTITUTES the entire canonical RHS expression (instantiated generics `base.List[int]`, pointers, slices, maps, funcs, predeclared,
  nested aliases — transitively, cycle-guarded) BEFORE producing `Local`/`Qualified` tokens; the alias index snapshots EVERY declaration variant for `(package_dir, package_clause, type_name)` with its kind (`Alias(canonical RHS)` or
  `Defined`) and declaring profile; expansion is allowed only when every EXACTLY visible variant (P10 exact visibility/certainty boundary, not mere
  compatibility) is an `Alias` and all RHS canonicalize identically — any visible `Defined` variant (e.g. `a_linux.go: type A = int` vs
  `a_windows.go: type A int`), uncertain profile visibility, or incomplete provenance → `AliasUnresolved` (sol r3-3); **parameterized aliases**
  (`type Twice[T any] = Pair[T, T]`, Go ≥ 1.24): arity-checked binding of alias type parameters to the supplied type arguments, capture-safe
  substitution into the RHS, transitive expansion; wrong arity / unbound parameters / cycles / unsupported constraints → `AliasUnresolved` (sol r3-4);
  **predeclared aliases** are normalized in canonical comparison: `byte → uint8`, `rune → int32` (keep the existing `any`/empty-interface
  normalization) (sol r3-5); an unresolvable alias target → `AliasUnresolved` (fail closed on the Exact
  path). Then the comparator: `(Local, Local) => left_path == right_path`; `(Bare, Bare) => true` stays.
- Tests: alias-to-local, alias-to-qualified, alias to an instantiated generic (`type L = base.List[int]`; interface `Use(L)` vs `Use(base.List[int])`
  → Exact kept), parameterized alias (`Twice[int]` vs `Pair[int, int]` → Exact; wrong arity → unresolved), `type B = byte` vs `uint8` → Exact and
  `rune` vs `int32` → Exact, alias to composite/predeclared types, aliases in two packages to one base type → Exact kept, build-profile variants
  `type A = int` (linux) vs `type A int` (windows) → `AliasUnresolved` (no Exact), agreeing variants → expanded; distinct defined types with the same name in two
  proven packages → empty (red today: name match); alias cycle → fail closed; `_test`-clause alias invisible; generic instantiation wrapping an
  alias keeps shape; the P10 fixture `s4_unqualified_named_types_keep_the_existing_bare_name_rule` is renamed to state that its two proven-path types
  no longer match, with a separate `Bare↔Bare` (no go.mod) variant keeping the name rule.
- Cache: **CPG 47 / sidecar 16**. Gate as slice 3; the step's recall loss is visible via slice 1's zero-fanout scoring (1→0 transitions).

## 6. Acceptance summary per slice
Full suite; tier-a `--matrix-only` 0 regr; fmt; clippy no new warnings; `git diff --check`; same-base `call-stats --no-cache` vs the then-current
main on ripgrep/caddy/prometheus/etcd/hugo (leaf diff; losses attributed); hardened oracle (slices 2–4) with `--baseline` delta mode: `gate_ok`
true and per-corpus precision ≥ baseline; cross-model pairs — implementer: slice 1 terra, slices 2–4 sol @ xhigh; adversarial reviewer: slice 1 sol, slice 2 terra, slices 3–4 terra @ xhigh
(with sol's spec record in hand).

## 7. Round-2 answers recorded + round-3 questions (cap)
Recorded from sol r2: Q1 identity includes `package_clause`, plus target file/span for the Exact-target check; unmappable gopls locations →
`oracle_unresolved` (blocks delta). SMELLs folded: `SKIP_POLICY_VERSION` 1→2 (slice 2); one immutable manifest snapshot shared by hashing and
parsing (slice 2); delta baselines pin corpus SHA / go / gopls / GOOS / GOARCH / tags / GOWORK (slice 1). Slice boundaries confirmed; 1 ‖ 2 for
implementation only.
Round 3 (slices 3–4 focus): R1 is the conservative applicability rule (wildcard replace needs a `require` by an active main module; version-specific
replaces fail closed; workspace version wins) sound in every case Go can present, and does it lose anything on etcd/prometheus/hugo beyond the
documented stance? R2 whole-workspace fail-closed on any malformed part — too coarse for a monorepo with one broken leaf module? R3 alias
substitution of full RHS expressions — interaction with `signature_has_generic_syntax`/Generic gaps and with P10 clause-keyed owner identity?
(Previous round-2 questions, now superseded:)
- Q1 Slice 1 identity `(package_dir, type_name)`: is there a repo shape where two Go packages share a directory (only `_test` external packages —
  should the identity include the clause?) or where gopls' implementation location is not in the receiver's declaring package?
- Q2 Slice 3 active-set rule without `go.work`: root module only (nested inactive unless replaced) — matches `go build` semantics; acceptable
  recall stance for hugo-style repos?
- Q3 Slice 3: should `replace` directives in INACTIVE modules be ignored entirely (yes per Go: only the main module's replaces apply; in a
  workspace, go.work replaces override and module replaces of workspace modules are... — confirm: in workspace mode, `replace` directives in the
  workspace modules' go.mod files are still honored unless overridden by go.work; state the precise rule).
- Q4 Slice 4: alias expansion scope — package-level aliases only, or also aliases of generic instantiations (`type L = List[int]`)?
- Q5 Cache: three transitions across three PRs vs consolidating when PRs land back-to-back — convention says one per PR; confirm.
- Q6 Anything in P10's clause-keyed owner identity that disagrees with effective-path identity for `replace` targets (the dir-based owner is
  `fork/p`, the path-based identity says `original.example/mod/p`)?

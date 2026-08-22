# Go nested-module import identity — design v2 (roadmap #14, folds #15b) — **four slices**

Date: 2026-08-22 · Status: **v2 for sol round 2** · Owner decision 2026-08-22: **full scope as sol's round-1 review specified, delivered as slices**
(each slice = its own spec section here → implementer → controller gate → adversarial diff review → PR). Sol round-1 findings (7 WRONG + test
corrections) are folded below; the slice that closes each finding is marked **[R1-n]**.

## 0. Slices, order, and what each closes
| Slice | Scope | Closes | Gate | Cache |
|---|---|---|---|---|
| **1 oracle hardening** | `interface-manifest` package-qualified identities; `dispatch_oracle.py` qualified compare, zero-fanout scoring, delta mode with timeout block, no vacuous 1.0; re-baseline main | [R1-3] [R1-4] | its own pytest + a synthetic collision fixture red on today's tool | none (additive manifest fields) |
| **2 loader/parser/cache hygiene** | Go `testdata/` excluded from Go inputs; full `module` directive validation; symlinked `go.mod` consistency (refuse + don't hash); `go.work` topology-hashed; cache transition | [R1-5] [R1-6] [R1-7] | full suite, tier-a, same-base control (5 corpora), hardened oracle ≥ baselines | CPG 45 / sidecar 14 |
| **3 effective module identity** | `GoModuleGraph`: declared (nearest `go.mod`) + active set (`go.work` `use` / root) + local `replace` → **effective** import path per dir, memoized; inactive/duplicate/ambiguous providers fail closed; nested `Local` tokens; telemetry | [R1-1] (+ #15b) | same-base control + hardened oracle (delta sites all scored; over_approx/timeout in delta = STOP) | CPG 46 / sidecar 15 |
| **4 alias-aware step 2** | clause/profile-scoped type-alias expansion before `Local`/`Qualified` tokens; then `Local↔Local` by path | [R1-2] | same as 3 | CPG 47 / sidecar 16 |
Order: 1 ‖ 2 (independent files; 2 bumps cache, 1 does not) → 3 (needs 1's gate and 2's parser/symlink rules) → 4. If slices merge back-to-back,
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
  method file>, "package_dir": <dir of that file>, "package_clause": <clause of that file, if known> }]` (one per kept `FunctionId`, sorted,
  deduped on (package_dir, name)); keep `implementers` byte-identical for existing consumers. ALSO list in-scope sites with `fanout == 0` (already
  present in the manifest? — confirm; if the manifest omits them, emit them with `implementers: []` so the oracle can score recall) **[R1-4]**.
- **Oracle** (`eval/tools/dispatch_oracle.py`): (a) identity of an implementer = `(package_dir, type_name)` on BOTH sides — prism from
  `implementer_identities`, gopls from each `textDocument/implementation` location's file → its repo-relative dir + the receiver type at that line
  (existing `_type_at`); name-only fallback ONLY when a manifest lacks identities (old binaries) and then the summary says `identity_mode:
  "name_only"` **[R1-3]**; (b) `load_dispatch_sites` keeps in-scope `fanout == 0` sites; classification adds `recall_gap` for them when gopls
  returns satisfiers (reported, non-gating, but VISIBLE) **[R1-4]**; (c) `--baseline <prior out.json>` **delta mode**: `delta.newly_exact_sites`
  (fanout 0→>0 or new implementer identities) each with its classification; `gate_ok = no over_approx AND no oracle_timeout among delta sites`;
  summary prints `gate_ok` and the offending sites **[R1-4]**; (d) empty scored denominator → `dispatch_precision: null` + `scored_sites` count,
  never a vacuous 1.0; timeouts still excluded from precision but counted and (in delta mode) blocking; (e) the collision fixture: a synthetic
  manifest with `good.Impl`/`bad.Impl` + a fake gopls adapter → today's tool says sound, hardened says over_approx (pytest, red→green).
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
- **`module` directive** **[R1-6]**: `parse_go_module_path` → validate the whole directive per the go.mod grammar: exactly one `module` directive
  (duplicate → None), path token either bare or an interpreted/raw string (unescape `"..."`), NO trailing tokens other than a `//` comment,
  non-empty, no whitespace inside → else None (fail closed). Unit-test poles: plain, quoted, raw-quoted, trailing comment, trailing junk, duplicate,
  empty, `module` inside a block comment line.
- **Symlinked `go.mod`** **[R1-7]**: resolution currently `read_to_string`s (follows symlinks) while `collect_manifest_hashes` hashes only
  `is_file()` entries (symlinks skipped) → a symlink-target module-path edit leaves the cache key unchanged. Fix: identity resolution REFUSES a
  symlinked `go.mod` (`symlink_metadata().file_type().is_symlink()` → treated as absent → fail closed) so what is read == what is hashed;
  telemetry reason `symlink`. Also hash `go.work` (and refuse symlinked `go.work`) now, so slice 3 inherits a correct key.
- **Cache**: derived data changes for identical inputs (testdata removal) → **CPG 45 / sidecar 14** with ancestry comment; pin tests for both
  round-trip and incremental; a go.mod/go.work add/remove/edit/malformed-transition/module-path-change test matrix on the cache key.

## 4. Slice 3 — effective module identity (memoized)
- **`GoModuleGraph`** (new module, e.g. `src/go_module_graph.rs`, built once per call-graph build from `repo_root`; all reads memoized; `go.mod`
  files read once): (i) every non-symlinked `go.mod` under the repo (vendor/testdata excluded) → `(module_dir, declared_path)` (validated parser);
  (ii) **active main modules** = `go.work` `use` directories (repo-relative, resolved against the go.work dir; `..`/absolute/outside-repo → ignored)
  when `go.work` exists at repo root, else the root `go.mod` module if present, else none; (iii) **local replacements** = `replace A [vX] => ./rel`
  (and `../rel`) directives in ACTIVE modules' `go.mod` and in `go.work` (workspace replaces) whose target dir is inside the repo → packages under
  that dir are addressed as `A + rel(pkg_dir, target_dir)` — the REPLACED path, regardless of the target's declared path **[R1-1]**; version
  replacements (`=> other/module v1.2.3`) and targets outside the repo → ignored. (iv) **Effective identity of a package dir** = from its nearest
  enclosing provider: an active main module (declared path + rel) or a replace target (replaced path + rel) — if BOTH apply to the same dir or two
  providers claim the same effective path (duplicate providers) or a dir is under an inactive module only → **no identity (fail closed)** with a
  reason (`inactive_module`, `duplicate_provider`, `ambiguous_provider`, `no_go_mod`, `malformed`, `symlink`). A nested `go.mod` excludes its
  subtree from the parent module (nearest wins) — inside an ACTIVE parent, a nested INACTIVE module's packages get no identity.
- `go_package_import_paths` → consults the graph (per-dir memo); `local_import_paths` rule unchanged (single ordinary clause) → nested `Local`
  tokens for proven effective identities. Comparator unchanged in this slice (`Local↔Local` still by name until slice 4).
- Telemetry: `go_module_graph { modules, active, replaces, duplicate_providers }`, `go_import_path_proven_files`, `go_import_path_unproven_files`
  + reason histogram; conservation test: `proven + unproven == loaded Go files` and `Σ reasons == unproven`.
- Cache: **CPG 46 / sidecar 15**; `go.work` + all `go.mod` already in the key (slice 2).
- Tests (all resolver + manifest parity, asserting TARGET FILES/identities, not only owner names **[R1-3]**): go.work with `use ./nested` → root
  interface bare `Context` ↔ nested implementer `root.Context` (Qualified, same path) → Exact `Impl` at `nested/impl.go`; the same fixture WITHOUT
  go.work → nested inactive → no identity → fail closed (empty); `replace original.example/mod => ./fork` with fork declaring `fork.example/mod`
  + inactive `decoy/` declaring `original.example/mod`: a signature `original.example/mod/p.T` matches `fork/p` (Exact) and NOT `decoy/p`;
  duplicate providers (two `use`d modules declaring the same path) → both dropped; nested-in-nested (depth 2) resolves against the nearest active
  module; workspace-level `replace`; relative `../` replace inside repo; replace target outside repo ignored; memoization: each `go.mod`/`go.work`
  read exactly once (test hook); full == incremental == cached (46) parity; non-Go byte-identical.
- Gate: same-base control (5 corpora; every Exact loss attributed; every Exact gain scored by the hardened oracle in delta mode: `gate_ok` required)
  + precision ≥ slice-1 baselines per corpus.

## 5. Slice 4 — alias-aware `Local↔Local` by path
- Alias expansion **[R1-2]**: record `type A = B` / `type A = pkg.B` per declaring (dir, clause, profile) (P10 identity; `_test` clause aliases
  invisible to production). `canon_type` resolves a `type_identifier`/`qualified_type` leaf through aliases TRANSITIVELY (cycle-guarded; via the
  file's import map for qualified targets; aliases from the file's own package only when the declaring profile is visible to the file's profile)
  to the ultimate defined type's identity BEFORE producing `Local`/`Qualified` tokens; an unresolvable alias target → new gap `AliasUnresolved`
  (fail closed on the Exact path). Then the comparator: `(Local, Local) => left_path == right_path`; `(Bare, Bare) => true` stays.
- Tests: alias-to-local, alias-to-qualified, aliases in two packages to one base type → Exact kept; distinct defined types with the same name in two
  proven packages → empty (red today: name match); alias cycle → fail closed; `_test`-clause alias invisible; generic instantiation wrapping an
  alias keeps shape; the P10 fixture `s4_unqualified_named_types_keep_the_existing_bare_name_rule` is renamed to state that its two proven-path types
  no longer match, with a separate `Bare↔Bare` (no go.mod) variant keeping the name rule.
- Cache: **CPG 47 / sidecar 16**. Gate as slice 3; the step's recall loss is visible via slice 1's zero-fanout scoring (1→0 transitions).

## 6. Acceptance summary per slice
Full suite; tier-a `--matrix-only` 0 regr; fmt; clippy no new warnings; `git diff --check`; same-base `call-stats --no-cache` vs the then-current
main on ripgrep/caddy/prometheus/etcd/hugo (leaf diff; losses attributed); hardened oracle (slices 2–4) with `--baseline` delta mode: `gate_ok`
true and per-corpus precision ≥ baseline; reviewer: slice 1 terra (tooling), slices 2–4 sol @ xhigh (precision code); implementer: slice 1 terra,
slices 2–4 sol @ xhigh.

## 7. Open questions for sol (round 2)
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

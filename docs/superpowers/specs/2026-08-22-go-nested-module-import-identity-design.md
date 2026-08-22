# Go nested-module import identity (+ memoized `go.mod` walk) — design (roadmap #14, folds #15b)

Date: 2026-08-22 · Status: DRAFT v1 for sol spec review · Owner-approved 2026-08-22: **step 1 (nearest-`go.mod` identity + nested `Local`
tokens, memoized) with a gopls dispatch-oracle precision gate, and step 2 (`Local↔Local` by path) measured separately in the same PR.**
Record: PR #176 (P10) body/known gaps; roadmap rows 14/15; `docs/superpowers/specs/2026-08-22-prism-mcp-lazy-handshake-design.md` is unrelated.

## 1. Problem (measured)
P10 (#176) made S4 signature identity import-aware: qualified leaves `pkg.T` → `@<import_path>::T`; a file's own bare types → `~<own_path>::T`
(`Local`) **only when the ROOT `go.mod` proves the path** (`CallGraph::go_package_import_paths`, `src/call_graph.rs` ~L2883: walk up from the
package dir; a `go.mod` found at any dir other than `repo_root` → `break` with no path). Files below a nested `go.mod` therefore keep `Bare`
leaves, and `Bare↔Qualified` / `Bare↔Local` fail closed (`GoTypeProvider::canon_signatures_match`). etcd has 13 modules (root
`go.etcd.io/etcd/v3` + `api/`, `client/v3/`, `server/`, `pkg/`, `etcdctl/`, `etcdutl/`, `tests/`, `cache/`, `tools/*`), prometheus 5, hugo 4 —
most of their code is nested. Measured: etcd `kind_exact/interface_dispatch` 1788 (main@36b2796) → 1742 (#176), the "nested-module mixed
bare/qualified fail-closed" population. Sol's wave-3 experiment that resolved nested modules measured etcd **2,002** (+214 vs main) but also moved
the RTA empty-live fallback fan-out, so it was refused pending an audit — that audit is this item's gate.
Correctness note: root+dir derivation is WRONG for nested modules (`api/etcdserverpb` is `go.etcd.io/etcd/api/v3/etcdserverpb`, not
`go.etcd.io/etcd/v3/api/etcdserverpb`); only the NEAREST `go.mod`'s `module` line + the package dir's path relative to THAT module dir is correct.

## 2. Design
### 2.1 Step 1 — nearest-`go.mod` import identity (memoized)
- `go_package_import_paths` → per-directory resolution: for each Go file's package dir, find the NEAREST ancestor dir (inclusive) within
  `repo_root` that contains `go.mod`; import path = `<module path>` + (`/` + relative path from that module dir, if non-empty). `module` line
  parsed as today (`parse_go_module_path`: first `module` directive; quotes/backticks trimmed; comments after the path ignored). Memoize per
  directory: `BTreeMap<PathBuf /*dir*/, Option<(PathBuf /*module_root*/, String /*module_path*/)>>` so each `go.mod` is read once and each
  directory resolved once (folds roadmap #15b; the walk is O(dirs) not O(files×depth)).
- Fail-closed cases (no import path → leaves stay `Bare`): no `go.mod` on the path up to `repo_root`; malformed/absent `module` line; the file
  lives under a `vendor/` directory (vendored packages' import paths are the vendored module's, unknowable without `modules.txt`) — confirm what
  `repo_loader` does with `vendor/` today and keep consistent; `testdata/` directories (Go ignores them) likewise no path. `go.work` is IGNORED
  for identity (it selects modules for builds; identity comes from each module's `go.mod`). `replace`/`exclude` directives do not change import
  paths — ignored. A nested `go.mod` excludes its subtree from the parent module — nearest-wins is exactly Go's rule.
- `local_import_paths` (`GoTypeProvider`, unchanged rule: a file gets a `Local` path only when its dir has exactly ONE ordinary (non-test)
  package clause and the file carries it) now receives paths for nested-module files too → `~path::T` tokens there. Comparator UNCHANGED in
  step 1 (`Local↔Qualified` by path; `Local↔Bare`/`Bare↔Qualified` false; `Local↔Local`/`Bare↔Bare` by name).
- Telemetry (`prism nav call-stats`): `go_import_path_proven_files` / `go_import_path_unproven_files` (+ a reason histogram: `no_go_mod`,
  `vendor`, `testdata`, `malformed_module`) so the fail-closed residue is visible; existing `interface_gaps/QualifiedTypeIdentity`,
  `interface_overapprox/NonLocalConstructionFallback`, `interface_fanout/*` unchanged.
- Cache: derived structures change for nested-module repos (more `Local` tokens → different satisfier sets) with identical inputs → **one
  cache transition: CPG 45 / edge-index sidecar 14** (convention from P9/P10; `go.mod` is already in topology hashing since #176). Pin tests.

### 2.2 Step 2 — `Local↔Local` by path (separate commit, measured separately)
`(Local, Local) => left_path == right_path` (today `_ => true`, the retained pre-P10 bare-name over-approximation; both sides now carry a
proven path, so the disproof is available). `(Bare, Bare) => true` stays (no proof either way). Mixed arms unchanged. Pins: two root-module
packages `lib/ID` vs `other/ID` both bare → today Exact `Impl`, after step 2 empty (resolver + manifest); same package, different files → still
Exact; predeclared types (`int`, `error`) unaffected (they are `Bare` on both sides).

### 2.3 Non-goals
No change to the P10 partition lanes, `Local↔Bare` policy, RTA/live-type policy, `NonLocalConstructionFallback` policy (its DELTA is reported,
not changed), dot-import handling, the nav CLI, or non-Go languages (ripgrep byte-identical).

## 3. Tests (TDD; resolver AND manifest parity for every dispatch pole)
1. Nearest-go.mod: root `example/root` + `nested/go.mod` = `example/nested` + package `nested/sub` (bare `Context`, interface `Act(Context)`);
   `good/impl.go` imports `example/nested/sub` → Exact `Good` (red today); a decoy importing the WRONG root+dir path `example/root/nested/sub`
   → no match (pins nearest semantics; keeps P10's `s4_nested_module_mixed_bare_and_qualified_types_fail_closed` shape but inverted to the
   positive). Reverse direction (nested interface bare, root implementer qualified) likewise.
2. Nested module at depth 2 (`a/go.mod`, `a/b/go.mod`) → `a/b/c` resolves against `a/b/go.mod`'s module path, not `a/go.mod`'s.
3. Fail-closed residue: file under `vendor/…` → no `Local` (stays `Bare`; `Bare↔Qualified` false); malformed `module` line → `Bare`;
   `testdata/` → `Bare`; a repo with NO root `go.mod` but nested ones → nested resolved (improvement over today); `go.work` present → ignored
   (same results with and without it).
4. Memoization: with N files in M directories under K modules, `go.mod` is read exactly K times and directories resolved M times (count via a
   test hook / injected reader or by temporarily making go.mod unreadable after the first read — pick the deterministic one).
5. Determinism/parity: full build == incremental rebuild (`remove_files`/`merge`) == cached load (CPG 45 round-trip) for a nested-module
   fixture; non-Go fixture byte-identical; cache version pins updated (45/14) with the ancestry comment.
6. Step 2 pins (§2.2) + the existing `s4_unqualified_named_types_keep_the_existing_bare_name_rule` flips to the new expectation for two
   PROVEN-path files (rename to state the rule) while a `Bare↔Bare` (unproven) variant keeps the name rule.
7. Telemetry: `go_import_path_proven_files`/`unproven` counts + reasons on the fixtures.

## 4. Acceptance (controller; before PR) — step 1 and step 2 measured SEPARATELY (binary per step or two commits)
- Full suite; tier-a `--matrix-only` 0 regressions; `cargo fmt`; clippy no new warnings; `git diff --check`.
- Same-base `prism nav --no-cache call-stats` control (main@264c8ef) on ripgrep (byte-identical), caddy (single module: step 1 no change
  expected; step 2 may REMOVE cross-package same-name bare matches — quantify), prometheus, etcd, hugo (3rd multi-module corpus): leaf diff;
  every Exact LOSS attributed (step 2 removals = the `Local↔Local` different-path population), every Exact GAIN audited by the oracle.
- **gopls dispatch oracle** (`eval/tools/dispatch_oracle.py`; `prism nav interface-manifest` per binary; `--corpus caddy|prometheus|etcd`;
  gopls v0.22 on PATH; etcd/prometheus have `go.work`): `dispatch_precision` per corpus at-or-above main's baseline; list every `over_approx`
  site in the DELTA (sites newly Exact on the branch) with its offending minted types — a delta over-approx that gopls refutes is a STOP;
  `recall_gap` and `NonLocalConstructionFallback` deltas reported. Baselines are run by the controller on main before the branch measurement.
- Step 2 keep/drop decision: keep only if oracle precision holds (expected ≥) and the recall loss is explained (name-only matches across
  distinct proven packages are false by construction — expect precision UP, recall DOWN only where gopls also says non-satisfier).

## 5. Risks / open questions for sol
- Q1 `vendor/`: does `repo_loader` include vendored files today? If yes, is "no import path" the right policy, or should vendored files be
  excluded from S4 entirely (they are today's `Bare`, i.e. name-matchable against `Bare` only)?
- Q2 Symlinked or duplicated module dirs; a `go.mod` whose module path has a `/vN` major suffix with the dir not named `vN` (fine — module path
  is authoritative) — any case where nearest-go.mod gives a path that differs from what `go build` uses?
- Q3 Cache: is a version bump truly needed (derived data changes for identical inputs) — yes by convention; confirm nothing else keys on the
  old `Local` token set (sidecar edge index).
- Q4 Step 2's interaction with P10 partition lanes (clause-keyed owner identity) — any site where `Local↔Local` by path and the partition
  filter disagree?
- Q5 The oracle on multi-module repos: gopls with `go.work` — per-module workspaces or the work file; timeouts → `oracle_timeout` is non-fatal
  but must not hide a precision drop (gate on the sites that resolved).
- Q6 Should `go_import_path_unproven_files` be split per module root in call-stats for custody? (Keep the schema additive.)

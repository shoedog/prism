> **Status: SHIPPED — PR [#164](https://github.com/shoedog/prism/pull/164), merged 2026-07-04 (main 7a06896).** As-executed brief incl. the folded codex spec-review findings ([B1] matchTag alias satisfaction incl. filename suffixes, [B2] 0-survivor counted drop, [B3] per-fact profile filtering, [M1] GoOwnerIdentity counted-not-fixed, [M2] subset-build plumbing, [MIN1] header-region rule). **As-shipped deltas beyond this text (two fix waves + controller commits; implementer = codex gpt-5.5 HIGH per owner directive, controller review replaced the Opus gate):** (1) receiver-fact key shape deviates ACCEPTED — `(dir, name) -> BTreeSet<GoTypedFact{ty, defining_file}>` with consult-time filtering instead of `(dir, package_clause, name)` keys (type change forces every consumer through `unique_visible_type`; imported-constructor path synthesizes a clause-aligned caller profile). (2) Wave 1 (controller review C1/C2 + fresh codex xhigh 3 BLOCKERs, all empirically reproduced): SAT enumeration = FULL syslist GOOS×GOARCH (sentinel plan9/amd64 values minted a false Exact on the canonical `!windows && !plan9 && !solaris` shape); header region = full leading comments+blanks run (the first-blank-line break made the build rung INERT on all 4 corpora — build_exact=0 was the tell); Go-exact directive predicates (`// go:build`/`//go:buildfoo` rejected); `build_unparsed` carried on the profile; syslist KnownOS/KnownArch (zos, nacl, amd64p32, ...); empty package clause fails open + Exact-ineligible; counters mint at Exact time only. (3) Wave 2 (fix-delta re-review, 3 toolchain-verified WRONGs — record now 9-of-10): header scan re-written as a faithful go/build `parseFileHeader` PORT (Comment4: no directive after `*/` on the same line; TooCloseNo: legacy `+build` needs a following blank line; //go:build exempt, pinned both ways); `GoBuildVisibility.certain` — SAT free-boolean bound-exceeded is visible-but-uncertain, and `visibility_allows_exact` (certain ∧ profile_allows_exact) is the ONE rule at all three Exact producers; `go_build_expr_unparsed` de-double-counted (1-not-51 pinned). (4) Deliberate behavior change: a raw-singleton candidate with an unparseable constraint now demotes (was Exact) — unproven singletons never mint Exact. Measured: zap withLogger 39/39 callers Exact 1.0, same_package NameOnly 76→0; etcd build_exact 0→2, prometheus 0→11, NameOnly 566→512; caddy all-zero (census-consistent). Cache 39/9 one transition.

# Task P13 — Go same-package-collision partitioning (package clause + build constraints)

Repo: /Users/wesleyjinks/code/slicing (branch off main @ 9de4402 or later; isolated worktree).
Plan item: P13 in docs/analysis/prism-llm-and-accuracy-plan.md. **The plan's framing is
corrected by grounding** (see §0) — read this brief as the design of record, not the plan text.

> **SPEC-REVIEW FOLD (codex gpt-5.5 xhigh, fix-then-ship; all findings folded below as
> binding text — marked [B1][B2][B3][M1][M2][MIN1] at their sites).** Summary: [B1] the SAT
> model must implement Go's tag-ALIAS satisfaction (android⊨linux, illumos⊨solaris, ios⊨darwin,
> unix derived from the syslist), for filename suffixes too — naive GOOS mutual exclusion mints
> false Exact; [B2] 0-survivor fall-through is unsound (R5 FreeSingle re-promotes the filtered
> definition) — counted drop instead; [B3] receiver-index recovery must be profile-filtered at
> consult time (per-fact defining-file profiles), namespace-only keying is not enough; [M1]
> GoOwnerIdentity-keyed S2/S4 lanes are OUT of scope but must be counted (accepted option per
> review); [M2] subset builds must populate the per-file profile map (full plumbing walk);
> [MIN1] //go:build header-region rule, multiple-directive handling.

## 0. Grounding corrections (verified 2026-07-03 against HEAD + live corpora)

1. **The adjudicated zap FP is NOT a build-tag problem.** zap `withLogger` has two same-directory
   definitions: `common_test.go:37` (`package zap`) and `stacktrace_ext_test.go:148`
   (`package zap_test`). Zero build constraints involved. The collision is a **package-clause
   split** — Go compiles a directory into up to three namespaces (`foo`, white-box `foo_test`
   files sharing `package foo`, black-box `package foo_test`), and a bare call in `package zap`
   can only bind to `package zap` definitions. Live today: `prism nav callers --location
   common_test.go:37` returns `global_test.go` call_site_line 244 (and ~41 more sites) demoted to
   `same_package` NameOnly score 0.6; all these callers are `package zap` and can only legally
   reach `common_test.go`'s definition → should be Exact 1.0.
2. **The prometheus `NewDiscovery` citation is a harness artifact, not a bug.** Live query shows
   `discovery/eureka/eureka.go:127` callers resolve correctly (no azure item). Do NOT treat it as
   an acceptance metric; it needs a re-adjudication note, not code (controller handles).
3. **Corpus census** (6 Go corpora: zap/prometheus/etcd/caddy/cobra/go-redis; same-directory
   free-function name collisions, regex census): 121 colliding `(dir, name)` keys total →
   **4 package-clause splits** (incl. the zap FP), **46 filename GOOS/GOARCH suffix splits**
   (e.g. etcd `TryLockFile` × 5 `lock_{linux,plan9,solaris,unix,windows}.go`),
   **31 `//go:build`/`+build` expression splits** (e.g. prometheus `model/labels` `StableHash` ×
   {`slicelabels`, `dedupelabels`, `!slicelabels && !dedupelabels`}; etcd `!cluster_proxy` vs
   `cluster_proxy`; go-redis `!appengine` vs `appengine`), **40 "true" collisions** — nearly all
   `func init()` (multiple `init` per package is legal Go and `init` is never call-referenced;
   inert) plus generated `.pb.go` duplicates.
4. **No build-constraint handling exists anywhere in src/** (grep `go:build`, `+build`, `GOOS`,
   `GOARCH` → zero hits). `_test.go` files are fully included in the graph (correct — keep it
   that way) and used only as heuristic signals elsewhere.
5. **`docs/eval/tier-a/baseline.md:345-346` records prism's build-agnosticism as a deliberate
   RECALL ADVANTAGE** over compiler-grade oracles (prism sees `#[cfg]`/GOOS-gated code gopls
   can't). This binds the design: partitioning applies ONLY when choosing among same-name
   candidates at resolution; **no file is ever excluded from parsing, indexing, or navigation**.

## 1. Design

### S1 — Per-file `GoBuildProfile` (extraction-time, per-file fact)

New struct (suggested home: a small new module `src/go_build_profile.rs`, or inside
`src/languages/`/`call_graph.rs` if more idiomatic — implementer's call, keep files <600 lines):

```rust
pub struct GoBuildProfile {
    pub package_clause: String,   // from tree-sitter package_clause node — NOT regex
    pub is_test_file: bool,       // filename ends with _test.go
    pub goos: Option<String>,     // filename suffix, after stripping _test
    pub goarch: Option<String>,   // filename suffix rules below
    pub build_expr: Option<BuildExpr>, // parsed //go:build (or legacy // +build) constraint
}
```

- Filename suffix rules (Go spec): strip `.go`, strip trailing `_test`; then
  `*_GOOS_GOARCH`, `*_GOOS`, or `*_GOARCH` — a suffix only counts if it is a known GOOS/GOARCH
  value (use the standard lists; include `unix` NOWHERE here — `unix` is a build-tag alias, not
  a filename suffix). NOTE the Go rule that the segment before the suffix must be non-empty
  (`linux.go` is NOT constrained; `x_linux.go` is).
- `//go:build` line [MIN1]: accepted ONLY in the leading Go header region (before the blank
  line that precedes package docs/code — mirror go/build's rule, see
  $GOROOT/src/go/build/build.go shouldBuild/goBuildLine); **multiple `//go:build` lines =
  treat as unparsed → `None` + count** (go/build errors on it); it takes precedence over legacy
  `// +build` lines (fallback only — within a line: space-separated = OR, comma = AND, `!` =
  negation; **multiple `+build` lines are ANDed together**).
- `BuildExpr`: parse idents, `!`, `&&`, `||`, parens. On parse failure → `None` PLUS a
  telemetry-visible count (see S5) — and `None` means UNCONSTRAINED for satisfiability (safe
  direction: today's demote persists; never mint Exact from an unparsed constraint).
- Missing profile (file failed to parse, non-Go, etc.) → treat as unconstrained/compatible
  (fail open; never a false Exact from absence — but also never a new drop).
- Storage: per-file map on `CallGraph` (e.g. `go_file_profiles: BTreeMap<String, GoBuildProfile>`),
  populated during extraction like other per-file facts. **Full plumbing walk [M2]**: the empty
  constructor, the full builder, AND `build_direct_subset` (subset builds must populate it for
  the changed files — unlike the whole-program Go facts which subset builds leave empty),
  `remove_files` (remove entries), `merge` (extend). It is a per-file fact — no rematerialization
  pass — but the whole-program passes that RUN during rematerialization (receiver-index
  extraction, registration application) consult it, so it must be complete on the merged graph
  BEFORE they run (it is, if subset+merge handle it). Bidirectional incremental parity test
  required (edit a //go:build line → incremental == full; revert likewise). It IS serialized
  with `CallGraph` (bincode) → cache bump (§4).

### S2 — Compatibility predicate (the single shared implementation; doctrine-6: no second copies)

```rust
/// Can a bare (unqualified) call in `caller`'s file legally bind to a definition in
/// `candidate`'s file, and can both files be part of the same build?
fn go_same_package_visible(caller: &GoBuildProfile, candidate: &GoBuildProfile) -> bool
```
Three rungs, all must pass:
1. **Namespace**: `caller.package_clause == candidate.package_clause`. (This alone separates
   `zap` vs `zap_test`.)
2. **Test visibility**: `candidate.is_test_file → caller.is_test_file`. (A non-test file never
   sees `_test.go` definitions; test files see everything in their clause.)
3. **Build satisfiability**: `SAT(constraint(caller) ∧ constraint(candidate))`, where
   `constraint(f)` = conjunction of filename-suffix-implied constraints and `build_expr`.
   Evaluate by brute-force enumeration of an ACTUAL (GOOS, GOARCH) pair — GOOS ∈ {each value
   mentioned anywhere in either constraint} ∪ {a fresh unmentioned one}, same for GOARCH —
   then derive each tag ident's truth from the enumerated actual value using **Go's matchTag
   ALIAS semantics [B1]** (mirror $GOROOT/src/go/build/build.go matchTag + syslist):
   - ident == actual GOOS/GOARCH → true;
   - `linux` is ALSO true when actual GOOS is `android`; `solaris` also true under `illumos`;
     `darwin` also true under `ios`;
   - `unix` is DERIVED: true iff actual GOOS ∈ the go/build unixOS set (aix android darwin
     dragonfly freebsd hurd illumos ios linux netbsd openbsd solaris);
   - **filename-suffix constraints use the SAME matchTag satisfaction, not string equality**
     [B1]: `x_linux.go` is satisfied by actual GOOS android; so `x_linux.go` and `x_android.go`
     ARE compatible (both build under GOOS=android) — a naive exclusivity model would falsely
     prove them incompatible and mint a false Exact. IMPORTANT: because of aliases, the GOOS
     enumeration must include the alias-satisfying values of every mentioned tag (mentioning
     `linux` must put `android` in the candidate set too).
   All other idents (cgo, race, custom tags like `cluster_proxy`, `go1.x`) are free booleans.
   Bound the free-boolean count (e.g. ≤8; above the bound → return compatible=true, count it).
   **Failure direction is a design input**: every ambiguity (parse failure, bound exceeded,
   missing profile) resolves to `true` (compatible) — the status quo demote survives; Exact is
   only ever minted from a PROVEN singleton.

### S3 — Consume at the resolution consult sites

Grounding enumerated every same-package consult (symbols to grep; line anchors are hints):

a. **R4.5 free-function rung** — `resolve_call_site_full`, `src/resolution.rs` ~1962-1995 (the
   `// R4.5: a Go unqualified call resolves within its own package` comment; `dir_of` filter).
   After the existing `dir_of` filter, apply `go_same_package_visible(caller_profile, cand)`.
   - exactly 1 survivor → `Exact` with the EXISTING `ResolutionKind::SamePackage` (do NOT mint a
     new kind — avoids another tier-a label shift; codex confirmed Exact-vs-demoted is carried
     by confidence/score, not kind alone).
   - ≥2 survivors → demote exactly as today (over the SURVIVOR set, not the raw set).
   - 0 survivors with raw same-dir candidates NON-empty [B2] → **counted DROP** (a labeled
     `DropReason`/counter, NOT fall-through): codex verified R5 promotes a unique remaining free
     function to `FreeSingle` Exact (src/resolution.rs ~:2026), so falling through would
     re-promote the very definition the predicate just excluded (e.g. the foo_test-only case) as
     a false Exact. Raw same-dir candidates EMPTY (today's situation) → fall through exactly as
     today (no behavior change for that path). Test both: (a) caller `package foo`, only
     `package foo_test` same-dir candidates, NO other definitions repo-wide → dropped (never
     FreeSingle-Exact to the foo_test def); (b) raw-empty → unchanged ladder behavior.
b. **`resolve_go_bare_value_ref`** — `src/resolution.rs` ~292-324 (used by
   `apply_go_registration_candidate` in call_graph.rs). Apply the same filter before the
   `same_pkg.len() == 1` gate. ALSO: the `same_pkg.len() > 1 → None` path is a SILENT drop today
   — add a counter (S5) whether or not the filter rescues it.
c. **`go_receiver_index` typed-fact lanes** — `extract_go_return_types` and
   `extract_go_package_vars` (src/go_receiver_index.rs) feed S1/S3 receiver recovery, which
   routes through owner_lookup/interface dispatch to Exact method edges — so namespace-only
   keying is NOT sufficient [B3]: a caller in `x_windows.go` could recover a receiver type from
   a `newT` fact defined only in `x_linux.go` and mint an impossible Exact. Required shape:
   **store each typed fact WITH its defining file** (e.g. values become
   `(type, defining_file)` entries, still BTreeMap-deterministic), keep entries per
   `(dir, package_clause, name)` key, and at consult time
   (`classify_go_receiver_expanded` / `classify_nested_selector` — and the rematerialization
   pass consuming these) filter entries by `go_same_package_visible(caller_profile,
   defining_file_profile)`, then require **exactly one surviving DISTINCT type** (>1 distinct →
   bail as today; 0 → no recovery). Same treatment for package vars. The existing
   ambiguous-drop-whole-key behavior is thereby replaced by consult-time filtering — the
   extraction gate keeps entries it would previously have dropped ONLY when they differ by
   defining-file profile (otherwise dedup as today).
d. **OUT OF SCOPE — counted, not fixed [M1, accepted option]**: the `GoOwnerIdentity`-keyed
   S2/S4 lanes (`go_field_types`, `struct_embeds`, embedded-interface routes in
   src/type_providers/go.rs — GoOwnerIdentity is `(package_dir, name)` and cannot distinguish
   `foo` from `foo_test` in one dir) and the bare-name `structs`/`interfaces` last-file-wins
   maps. Do NOT re-key GoOwnerIdentity in this task (blast radius: the whole P11 lane +
   serialization). Instead: add cheap overwrite/conflict counters where the colliding
   definitions have differing package clause or build profile (S5), and leave a documented
   follow-up note (docs comment at GoOwnerIdentity + PR body) that field_typed /
   interface-dispatch Exact recovery can still cross build partitions — a measured, named gap.

### S4 — Cache

One transition: `CACHE_VERSION` 38→39 (src/cpg_cache.rs; doc-comment entry "v39: Go build-profile
same-package partitioning") and `NAV_CALL_EDGE_CACHE_VERSION` 8→9 (src/navigation/
call_edge_cache.rs). Rename the pin tests accordingly (`cache_version_is_39_...`,
`sidecar_version_is_9`). NEVER re-bump during fix waves (pipeline lesson 10).

### S5 — Telemetry (call-stats)

New counters surfaced in `prism nav call-stats` (src/navigation/queries.rs; follow the existing
counter plumbing, e.g. the callback_registration_* pattern):
- `go_pkg_clause_partition_exact` — R4.5 collisions resolved to Exact by the predicate.
- `go_build_partition_exact` — ditto where the build-SAT rung was decisive.
- `go_same_pkg_all_filtered_drop` — the [B2] raw-nonempty/0-survivor counted drop.
- `go_bare_value_ref_ambiguous` — the previously-silent §S3b drop path (counted regardless).
- `go_build_expr_unparsed` — parse failures / multiple-`//go:build` / bound-exceeded.
- `go_owner_identity_profile_conflict` — §S3d [M1] counted-not-fixed class (S2/S4 lanes).

## 2. Tests (TDD; failing-first where feasible)

- Unit: profile extraction (suffix rules incl. `_test` stripping, bare `linux.go` NOT
  constrained, `//go:build` precedence over `+build`, first-before-package-clause only).
- Unit: SAT predicate — GOOS exclusion (`linux` vs `windows` → incompatible), `unix` alias vs
  `windows` (incompatible) and vs `linux` (compatible), **matchTag ALIASES [B1]:
  `x_linux.go` vs `x_android.go` COMPATIBLE (GOOS=android satisfies both), `//go:build linux`
  vs filename `_android.go` compatible, `illumos`/`solaris` compatible, `ios`/`darwin`
  compatible, and `unix` vs `_android.go` compatible (android ∈ unixOS)**, `X` vs `!X`
  incompatible, the 3-way prometheus labels family (pairwise incompatible), free-tag
  `cluster_proxy` pair, parse-failure → compatible, bound-exceeded → compatible, multiple
  `//go:build` lines → unparsed/compatible + counted.
- Resolution: R4.5 clause partition (foo/foo_test two-file fixture → Exact for foo caller;
  foo_test-candidates-only → falls through, NOT demoted); suffixed-caller Exact vs
  unsuffixed-caller still-demoted (the etcd TryLockFile shape); go:build complement pair Exact.
- §S3b filter + counter; §S3c profile-filtered typed facts [B3]: (i) `x_windows.go` caller does
  NOT recover a receiver type whose only defining fact lives in `x_linux.go` (no Exact method
  edge — failing-first); (ii) a `foo_test`-defined `newT` fact is not consumed by a `foo`
  caller; (iii) two facts for one key differing only by build profile → a suffixed caller
  compatible with exactly one recovers it (positive control), an unsuffixed caller compatible
  with both bails (>1 distinct type).
- Incremental parity (P11 precedent, bidirectional): edit a `//go:build` line → incremental
  rebuild resolution == full-build resolution; revert likewise.
- Eval fixtures (`eval/fixtures/go/`, flat snake_case dirs, follow an existing `expected.toml`
  shape — read `same_pkg_free_fn/` first): `pkg_clause_partition`, `build_suffix_partition`,
  `gobuild_expr_partition`. Register per conventions; run
  `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` (0
  regressions required) and `uv run pytest` in eval/ if fixtures touch harness inputs.

## 3. Acceptance / metrics (report measured numbers)

- zap: callers of `common_test.go:37` `withLogger` — `global_test.go` site 244 (and the other
  `package zap` caller sites) flip `same_package` 0.6 → Exact 1.0; the `zap_test` definition
  gets NO callers from `package zap` files.
- Before/after `prism nav call-stats` on zap, prometheus, etcd, caddy (use `--no-cache` or a
  scratch `--cache-dir`; the shared nav cache must not be poisoned mid-task): report
  `same_package` exact/nameonly deltas + the new counters. Expect modest Exact gains (platform
  shims are usually called from unsuffixed files, which correctly STAY demoted — that is the
  design working, say so in the report).
- Full `cargo test` green; `cargo fmt` clean; no NEW warnings (one pre-existing unused-import
  class in go.rs tests predates this branch).
- Matrix: 0 regressions (`--matrix-only`). Quick M2 exact-tier comparison vs a pre-change run on
  the prism corpus if time permits (harness: eval/README.md).

## 4. Global constraints (binding)

- Precision floor: Rust/Go drop-not-fanout unchanged; nothing below Exact feeds asserted
  findings; `Exact` is minted ONLY from the proven-singleton path above.
- No file exclusion anywhere (recall-advantage guard, §0.5).
- `BTreeMap`/`BTreeSet` for determinism; 1-indexed lines; files <600 lines (split modules).
- One shared predicate implementation; the S3 consult sites call IT (no re-derived copies).
- Commit trailer: `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`. Small commits,
  failing-first tests. Report file: task-p13-report.md next to this brief (full detail there;
  return ≤15-line summary).

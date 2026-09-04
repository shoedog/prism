# Prism Phase 0 — SARIF output, `prism targets`, `prism::api` facade, README truth pass — design

**Status:** DRAFT v1 (2026-09-04) — for sol spec review round 1 (declared cap: **2 rounds**).
**Recorded:** 2026-09-04 · **Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9` (`origin/main`, PR #225 merge).
**Scope:** the four "Phase 0" items of the tooling roadmap (`~/code/tools/04-prism-plan-roadmap.md` §2, `03-tooling-plan-roadmap.md` §3 Phase 0): the interfaces the analyzer roadmap needs from prism before any analyzer is written. All four are additive serializers and a facade; **no CPG/cache/resolution change and no cache version bump**.
**Grounding:** `~/code/tools/grounding/finding-inventory.md` (every `SliceFinding` construction site), `grounding/cli-output-api.md` (CLI branches, private main.rs helpers, build identity, nav API, test conventions), `grounding/readme-truth.md` (README claims vs code). Line anchors below are hints against the exact base; symbols are the authority (pipeline-lessons #6).
**Contracts:** `~/code/tools/contracts/targets.schema.json` (v1) is the authority for the `prism targets` document shape. A copy is vendored into the repo by this work (§5.6).

---

## 1. Problem (measured)

1. Findings cannot reach CI today except through prism's own `json`/`review` shapes. There is no SARIF, so GitHub code scanning (the roadmap's stand-in findings plane) cannot ingest prism at all. `SliceFinding` (`src/slice.rs:23-43`) already carries every field SARIF needs.
2. The runtime harness (roadmap Phase 1) needs a stable, versioned "instrumentation targets" document. Nothing projects findings into one; the closest analogue, `CallersOutput` (`src/output/review.rs:264-291`), is `Serialize`-only and shaped for a different consumer.
3. Downstream crates can `use prism::*` (the whole crate is `pub`, no `[lib]` section, no semver statement — `grounding/cli-output-api.md` §10), but the review pipeline that actually produces findings is private to `src/main.rs`: `run_algorithm` (`:1223`, the only place per-algorithm configs are assembled — `Chop`, `ConditionedSlice`, `DeltaSlice` return empty results without it), `annotate_finding_parse_quality` (`:1201`), and the ~200-line diff → parse → type_db → CPG-cache orchestration (`:724-926`). An analyzer cannot reproduce prism's own findings without copying main.rs.
4. `--format` is an unvalidated `String` (`src/main.rs:66-68`): an unknown value silently renders `text`; in the multi-algorithm branch even `paper` degrades to text (`:1095`).
5. README is stale in 12 places (`grounding/readme-truth.md`: 8 STALE, 4 WRONG), most visibly ~40 `slicing …` examples for a binary named `prism`, "six" MCP tools (there are eight), three output formats (there are six), and a `tests/fixtures/cve/` directory that does not exist.
6. Confidence is a dead letter for findings: `ResolutionConfidence` (`src/resolution.rs:26`) lives on `CpgEdge::Call/Return`, only `barrier_slice.rs:107` reads it, and no finding-emitting algorithm consults it (`grounding/finding-inventory.md` §2, §5.7). DataFlow edges carry no confidence at all (roadmap item 2, out of scope here). Any confidence label Phase 0 emits must therefore be derived from what is *knowable* today, and must not claim more.

## 2. Design

### 2.1 Shared finding classification — `src/finding_confidence.rs` (new, single source)

The doctrine ("nothing below Exact feeds an asserted finding") needs one function that both new serializers consult. Phase 0 can only know two things about a finding's evidence path: whether the algorithm used the CPG at all (`SlicingAlgorithm::needs_cpg()`, `src/slice.rs:211`), and the file's parse quality. Everything else is unlabeled today, so the label says so.

```rust
//! Single source for the confidence/tier label attached to a finding by the
//! SARIF and targets serializers. Phase 0 rule (roadmap 04 §3.4): AST-only
//! algorithms are Exact by construction unless the parse is not clean; any
//! CPG-derived finding is Nominal because DataFlow edges carry no confidence
//! yet. Item 2 (reaching-definitions labeling) replaces the CPG arm with a
//! min over the evidence path; the public shape here does not change.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingConfidence { Exact, Scoped, Nominal }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingTier { Asserted, Candidate }

/// The highest resolution tier this build can label. Phase 0: always "nominal".
pub const RESOLUTION_MODE: &str = "nominal";

pub fn classify(finding: &SliceFinding) -> (FindingConfidence, FindingTier)
```

Rules (binding; each has a test in §7.1):
- `SlicingAlgorithm::from_str(&finding.algorithm)` is `None` → `(Nominal, Candidate)` (fail-safe for unknown producers; every production algorithm string — `absence`, `callback_dispatcher`, `contract`, `echo`, `membrane`, `peer_consistency`, `primitive`, `provenance`, `symmetry`, `taint` — round-trips through `from_str`, pinned by test §7.1.4).
- `needs_cpg()` → `Nominal`; else `Exact`. `Scoped` is reserved for item 2 and is never produced in Phase 0.
- `parse_quality` is `None` (clean; `annotate_finding_parse_quality` only writes degraded/poor/unparseable) or `Some("clean")` → tier is `Asserted` iff confidence is `Exact`; any other `parse_quality` → `Candidate`.
- `classify` is pure and total; it never reads the CPG.

### 2.2 SARIF 2.1 output — `src/output/sarif.rs` (new) + `--format sarif`

Hand-rolled typed structs (no new dependency; the SARIF subset is ~12 structs). Field order in the structs is the serialized key order (serde), so output is deterministic given deterministic input ordering (§2.2.4).

**2.2.1 Document shape.**

```json
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [{
    "tool": { "driver": {
      "name": "prism",
      "version": "<CARGO_PKG_VERSION>",
      "semanticVersion": "<CARGO_PKG_VERSION>",
      "informationUri": "https://github.com/shoedog/prism",
      "rules": [ { "id": "prism/echo/missing_error_handling", "name": "missing_error_handling",
                   "shortDescription": { "text": "echo: missing_error_handling" },
                   "properties": { "algorithm": "echo", "category": "missing_error_handling" } } ]
    } },
    "invocations": [{ "executionSuccessful": true,
                      "toolExecutionNotifications": [ { "level": "error", "message": { "text": "Chop: --chop-source required for chop algorithm" } },
                                                      { "level": "warning", "message": { "text": "<parse warning verbatim>" } } ] }],
    "originalUriBaseIds": { "%SRCROOT%": { "uri": "file:///abs/repo/root/" } },
    "results": [ { ...see 2.2.2... } ],
    "properties": { "algorithms_run": ["EchoSlice", "AbsenceSlice"], "resolution_mode": "nominal",
                    "cache_build_identity": "<sha256>", "prism_git_sha": "<GIT_SHA>", "binary_input_dirty": false }
  }]
}
```

- `toolExecutionNotifications` is omitted when empty; `executionSuccessful = errors.is_empty()`.
- `originalUriBaseIds` uses the canonicalized repo root with a trailing `/`; it is the only machine-specific value and tests never pin it.
- `rules` holds one entry per distinct `ruleId` present in `results`, sorted by id; `results[i].ruleIndex` indexes into it.

**2.2.2 Result mapping** (one `result` per `SliceFinding`; nothing is dropped):

| SARIF | from |
|---|---|
| `ruleId` | `format!("prism/{}/{}", finding.algorithm, finding.category.as_deref().unwrap_or("uncategorized"))`. All 27 production sites set a category; `uncategorized` exists for forward compatibility (`grounding/finding-inventory.md` §4). |
| `level` | severity map: `concern → "error"`, `warning → "warning"`, `suggestion → "note"`, `info → "note"`, anything else → `"none"` (fail-safe; 4 severities exist, not the 3 the struct comment claims — §5.4). |
| `message.text` | `finding.description` verbatim. |
| `locations[0].physicalLocation.artifactLocation` | `{ "uri": finding.file, "uriBaseId": "%SRCROOT%" }` — `finding.file` is already repo-relative with forward slashes (it is the diff path). |
| `locations[0].physicalLocation.region` | `{ "startLine": finding.line }` when `line >= 1`; the `region` key is **omitted** when `line == 0` (SARIF requires `startLine >= 1`; `symmetry_slice.rs:230` has a dead `unwrap_or(0)` fallback — §7.2.6 pins the omission). |
| `locations[0].logicalLocations` | `[ { "name": function_name, "kind": "function" } ]` when `function_name` is `Some`; omitted otherwise. |
| `relatedLocations` | one entry per `related_lines[i]` (same file, `region.startLine = related_lines[i]`, `id = i`), then one per `related_files[j]` (that file, no region, `id = related_lines.len() + j`). Omitted when both are empty. |
| `partialFingerprints` | `{ "prism/finding/v1": sha256(algorithm|category|file|function_name|description) }` — deliberately excludes `line` so a pure line shift does not change identity; `description` may embed line numbers for some categories (documented limitation, §9). |
| `properties` | `{ "algorithm", "category", "severity", "confidence", "tier", "resolution_mode", "parse_quality"?, "function_name"?, "related_files"? }` — `confidence`/`tier` from `finding_confidence::classify`; optional keys omitted when absent/empty. |

Diagrams (`finding.diagrams`) are not serialized into SARIF (they are prism-specific; the `json` format keeps them).

**2.2.3 CLI wiring** (`src/main.rs`):
- `--format` gains `value_parser = ["text", "json", "paper", "review", "callers", "mermaid", "sarif"]`. An unknown format is now a clap error (exit 2) instead of silently rendering text. This is a deliberate fix of problem §1.4; the multi-run `paper` gap is **not** fixed here (follow-up, §9).
- A `"sarif"` arm is added to **both** `match cli.format.as_str()` sites (multi at `:999`, single at `:1124`). Both call one helper `emit_sarif(&SarifInputs { findings: &all_findings, errors: &all_errors, parse_warnings: &parse_warnings, algorithms_run: &algorithms_run, repo })` **after** `annotate_finding_parse_quality` has run (it runs at `:997` and `:1122`; SARIF must see the annotated findings or `parse_quality` is always absent — `grounding/finding-inventory.md` §5.3). Single-run wraps its one result exactly as the `mermaid` arm does.
- Trailer identical to the `json` arm: `emit_warnings_to_stderr` + `determine_exit_code`.
- Output: `serde_json::to_string_pretty` + `\n` to stdout, same as `json`.

**2.2.4 Ordering.** `results` are sorted by `(file, line, ruleId, message.text)` with a stable sort before serialization, independent of algorithm run order. `rules` sorted by `id`. `relatedLocations` keep source order (already deterministic: `related_lines` come from BTreeSets or ordered walks).

**2.2.5 Byte-pinning.** `json`, `review`, `text`, `paper`, `mermaid`, `callers` are untouched (§8 gate 5 proves it with a same-base byte control). SARIF is a new serializer with its own structural tests; **no byte-pinned golden for multi-algorithm SARIF** (Taint is not byte-stable on some fixtures — `tests/cli/nav_compat_test.rs` header).

### 2.3 `prism::api` facade — `src/api/` (new; `pub mod api` in `src/lib.rs`)

**Decision (roadmap 04 §2.3):** no workspace split. `prism::api` is the first and only module with a stated compatibility promise; the rest of the crate stays `pub` but is documented as internal.

**2.3.1 Compatibility promise** (module docs, README §Library):
> `prism::api` is semver-stable within a major version: items are only added; a removal or signature change is preceded by a `#[deprecated]` release. Everything outside `prism::api` is internal and may change in any release. Output *formats* (`json`, `review`, SARIF, targets) are versioned separately by their own `version`/`schema_version` fields.

**2.3.2 Surface** (`src/api/mod.rs` re-exports; each in its own file to respect the 600-line cap):

```rust
// src/api/build_info.rs
pub struct BuildInfo {
    pub package_version: &'static str,      // env!("CARGO_PKG_VERSION")
    pub git_sha: &'static str,              // env!("GIT_SHA")
    pub cache_build_identity: &'static str, // cpg_cache::current_cache_build_identity()
    pub binary_input_dirty: bool,           // cpg_cache::binary_input_dirty()
    pub grammar_fingerprint: &'static str,  // env!("GRAMMAR_FINGERPRINT")
}
pub fn build_info() -> BuildInfo;
// Rationale: env!() in a downstream crate reads the downstream crate's values
// (grounding §3); the facade is the only place these are correct.

// src/api/review.rs — the review pipeline, lifted out of main.rs
pub struct ReviewOptions {
    pub repo: PathBuf,
    pub files_filter: Option<HashSet<String>>,   // --files
    pub compile_commands: Option<PathBuf>,       // --compile-commands
    pub scoped_cpg: bool,                        // --scoped-cpg
    pub cache_dir: Option<PathBuf>,              // --cache-dir (CPG cache; review path)
    pub no_cache: bool,                          // --no-cache
    pub language_versions: Vec<(Language, LanguageVersion)>, // --python-version …
}
impl ReviewOptions { pub fn new(repo: impl Into<PathBuf>) -> Self }  // all else default/empty

/// Everything the CPG borrows: parsed files, sources, type db, diff, parse quality.
pub struct ReviewInputs {
    pub files: BTreeMap<String, ParsedFile>,
    pub sources: BTreeMap<String, String>,
    pub type_db: Option<TypeDatabase>,
    pub diff: DiffInput,
    pub parse_warnings: Vec<String>,
    pub parse_quality: BTreeMap<String, FileParseQuality>,
    pub scope_graph_inputs: ScopeGraphBuildInputs,
}
pub fn load_review_inputs(opts: &ReviewOptions, diff_text: &str) -> Result<ReviewInputs>;
//  = main.rs :724-816 verbatim behaviour: JSON-or-unified diff detection, --files filter,
//    per-diff-file parse (unsupported languages warned to stderr and skipped — UNCHANGED),
//    TypeDatabase auto/explicit, parse-quality computation, scope-graph inputs.

/// Builds the CpgContext with the same cache decision tree as main.rs :819-926
/// (use_cache = cache_dir.is_some() && !no_cache && !scoped_cpg; Hit / PartialHit /
/// Miss / scoped), applies language_versions, and saves the cache on non-Hit paths.
pub fn build_context<'a>(inputs: &'a ReviewInputs, opts: &ReviewOptions) -> Result<CpgContext<'a>>;

/// Per-algorithm parameters that main.rs hand-plumbs from ReviewArgs today.
#[derive(Debug, Clone, Default)]
pub struct AlgorithmParams {
    pub barrier_depth: usize, pub barrier_symbols: Vec<String>,
    pub chop_source: Option<String>, pub chop_sink: Option<String>,   // "file:line"
    pub taint_sources: Vec<String>, pub taint_return_flow: bool,
    pub condition: Option<String>,
    pub old_repo: Option<PathBuf>,
    pub spiral_max_ring: usize,
    pub quantum_var: Option<String>,
    pub peer_pattern: Option<String>,
    pub layers: Option<String>,
    pub concern: Option<String>,
    pub temporal_days: usize,
}
// Default values are the clap defaults from ReviewArgs (barrier_depth, spiral_max_ring,
// temporal_days copied verbatim from src/main.rs arg attributes) so library callers and
// the CLI agree. A test pins them (§7.3.3).

/// main.rs::run_algorithm, moved. Identical match arms; main.rs calls this.
pub fn run_algorithm(algorithm: SlicingAlgorithm, ctx: &CpgContext, inputs: &ReviewInputs,
                     config: &SliceConfig, params: &AlgorithmParams, repo: &Path) -> Result<SliceResult>;

pub struct ReviewRun {
    pub results: Vec<SliceResult>,          // in algorithm order
    pub findings: Vec<SliceFinding>,        // flattened, parse-quality annotated
    pub errors: Vec<AlgorithmError>,
    pub algorithms_run: Vec<String>,        // SlicingAlgorithm::name()
}
/// Runs each algorithm (errors collected, not fatal — same as the multi-run branch),
/// flattens findings and annotates parse quality (annotate_finding_parse_quality, moved).
pub fn run_review(ctx: &CpgContext, inputs: &ReviewInputs, algorithms: &[SlicingAlgorithm],
                  config: &SliceConfig, params: &AlgorithmParams, repo: &Path) -> ReviewRun;

/// One-shot convenience for analyzers: installs the build pool, loads, builds, runs.
pub fn review(opts: &ReviewOptions, diff_text: &str, algorithms: &[SlicingAlgorithm],
              config: &SliceConfig, params: &AlgorithmParams) -> Result<ReviewRun>;

pub fn parse_algorithms(spec: &str) -> Result<Vec<SlicingAlgorithm>>;   // "review" | "all" | "a,b,c" | "a" — main.rs :687-704, moved
pub fn annotate_finding_parse_quality(findings: &mut [SliceFinding], files: &BTreeMap<String, ParsedFile>);

// src/api/nav.rs — navigation session and the two queries analyzers need
pub struct NavOptions { pub no_cache: bool, pub cache_dir: Option<PathBuf> }
pub fn nav_session(repo: &Path, opts: &NavOptions) -> Result<NavigationSession>;  // main.rs::build_session, moved
pub enum Seed<'a> { Symbol(&'a str), Location(&'a str) /* "file:line" */, SymbolInFile { symbol: &'a str, file: &'a str } }
pub fn callers(s: &NavigationSession, seed: Seed, depth: usize, exact_only: bool) -> Result<Evidence, QueryError>;
pub fn callees(s: &NavigationSession, seed: Seed, depth: usize, exact_only: bool) -> Result<Evidence, QueryError>;
// Thin wrappers over navigation::queries::{callers,callees}_with_confidence. Typed call edges
// (FunctionId, ResolutionConfidence, ResolutionKind) stay pub(crate) — exposing them is a real
// API decision deferred to item 2/3 (§9).

// src/api/mod.rs re-exports, plus:
pub use crate::build_pool::install as with_build_pool;   // callers of the two-phase API must run inside it
pub use crate::finding_confidence::{classify, FindingConfidence, FindingTier, RESOLUTION_MODE};
pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::{project, TargetsDocument, TargetsMeta};
```

**2.3.3 main.rs becomes the first consumer.** `run_review` in main.rs is reduced to: parse args → `ReviewOptions`/`AlgorithmParams`/`SliceConfig` from `ReviewArgs` → `api::load_review_inputs` → `api::build_context` → `--format callers` short-circuit (unchanged) → `api::run_review` (multi) or `api::run_algorithm` + annotate (single) → the existing format `match` arms unchanged. The private helpers `run_algorithm`, `annotate_finding_parse_quality`, `parse_file_line` (kept private in api, used by `run_algorithm`), `build_session` are moved, not copied (doctrine 6: no second copies). `determine_exit_code`, `emit_warnings_to_stderr`, `parse_diagram_cap` stay in main.rs (CLI policy, not library).

**2.3.4 Lifetimes.** `CpgContext<'a>` borrows `files`/`type_db` (`src/cpg/context.rs:39-55`). The facade therefore exposes the two-phase `ReviewInputs` → `build_context(&inputs)` shape rather than a self-referential session; the one-shot `review()` hides it for callers who only need `ReviewRun`.

**2.3.5 Build pool.** `load_review_inputs`, `build_context` and `run_review` document that they must run inside `prism::build_pool::install` (deep files overflow the default stack — grounding §2). `review()` installs it itself. `nav_session` does the same as main.rs today (called inside `install` by `main`).

### 2.4 `prism targets` — `src/targets.rs` (new) + `Command::Targets` in main.rs

**2.4.1 CLI.**
```
prism targets --repo <dir> --diff <patch|json> [--algorithm echo,absence,contract,provenance,membrane]
              [--files a.py,b.py] [--compile-commands cc.json] [--scoped-cpg] [--cache-dir D | --no-cache]
              [--old-repo <dir>] [--min-severity info|suggestion|warning|concern] [--min-tier asserted|candidate]
              [--out <path>] [--format json]
```
- Defaults: `--algorithm echo,absence,contract,provenance,membrane` (the roadmap's list minus `angle`, which constructs no findings today — `grounding/finding-inventory.md` §5.1; `angle` is still accepted and yields no targets, with a stderr note); `--min-severity info`; `--min-tier candidate` (everything); `--format json` with `value_parser = ["json"]`.
- Algorithms needing parameters the subcommand does not take (`chop`, `conditioned`) → error before any work: `targets: algorithm Chop requires --chop-source/--chop-sink; use the top-level command`. `delta` and `contract` accept `--old-repo`; `taint` runs with `taint_from_diff = true`; everything else runs with `AlgorithmParams::default()`.
- Implementation: `TargetsArgs` (own clap struct — not a flatten of `ReviewArgs`, so `--format text` and `--list-algorithms` cannot leak in) → `api::review(...)` → `targets::project(...)` → pretty JSON + `\n` to stdout or `--out`. Exit 0 on success even with algorithm errors (they are recorded in `errors`); exit 1 on load/build failure; exit 2 on clap errors.

**2.4.2 Projection** (`pub fn project(findings: &[SliceFinding], files: Option<&BTreeMap<String, ParsedFile>>, meta: &TargetsMeta) -> TargetsDocument`) — pure, total, deterministic. `TargetsDocument`/`Target`/`Site`/… are `Serialize + Deserialize` structs mirroring `targets.schema.json` v1 exactly (field order = schema property order; optional fields `skip_serializing_if`).

Per-finding mapping (category → `kind` / `expected.property`; `dependency_hint` parsed **best-effort** from the description with a category-specific regex — a non-match omits the hint, never guesses):

| algorithm / category | kind | expected.property | dependency_hint |
|---|---|---|---|
| echo / `missing_error_handling` | `external_call` | `error_handled` | `callee` = 2nd single-quoted token of `'{caller}' calls '{callee}' without handling: …` |
| membrane / `unprotected_caller` | `boundary` | `error_handled` | `callee` = 1st quoted token of `unprotected call to '{callee}' from '{caller}'` |
| absence / `missing_counterpart`, `missing_close_on_error_path` | `resource_acquire` | `resource_released` | `counterpart` = word after `without ` (e.g. `close`, `unlock`, `free`); `kind` = `filesystem` if description starts with `file `, else omitted |
| absence / `close_only_on_error_path` | `resource_release` | `resource_released` | same parse |
| absence / `double_close` | `resource_release` | `resource_not_double_released` | `counterpart` = token before `()` in `… {close}() at line …` |
| contract / `contract_violation`, `contract_precondition_*`, `contract` | `contract` | `precondition_holds` | none |
| contract / `contract_postcondition*` | `contract` | `postcondition_holds` | none |
| provenance / `untrusted_origin` | `data_origin` | `origin_trusted` | `expected.detail` = origin word from `has {origin} origin:`; `kind` = `db` for `database`, `network` for `external_call`, else omitted |
| taint / `taint_source` | `data_origin` | `origin_trusted` | none |
| taint / `taint_sink`, `unquoted_expansion` | `other` | `not_reached_by_taint` | none |
| symmetry / `broken_symmetry` | `contract` | `counterpart_present` | `counterpart` = 2nd quoted token |
| peer_consistency / `peer_guard_divergence` | `contract` | `peer_consistent` | none |
| callback_dispatcher / any; primitive / any; anything unmatched | `other` | `unknown` | none |

Other fields: `site.file/line` = finding file/line; `site.symbol` = `function_name`; `site.function_start_line/end_line` looked up via `files[file].functions()` (`FunctionInfo.start_line/end_line`, `src/ast.rs:125-131`) for the function containing `line` when `files` is given, else omitted; `site.language` from `Language::from_path`; `category` verbatim; `source_algorithm` = `finding.algorithm`; `confidence`/`tier` = `finding_confidence::classify`; `severity`, `description`, `parse_quality` verbatim; `related.lines/files` verbatim (key omitted when both empty); `id` = hex sha256 of `"{file}|{line}|{symbol}|{algorithm}|{category}|{description}"` (symbol empty string when `None`). A finding with `line == 0` cannot be represented (the schema requires `line ≥ 1`): it is dropped from `targets` and recorded in `warnings` as `targets: dropped finding with line 0: <algorithm>/<category> in <file>` (the case is dead code today — `symmetry_slice.rs:230` — but the drop must be visible, never silent).

Document-level: `schema_version "1"`; `producer { tool: "prism", version: build_info().package_version, resolution_mode: RESOLUTION_MODE, cache_build_identity: build_info().cache_build_identity, algorithms: algorithms_run }`; `repo { root: canonical repo path, sha: git rev-parse HEAD via std::process::Command when the repo is a git checkout, else omitted }`; `diff { sha256 of the diff text, files: DiffInput file paths }`; `errors`/`warnings` from `ReviewRun` (omitted when empty). Targets: filtered by `--min-severity`/`--min-tier`, deduplicated by `id` (first wins), sorted by `(site.file, site.line, source_algorithm, category, id)`.

**2.4.3 Schema custody.** `contracts/targets.schema.json` is vendored to `docs/contracts/targets.schema.json` in this repo; the projection test (§7.4.5) validates an emitted document against it structurally (required keys, enum membership, id pattern) using a small hand-written checker — no `jsonschema` crate is added. `schema_version` bumps only on breaking changes; additive optional fields do not bump it.

### 2.5 README truth pass (docs only; no code)

Apply `grounding/readme-truth.md` row by row. Binding corrections:
1. Every `slicing …` CLI example → `prism …`; delete the "binary is named slicing for historical reasons / rename planned" paragraph.
2. MCP: eight tools (six `nav_*` + `taint_reaches` + `refresh_index`), matching `src/mcp/registry.rs::ToolRegistry::all_v1`.
3. Output formats: `text`, `json`, `paper`, `review`, `callers`, `mermaid`, `sarif` (with one-line descriptions and a SARIF upload snippet for GitHub code scanning); options table row updated.
4. `tests/fixtures/cve/` → the real locations (`tests/fixtures/c/cve_*.c`, `tests/fixtures/sanitizer-suite-*`, `eval/fixtures/`).
5. Framework list, nav subcommand list, language count, algorithm count, cache-scope paragraph (CPG cache per-diff and opt-in via `--cache-dir`; nav index whole-repo under `dirs::cache_dir()/prism/nav/<hash>/`), version/install — each to the truth column of the grounding table.
6. New sections: `prism targets` (with the schema pointer) and `Library use (prism::api)` with the compatibility promise verbatim from §2.3.1.
7. `skills/prism-code-slicing/SKILL.md` "Output formats" gains `sarif` and `targets`; `CLAUDE.md` gets `api`, `targets`, `finding_confidence`, `output/sarif.rs` in its module maps and the four-value severity vocabulary; `src/slice.rs:27` doc comment corrected to the four values.

**Explicitly not done:** the clap `name = "slicing"` (`src/main.rs:38`) is left as is — the `--version` line grammar `slicing <ver> (<sha>)` has two consumers (`tests/cli/version_test.rs:15` and `eval/tier_a/sut.py::parse_version`, per the IDENTITY GRAMMAR comment in `build.rs:45-56`); renaming it is a coordinated change (§9 follow-up).

## 3. Compatibility

- **Byte-identical**: `text`, `json`, `paper`, `review`, `callers`, `mermaid` for every existing invocation (gate §8.5). No `SliceFinding` field is added or renamed; no algorithm file is touched.
- **CLI grammar**: `--format` becomes validated (unknown → clap error); `targets` subcommand added; nothing removed. `--help` text changes only by the added format and subcommand.
- **Caches**: CPG cache version stays `55`; nav sidecar stays `24`; `SKIP_POLICY_VERSION` stays `2`. `build_context` reproduces the exact cache decision tree, so cache hit/miss behaviour for a given `--cache-dir` is unchanged (gate §8.6).
- **Library**: `prism::api` is new; existing `pub` paths are untouched (`run_slicing`, `run_slicing_inner`, `CpgContext::*`, `navigation::queries::*` keep their signatures).
- **MCP**: untouched.

## 4. Non-goals

DataFlow confidence / reaching definitions (item 2); SCIP rung (item 3); boundary nodes (item 4); workspace split; typed call-edge exposure through `api`; `angle`/`delta` findings; fixing the multi-run `paper` degradation; renaming the clap command; a structured `FindingHint` on `SliceFinding` (would touch 27 production sites + 5 test fixtures; description parsing is contained in `targets.rs` for v0); SARIF `codeFlows`/`threadFlows` from diagrams.

## 5. Failure directions (binding for reviewers)

1. **Confidence never over-claims.** Unknown algorithm string, any CPG use, or any non-clean parse → `nominal`/`candidate`. The safe direction is under-assertion; a false `asserted` would let a candidate into a required CI check.
2. **Targets never invent a hint.** A regex miss omits `dependency_hint`; the harness resolves from site text or reports the site as unreached with a reason. A wrong callee would inject a fault at the wrong site.
3. **SARIF never drops a finding silently.** Every finding becomes a result; the only drop is `line == 0` in **targets** (schema constraint), and it is recorded in `warnings`.
4. **The facade changes no behaviour.** Moving `run_algorithm`/`annotate_finding_parse_quality`/`build_session`/the load block is a move with identical bodies; the same-base byte control (§8.5) and cache control (§8.6) are the proof, not the diff.

## 6. Permitted implementation files

New: `src/finding_confidence.rs`, `src/output/sarif.rs`, `src/targets.rs`, `src/api/{mod,build_info,review,nav}.rs`, `tests/cli/sarif_test.rs`, `tests/cli/targets_test.rs`, `tests/integration/api_test.rs`, `tests/fixtures/targets/**` (small Python fixture + diff exercising echo/absence/contract/provenance/membrane), `docs/contracts/targets.schema.json`, `docs/superpowers/plans/2026-09-04-prism-phase0-*.md`, `docs/superpowers/handoffs/2026-09-04-prism-phase0-*.md`.
Modified: `src/lib.rs` (three `pub mod` lines), `src/output/mod.rs` (one `pub mod sarif;` + re-export), `src/main.rs` (format validation, sarif arms, `Targets` subcommand, helpers moved out), `src/slice.rs` (doc comment on `severity` only), `tests/cli/main.rs` and `tests/integration/main.rs` (module registrations), `tests/integration/coverage_test.rs` (only if a new test file must be listed — it should not: no algorithm tests are added), `README.md`, `CLAUDE.md`, `skills/prism-code-slicing/SKILL.md`.
**Forbidden:** `src/algorithms/**`, `src/cpg/**`, `src/cpg_cache.rs`, `src/navigation/**`, `src/resolution*.rs`, `src/call_graph.rs`, `src/ast.rs`, `Cargo.toml` dependencies (no new deps; `sha2` is already a dependency), any cache version constant. If compiled reality requires touching one of these, stop and amend this design.

## 7. Tests (TDD; each names its observable)

Run with `cargo test --test cli sarif_test::`, `cargo test --test cli targets_test::`, `cargo test --test integration api_test::`, and `cargo test --lib finding_confidence`.

### 7.1 `finding_confidence` (unit, in-module)
1. `absence` finding, `parse_quality: None` → `(Exact, Asserted)`.
2. `echo` finding, `parse_quality: None` → `(Nominal, Candidate)`.
3. `absence` finding, `parse_quality: Some("degraded")` → `(Exact, Candidate)`; `Some("clean")` → `(Exact, Asserted)`.
4. Every string in `["absence","callback_dispatcher","contract","echo","membrane","peer_consistency","primitive","provenance","symmetry","taint"]` round-trips through `SlicingAlgorithm::from_str` (pins the assumption that finding strings are parseable).
5. `algorithm: "not_an_algorithm"` → `(Nominal, Candidate)`.
6. Serde: `FindingConfidence::Exact` serializes to `"exact"`, `FindingTier::Candidate` to `"candidate"`.

### 7.2 SARIF (`tests/cli/sarif_test.rs`, structural, via `Command::cargo_bin("prism")`)
1. Single algorithm `--algorithm absence --format sarif` on a temp Python repo with an `open()` without `close()` (fixture from `write_repo` shape in `review_compact_test.rs`): stdout parses; `version == "2.1.0"`; `runs[0].tool.driver.name == "prism"`; exactly one rule `prism/absence/missing_counterpart`; that result has `level == "warning"`, `locations[0].physicalLocation.artifactLocation.uri == "a.py"`, `region.startLine == <open line>`, `properties.confidence == "exact"`, `properties.tier == "asserted"`, `properties.resolution_mode == "nominal"`, `ruleIndex == 0`.
2. Multi algorithm `--algorithm echo,absence --format sarif` on a fixture where a changed function's caller lacks error handling: a result with ruleId `prism/echo/missing_error_handling` has `properties.confidence == "nominal"` and `tier == "candidate"`; `runs[0].properties.algorithms_run == ["EchoSlice","AbsenceSlice"]`; results are sorted by `(uri, startLine, ruleId)` (assert the sequence of `(uri,startLine,ruleId)` tuples is non-decreasing).
3. `--algorithm chop --format sarif` without `--chop-source` in a multi list (`chop,absence`): `invocations[0].executionSuccessful == false`, one `toolExecutionNotifications[]` entry with `level == "error"` whose text contains `--chop-source required`; the absence result is still present.
4. `--format sarif` twice on the same input → identical stdout bytes (determinism; single deterministic algorithm only).
5. `--format bogus` → exit code 2 and stderr contains `invalid value 'bogus'` (clap allow-list).
6. Unit test in `sarif.rs`: a `SliceFinding` with `line: 0` serializes without a `region` key; with `related_lines: [3,5]` and `related_files: ["b.py"]` → three `relatedLocations` with ids `0,1,2`, the third without `region`.
7. Unit test: severity map covers all four vocabulary values plus an unknown → `"none"`.
8. Same-base control test (§8.5) is a script, not a cargo test.

### 7.3 `prism::api` (`tests/integration/api_test.rs`, in-process)
1. `api::review(&ReviewOptions::new(repo), diff_text, &[AbsenceSlice], &SliceConfig::default(), &AlgorithmParams::default())` on the §7.2.1 fixture returns one finding with `category == Some("missing_counterpart")` and `parse_quality == None`.
2. Two-phase API: `with_build_pool(|| { let inputs = load_review_inputs(..)?; let ctx = build_context(&inputs, &opts)?; run_review(&ctx, &inputs, &[EchoSlice, AbsenceSlice], ..) })` returns `algorithms_run == ["EchoSlice","AbsenceSlice"]` and `errors.is_empty()`.
3. `AlgorithmParams::default()` equals the clap defaults: a test reads `barrier_depth`, `spiral_max_ring`, `temporal_days` from `prism --help`'s `[default: N]` annotations (or pins the literal values copied from `src/main.rs` with a comment naming the source line) — the assertion is that CLI and library defaults cannot drift silently.
4. `run_algorithm(Chop, …, &AlgorithmParams::default())` returns `Err` whose message contains `--chop-source required` (moved behaviour preserved).
5. `build_info()`: `package_version == env!("CARGO_PKG_VERSION")`, `cache_build_identity.len() == 64`.
6. `nav_session(repo, &NavOptions{no_cache:true, cache_dir:None})` then `callers(&s, Seed::Symbol("helper"), 1, false)` on a two-file Python fixture returns evidence whose JSON contains the caller function name.
7. A doc-test on `api::review` (the README snippet) compiles and runs.

### 7.4 `prism targets` (`tests/cli/targets_test.rs`)
1. Default run on `tests/fixtures/targets/` (Python: `svc.py` calls `fetch()` from `client.py` without handling → echo; `open()` without close → absence; a guard-clause edit → contract; a `request.args` read → provenance): stdout parses; `schema_version == "1"`; `producer.tool == "prism"`; `producer.resolution_mode == "nominal"`; `producer.algorithms` equals the five default names; at least one target per `source_algorithm ∈ {echo, absence}` (contract/provenance asserted only if the fixture reliably produces them — the implementer records which in the test comment, per the `review_compact_test.rs:145-158` convention).
2. The echo target: `kind == "external_call"`, `expected.property == "error_handled"`, `dependency_hint.callee == "fetch"`, `confidence == "nominal"`, `tier == "candidate"`, `site.symbol` is the caller function, `site.function_start_line <= site.line <= site.function_end_line`.
3. The absence target: `kind == "resource_acquire"`, `expected.property == "resource_released"`, `dependency_hint.counterpart == "close"`, `dependency_hint.kind == "filesystem"`, `confidence == "exact"`, `tier == "asserted"`.
4. `id` is 64 hex chars; running twice gives byte-identical stdout; changing `--min-tier asserted` removes every `tier == "candidate"` target and keeps the absence one.
5. Every emitted document passes the vendored-schema structural check (required keys per object, enum membership for `kind`, `expected.property`, `confidence`, `tier`, `severity`, `language`, id regex, `additionalProperties` sets) — implemented as a small checker in the test module over `docs/contracts/targets.schema.json`.
6. `--algorithm chop` → exit 1 and stderr contains `requires --chop-source`; `--algorithm angle` → exit 0, `targets == []`, stderr contains `angle: no findings are produced by this algorithm`.
7. `--out <file>` writes the same bytes as stdout would and prints nothing to stdout.
8. Unit tests in `targets.rs`: one per row of the mapping table using a hand-built `SliceFinding` with the verbatim description format from `grounding/finding-inventory.md` §1 (echo, membrane, four absence categories, provenance `database`/`user_input`, symmetry, an unknown category) asserting `kind`, `expected.property`, and the exact `dependency_hint`; plus a `line: 0` finding → dropped + warning text.

## 8. Acceptance gates (controller, before review and before PR)

1. `cargo fmt --all -- --check` → exit 0.
2. `cargo clippy --all-targets --all-features -- -D warnings` → exit 0 (or the pre-existing warning set, diffed against the exact base built in the same worktree — new warnings only are blockers).
3. Focused tests (§7) GREEN, and each new test observed RED on the exact base where applicable (`finding_confidence`, `sarif`, `targets`, `api` tests fail to compile on base — the RED observation is "does not compile / feature absent", recorded as such).
4. Full suite: `cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee <log>`; totals computed by `awk` over every `test result:` line of the complete log (never `tail`); base was `3543 passed / 0 failed / 1 ignored` (`~/code/tools/logs/baseline-c220525.log`). Expected: base totals + new tests, 0 failed.
5. **Same-base byte control (the facade proof):** script `scripts/phase0-byte-control.sh` (new, committed) runs both the base binary (built from `c220525c` in a separate worktree) and the branch binary over every checked-in fixture diff (`tests/fixtures/python/calc.diff`, `tests/fixtures/hapi-4552.diff`, `tests/fixtures/nav_compat/*`, `tests/fixtures/review_no_diagrams/*`, `tests/fixtures/c/*.diff` if present) for formats `text`, `json`, `paper`, `review` (single algorithm each: `leftflow`, `absence`, `contract`, `echo`, `membrane`, `provenance`, `primitive`) and `--format callers`, and `diff`s stdout byte-for-byte. Taint is excluded (documented non-byte-stability). Expected: zero differing files. Any difference is a STOP.
6. **Cache decision control:** with `--cache-dir <tmp>`: run 1 (Miss → save), run 2 (Hit), edit one file, run 3 (PartialHit) — assert via the cache dir's `cache-meta.json` mtime/`file_count` and stderr that the branch behaves identically to base on the same sequence.
7. Tier-A: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` → same pass count as base (CPG untouched; this is a cheap regression tripwire, not a measurement).
8. One real SARIF file validated against the official SARIF 2.1 JSON schema with a Python `jsonschema` check (controller-side, recorded in the handoff with the command and output).
9. Do not rebaseline Tier-A; a suite or corpus failure is attributable only after the exact base fails or passes the same probe in the same environment.

## 9. Follow-ups filed (not in this PR)

- Multi-run `paper` silently degrades to text (`src/main.rs:1095`) — add the arm or reject.
- Rename clap `name = "slicing"` → `prism` together with `tests/cli/version_test.rs:15` and `eval/tier_a/sut.py::parse_version` (IDENTITY GRAMMAR, `build.rs:45-56`).
- `angle_slice` and `delta_slice` emit no findings; both need finding construction before they can feed targets/SARIF.
- `provenance_slice` discards `origin_line/origin_file` (`grounding/finding-inventory.md` §5.5); surfacing them changes `json` bytes for provenance and needs its own byte-pinned PR.
- Structured `FindingHint` (`#[serde(skip)]`) populated at construction, replacing the description regexes in `targets.rs`.
- Typed call edges through `prism::api` (`IndexedIncomingCall` is `pub(crate)`), with item 2/3.
- SARIF `partialFingerprints` includes `description`, which embeds line numbers for some categories; a line-free fingerprint needs the structured hint above.
- Item 2 (DataFlow confidence + reaching definitions) will change `classify` to a min over the evidence path and add `--min-confidence`; the `FindingConfidence::Scoped` variant is reserved for it.

## 10. Implementation sequencing (each task PR-sized within the one branch; one PR at the end)

1. `finding_confidence.rs` + unit tests (§7.1).
2. `output/sarif.rs` + `--format` allow-list + both arms + `tests/cli/sarif_test.rs` (§7.2).
3. `api/` (build_info, review, nav) with main.rs consuming it; `tests/integration/api_test.rs` (§7.3); gate §8.5/§8.6 run here, before anything else builds on the facade.
4. `targets.rs` + `Command::Targets` + fixture + `tests/cli/targets_test.rs` (§7.4) + vendored schema.
5. README/CLAUDE.md/SKILL.md truth pass (§2.5) + doc-test.
6. Closeout: full gates §8, handoff, PR.

## 11. Review and convergence record

Design review cap: **2 rounds** (sol, read-only, via bridge). Round entries are appended here with verdicts (`FIX` / `SETTLED`) and the disclosed-extension rationale if the cap is extended by one converging round.

- Round 1 — pending.

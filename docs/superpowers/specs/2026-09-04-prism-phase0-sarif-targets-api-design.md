# Prism Phase 0 — SARIF output, `prism targets`, `prism::api` facade, README truth pass — design

**Status:** v2 (2026-09-04) — round-1 findings from the parallel seat (Opus, 9 WRONG / 15 SMELL) folded; sol round-1 pending; declared review cap **2 rounds** (owner authorised extension and a sol judge seat for disputed findings — see §11).
**Recorded:** 2026-09-04 · **Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9` (`origin/main`, PR #225 merge).
**Scope:** the four "Phase 0" items of the tooling roadmap (`~/code/tools/04-prism-plan-roadmap.md` §2, `03-tooling-plan-roadmap.md` §3 Phase 0): the interfaces the analyzer roadmap needs from prism before any analyzer is written. All four are additive serializers and a facade; **no CPG/cache/resolution change and no cache version bump**.
**Grounding:** `~/code/tools/grounding/finding-inventory.md` (every `SliceFinding` construction site), `grounding/cli-output-api.md` (CLI branches, private main.rs helpers, build identity, nav API, test conventions), `grounding/readme-truth.md` (README claims vs code). Line anchors below are hints against the exact base; symbols are the authority (pipeline-lessons #6).
**Contract custody:** `docs/contracts/targets.schema.json` **in this repo is authoritative** (it is what the tests read); `~/code/tools/contracts/targets.schema.json` is a mirror whose sha256 is recorded in the handoff at each sync (§2.4.3).

---

## 1. Problem (measured)

1. Findings cannot reach CI today except through prism's own `json`/`review` shapes. There is no SARIF, so GitHub code scanning (the roadmap's stand-in findings plane) cannot ingest prism at all. `SliceFinding` (`src/slice.rs:23-43`) already carries every field SARIF needs.
2. The runtime harness (roadmap Phase 1) needs a stable, versioned "instrumentation targets" document. Nothing projects findings into one; the closest analogue, `CallersOutput` (`src/output/review.rs:264-291`), is `Serialize`-only and shaped for a different consumer.
3. Downstream crates can `use prism::*` (the whole crate is `pub`, no `[lib]` section, no semver statement — `grounding/cli-output-api.md` §10), but the review pipeline that actually produces findings is private to `src/main.rs`: `run_algorithm` (`:1223`, the only place per-algorithm configs are assembled — `Chop`, `ConditionedSlice`, `DeltaSlice` return empty results without it), `annotate_finding_parse_quality` (`:1201`), and the ~200-line diff → parse → type_db → CPG-cache orchestration (`:724-926`). An analyzer cannot reproduce prism's own findings without copying main.rs.
4. `--format` is an unvalidated `String` (`src/main.rs:66-68`): an unknown value silently renders `text`; in the multi-algorithm branch even `paper` degrades to text (`:1095`).
5. README is stale in 12 places (`grounding/readme-truth.md`: 8 STALE, 4 WRONG), most visibly ~40 `slicing …` examples for a binary named `prism`, "six" MCP tools (there are eight), three output formats (there are six), and a `tests/fixtures/cve/` directory that does not exist.
6. Confidence is a dead letter for findings: `ResolutionConfidence` (`src/resolution.rs:26`) lives on `CpgEdge::Call/Return`, only `barrier_slice.rs:107` reads it, and no finding-emitting algorithm consults it (`grounding/finding-inventory.md` §2, §5.7). DataFlow edges carry no confidence at all (roadmap item 2, out of scope here). Any confidence label Phase 0 emits must therefore be derived from what is *knowable* today, and must say when it knows nothing.

## 2. Design

### 2.1 Shared finding classification — `src/finding_confidence.rs` (new, single source)

The doctrine ("nothing below Exact feeds an asserted finding") needs one function that both new serializers consult. Phase 0 can only know two things about a finding's evidence path: whether the algorithm used the CPG at all (`SlicingAlgorithm::needs_cpg()`, `src/slice.rs:211`), and the file's parse quality. Everything else is unlabeled today, so the label says exactly that.

```rust
//! Single source for the confidence/tier label attached to a finding by the
//! SARIF and targets serializers.
//! Phase 0 rule (roadmap 04 §3.4): AST-only algorithms are Exact by
//! construction; any CPG-derived finding is Unlabeled because DataFlow edges
//! carry no confidence yet. Item 2 (reaching-definitions labeling) replaces
//! the CPG arm with a min over the evidence path and starts producing
//! NameOnly; the public shape here does not change.
//!
//! `Asserted` is a claim about the EVIDENCE PATH (no unlabeled or
//! name-only edge, clean parse), never about the truth of the heuristic
//! the algorithm encodes. A required CI check may gate on `asserted`;
//! whether an asserted finding is a real defect is the reviewer's call.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingConfidence { Exact, NameOnly, Unlabeled }   // mirrors ResolutionConfidence + "unlabeled"

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingTier { Asserted, Candidate }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseQuality { Clean, Degraded, Poor, Unparseable, Unknown }

impl ParseQuality {
    /// ONLY valid for findings that passed through `api::annotate_finding_parse_quality`
    /// (every finding produced by the CLI or by `prism::api` has). `None` ⇒ Clean because the
    /// annotator writes only degraded/poor/unparseable. A hand-built finding must use `Unknown`.
    pub fn from_annotated(finding: &SliceFinding) -> Self;
}

/// The build's dataflow-labeling capability (roadmap 04 §3.6 `--resolution`): a DIFFERENT AXIS
/// from a finding's confidence. Phase 0: "nominal" = DataFlow edges unlabeled.
pub const RESOLUTION_MODE: &str = "nominal";

pub fn classify(algorithm: &str, parse_quality: ParseQuality) -> (FindingConfidence, FindingTier)
```

Rules (binding; each has a test in §7.1):
- `SlicingAlgorithm::from_str(algorithm)` is `None` → `(Unlabeled, Candidate)` (fail-safe for unknown producers; every production algorithm string — `absence`, `callback_dispatcher`, `contract`, `echo`, `membrane`, `peer_consistency`, `primitive`, `provenance`, `symmetry`, `taint` — round-trips through `from_str`, pinned by §7.1.4).
- `needs_cpg()` → `Unlabeled`; else `Exact`. `NameOnly` is reserved for item 2 and is never produced in Phase 0.
- Tier is `Asserted` iff confidence is `Exact` **and** `parse_quality == Clean`. `Unknown` parse quality is `Candidate` (the safe direction for absent information is under-assertion — W3).
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
      "name": "prism", "version": "<CARGO_PKG_VERSION>", "semanticVersion": "<CARGO_PKG_VERSION>",
      "informationUri": "https://github.com/shoedog/prism",
      "rules": [ { "id": "prism/echo/missing_error_handling", "name": "missing_error_handling",
                   "shortDescription": { "text": "echo: missing_error_handling" },
                   "fullDescription": { "text": "<one sentence per category from the table in sarif.rs; categories with two construction sites (contract_violation: guard modified | return behaviour modified) say so>" },
                   "properties": { "algorithm": "echo", "category": "missing_error_handling" } } ]
    } },
    "invocations": [{ "executionSuccessful": true,
                      "toolExecutionNotifications": [ { "level": "error", "message": { "text": "Chop: --chop-source required for chop algorithm" } },
                                                      { "level": "warning", "message": { "text": "<parse warning verbatim>" } } ] }],
    "originalUriBaseIds": { "%SRCROOT%": { "uri": "file:///abs/repo/root/" } },
    "results": [ { ...see 2.2.2... } ],
    "properties": { "algorithms_run": ["EchoSlice", "AbsenceSlice"], "resolution_mode": "nominal",
                    "errors": [ { "algorithm": "Chop", "error": "--chop-source required for chop algorithm" } ],
                    "cache_build_identity": "<sha256>", "prism_git_sha": "<GIT_SHA>", "binary_input_dirty": false }
  }]
}
```

- `toolExecutionNotifications` and `properties.errors` are omitted when empty; `executionSuccessful = errors.is_empty()`. `properties.errors` mirrors `AlgorithmError` so partial coverage is machine-readable (S14), the same way `targets.errors` is.
- **Machine/checkout-specific fields** (tests must ignore them; no golden may contain them): `originalUriBaseIds`, `tool.driver.version`, `tool.driver.semanticVersion`, `runs[0].properties.cache_build_identity`, `runs[0].properties.prism_git_sha` (carries `-dirty`), `runs[0].properties.binary_input_dirty` (W8).
- `rules` holds one entry per distinct `ruleId` present in `results`, sorted by id; `results[i].ruleIndex` indexes into it. Rule ids keep prism's category strings verbatim even though `primitive` uses SCREAMING_SNAKE ids next to snake_case categories — stability wins; the wart is recorded (S24).

**2.2.2 Result mapping** (one `result` per `SliceFinding`; nothing is dropped):

| SARIF | from |
|---|---|
| `ruleId` | `format!("prism/{}/{}", finding.algorithm, finding.category.as_deref().unwrap_or("uncategorized"))`. All 27 production sites set a category; `uncategorized` exists for forward compatibility (`grounding/finding-inventory.md` §4). |
| `level` | severity map: `concern → "error"`, `warning → "warning"`, `suggestion → "note"`, `info → "note"`, anything else → `"none"` (fail-safe; 4 severities exist, not the 3 the struct comment claims — §2.5.7). |
| `message.text` | `finding.description` verbatim. |
| `locations[0].physicalLocation.artifactLocation` | `{ "uri": finding.file, "uriBaseId": "%SRCROOT%" }` — `finding.file` is already repo-relative with forward slashes (it is the diff path). |
| `locations[0].physicalLocation.region` | `{ "startLine": finding.line }` when `line >= 1`; the `region` key is **omitted** when `line == 0` (SARIF requires `startLine >= 1`; `symmetry_slice.rs:230` has a dead `unwrap_or(0)` fallback — §7.2.6 pins the omission). |
| `locations[0].logicalLocations` | `[ { "name": function_name, "kind": "function" } ]` when `function_name` is `Some`; omitted otherwise. |
| `relatedLocations` | `related_lines` **sorted ascending and deduplicated**, then `related_files` **sorted and deduplicated** (S12): one entry per line (same file, `region.startLine`, `id = i`), then one per file (no region, `id = n_lines + j`). Omitted when both are empty. |
| `partialFingerprints` | `{ "prism/finding/v1": sha256(algorithm|category|file|function_name|masked_description) }` where `masked_description` replaces every maximal run of ASCII digits in `description` with `#` (W6: 13 of the 27 sites embed a line number in the description). `line` is excluded. Collision behaviour: two findings in one function whose descriptions differ only in digits share a fingerprint; GitHub code scanning keys alerts on fingerprint **plus** location, so they stay distinct alerts. Stated in the module docs. |
| `properties` | `{ "algorithm", "category", "severity", "confidence", "tier", "resolution_mode", "parse_quality"?, "function_name"?, "related_files"? }` — `confidence`/`tier` from `finding_confidence::classify(&finding.algorithm, ParseQuality::from_annotated(&finding))` (valid because every finding reaching the serializer has been annotated — §2.3.3); optional keys omitted when absent/empty. |

Diagrams (`finding.diagrams`) are not serialized into SARIF (they are prism-specific; the `json` format keeps them).

**2.2.3 CLI wiring** (`src/main.rs`):
- `--format` gains `value_parser = ["text", "json", "paper", "review", "callers", "mermaid", "sarif"]`. An unknown format is now a clap error (exit 2) instead of silently rendering text. **Compatibility note (S18):** values that previously "worked" by falling through to text — e.g. `-f Json`, `-f ""`, `-f txt` — now fail; §7.2.5 enumerates them. This is a deliberate fix of problem §1.4 and is disclosed in the PR; the multi-run `paper` gap is **not** fixed here (follow-up, §9).
- A `"sarif"` arm is added to **both** `match cli.format.as_str()` sites (multi at `:999`, single at `:1124`). Both call one helper `emit_sarif(&SarifInputs { findings, errors, parse_warnings, algorithms_run, repo })`. Multi-run passes `run.findings` (already annotated by `api::run_review`), `run.errors`, `run.warnings`, `run.algorithms_run`. Single-run passes `result.findings` (annotated by `api::run_algorithm`), `errors: &[]`, `result.warnings` (= parse warnings, set at `:1121`), `algorithms_run: &[algorithm.name()]` — **not** the `mermaid` arm's wrapper, which discards findings/warnings (S13).
- Trailer identical to the `json` arm: `emit_warnings_to_stderr` + `determine_exit_code`.
- Output: `serde_json::to_string_pretty` + `\n` to stdout, same as `json`.

**2.2.4 Ordering.** `results` are sorted by `(file, line, ruleId, message.text)` with a stable sort before serialization, independent of algorithm run order. `rules` sorted by `id`. `relatedLocations` sorted per §2.2.2. Determinism is therefore by construction, not by assumption about algorithm output order.

**2.2.5 Byte-pinning.** `json`, `review`, `text`, `paper`, `mermaid`, `callers` are untouched (§8.5 proves it with a same-base byte control covering single- and multi-algorithm runs). SARIF is a new serializer with its own structural tests; **no byte-pinned golden for multi-algorithm SARIF that includes Taint** (not byte-stable on some fixtures — `tests/cli/nav_compat_test.rs` header).

### 2.3 `prism::api` facade — `src/api/` (new; `pub mod api` in `src/lib.rs`)

**Decision (roadmap 04 §2.3):** no workspace split. `prism::api` is the first and only module with a stated compatibility promise; the rest of the crate stays `pub` but is documented as internal.

**2.3.1 Compatibility promise** (module docs, README §Library):
> `prism::api` is semver-stable within a major version: items are only added; a removal or signature change is preceded by a `#[deprecated]` release. Every struct and enum in `prism::api` is `#[non_exhaustive]`: construct with `new`/`Default` and assign public fields, never with a struct literal, so fields can be added without breaking you (S20). Everything outside `prism::api` is internal and may change in any release. Output *formats* (`json`, `review`, SARIF, targets) are versioned separately by their own `version`/`schema_version` fields.

**2.3.2 Surface** (`src/api/mod.rs` re-exports; each in its own file to respect the 600-line cap; all structs/enums `#[non_exhaustive]`):

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
    pub diff_text_sha256: String,                // hex sha256 of the diff text as read
    pub parse_warnings: Vec<String>,
    pub parse_quality: BTreeMap<String, FileParseQuality>,
    pub scope_graph_inputs: ScopeGraphBuildInputs,
}
/// = main.rs :724-816 verbatim behaviour: JSON-or-unified diff detection, --files filter,
///   per-diff-file parse (unsupported languages warned to stderr and skipped — UNCHANGED),
///   TypeDatabase auto/explicit, parse-quality computation, scope-graph inputs.
/// Runs inside build_pool::install (re-entrant; S19).
pub fn load_review_inputs(opts: &ReviewOptions, diff_text: &str) -> Result<ReviewInputs>;

/// Builds the CpgContext with the same cache decision tree as main.rs :819-926
/// (use_cache = cache_dir.is_some() && !no_cache && !scoped_cpg; Hit / PartialHit /
/// Miss / scoped), applies language_versions, and saves the cache on non-Hit paths.
/// Runs inside build_pool::install (S19).
pub fn build_context<'a>(inputs: &'a ReviewInputs, opts: &ReviewOptions) -> Result<CpgContext<'a>>;

/// Per-algorithm parameters that main.rs hand-plumbs from ReviewArgs today.
#[derive(Debug, Clone)]
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
impl Default for AlgorithmParams { /* the clap defaults from ReviewArgs (barrier_depth,
    spiral_max_ring, temporal_days) copied verbatim from src/main.rs arg attributes, with a
    comment naming the source; §7.3.3 pins them so CLI and library cannot drift silently */ }

/// main.rs::run_algorithm, moved. Identical match arms; then annotates the result's
/// findings with parse quality (idempotent) so no finding leaves the facade unannotated (W3).
pub fn run_algorithm(algorithm: SlicingAlgorithm, ctx: &CpgContext, inputs: &ReviewInputs,
                     config: &SliceConfig, params: &AlgorithmParams, repo: &Path) -> Result<SliceResult>;

pub struct ReviewRun {
    pub results: Vec<SliceResult>,          // in algorithm order
    pub findings: Vec<SliceFinding>,        // flattened, parse-quality annotated
    pub errors: Vec<AlgorithmError>,
    pub warnings: Vec<String>,              // = inputs.parse_warnings (W1)
    pub parse_quality: BTreeMap<String, FileParseQuality>,   // = inputs.parse_quality (W1)
    pub algorithms_run: Vec<String>,        // SlicingAlgorithm::name()
}
/// Runs each algorithm (errors collected, not fatal — same as the multi-run branch),
/// flattens findings (annotated). Runs inside build_pool::install.
pub fn run_review(ctx: &CpgContext, inputs: &ReviewInputs, algorithms: &[SlicingAlgorithm],
                  config: &SliceConfig, params: &AlgorithmParams, repo: &Path) -> ReviewRun;

pub struct ReviewOutcome { pub inputs: ReviewInputs, pub run: ReviewRun }
/// One-shot convenience for analyzers: installs the build pool, loads, builds, runs, and
/// returns BOTH the inputs (files, diff, sources — needed by targets::project) and the run (W1).
pub fn review(opts: &ReviewOptions, diff_text: &str, algorithms: &[SlicingAlgorithm],
              config: &SliceConfig, params: &AlgorithmParams) -> Result<ReviewOutcome>;

pub fn parse_algorithms(spec: &str) -> Result<Vec<SlicingAlgorithm>>;   // "review" | "all" | "a,b,c" | "a" — main.rs :687-704, moved
pub fn annotate_finding_parse_quality(findings: &mut [SliceFinding], files: &BTreeMap<String, ParsedFile>);

// src/api/nav.rs — navigation session and the two queries analyzers need
pub struct NavOptions { pub no_cache: bool, pub cache_dir: Option<PathBuf> }
/// main.rs::build_session, moved; runs inside build_pool::install (whole-repo parse is the
/// deepest-stack path in the crate — S19).
pub fn nav_session(repo: &Path, opts: &NavOptions) -> Result<NavigationSession>;
pub enum Seed<'a> { Symbol(&'a str), Location(&'a str) /* "file:line" */, SymbolInFile { symbol: &'a str, file: &'a str } }
pub fn callers(s: &NavigationSession, seed: Seed, depth: usize, exact_only: bool) -> Result<Evidence, QueryError>;
pub fn callees(s: &NavigationSession, seed: Seed, depth: usize, exact_only: bool) -> Result<Evidence, QueryError>;
// Thin wrappers over navigation::queries::{callers,callees}_with_confidence. Typed call edges
// (FunctionId, ResolutionConfidence, ResolutionKind) stay pub(crate) — exposing them is a real
// API decision deferred to item 2/3 (§9).

// src/api/mod.rs re-exports, plus:
pub use crate::build_pool::install as with_build_pool;   // still exported; nesting is safe (OnceLock pool, re-entrant install)
pub use crate::finding_confidence::{classify, FindingConfidence, FindingTier, ParseQuality, RESOLUTION_MODE};
pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::{project, TargetsDocument, TargetsMeta};
```

**2.3.3 main.rs becomes the first consumer.** `run_review` in main.rs is reduced to: parse args → `ReviewOptions`/`AlgorithmParams`/`SliceConfig` from `ReviewArgs` → `api::load_review_inputs` → `api::build_context` → `--format callers` short-circuit (unchanged) → `api::run_review` (multi) or `api::run_algorithm` (single; the existing `annotate_finding_parse_quality` call at `:1122` becomes redundant and is removed — the facade annotated already; the byte control §8.5 proves equivalence) → the existing format `match` arms unchanged. The private helpers `run_algorithm`, `annotate_finding_parse_quality`, `parse_file_line` (kept private in api, used by `run_algorithm`), `build_session` are moved, not copied (doctrine 6: no second copies). `determine_exit_code`, `emit_warnings_to_stderr`, `parse_diagram_cap` stay in main.rs (CLI policy, not library).

**2.3.4 Lifetimes.** `CpgContext<'a>` borrows `files`/`type_db` (`src/cpg/context.rs:39-55`). The facade therefore exposes the two-phase `ReviewInputs` → `build_context(&inputs)` shape rather than a self-referential session; the one-shot `review()` hides it and returns `ReviewOutcome` so callers keep the inputs.

**2.3.5 Build pool.** `load_review_inputs`, `build_context`, `run_review`, `run_algorithm` and `nav_session` each run their body inside `build_pool::install` (re-entry is safe — `build_pool()` is a shared `OnceLock` pool and `ThreadPool::install` nests), so a downstream analyzer cannot hit the stack overflow the pool exists to prevent by forgetting to wrap (S19). `with_build_pool` stays exported for callers that want one outer install.

### 2.4 `prism targets` — `src/targets.rs` (new) + `Command::Targets` in main.rs

**2.4.1 CLI.**
```
prism targets --repo <dir> --diff <patch|json> [--algorithm echo,absence,contract,provenance,membrane]
              [--files a.py,b.py] [--compile-commands cc.json] [--scoped-cpg] [--cache-dir D | --no-cache]
              [--old-repo <dir>] [--min-severity info|suggestion|warning|concern] [--min-tier asserted|candidate]
              [--strict] [--out <path>] [--format json]
```
- Defaults: `--algorithm echo,absence,contract,provenance,membrane`; `--min-severity info`; `--min-tier candidate` (everything); `--format json` with `value_parser = ["json"]`.
- **Algorithm acceptance table (S16)** — decided before any work starts, in this order:

| Algorithms | Behaviour |
|---|---|
| `echo`, `absence`, `contract`, `provenance`, `membrane`, `taint`, `symmetry`, `peer_consistency`, `callback_dispatcher`, `primitive` | accepted; produce targets. `contract` and `delta` use `--old-repo` when given. |
| `angle`, `delta` | accepted (roadmap-named) but construct no findings at this base (`grounding/finding-inventory.md` §5.1): stderr note `targets: <name> produces no findings at this version`; `delta` **without** `--old-repo` → exit 1 before any work (`targets: algorithm DeltaSlice requires --old-repo`). |
| `chop`, `conditioned` | exit 1 before any work: `targets: algorithm <Name> requires --chop-source/--chop-sink (or --condition); use the top-level command`. |
| every other algorithm (`leftflow`, `fullflow`, `originaldiff`, `parentfunction`, `thin`, `relevant`, `barrier`, `spiral`, `circular`, `quantum`, `horizontal`, `vertical`, `3d`, `gradient`, `resonance`, `phantom`) and the presets `review`/`all` | exit 1 before any work: `targets: algorithm <Name> produces slice blocks, not findings; accepted: <comma list of the first two rows>`. |

- `--strict` (S17): exit **3** when `errors` is non-empty after a successful run (partial coverage); default exit 0 with the errors recorded in the document. Exit 1 on load/build failure; exit 2 on clap errors.
- Implementation: `TargetsArgs` (own clap struct — not a flatten of `ReviewArgs`, so `--format text` and `--list-algorithms` cannot leak in) → `api::review(...)` → `targets::project(&outcome.run.findings, Some(&outcome.inputs.files), &TargetsMeta::from(&outcome, ...))` → pretty JSON + `\n` to stdout or `--out`.

**2.4.2 Projection** (`pub fn project(findings: &[SliceFinding], files: Option<&BTreeMap<String, ParsedFile>>, meta: &TargetsMeta) -> TargetsDocument`) — pure, total, deterministic. `TargetsDocument`/`Target`/`Site`/… are `Serialize + Deserialize` structs mirroring `docs/contracts/targets.schema.json` v1 exactly (field order = schema property order; optional fields `skip_serializing_if`).

```rust
pub struct TargetsMeta {
    pub algorithms_run: Vec<String>,         // ReviewRun.algorithms_run
    pub repo_root: PathBuf,                  // canonicalized
    pub repo_sha: Option<String>,            // `git rev-parse HEAD` via std::process::Command; None if not a git checkout or git absent
    pub diff_sha256: String,                 // ReviewInputs.diff_text_sha256
    pub diff_files: Vec<String>,             // ReviewInputs.diff.files[].file_path
    pub errors: Vec<AlgorithmError>,         // ReviewRun.errors
    pub warnings: Vec<String>,               // ReviewRun.warnings (+ projection warnings appended)
    pub min_severity_rank: u8, pub min_tier: FindingTier,
}
```

Per-finding mapping (category → `kind` / `expected.property`; `dependency_hint` is derived **only** where the value is verbatim in the format string or comes from a closed table — a miss omits the hint, never guesses — W7/§5.2):

| algorithm / category | kind | expected.property | dependency_hint |
|---|---|---|---|
| echo / `missing_error_handling` | `external_call` | `error_handled` | `callee` = 2nd single-quoted token of `'{caller}' calls '{callee}' without handling: …` (`echo_slice.rs:237` format string, verbatim) |
| membrane / `unprotected_caller` | `boundary` | `error_handled` | `callee` = 1st quoted token of `unprotected call to '{callee}' from '{caller}'` (`membrane_slice.rs:235`) |
| absence / `missing_counterpart`, `missing_close_on_error_path` | `resource_acquire` | `resource_released` | from the **closed table** `ABSENCE_PAIRS: &[(&str /* PairedPattern.description literal */, Option<&str> /* counterpart call base */, Option<&str> /* kind */)]` in `targets.rs`, one row per description literal in `absence_slice.rs:29-160` at this base; `counterpart` only where exactly one close-call base name exists (e.g. `file open without close` → `close`, `filesystem`; `lock without unlock` → `unlock`; `transaction begin without commit/rollback` → none). Description prefix match on the literal. |
| absence / `close_only_on_error_path` | `resource_release` | `resource_released` | same table |
| absence / `double_close` | `resource_release` | `resource_not_double_released` | `counterpart` = the token before `()` in `… {close}() at line …` (`absence_slice.rs:990`, verbatim) |
| contract / `contract_violation`, `contract_precondition_*`, `contract` | `contract` | `precondition_holds` | none |
| contract / `contract_postcondition*` | `contract` | `postcondition_holds` | none |
| provenance / `untrusted_origin` | `data_origin` | `origin_trusted` | `expected.detail` = the origin word from `has {origin} origin:` (`Origin::name()`, a closed set); `kind` = `db` for `database`, `network` for `external_call`, else omitted |
| taint / `taint_source` | `data_origin` | `origin_trusted` | none |
| taint / `taint_sink`, `unquoted_expansion` | `other` | `not_reached_by_taint` | none |
| symmetry / `broken_symmetry` | `contract` | `counterpart_present` | `counterpart` = 2nd quoted token of `'{}' changed but symmetric counterpart '{}' was not` |
| peer_consistency / `peer_guard_divergence` | `contract` | `peer_consistent` | none |
| callback_dispatcher / any; primitive / any; anything unmatched | `other` | `unknown` | none |

Other fields:
- `site.file/line` = finding file/line; `site.symbol` = `function_name`.
- `site.function_start_line/end_line` = the **innermost** function node spanning `line` via `ParsedFile::function_node_spanning(line)` + `node_line_range` (`src/ast.rs:584`, public; S10) when `files` contains the file; omitted otherwise. If the innermost node is anonymous its range is still used (the range is what scopes injection); `symbol` stays `finding.function_name`. The range is not asserted to contain `line` for every algorithm (S11: `symmetry` anchors on the file's first diff line, `callback_dispatcher` on the definition); §7.4.2 scopes that assertion to `echo`.
- `site.language`: explicit lowering table `Language → &'static str` in `targets.rs` (`Python→python, JavaScript→javascript, TypeScript→typescript, Tsx→tsx, Go→go, Java→java, C→c, Cpp→cpp, Rust→rust, Lua→lua, Terraform→hcl, Bash→bash`), pinned by a test over `Language::all()` against the schema enum (W2). Omitted when `Language::from_path` is `None`.
- `category` verbatim; `source_algorithm` = `finding.algorithm`; `confidence`/`tier` = `classify(&finding.algorithm, ParseQuality::from_annotated(&finding))`; `severity`, `description`, `parse_quality` verbatim; `related.lines/files` sorted+deduplicated (key omitted when both empty).
- `id` = hex sha256 of `"{file}|{line}|{symbol}|{algorithm}|{category}|{description}"` (symbol empty string when `None`).
- A finding with `line == 0` cannot be represented (schema requires `line ≥ 1`): dropped and recorded in `warnings` as `targets: dropped finding with line 0: <algorithm>/<category> in <file>`.
- Dedupe by `id`, first wins; **every** dropped duplicate is recorded in `warnings` as `targets: duplicate id <id> dropped (<algorithm>/<category> <file>:<line>)` (S15; duplicates can differ in severity/related fields at `peer_consistency_slice.rs:141`, `provenance_slice.rs:653`, `primitive_slice.rs:434-441`).
- Filter by `--min-severity` (via `output::severity_rank`) and `--min-tier`; sort by `(site.file, site.line, source_algorithm, category, id)`.

Document-level: `schema_version "1"`; `producer { tool: "prism", version: build_info().package_version, resolution_mode: RESOLUTION_MODE, cache_build_identity: build_info().cache_build_identity, algorithms: meta.algorithms_run }`; `repo { root, sha? }`; `diff { sha256, files }`; `errors`/`warnings` (omitted when empty).

**Comparability (Q1):** the schema's top-level description names the **stable keys** a consumer may diff on across runs and machines — `targets[].id`, `site.{file,line,symbol}`, `kind`, `category`, `expected`, `source_algorithm`, `severity` — and the **envelope** fields it must ignore — `producer.{version,cache_build_identity}`, `repo.root`, `repo.sha`, `diff.sha256`. `confidence`/`tier` are stable for a given prism version and may change across versions as labeling improves.

**2.4.3 Schema custody (S22).** `docs/contracts/targets.schema.json` in this repo is authoritative (tests read it). The projection test (§7.4.5) validates an emitted document against it structurally (required keys, enum membership, id pattern, `additionalProperties` sets) with a small hand-written checker — no `jsonschema` crate. `schema_version` bumps only on breaking changes; additive optional fields do not bump it. The `~/code/tools/contracts/` copy is a mirror; its sha256 is recorded in the handoff at each sync.

### 2.5 README truth pass (docs only; no code) — with a gate (S21)

Apply `grounding/readme-truth.md` row by row. Binding corrections:
1. Every `slicing …` CLI example → `prism …`; delete the "binary is named slicing for historical reasons / rename planned" paragraph.
2. MCP: eight tools (six `nav_*` + `taint_reaches` + `refresh_index`), matching `src/mcp/registry.rs::ToolRegistry::all_v1`.
3. Output formats: `text`, `json`, `paper`, `review`, `callers`, `mermaid`, `sarif` (with one-line descriptions and a SARIF upload snippet for GitHub code scanning); options table row updated.
4. `tests/fixtures/cve/` → the real locations (`tests/fixtures/c/cve_*.c`, `tests/fixtures/sanitizer-suite-*`, `eval/fixtures/`).
5. Framework list, nav subcommand list, language count, algorithm count, cache-scope paragraph (CPG cache per-diff and opt-in via `--cache-dir`; nav index whole-repo under `dirs::cache_dir()/prism/nav/<hash>/`), version/install — each to the truth column of the grounding table.
6. New sections: `prism targets` (with the schema pointer and the acceptance table) and `Library use (prism::api)` with the compatibility promise verbatim from §2.3.1 and the `review()` snippet that is also the doc-test.
7. `skills/prism-code-slicing/SKILL.md` "Output formats" gains `sarif` and `targets`; `CLAUDE.md` gets `api`, `targets`, `finding_confidence`, `output/sarif.rs` in its module maps and the four-value severity vocabulary; `src/slice.rs:27` doc comment corrected to the four values.
8. **Gate:** `tests/cli/readme_test.rs` — (a) every fenced code block line in README.md that starts with `prism ` and contains no `<` placeholder is split shell-style and parsed with `Cli::try_get_matches_from` (parse only; no execution); (b) the README's documented format list equals the `--format` `value_parser` array (both extracted, compared as sets).

**Explicitly not done:** the clap `name = "slicing"` (`src/main.rs:38`) is left as is — the `--version` line grammar `slicing <ver> (<sha>)` has two consumers (`tests/cli/version_test.rs:15` and `eval/tier_a/sut.py::parse_version`, per the IDENTITY GRAMMAR comment in `build.rs:45-56`); renaming it is a coordinated change (§9 follow-up).

## 3. Compatibility

- **Byte-identical**: `text`, `json`, `paper`, `review`, `callers`, `mermaid` for every existing invocation, single- and multi-algorithm (gate §8.5). No `SliceFinding` field is added or renamed; no algorithm file is touched.
- **CLI grammar**: `--format` becomes validated (unknown → clap error, disclosed §2.2.3); `targets` subcommand added; nothing removed. `--help` text changes only by the added format and subcommand.
- **Caches (W4):** the cache **format** and **decision tree** are unchanged: CPG cache version stays `55`, nav sidecar `24`, `SKIP_POLICY_VERSION` `2`, and `build_context` reproduces the Hit/PartialHit/Miss/scoped tree exactly. `PRISM_CACHE_BUILD_IDENTITY` is a sha256 over all `src/**` contents (`build.rs:189-203`), so — as with **every** prism source change — the first run of the branch binary misses on any cache written by another build. That is identity-driven invalidation, orthogonal to this design, and §8.6 measures each binary against its own cache directory.
- **Library**: `prism::api` is new; existing `pub` paths are untouched (`run_slicing`, `run_slicing_inner`, `CpgContext::*`, `navigation::queries::*` keep their signatures).
- **MCP**: untouched.

## 4. Non-goals

DataFlow confidence / reaching definitions (item 2); SCIP rung (item 3); boundary nodes (item 4); workspace split; typed call-edge exposure through `api`; `angle`/`delta` findings; fixing the multi-run `paper` degradation; renaming the clap command; a structured `FindingHint` on `SliceFinding` (would touch 27 production sites + 5 test fixtures; the closed table and two verbatim format-string parses in `targets.rs` are v0); SARIF `codeFlows`/`threadFlows` from diagrams; a `jsonschema` dependency.

## 5. Failure directions (binding for reviewers)

1. **Confidence never over-claims.** Unknown algorithm string, any CPG use, or any non-clean/unknown parse quality → `unlabeled`/`candidate`. The safe direction is under-assertion; a false `asserted` would let a candidate into a required CI check. `asserted` is a claim about the evidence path, not about the heuristic's truth (Q2).
2. **Targets never invent a hint.** A hint is emitted only from a verbatim format-string token or a closed-table row with exactly one counterpart; everything else omits the hint. The harness resolves from site text or reports the site as unreached with `not_reached_reason: callee_unresolved`. A wrong callee would inject a fault at the wrong site.
3. **Nothing is dropped silently.** SARIF: every finding becomes a result. Targets: the only drops are `line == 0` and duplicate ids, each recorded in `warnings`.
4. **The facade changes no behaviour.** Moving `run_algorithm`/`annotate_finding_parse_quality`/`build_session`/the load block is a move with identical bodies; the same-base byte control (§8.5, single and multi runs) and the per-binary cache-decision control (§8.6) are the proof, not the diff.

## 6. Permitted implementation files

New: `src/finding_confidence.rs`, `src/output/sarif.rs`, `src/targets.rs`, `src/api/{mod,build_info,review,nav}.rs`, `tests/cli/sarif_test.rs`, `tests/cli/targets_test.rs`, `tests/cli/readme_test.rs`, `tests/integration/api_test.rs`, `tests/fixtures/targets/**` (small Python fixture + diff exercising echo/absence/contract/provenance/membrane), `scripts/phase0-byte-control.sh`, `docs/superpowers/plans/2026-09-04-prism-phase0-*.md`, `docs/superpowers/handoffs/2026-09-04-prism-phase0-*.md`.
Modified: `src/lib.rs` (three `pub mod` lines), `src/output/mod.rs` (one `pub mod sarif;` + re-export), `src/main.rs` (format validation, sarif arms, `Targets` subcommand, helpers moved out), `src/slice.rs` (doc comment on `severity` only), `tests/cli/main.rs` and `tests/integration/main.rs` (module registrations), `docs/contracts/targets.schema.json` (already vendored), `README.md`, `CLAUDE.md`, `skills/prism-code-slicing/SKILL.md`.
**Forbidden:** `src/algorithms/**`, `src/cpg/**`, `src/cpg_cache.rs`, `src/navigation/**`, `src/resolution*.rs`, `src/call_graph.rs`, `src/ast.rs`, `src/languages/**`, `Cargo.toml` dependencies (no new deps; `sha2` is already a dependency), any cache version constant, `tests/integration/coverage_test.rs` (no algorithm tests are added). If compiled reality requires touching one of these, stop and amend this design.

## 7. Tests (TDD; each names its observable)

Run with `cargo test --test cli sarif_test::`, `cargo test --test cli targets_test::`, `cargo test --test cli readme_test::`, `cargo test --test integration api_test::`, and `cargo test --lib finding_confidence`.

### 7.1 `finding_confidence` (unit, in-module)
1. `("absence", Clean)` → `(Exact, Asserted)`.
2. `("echo", Clean)` → `(Unlabeled, Candidate)`.
3. `("absence", Degraded)` → `(Exact, Candidate)`; `("absence", Unknown)` → `(Exact, Candidate)`.
4. Every string in `["absence","callback_dispatcher","contract","echo","membrane","peer_consistency","primitive","provenance","symmetry","taint"]` round-trips through `SlicingAlgorithm::from_str`.
5. `("not_an_algorithm", Clean)` → `(Unlabeled, Candidate)`.
6. Serde: `FindingConfidence::Exact` → `"exact"`, `Unlabeled` → `"unlabeled"`, `FindingTier::Candidate` → `"candidate"`.
7. `ParseQuality::from_annotated`: `None` → `Clean`; `Some("degraded"|"poor"|"unparseable")` → the variant; `Some("clean")` → `Clean`; any other string → `Unknown`.

### 7.2 SARIF (`tests/cli/sarif_test.rs`, structural, via `Command::cargo_bin("prism")`)
1. Single algorithm `--algorithm absence --format sarif` on a temp Python repo with an `open()` without `close()` (fixture from the `write_repo` shape in `review_compact_test.rs`): stdout parses; `version == "2.1.0"`; `runs[0].tool.driver.name == "prism"`; exactly one rule `prism/absence/missing_counterpart` with a non-empty `fullDescription.text`; that result has `level == "warning"`, `locations[0].physicalLocation.artifactLocation.uri == "a.py"`, `region.startLine == <open line>`, `properties.confidence == "exact"`, `properties.tier == "asserted"`, `properties.resolution_mode == "nominal"`, `ruleIndex == 0`. The machine-specific keys of §2.2.1 are present but never compared.
2. Multi algorithm `--algorithm echo,absence --format sarif` on a fixture where a changed function's caller lacks error handling: a result with ruleId `prism/echo/missing_error_handling` has `properties.confidence == "unlabeled"` and `tier == "candidate"`; `runs[0].properties.algorithms_run == ["EchoSlice","AbsenceSlice"]`; the sequence of `(uri,startLine,ruleId)` tuples is non-decreasing.
3. `--algorithm chop,absence --format sarif` without `--chop-source`: `invocations[0].executionSuccessful == false`; one `toolExecutionNotifications[]` entry with `level == "error"` whose text contains `--chop-source required`; `runs[0].properties.errors[0].algorithm == "Chop"`; the absence result is still present.
4. `--format sarif` twice on the same input (single deterministic algorithm) → identical stdout bytes.
5. `--format bogus`, `--format Json`, `--format ""` → exit code 2 and stderr contains `invalid value` (clap allow-list; compat note §2.2.3).
6. Unit test in `sarif.rs`: a `SliceFinding` with `line: 0` serializes without a `region` key; with `related_lines: [5,3,5]` and `related_files: ["b.py"]` → three `relatedLocations` with ids `0,1,2`, lines `3,5`, the third without `region`.
7. Unit test: severity map covers all four vocabulary values plus an unknown → `"none"`.
8. Unit test: fingerprint of two findings whose descriptions differ only in `(line 12)` vs `(line 13)` is identical; changing `function_name` changes it.

### 7.3 `prism::api` (`tests/integration/api_test.rs`, in-process)
1. `api::review(&ReviewOptions::new(repo), diff_text, &[AbsenceSlice], &SliceConfig::default(), &AlgorithmParams::default())` on the §7.2.1 fixture returns `outcome.run.findings` with one `category == Some("missing_counterpart")` and `parse_quality == None`, and `outcome.inputs.diff.files.len() == 1`.
2. Two-phase API without an outer `with_build_pool`: `load_review_inputs` → `build_context` → `run_review(&[EchoSlice, AbsenceSlice], ..)` returns `algorithms_run == ["EchoSlice","AbsenceSlice"]`, `errors.is_empty()`, `warnings == inputs.parse_warnings` (S19: the facade installs the pool itself).
3. `AlgorithmParams::default()` equals the clap defaults: the test invokes `prism --help` and parses the `[default: N]` annotations for `--barrier-depth`, `--spiral-max-ring`, `--temporal-days`, asserting equality with the struct's defaults — CLI and library defaults cannot drift silently.
4. `run_algorithm(Chop, …, &AlgorithmParams::default())` returns `Err` whose message contains `--chop-source required` (moved behaviour preserved).
5. `build_info()`: `package_version == env!("CARGO_PKG_VERSION")`, `cache_build_identity.len() == 64`.
6. `nav_session(repo, &NavOptions{no_cache:true, cache_dir:None})` (no outer install) then `callers(&s, Seed::Symbol("helper"), 1, false)` on a two-file Python fixture returns evidence whose JSON contains the caller function name.
7. A doc-test on `api::review` (the README snippet) compiles and runs.
8. Compile-time: a test module asserts `#[non_exhaustive]` is present by attempting `let _ = ReviewOptions { .. }` in a `compile_fail` doc-test on `ReviewOptions` (rustdoc `compile_fail`), or, if the harness cannot host it, by grep in `readme_test.rs` over `src/api/*.rs` for `#[non_exhaustive]` on every `pub struct`/`pub enum` line.

### 7.4 `prism targets` (`tests/cli/targets_test.rs`)
1. Default run on `tests/fixtures/targets/` (Python: `svc.py` calls `fetch()` from `client.py` without handling → echo; `open()` without close → absence; a guard-clause edit → contract; a `request.args` read → provenance): stdout parses; `schema_version == "1"`; `producer.tool == "prism"`; `producer.resolution_mode == "nominal"`; `producer.algorithms` equals the five default names; `diff.files` non-empty; at least one target per `source_algorithm ∈ {echo, absence}` (contract/provenance asserted only if the fixture reliably produces them — the implementer records which in the test comment, per the `review_compact_test.rs:145-158` convention).
2. The echo target: `kind == "external_call"`, `expected.property == "error_handled"`, `dependency_hint.callee == "fetch"`, `confidence == "unlabeled"`, `tier == "candidate"`, `site.symbol` is the caller function, `site.function_start_line <= site.line <= site.function_end_line`, `site.language == "python"`.
3. The absence target: `kind == "resource_acquire"`, `expected.property == "resource_released"`, `dependency_hint.counterpart == "close"`, `dependency_hint.kind == "filesystem"`, `confidence == "exact"`, `tier == "asserted"`.
4. `id` is 64 hex chars; running twice gives byte-identical stdout; `--min-tier asserted` removes every `tier == "candidate"` target and keeps the absence one; `--min-severity concern` on this fixture removes the absence target.
5. Every emitted document passes the vendored-schema structural check (required keys per object, enum membership for `kind`, `expected.property`, `confidence`, `tier`, `severity`, `language`, id regex, `additionalProperties` sets) — a small checker in the test module over `docs/contracts/targets.schema.json`.
6. Acceptance table: `--algorithm chop` → exit 1 + `requires --chop-source`; `--algorithm delta` without `--old-repo` → exit 1 + `requires --old-repo`; `--algorithm leftflow` → exit 1 + `produces slice blocks, not findings`; `--algorithm angle` → exit 0, `targets == []`, stderr contains `produces no findings`; `--algorithm chop,absence --strict` → exit 1 (pre-flight rejection precedes running); a run where an accepted algorithm records an error (`--algorithm contract --old-repo /nonexistent` if that errors at run time — implementer verifies; otherwise a constructed case) with `--strict` → exit 3 and `errors` non-empty.
7. `--out <file>` writes the same bytes as stdout would and prints nothing to stdout.
8. Unit tests in `targets.rs`: one per row of the mapping table using a hand-built `SliceFinding` with the verbatim description format from `grounding/finding-inventory.md` §1 (echo, membrane, four absence categories incl. one whose table row has no counterpart — e.g. `transaction begin without commit/rollback` → no `dependency_hint`, provenance `database`/`user_input`, symmetry, an unknown category) asserting `kind`, `expected.property`, and the exact `dependency_hint`; a `line: 0` finding → dropped + warning text; two findings with identical id → one target + a duplicate warning naming the id; `Language::all()` lowering ⊆ schema enum.

### 7.5 README gate (`tests/cli/readme_test.rs`) — §2.5.8.

## 8. Acceptance gates (controller, before review and before PR)

| Deliverable | Discriminating evidence (S23) |
|---|---|
| SARIF | §7.2 structural tests + §8.8 official-schema validation of one real document |
| targets | §7.4.5 vendored-schema check + §7.4.2/3 field assertions |
| facade | §8.5 byte control (single + multi) + §8.6 per-binary cache decisions |
| README | §7.5 |

1. `cargo fmt --all -- --check` → exit 0.
2. `cargo clippy --all-targets --all-features -- -D warnings` → exit 0 (or the pre-existing warning set, diffed against the exact base built in the same worktree — new warnings only are blockers).
3. Focused tests (§7) GREEN. RED is observed on the exact base for the CLI-level tests by running the base binary (`~/code/tools/bin/prism-base-c220525`) with the same invocations: `--format sarif` on base renders text (assertion "stdout parses as JSON" fails), `targets` on base is an unknown subcommand. Unit tests that cannot compile on base are recorded as "feature absent".
4. Full suite: `cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee <log>`; totals computed by `awk` over every `test result:` line of the complete log (never `tail`); base was `3543 passed / 0 failed / 1 ignored` (`~/code/tools/logs/baseline-c220525.log`). Expected: base totals + new tests, 0 failed.
5. **Same-base byte control (the facade proof; W5):** `scripts/phase0-byte-control.sh <base-bin> <branch-bin>` runs both binaries over every checked-in fixture diff (`tests/fixtures/python/calc.diff`, `tests/fixtures/hapi-4552.diff`, `tests/fixtures/nav_compat/*`, `tests/fixtures/review_no_diagrams/*`, `tests/fixtures/c/*.diff` if present) with: (a) single algorithms `leftflow`, `absence`, `contract`, `echo`, `membrane`, `provenance`, `primitive` × formats `text`, `json`, `paper`, `review`, `mermaid`; (b) multi sets `echo,absence,contract` and `absence,contract,primitive` × `text`, `json`, `review`, `mermaid` (exercising `MultiReviewOutput`/`CompactMultiReviewOutput`, `errors`, `warnings`, `parse_quality`); (c) `chop,absence --format json` (an `errors[]` entry); (d) `--format callers`; and `diff`s stdout **and exit code** byte-for-byte. Taint is excluded (documented non-byte-stability). Expected: zero differing invocations. Any difference is a STOP.
6. **Per-binary cache-decision control (W4):** for each binary with its **own** empty `--cache-dir`: run 1, run 2, edit one fixture file, run 3; the observable per run is `(cpg-cache.bin exists?, its mtime changed vs previous run?)` → expected `(created), (unchanged), (changed)` for both binaries. Compare the two sequences, not the artifacts.
7. Tier-A: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` → same pass count as base (regression tripwire only; not evidence for this PR).
8. One real SARIF file validated against the official SARIF 2.1 JSON schema with a Python `jsonschema` check (controller-side, recorded in the handoff with the command and output; checkout-specific fields are present but not compared).
9. Do not rebaseline Tier-A; a suite or corpus failure is attributable only after the exact base fails or passes the same probe in the same environment.

## 9. Follow-ups filed (not in this PR)

- Multi-run `paper` silently degrades to text (`src/main.rs:1095`) — add the arm or reject.
- Rename clap `name = "slicing"` → `prism` together with `tests/cli/version_test.rs:15` and `eval/tier_a/sut.py::parse_version` (IDENTITY GRAMMAR, `build.rs:45-56`).
- `angle_slice` and `delta_slice` emit no findings; both need finding construction before they can feed targets/SARIF.
- `provenance_slice` discards `origin_line/origin_file` (`grounding/finding-inventory.md` §5.5); surfacing them changes `json` bytes for provenance and needs its own byte-pinned PR.
- Structured `FindingHint` (`#[serde(skip)]`) populated at construction, replacing the format-string parses and the closed table in `targets.rs`, and enabling a digit-free fingerprint.
- Typed call edges through `prism::api` (`IndexedIncomingCall` is `pub(crate)`), with item 2/3.
- Item 2 (DataFlow confidence + reaching definitions) will change `classify` to a min over the evidence path, start producing `nameonly`, and add `--min-confidence`.
- `cargo-semver-checks` in CI for `prism::api` once a downstream consumer exists.

## 10. Implementation sequencing (each task PR-sized within the one branch; one PR at the end)

1. `finding_confidence.rs` + unit tests (§7.1).
2. `output/sarif.rs` + `--format` allow-list + both arms + `tests/cli/sarif_test.rs` (§7.2). (Temporarily calls `annotate_finding_parse_quality` from main.rs; task 3 moves it.)
3. `api/` (build_info, review, nav) with main.rs consuming it; `tests/integration/api_test.rs` (§7.3); gates §8.5/§8.6 run here, before anything else builds on the facade.
4. `targets.rs` + `Command::Targets` + fixture + `tests/cli/targets_test.rs` (§7.4).
5. README/CLAUDE.md/SKILL.md truth pass (§2.5) + `readme_test.rs` + doc-test.
6. Closeout: full gates §8, handoff, PR.

## 11. Review and convergence record

Design review cap: **2 rounds** (sol gate via bridge, read-only; Opus parallel seat). Owner authorisation (2026-09-04, verbatim in `~/code/tools/LEDGER.md`): the cap may be exceeded as needed; a disputed sol finding may be adjudicated by a **separate sol judge seat** on whether it is out of scope, deferred, or an implementation detail (the project uses capable implementers, so line-level prescriptions belong in the plan/implementation, not the design).

- **Round 1 — Opus seat (2026-09-04, `~/code/tools/reviews/phase0-spec-r1-opus.md`): FIX, W=9 S=15.** All nine WRONG folded: W1 (`ReviewRun.warnings/parse_quality`, `ReviewOutcome`, `TargetsMeta` defined — §2.3.2/§2.4.2), W2 (Language lowering table — §2.4.2), W3 (`ParseQuality` with `Unknown ⇒ Candidate`; facade annotates — §2.1/§2.3.2), W4 (cache claim restated; per-binary control — §3/§8.6), W5 (byte control widened to multi runs + mermaid + exit codes — §8.5), W6 (digit-masked fingerprint — §2.2.2), W7 (closed table for absence counterparts — §2.4.2), W8 (machine-specific set named — §2.2.1), W9 (`confidence` enum `exact|nameonly|unlabeled`; `resolution_mode` documented as a different axis — §2.1, schema). All fifteen SMELL folded (S10 innermost function, S11 scoped assertion, S12 sorted relatedLocations, S13 explicit single-run fields, S14 `properties.errors`, S15 dedupe warnings, S16 acceptance table, S17 `--strict`/exit 3, S18 compat note + test, S19 pool inside the facade, S20 `#[non_exhaustive]`, S21 README gate, S22 schema custody, S23 discriminating-gate table, S24 `fullDescription` + wart). Open questions ruled: Q1 comparability keys documented in the schema; Q2 `asserted` = evidence-path claim; Q3 `dependency_hint` ships only for verbatim/closed-table sources.
- **Round 1 — sol seat:** pending.

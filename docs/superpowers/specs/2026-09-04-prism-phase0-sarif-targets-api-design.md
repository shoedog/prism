# Prism Phase 0 — SARIF output, `prism targets`, `prism::api` facade, README truth pass — design

**Status:** v5 (2026-09-04) — **SETTLED** by controller ruling after sol round 3 (3W/1S/0I, all closed-form, folded here). Findings converged 31 → 6 → 4 with no repeats; the owner-authorised extension round is spent; residuals are disclosed in §11. Implementation proceeds against v5.
**Recorded:** 2026-09-04 · **Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9` (`origin/main`, PR #225 merge).
**Scope:** the four "Phase 0" items of the tooling roadmap (`~/code/tools/04-prism-plan-roadmap.md` §2, `03-tooling-plan-roadmap.md` §3 Phase 0): the interfaces the analyzer roadmap needs from prism before any analyzer is written. All four are additive serializers and a facade; **no CPG/cache/resolution change and no cache version bump**.
**Grounding:** `~/code/tools/grounding/finding-inventory.md` (every `SliceFinding` construction site), `grounding/cli-output-api.md` (CLI branches, private main.rs helpers, build identity, nav API, test conventions), `grounding/readme-truth.md` (README claims vs code). Line anchors below are hints against the exact base; symbols are the authority (pipeline-lessons #6).
**Contract custody:** `docs/contracts/targets.schema.json` **in this repo is authoritative** (it is what the tests read); `~/code/tools/contracts/targets.schema.json` is a mirror whose sha256 is recorded in the handoff at each sync (§2.4.3).
**Design granularity:** this document fixes architecture, contracts, failure directions, tests and gates. Names and signatures are binding where a consumer depends on them (`prism::api`, the targets schema, SARIF shape); everything else is the implementer's call within the permitted files (§6).

---

## 1. Problem (measured)

1. Findings cannot reach CI today except through prism's own `json`/`review` shapes. There is no SARIF, so GitHub code scanning (the roadmap's stand-in findings plane) cannot ingest prism at all. `SliceFinding` (`src/slice.rs:23-43`) already carries every field SARIF needs.
2. The runtime harness (roadmap Phase 1) needs a stable, versioned "instrumentation targets" document. Nothing projects findings into one; the closest analogue, `CallersOutput` (`src/output/review.rs:264-291`), is `Serialize`-only and shaped for a different consumer.
3. Downstream crates can `use prism::*` (the whole crate is `pub`, no `[lib]` section, no semver statement — `grounding/cli-output-api.md` §10), but the review pipeline that actually produces findings is private to `src/main.rs`: `run_algorithm` (`:1223`, the only place per-algorithm configs are assembled — `Chop`, `ConditionedSlice`, `DeltaSlice` return empty results without it), `annotate_finding_parse_quality` (`:1201`), and the ~200-line diff → parse → type_db → CPG-cache orchestration (`:724-926`). An analyzer cannot reproduce prism's own findings without copying main.rs.
4. `--format` is an unvalidated `String` (`src/main.rs:66-68`): an unknown value silently renders `text`; in the multi-algorithm branch even `paper` degrades to text (`:1095`).
5. README is stale in 12 places (`grounding/readme-truth.md`: 8 STALE, 4 WRONG), most visibly ~40 `slicing …` examples for a binary named `prism`, "six" MCP tools (there are eight), three output formats (there are six), and a `tests/fixtures/cve/` directory that does not exist.
6. Confidence is a dead letter for findings: `ResolutionConfidence` (`src/resolution.rs:26`) lives on `CpgEdge::Call/Return`, only `barrier_slice.rs:107` reads it, and no finding-emitting algorithm consults it (`grounding/finding-inventory.md` §2, §5.7). DataFlow edges carry no confidence at all (roadmap item 2, out of scope here). Any confidence label Phase 0 emits must therefore be derived from what is *knowable* today, and must say when it knows nothing.
7. Finding anchors are lossy: `provenance` anchors on the diff-line *use* and discards the traced origin (`provenance_slice.rs:653`); `symmetry` anchors every finding on the file's *first* diff line and stores the counterpart's line range against `related_files[0]`; `callback_dispatcher` anchors on the registered function's definition; `close_only_on_error_path` anchors on the *open* call (`absence_slice.rs:771`). A projection must not pretend these anchors are something they are not.

## 2. Design

### 2.1 Shared finding classification — `src/finding_confidence.rs` (new, single source)

The doctrine ("nothing below Exact feeds an asserted finding") needs one function that both new serializers consult. Phase 0 can only know two things about a finding's evidence path: whether the algorithm used the CPG at all (`SlicingAlgorithm::needs_cpg()`, `src/slice.rs:211`), and the parse quality of every file the finding's evidence came from. Everything else is unlabeled today, so the label says exactly that.

```rust
//! Single source for the confidence/tier label attached to a finding by the
//! SARIF and targets serializers.
//! Phase 0 rule (roadmap 04 §3.4): AST-only algorithms are Exact by
//! construction; any CPG-derived finding is Unlabeled because DataFlow edges
//! carry no confidence yet. Item 2 (reaching-definitions labeling) adds
//! `classify_with_evidence(algorithm, quality, evidence: &EvidencePath)`
//! taking a min over the path; `classify` stays as the evidence-free entry.
//!
//! `Asserted` is a claim about the EVIDENCE PATH (no unlabeled or
//! name-only edge, clean parse of every evidence-bearing file), never about
//! the truth of the heuristic the algorithm encodes. A required CI check may
//! gate on `asserted`; whether an asserted finding is a real defect is the
//! reviewer's call.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingConfidence { Exact, NameOnly, Unlabeled }   // mirrors ResolutionConfidence + "unlabeled"

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingTier { Asserted, Candidate }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]   // ordered best → worst
#[non_exhaustive]
pub enum ParseQuality { Clean, Degraded, Poor, Unparseable, Unknown }

impl ParseQuality {
    /// From the AUTHORITATIVE per-file map produced by `algorithms::check_parse_quality`
    /// (`ReviewInputs.parse_quality`). That map is SPARSE: `check_parse_quality` inserts a file
    /// only when its error rate exceeds 1% (`src/algorithms/mod.rs:75-84`), so absence means
    /// "clean" for a file prism parsed and "unknown" for a file it did not (sol r2 W1). Hence:
    /// for each file — in `map` → its grade; else in `parsed` → Clean; else Unknown. Result =
    /// the worst over `files`; Unknown when `files` is empty. The sparse map itself is never
    /// modified (legacy `json`/`review` output serialises it byte-for-byte).
    pub fn min_over(files: &[&str], map: &BTreeMap<String, FileParseQuality>,
                    parsed: &BTreeMap<String, ParsedFile>) -> Self;
}

/// Files whose parse quality bears on a finding's evidence: the anchor file plus every
/// `related_files` entry (symmetry's counterpart, callback registrations, primitive's
/// callee file, provenance's origin file — sol #2).
pub fn evidence_files(finding: &SliceFinding) -> Vec<&str>;

/// The one entry point both serializers use. Encodes the evidence rules: contract findings
/// computed against an `--old-repo` tree (categories `contract_precondition_weakened`,
/// `contract_precondition_strengthened`, `contract_postcondition_weakened`,
/// `contract_postcondition_strengthened` — the only categories `slice_delta` emits) depend on
/// old-tree files prism parsed separately and never graded → `Unknown` (sol r2 #2); everything
/// else → `min_over(&evidence_files(f), map, parsed)`.
pub fn parse_quality_for(finding: &SliceFinding, map: &BTreeMap<String, FileParseQuality>,
                         parsed: &BTreeMap<String, ParsedFile>) -> ParseQuality;

/// The BUILD's dataflow-labeling capability (roadmap 04 §3.6 `--resolution`): a DIFFERENT AXIS
/// from a finding's confidence. Phase 0: "nominal" = DataFlow edges unlabeled.
pub const RESOLUTION_MODE: &str = "nominal";

pub fn classify(algorithm: &str, parse_quality: ParseQuality) -> (FindingConfidence, FindingTier)
```

Rules (binding; each has a test in §7.1):
- `SlicingAlgorithm::from_str(algorithm)` is `None` → `(Unlabeled, Candidate)` (fail-safe for unknown producers; every production algorithm string — `absence`, `callback_dispatcher`, `contract`, `echo`, `membrane`, `peer_consistency`, `primitive`, `provenance`, `symmetry`, `taint` — round-trips through `from_str`, pinned by §7.1.4).
- `needs_cpg()` → `Unlabeled`; else `Exact`. `NameOnly` is reserved for item 2 and is never produced in Phase 0.
- Tier is `Asserted` iff confidence is `Exact` **and** `parse_quality == Clean`. `Unknown` is `Candidate` (the safe direction for absent information is under-assertion).
- Both serializers compute `parse_quality` as `parse_quality_for(f, &inputs.parse_quality, &inputs.files)`. The review path parses exactly the diff files; an evidence file absent from both the map and `files` is a genuine unknown.
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
                   "fullDescription": { "text": "<one sentence per category from the table in sarif.rs; contract_violation names both of its shapes>" },
                   "properties": { "algorithm": "echo", "category": "missing_error_handling" } } ]
    } },
    "invocations": [{ "executionSuccessful": true,
                      "toolExecutionNotifications": [ { "level": "error", "message": { "text": "Chop: --chop-source required for chop algorithm" } },
                                                      { "level": "warning", "message": { "text": "<parse warning verbatim>" } },
                                                      { "level": "warning", "message": { "text": "skipped unsupported file: <path> (<reason>)" } } ] }],
    "results": [ { ...see 2.2.2... } ],
    "properties": { "mapping_version": "1", "algorithms_run": ["EchoSlice", "AbsenceSlice"], "resolution_mode": "nominal",
                    "errors": [ { "algorithm": "Chop", "error": "--chop-source required for chop algorithm" } ],
                    "prism_build_identity": "<sha256>", "prism_git_sha": "<GIT_SHA>", "binary_input_dirty": false }
  }]
}
```

- No `originalUriBaseIds` (a direct producer should not set it, and it discloses local paths — sol #23). Artifact URIs are repo-relative with `uriBaseId: "%SRCROOT%"`, which consumers resolve against the checkout root (GitHub code scanning does).
- `properties.mapping_version` versions prism's SARIF *mapping* (the SARIF `version` versions the standard — sol #17).
- `toolExecutionNotifications` and `properties.errors` are omitted when empty; `executionSuccessful = errors.is_empty()`. `properties.errors` mirrors `AlgorithmError` so partial coverage is machine-readable.
- **Build-specific fields** (tests must ignore them; no golden may contain them): `tool.driver.version`, `tool.driver.semanticVersion`, `runs[0].properties.prism_build_identity` (the binary's source-input hash — see §3), `prism_git_sha` (carries `-dirty`), `binary_input_dirty`.
- `rules` holds one entry per distinct `ruleId` present in `results`, sorted by id; `results[i].ruleIndex` is the index of `results[i].ruleId` in that array and §7.2.1 asserts the correspondence for **every** result (sol #20). Rule ids keep prism's category strings verbatim even though `primitive` uses SCREAMING_SNAKE ids next to snake_case categories — stability wins; the wart is recorded.

**2.2.2 Result mapping** (one `result` per `SliceFinding`; nothing is dropped):

| SARIF | from |
|---|---|
| `ruleId` | `format!("prism/{}/{}", finding.algorithm, finding.category.as_deref().unwrap_or("uncategorized"))`. All 27 production sites set a category; `uncategorized` exists for forward compatibility. |
| `level` | `concern → "error"`, `warning → "warning"`, `suggestion → "note"`, `info → "note"`; **any other severity → `"error"`** (conservative: an unknown future severity must not become invisible — sol #12) with the original string kept in `properties.severity`. |
| `message.text` | `finding.description` verbatim. |
| `locations[0].physicalLocation.artifactLocation` | `{ "uri": uri(finding.file), "uriBaseId": "%SRCROOT%" }` where `uri()` normalises `\` to `/` and percent-encodes each path segment per RFC 3986 (spaces, `#`, `?`, `%`, non-ASCII) — `DiffInput::from_json` accepts arbitrary strings (`diff.rs:145-147`; sol #10). Absolute or `..`-containing paths are emitted as given after encoding and produce one `toolExecutionNotifications` warning `path escapes repo root: <path>`. |
| `locations[0].physicalLocation.region` | `{ "startLine": finding.line }` when `line >= 1`; the `region` key is **omitted** when `line == 0` (SARIF requires `startLine >= 1`; `symmetry_slice.rs:230` has a dead `unwrap_or(0)` fallback — §7.2.6 pins the omission). |
| `locations[0].logicalLocations` | `[ { "name": function_name, "kind": "function" } ]` when `function_name` is `Some`; omitted otherwise. |
| `relatedLocations` | `related_lines` are attributed to a file by the **per-algorithm attribution table** in `sarif.rs` (sol #8): `SameFile` — `echo`, `membrane`, `absence`, `contract`, `provenance`, `taint`, `peer_consistency` (lines are in `finding.file`; `related_files` are separate artifacts); `CounterpartFile` — `symmetry`, `primitive` (lines are in `related_files[0]`; grounding: `symmetry_slice.rs:246-247`, `primitive_slice.rs:434-441`); `Ambiguous` — `callback_dispatcher` and any unknown algorithm (lines are emitted as locations **only** when `related_files` is empty). Lines that cannot be attributed go to `properties.related_lines` (data preserved, no wrong location). Lines equal to `0` are skipped. Within the emitted set: lines sorted+deduplicated first (ids `0..`), then files sorted+deduplicated without region. Omitted when nothing remains. |
| `partialFingerprints` | `{ "prism/finding/v1": sha256(algorithm ‖ category ‖ file ‖ function_name ‖ masked_description ‖ line_text) }` computed over a canonical JSON array of the six strings (no delimiter ambiguity — sol #14), where `masked_description` replaces every maximal run of ASCII digits with `#` and `line_text` is the whitespace-trimmed source text of `finding.line` (empty when unavailable) — a stable occurrence discriminator under line shifts (sol #29). `line` itself is excluded. Two findings whose masked descriptions and line text coincide share a fingerprint. GitHub code scanning matches alerts on its own `primaryLocationLineHash` (computed from the primary location's line content when absent — GitHub's SARIF support page) and ignores custom `partialFingerprints`, so on GitHub such findings are still distinct alerts by location; other consumers that key on `prism/finding/v1` must combine it with the primary location, and the module docs say so (sol r3 audit #29). |
| `properties` | `{ "algorithm", "category", "severity", "confidence", "tier", "resolution_mode", "parse_quality", "function_name"?, "related_files"?, "related_lines"? }` — `confidence`/`tier` from `classify(&f.algorithm, parse_quality_for(f, map, files))`; `parse_quality` = that value (lowercase, may be `unknown`). |

Diagrams (`finding.diagrams`) are not serialized into SARIF (they are prism-specific; the `json` format keeps them).

**2.2.3 CLI wiring** (`src/main.rs`):
- `--format` gains `value_parser = ["text", "json", "paper", "review", "callers", "mermaid", "sarif"]`. An unknown format is now a clap error (exit 2) instead of silently rendering text. **Compatibility note:** values that previously "worked" by falling through to text — e.g. `-f Json`, `-f ""`, `-f txt` — now fail; §7.2.5 enumerates them. This is a deliberate fix of problem §1.4 and is disclosed in the PR; the multi-run `paper` gap is **not** fixed here (follow-up, §9).
- A `"sarif"` arm is added to **both** `match cli.format.as_str()` sites (multi at `:999`, single at `:1124`). Both call one helper `emit_sarif(&SarifInputs { findings, errors, parse_warnings, load_warnings, algorithms_run, parse_quality, files, sources })`. Multi-run passes `run.findings`, `run.errors`, `run.warnings`, `inputs.load_warnings`, `run.algorithms_run`, `inputs.parse_quality`, `inputs.files`, `inputs.sources`. Single-run passes `result.findings`, `errors: &[]`, `result.warnings`, `inputs.load_warnings`, `&[algorithm.name()]`, the same map, files and sources — **not** the `mermaid` arm's wrapper, which discards findings/warnings.
- Trailer identical to the `json` arm: `emit_warnings_to_stderr` + `determine_exit_code`.
- Output: `serde_json::to_string_pretty` + `\n` to stdout, same as `json`.

**2.2.4 Ordering.** `results` are sorted by `(uri, line, ruleId, message.text)` with a stable sort before serialization, independent of algorithm run order. `rules` sorted by `id`. `relatedLocations` sorted per §2.2.2. Determinism is by construction.

**2.2.5 Byte-pinning.** `json`, `review`, `text`, `paper`, `mermaid`, `callers` are untouched (§8.5 proves it — stdout, stderr and exit status — for single- and multi-algorithm runs). SARIF is a new serializer with its own structural tests; **no byte-pinned golden for multi-algorithm SARIF that includes Taint** (not byte-stable on some fixtures — `tests/cli/nav_compat_test.rs` header).

### 2.3 `prism::api` facade — `src/api/` (new; `pub mod api` in `src/lib.rs`)

**Decision (roadmap 04 §2.3):** no workspace split. `prism::api` is the first and only module with a stated compatibility promise; the rest of the crate stays `pub` but is documented as internal.

**2.3.1 Compatibility promise** (module docs, README §Library) — sol #16/#17, Opus S20:
> Within a major version, every item of `prism::api` keeps its name and signature; a removal or signature change is preceded by a `#[deprecated]` release. Every struct and enum defined in `prism::api` is `#[non_exhaustive]`: construct with `new`/`Default` and assign public fields, never with a struct literal or exhaustive `match`. Types from other modules that appear in `prism::api` signatures (`ParsedFile`, `TypeDatabase`, `CpgContext`, `SliceConfig`, `SlicingAlgorithm`, `SliceResult`, `SliceFinding`, `NavigationSession`, `Evidence`, `QueryError`, `DiffInput`, `Language`, `LanguageVersion`) are **stable as handles**: you may obtain them from `prism::api`, pass them back into `prism::api`, and read the fields the `prism::api` docs name; their other fields and methods are internal and may change. Everything else in the crate is internal. Output formats are versioned by their own fields: multi-run `json`/`review` carry `version: "1.0"`; single-run `json`/`review` shapes are unversioned and pinned by tests; SARIF carries `properties.mapping_version`; targets carries `schema_version`.

**2.3.2 Surface** (`src/api/mod.rs` re-exports; split as `build_info.rs`, `review.rs` (options, inputs, loader, context), `run.rs` (algorithm dispatch, run, review), `nav.rs` — each under the 600-line cap; all structs/enums `#[non_exhaustive]`):

```rust
// src/api/build_info.rs
pub struct BuildInfo {
    pub package_version: &'static str,      // env!("CARGO_PKG_VERSION")
    pub git_sha: &'static str,              // env!("GIT_SHA")
    pub build_identity: &'static str,       // cpg_cache::current_cache_build_identity(): sha256 over this BINARY's source inputs (§3)
    pub binary_input_dirty: bool,           // cpg_cache::binary_input_dirty()
    pub grammar_fingerprint: &'static str,  // env!("GRAMMAR_FINGERPRINT")
}
pub fn build_info() -> BuildInfo;
// Rationale: env!() in a downstream crate reads the downstream crate's values (grounding §3).

// src/api/review.rs
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

/// Owned inputs the CPG is built from. `CpgContext<'a>` borrows `files` (sol #24: the type
/// database is cloned into the CPG/providers by the constructors, not borrowed; it is kept
/// here because `build_context` needs it as an argument).
pub struct ReviewInputs {
    pub files: BTreeMap<String, ParsedFile>,
    pub sources: BTreeMap<String, String>,
    pub type_db: Option<TypeDatabase>,
    pub diff: DiffInput,
    pub diff_text_sha256: String,                // hex sha256 of the diff text as read
    pub parse_warnings: Vec<String>,                         // exactly what main.rs puts in legacy `warnings` today
    pub load_warnings: Vec<String>,                          // NEW (sol r2 W4): diff files SKIPPED at load, i.e. unsupported language —
                                                             // `skipped unsupported file: <path> (unsupported language)`; today stderr-only,
                                                             // stderr text unchanged. An UNREADABLE file is fatal on the exact base
                                                             // (`fs::read_to_string(..).context(..)?`, main.rs:757) and stays fatal (sol r3 W1).
    pub parse_quality: BTreeMap<String, FileParseQuality>,   // AUTHORITATIVE (sparse) per-file map (§2.1)
    pub scope_graph_inputs: ScopeGraphBuildInputs,
}
/// = main.rs :724-816 verbatim behaviour (JSON-or-unified diff detection, --files filter,
/// per-diff-file parse with unsupported languages warned to stderr and skipped, TypeDatabase
/// auto/explicit, parse-quality map, scope-graph inputs). Runs inside build_pool::install.
pub fn load_review_inputs(opts: &ReviewOptions, diff_text: &str) -> Result<ReviewInputs>;

/// Same cache decision tree as main.rs :819-926 (use_cache = cache_dir.is_some() && !no_cache
/// && !scoped_cpg; Hit / PartialHit / Miss / scoped), applies language_versions, saves the cache
/// on non-Hit paths. Runs inside build_pool::install. Non-fatal conditions main.rs prints today
/// (cache-save failure, type-database fallback) are ALSO returned as `warnings` (stderr text
/// unchanged) so `targets`/SARIF can surface them (sol r2 W4 / r3 audit).
pub struct BuiltContext<'a> { pub ctx: CpgContext<'a>, pub warnings: Vec<String> }
pub fn build_context<'a>(inputs: &'a ReviewInputs, opts: &ReviewOptions) -> Result<BuiltContext<'a>>;

// src/api/run.rs
pub const DEFAULT_BARRIER_DEPTH: usize = …;   // the single source for both clap `default_value_t`
pub const DEFAULT_SPIRAL_MAX_RING: usize = …; // and AlgorithmParams::default() (sol #20)
pub const DEFAULT_TEMPORAL_DAYS: usize = …;
pub struct AlgorithmParams { /* barrier_depth, barrier_symbols, chop_source, chop_sink, taint_sources,
    taint_return_flow, condition, old_repo, spiral_max_ring, quantum_var, peer_pattern, layers, concern,
    temporal_days — one field per ReviewArgs algorithm flag, same types */ }
impl Default for AlgorithmParams { /* uses the DEFAULT_* consts */ }

/// main.rs::run_algorithm, moved verbatim, then annotates the result's findings with parse
/// quality (idempotent) so no finding leaves the facade unannotated. Does NOT set
/// `result.warnings` (main.rs keeps `result.warnings = parse_warnings` for the single run —
/// sol #18). Runs inside build_pool::install.
pub fn run_algorithm(algorithm: SlicingAlgorithm, ctx: &CpgContext, inputs: &ReviewInputs,
                     config: &SliceConfig, params: &AlgorithmParams, repo: &Path) -> Result<SliceResult>;

pub struct ReviewRun {
    pub results: Vec<SliceResult>,          // the SUCCESSFUL subsequence in request order (an erroring algorithm has no entry —
                                            // exactly today's multi-run behaviour); identify each by `results[i].algorithm` (sol r2 W2)
    pub findings: Vec<SliceFinding>,        // flattened, annotated
    pub errors: Vec<AlgorithmError>,
    pub warnings: Vec<String>,              // = inputs.parse_warnings (legacy `warnings` field; NOT load_warnings — byte-pinned)
    pub algorithms_run: Vec<String>,        // SlicingAlgorithm::name(), every REQUESTED algorithm, in request order
}
pub fn run_review(ctx: &CpgContext, inputs: &ReviewInputs, algorithms: &[SlicingAlgorithm],
                  config: &SliceConfig, params: &AlgorithmParams, repo: &Path) -> ReviewRun;

pub struct ReviewOutcome { pub inputs: ReviewInputs, pub run: ReviewRun, pub build_warnings: Vec<String> }
/// One-shot: installs the build pool, loads, builds, runs; returns inputs AND run.
pub fn review(opts: &ReviewOptions, diff_text: &str, algorithms: &[SlicingAlgorithm],
              config: &SliceConfig, params: &AlgorithmParams) -> Result<ReviewOutcome>;

pub fn parse_algorithms(spec: &str) -> Result<Vec<SlicingAlgorithm>>;   // "review" | "all" | "a,b,c" | "a" — main.rs :687-704, moved
pub fn annotate_finding_parse_quality(findings: &mut [SliceFinding], files: &BTreeMap<String, ParsedFile>);

// src/api/nav.rs
#[derive(Default)] pub struct NavOptions { pub no_cache: bool, pub cache_dir: Option<PathBuf> }   // non_exhaustive ⇒ construct via Default (sol r2 #16)
/// main.rs::build_session, moved; installs the build pool internally (whole-repo parse is the
/// deepest-stack path in the crate).
pub fn nav_session(repo: &Path, opts: &NavOptions) -> Result<NavigationSession>;
pub enum Seed<'a> { Symbol(&'a str), Location(&'a str) /* "file:line" */, SymbolInFile { symbol: &'a str, file: &'a str } }
pub fn callers(s: &NavigationSession, seed: Seed, depth: usize, exact_only: bool) -> Result<Evidence, QueryError>;
pub fn callees(s: &NavigationSession, seed: Seed, depth: usize, exact_only: bool) -> Result<Evidence, QueryError>;
// Thin wrappers over navigation::queries::{callers,callees}_with_confidence. Typed call edges stay pub(crate) (§9).

// src/api/mod.rs re-exports, plus:
pub use crate::build_pool::install as with_build_pool;   // nesting is safe (OnceLock pool, re-entrant install)
pub use crate::finding_confidence::{classify, evidence_files, FindingConfidence, FindingTier, ParseQuality, RESOLUTION_MODE};
pub use crate::output::sarif::{to_sarif, SarifInputs};
pub use crate::targets::{project, TargetsDocument, TargetsMeta};
```

**2.3.3 main.rs becomes the first consumer.** `run_review` in main.rs is reduced to: parse args → `ReviewOptions`/`AlgorithmParams`/`SliceConfig` from `ReviewArgs` (clap `default_value_t = api::DEFAULT_*`) → `api::load_review_inputs` → `api::build_context` → `--format callers` short-circuit (unchanged) → multi: `api::run_review`; single: `api::run_algorithm` **then `result.warnings = parse_warnings` exactly as today at `:1121`** (the `annotate_finding_parse_quality` call at `:1122` is removed as redundant) → the existing format `match` arms unchanged. The private helpers `run_algorithm`, `annotate_finding_parse_quality`, `parse_file_line` (private in api), `build_session` are moved, not copied (doctrine 6). `determine_exit_code`, `emit_warnings_to_stderr`, `parse_diagram_cap` stay in main.rs.

**2.3.4 Lifetimes.** `CpgContext<'a>` borrows `files` (`src/cpg/context.rs:39-55`). The facade exposes the two-phase `ReviewInputs` → `build_context(&inputs)` shape rather than a self-referential session; `review()` hides it and returns `ReviewOutcome` so callers keep the inputs.

**2.3.5 Build pool.** `load_review_inputs`, `build_context`, `run_review`, `run_algorithm` and `nav_session` each run their body inside `build_pool::install` (re-entry is safe), so a downstream analyzer cannot hit the stack overflow the pool exists to prevent by forgetting to wrap. `with_build_pool` stays exported.

### 2.4 `prism targets` — `src/targets/{mod,model,mapping}.rs` (new) + `Command::Targets` in main.rs

Module split (sol #31): `model.rs` — DTOs mirroring the schema (`Serialize + Deserialize`); `mapping.rs` — category table, hint parsers, language lowering, related-file attribution; `mod.rs` — `project`, `TargetsMeta`, id/dedupe/sort/warnings.

**2.4.1 CLI.**
```
prism targets --repo <dir> --diff <patch|json> [--algorithm echo,absence,contract,provenance,membrane]
              [--files a.py,b.py] [--compile-commands cc.json] [--scoped-cpg] [--cache-dir D | --no-cache]
              [--old-repo <dir>] [--min-severity info|suggestion|warning|concern] [--min-tier asserted|candidate]
              [--strict] [--out <path>] [--format json]
```
- Defaults: `--algorithm echo,absence,contract,provenance,membrane`; `--min-severity info`; `--min-tier candidate`; `--format json` with `value_parser = ["json"]`.
- **Algorithm acceptance table** — decided before any work starts:

| Algorithms | Behaviour |
|---|---|
| `echo`, `absence`, `contract`, `provenance`, `membrane`, `taint`, `symmetry`, `peer_consistency`, `callback_dispatcher`, `primitive` | accepted; produce targets. `contract` and `delta` use `--old-repo` when given. |
| `angle`, `delta` | accepted (roadmap-named) but construct no findings at this base: stderr note `targets: <name> produces no findings at this version`; `delta` **without** `--old-repo` → exit 1 before any work. |
| `chop`, `conditioned` | exit 1 before any work: `targets: algorithm <Name> requires --chop-source/--chop-sink (or --condition); use the top-level command`. |
| every other algorithm and the presets `review`/`all` | exit 1 before any work: `targets: algorithm <Name> produces slice blocks, not findings; accepted: <list of the first two rows>`. |

- `--strict`: exit **3** when `errors` is non-empty after a successful run (partial coverage); default exit 0 with the errors recorded in the document. Exit 1 on load/build failure; exit 2 on clap errors.
- Implementation: `TargetsArgs` (own clap struct — not a flatten of `ReviewArgs`) → `api::review(...)` → `targets::project(&outcome.run.findings, &outcome.inputs, &TargetsMeta { … })` → pretty JSON + `\n` to stdout or `--out`.

**2.4.2 Projection** (`pub fn project(findings: &[SliceFinding], inputs: &ReviewInputs, meta: &TargetsMeta) -> TargetsDocument`) — pure, total, deterministic; `inputs` supplies `files` (function bounds), `parse_quality` (the authoritative map), `sources` (unused in v1), `diff`.

```rust
pub struct TargetsMeta {
    pub algorithms_run: Vec<String>, pub repo_root: PathBuf /* canonicalized */,
    pub repo_sha: Option<String> /* `git rev-parse HEAD`; None if not a git checkout or git absent */,
    pub errors: Vec<AlgorithmError>,
    pub run_warnings: Vec<String> /* assembled by the CLI: inputs.parse_warnings ++ inputs.load_warnings ++ outcome.build_warnings, in that order */,
    pub min_severity_rank: u8, pub min_tier: FindingTier,
}
// Ownership (sol r3 S1): `project` emits `document.warnings = meta.run_warnings ++ <projection warnings it generated>`; nothing else
// writes to it, so no source is duplicated or dropped.
```

**Q2 ruling (sol):** a targets entry need not be an executable instrumentation site; `kind` says what the anchor *is*, and lossy anchors are `other` with the property preserved so the harness can decide. Per-finding mapping (category → `kind` / `expected.property`; `dependency_hint` is derived **only** from a verbatim format-string token or a closed-table row; parsers are bounded `str::find`/`split` code — **no `regex` dependency**, sol #13):

| algorithm / category | kind | expected.property | dependency_hint / notes |
|---|---|---|---|
| echo / `missing_error_handling` | `external_call` | `error_handled` | `callee` = 2nd single-quoted token of `'{caller}' calls '{callee}' without handling: …` (`echo_slice.rs:237`) |
| membrane / `unprotected_caller` | `boundary` | `error_handled` | `callee` = 1st quoted token of `unprotected call to '{callee}' from '{caller}'` (`membrane_slice.rs:235`) |
| absence / `missing_counterpart`, `missing_close_on_error_path`, **`close_only_on_error_path`** | `resource_acquire` (all three anchor on the *open* call — sol #6) | `resource_released` | closed table `ABSENCE_PAIRS: &[(&str /* PairedPattern.description literal */, Option<&str> /* counterpart base */, Option<&str> /* kind */)]`, one row per literal in `absence_slice.rs:29-160` at this base; `counterpart` only where exactly one close-call base exists (`file open without close` → `close`/`filesystem`; `lock without unlock` → `unlock`; `transaction begin without commit/rollback` → none). Description prefix match. |
| absence / `double_close` | `resource_release` | `resource_not_double_released` | `counterpart` = token before `()` in `… {close}() at line …` (`absence_slice.rs:990`) |
| contract / `contract_violation` | `contract` | description starts with `Guard clause modified` → `precondition_holds`; starts with `Return behavior modified` → `postcondition_holds`; else `unknown` (two construction sites share the category — sol #5) | none |
| contract / `contract_precondition_*`, `contract` | `contract` | `precondition_holds` | none |
| contract / `contract_postcondition*` | `contract` | `postcondition_holds` | none |
| provenance / `untrusted_origin` | **`other`** (the anchor is the diff-line *use*; the traced origin is discarded by the algorithm — sol #7) | `origin_trusted` | `expected.detail` = `"<origin> origin at use site"` with the origin word from `has {origin} origin:` (closed set `Origin::name()`); `dependency_hint.kind` = `db` for `database`, `network` for `external_call`, else omitted |
| taint / `taint_source` | `data_origin` (the anchor *is* the source) | `origin_trusted` | none |
| taint / `taint_sink`, `unquoted_expansion` | `other` | `not_reached_by_taint` | none |
| symmetry / `broken_symmetry` | `contract` | `counterpart_present` | `counterpart` = 2nd quoted token of `'{}' changed but symmetric counterpart '{}' was not` |
| peer_consistency / `peer_guard_divergence` | `contract` | `peer_consistent` | none |
| callback_dispatcher / any; primitive / any; anything unmatched | `other` | `unknown` | none |

Other fields (totality rules — sol #11):
- `site.file` = `finding.file` with `\` normalised to `/`. An absolute or `..`-containing path cannot satisfy the contract (repo-relative): the finding is **dropped** and `warnings` += `targets: dropped finding with path escaping repo root: <path>` (sol r2 #10).
- `site.line` = `finding.line`; a finding with `line == 0` cannot be represented (schema `≥ 1`): dropped, `warnings` += `targets: dropped finding with line 0: <algorithm>/<category> in <file>`.
- `site.symbol` = the name of the innermost function enclosing `line` when `inputs.files` has the file and that node is named; otherwise `finding.function_name`. When both exist and disagree (symmetry anchors on the file's first diff line), the enclosing name wins — the contract defines `symbol` as the enclosing function — and `warnings` += `targets: symbol <enclosing> differs from finding's function <named> at <file>:<line>` (sol r2 #9).
- `site.function_start_line/end_line` = the range of the **innermost** function node spanning `line` via `ParsedFile::function_node_spanning(line)` + `node_line_range` (`src/ast.rs:584`) whenever `inputs.files` contains the file and such a node exists — bounds and `symbol` then describe the same enclosing function, which is what the harness scopes injection to (sol r3 W3). Omitted only when no enclosing function is found. A disagreement between that function's name and the finding's `function_name` is a warning (next bullet), never a reason to omit bounds.
- `site.language`: explicit lowering table `Language → &'static str` in `mapping.rs` (`Python→python, JavaScript→javascript, TypeScript→typescript, Tsx→tsx, Go→go, Java→java, C→c, Cpp→cpp, Rust→rust, Lua→lua, Terraform→hcl, Bash→bash`), pinned by a test over `Language::all()` against the schema enum. Omitted when `Language::from_path` is `None`.
- `category` = `finding.category` or `"uncategorized"` (same fallback as SARIF); `source_algorithm` = `finding.algorithm`.
- `severity`: one of the four known values verbatim; any other → `"concern"` (conservative) plus `warnings` += `targets: unknown severity '<s>' mapped to concern for <file>:<line>`.
- `confidence`/`tier` = `classify(&finding.algorithm, parse_quality_for(f, &inputs.parse_quality, &inputs.files))`; `parse_quality` = that value, lowercase (`unknown` is a legal schema value).
- `description` verbatim; `related.lines` = `related_lines` with zeros removed, sorted, deduplicated; `related.files` sorted, deduplicated (key omitted when both empty). Attribution of lines to files is not encoded in v1 (see §2.2.2's table; a structured `related` v2 is a follow-up).
- `id` = hex sha256 of the compact canonical JSON array `[file, line, symbol_or_"", algorithm, category, description, severity, related_lines_sorted_dedup, related_files_sorted_dedup]` (`serde_json::to_vec` of a `Vec<Value>`; the last two are JSON arrays) — no delimiter ambiguity; severity and related evidence included so findings that differ in any structured field get distinct ids (sol r2 #14 / r3 audit). Two findings with identical ids are therefore identical in every field the contract carries, and first-wins dedupe loses nothing but the warning still records it.
- Dedupe by `id`, first wins; every dropped duplicate → `warnings` += `targets: duplicate id <id> dropped (<algorithm>/<category> <file>:<line>)`.
- Filter by `--min-severity` (`output::severity_rank`) and `--min-tier`; sort by `(site.file, site.line, source_algorithm, category, id)`.

Document-level: `schema_version "1.0"`; `producer { tool: "prism", version, resolution_mode: RESOLUTION_MODE, build_identity: build_info().build_identity, algorithms: meta.algorithms_run }` (all required); `repo { root, sha? }` (required, `root` required); `diff { sha256: inputs.diff_text_sha256, files: inputs.diff.files[].file_path }` (required); `errors` (omitted when empty); `warnings` = `meta.run_warnings` ++ projection warnings (omitted when empty) — the machine-readable partial-coverage signal (sol r2 W3/W4, r3 S1). Every target carries `description` and `parse_quality` (required in the schema).

**Comparability (Opus Q1):** the schema's top-level description names the stable keys a consumer may diff on across runs and machines — `targets[].id`, `site.{file,line,symbol}`, `kind`, `category`, `expected`, `source_algorithm`, `severity` — and the envelope fields it must ignore — `producer.{version,build_identity}`, `repo.root`, `repo.sha`, `diff.sha256`.

**2.4.3 Schema custody and versioning.** `docs/contracts/targets.schema.json` in this repo is authoritative. `schema_version` is the constant `"1.0"` in this schema (`const`, not a pattern — an exact minor discriminator, sol r3 W2); each minor ships as its own schema file (`docs/contracts/targets.schema.json` is always the current minor; prior minors are kept as `targets-<minor>.schema.json` when superseded). Every object is `additionalProperties: false` (closed world), so **any newly emitted property is a minor bump** and a document with `schema_version` `1.k` validates **only** against schema `1.k` — consumers fetch the schema matching the document's minor. Minor bumps are backward-compatible in *meaning* (fields are only added, existing semantics kept), not in closed-world validation; breaking changes bump the major (sol r1 #21, r2 #21 — closed world is deliberate: a harness must not silently ignore a field it does not understand). The in-repo projection test (§7.4.5) is a hand-written structural checker; the controller gate §8.8 validates one real document with a full Draft 2020-12 validator (Python `jsonschema`) because no Rust dependency may be added.

### 2.5 README truth pass (docs only; no code) — with a gate

Apply `grounding/readme-truth.md` row by row. Binding corrections:
1. Every `slicing …` CLI example → `prism …`; delete the "binary is named slicing for historical reasons / rename planned" paragraph.
2. MCP: eight tools (six `nav_*` + `taint_reaches` + `refresh_index`), matching `src/mcp/registry.rs::ToolRegistry::all_v1`.
3. Output formats: `text`, `json`, `paper`, `review`, `callers`, `mermaid`, `sarif` (with one-line descriptions and a SARIF upload snippet for GitHub code scanning); options table row updated.
4. `tests/fixtures/cve/` → the real locations (`tests/fixtures/c/cve_*.c`, `tests/fixtures/sanitizer-suite-*`, `eval/fixtures/`).
5. Framework list, nav subcommand list, language count, algorithm count, cache-scope paragraph (CPG cache per-diff and opt-in via `--cache-dir`; nav index whole-repo under `dirs::cache_dir()/prism/nav/<hash>/`), version/install — each to the truth column of the grounding table.
6. New sections: `prism targets` (with the schema pointer and the acceptance table) and `Library use (prism::api)` with the compatibility promise verbatim from §2.3.1 and the `review()` snippet that is also the doc-test.
7. `skills/prism-code-slicing/SKILL.md` "Output formats" gains `sarif` and `targets`; `CLAUDE.md` gets `api`, `targets`, `finding_confidence`, `output/sarif.rs` in its module maps and the four-value severity vocabulary; `src/slice.rs:27` doc comment corrected to the four values.
8. **Gate:** `tests/cli/readme_test.rs` — (a) every fenced code block line in README.md that starts with `prism ` and contains no `<` placeholder is split shell-style and parsed with `prism::cli::Cli::try_parse_from` (parse only; no execution; the clap structs move to `src/cli.rs` for this — §6); (b) the README's documented format list equals the `--format` `value_parser` array (both extracted, compared as sets).

**Explicitly not done:** the clap `name = "slicing"` (`src/main.rs:38`) is left as is — the `--version` line grammar `slicing <ver> (<sha>)` has two consumers (`tests/cli/version_test.rs:15` and `eval/tier_a/sut.py::parse_version`, per the IDENTITY GRAMMAR comment in `build.rs:45-56`); renaming it is a coordinated change (§9).

## 3. Compatibility

- **Byte-identical**: `text`, `json`, `paper`, `review`, `callers`, `mermaid` for every existing invocation, single- and multi-algorithm — stdout, stderr and exit status (gate §8.5). No `SliceFinding` field is added or renamed; no algorithm file is touched.
- **CLI grammar**: `--format` becomes validated (unknown → clap error, disclosed §2.2.3); `targets` subcommand added; nothing removed.
- **Caches (sol #15):** the cache **format** and **decision tree** are unchanged: CPG cache version stays `55`, nav sidecar `24`, `SKIP_POLICY_VERSION` `2`, and `build_context` reproduces the Hit/PartialHit/Miss/scoped tree exactly. What `cpg_cache::current_cache_build_identity()` returns is a **binary build identity** — a sha256 over this binary's own source inputs (`build.rs:189-203`), identical across repos and diffs and unrelated to the CPG's inputs (those are keyed by `compute_file_hashes`/`compute_topology_key`). The facade and the serializers name it `build_identity` for that reason. As with **every** prism source change, the first run of the branch binary misses on any cache written by another build; §8.6 therefore measures each binary against its own cache directory.
- **Library**: `prism::api` is new; existing `pub` paths are untouched.
- **MCP**: untouched.

## 4. Non-goals

DataFlow confidence / reaching definitions (item 2); SCIP rung (item 3); boundary nodes (item 4); workspace split; typed call-edge exposure through `api`; `angle`/`delta` findings; fixing the multi-run `paper` degradation; renaming the clap command; a structured `FindingHint` on `SliceFinding`; structured `related` locations (v2 of the targets schema); repairing lossy anchors (provenance origin, symmetry first-diff-line); SARIF `codeFlows`; any new crate dependency.

## 5. Failure directions (binding for reviewers)

1. **Confidence never over-claims.** Unknown algorithm string, any CPG use, any non-clean quality of any evidence-bearing file, or an unknown quality → `unlabeled`/`candidate` as applicable. `asserted` is a claim about the evidence path, not about the heuristic's truth.
2. **Targets never invent a hint or a location.** A hint comes only from a verbatim format-string token or a closed-table row with exactly one counterpart; a `kind` never claims an executable site the anchor is not; function bounds describe the innermost function enclosing `site.line` and are emitted whenever that function exists (they are what the harness scopes injection to); a disagreement with the finding's named function is recorded as a warning, never guessed away (v5 rule, §2.4.2).
3. **Nothing is dropped silently.** SARIF: every finding becomes a result; unattributable related lines go to `properties`. Targets: exactly three drop reasons exist — `line == 0`, a path that is absolute or escapes the repo root, and a duplicate id — each recorded in `warnings`; every normalisation (severity, slashes) is recorded.
4. **The facade changes no behaviour.** Moves are moves; the byte control (§8.5: stdout + stderr + exit, single and multi, including a poor-parse fixture, an algorithm error, and a strict-diagram case) and the per-binary cache-decision control (§8.6) are the proof.
5. **Unknown severities become louder, not quieter.** SARIF `error`, targets `concern`, original preserved.

## 6. Permitted implementation files

New: `src/finding_confidence.rs`, `src/output/sarif.rs`, `src/output/sarif_rules.rs` (rule descriptions + pure mapping helpers; split for the 600-line cap — Task 2 record), `src/output/sarif_model.rs` (the serde model structs) and `tests/cli/sarif_shape_test.rs` (second SARIF test file) — both permitted so neither `sarif.rs` nor `sarif_test.rs` sits at the cap (Task 2 review), `src/targets/{mod,model,mapping}.rs`, `src/api/{mod,build_info,review,run,nav}.rs`, `src/cli.rs` (the clap derive structs moved out of main.rs so `tests/cli/readme_test.rs` can call `Cli::try_parse_from` — a pure move, covered by the byte control), `tests/cli/sarif_test.rs`, `tests/cli/targets_test.rs`, `tests/cli/readme_test.rs`, `tests/integration/api_test.rs`, `tests/integration/targets_mapping_test.rs`, `tests/fixtures/targets/**`, `scripts/phase0-byte-control.sh`, `docs/superpowers/plans/2026-09-04-prism-phase0-*.md`, `docs/superpowers/handoffs/2026-09-04-prism-phase0-*.md`.
Modified: `src/lib.rs` (`pub mod` lines), `src/output/mod.rs` (one `pub mod sarif;` + re-export), `src/main.rs` (format validation, sarif arms, `Targets` subcommand, helpers moved out, `default_value_t = api::DEFAULT_*`), `src/slice.rs` (doc comment on `severity` only), `tests/cli/main.rs` and `tests/integration/main.rs` (module registrations), `docs/contracts/targets.schema.json`, `README.md`, `CLAUDE.md`, `skills/prism-code-slicing/SKILL.md`.
**Forbidden:** `src/algorithms/**`, `src/cpg/**`, `src/cpg_cache.rs`, `src/navigation/**`, `src/resolution*.rs`, `src/call_graph.rs`, `src/ast.rs`, `src/languages/**`, `Cargo.toml` dependencies (no new deps; `sha2` and `serde_json` exist), any cache version constant, `tests/integration/coverage_test.rs`. If compiled reality requires touching one of these, stop and amend this design.

## 7. Tests (TDD; each names its observable)

Run with `cargo test --test cli sarif_test::`, `cargo test --test cli targets_test::`, `cargo test --test cli readme_test::`, `cargo test --test integration api_test::`, `cargo test --test integration targets_mapping_test::`, and `cargo test --lib finding_confidence`.

### 7.1 `finding_confidence` (unit, in-module)
1. `("absence", Clean)` → `(Exact, Asserted)`.
2. `("echo", Clean)` → `(Unlabeled, Candidate)`.
3. `("absence", Degraded)` → `(Exact, Candidate)`; `("absence", Unknown)` → `(Exact, Candidate)`.
4. Every string in `["absence","callback_dispatcher","contract","echo","membrane","peer_consistency","primitive","provenance","symmetry","taint"]` round-trips through `SlicingAlgorithm::from_str`.
5. `("not_an_algorithm", Clean)` → `(Unlabeled, Candidate)`.
6. Serde: `Exact` → `"exact"`, `Unlabeled` → `"unlabeled"`, `Candidate` → `"candidate"`.
7. `ParseQuality::min_over(files, map, parsed)`: `a.py` parsed and absent from the sparse map → `Clean`; `["a.py","b.py"]` with `b.py: degraded` in the map → `Degraded`; `["a.py","missing.py"]` where `missing.py` is in neither → `Unknown`; empty `files` → `Unknown`; a map quality string outside the four → `Unknown`.
9. `parse_quality_for`: a `contract_precondition_weakened` finding on a clean parsed file → `Unknown`; a `contract_violation` finding on the same file → `Clean`; a symmetry finding whose `related_files[0]` is degraded → `Degraded`.
8. `evidence_files`: a symmetry finding (`file: a.py`, `related_files: [b.py]`) → `["a.py","b.py"]`; no related files → `["a.py"]`.

### 7.2 SARIF (`tests/cli/sarif_test.rs`, structural, via `Command::cargo_bin("prism")`)
1. Single `--algorithm absence --format sarif` on a temp Python repo with `open()` without `close()`: stdout parses; `version == "2.1.0"`; `runs[0].tool.driver.name == "prism"`; exactly one rule `prism/absence/missing_counterpart` with non-empty `fullDescription.text`; the result has `level == "warning"`, `artifactLocation.uri == "a.py"`, `uriBaseId == "%SRCROOT%"`, `region.startLine == <open line>`, `properties.confidence == "exact"`, `tier == "asserted"`, `parse_quality == "clean"`, `resolution_mode == "nominal"`; **for every result** `rules[result.ruleIndex].id == result.ruleId`; no `originalUriBaseIds` key. Build-specific keys of §2.2.1 are present but never compared.
2. Multi `--algorithm echo,absence --format sarif` on a fixture where a changed function's caller lacks error handling: a result with ruleId `prism/echo/missing_error_handling` has `confidence == "unlabeled"`, `tier == "candidate"`; `runs[0].properties.algorithms_run == ["EchoSlice","AbsenceSlice"]`; the `(uri,startLine,ruleId)` tuple sequence is non-decreasing; the ruleIndex correspondence holds for every result.
3. `--algorithm chop,absence --format sarif` without `--chop-source`: `executionSuccessful == false`; one notification with `level == "error"` containing `--chop-source required`; `properties.errors[0].algorithm == "Chop"`; the absence result is still present.
4. `--format sarif` twice on the same input (single deterministic algorithm) → identical stdout bytes.
5. `--format bogus`, `--format Json` → exit 2, stderr contains `invalid value`; `--format ""` → exit 2 with clap's `a value is required` (an empty string is rejected before value matching — Task 2 record).
6. Unit: `line: 0` → no `region`; `related_lines: [5,0,3,5]`, `related_files: ["b.py"]`, algorithm `absence` (SameFile) → three `relatedLocations` (ids `0,1,2`; lines `3,5` in `a.py`; `b.py` without region); algorithm `symmetry` with `related_lines: [10,20]`, `related_files: ["b.py"]` → two locations in `b.py` at lines 10 and 20 and no location in `a.py`; algorithm `callback_dispatcher` with lines and a related file → files only, lines under `properties.related_lines`.
7. Unit: severity map covers the four vocabulary values; `"critical"` → `"error"` with `properties.severity == "critical"`.
8. Unit: fingerprint of two findings whose descriptions differ only in `(line 12)`/`(line 13)` and share line text is identical; differing `line_text` or `function_name` changes it.
9. Unit: `uri("dir with space/a b.py")` → `dir%20with%20space/a%20b.py`; `uri("a\\b.py")` → `a/b.py`; `uri("../x.py")` → emitted plus a warning notification.
10. A degraded-parse fixture (a Python file with a syntax error above the finding): the absence result has `tier == "candidate"`, `parse_quality == "degraded"`.

### 7.3 `prism::api` (`tests/integration/api_test.rs`, in-process)
1. `api::review(&ReviewOptions::new(repo), diff_text, &[AbsenceSlice], &SliceConfig::default(), &AlgorithmParams::default())` on the §7.2.1 fixture: `outcome.run.findings` has one `category == Some("missing_counterpart")`; `outcome.inputs.diff.files.len() == 1`; `outcome.inputs.parse_quality` does **not** contain `a.py` (sparse map; clean) while `outcome.inputs.files` does. A diff that also names `notes.txt` (unsupported language) → `outcome.inputs.load_warnings == ["skipped unsupported file: notes.txt (unsupported language)"]` and `run.warnings` is unchanged (sol r2 W4).
2. Two-phase API without an outer `with_build_pool`: `let built = build_context(&inputs, &opts)?; run_review(&built.ctx, &inputs, &[EchoSlice, AbsenceSlice], ..)` returns `algorithms_run == ["EchoSlice","AbsenceSlice"]`, `results.len() == 2`, `results[0].algorithm == EchoSlice`, `results[1].algorithm == AbsenceSlice`, `errors.is_empty()`, `warnings == inputs.parse_warnings`. And `run_review(&[Chop, AbsenceSlice], ..)` with default params → `algorithms_run == ["Chop","AbsenceSlice"]`, `results.len() == 1`, `results[0].algorithm == AbsenceSlice`, `errors[0].algorithm == "Chop"` (sol r2 W2: results are the successful subsequence).
3. `AlgorithmParams::default().barrier_depth == api::DEFAULT_BARRIER_DEPTH` (and spiral/temporal), and `prism --help` shows `[default: <DEFAULT_BARRIER_DEPTH>]` for `--barrier-depth` (parsed from the help text) — one source, two consumers checked.
4. `run_algorithm(Chop, …, &AlgorithmParams::default())` → `Err` containing `--chop-source required`.
5. `build_info()`: `package_version == env!("CARGO_PKG_VERSION")`, `build_identity.len() == 64`.
6. `nav_session(repo, &{ let mut o = NavOptions::default(); o.no_cache = true; o })` with no outer install, then `callers(&s, Seed::Symbol("helper"), 1, false)` on a two-file Python fixture returns evidence whose JSON contains the caller function name.
7. Doc-test on `api::review` (the README snippet) compiles and runs.
8. `readme_test.rs` greps `src/api/*.rs`: every `pub struct`/`pub enum` is preceded by `#[non_exhaustive]`.

### 7.4 `prism targets` (`tests/cli/targets_test.rs` live; `tests/integration/targets_mapping_test.rs` unit)
1. Default run on `tests/fixtures/targets/` — the fixture is built so that **all five default producers emit at least one finding** (sol #20): `svc.py` calls `fetch()` from `client.py` without handling (echo, and a cross-file unprotected caller for membrane), `open()` without `close()` (absence), a guard-clause change with `--old-repo` pointing at a copy of the pre-change tree or a same-tree guard modification on a diff line (contract — implementer picks whichever the algorithm reliably triggers and says which in a comment), a `request.args` read flowing to a diff line (provenance). Asserts: parses; `schema_version == "1.0"`; `producer.tool == "prism"`; `producer.resolution_mode == "nominal"`; `producer.algorithms` equals the five names; `diff.files` non-empty; at least one target per `source_algorithm ∈ {echo, absence, contract, provenance, membrane}`.
2. Echo target: `kind == "external_call"`, `expected.property == "error_handled"`, `dependency_hint.callee == "fetch"`, `confidence == "unlabeled"`, `tier == "candidate"`, `site.symbol` is the caller, `function_start_line <= line <= function_end_line`, `site.language == "python"`. Membrane target: `kind == "boundary"`, `dependency_hint.callee` names the changed function (live hint assertion — sol #25).
3. Absence target: `kind == "resource_acquire"`, `expected.property == "resource_released"`, `dependency_hint.counterpart == "close"`, `dependency_hint.kind == "filesystem"`, `confidence == "exact"`, `tier == "asserted"`, `parse_quality == "clean"`. Provenance target: `kind == "other"`, `expected.property == "origin_trusted"`, `expected.detail` contains `origin at use site`. Symmetry (add `serialize_x`/`deserialize_x` to the fixture with only one changed; run `--algorithm symmetry`): `kind == "contract"`, `dependency_hint.counterpart` names the unchanged counterpart, `site.symbol` is the function enclosing `site.line`, `function_start_line <= site.line <= function_end_line` (bounds are emitted whenever the enclosing function exists — sol r3 W3), and if the enclosing name differs from the finding's named function a `warnings` entry says so (sol r2 #9/#25).
4. `id` is 64 hex chars; running twice gives byte-identical stdout; `--min-tier asserted` keeps the absence target and removes every `candidate`; `--min-severity concern` removes the absence target.
5. Every emitted document passes the in-repo structural checker over `docs/contracts/targets.schema.json` (required keys per object, enum membership, id regex, `additionalProperties` sets, integer minima).
6. Acceptance table: `--algorithm chop` → exit 1 + `requires --chop-source`; `--algorithm delta` without `--old-repo` → exit 1 + `requires --old-repo`; `--algorithm leftflow` → exit 1 + `produces slice blocks, not findings`; `--algorithm angle` → exit 0, `targets == []`, stderr `produces no findings`; a run with a recorded algorithm error and `--strict` → exit 3 and `errors` non-empty (implementer constructs the case and documents it).
7. `--out <file>` writes the same bytes as stdout would; stdout empty.
7b. A diff naming an unsupported-language file (`notes.txt`) → `warnings` contains `skipped unsupported file: notes.txt (unsupported language)`; a JSON diff naming a file that does not exist → exit 1 with the base's `Failed to read source` error (unreadable stays fatal — sol r3 W1); a JSON diff naming `/abs/elsewhere.py` that DOES exist → no target for it and `warnings` contains `dropped finding with path escaping repo root` if a finding was produced for it — implementer records which.
8. Unit (mapping): one case per table row with the verbatim description formats (echo, membrane, four absence categories incl. a table row with no counterpart, both `contract_violation` shapes → `precondition_holds`/`postcondition_holds`, an unrecognised `contract_violation` text → `unknown`, provenance `database`/`user_input`, symmetry, unknown category, `category: None` → `"uncategorized"`, severity `"critical"` → `"concern"` + warning); `line: 0` → dropped + warning; two identical ids → one target + duplicate warning; symmetry finding whose containing function ≠ `function_name` → bounds omitted + warning; `Language::all()` lowering ⊆ schema enum; path `a\\b.py` → `a/b.py`; `../x.py` → warning.

### 7.5 README gate (`tests/cli/readme_test.rs`) — §2.5.8.

## 8. Acceptance gates (controller, before review and before PR)

| Deliverable | Discriminating evidence |
|---|---|
| SARIF | §7.2 + §8.8 official-schema validation of one real document |
| targets | §7.4.5 + §8.8 full Draft 2020-12 validation of one real document + §7.4.2/3 live field assertions |
| facade | §8.5 byte control (stdout+stderr+exit; single + multi) + §8.6 per-binary cache decisions |
| README | §7.5 |

1. `cargo fmt --all -- --check` → exit 0.
2. `cargo clippy --all-targets --all-features -- -D warnings` → exit 0 (or the pre-existing warning set, diffed against the exact base built in the same worktree — new warnings only are blockers).
3. Focused tests (§7) GREEN. RED is observed on the exact base for the CLI-level tests by running the base binary (`~/code/tools/bin/prism-base-c220525`, sha256 `299f02c4f15c4e7d…`) with the same invocations (`--format sarif` renders text on base; `targets` is an unknown subcommand). Unit tests that cannot compile on base are recorded as "feature absent".
4. Full suite: `cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee <log>`; totals by `awk` over every `test result:` line of the complete log (never `tail`); base was `3543 passed / 0 failed / 1 ignored` (`~/code/tools/logs/baseline-c220525.log`). Expected: base totals + new tests, 0 failed.
5. **Same-base control (facade proof):** `scripts/phase0-byte-control.sh <base-bin> <branch-bin>` runs both binaries over **every** checked-in fixture diff, enumerated by the script itself (`find tests/fixtures -name '*.diff' -o -name '*.patch' -o -name 'diff.json'`, each paired with its repo directory by the layout convention the existing tests use; the script prints the list it found and fails if it is empty; at this base the population includes at least `python/calc.diff`, `hapi-4552.diff`, `bash/firmware_update.diff`, `terraform/main.diff`, `nav_compat/*`, `review_no_diagrams/*` — sol r2 S1) plus a generated **poor-parse fixture** (a file with >10% error nodes) with: (a) single algorithms `leftflow`, `absence`, `contract`, `echo`, `membrane`, `provenance`, `primitive` × formats `text`, `json`, `paper`, `review`, `mermaid`; (b) multi sets `echo,absence,contract` and `absence,contract,primitive` × `text`, `json`, `paper` (today's text fallback), `review`, `mermaid`; (c) `chop,absence --format json` (an `errors[]` entry); (d) `--format callers`; (e) one `--strict-diagrams` invocation on a diagram-warning fixture — capturing **stdout, stderr and exit status** for each and diffing all three (sol #19). Taint is excluded (documented non-byte-stability). Expected: zero differing invocations. Any difference is a STOP.
6. **Per-binary cache-decision control:** for each binary with its **own** empty `--cache-dir`: run 1, run 2, edit one fixture file, run 3; the observable per run is `(cpg-cache.bin exists?, mtime changed vs previous run?)` → expected `(created), (unchanged), (changed)` for both binaries. Compare the two sequences, not the artifacts.
7. Tier-A: `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` → same pass count as base (tripwire only).
8. One real SARIF document validated against the official SARIF 2.1 JSON schema, and one real targets document validated against `docs/contracts/targets.schema.json`, both with Python `jsonschema` (Draft 2020-12), controller-side, commands and output recorded in the handoff.
9. Do not rebaseline Tier-A; a suite or corpus failure is attributable only after the exact base fails or passes the same probe in the same environment.

## 9. Follow-ups filed (not in this PR)

- Multi-run `paper` silently degrades to text (`src/main.rs:1095`).
- Rename clap `name = "slicing"` → `prism` together with `tests/cli/version_test.rs:15` and `eval/tier_a/sut.py::parse_version` (IDENTITY GRAMMAR, `build.rs:45-56`).
- `angle_slice` and `delta_slice` emit no findings.
- Lossy anchors: `provenance_slice` discards `origin_line/origin_file`; `symmetry_slice` anchors on the file's first diff line; `callback_dispatcher` anchors on the definition — each repair changes `json` bytes and needs its own byte-pinned PR.
- Structured `FindingHint` (`#[serde(skip)]`) and structured `related` locations (targets schema v2), replacing the closed table, the format-string parses and the attribution table.
- Typed call edges through `prism::api`; `cargo-semver-checks` in CI once a downstream consumer exists.
- Item 2 (DataFlow confidence + reaching definitions): `classify_with_evidence`, `nameonly`, `--min-confidence`.

## 10. Implementation sequencing (each task PR-sized within the one branch; one PR at the end)

1. `finding_confidence.rs` + unit tests (§7.1).
2. `output/sarif.rs` + `--format` allow-list + both arms + `tests/cli/sarif_test.rs` (§7.2). (Reads the parse-quality map and sources from main.rs's locals; task 3 moves them into `ReviewInputs`.)
3. `api/` (build_info, review, run, nav) with main.rs consuming it; `tests/integration/api_test.rs` (§7.3); gates §8.5/§8.6 run here, before anything else builds on the facade.
4. `targets/` + `Command::Targets` + fixture + `tests/cli/targets_test.rs` + `tests/integration/targets_mapping_test.rs` (§7.4).
5. README/CLAUDE.md/SKILL.md truth pass (§2.5) + `readme_test.rs` + doc-test.
6. Closeout: full gates §8, handoff, PR.

## 11. Review and convergence record

Design review cap: **2 rounds** (sol gate via bridge, read-only; Opus parallel seat). Owner authorisation (2026-09-04, verbatim in `~/code/tools/LEDGER.md`): the cap may be exceeded as needed; a disputed sol finding may be adjudicated by a **separate sol judge seat** on whether it is out of scope, deferred, or an implementation detail.

- **Round 1 — Opus seat** (`~/code/tools/reviews/phase0-spec-r1-opus.md`): FIX, W=9 S=15. All folded in v2 (W1 pipeline/`ReviewOutcome`/`TargetsMeta`; W2 Language lowering; W3 parse-quality fail-safe; W4 cache identity; W5 wider byte control; W6 digit-masked fingerprint; W7 closed absence table; W8 build-specific fields; W9 confidence enum `exact|nameonly|unlabeled`; S10–S24 as listed in v2). Q1 comparability keys → schema; Q2 `asserted` = evidence-path claim; Q3 hints only from verbatim/closed sources.
- **Round 1 — sol seat** (`~/code/tools/reviews/phase0-spec-r1-sol.md`, reviewed v1): FIX, W=22 S=9. Folded in v3 — #1/#2/#3 authoritative parse-quality map, `evidence_files`, `min_over`, `classify_with_evidence` deferred (§2.1); #4 = Opus W1; #5 `contract_violation` discrimination; #6 `close_only_on_error_path` → `resource_acquire`; #7 provenance → `other` + Q2 ruling; #8 related-line attribution table; #9 bounds must agree with `function_name`; #10 URI encoding / path normalisation; #11 totality rules; #12 unknown severity → louder; #13 no `regex`; #14 canonical-JSON id incl. severity; #15 `build_identity` naming + §3; #16/#17 compatibility boundary and version wording; #18 `result.warnings` kept explicitly; #19 control compares stdout+stderr+exit incl. multi-paper, poor-parse, error, strict-diagram; #20 ruleIndex per result, shared `DEFAULT_*` consts, per-result algorithm check, all five producers live, full validator in gate §8.8; #21 minor-bump policy; #22 = Opus S19; #23 no `originalUriBaseIds`; #24 ownership wording; #25 live hint assertions; #26 = S10 + name agreement; #27 = W2; #28 = S16/S17; #29 line-text discriminator; #30 script in §6; #31 module/test split. sol Q1 → the `ReviewInputs.parse_quality` map; Q2 → lossy anchors allowed, `kind` tells the truth; Q3 → handles-stable boundary (§2.3.1).
- **Round 2 — sol seat (v3)** (`~/code/tools/reviews/phase0-spec-r2-sol.md`): FIX, W=4 S=1 I=1, 21 FOLDED / 10 PARTIAL of the round-1 items. Folded in v4: W1 sparse parse-quality map → `min_over(files, map, parsed)` + `parse_quality_for` (§2.1); W2 `ReviewRun.results` = successful subsequence + error-before-success test (§2.3.2, §7.3.2); W3 schema `required` sets now cover every always-emitted field (schema, §2.4.2); W4 `ReviewInputs.load_warnings` surfaced in targets `warnings` and SARIF notifications, legacy `warnings`/stderr unchanged (§2.3.2, §2.2, §2.4.2); S1 byte control enumerates fixtures programmatically (§8.5); I1 → plan Task 2 (fullDescription table). PARTIALs: #2 delta-contract categories → Unknown (`parse_quality_for`); #9 `site.symbol` = enclosing function, warning on disagreement; #10 escaping paths dropped with warning; #11 `unknown` added to the schema enum; #14 schema id formula updated to the canonical-JSON array, first-wins dedupe with warning kept (documented); #16 `NavOptions: Default`; #21 closed-world validation stated (a 1.k document validates only against schema 1.k); #25 symmetry added to the live fixture, `double_close` stays unit-only (C goto fixture cost; disclosed); #29 identical call-site lines share a fingerprint — accepted and documented (location disambiguates in GitHub); #1 = W1. sol Q1 → sparse map retained for legacy, total classification via `parsed`; Q2 → closed world is deliberate; Q3 → success-only sequence, documented.
- **Round 3 — sol seat (v4), disclosed extension** (`~/code/tools/reviews/phase0-spec-r3-sol.md`): FIX, W=3 S=1 I=0; 12 FOLDED, 3 PARTIAL, 1 NOT FOLDED of round 2. Folded in v5: W1 unreadable files stay fatal (only unsupported-language skips are load warnings); W2 `schema_version` is `const "1.0"` with per-minor schema files; W3 bounds are emitted whenever an enclosing function exists, disagreement is a warning only; S1 warnings ownership (`TargetsMeta.run_warnings` assembled by the CLI, `project` appends projection warnings); r2-W4 residual: `BuiltContext.warnings` carries cache/type-db non-fatal conditions, `ReviewOutcome.build_warnings`; #10 §5.3 lists the three drops; #14 id includes sorted related lines/files; #29 GitHub matches on `primaryLocationLineHash` and ignores custom fingerprints — stated, consumers told to combine with location. **Residual, disclosed:** #25 `double_close` remains unit-only (a C goto fixture is out of proportion for Phase 0).
- **Controller ruling (settled):** three rounds, 31 → 6 → 4 findings, no repeats, round-3 items all closed-form and folded without dispute; no judge seat was needed. Implementation proceeds against v5. Any later finding against the design is handled in the per-task code review loop.

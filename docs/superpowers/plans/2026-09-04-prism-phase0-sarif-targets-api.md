# Prism Phase 0 (SARIF · `prism targets` · `prism::api` · README truth pass) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the analyzer roadmap its Phase 0 interfaces from prism — a SARIF 2.1 serializer, a `prism targets` projection into the pinned targets contract, a semver-stable `prism::api` facade that main.rs itself consumes, and a README that tells the truth — without touching the CPG, caches, or any algorithm.

**Architecture:** One new classification module (`finding_confidence`) is the single source both new serializers consult. The SARIF serializer and the targets projection are pure functions over `SliceFinding` + the authoritative parse-quality map + the parsed files. The facade lifts main.rs's private review pipeline (load → build context → run) into `src/api/` and main.rs becomes its first consumer; `prism targets` is its second. A same-base control (stdout + stderr + exit status, single and multi runs) proves the move changed nothing.

**Tech Stack:** Rust 2021 (existing crate `prism` 3.1.2), clap 4 derive, serde/serde_json, sha2 (existing), assert_cmd + predicates + tempfile (dev), bash for the control script, Python `jsonschema` (controller-side gate only).

**Spec:** `docs/superpowers/specs/2026-09-04-prism-phase0-sarif-targets-api-design.md` (v4). The spec is the binding authority; this plan argues from it. Section numbers below (§x.y) refer to the spec. Plan v2 (2026-09-04): aligned to spec v4 — sparse parse-quality map (`parse_quality_for`), `load_warnings`, `ReviewRun.results` = successful subsequence, `NavOptions: Default`, symbol/escaping-path rules, live symmetry check. Plan v3: aligned to spec v5 (settled) — `build_context` returns `BuiltContext { ctx, warnings }`, `ReviewOutcome.build_warnings`, `TargetsMeta.run_warnings`, unreadable files stay fatal, bounds always emitted when an enclosing function exists, id includes sorted related lines/files, `schema_version` const.

## Global Constraints

- Exact base `c220525c`; branch `phase0-sarif-targets-api`; worktree `/Users/wesleyjinks/code/slicing-phase0`. Never touch `/Users/wesleyjinks/code/slicing`.
- **No new crate dependencies** (`Cargo.toml [dependencies]` unchanged; `sha2`, `serde_json`, `clap`, `dirs` already exist). No `regex`, no `jsonschema`, no `serde-sarif`.
- **No cache version bump**: `CACHE_VERSION` stays `55`, nav sidecar `24`, `SKIP_POLICY_VERSION` `2`.
- **Forbidden files** (spec §6): `src/algorithms/**`, `src/cpg/**`, `src/cpg_cache.rs`, `src/navigation/**`, `src/resolution*.rs`, `src/call_graph.rs`, `src/ast.rs`, `src/languages/**`, `tests/integration/coverage_test.rs`. If a task cannot be completed without touching one, STOP and report BLOCKED.
- **Byte-identical existing outputs**: `text`, `json`, `paper`, `review`, `callers`, `mermaid` — stdout, stderr and exit status — for single and multi runs. Proven by `scripts/phase0-byte-control.sh` (Task 3), which any later task must keep green.
- **600-line cap** per source and test file (`CLAUDE.md`); split before approaching it (module layout is fixed in each task).
- Every `pub struct`/`pub enum` in `src/api/**` and in `src/finding_confidence.rs` carries `#[non_exhaustive]`.
- Severity vocabulary is four values: `info`, `suggestion`, `warning`, `concern`. Confidence vocabulary: `exact`, `nameonly`, `unlabeled`. Tier: `asserted`, `candidate`. Parse quality: `clean`, `degraded`, `poor`, `unparseable`, `unknown`. `RESOLUTION_MODE = "nominal"`.
- Rule ids are `prism/<finding.algorithm>/<category or "uncategorized">`. Targets `schema_version` is `"1.0"`.
- Tests: `cargo test --test cli <module>::`, `cargo test --test integration <module>::`, `cargo test --lib finding_confidence`. Full suite before each commit that touches `src/main.rs` or `src/api/**`: `cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/phase0-suite-<task>.log`, totals via `awk '/^test result:/' ` over the whole log — never `tail`. Base totals: 3543 passed / 0 failed / 1 ignored.
- `cargo fmt --all` before every commit; `cargo clippy --all-targets --all-features -- -D warnings` must not add warnings.
- Commit messages: imperative subject ≤ 72 chars with a `feat(phase0):`/`test(phase0):`/`docs(phase0):` prefix; body names the spec section; end with `Co-Authored-By: Claude <Model name> <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT`.
- Implementers do not dispatch subagents and do not push.

---

### Task 1: `finding_confidence` — single-source confidence/tier classification

**Files:**
- Create: `src/finding_confidence.rs`
- Modify: `src/lib.rs` (add `pub mod finding_confidence;` in alphabetical position)
- Test: unit tests inside `src/finding_confidence.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `crate::slice::{SliceFinding, SlicingAlgorithm, FileParseQuality}`.
- Produces (used verbatim by Tasks 2 and 4):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingConfidence { Exact, NameOnly, Unlabeled }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum FindingTier { Asserted, Candidate }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ParseQuality { Clean, Degraded, Poor, Unparseable, Unknown }   // declaration order = best → worst

pub const RESOLUTION_MODE: &str = "nominal";

impl ParseQuality {
    pub fn from_quality_str(q: &str) -> Self;   // "clean"|"degraded"|"poor"|"unparseable" → variant; anything else → Unknown
    /// Worst quality over `files`. The map is SPARSE (only files with error rate > 1%): per file — in map → grade;
    /// else in `parsed` → Clean; else Unknown. Empty `files` → Unknown.
    pub fn min_over(files: &[&str], map: &BTreeMap<String, FileParseQuality>, parsed: &BTreeMap<String, ParsedFile>) -> Self;
    pub fn as_str(self) -> &'static str;        // lowercase name
}

/// Anchor file followed by every `related_files` entry (deduplicated, order preserved).
pub fn evidence_files(finding: &SliceFinding) -> Vec<&str>;
/// Delta-contract categories (contract_precondition_weakened/strengthened, contract_postcondition_weakened/strengthened) → Unknown;
/// otherwise min_over(evidence_files(finding), map, parsed). The entry point both serializers use.
pub fn parse_quality_for(finding: &SliceFinding, map: &BTreeMap<String, FileParseQuality>, parsed: &BTreeMap<String, ParsedFile>) -> ParseQuality;

pub fn classify(algorithm: &str, parse_quality: ParseQuality) -> (FindingConfidence, FindingTier);
```

- [ ] **Step 1: Write the failing tests** (in `src/finding_confidence.rs`, module `tests`; the module does not compile yet, so the RED observation is "module absent"):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::slice::{FileParseQuality, SliceFinding};
    use std::collections::BTreeMap;

    fn fpq(q: &str) -> FileParseQuality {
        FileParseQuality { error_count: 0, node_count: 1, error_rate: 0.0, quality: q.to_string(), error_lines: vec![] }
    }
    fn finding(algorithm: &str, file: &str, related: &[&str]) -> SliceFinding {
        SliceFinding {
            algorithm: algorithm.into(), file: file.into(), line: 1, severity: "warning".into(),
            description: String::new(), function_name: None, related_lines: vec![],
            related_files: related.iter().map(|s| s.to_string()).collect(),
            category: None, parse_quality: None, diagrams: vec![],
        }
    }

    #[test] fn ast_only_clean_is_exact_asserted() { assert_eq!(classify("absence", ParseQuality::Clean), (FindingConfidence::Exact, FindingTier::Asserted)); }
    #[test] fn cpg_algorithm_is_unlabeled_candidate() { assert_eq!(classify("echo", ParseQuality::Clean), (FindingConfidence::Unlabeled, FindingTier::Candidate)); }
    #[test] fn degraded_or_unknown_parse_is_candidate() {
        assert_eq!(classify("absence", ParseQuality::Degraded), (FindingConfidence::Exact, FindingTier::Candidate));
        assert_eq!(classify("absence", ParseQuality::Unknown), (FindingConfidence::Exact, FindingTier::Candidate));
    }
    #[test] fn every_production_algorithm_string_parses() {
        for s in ["absence","callback_dispatcher","contract","echo","membrane","peer_consistency","primitive","provenance","symmetry","taint"] {
            assert!(crate::slice::SlicingAlgorithm::from_str(s).is_some(), "{s} must round-trip through from_str");
        }
    }
    #[test] fn unknown_algorithm_is_unlabeled_candidate() { assert_eq!(classify("not_an_algorithm", ParseQuality::Clean), (FindingConfidence::Unlabeled, FindingTier::Candidate)); }
    #[test] fn serde_spellings() {
        assert_eq!(serde_json::to_string(&FindingConfidence::Exact).unwrap(), "\"exact\"");
        assert_eq!(serde_json::to_string(&FindingConfidence::Unlabeled).unwrap(), "\"unlabeled\"");
        assert_eq!(serde_json::to_string(&FindingTier::Candidate).unwrap(), "\"candidate\"");
        assert_eq!(ParseQuality::Poor.as_str(), "poor");
    }
    #[test] fn min_over_takes_the_worst_and_fails_unknown() {
        let mut map = BTreeMap::new();
        map.insert("a.py".to_string(), fpq("clean"));
        map.insert("b.py".to_string(), fpq("degraded"));
        assert_eq!(ParseQuality::min_over(&["a.py"], &map), ParseQuality::Clean);
        assert_eq!(ParseQuality::min_over(&["a.py", "b.py"], &map), ParseQuality::Degraded);
        assert_eq!(ParseQuality::min_over(&["a.py", "missing.py"], &map), ParseQuality::Unknown);
        assert_eq!(ParseQuality::min_over(&[], &map), ParseQuality::Unknown);
        map.insert("c.py".to_string(), fpq("weird"));
        assert_eq!(ParseQuality::min_over(&["c.py"], &map), ParseQuality::Unknown);
    }
    #[test] fn evidence_files_is_anchor_then_related() {
        assert_eq!(evidence_files(&finding("symmetry", "a.py", &["b.py"])), vec!["a.py", "b.py"]);
        assert_eq!(evidence_files(&finding("absence", "a.py", &[])), vec!["a.py"]);
        assert_eq!(evidence_files(&finding("echo", "a.py", &["a.py", "b.py", "b.py"])), vec!["a.py", "b.py"]);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --lib finding_confidence`
Expected: compile error (`finding_confidence` module not found) — record as "feature absent".

- [ ] **Step 3: Implement** `src/finding_confidence.rs` with the module doc from spec §2.1 verbatim (the "Asserted is a claim about the EVIDENCE PATH" paragraph included), the types above, and:
  - `classify`: `match SlicingAlgorithm::from_str(algorithm) { None => (Unlabeled, Candidate), Some(a) => { let conf = if a.needs_cpg() { Unlabeled } else { Exact }; let tier = if conf == Exact && parse_quality == Clean { Asserted } else { Candidate }; (conf, tier) } }`.
  - `min_over`: empty → `Unknown`; fold with `max` over `from_quality_str(&map[f].quality)`, any missing → `Unknown`.
  - `evidence_files`: `finding.file` then each `related_files` entry not already present.
- [ ] **Step 4: Register the module** in `src/lib.rs` (`pub mod finding_confidence;` between `pub mod diff;` and `pub mod framework_entries;`).
- [ ] **Step 5: Run to verify it passes**

Run: `cargo test --lib finding_confidence`
Expected: 8 passed.

- [ ] **Step 6: fmt + clippy + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
git add src/finding_confidence.rs src/lib.rs
git commit -m "feat(phase0): finding_confidence — single-source confidence/tier classification (spec §2.1)"
```

---

### Task 2: SARIF 2.1 serializer and `--format sarif`

**Files:**
- Create: `src/output/sarif.rs` (≤ 600 lines; if the rule `fullDescription` table plus structs approach the cap, move the table to `src/output/sarif_rules.rs`)
- Modify: `src/output/mod.rs` (`pub mod sarif;` + `pub use sarif::{to_sarif, SarifInputs};`), `src/main.rs` (`--format` value_parser; two `"sarif"` arms)
- Create: `tests/cli/sarif_test.rs`; Modify: `tests/cli/main.rs` (`mod sarif_test;`)

**Interfaces:**
- Consumes: Task 1 (`classify`, `parse_quality_for`, `ParseQuality`, `RESOLUTION_MODE`); `crate::slice::{SliceFinding, AlgorithmError, FileParseQuality}`; `crate::cpg_cache::{current_cache_build_identity, binary_input_dirty}`; `env!("CARGO_PKG_VERSION")`, `env!("GIT_SHA")`.
- Produces (used by Task 3's main.rs rewrite and by README):

```rust
pub struct SarifInputs<'a> {
    pub findings: &'a [SliceFinding],
    pub errors: &'a [AlgorithmError],
    pub parse_warnings: &'a [String],
    pub load_warnings: &'a [String],      // files skipped at load (spec §2.3.2); one `warning` notification each
    pub algorithms_run: &'a [String],
    pub parse_quality: &'a BTreeMap<String, FileParseQuality>,   // sparse authoritative map
    pub files: &'a BTreeMap<String, ParsedFile>,                 // parsed files (clean ⇔ parsed and absent from the map)
    pub sources: &'a BTreeMap<String, String>,
}
/// Deterministic SARIF 2.1 document (spec §2.2). Never fails; unrepresentable data is
/// preserved in `properties` and reported as notifications.
pub fn to_sarif(inputs: &SarifInputs) -> serde_json::Value;
pub fn sarif_uri(path: &str) -> (String, bool);   // (encoded uri, escapes_repo_root)
pub fn level_for_severity(severity: &str) -> &'static str;
pub fn fingerprint(finding: &SliceFinding, line_text: &str) -> String;
```

  `to_sarif` returns `serde_json::Value` built from typed `#[derive(Serialize)]` structs (`SarifLog`, `Run`, `Tool`, `Driver`, `Rule`, `Invocation`, `Notification`, `Result`, `Location`, `PhysicalLocation`, `ArtifactLocation`, `Region`, `LogicalLocation`, `RelatedLocation`, `ResultProperties`, `RunProperties`) then `serde_json::to_value`; struct field order = key order. The CLI prints `serde_json::to_string_pretty(&value)? + "\n"`.

- [ ] **Step 1: Write the failing CLI tests** `tests/cli/sarif_test.rs` (copy the `prism_cmd()` and `write_repo(files, diff_files)` helpers from `tests/cli/review_compact_test.rs:16-53` locally — CLI test files keep their own helpers):

```rust
// Fixture A (absence): a.py has `f = open("x")` on line 2 inside `def read():` and never closes it; diff = a.py lines [2].
// Fixture B (echo): client.py `def fetch(): ...` (changed, diff line inside it); svc.py `def handle(): return fetch()` with no try/except.
// Fixture C (degraded): a.py as in A but with a syntax error line `def broken(:` inserted above `read`.

#[test] fn single_algorithm_sarif_shape_and_rule_index() { /* §7.2.1: parse stdout; version == "2.1.0"; driver.name == "prism"; rules.len()==1; rules[0].id == "prism/absence/missing_counterpart"; fullDescription.text non-empty; result level "warning"; uri "a.py"; uriBaseId "%SRCROOT%"; region.startLine == 2; properties.confidence "exact", tier "asserted", parse_quality "clean", resolution_mode "nominal"; for every result: rules[ruleIndex].id == ruleId; runs[0] has no "originalUriBaseIds" key */ }
#[test] fn multi_algorithm_sarif_is_sorted_and_unlabeled_for_cpg() { /* §7.2.2 on fixture B with --algorithm echo,absence */ }
#[test] fn algorithm_error_becomes_notification_and_properties_error() { /* §7.2.3 with --algorithm chop,absence on fixture A */ }
#[test] fn sarif_is_byte_deterministic() { /* §7.2.4 */ }
#[test] fn unknown_format_values_are_rejected() { for v in ["bogus", "Json", ""] { /* exit code 2; stderr contains "invalid value" */ } }
#[test] fn degraded_parse_demotes_to_candidate() { /* §7.2.10 on fixture C: absence result properties.tier == "candidate", parse_quality == "degraded" */ }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test cli sarif_test::`
Expected: every test fails (`--format sarif` renders text today → `serde_json::from_slice` fails; `--format bogus` exits 0). Record the outputs.

- [ ] **Step 3: Implement `src/output/sarif.rs`** per spec §2.2:
  - Rule table: `fn rule_description(algorithm: &str, category: &str) -> String` with one sentence per known category (all 29 from `grounding/finding-inventory.md` §4 — copy the category list into the match; `contract_violation` text names both shapes; default: `format!("{algorithm}: {category}")`).
  - Attribution table: `enum Attribution { SameFile, CounterpartFile, Ambiguous }`; `fn attribution(algorithm: &str) -> Attribution` with `SameFile` for `echo|membrane|absence|contract|provenance|taint|peer_consistency`, `CounterpartFile` for `symmetry|primitive`, else `Ambiguous`.
  - `relatedLocations` per §2.2.2: lines (`> 0`, sorted, dedup) attributed by the table; `Ambiguous` with non-empty `related_files` → lines to `properties.related_lines`; files (sorted, dedup) without region.
  - `sarif_uri`: replace `\` with `/`; percent-encode each `/`-separated segment (unreserved `A-Za-z0-9-._~` kept; everything else `%XX` uppercase, UTF-8 bytes); `escapes_repo_root = path.starts_with('/') || Path::new(path).is_absolute() || segments.any(|s| s == "..")`.
  - `level_for_severity`: `concern→error`, `warning→warning`, `suggestion|info→note`, else `error`.
  - `fingerprint`: `sha2::Sha256` over `serde_json::to_vec(&[algorithm, category_or_uncategorized, file, function_name_or_empty, masked_description, line_text])`; `masked_description`: every maximal ASCII-digit run → `#`. Hex lowercase.
  - `properties.severity` always carries the original severity string; `properties.confidence/tier/parse_quality` come from `classify(&f.algorithm, parse_quality_for(f, inputs.parse_quality, inputs.files))`.
  - Notifications also include one `warning` per `load_warnings` entry (text verbatim).
  - Results sorted by `(uri, line, ruleId, message)`; rules sorted by id; `ruleIndex` assigned after sorting.
  - Notifications: one `error` per `AlgorithmError` (`"{algorithm}: {error}"`), one `warning` per parse warning, one `warning` per escaping path (`"path escapes repo root: {path}"`). `executionSuccessful = errors.is_empty()`.
  - `run.properties`: `mapping_version "1"`, `algorithms_run`, `resolution_mode`, `errors` (omit if empty), `prism_build_identity`, `prism_git_sha`, `binary_input_dirty`.
  - Line text: `sources.get(file).and_then(|s| s.lines().nth(line - 1)).map(str::trim).unwrap_or("")` for `line >= 1`.
- [ ] **Step 4: Wire the CLI** in `src/main.rs`:
  - `--format` arg: add `value_parser = ["text", "json", "paper", "review", "callers", "mermaid", "sarif"]` (keep `default_value = "text"` and the doc comment; add `sarif`).
  - In the per-diff-file parse loop (≈`:743-763`) collect a local `load_warnings: Vec<String>` next to the existing unsupported-language `eprintln!`: `format!("skipped unsupported file: {path} (unsupported language)")` — the stderr text stays byte-identical; an unreadable file remains FATAL exactly as today (`fs::read_to_string(..).context(..)?`). This vector is new and only feeds SARIF (Task 3 moves it into `ReviewInputs.load_warnings`).
  - Multi-run `match cli.format.as_str()` (≈`:999`): add arm `"sarif"` building `SarifInputs { findings: &all_findings, errors: &all_errors, parse_warnings: &parse_warnings, load_warnings: &load_warnings, algorithms_run: &algorithms_run, parse_quality: &parse_quality, files: &files, sources: &sources }`, print pretty JSON + newline, then the same trailer as `"json"` (`emit_warnings_to_stderr` + `determine_exit_code`).
  - Single-run `match` (≈`:1124`): arm `"sarif"` with `findings: &result.findings` (already annotated at `:1122`), `errors: &[]`, `parse_warnings: &result.warnings`, `load_warnings: &load_warnings`, `algorithms_run: &[algorithm.name().to_string()]`, same map, files and sources.
- [ ] **Step 5: Unit tests inside `sarif.rs`** (§7.2.6–7.2.9): `line: 0` → no `region`; attribution cases (`absence` lines 5,0,3,5 + `b.py`; `symmetry` lines 10,20 in `b.py`; `callback_dispatcher` → files only + `properties.related_lines`); severity map incl. `"critical"` → `"error"` with `properties.severity == "critical"`; fingerprint invariance under `(line 12)`→`(line 13)` and sensitivity to `line_text`/`function_name`; `sarif_uri` cases (`dir with space/a b.py` → `dir%20with%20space/a%20b.py`; `a\\b.py` → `a/b.py`; `../x.py` → `escapes == true`).
- [ ] **Step 6: Run to verify it passes**

Run: `cargo test --test cli sarif_test:: && cargo test --lib output::sarif`
Expected: all pass. Then `cargo test --test cli` (whole CLI target) — the format allow-list must not break existing tests; if any existing test passes an invalid format value, report it (do not "fix" the test silently).

- [ ] **Step 7: fmt + clippy + full suite + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/phase0-suite-task2.log; awk '/^test result:/' /tmp/phase0-suite-task2.log
git add src/output/sarif.rs src/output/mod.rs src/main.rs tests/cli/sarif_test.rs tests/cli/main.rs
git commit -m "feat(phase0): --format sarif (SARIF 2.1 serializer) and --format allow-list (spec §2.2)"
```

---

### Task 3: `prism::api` facade; main.rs becomes its first consumer; same-base control

**Files:**
- Create: `src/api/mod.rs`, `src/api/build_info.rs`, `src/api/review.rs`, `src/api/run.rs`, `src/api/nav.rs`, `scripts/phase0-byte-control.sh`
- Modify: `src/lib.rs` (`pub mod api;`), `src/main.rs` (consume the facade; delete moved helpers), `src/output/mod.rs` only if a re-export is needed
- Create: `tests/integration/api_test.rs`; Modify: `tests/integration/main.rs` (`mod api_test;`)

**Interfaces:**
- Consumes: Task 1/2 re-exports; everything main.rs uses today (`repo_loader`, `cpg_cache`, `CpgContext::*`, `algorithms::*`, `navigation::*`).
- Produces: exactly the surface in spec §2.3.2 v5 (copy the signatures from the spec verbatim — including `ReviewInputs.load_warnings`, `build_context -> Result<BuiltContext<'a>>` with `BuiltContext { ctx, warnings }` carrying the cache-save-failure / type-db-fallback messages main.rs prints today (stderr unchanged), `ReviewOutcome { inputs, run, build_warnings }`, `ReviewRun.results` = successful subsequence with `warnings == parse_warnings` only, `#[derive(Default)] NavOptions`; every struct/enum `#[non_exhaustive]`; `AlgorithmParams` fields = one per `ReviewArgs` algorithm flag: `barrier_depth: usize, barrier_symbols: Vec<String>, chop_source: Option<String>, chop_sink: Option<String>, taint_sources: Vec<String>, taint_return_flow: bool, condition: Option<String>, old_repo: Option<PathBuf>, spiral_max_ring: usize, quantum_var: Option<String>, peer_pattern: Option<String>, layers: Option<String>, concern: Option<String>, temporal_days: usize`). `pub const DEFAULT_BARRIER_DEPTH`, `DEFAULT_SPIRAL_MAX_RING`, `DEFAULT_TEMPORAL_DAYS` in `src/api/run.rs` hold the values currently written as clap defaults in `src/main.rs` for `--barrier-depth`, `--spiral-max-ring`, `--temporal-days` (read them from the source; do not guess), and those clap args switch to `default_value_t = prism::api::DEFAULT_*`.

- [ ] **Step 1: Write the control script first** — `scripts/phase0-byte-control.sh <base-bin> <branch-bin>`:
  - Fixtures: every `*.diff`/`*.patch`/`diff.json` under `tests/fixtures/` with a sibling repo directory (enumerate: `tests/fixtures/python` + `calc.diff`, `tests/fixtures/hapi-4552-source` + `hapi-4552.diff`, `tests/fixtures/nav_compat/*`, `tests/fixtures/review_no_diagrams/*`, `tests/fixtures/c/*` if a diff exists — read each directory's layout and existing tests to pair repo and diff correctly), plus a generated poor-parse fixture written to a temp dir (a Python file whose first 20 lines are `def broken(:` repeated, followed by a valid function that is on the diff).
  - Invocations per fixture: (a) single `--algorithm {leftflow,absence,contract,echo,membrane,provenance,primitive}` × `--format {text,json,paper,review,mermaid}`; (b) `--algorithm echo,absence,contract` and `--algorithm absence,contract,primitive` × `--format {text,json,paper,review,mermaid}`; (c) `--algorithm chop,absence --format json`; (d) `--format callers`; (e) `--algorithm leftflow --format json --strict-diagrams` on `tests/fixtures/diagram_snapshot` if it has a diff (else skip with a printed note).
  - For each invocation run both binaries with identical args and cwd, capture stdout, stderr, exit code to files, `cmp` all three; print `DIFF <invocation>` on any mismatch; exit 1 if any mismatch, else print the count of identical invocations and exit 0.
  - Run it now with `<base-bin> = /Users/wesleyjinks/code/tools/bin/prism-base-c220525` and `<branch-bin>` = a fresh `cargo build --release` of the current HEAD (Task 2 state). Expected: zero diffs (Task 2 changed no existing format). If Task 2 already broke something, STOP and report.
- [ ] **Step 2: Write the failing API tests** `tests/integration/api_test.rs` (§7.3; in-process; fixtures via `tempfile` + `std::fs`; use `prism::api::*`):

```rust
#[test] fn one_shot_review_returns_inputs_and_findings() { /* §7.3.1: one missing_counterpart finding; inputs.diff.files.len()==1; inputs.parse_quality has NO entry for a.py while inputs.files does; a second fixture whose diff also names notes.txt → inputs.load_warnings == ["skipped unsupported file: notes.txt (unsupported language)"] and run.warnings unchanged */ }
#[test] fn two_phase_api_installs_its_own_pool_and_reports_each_algorithm() { /* §7.3.2: no with_build_pool wrapper; let built = build_context(&inputs,&opts)?; run_review(&built.ctx, ...); algorithms_run == ["EchoSlice","AbsenceSlice"]; results[0].algorithm == SlicingAlgorithm::EchoSlice; results[1] == AbsenceSlice; errors empty; warnings == inputs.parse_warnings; built.warnings is empty for this fixture */ }
#[test] fn results_are_the_successful_subsequence() { /* §7.3.2b: run_review(&[Chop, AbsenceSlice]) → algorithms_run == ["Chop","AbsenceSlice"], results.len()==1, results[0].algorithm == AbsenceSlice, errors[0].algorithm == "Chop" */ }
#[test] fn defaults_are_shared_with_clap() { /* §7.3.3: AlgorithmParams::default().barrier_depth == DEFAULT_BARRIER_DEPTH; run `prism --help` via assert_cmd and assert it contains the string format!("[default: {}]", DEFAULT_BARRIER_DEPTH) on the --barrier-depth line */ }
#[test] fn chop_without_params_errors_like_the_cli() { /* §7.3.4 */ }
#[test] fn build_info_is_this_binary() { /* §7.3.5 */ }
#[test] fn nav_session_and_callers_work_without_outer_pool() { /* §7.3.6: let mut o = NavOptions::default(); o.no_cache = true; */ }
#[test] fn api_types_are_non_exhaustive() { /* §7.3.8: read src/api/*.rs and src/finding_confidence.rs; for each line starting with `pub struct`/`pub enum`, the nearest preceding non-blank attribute lines must include `#[non_exhaustive]` */ }
```

- [ ] **Step 3: Run to verify it fails** — `cargo test --test integration api_test::` → compile error (`prism::api` absent). Record.
- [ ] **Step 4: Implement the facade by MOVING code** (spec §2.3.3; doctrine: moves, not copies):
  1. `src/api/build_info.rs`: `BuildInfo` + `build_info()`.
  2. `src/api/review.rs`: `ReviewOptions` (+ `new`), `ReviewInputs`, `load_review_inputs` (cut main.rs `:724-816`: diff read/parse → `DiffInput`, `--files` filter, per-file parse loop with the exact `eprintln!` warnings AND the `load_warnings` vector Task 2 introduced (moved, message formats unchanged), TypeDatabase auto/explicit, `check_parse_quality` (sparse map, unchanged), `scope_graph_build_inputs`; compute `diff_text_sha256` with `sha2`), `build_context` (cut `:819-926` cache decision tree + `:929-960` language versions; the `eprintln!` on cache-save failure stays verbatim).
  3. `src/api/run.rs`: `DEFAULT_*`, `AlgorithmParams` (+ `Default`), `run_algorithm` (cut `:1223-1380` verbatim, replacing `cli.<field>` reads with `params.<field>`; then `annotate_finding_parse_quality(&mut result.findings, &inputs.files)` before `Ok(result)`), `annotate_finding_parse_quality` (cut `:1201-1220`), `parse_file_line` (cut `:1386`, keep private), `parse_algorithms` (cut `:687-704`), `ReviewRun`, `run_review` (the multi-run loop from `:968-997`: per-algorithm `SliceConfig` cloned from `config` with `algorithm` set; errors collected as `AlgorithmError`; `findings` flattened; `warnings = inputs.parse_warnings.clone()`), `ReviewOutcome`, `review` (wraps everything in `build_pool::install`).
  4. `src/api/nav.rs`: `NavOptions`, `nav_session` (cut `build_session` `:610-626`, wrap body in `build_pool::install`), `Seed`, `callers`/`callees` delegating to `navigation::queries::{callers,callees}_with_confidence(s, symbol, file, location, depth, exact_only)` with `Seed::Symbol(s) → (Some(s), None, None)`, `Location(l) → (None, None, Some(l))`, `SymbolInFile{symbol,file} → (Some(symbol), Some(file), None)`.
  5. Wrap `load_review_inputs`, `build_context`, `run_algorithm`, `run_review` bodies in `crate::build_pool::install(|| { ... })`.
  6. `src/api/mod.rs`: module docs = the compatibility promise (spec §2.3.1 verbatim) + re-exports listed in spec §2.3.2.
  7. `src/main.rs`: `run_review` now builds `ReviewOptions`/`AlgorithmParams`/`SliceConfig` from `ReviewArgs`, calls `api::load_review_inputs`, `api::build_context`, keeps the `--format callers` short-circuit, calls `api::run_review` (multi) or `api::run_algorithm` + **`result.warnings = inputs.parse_warnings.clone();`** (single; delete the now-redundant annotate call), keeps every format arm byte-for-byte, and the SARIF arms now read `load_warnings`/`files` from `inputs`. `run_nav` uses `api::nav_session`. Delete the moved private helpers. `main()` still wraps in `build_pool::install` (nesting is safe).
- [ ] **Step 5: Run the tests and the control**

```bash
cargo test --test integration api_test::      # expected: 7 passed
cargo test --test cli                          # expected: unchanged pass count
cargo build --release && scripts/phase0-byte-control.sh /Users/wesleyjinks/code/tools/bin/prism-base-c220525 target/release/prism
```
Expected: control prints zero `DIFF` lines. Any `DIFF` is a defect in the move — fix the move, never the script or the fixtures.

- [ ] **Step 6: Cache-decision control** (§8.6) — for each binary with its own empty temp `--cache-dir`: run 1, run 2, append a comment line to a copy of the fixture's diff-touched file, run 3; record `(cpg-cache.bin exists, mtime changed)` per run; both sequences must read `(created), (unchanged), (changed)`. Put the commands and the observed sequences in the task report.
- [ ] **Step 7: fmt + clippy + full suite + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/phase0-suite-task3.log; awk '/^test result:/' /tmp/phase0-suite-task3.log
git add src/api src/lib.rs src/main.rs tests/integration/api_test.rs tests/integration/main.rs scripts/phase0-byte-control.sh
git commit -m "feat(phase0): prism::api facade; main.rs consumes it; same-base byte control (spec §2.3)"
```

---

### Task 4: `prism targets` — projection into the targets contract

**Files:**
- Create: `src/targets/mod.rs`, `src/targets/model.rs`, `src/targets/mapping.rs`
- Modify: `src/lib.rs` (`pub mod targets;`), `src/main.rs` (`Command::Targets(TargetsArgs)` + `run_targets`), `src/api/mod.rs` (add the spec §2.3.2 re-export `pub use crate::targets::{project, TargetsDocument, TargetsMeta};` — deferred from Task 3 by controller ruling because the module did not exist yet)
- Create: `tests/fixtures/targets/` (repo: `client.py`, `svc.py`, `diff.json`, and `old/` copy for `--old-repo` if used), `tests/cli/targets_test.rs`, `tests/integration/targets_mapping_test.rs`; Modify: `tests/cli/main.rs`, `tests/integration/main.rs`
- Read-only reference: `docs/contracts/targets.schema.json` (authoritative; do not edit)

**Interfaces:**
- Consumes: `prism::api::{review, ReviewOptions, ReviewOutcome, ReviewInputs, AlgorithmParams, build_info}`, Task 1 `classify` + `parse_quality_for`, `crate::output::severity_rank`, `ParsedFile::{function_node_spanning, node_line_range}` (public; `src/ast.rs:584`) plus the node's name (the same helpers the file uses for `enclosing_function`), `Language::{from_path, all}`.
- Produces:

```rust
// src/targets/model.rs — Serialize + Deserialize DTOs mirroring the schema; field order = schema order
pub struct TargetsDocument { pub schema_version: String, pub producer: Producer, pub repo: Option<Repo>, pub diff: Option<Diff>,
                             pub errors: Vec<AlgorithmError> /* skip if empty */, pub warnings: Vec<String> /* skip if empty */, pub targets: Vec<Target> }
pub struct Producer { pub tool: String, pub version: String, pub resolution_mode: String, pub build_identity: Option<String>, pub algorithms: Vec<String> }
pub struct Repo { pub root: Option<String>, pub sha: Option<String> }
pub struct Diff { pub sha256: Option<String>, pub files: Vec<String> }
pub struct Target { pub id: String, pub site: Site, pub kind: String, pub category: String, pub expected: Expected,
                    pub dependency_hint: Option<DependencyHint>, pub source_algorithm: String, pub confidence: FindingConfidence,
                    pub tier: FindingTier, pub severity: String, pub description: Option<String>, pub related: Option<Related>, pub parse_quality: Option<String> }
pub struct Site { pub file: String, pub line: usize, pub symbol: Option<String>, pub function_start_line: Option<usize>, pub function_end_line: Option<usize>, pub language: Option<String> }
pub struct Expected { pub property: String, pub detail: Option<String> }
pub struct DependencyHint { pub kind: Option<String>, pub callee: Option<String>, pub counterpart: Option<String> }
pub struct Related { pub lines: Vec<usize>, pub files: Vec<String> }

// src/targets/mapping.rs
pub struct Mapped { pub kind: &'static str, pub property: &'static str, pub detail: Option<String>, pub hint: Option<DependencyHint> }
pub fn map_finding(finding: &SliceFinding) -> Mapped;            // the spec §2.4.2 table; bounded str parsers; no regex
pub fn language_tag(lang: Language) -> &'static str;             // the lowering table
pub const ABSENCE_PAIRS: &[(&str, Option<&str>, Option<&str>)];  // (PairedPattern.description literal, counterpart, kind)

// src/targets/mod.rs
pub struct TargetsMeta { pub algorithms_run: Vec<String>, pub repo_root: PathBuf, pub repo_sha: Option<String>,
                         pub errors: Vec<AlgorithmError>, pub run_warnings: Vec<String> /* parse ++ load ++ build, assembled by the CLI */,
                         pub min_severity_rank: u8, pub min_tier: FindingTier }   // project() emits warnings = run_warnings ++ its own projection warnings
pub fn project(findings: &[SliceFinding], inputs: &ReviewInputs, meta: &TargetsMeta) -> TargetsDocument;
pub fn target_id(file: &str, line: usize, symbol: Option<&str>, algorithm: &str, category: &str, description: &str, severity: &str, related_lines_sorted: &[usize], related_files_sorted: &[String]) -> String;   // sha256 of the 9-element canonical JSON array
```

- [ ] **Step 1: Build the fixture** `tests/fixtures/targets/` so that all five default producers emit at least one finding (§7.4.1), plus a `serialize_x`/`deserialize_x` pair with only one changed (symmetry, run separately with `--algorithm symmetry`, §7.4.3) and a `notes.txt` named in a second diff file `diff-with-unsupported.json` (§7.4.7b). Iterate with the release binary: `target/release/prism --repo tests/fixtures/targets --diff tests/fixtures/targets/diff.json --algorithm echo,absence,contract,provenance,membrane --format json | jq '.all_findings[] | {algorithm, category, file, line}'`. Known triggers (grounding/finding-inventory.md): echo — a changed function `fetch` in `client.py` called from `svc.py` as `return fetch()` with no error handling; membrane — the same cross-file call site (the caller is in another file); absence — `f = open("x")` never closed, on a diff line; contract — a guard clause (`if not x: return None`) on a diff line inside a function that also has non-null returns; provenance — `v = request.args.get("q")` on a diff line. Record which lines are on the diff in `diff.json` and, in a `README` comment inside `tests/cli/targets_test.rs`, which assertion discriminates each producer. If a producer cannot be triggered after a bounded effort (≤ 45 min), report DONE_WITH_CONCERNS naming it — do not weaken §7.4.1 silently.
- [ ] **Step 2: Write the failing tests** — `tests/cli/targets_test.rs` (§7.4.1–7.4.7; include the in-repo structural checker `fn check_against_schema(doc: &Value, schema: &Value)` that walks `required`, `enum`, `pattern` for `id`, `additionalProperties: false`, `minimum` on integers, and `$ref` into `$defs`) and `tests/integration/targets_mapping_test.rs` (§7.4.8: one case per mapping row with the verbatim description formats from `grounding/finding-inventory.md` §1; `line: 0` drop + warning; duplicate id + warning; symmetry bounds omitted + warning; `Language::all()` lowering ⊆ schema enum read from `docs/contracts/targets.schema.json`; `a\\b.py` → `a/b.py`; `../x.py` warning; `category: None` → `uncategorized`; severity `critical` → `concern` + warning; `contract_violation` both shapes + unrecognised → `unknown`).
- [ ] **Step 3: Run to verify it fails** — `cargo test --test cli targets_test::` (unknown subcommand → exit 2) and `cargo test --test integration targets_mapping_test::` (compile error). Record.
- [ ] **Step 4: Implement** `src/targets/*` per spec §2.4.2 (all rules, v5 wording: `site.symbol` = innermost enclosing function name when known and bounds ALWAYS emitted when that function exists, warning on disagreement with `function_name`; absolute/`..` paths → finding dropped + warning; `parse_quality_for` for confidence/tier/parse_quality; document `warnings` = parse_warnings ++ load_warnings ++ projection warnings; `ABSENCE_PAIRS` rows copied from the `PairedPattern` descriptions in `src/algorithms/absence_slice.rs:29-160` — read the file; one row per literal; `counterpart` only where exactly one close-call base exists) and the subcommand in `src/main.rs`:
  - `TargetsArgs { repo, diff, algorithm (default "echo,absence,contract,provenance,membrane"), files, compile_commands, scoped_cpg, cache_dir, no_cache, old_repo, min_severity (value_parser four values, default "info"), min_tier (value_parser ["asserted","candidate"], default "candidate"), strict, out, format (value_parser ["json"], default "json") }`.
  - Pre-flight acceptance table (spec §2.4.1) before any loading; error messages verbatim from the spec.
  - `repo_sha`: `std::process::Command::new("git").args(["rev-parse","HEAD"]).current_dir(&repo)`; `None` on any failure.
  - Exit codes: 0; 3 with `--strict` and non-empty `errors`; 1 on load/build error; 2 clap.
- [ ] **Step 5: Run to verify it passes** — both test modules green; then re-run `scripts/phase0-byte-control.sh` (must still be zero diffs) and `cargo test --test cli`.
- [ ] **Step 6: fmt + clippy + full suite + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/phase0-suite-task4.log; awk '/^test result:/' /tmp/phase0-suite-task4.log
git add src/targets src/lib.rs src/main.rs tests/fixtures/targets tests/cli/targets_test.rs tests/cli/main.rs tests/integration/targets_mapping_test.rs tests/integration/main.rs
git commit -m "feat(phase0): prism targets — projection into the targets contract v1.0 (spec §2.4)"
```

---

### Task 5: README truth pass, CLAUDE.md / SKILL.md updates, README gate

**Files:**
- Modify: `README.md`, `CLAUDE.md`, `skills/prism-code-slicing/SKILL.md`, `src/slice.rs` (only the `severity` doc comment at `:27`), `src/api/mod.rs` (doc-test snippet if not already present)
- Create: `tests/cli/readme_test.rs`; Modify: `tests/cli/main.rs`
- Read-only input: `/Users/wesleyjinks/code/tools/grounding/readme-truth.md` (row-by-row corrections), spec §2.5

**Interfaces:** consumes Tasks 2–4 CLI surface (`--format sarif`, `prism targets`, `prism::api`); produces nothing code-level beyond the gate test.

- [ ] **Step 1: Write the failing gate** `tests/cli/readme_test.rs`:

```rust
// (a) every line inside ``` fences of README.md that starts with "prism " and contains no '<' is split on whitespace
//     (naive shell split is enough: README examples do not use quotes with spaces — if one does, the test may skip lines containing '"' and say so)
//     and parsed with clap: `prism::cli_command().try_get_matches_from(argv)` where `cli_command()` is a small `pub fn` added to
//     src/main.rs? — main.rs is a bin; instead expose the parse via the binary: run `prism <args...> --help`? No: parsing must not execute.
//     Decision: move the clap `Cli` derive to `src/cli.rs` (pub mod cli in lib.rs) so tests can call `prism::cli::Cli::try_parse_from`.
//     This is a pure move of the derive structs (Cli, ReviewArgs, Command, NavArgs, NavQuery, TargetsArgs); main.rs keeps the run_* fns.
// (b) README's documented format list (the table row `--format` values) == the value_parser array from the Cli definition (compare as sets).
#[test] fn every_readme_prism_invocation_parses() { ... }
#[test] fn readme_format_list_matches_cli() { ... }
```

  Note the design decision embedded above: the clap structs move to `src/cli.rs` (a move, no behaviour change; the byte control re-run proves it). This is an allowed modification (`src/main.rs`) plus one new file `src/cli.rs` — record it in the task report as an addition to spec §6's file list.

- [ ] **Step 2: Run to verify it fails** — `cargo test --test cli readme_test::` (README still says `slicing …` → parse fails; formats list `text, json, paper` ≠ CLI).
- [ ] **Step 3: Apply the truth pass** per spec §2.5 items 1–7 using the grounding table; add the `prism targets` section (acceptance table, schema pointer `docs/contracts/targets.schema.json`, `--strict`), the `Library use (prism::api)` section with the compatibility promise verbatim and this snippet as both README text and a doc-test on `prism::api::review`:

```rust
use prism::api::{review, AlgorithmParams, ReviewOptions};
use prism::slice::{SliceConfig, SlicingAlgorithm};
let outcome = review(&ReviewOptions::new("path/to/repo"), diff_text, &[SlicingAlgorithm::EchoSlice, SlicingAlgorithm::AbsenceSlice],
                     &SliceConfig::default(), &AlgorithmParams::default())?;
for f in &outcome.run.findings { println!("{}:{} {} {}", f.file, f.line, f.algorithm, f.description); }
```
  (the doc-test uses a temp repo built inline so it runs; `no_run` is not acceptable).
- [ ] **Step 4: Run to verify it passes** — `cargo test --test cli readme_test:: && cargo test --doc api`; re-run `scripts/phase0-byte-control.sh`.
- [ ] **Step 5: fmt + clippy + full suite + commit**

```bash
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/phase0-suite-task5.log; awk '/^test result:/' /tmp/phase0-suite-task5.log
git add README.md CLAUDE.md skills/prism-code-slicing/SKILL.md src/slice.rs src/cli.rs src/main.rs src/lib.rs src/api/mod.rs tests/cli/readme_test.rs tests/cli/main.rs
git commit -m "docs(phase0): README truth pass, targets + api sections, README gate test (spec §2.5)"
```

---

### Task 6: Closeout (controller)

- [ ] Gates §8.1–§8.9 with recorded outputs: fmt, clippy, full suite totals (awk), byte control zero diffs, cache-decision sequences, Tier-A `--matrix-only` pass count vs base, Python `jsonschema` validation of one SARIF document (official schema) and one targets document (`docs/contracts/targets.schema.json`).
- [ ] Handoff `docs/superpowers/handoffs/2026-09-04-prism-phase0-handoff.md` (template `~/.claude/handoff-template.md`), spec §11 updated with the final review round, roadmap row in `docs/analysis/prism-post-plan-roadmap.md` §1 (new item: Phase 0 interfaces — DONE/PR #).
- [ ] Final whole-branch review (terra via bridge, read-only; Opus parallel seat); one fix wave; scoped re-review; PR opened against `shoedog/prism` main (not merged).

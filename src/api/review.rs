use crate::algorithms;
use crate::ast::ParsedFile;
use crate::call_graph::ScopeGraphBuildInputs;
use crate::cpg::{CodePropertyGraph, CpgContext};
use crate::cpg_cache::{self, CacheResult};
use crate::diff::DiffInput;
use crate::finding_confidence::{
    MinConfidence, ResolutionMode, DEFAULT_MIN_CONFIDENCE, DEFAULT_RESOLUTION,
};
use crate::languages::Language;
use crate::slice::FileParseQuality;
use crate::type_db::TypeDatabase;
use crate::type_provider::LanguageVersion;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[non_exhaustive]
pub struct ReviewOptions {
    pub repo: PathBuf,
    pub files_filter: Option<HashSet<String>>,
    pub compile_commands: Option<PathBuf>,
    pub scoped_cpg: bool,
    pub cache_dir: Option<PathBuf>,
    pub no_cache: bool,
    pub language_versions: Vec<(Language, LanguageVersion)>,
    pub min_confidence: MinConfidence,
    pub resolution: ResolutionMode,
}

impl ReviewOptions {
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        Self {
            repo: repo.into(),
            files_filter: None,
            compile_commands: None,
            scoped_cpg: false,
            cache_dir: None,
            no_cache: false,
            language_versions: Vec::new(),
            min_confidence: DEFAULT_MIN_CONFIDENCE,
            resolution: DEFAULT_RESOLUTION,
        }
    }
}

/// Owned inputs the CPG is built from.
#[non_exhaustive]
pub struct ReviewInputs {
    pub files: BTreeMap<String, ParsedFile>,
    pub sources: BTreeMap<String, String>,
    pub type_db: Option<TypeDatabase>,
    pub diff: DiffInput,
    pub diff_text_sha256: String,
    pub parse_warnings: Vec<String>,
    pub load_warnings: Vec<String>,
    pub parse_quality: BTreeMap<String, FileParseQuality>,
    pub scope_graph_inputs: ScopeGraphBuildInputs,
    pub min_confidence: MinConfidence,
    pub resolution: ResolutionMode,
    build_warnings: Vec<String>,
}

pub fn load_review_inputs(opts: &ReviewOptions, diff_text: &str) -> Result<ReviewInputs> {
    crate::build_pool::install(|| {
        let mut diff_input = if diff_text.trim_start().starts_with('{') {
            DiffInput::from_json(diff_text)?
        } else {
            DiffInput::parse_unified_diff(diff_text)
        };

        // Apply --files filter early so algorithms only see the selected files
        diff_input.filter_files(opts.files_filter.as_ref());

        // Parse all referenced source files
        let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
        let mut sources: BTreeMap<String, String> = BTreeMap::new();
        // Files the loader skipped (design §2.3.2). Distinct from `parse_warnings`,
        // which grade files prism DID parse. Only `--format sarif` reads this
        // today; the stderr text above is unchanged. An unreadable file stays
        // fatal (the `?` on `fs::read_to_string` below).
        let mut load_warnings: Vec<String> = Vec::new();

        for diff_info in &diff_input.files {
            let file_path = opts.repo.join(&diff_info.file_path);
            let language = match Language::from_path(&diff_info.file_path) {
                Some(l) => l,
                None => {
                    eprintln!(
                        "Warning: unsupported language for {}, skipping",
                        diff_info.file_path
                    );
                    load_warnings.push(format!(
                        "skipped unsupported file: {} (unsupported language)",
                        diff_info.file_path
                    ));
                    continue;
                }
            };

            let source = fs::read_to_string(&file_path)
                .context(format!("Failed to read source: {:?}", file_path))?;

            let parsed = ParsedFile::parse(&diff_info.file_path, &source, language)?;
            sources.insert(diff_info.file_path.clone(), source);
            files.insert(diff_info.file_path.clone(), parsed);
        }

        let mut build_warnings = Vec::new();

        // Load type database if compile_commands.json is provided
        let type_db: Option<TypeDatabase> = if let Some(cc_path) = &opts.compile_commands {
            let diff_files: Vec<&str> = diff_input
                .files
                .iter()
                .map(|f| f.file_path.as_str())
                .collect();
            match TypeDatabase::from_compile_commands(cc_path, Some(&diff_files)) {
                Ok(db) => {
                    eprintln!(
                        "Type enrichment: {} records, {} typedefs from {}",
                        db.records.len(),
                        db.typedefs.len(),
                        cc_path.display()
                    );
                    Some(db)
                }
                Err(e) => {
                    eprintln!("Warning: failed to load type database: {}", e);
                    build_warnings.push(format!("Warning: failed to load type database: {}", e));
                    None
                }
            }
        } else {
            // Auto-enable tree-sitter fallback for C/C++ files
            let has_c_cpp = files.values().any(|pf| {
                matches!(
                    pf.language,
                    crate::languages::Language::C | crate::languages::Language::Cpp
                )
            });
            if has_c_cpp {
                let db = TypeDatabase::from_parsed_files(&files);
                if !db.records.is_empty() || !db.typedefs.is_empty() {
                    eprintln!(
                        "Type enrichment (tree-sitter fallback): {} records, {} typedefs",
                        db.records.len(),
                        db.typedefs.len()
                    );
                    Some(db)
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Check parse quality for all files and collect warnings + structured data.
        let (parse_warnings, parse_quality) = algorithms::check_parse_quality(&files);
        let scope_graph_inputs = crate::repo_loader::scope_graph_build_inputs(&opts.repo, &files);
        let diff_text_sha256 = format!("{:x}", Sha256::digest(diff_text.as_bytes()));

        Ok(ReviewInputs {
            files,
            sources,
            type_db,
            diff: diff_input,
            diff_text_sha256,
            parse_warnings,
            load_warnings,
            parse_quality,
            scope_graph_inputs,
            min_confidence: opts.min_confidence,
            resolution: opts.resolution,
            build_warnings,
        })
    })
}

#[non_exhaustive]
pub struct BuiltContext<'a> {
    pub ctx: CpgContext<'a>,
    pub warnings: Vec<String>,
}

pub fn build_context<'a>(
    inputs: &'a ReviewInputs,
    opts: &ReviewOptions,
) -> Result<BuiltContext<'a>> {
    crate::build_pool::install(|| {
        let mut warnings = inputs.build_warnings.clone();
        // Build CPG once — shared across all algorithm runs.
        // With --cache-dir, attempt to load from cache first.
        // With --scoped-cpg, only process diff-changed files + direct callers/callees.
        let mut ctx = {
            let use_cache = opts.cache_dir.is_some() && !opts.no_cache && !opts.scoped_cpg;
            let file_hashes = if use_cache {
                Some(cpg_cache::compute_file_hashes(&inputs.sources))
            } else {
                None
            };
            let topology_key = file_hashes.as_ref().map(|hashes| {
                let mut key = cpg_cache::compute_topology_key(
                    hashes,
                    &inputs.scope_graph_inputs.manifest_hashes,
                );
                if let Some(type_db) = inputs.type_db.as_ref() {
                    key.insert(
                        "type_db:fingerprint".to_string(),
                        type_db.cache_fingerprint(),
                    );
                }
                key
            });

            // Try loading from cache.
            // Pass type_db availability so cache can detect virtual dispatch edge mismatches.
            let has_type_db = inputs.type_db.is_some();
            let cache_result = if use_cache {
                let cache_dir = opts.cache_dir.as_ref().unwrap();
                let hashes = file_hashes.as_ref().unwrap();
                cpg_cache::load_cache_with_topology(
                    hashes,
                    topology_key.as_ref().unwrap(),
                    has_type_db,
                    cache_dir,
                )
            } else {
                CacheResult::Miss
            };

            match cache_result {
                CacheResult::Hit(cpg) => {
                    let hashes = file_hashes.as_ref().unwrap();
                    eprintln!("CPG loaded from cache ({} files)", hashes.len());
                    CpgContext::build_with_cached_cpg(&inputs.files, cpg, inputs.type_db.as_ref())
                }
                CacheResult::PartialHit {
                    cached_call_graph,
                    cached_dfg,
                    changed_files,
                } => {
                    eprintln!(
                        "CPG cache partial hit: {} of {} files changed, rebuilding incrementally",
                        changed_files.len(),
                        file_hashes.as_ref().map_or(0, |h| h.len())
                    );
                    let cpg = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
                        cached_call_graph,
                        cached_dfg,
                        &changed_files,
                        &inputs.files,
                        inputs.type_db.clone(),
                        Some(&inputs.scope_graph_inputs),
                    );
                    // P15a-fix2: this CPG was freshly rebuilt from the current
                    // `files`, so its stashed plain Go provider transfers into
                    // the registry (same provenance as a full build).
                    let ctx = CpgContext::build_with_fresh_cpg(
                        &inputs.files,
                        cpg,
                        inputs.type_db.as_ref(),
                    );

                    // Save updated cache.
                    if let (Some(cache_dir), Some(hashes)) = (&opts.cache_dir, &file_hashes) {
                        if let Err(e) = cpg_cache::save_cache_with_topology(
                            &ctx.cpg,
                            hashes,
                            topology_key.as_ref().unwrap(),
                            has_type_db,
                            cache_dir,
                        ) {
                            eprintln!("Warning: failed to write CPG cache: {}", e);
                            warnings.push(format!("Warning: failed to write CPG cache: {}", e));
                        } else {
                            eprintln!("CPG cache updated to {}", cache_dir.display());
                        }
                    }
                    ctx
                }
                CacheResult::Miss => {
                    let ctx = if opts.scoped_cpg {
                        CpgContext::build_scoped(
                            &inputs.files,
                            &inputs.diff,
                            inputs.type_db.as_ref(),
                        )
                    } else {
                        CpgContext::build_with_scope_graph_inputs(
                            &inputs.files,
                            inputs.type_db.as_ref(),
                            Some(&inputs.scope_graph_inputs),
                        )
                    };

                    // Save cache after a full build (not for scoped builds).
                    if let (Some(cache_dir), Some(hashes)) = (&opts.cache_dir, &file_hashes) {
                        if let Err(e) = cpg_cache::save_cache_with_topology(
                            &ctx.cpg,
                            hashes,
                            topology_key.as_ref().unwrap(),
                            has_type_db,
                            cache_dir,
                        ) {
                            eprintln!("Warning: failed to write CPG cache: {}", e);
                            warnings.push(format!("Warning: failed to write CPG cache: {}", e));
                        } else {
                            eprintln!("CPG cache written to {}", cache_dir.display());
                        }
                    }
                    ctx
                }
            }
        };

        // Store target language versions in the registry (informational in Phase 1).
        for (language, version) in &opts.language_versions {
            ctx.types.set_target_version(*language, version.clone());
        }

        Ok(BuiltContext { ctx, warnings })
    })
}

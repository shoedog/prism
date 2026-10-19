//! Complete serialized CPG identity, including labels, byte spans and indexes.
use super::*;
use crate::cpg::dfg_observation::{fieldwise_payload, lookup_indexes};
use crate::languages::Language;
use std::path::{Path, PathBuf};

use super::callee_cases as cases;

// The selected slice supplies this scoped switch after the census. It must
// affect only fact reuse; assembly/edge insertion runs in full in both modes.
fn legacy_assembly(
    cg: CallGraph,
    dfg: DataFlowGraph,
    files: &BTreeMap<String, ParsedFile>,
) -> CodePropertyGraph {
    super::callee_legacy::with_legacy(|| CodePropertyGraph::assemble_graph(cg, dfg, files, None))
}

fn compare(files: &BTreeMap<String, ParsedFile>) {
    let cg = CallGraph::build(files);
    let dfg = DataFlowGraph::build(files);
    let candidate = CodePropertyGraph::assemble_graph(cg.clone(), dfg.clone(), files, None);
    let legacy = legacy_assembly(cg, dfg, files);
    assert_eq!(fieldwise_payload(&candidate), fieldwise_payload(&legacy));
    assert_eq!(lookup_indexes(&candidate), lookup_indexes(&legacy));
}

#[test]
fn generated_complete_cpg_identity() {
    for m in [1, 8, 32] {
        compare(&super::callee_census_tests::synthetic(m));
    }
    for (ext, language, source) in cases::languages() {
        compare(&cases::parsed(ext, language, source));
    }
    let mut files = BTreeMap::new();
    for file in 0..4 {
        let path = format!("unit{file}.ts");
        let mut source = format!("export function helper(x: number) {{ return x + {file}; }}\n");
        for caller in 0..16 {
            source.push_str(&format!(
                "function caller{caller}(x: number) {{ const a = helper(x); return a; }}\n"
            ));
        }
        files.insert(
            path.clone(),
            ParsedFile::parse(&path, &source, Language::TypeScript).unwrap(),
        );
    }
    compare(&files);
    // Same owner identities, different source facts in a later build.
    compare(&cases::parsed("js", Language::JavaScript,
        "function helper(first, second) { return second; }\nfunction caller(x,y) { let a = helper(x,y); return a; }\n"));
    compare(&cases::parsed("js", Language::JavaScript,
        "function helper({first}, second) { return second; }\nfunction caller(x,y) { let a = helper(x,y); return a; }\n"));
}

fn sources(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "private-corpus"
            || name.starts_with("f-")
            || name.starts_with("F-private")
            || name.to_ascii_lowercase().contains("secbench")
        {
            continue;
        }
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            sources(&entry.path(), out);
        } else if kind.is_file() && Language::from_path(&entry.path().to_string_lossy()).is_some() {
            out.push(entry.path());
        }
    }
}

#[test]
fn repository_fixtures_complete_cpg_identity() {
    crate::build_pool::install(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut paths = Vec::new();
        sources(&root, &mut paths);
        assert!(
            paths.len() >= 100,
            "fixture population unexpectedly empty or truncated"
        );
        let mut all = BTreeMap::new();
        for path in &paths {
            let relative = path.strip_prefix(&root).unwrap().to_str().unwrap();
            let source = std::fs::read_to_string(path).unwrap();
            let parsed =
                ParsedFile::parse(relative, &source, Language::from_path(relative).unwrap())
                    .unwrap();
            let files = BTreeMap::from([(relative.to_string(), parsed)]);
            compare(&files);
            all.extend(files);
        }
        compare(&all);
        eprintln!(
            "complete CPG identity: {} fixture files individually and together",
            paths.len()
        );
    });
}

use crate::common::*;
use algorithms::taint::{self, TaintConfig};

const FILE: &str = "m.tsx";
const SEED: usize = 6;
const SINK: usize = 3;

fn source(flow: bool) -> String {
    let row = include_str!("../../fixtures/props_matrix/rows.jsonl")
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["id"] == "j01")
        .unwrap();
    let source = row["files"]["m"].as_str().unwrap();
    assert!(source
        .lines()
        .nth(SEED - 1)
        .unwrap()
        .contains("const text = useText()"));
    assert!(source
        .lines()
        .nth(SINK - 1)
        .unwrap()
        .contains("dangerouslySetInnerHTML"));
    if flow {
        source.into()
    } else {
        source.replace("html={text}", "html={\"constant\"}")
    }
}

fn run(flow: bool) -> SliceResult {
    let source = source(flow);
    let parsed = ParsedFile::parse(FILE, &source, Language::Tsx).unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    let files = BTreeMap::from([(FILE.into(), parsed)]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: FILE.into(),
            modify_type: ModifyType::Modified,
            diff_lines: [SEED].into_iter().collect(),
        }],
    };
    let config = TaintConfig {
        sources: vec![(FILE.into(), SEED)],
        ..TaintConfig::default()
    };
    taint::slice(&CpgContext::build(&files, None), &diff, &config).unwrap()
}

#[test]
fn jsx_prop_connects_explicit_source_to_html_sink() {
    let result = run(true);
    assert!(
        result.findings.iter().any(|finding| {
            finding.file == FILE
                && finding.line == SINK
                && finding.category.as_deref() == Some("taint_sink")
        }),
        "expected taint_sink at {FILE}:{SINK}, got {:?}",
        result.findings
    );
}

#[test]
fn constant_jsx_prop_keeps_sink_and_seed_but_drops_finding() {
    let result = run(false);
    assert!(
        result
            .findings
            .iter()
            .all(|finding| { finding.category.as_deref() != Some("taint_sink") }),
        "unexpected taint sink: {:?}",
        result.findings
    );
}

//! U11 attempts unchanged REAL producers first. Only the explicitly named fallback injects an edge.
use super::*;
use crate::algorithms::taint::source_scope::admission::{slice_in_context, ScopedTaintOutcome};
use crate::algorithms::taint::{slice, TaintConfig};
use prism::ast::ParsedFile;
use prism::cpg::{CpgContext, CpgEdge, CrossingCeiling, TaintCrossingPolicy};
use prism::diff::{DiffInfo, DiffInput};
use prism::languages::Language;

fn parse(files: &[(&str, &str)]) -> std::collections::BTreeMap<String, ParsedFile> {
    files
        .iter()
        .map(|(file, source)| {
            (
                file.to_string(),
                ParsedFile::parse(file, source, Language::Python).unwrap(),
            )
        })
        .collect()
}
fn guard_and_check(
    baseline: &SliceResult,
    expanded: &ScopedTaintOutcome,
    originals: &BTreeSet<String>,
    context: &BTreeSet<String>,
) -> bool {
    assert!(
        baseline.findings.iter().all(|f| !context.contains(&f.file)),
        "inadmissible precondition: baseline contains no context finding"
    );
    for emission in &expanded.emission.sinks {
        let finding = &expanded.result.findings[emission.finding_index];
        assert_eq!(
            (&finding.file, finding.line, finding.category.as_deref()),
            (
                &emission.file,
                emission.line,
                Some(emission.category.as_str())
            ),
            "inadmissible precondition: sink sidecar index is aligned"
        );
    }
    for source in &expanded.emission.sources {
        let finding = &expanded.result.findings[source.finding_index];
        assert_eq!(
            (&finding.file, finding.line, finding.category.as_deref()),
            (&source.file, source.line, Some("taint_source")),
            "inadmissible precondition: source sidecar index is aligned"
        );
    }
    let Some(sink) = expanded.emission.sinks.iter().find(|s| {
        context.contains(&s.file)
            && s.category == "taint_sink"
            && s.licensed.iter().any(|l| {
                expanded
                    .emission
                    .sources
                    .iter()
                    .any(|e| e.file == l.file && e.line == l.line)
            })
    }) else {
        eprintln!("U11 producer observation: no context sink with emitted licensed source; findings={:?}; sidecar={:?}", expanded.result.findings, expanded.emission);
        return false;
    };
    assert!(
        expanded
            .result
            .evidence
            .get(sink.finding_index)
            .is_some_and(Option::is_some),
        "inadmissible precondition: context sink selected evidence is present"
    );
    let source = sink
        .licensed
        .iter()
        .find(|l| {
            expanded
                .emission
                .sources
                .iter()
                .any(|s| s.file == l.file && s.line == l.line)
        })
        .unwrap();
    assert!(
        baseline
            .findings
            .iter()
            .all(|f| !(f.category.as_deref() == Some("taint_source")
                && f.file == source.file
                && f.line == source.line)),
        "inadmissible precondition: licensed source is new to baseline"
    );
    let out = union_results(UnionInput {
        baseline,
        expanded: &expanded.result,
        attribution: &expanded.emission,
        original_files: originals,
        context_files: context,
    })
    .unwrap();
    assert_prefix(baseline, &out.result);
    assert!(out.result.findings.iter().any(|f| f.file == sink.file
        && f.line == sink.line
        && f.category.as_deref() == Some("taint_sink")));
    let added = out
        .result
        .findings
        .iter()
        .find(|f| {
            f.file == source.file
                && f.line == source.line
                && f.category.as_deref() == Some("taint_source")
        })
        .unwrap();
    if added.file != sink.file {
        assert!(added.related_files.contains(&sink.file));
    }
    assert!(out.stats.additional_sinks > 0);
    true
}

#[test]
fn u11_real_context_sink_or_explicit_controlled_fallback() {
    let pairs = [
        (
            "simple",
            "def run(user):\n    value = user\n    helper(value)\n",
            "import os\ndef helper(p):\n    os.system(p)\n",
        ),
        (
            "part-r",
            "from lib import helper\ndef run(user):\n    value = helper(user)\n    sink(value)\n",
            "import os\ndef helper(p):\n    q = p\n    os.system(q)\n    return q\n",
        ),
    ];
    let originals = set(&["main.py"]);
    let context = set(&["lib.py"]);
    let input = DiffInput {
        files: vec![DiffInfo {
            file_path: "main.py".into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let config = TaintConfig {
        return_flow: true,
        ..TaintConfig::default()
    };
    for (name, main, lib) in pairs {
        let base_files = parse(&[("main.py", main)]);
        let baseline = slice(&CpgContext::build(&base_files, None), &input, &config).unwrap();
        let files = parse(&[("main.py", main), ("lib.py", lib)]);
        let ctx = CpgContext::build(&files, None);
        let policy = TaintCrossingPolicy::new(&context, CrossingCeiling::NameOnly);
        let expanded = slice_in_context(&ctx, &input, &config, &originals, &policy).unwrap();
        eprintln!("U11 REAL attempt: {name}");
        if guard_and_check(&baseline, &expanded, &originals, &context) {
            return;
        }
    }
    // CONTROLLED fallback: parsed roots/sinks, one injected Def->Def relation. No recall repair.
    eprintln!("U11 CONTROLLED fallback: neither REAL fixture met the context-source preconditions; inherited recall gap retained");
    let main = "def g(d):\n    p = d\n    return p\n";
    let lib = "import os\ndef handle(cmd):\n    os.system(cmd)\n";
    let input = DiffInput {
        files: vec![DiffInfo {
            file_path: "main.py".into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let base_files = parse(&[("main.py", main)]);
    let baseline = slice(&CpgContext::build(&base_files, None), &input, &config).unwrap();
    let files = parse(&[("main.py", main), ("lib.py", lib)]);
    let mut ctx = CpgContext::build(&files, None);
    let node = |file: &str, line, name: &str| {
        let hits: Vec<_> = ctx
            .cpg
            .nodes_at(file, line)
            .into_iter()
            .filter(|n| {
                ctx.cpg
                    .to_var_location(*n)
                    .is_some_and(|v| v.path.base == name && v.kind == VarAccessKind::Def)
            })
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "inadmissible precondition: unique parsed {file}:{line} {name} Def"
        );
        hits[0]
    };
    let from = node("main.py", 2, "p");
    let to = node("lib.py", 2, "cmd");
    ctx.cpg.graph.add_edge(from, to, CpgEdge::DataFlow(WEAK));
    let policy = TaintCrossingPolicy::new(&context, CrossingCeiling::NameOnly);
    let expanded = slice_in_context(&ctx, &input, &config, &originals, &policy).unwrap();
    assert!(
        guard_and_check(&baseline, &expanded, &originals, &context),
        "inadmissible precondition: CONTROLLED context sink and licensed source are emitted"
    );
}

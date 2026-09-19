//! Public fail-first and preservation oracles for byte-distinct caller producers.

use super::*;
use crate::data_flow::{DataFlowGraph, ExactFlowEdge, VarAccessKind, VarLocation};
use crate::languages::Language;
use std::hash::{Hash, Hasher};

const ORIGIN: &str = "export function item(input){return input;}";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProducerRow {
    file: String,
    owner: String,
    owner_start: usize,
    def_line: usize,
    def_start: usize,
    def_end: usize,
    use_line: usize,
    use_start: usize,
    use_end: usize,
    confidence: FlowConfidence,
}

fn fixture(language: Language, ext: &str, app: &str) -> BTreeMap<String, ParsedFile> {
    [
        (format!("app.{ext}"), app),
        (format!("origin.{ext}"), ORIGIN),
    ]
    .into_iter()
    .map(|(path, source)| {
        let parsed = ParsedFile::parse(&path, source, language).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{path}: {source}");
        (path, parsed)
    })
    .collect()
}

fn producer_rows(cpg: &CodePropertyGraph, file: &str, owner: &str, path: &str) -> Vec<ProducerRow> {
    let expected_path = AccessPath::simple(path);
    let mut rows: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(confidence) = cpg.graph[edge] else {
                return None;
            };
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            match (&cpg.graph[from], &cpg.graph[to]) {
                (
                    CpgNode::Variable {
                        file: from_file,
                        function: from_owner,
                        function_start_line,
                        line: def_line,
                        path: from_path,
                        access: VarAccess::Def,
                        start_byte: def_start,
                        end_byte: def_end,
                    },
                    CpgNode::Variable {
                        file: to_file,
                        function: to_owner,
                        function_start_line: to_owner_start,
                        line: use_line,
                        path: to_path,
                        access: VarAccess::Use,
                        start_byte: use_start,
                        end_byte: use_end,
                    },
                ) if from_file == file
                    && to_file == file
                    && from_owner == owner
                    && to_owner == owner
                    && function_start_line == to_owner_start
                    && from_path == &expected_path
                    && to_path == &expected_path =>
                {
                    Some(ProducerRow {
                        file: from_file.clone(),
                        owner: from_owner.clone(),
                        owner_start: *function_start_line,
                        def_line: *def_line,
                        def_start: *def_start,
                        def_end: *def_end,
                        use_line: *use_line,
                        use_start: *use_start,
                        use_end: *use_end,
                        confidence,
                    })
                }
                _ => None,
            }
        })
        .collect();
    rows.sort();
    rows
}

fn value_spans(source: &str) -> Vec<(usize, usize)> {
    source
        .match_indices("value")
        .map(|(start, text)| (start, start + text.len()))
        .collect()
}

fn expected_rows(
    file: &str,
    owner_start: usize,
    def_line: usize,
    def: (usize, usize),
    uses: &[(usize, (usize, usize), FlowConfidence)],
) -> Vec<ProducerRow> {
    uses.iter()
        .map(|(line, use_span, confidence)| ProducerRow {
            file: file.to_string(),
            owner: "outer".to_string(),
            owner_start,
            def_line,
            def_start: def.0,
            def_end: def.1,
            use_line: *line,
            use_start: use_span.0,
            use_end: use_span.1,
            confidence: *confidence,
        })
        .collect()
}

#[test]
fn exact_caller_reads_r01_r02_all_dialects() {
    let cases = [
        (
            "one-line-parameter",
            "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}",
            2,
            2,
            vec![
                (2, (58, 63), FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
                (2, (77, 82), FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
            ],
        ),
        (
            "multiline-parameter",
            "import {item} from './origin';\nfunction outer(value){\nprepare();\nsink(value); return item(value);\n}",
            2,
            2,
            vec![(4, (70, 75), FlowConfidence::Exact), (4, (90, 95), FlowConfidence::Exact)],
        ),
        (
            "prior-line-local",
            "function outer(){\nconst value=source();\nprepare();\nsink(value); return item(value);\n}",
            1,
            2,
            vec![(4, (56, 61), FlowConfidence::Exact), (4, (76, 81), FlowConfidence::Exact)],
        ),
    ];
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for (case, app, owner_start, def_line, uses) in &cases {
            let spans = value_spans(app);
            let def = spans[0];
            let files = fixture(language, ext, app);
            let file = format!("app.{ext}");
            let cpg = CodePropertyGraph::build(&files);
            let actual = producer_rows(&cpg, &file, "outer", "value");
            let expected = expected_rows(&file, *owner_start, *def_line, def, uses);
            if actual != expected {
                failures.push(format!(
                    "{ext}/{case}: expected {expected:?}, got {actual:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn exact_caller_reads_owner_bytes_and_mutation_are_discriminating() {
    let variants = [
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}",
        "import {item} from './origin';\n// π shifts bytes\nfunction outer(value){sink(value);return item(value);}",
    ];
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for app in variants {
            let spans = value_spans(app);
            let files = fixture(language, ext, app);
            let file = format!("app.{ext}");
            let rows = producer_rows(&CodePropertyGraph::build(&files), &file, "outer", "value");
            let later_count = rows
                .iter()
                .filter(|row| (row.use_start, row.use_end) == spans[2])
                .count();
            if later_count != 1 {
                failures.push(format!(
                    "{ext}/shifted={}: later rows={rows:?}",
                    app.contains('π')
                ));
            }
        }

        let mutated =
            "import {item} from './origin';\nfunction outer(value){sink(value);return item(other);}";
        let files = fixture(language, ext, mutated);
        let file = format!("app.{ext}");
        let rows = producer_rows(&CodePropertyGraph::build(&files), &file, "outer", "value");
        if rows.len() != 1 || (rows[0].use_start, rows[0].use_end) != value_spans(mutated)[1] {
            failures.push(format!(
                "{ext}/mutated: expected only retained value edge, got {rows:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn exact_caller_reads_preserve_refused_domains() {
    let cases = [
        (
            "write",
            "import {item} from './origin';\nfunction outer(value){value=source();sink(value);return item(value);}",
        ),
        (
            "branch",
            "import {item} from './origin';\nfunction outer(value){if(flag){sink(value);return item(value);}}",
        ),
        (
            "loop",
            "import {item} from './origin';\nfunction outer(value){while(flag){sink(value);return item(value);}}",
        ),
        (
            "alias",
            "import {item} from './origin';\nfunction outer(value){const alias=value;sink(alias);return item(alias);}",
        ),
        (
            "short-circuit",
            "import {item} from './origin';\nfunction outer(value){flag&&sink(value);return item(value);}",
        ),
        (
            "conditional",
            "import {item} from './origin';\nfunction outer(value){flag?sink(value):noop();return item(value);}",
        ),
        (
            "switch",
            "import {item} from './origin';\nfunction outer(value){switch(flag){case 1:sink(value);break;}return item(value);}",
        ),
        (
            "compound-write",
            "import {item} from './origin';\nfunction outer(value){value+=1;sink(value);return item(value);}",
        ),
        (
            "update-write",
            "import {item} from './origin';\nfunction outer(value){value++;sink(value);return item(value);}",
        ),
        (
            "redeclaration",
            "import {item} from './origin';\nfunction outer(value){var value=source();sink(value);return item(value);}",
        ),
        (
            "same-line-local-write",
            "import {item} from './origin';\nfunction outer(){let value=source();value=next();sink(value);return item(value);}",
        ),
        (
            "destructure",
            "import {item} from './origin';\nfunction outer(value){const {x}=value;sink(value);return item(value);}",
        ),
        (
            "default-parameter",
            "import {item} from './origin';\nfunction outer(value=0){sink(value);return item(value);}",
        ),
        (
            "rest-parameter",
            "import {item} from './origin';\nfunction outer(...value){sink(value);return item(value);}",
        ),
        (
            "nested-capture",
            "import {item} from './origin';\nfunction outer(value){function inner(){sink(value);return item(value);}return inner();}",
        ),
        (
            "shadow",
            "import {item} from './origin';\nfunction outer(value){{const value=source();sink(value);return item(value);}}",
        ),
        (
            "try",
            "import {item} from './origin';\nfunction outer(value){try{sink(value);return item(value);}catch(err){return err;}}",
        ),
        (
            "reflection",
            "import {item} from './origin';\nfunction outer(value){eval('x');sink(value);return item(value);}",
        ),
    ];
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for (case, app) in cases {
            let spans = value_spans(app);
            let later = *spans.last().unwrap();
            let files = fixture(language, ext, app);
            let file = format!("app.{ext}");
            let rows = producer_rows(&CodePropertyGraph::build(&files), &file, "outer", "value");
            if rows.iter().any(|row| (row.use_start, row.use_end) == later) {
                failures.push(format!(
                    "{ext}/{case}: refused later edge appeared {rows:?}"
                ));
            }
        }
    }
    let optional =
        "import {item} from './origin';\nfunction outer(value?: number){sink(value);return item(value);}";
    for (language, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        let later = *value_spans(optional).last().unwrap();
        let files = fixture(language, ext, optional);
        let file = format!("app.{ext}");
        let rows = producer_rows(&CodePropertyGraph::build(&files), &file, "outer", "value");
        if rows.iter().any(|row| (row.use_start, row.use_end) == later) {
            failures.push(format!(
                "{ext}/optional-parameter: refused later edge appeared {rows:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn exact_caller_reads_preserve_member_and_non_js_routes() {
    let member = "function outer(obj){sink(obj.value);return sink(obj.value);}";
    let member_files = BTreeMap::from([(
        "app.js".to_string(),
        ParsedFile::parse("app.js", member, Language::JavaScript).unwrap(),
    )]);
    let member_dfg = DataFlowGraph::build(&member_files);
    assert!(member_dfg.exact_labels.is_empty());

    let with_source = "function outer(value){with(ctx){sink(value);return sink(value);}}";
    let with_files = BTreeMap::from([(
        "with.js".to_string(),
        ParsedFile::parse("with.js", with_source, Language::JavaScript).unwrap(),
    )]);
    assert!(DataFlowGraph::build(&with_files).exact_labels.is_empty());

    let recovered = "function outer(value){sink(value);return sink(value)";
    let recovered_file =
        ParsedFile::parse("recovered.js", recovered, Language::JavaScript).unwrap();
    assert!(recovered_file.tree.root_node().has_error());
    let recovered_files = BTreeMap::from([("recovered.js".to_string(), recovered_file)]);
    assert!(DataFlowGraph::build(&recovered_files)
        .exact_labels
        .is_empty());

    let python = "def outer(value):\n    sink(value); return sink(value)\n";
    let python_files = BTreeMap::from([(
        "app.py".to_string(),
        ParsedFile::parse("app.py", python, Language::Python).unwrap(),
    )]);
    let python_dfg = DataFlowGraph::build(&python_files);
    assert!(python_dfg.exact_labels.is_empty());
    let rows: Vec<_> = python_dfg
        .edges
        .iter()
        .filter(|edge| {
            edge.from.function == "outer"
                && edge.from.path == AccessPath::simple("value")
                && edge.to.path == AccessPath::simple("value")
        })
        .map(|edge| {
            (
                edge.from.start_byte,
                edge.from.end_byte,
                edge.to.start_byte,
                edge.to.end_byte,
            )
        })
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "legacy Python producer shape changed: {rows:?}"
    );
}

#[test]
fn exact_caller_reads_same_name_owners_remain_separate() {
    let app = "function outer(value){sink(value);return sink(value);}\nfunction outer(value){sink(value);return sink(value);}";
    let parsed = ParsedFile::parse("app.js", app, Language::JavaScript).unwrap();
    let files = BTreeMap::from([("app.js".to_string(), parsed)]);
    let cpg = CodePropertyGraph::build(&files);
    let rows = producer_rows(&cpg, "app.js", "outer", "value");
    let starts: BTreeMap<_, Vec<_>> = rows.into_iter().fold(BTreeMap::new(), |mut out, row| {
        out.entry(row.owner_start).or_default().push(row);
        out
    });
    assert_eq!(starts.keys().copied().collect::<Vec<_>>(), [1, 2]);
    assert!(starts.values().all(|rows| rows.len() == 2));
    assert_eq!(cpg.dfg.exact_labels.len(), 2);
}

#[test]
fn exact_caller_reads_persist_only_supplemental_and_keep_legacy_identity() {
    let app =
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}";
    let files = fixture(Language::JavaScript, "js", app);
    let cpg = CodePropertyGraph::build(&files);
    assert_eq!(cpg.dfg.exact_labels.len(), 1);
    let (edge, label) = cpg.dfg.exact_labels.iter().next().unwrap();
    assert_eq!((edge.from.start_byte, edge.from.end_byte), (46, 51));
    assert_eq!((edge.to.start_byte, edge.to.end_byte), (77, 82));
    assert_eq!(*label, FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));

    let legacy: Vec<_> = cpg
        .dfg
        .edges
        .iter()
        .filter(|edge| {
            edge.from.file == "app.js"
                && edge.from.function == "outer"
                && edge.from.path == AccessPath::simple("value")
                && edge.to.path == AccessPath::simple("value")
        })
        .map(|edge| {
            (
                edge.from.start_byte,
                edge.from.end_byte,
                edge.to.start_byte,
                edge.to.end_byte,
                cpg.dfg
                    .labels
                    .get(&(edge.from.clone(), edge.to.clone()))
                    .copied(),
            )
        })
        .collect();
    assert_eq!(
        legacy,
        [(
            46,
            51,
            58,
            63,
            Some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))
        )]
    );
    let legacy_edge = cpg
        .dfg
        .edges
        .iter()
        .find(|edge| {
            edge.from.file == "app.js"
                && edge.from.function == "outer"
                && edge.from.path == AccessPath::simple("value")
                && edge.to.path == AccessPath::simple("value")
        })
        .unwrap();
    assert_eq!(
        cpg.dfg.forward.get(&legacy_edge.from),
        Some(&vec![legacy_edge.to.clone()])
    );
    assert_eq!(
        cpg.dfg.backward.get(&legacy_edge.to),
        Some(&vec![legacy_edge.from.clone()])
    );

    let first = VarLocation {
        file: "app.js".into(),
        function: "outer".into(),
        function_start_line: 2,
        line: 2,
        path: AccessPath::simple("value"),
        start_byte: 58,
        end_byte: 63,
        kind: VarAccessKind::Use,
    };
    let mut later = first.clone();
    later.start_byte = 77;
    later.end_byte = 82;
    assert_eq!(first, later);
    let hash = |loc: &VarLocation| {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        loc.hash(&mut state);
        state.finish()
    };
    assert_eq!(hash(&first), hash(&later));
}

#[test]
fn exact_caller_reads_subset_merge_and_direct_removal_are_complete() {
    let source = "function outer(value){sink(value);return sink(value);}";
    let files: BTreeMap<_, _> = ["drop.js", "keep.js"]
        .into_iter()
        .map(|file| {
            (
                file.to_string(),
                ParsedFile::parse(file, source, Language::JavaScript).unwrap(),
            )
        })
        .collect();
    let full = DataFlowGraph::build(&files);
    assert_eq!(full.exact_labels.len(), 2);

    let mut merged = DataFlowGraph::empty();
    for file in ["drop.js", "keep.js"] {
        merged.merge(DataFlowGraph::build_subset(
            &files,
            &BTreeSet::from([file.to_string()]),
        ));
    }
    assert_eq!(merged.exact_labels, full.exact_labels);
    assert_eq!(merged.edges, full.edges);
    assert_eq!(merged.labels, full.labels);

    merged.remove_files(&BTreeSet::from(["drop.js".to_string()]));
    assert_eq!(merged.exact_labels.len(), 1);
    assert!(merged
        .exact_labels
        .keys()
        .all(|edge| edge.from.file == "keep.js" && edge.to.file == "keep.js"));
    let keep = DataFlowGraph::build_subset(&files, &BTreeSet::from(["keep.js".to_string()]));
    assert_eq!(merged.exact_labels, keep.exact_labels);
    assert_eq!(merged.edges, keep.edges);
    assert_eq!(merged.labels, keep.labels);
}

#[test]
fn exact_caller_reads_missing_wrong_and_conflicting_facts_fail_closed() {
    let app = "import {item} from './origin';\nfunction outer(value,other){sink(value);item(value);sink(other);return item(other);}";
    let files = fixture(Language::JavaScript, "js", app);
    let built = CodePropertyGraph::build(&files);
    assert_eq!(built.dfg.exact_labels.len(), 2);

    let value_later = value_spans(app)[2];
    let other_spans: Vec<_> = app
        .match_indices("other")
        .map(|(start, text)| (start, start + text.len()))
        .collect();
    let other_later = other_spans[2];

    let mut missing = built.dfg.clone();
    missing
        .exact_labels
        .retain(|edge, _| edge.from.path != AccessPath::simple("value"));
    let missing_cpg =
        CodePropertyGraph::assemble_graph(built.call_graph.clone(), missing, &files, None);
    assert!(producer_rows(&missing_cpg, "app.js", "outer", "value")
        .iter()
        .all(|row| (row.use_start, row.use_end) != value_later));
    assert!(producer_rows(&missing_cpg, "app.js", "outer", "other")
        .iter()
        .any(|row| (row.use_start, row.use_end) == other_later));

    let value_fact = built
        .dfg
        .exact_labels
        .iter()
        .find(|(edge, _)| edge.from.path == AccessPath::simple("value"))
        .map(|(edge, label)| (edge.clone(), *label))
        .unwrap();
    let forged = [
        {
            let mut edge = value_fact.0.clone();
            edge.to.start_byte += 1;
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.end_byte -= 1;
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.start_byte = edge.to.end_byte;
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.start_byte = edge.to.end_byte + 1;
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.path = AccessPath::simple("other");
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.kind = VarAccessKind::Def;
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.function = "missing".into();
            edge
        },
        {
            let mut edge = value_fact.0.clone();
            edge.to.function_start_line += 1;
            edge
        },
    ];
    for edge in forged {
        let mut wrong = built.dfg.clone();
        wrong.exact_labels.remove(&value_fact.0);
        wrong.exact_labels.insert(edge, value_fact.1);
        let wrong_cpg =
            CodePropertyGraph::assemble_graph(built.call_graph.clone(), wrong, &files, None);
        assert!(producer_rows(&wrong_cpg, "app.js", "outer", "value")
            .iter()
            .all(|row| (row.use_start, row.use_end) != value_later));
    }

    let mut conflicting = built.dfg.clone();
    let legacy_value = conflicting
        .edges
        .iter()
        .find(|edge| {
            edge.from.function == "outer"
                && edge.from.path == AccessPath::simple("value")
                && edge.to.path == AccessPath::simple("value")
        })
        .cloned()
        .unwrap();
    conflicting.exact_labels.insert(
        ExactFlowEdge::from_legacy(&legacy_value),
        FlowConfidence::Exact,
    );
    let conflict_cpg =
        CodePropertyGraph::assemble_graph(built.call_graph.clone(), conflicting, &files, None);
    assert!(producer_rows(&conflict_cpg, "app.js", "outer", "value")
        .iter()
        .all(|row| (row.use_start, row.use_end) != value_later));
    assert!(producer_rows(&conflict_cpg, "app.js", "outer", "other")
        .iter()
        .any(|row| (row.use_start, row.use_end) == other_later));
}

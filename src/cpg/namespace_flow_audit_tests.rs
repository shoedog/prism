//! Source-backed observations: no production boundary-flow admission.
use super::*;
use crate::languages::Language;

const ORIGIN: &str = "export function item(input) { return input; }";

#[test]
#[ignore = "Known compact callback rvalue ownership defect; captured RED, repair not admitted by this audit"]
fn namespace_flow_compact_callback_requires_argument_edge() {
    let mut failures = Vec::new();
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for case_index in [0, 2, 4, 1, 3] {
            let case = &CASES[case_index];
            let files = audit_files(lang, ext, case);
            let cpg = CodePropertyGraph::build(&files);
            // The same strict audit checks exact argument/parameter endpoints and
            // confidence. An unrelated origin Def edge is not a successful flow.
            if observe(case, ext, &files, &cpg, "desired") != Disposition::Supported {
                failures.push(format!("{}/{ext}", case.id));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Missing expected boundary flow: {failures:?}"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Disposition {
    Supported,
    ParameterTokenUse,
    GenuineSameLineUse,
    RefusedTarget,
    RefusedParameter,
}

struct Case {
    id: &'static str,
    app: &'static str,
    origin: &'static str,
    expected: Disposition,
}

const CASES: &[Case] = &[
    Case{id:"namespace-declaration",app:"import * as ns from './origin'; function run(value){return ns.item(value);}",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"named-import-declaration",app:"import {item as invoke} from './origin'; function run(value){return invoke(value);}",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"compact-namespace-property",app:"import * as ns from './origin'; const obj={ns:null};obj.ns=function(value){return ns.item(value);};",origin:ORIGIN,expected:Disposition::ParameterTokenUse},
    Case{id:"compact-named-import-property",app:"import {item as invoke} from './origin'; const obj={callback:null};obj.callback=function(value){return invoke(value);};",origin:ORIGIN,expected:Disposition::ParameterTokenUse},
    Case{id:"multiline-property",app:"import * as ns from './origin';\nconst obj={ns:null};\nobj.ns=function(value){\nreturn ns.item(value);\n};",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"assignment-head-prior-line",app:"import * as ns from './origin'; const obj={ns:null};obj.ns=\nfunction(value){return ns.item(value);};",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"compact-variable-function",app:"import * as ns from './origin'; const run=function(value){return ns.item(value);};",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"compact-variable-arrow",app:"import * as ns from './origin'; const run=(value)=>ns.item(value);",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"compact-object-pair",app:"import * as ns from './origin'; const obj={callback:function(value){return ns.item(value);}};",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"compact-explicit-other-name",app:"import * as ns from './origin'; const obj={ns:null};obj.ns=function callback(value){return ns.item(value);};",origin:ORIGIN,expected:Disposition::ParameterTokenUse},
    Case{id:"genuine-same-line-earlier-use",app:"import * as ns from './origin'; function run(value){sink(value);return ns.item(value);}",origin:ORIGIN,expected:Disposition::GenuineSameLineUse},
    Case{id:"genuine-distinct-line-earlier-use",app:"import * as ns from './origin'; function run(value){sink(value);\nreturn ns.item(value);}",origin:ORIGIN,expected:Disposition::Supported},
    Case{id:"explicit-self-refusal",app:"import * as ns from './origin'; const obj={ns:null};obj.ns=function ns(value){return ns.item(value);};",origin:ORIGIN,expected:Disposition::RefusedTarget},
    Case{id:"unsupported-parameter-refusal",app:"import * as ns from './origin'; function run(value){return ns.item(value);}",origin:"export function item(input=seed()) { return input; }",expected:Disposition::RefusedParameter},
];

fn audit_files(lang: Language, ext: &str, case: &Case) -> BTreeMap<String, ParsedFile> {
    [
        (format!("app.{ext}"), case.app),
        (format!("origin.{ext}"), case.origin),
    ]
    .into_iter()
    .map(|(p, s)| {
        let parsed = ParsedFile::parse(&p, s, lang).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{} {ext}", case.id);
        (p, parsed)
    })
    .collect()
}

/// Trace each producer/consumer seam without changing the production graph.
fn observe(
    case: &Case,
    ext: &str,
    files: &BTreeMap<String, ParsedFile>,
    cpg: &CodePropertyGraph,
    mode: &str,
) -> Disposition {
    use crate::queries::{get_query, QueryKind};
    use tree_sitter::StreamingIterator;
    let app = format!("app.{ext}");
    let origin = format!("origin.{ext}");
    let parsed = &files[&app];
    let sites: Vec<_> = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.file == app && matches!(s.callee_name.as_str(), "item" | "invoke"))
        .collect();
    assert_eq!(sites.len(), 1, "{}/{ext}/{mode}", case.id);
    let site = sites[0];
    let outcome = cpg.call_graph.resolve_call_site_full(site);
    let exact = singleton_exact(&outcome);
    let args = parsed
        .call_argument_texts_and_spans_at(site.start_byte, site.effective_source_callee_name());
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].0, "value");
    let span = &args[0].1;
    let function = parsed
        .all_functions()
        .into_iter()
        .find(|n| {
            parsed.node_line_range(n) == (site.caller.start_line, site.caller.end_line)
                && parsed
                    .language
                    .function_name(n)
                    .is_some_and(|name| parsed.node_text(&name) == site.caller.name)
        })
        .unwrap();
    let parameter_spans: BTreeSet<_> = parsed
        .function_parameter_occurrences(&function)
        .into_iter()
        .map(|(_, a, b)| (a, b))
        .collect();
    let lines: BTreeSet<_> = (site.caller.start_line..=site.caller.end_line).collect();
    let query = get_query(parsed.language, QueryKind::Assignments).unwrap();
    let assign = query.capture_index_for_name("assign").unwrap();
    let mut cursor = tree_sitter::QueryCursor::new();
    cursor.set_byte_range(function.byte_range());
    let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
    let mut escaping_captures = BTreeSet::new();
    while let Some(m) = matches.next() {
        for c in m.captures {
            if c.index == assign
                && parsed.language.is_assignment_node(c.node.kind())
                && lines.contains(&(c.node.start_position().row + 1))
                && !(function.start_byte() <= c.node.start_byte()
                    && c.node.end_byte() <= function.end_byte())
            {
                escaping_captures.insert((c.node.start_byte(), c.node.end_byte()));
            }
        }
    }
    let raw = parsed.rvalue_identifier_spans_on_lines(&function, &lines);
    let phantom_raw = raw.iter().any(|r| {
        r.path.to_string() == "value" && parameter_spans.contains(&(r.start_byte, r.end_byte))
    });
    let key = (
        app.clone(),
        site.caller.name.clone(),
        site.caller.start_line,
        parsed.line_for_byte(span.start),
        AccessPath::simple("value"),
        VarAccess::Use,
    );
    let indexed = cpg.var_index.get(&key).copied();
    let indexed_span = indexed.map(|n| match cpg.node(n) {
        CpgNode::Variable {
            start_byte,
            end_byte,
            ..
        } => (*start_byte, *end_byte),
        _ => panic!("variable index must point at variable"),
    });
    let indexed_parameter = indexed_span.is_some_and(|s| parameter_spans.contains(&s));
    let selected = CodePropertyGraph::argument_var_node_in_span(
        &site.caller,
        parsed,
        &AccessPath::simple("value"),
        span,
        &cpg.var_index,
        &cpg.graph,
    );
    let true_uses: Vec<_> = cpg
        .dfg
        .uses
        .values()
        .flatten()
        .filter(|l| {
            l.file == app
                && l.function == site.caller.name
                && l.function_start_line == site.caller.start_line
                && l.path.to_string() == "value"
                && span.start <= l.start_byte
                && l.end_byte <= span.end
        })
        .collect();
    assert!(
        !true_uses.is_empty(),
        "actual argument Use must survive in DFG: {}/{ext}",
        case.id
    );
    let actual_spans: BTreeSet<_> = true_uses
        .iter()
        .map(|l| (l.start_byte, l.end_byte))
        .collect();
    assert_eq!(actual_spans.len(), 1);
    let params = exact.and_then(|r| {
        compute_param_def_nodes(&files[&r.target.file], r.target, &cpg.var_index, &cpg.graph)
    });
    let parameter = params.as_ref().and_then(|p| p.first()).copied().flatten();
    if let Some(p) = parameter {
        match cpg.node(p) {
            CpgNode::Variable {
                file,
                path,
                access: VarAccess::Def,
                start_byte,
                end_byte,
                ..
            } => {
                assert_eq!(file, &origin);
                assert_eq!(path.to_string(), "input");
                assert_eq!(&files[file].source[*start_byte..*end_byte], "input");
            }
            _ => panic!("exact input parameter Def required"),
        }
    }
    let boundary: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|e| {
            let (a, b) = cpg.graph.edge_endpoints(e).unwrap();
            match (cpg.node(a), cpg.node(b), &cpg.graph[e]) {
                (
                    CpgNode::Variable {
                        file,
                        path,
                        access: VarAccess::Use,
                        ..
                    },
                    CpgNode::Variable {
                        file: to,
                        access: VarAccess::Def,
                        ..
                    },
                    CpgEdge::DataFlow(confidence),
                ) if file == &app && to == &origin && path.to_string() == "value" => {
                    Some((a, b, *confidence))
                }
                _ => None,
            }
        })
        .collect();
    let disposition = if exact.is_none() {
        Disposition::RefusedTarget
    } else if parameter.is_none() {
        Disposition::RefusedParameter
    } else if selected.is_some() {
        Disposition::Supported
    } else if indexed_parameter {
        Disposition::ParameterTokenUse
    } else {
        Disposition::GenuineSameLineUse
    };
    if disposition == Disposition::Supported {
        assert_eq!(
            boundary,
            vec![(selected.unwrap(), parameter.unwrap(), FlowConfidence::Exact)]
        );
    } else {
        assert!(boundary.is_empty());
    }
    if disposition == Disposition::ParameterTokenUse {
        assert!(phantom_raw);
        assert!(!escaping_captures.is_empty());
        // Counterfactual only: index the already-retained real DFG argument Use.
        // This is NOT a production admission or a same-line collision policy.
        let loc = true_uses[0];
        let mut graph = cpg.graph.clone();
        let from = graph.add_node(CpgNode::Variable {
            path: loc.path.clone(),
            file: loc.file.clone(),
            function: loc.function.clone(),
            function_start_line: loc.function_start_line,
            line: loc.line,
            access: VarAccess::Use,
            start_byte: loc.start_byte,
            end_byte: loc.end_byte,
        });
        let mut index = cpg.var_index.clone();
        index.insert(key.clone(), from);
        let pending = CodePropertyGraph::step5b_edges_for_caller(
            &site.caller,
            &BTreeSet::from([site.clone()]),
            &cpg.call_graph,
            &index,
            &graph,
            files,
        );
        assert_eq!(
            pending,
            vec![(
                from,
                parameter.unwrap(),
                CpgEdge::DataFlow(FlowConfidence::Exact)
            )]
        );
        // Out-of-argument bytes must still refuse; never relax containment.
        if let CpgNode::Variable { end_byte, .. } = &mut graph[from] {
            *end_byte = span.end + 1;
        }
        assert!(CodePropertyGraph::argument_var_node_in_span(
            &site.caller,
            parsed,
            &AccessPath::simple("value"),
            span,
            &index,
            &graph
        )
        .is_none());
    }
    println!("NS_AUDIT {}/{ext}/{mode} {disposition:?} args={args:?} indexed={indexed_span:?} phantom_raw={phantom_raw} escaping={escaping_captures:?} retained_true_use={actual_spans:?} edges={}",case.id,boundary.len());
    disposition
}

#[test]
fn namespace_flow_source_backed_classification() {
    let mut mismatches = Vec::new();
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for case in CASES {
            let files = audit_files(lang, ext, case);
            let cpg = CodePropertyGraph::build(&files);
            let actual = observe(case, ext, &files, &cpg, "full");
            if actual != case.expected {
                mismatches.push(format!(
                    "{}/{ext}: {actual:?} != {:?}",
                    case.id, case.expected
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn namespace_flow_classification_epochs() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let mut previous: Option<CodePropertyGraph> = None;
        let mut previous_sources: BTreeMap<String, String> = BTreeMap::new();
        for (epoch, case_index) in [2, 4, 2, 12, 4].into_iter().enumerate() {
            let case = &CASES[case_index];
            let files = audit_files(lang, ext, case);
            let full = CodePropertyGraph::build(&files);
            assert_eq!(
                observe(case, ext, &files, &full, &format!("epoch{epoch}-full")),
                case.expected
            );
            let current = if let Some(old) = previous {
                let changed: BTreeSet<_> = files
                    .iter()
                    .filter(|(p, f)| previous_sources.get(*p) != Some(&f.source))
                    .map(|(p, _)| p.clone())
                    .collect();
                assert_eq!(changed, BTreeSet::from([format!("app.{ext}")]));
                let incremental = CodePropertyGraph::build_incremental(
                    old.call_graph,
                    old.dfg,
                    &changed,
                    &files,
                    None,
                );
                assert_eq!(
                    observe(
                        case,
                        ext,
                        &files,
                        &incremental,
                        &format!("epoch{epoch}-incremental")
                    ),
                    case.expected
                );
                incremental
            } else {
                full
            };
            previous = Some(current);
            previous_sources = files
                .iter()
                .map(|(p, f)| (p.clone(), f.source.clone()))
                .collect();
        }
    }
}

#[test]
fn namespace_flow_nested_callable_ownership_observations() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for (style, callback, phantom_expected) in [
            (
                "initializer",
                "const cb = function(inner) { return inner; };",
                false,
            ),
            (
                "assignment",
                "let cb; cb = function(inner) { return inner; };",
                true,
            ),
        ] {
            let source = format!(
                "function outer(seed) {{\nlet local; local = seed;\n{callback}\nreturn local;\n}}"
            );
            let parsed = ParsedFile::parse(&format!("nested.{ext}"), &source, lang).unwrap();
            assert!(!parsed.tree.root_node().has_error());
            let outer = parsed
                .all_functions()
                .into_iter()
                .find(|n| {
                    parsed
                        .language
                        .function_name(n)
                        .is_some_and(|name| parsed.node_text(&name) == "outer")
                })
                .unwrap();
            let lines = (1..=5).collect();
            let spans = parsed.rvalue_identifier_spans_on_lines(&outer, &lines);
            let paths = parsed.rvalue_identifier_paths_on_lines(&outer, &lines);
            let parameter = source.find("inner)").unwrap();
            // Existing false ownership is an audit observation, not desired behavior.
            assert_eq!(
                spans
                    .iter()
                    .any(|s| s.path.to_string() == "inner" && s.start_byte == parameter),
                phantom_expected
            );
            let path_has_inner = paths.iter().any(|(p, _)| p.to_string() == "inner");
            let body = source.find("return inner").unwrap() + 7;
            assert!(spans
                .iter()
                .any(|s| s.path.to_string() == "inner" && s.start_byte == body));
            assert_eq!(path_has_inner, phantom_expected);
            // Genuine immediate RHS and return reads must survive a future repair.
            assert!(spans.iter().any(|s| s.path.to_string() == "seed"
                && s.start_byte == source.find("= seed").unwrap() + 2));
            assert!(spans.iter().any(|s| s.path.to_string() == "local"
                && s.start_byte == source.find("return local").unwrap() + 7));
            println!("NS_OWNERSHIP {ext}/{style} outer_has_nested_parameter_use={phantom_expected} path_has_inner={path_has_inner} nested_return_use=true immediate_rhs_and_return_reads=true");
        }
    }
}

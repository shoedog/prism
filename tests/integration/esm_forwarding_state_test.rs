//! Source epoch transitions must revoke and restore forwarded callable flow.
use prism::{
    ast::ParsedFile,
    cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess},
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};

const ORIGIN: &str = "function origin(input) { return input; }\nexport { origin as item };";
const BRIDGE: &str = "import { item as local } from './origin'; export { local as publicName };";
const APP: &str = "import { publicName as invoke } from './bridge';\nfunction run(value) { return invoke(value); }";

fn endpoints(
    cpg: &CodePropertyGraph,
    files: &BTreeMap<String, ParsedFile>,
    app: &str,
) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for edge in cpg.graph.edge_indices() {
        if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
            continue;
        }
        let (a, b) = cpg.graph.edge_endpoints(edge).unwrap();
        if let (
            CpgNode::Variable {
                file,
                function,
                path,
                access: VarAccess::Use,
                ..
            },
            CpgNode::Variable {
                file: to,
                function: callee,
                path: param,
                access: VarAccess::Def,
                start_byte,
                end_byte,
                ..
            },
        ) = (cpg.node(a), cpg.node(b))
        {
            if file == app && function == "run" && path.to_string() == "value" && to != app {
                assert_eq!(&files[to].source[*start_byte..*end_byte], "input");
                assert_eq!(param.to_string(), "input");
                out.insert((to.clone(), callee.clone()));
            }
        }
    }
    out
}

fn transitions(lang: Language, ext: &str) {
    let origin_path = format!("origin.{ext}");
    let bridge_path = format!("bridge.{ext}");
    let app_path = format!("app.{ext}");
    let states = [
        (ORIGIN.to_owned(), BRIDGE.to_owned(), Some("origin")),
        (ORIGIN.to_owned(), format!("{BRIDGE}\nlocal = other;"), None),
        (ORIGIN.to_owned(), BRIDGE.to_owned(), Some("origin")),
        (
            format!("{ORIGIN}\norigin = other;"),
            BRIDGE.to_owned(),
            None,
        ),
        (ORIGIN.to_owned(), BRIDGE.to_owned(), Some("origin")),
        (
            ORIGIN.to_owned(),
            format!("{BRIDGE}\nexport {{ local as publicName }};"),
            None,
        ),
        (
            ORIGIN.replace("origin", "replacement"),
            BRIDGE.to_owned(),
            Some("replacement"),
        ),
        (ORIGIN.to_owned(), BRIDGE.to_owned(), Some("origin")),
    ];
    let mut previous: Option<CodePropertyGraph> = None;
    let mut previous_sources: BTreeMap<String, String> = BTreeMap::new();
    for (epoch, (origin, bridge, target)) in states.into_iter().enumerate() {
        let files: BTreeMap<_, _> = [
            (origin_path.clone(), origin),
            (bridge_path.clone(), bridge),
            (app_path.clone(), APP.to_owned()),
            (
                format!("decoy.{ext}"),
                "function invoke(input) { return input; }".to_owned(),
            ),
        ]
        .into_iter()
        .map(|(p, s)| {
            let parsed = ParsedFile::parse(&p, &s, lang).unwrap();
            assert!(!parsed.tree.root_node().has_error());
            (p, parsed)
        })
        .collect();
        let full = CodePropertyGraph::build(&files);
        let expected = target
            .map(|n| (origin_path.clone(), n.to_owned()))
            .into_iter()
            .collect();
        assert_eq!(
            endpoints(&full, &files, &app_path),
            expected,
            "{ext} full epoch{epoch}"
        );
        let current = if let Some(old) = previous {
            let changed: BTreeSet<_> = files
                .iter()
                .filter(|(path, parsed)| previous_sources.get(*path) != Some(&parsed.source))
                .map(|(path, _)| path.clone())
                .collect();
            let expected_changed = match epoch {
                1 | 2 | 5 => BTreeSet::from([bridge_path.clone()]),
                3 | 4 | 7 => BTreeSet::from([origin_path.clone()]),
                6 => BTreeSet::from([origin_path.clone(), bridge_path.clone()]),
                _ => unreachable!(),
            };
            assert_eq!(changed, expected_changed, "{ext} changed set epoch{epoch}");
            let incremental = CodePropertyGraph::build_incremental(
                old.call_graph,
                old.dfg,
                &changed,
                &files,
                None,
            );
            assert_eq!(
                endpoints(&incremental, &files, &app_path),
                expected,
                "{ext} incremental epoch{epoch}"
            );
            incremental
        } else {
            full
        };
        previous = Some(current);
        previous_sources = files
            .iter()
            .map(|(path, parsed)| (path.clone(), parsed.source.clone()))
            .collect();
    }
}

#[test]
fn esm_forwarding_js_source_epochs() {
    transitions(Language::JavaScript, "js");
}
#[test]
fn esm_forwarding_ts_source_epochs() {
    transitions(Language::TypeScript, "ts");
}
#[test]
fn esm_forwarding_tsx_source_epochs() {
    transitions(Language::Tsx, "tsx");
}

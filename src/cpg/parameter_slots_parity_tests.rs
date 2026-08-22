//! Parallel/serial Step-5b coverage for fail-closed JavaScript/TypeScript slots.

use super::build::{compute_param_names, CodePropertyGraph};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;

#[test]
fn step5b_parallel_and_serial_match_for_unknown_and_truncated_js_ts_slots() {
    let fixtures = [
        (
            "slots.js",
            Language::JavaScript,
            "function destruct({x}, cb) { return cb; }\n\
             function duplicate({x = 0}, x) { return x; }\n\
             function rest(a, ...rest) { return a; }\n\
             function run(first, second) { destruct(first, second); duplicate(first, second); rest(first, second); }\n",
        ),
        (
            "slots.ts",
            Language::TypeScript,
            "function destruct({x}: {x: number}, cb: number) { return cb; }\n\
             function duplicate({x = 0}: {x?: number}, x: number) { return x; }\n\
             function rest(a: number, ...rest: number[]) { return a; }\n\
             function run(first: number, second: number) { destruct(first, second); duplicate(first, second); rest(first, second); }\n",
        ),
    ];

    for (path, language, source) in fixtures {
        let files = BTreeMap::from([(
            path.to_string(),
            ParsedFile::parse(path, source, language).unwrap(),
        )]);
        let cpg = CodePropertyGraph::build(&files);
        let parallel = CodePropertyGraph::collect_step5b_edges(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        let serial = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );

        let params = |name: &str| {
            let function = cpg.call_graph.functions[name].first().unwrap();
            compute_param_names(files.get(path).unwrap(), function)
        };
        assert_eq!(
            params("destruct"),
            Some(vec![]),
            "{language:?}: destructuring must truncate before binding any slot"
        );
        assert_eq!(
            params("duplicate"),
            None,
            "{language:?}: duplicate bindings must fail closed"
        );
        assert_eq!(
            params("rest"),
            Some(vec!["a".to_string()]),
            "{language:?}: the simple prefix before rest remains positional"
        );
        assert_eq!(parallel, serial, "{language:?}: Step-5b par != serial");
    }
}

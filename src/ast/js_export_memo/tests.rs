use super::*;
use crate::languages::Language;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Stats {
    pub lexical: usize,
    pub writes: usize,
    pub replacements: usize,
    pub inventories: usize,
    pub references: usize,
}
thread_local! {
    static REFERENCE: Cell<bool> = Cell::new(std::env::var_os("PRISM_SLICE_O_REFERENCE").is_some());
    static STATS: Cell<Stats> = Cell::new(Stats::default());
}
pub(super) fn reference() -> bool {
    REFERENCE.with(Cell::get)
}
pub(super) fn count(update: impl FnOnce(&mut Stats)) {
    STATS.with(|s| {
        let mut value = s.get();
        update(&mut value);
        s.set(value);
    });
}
fn reference_run<T>(op: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REFERENCE.with(|r| r.set(self.0));
        }
    }
    let _restore = Restore(REFERENCE.with(|r| r.replace(true)));
    op()
}
fn parse(source: &str, language: Language) -> ParsedFile {
    ParsedFile::parse("fixture.ts", source, language).unwrap()
}
fn fixtures() -> Vec<String> {
    let mut cases = vec![
        "function target() {} exports.run = target; module.exports.other = target; require('./m');".into(),
        "function target() {} module.exports = {run: target}; exports.run = target;".into(),
        "function target() {} module.exports = target; exports = module.exports; exports.run = target;".into(),
        "function target() {} exports.run = target; module = {};".into(),
        "function target() {} exports.run = target; exports = {};".into(),
        "function target() {} module.exports.run = target; module.exports = {};".into(),
        "function target() {} module.exports.run = target; module['exports'] = {};".into(),
        "function target() {} exports.run = target; function mutate() { exports++; module = {}; }".into(),
        "import {exports, module, require} from './m'; exports.run = target; require('./m');".into(),
        "import type {exports} from './m'; exports.run = target;".into(),
        "export function target() {} export const arrow = () => target(); export {target as other};".into(),
        "import {target} from './m'; export {target};".into(),
        "export default class C {} const c = new C();".into(),
        "namespace exports { export function f() { require('./m'); } } exports.run = target;".into(),
        "namespace N { var exports; exports.run = target; } exports.run = target;".into(),
        "namespace N { const = ; exports.run = target; } exports.run = target;".into(),
        "namespace N { namespace M { var exports; require('./m'); } exports.run = target; }".into(),
        "import module = N; module.exports.run = target;".into(),
        "const C = class exports { method() { exports.run(); require('./m'); } }; exports.run = target;".into(),
        "class exports { method(module) { module.exports.run(); } } exports.run = target;".into(),
        "abstract class exports {} enum module {x} declare function require(): void; exports.run = target;".into(),
        "interface exports {} type module = {}; exports.run = target; module.exports.run = target;".into(),
        "function exports() {} exports.run = target; const f = function module() { module.exports.run(); };".into(),
        "function* module() { module.exports.run(); } const f = function* exports() { exports.run(); };".into(),
        "const require = (exports => exports.run()); exports.run = target; require('./m');".into(),
        "for (let exports of xs) { exports.run(); } exports.run = target;".into(),
        "for (var exports in xs) { exports.run(); } exports.run = target;".into(),
        "for (const module of xs) { module++; } module.exports.run = target;".into(),
        "for (using module of xs) {} module.exports.run = target;".into(),
        "using module = acquire(); module.exports.run = target;".into(),
        "try {} catch ({exports}) { exports.run(); } exports.run = target;".into(),
        "switch (x) {case 1: let exports; exports.run(); break;} exports.run = target;".into(),
        "function f({x: exports = require('./m')}, module = exports) { exports.run(); module++; }".into(),
        "const f = (module = require('./m')) => { let exports; exports.run(); }; exports.run = target;".into(),
        "exports.run = target; const = ; module.exports.run = target; require('./m');".into(),
        "function f(exports { exports.run(); } exports.run = target;".into(),
        "const C = class exports { [exports.run()] = require('./m'); }; exports.run = target;".into(),
        "module.exports.run = target; ({module} = other); [exports] = other; require++;".into(),
        "module.exports.run = target; const f = function module() { module++; };".into(),
        "module.exports.run = target; function module() { module++; }".into(),
        "function f() { for (let exports of xs) { require('./m'); } require('./m'); }".into(),
    ];
    // Each declaration at module, function, nested function and sibling block
    // levels, with calls both inside and outside the declaration's scope.
    for name in ["exports", "module", "require"] {
        for declaration in [
            format!("var {name};"),
            format!("let {name};"),
            format!("const {name} = other;"),
            format!("function {name}() {{}}"),
            format!("class {name} {{}}"),
            format!("import {{{name}}} from './m';"),
        ] {
            let calls = "exports.run = target; module.exports.run = target; require('./m');";
            for body in [
                format!("{declaration} {calls}"),
                format!("{{ {declaration} {calls} }} {calls}"),
                format!(
                    "function f() {{ {declaration} {calls} function g() {{ {calls} }} }} {calls}"
                ),
                format!(
                    "function f() {{ {{ {declaration} {calls} }} {{ {calls} }} {calls} }} {calls}"
                ),
                format!("function f({name}) {{ {calls} function g() {{ {calls} }} }} {calls}"),
                format!("try {{}} catch ({name}) {{ {calls} function g() {{ {calls} }} }} {calls}"),
            ] {
                cases.push(format!("function target() {{}} {body}"));
            }
        }
    }
    cases
}

#[test]
fn every_ambient_occurrence_and_write_matches_frozen_helpers() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in fixtures() {
            let p = parse(&source, language);
            let _session = Session::new(&p);
            let mut stack = vec![p.tree.root_node()];
            while let Some(node) = stack.pop() {
                if node.kind() == "identifier"
                    && matches!(p.node_text(&node), "module" | "exports" | "require")
                {
                    assert_eq!(
                        p.js_ts_export_lexically_bound(node),
                        p.js_ts_receiver_lexically_bound_at_call(&p.tree.root_node(), Some(node)),
                        "lexical {:?} at {}: {source}",
                        language,
                        node.start_byte()
                    );
                    assert_eq!(
                        p.js_ts_unshadowed_ambient(node),
                        p.js_ts_unshadowed_ambient_reference(node),
                        "ambient {:?} at {}: {source}",
                        language,
                        node.start_byte()
                    );
                }
                let mut cursor = node.walk();
                stack.extend(node.named_children(&mut cursor));
            }
            for name in ["module", "exports", "require", "target", "absent"] {
                for skip_using in [false, true, false] {
                    assert_eq!(
                        p.js_ts_module_written(name, skip_using),
                        p.js_ts_module_written_reference(name, skip_using),
                        "writes {name}: {source}"
                    );
                }
            }
            assert_eq!(
                p.js_ts_module_exports_replaced(p.tree.root_node()),
                p.js_ts_module_exports_replaced_reference(p.tree.root_node()),
                "replacement: {source}"
            );
        }
    }
}

#[test]
fn complete_export_facts_match_frozen_path_cold_and_warm() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in fixtures() {
            let expected = reference_run(|| parse(&source, language).extract_js_ts_export_facts());
            let p = parse(&source, language);
            let actual = p.extract_js_ts_export_facts();
            assert_eq!(actual, expected, "{language:?}: {source}");
            assert_eq!(
                bincode::serialize(&actual).unwrap(),
                bincode::serialize(&expected).unwrap()
            );
            assert_eq!(p.extract_js_ts_export_facts(), expected);
        }
    }
}

#[test]
fn absent_names_and_same_scope_queries_have_bounded_work() {
    for (declaration, lexical) in [("", 0), ("let exports;", 1), ("const = ;", 1)] {
        let source = format!("{declaration}\n{}", "exports.run = target;\n".repeat(256));
        let p = parse(&source, Language::JavaScript);
        STATS.with(|s| s.set(Stats::default()));
        let _facts = p.extract_js_ts_export_facts();
        let stats = STATS.with(Cell::get);
        assert_eq!(stats.inventories, 1, "{declaration}: {stats:?}");
        assert_eq!(stats.lexical, lexical, "{declaration}: {stats:?}");
        assert!(
            stats.writes <= 1 && stats.replacements <= 1,
            "{declaration}: {stats:?}"
        );
    }
}

#[test]
fn sessions_release_on_nesting_unwind_and_file_replacement() {
    let mut p = parse("exports.run = target;", Language::JavaScript);
    let q = parse(
        "let exports; exports = other; exports.run = target;",
        Language::JavaScript,
    );
    let outer = Session::new(&p);
    let inner = Session::new(&p);
    drop(outer); // out-of-order drop must retain the inner session
    assert!(active(&p).is_some());
    // A same-tree, non-ambient spelling is intentionally absent from the
    // inventory's scope map and must fall back to the original predicate.
    let start = p.source.find("target").unwrap();
    let own_identifier = p
        .tree
        .root_node()
        .descendant_for_byte_range(start, start + 6)
        .unwrap();
    assert_eq!(own_identifier.kind(), "identifier");
    assert_eq!(
        p.js_ts_export_lexically_bound(own_identifier),
        p.js_ts_receiver_lexically_bound_at_call(&p.tree.root_node(), Some(own_identifier))
    );
    let unknown = parse("other();", Language::JavaScript);
    let identifier = unknown
        .tree
        .root_node()
        .named_child(0)
        .unwrap()
        .named_child(0)
        .unwrap()
        .child_by_field_name("function")
        .unwrap();
    assert_eq!(
        p.js_ts_export_lexically_bound(identifier),
        p.js_ts_receiver_lexically_bound_at_call(&p.tree.root_node(), Some(identifier))
    );
    let foreign = q.tree.root_node().named_child(1).unwrap();
    assert_eq!(
        p.js_ts_module_exports_replaced(foreign),
        p.js_ts_module_exports_replaced_reference(foreign)
    );
    let other = Session::new(&q);
    assert!(q.js_ts_module_written("exports", false));
    assert!(!p.js_ts_module_written("exports", false));
    drop(other);
    assert!(active(&p).is_some());
    drop(inner);
    assert!(active(&p).is_none() && active(&q).is_none());
    assert_eq!(
        p.js_ts_export_lexically_bound(own_identifier),
        p.js_ts_receiver_lexically_bound_at_call(&p.tree.root_node(), Some(own_identifier))
    );
    // Outside a session all public helpers retain their original raw behavior.
    assert_eq!(
        p.js_ts_module_written("exports", false),
        p.js_ts_module_written_reference("exports", false)
    );
    assert_eq!(
        p.js_ts_module_exports_replaced(p.tree.root_node()),
        p.js_ts_module_exports_replaced_reference(p.tree.root_node())
    );
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _session = Session::new(&p);
        assert!(!p.js_ts_module_written("exports", false));
        panic!("exercise session cleanup");
    }));
    assert!(active(&p).is_none());
    p = parse("exports = other;", Language::JavaScript);
    let _session = Session::new(&p);
    assert!(p.js_ts_module_written("exports", false));
}

#[test]
fn serialized_call_graphs_match_reference_at_one_and_four_threads() {
    const OUTPUT: &str = "PRISM_SLICE_O_GRAPH_OUTPUT";
    if let Some(output) = std::env::var_os(OUTPUT) {
        let mut files = BTreeMap::new();
        for (language, ext) in [
            (Language::JavaScript, "js"),
            (Language::TypeScript, "ts"),
            (Language::Tsx, "tsx"),
        ] {
            for (i, source) in fixtures().into_iter().enumerate() {
                let path = format!("fixture{i}.{ext}");
                files.insert(
                    path.clone(),
                    ParsedFile::parse(&path, &source, language).unwrap(),
                );
            }
        }
        files.insert(
            "m.ts".into(),
            ParsedFile::parse(
                "m.ts",
                "export function target() {} export function require() {}",
                Language::TypeScript,
            )
            .unwrap(),
        );
        files.insert(
            "consumer.ts".into(),
            ParsedFile::parse(
                "consumer.ts",
                "import {run} from './fixture0'; function caller() { run(); }",
                Language::TypeScript,
            )
            .unwrap(),
        );
        let graph = crate::call_graph::CallGraph::build(&files);
        let counts: Vec<_> = crate::build_pool::build_pool().broadcast(|_| STATS.with(Cell::get));
        let references: usize = counts.iter().map(|s| s.references).sum();
        let inventories: usize = counts.iter().map(|s| s.inventories).sum();
        assert_eq!(
            references > 0,
            reference(),
            "reference route did not execute"
        );
        assert_eq!(
            inventories > 0,
            !reference(),
            "indexed route did not execute"
        );
        std::fs::write(output, bincode::serialize(&graph).unwrap()).unwrap();
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut snapshots = Vec::new();
    for threads in ["1", "4"] {
        for old in [true, false] {
            let output = dir.path().join(format!("{threads}-{old}"));
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", "ast::js_export_memo::tests::serialized_call_graphs_match_reference_at_one_and_four_threads", "--nocapture"])
                .env("RAYON_NUM_THREADS", threads).env(OUTPUT, &output);
            if old {
                command.env("PRISM_SLICE_O_REFERENCE", "1");
            } else {
                command.env_remove("PRISM_SLICE_O_REFERENCE");
            }
            let result = command.output().unwrap();
            assert!(
                result.status.success(),
                "{threads}/{old}: {}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            snapshots.push(std::fs::read(output).unwrap());
        }
    }
    for snapshot in &snapshots[1..] {
        assert!(
            snapshot == &snapshots[0],
            "whole serialized call graph differs"
        );
    }
}

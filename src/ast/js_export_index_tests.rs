use super::*;
use crate::languages::Language;

thread_local! {
    static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(super) fn record_build() {
    BUILDS.with(|n| n.set(n.get() + 1));
}
impl ParsedFile {
    pub(crate) fn export_index_builds() -> usize {
        BUILDS.with(std::cell::Cell::get)
    }
    pub(crate) fn export_facts_cached(&self) -> bool {
        self.js_export_facts.get().is_some()
    }
}

fn check(source: &str, language: Language) {
    let parsed = ParsedFile::parse("index.ts", source, language).unwrap();
    let root = parsed.tree.root_node();
    let index = ExportIndex::new(&parsed);
    let mut names = BTreeSet::from(["absent".to_string()]);
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(
            node.kind(),
            "identifier" | "type_identifier" | "shorthand_property_identifier_pattern"
        ) {
            names.insert(parsed.node_text(&node).to_string());
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    for name in &names {
        assert_eq!(
            index.written(&parsed, name),
            parsed.js_ts_module_value_written(name),
            "write {name}: {source}"
        );
        let mut allowed = index.declarations.get(name).cloned().unwrap_or_default();
        allowed.insert(root.id());
        for id in allowed {
            assert_eq!(
                index.competes(name, id),
                parsed.js_ts_forwarding_competitor_except(root, name, &[id]),
                "competitor {name}: {source}"
            );
        }
        assert_eq!(
            parsed.js_ts_esm_named_imports(Some(name)),
            parsed.js_ts_esm_named_imports_legacy(Some(name)),
            "only {name}: {source}"
        );
    }
    assert_eq!(
        parsed.js_ts_esm_named_imports(None),
        parsed.js_ts_esm_named_imports_legacy(None),
        "imports: {source}"
    );
    assert_eq!(
        parsed.js_ts_forwardable_functions(),
        parsed.js_ts_forwardable_functions_legacy(),
        "forwarding: {source}"
    );
}

#[test]
fn indexed_predicates_match_legacy_syntax_and_shadowing() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in [
            "",
            "import {a, b as alias} from './m'; import type {T} from './t'; function origin() {}",
            "import {a} from './m'; a = other; function origin() { origin = other; }",
            "import {a} from './m'; function origin(a) { a++; } function okay() {}",
            "function origin() {} function nested() { function origin() {} }",
            "function origin() {} const {a: origin, ...rest} = x; const [first, ...others] = y;",
            "function origin() {} class C { method({origin}) { origin++; } *gen() { yield origin; } }",
            "function origin() {} let expr = function origin() { origin++; }; let gen = function* origin() { origin++; };",
            "let origin; function write({x = (origin = other)}) {} consume(() => { origin++; });",
            "let origin; try {} catch(origin) { origin = other; } for (let origin of values) { origin++; }",
            "import {a,b,c,d} from './m'; ({a} = x); [b] = y; c += 1; for (d of values) {}",
            "function origin() {} interface origin {} type alias = string; enum E {} namespace N.Inner {} import Q = N;",
            "declare function origin(): void; export function origin() {}",
            "export default class C {} const expr = class C {}; abstract class A {}",
            "function origin() {} with (object) { origin = other; } eval(''); module.exports = origin;",
            "import {a} from './m'; const = ; function origin( { return a; }",
            "import {a} from './m'; using x = value; await using y = value; a++;",
        ] {
            check(source, language);
        }
    }
}

#[test]
fn import_filter_pins_visible_writes_and_shadowed_writes() {
    let parsed = ParsedFile::parse("index.ts", "import {a,b,c,d,e,f} from './m'; a = x; b++; [c] = y; for (d of xs) {} function inner(e) { e++; } const x = function f() { f++; };", Language::TypeScript).unwrap();
    assert_eq!(
        parsed.js_ts_esm_named_imports(None),
        BTreeSet::from(["e".into(), "f".into()])
    );
    assert!(parsed.js_ts_esm_named_imports(Some("a")).is_empty());
    assert_eq!(
        parsed.js_ts_esm_named_imports(Some("e")),
        BTreeSet::from(["e".into()])
    );
}

#[test]
fn empty_and_refused_queries_do_not_build_an_index() {
    for source in [
        "",
        "import type {A} from './m';",
        "import {a} from './m'; const = ;",
        "function f() { eval(''); }",
    ] {
        let parsed = ParsedFile::parse("index.ts", source, Language::TypeScript).unwrap();
        super::super::tests::reset_visits();
        parsed.js_ts_esm_named_imports(None);
        parsed.js_ts_forwardable_functions();
        assert_eq!(super::super::tests::visits(), 0, "{source}");
    }
}

#[test]
fn many_import_queries_share_one_inventory() {
    let imports = (0..128)
        .map(|i| format!("f{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let source =
        format!("import {{{imports}}} from './m'; function nested(f0) {{ f0++; }} f1 = other;");
    let parsed = ParsedFile::parse("index.ts", &source, Language::TypeScript).unwrap();
    let mut stack = vec![parsed.tree.root_node()];
    let mut named_nodes = 0;
    while let Some(node) = stack.pop() {
        named_nodes += 1;
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    super::super::tests::reset_visits();
    let names = parsed.js_ts_esm_named_imports(None);
    assert_eq!(names.len(), 127);
    assert!(names.contains("f0") && !names.contains("f1"));
    let visits = super::super::tests::visits();
    let bound = named_nodes + 2; // f0 and f1 each require one matching-target scope check.
    println!("IMPORT_WORK {visits} visits; {named_nodes} nodes; bound {bound}");
    assert!(
        visits > 0 && visits <= bound,
        "{visits} visits > {bound} for 128 imports"
    );
    super::super::tests::reset_visits();
    assert_eq!(parsed.js_ts_esm_named_imports_legacy(None), names);
    let legacy_visits = super::super::tests::visits();
    println!("IMPORT_WORK_LEGACY {legacy_visits} visits; bound {bound}");
    assert!(legacy_visits > bound, "legacy witness must discriminate");
}

#[test]
fn single_name_queries_do_not_build_an_index() {
    let parsed = ParsedFile::parse(
        "index.ts",
        "import {a,b,c} from './m'; a++; function inner(b) { b++; }",
        Language::TypeScript,
    )
    .unwrap();
    let before = ParsedFile::export_index_builds();
    for (name, expected) in [("a", false), ("b", true), ("c", true), ("absent", false)] {
        assert_eq!(
            parsed.js_ts_esm_named_imports(Some(name)).contains(name),
            expected
        );
    }
    let builds = ParsedFile::export_index_builds() - before;
    println!("SINGLE_NAME_BUILDS {builds}");
    assert_eq!(builds, 0);
}

#[test]
fn written_refuses_non_js_ts_languages() {
    let parsed = ParsedFile::parse(
        "index.java",
        "class C { void f() { a = b; } }",
        Language::Java,
    )
    .unwrap();
    assert!(!parsed.tree.root_node().has_error());
    let index = ExportIndex::new(&parsed);
    assert!(
        index.writes.contains_key("a"),
        "exercise a matching write target"
    );
    assert!(!parsed.js_ts_module_value_written("a"));
    assert!(!index.written(&parsed, "a"));
    assert!(!index.written(&parsed, "absent"));
}

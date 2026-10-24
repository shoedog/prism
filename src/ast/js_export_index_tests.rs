use super::*;
use crate::languages::Language;

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
    super::super::tests::reset_visits();
    let names = parsed.js_ts_esm_named_imports(None);
    assert_eq!(names.len(), 127);
    assert!(names.contains("f0") && !names.contains("f1"));
    let visits = super::super::tests::visits();
    assert!(
        visits > 0 && visits < 400,
        "{visits} visits for 128 imports"
    );
}

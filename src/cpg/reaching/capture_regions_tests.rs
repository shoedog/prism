//! Which bytes of a JS/TS/TSX owner lie in which deferred region (#660 S1a, design 4.4 and 6.1f).
use super::*;
use crate::languages::Language;

/// The region kind at the first occurrence of `needle` after the first occurrence of `after`.
fn kind_at(language: Language, src: &str, after: &str, needle: &str) -> Option<RegionKind> {
    let file = ParsedFile::parse("a.ts", src, language).unwrap();
    assert_eq!(file.parse_error_count, 0, "{src}");
    let owner = file.tree.root_node().child(0).unwrap();
    let regions = js_regions(&file, owner);
    let from = src.find(after).unwrap();
    let byte = from + src[from..].find(needle).unwrap();
    regions.innermost(byte).map(|r| regions.kind(r))
}

const TS: Language = Language::TypeScript;
const JS: Language = Language::JavaScript;

#[test]
fn parameter_lists_and_bodies_are_regions() {
    let src = "function f(y) { g(function (z = YY) { return BB; }); }";
    assert_eq!(kind_at(JS, src, "g(", "YY"), Some(RegionKind::Param));
    assert_eq!(kind_at(JS, src, "g(", "BB"), Some(RegionKind::Body));
    // The owner's own parameter list is no region.
    assert_eq!(kind_at(JS, "function f(a = YY) { g(); }", "f(", "YY"), None);
}

#[test]
fn a_lone_arrow_parameter_is_a_parameter_list() {
    let src = "function f(y) { g(PP => PP + 1); }";
    assert_eq!(kind_at(JS, src, "g(", "PP"), Some(RegionKind::Param));
}

#[test]
fn methods_generators_and_async_callables_have_parameter_regions() {
    for src in [
        "function f(y) { return { m(z = YY) {} }; }",
        "function f(y) { return { async m(z = YY) {} }; }",
        "function f(y) { g(function* (z = YY) {}); }",
        "function f(y) { g(async (z = YY) => z); }",
        "function f(y) { const C = class { m(z = YY) {} }; }",
    ] {
        assert_eq!(
            kind_at(JS, src, "{", "YY"),
            Some(RegionKind::Param),
            "{src}"
        );
    }
}

#[test]
fn only_instance_field_values_are_regions() {
    let inst = "function f(y) { const C = class { x = YY; }; }";
    assert_eq!(kind_at(JS, inst, "class", "YY"), Some(RegionKind::Field));
    let private = "function f(y) { const C = class { #x = YY; }; }";
    assert_eq!(kind_at(JS, private, "class", "YY"), Some(RegionKind::Field));
    for (language, src) in [
        (JS, "function f(y) { const C = class { static x = YY; }; }"),
        (TS, "function f(y) { const C = class { static x = YY; }; }"),
        (JS, "function f(y) { const C = class { static { YY; } }; }"),
        (JS, "function f(y) { const C = class { [YY]() {} }; }"),
        (JS, "function f(y) { const C = class { [YY] = 1; }; }"),
    ] {
        assert_eq!(kind_at(language, src, "class", "YY"), None, "{src}");
    }
    let ts = "function f(y) { const C = class { x: number = YY; }; }";
    assert_eq!(kind_at(TS, ts, "class", "YY"), Some(RegionKind::Field));
}

#[test]
fn a_callable_in_an_eager_position_is_its_own_region() {
    let key = "function f(y) { const C = class { [(() => YY)()]() {} }; }";
    assert_eq!(kind_at(JS, key, "class", "YY"), Some(RegionKind::Body));
    let stat = "function f(y) { const C = class { static x = () => YY; }; }";
    assert_eq!(kind_at(JS, stat, "class", "YY"), Some(RegionKind::Body));
}

#[test]
fn parameter_decorators_are_eager_but_nested_callables_inside_them_are_regions() {
    let src = "function f(y) { const C = class { constructor(@dec(DD) p, q = QQ) {} }; }";
    assert_eq!(kind_at(TS, src, "class", "DD"), None);
    assert_eq!(kind_at(TS, src, "class", "QQ"), Some(RegionKind::Param));
    let nested = "function f(y) { const C = class { constructor(@dec(() => NN) p) {} }; }";
    assert_eq!(kind_at(TS, nested, "class", "NN"), Some(RegionKind::Body));
    // A method decorator is eager too.
    let method = "function f(y) { const C = class { @dec(MM) m() {} }; }";
    assert_eq!(kind_at(TS, method, "class", "MM"), None);
}

#[test]
fn nested_regions_nest_and_within_walks_outward() {
    let src = "function f(y) { g(() => { h(() => { return II; }); OO; }); }";
    let file = ParsedFile::parse("a.js", src, JS).unwrap();
    let owner = file.tree.root_node().child(0).unwrap();
    let regions = js_regions(&file, owner);
    let at = |needle: &str| src.find(needle).unwrap();
    let inner = regions.innermost(at("II")).unwrap();
    let outer = regions.innermost(at("OO")).unwrap();
    assert_ne!(inner, outer);
    assert!(
        regions.within(at("II"), outer),
        "an inner byte is within the outer region"
    );
    assert!(
        !regions.within(at("OO"), inner),
        "an outer byte is not within the inner region"
    );
    assert!(
        !regions.within(at("y)"), outer),
        "the owner's own parameter is in no region"
    );
}

#[test]
fn one_line_key_in_two_regions_is_ambiguous() {
    let src = "function f(a) {\n  g(() => a, () => a);\n}\n";
    let file = ParsedFile::parse("a.js", src, JS).unwrap();
    let owner = file.tree.root_node().child(0).unwrap();
    let regions = js_regions(&file, owner);
    let a = AccessPath::simple("a");
    assert_eq!(regions.line_region(&a, 2), Some(None));
    assert_eq!(
        regions.line_region(&a, 1),
        None,
        "the owner's own parameter is not a region read"
    );
    let one = "function f(a) {\n  g(() => a);\n}\n";
    let file = ParsedFile::parse("a.js", one, JS).unwrap();
    let owner = file.tree.root_node().child(0).unwrap();
    let regions = js_regions(&file, owner);
    assert!(matches!(regions.line_region(&a, 2), Some(Some(_))));
}

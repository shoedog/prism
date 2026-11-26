use super::pattern_params::PatternParam;
use crate::{ast::ParsedFile, languages::Language};

fn facts(source: &str, language: Language) -> Vec<PatternParam> {
    let parsed = ParsedFile::parse("fixture", source, language).unwrap();
    parsed.js_ts_pattern_params(&parsed.all_functions()[0])
}

fn js(source: &str) -> Vec<PatternParam> {
    facts(source, Language::JavaScript)
}

fn admit(pattern: &PatternParam) -> Vec<&str> {
    pattern
        .elements
        .iter()
        .filter(|e| e.admitted)
        .map(|e| e.name.as_str())
        .collect()
}

#[test]
fn runtime_positions_keep_holes_and_empty_objects() {
    for grammar in [Language::TypeScript, Language::Tsx] {
        let patterns = facts(
            "function f(this: C, x: T, /* comment */ {}: T, [a]: T, {b}: T, ...{length}: T) {}",
            grammar,
        );
        assert_eq!(
            patterns.iter().map(|p| p.position).collect::<Vec<_>>(),
            [1, 3]
        );
        assert!(patterns[0].elements.is_empty());
        assert_eq!(admit(&patterns[1]), ["b"]);
    }
}

#[test]
fn source_order_local_bytes_and_structured_paths() {
    let source = "function f({x, outer: {'a.b': local, x: other}, '': empty, x: repeat}) {}";
    let patterns = js(source);
    let elements = &patterns[0].elements;
    assert_eq!(
        admit(&patterns[0]),
        ["x", "local", "other", "empty", "repeat"]
    );
    for (element, path) in elements.iter().zip([
        vec!["x"],
        vec!["outer", "a.b"],
        vec!["outer", "x"],
        vec![""],
        vec!["x"],
    ]) {
        assert_eq!(element.path.as_ref().unwrap(), &path);
        assert_eq!(&source[element.bytes.clone()], element.name);
    }
}

#[test]
fn consumed_keys_include_excluded_and_empty_children() {
    let source = "function f({a, b: renamed, c: [hidden], d: {}, e: [], f = effect(), a: again, ...rest}) {}";
    let patterns = js(source);
    let p = &patterns[0];
    assert_eq!(admit(p), ["a", "renamed", "again", "rest"]);
    assert_eq!(
        p.levels[0]
            .consumed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a", "b", "c", "d", "e", "f"]
    );
    assert_eq!(&source[p.levels[1].bytes.clone()], "{}");
    let rest = p.elements.last().unwrap();
    assert_eq!(rest.rest_level, Some(0));
    assert_eq!(rest.path, Some(vec![]));
    assert!(p.levels[0].accepts_rest_leaf(&["keep".into(), "a".into()]));
    assert!(!p.levels[0].accepts_rest_leaf(&["a".into(), "keep".into()]));
    assert!(!p.levels[0].accepts_rest_leaf(&[]));
}

#[test]
fn unknown_keys_stay_local_to_their_level() {
    let patterns = js("function f({box: {[key]: unknown, ok, ...inside}, ...outside}) {}");
    let p = &patterns[0];
    assert_eq!(admit(p), ["unknown", "ok", "inside", "outside"]);
    assert!(!p.levels[0].unknown_sibling);
    assert!(p.levels[1].unknown_sibling);
    assert_eq!(p.elements[0].path, None);
    assert_eq!(p.elements[1].path.as_ref().unwrap(), &["box", "ok"]);
    assert_eq!(p.elements[2].rest_level, Some(1));
    assert_eq!(p.elements[3].rest_level, Some(0));
    assert!(!p.levels[1].accepts_rest_leaf(&["box".into(), "keep".into()]));
    assert!(p.levels[0].accepts_rest_leaf(&["keep".into()]));
}

#[test]
fn unknown_ancestor_prevents_all_descendant_paths() {
    let patterns = js("function f({[key]: {child, ...rest}, ok}) {}");
    let p = &patterns[0];
    assert_eq!(admit(p), ["child", "rest", "ok"]);
    assert_eq!(p.levels[1].path, None);
    assert_eq!(p.elements[0].path, None);
    assert_eq!(p.elements[1].path, None);
}

#[test]
fn safe_property_spelling_is_independent_of_local_admission() {
    let patterns = js(
        r#"function f({'1': a, 1: b, 1e0: c, 9007199254740992: d, 'aria-label': e, '\u0061': f, café: g, __proto__: h}) {}"#,
    );
    let p = &patterns[0];
    assert_eq!(admit(p), ["a", "b", "c", "d", "e", "f", "g", "h"]);
    assert_eq!(
        p.levels[0]
            .consumed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["1", "__proto__", "aria-label"]
    );
    assert!(p.levels[0].unknown_sibling);
}

#[test]
fn every_intermediate_default_must_be_inert() {
    let patterns = js("function f({outer: {a, b: {c} = make(), d: {e} = ({}), f = true}, keep} = {/* comment */}) {}" );
    let p = &patterns[0];
    assert_eq!(admit(p), ["a", "e", "f", "keep"]);
    assert!(p.elements.iter().all(|e| e.defaulted));
    assert!(p.levels[1].consumed.contains("b"));
}

#[test]
fn inert_values_accept_comments_but_not_array_holes() {
    let patterns = js("function f({a = 1, b = 's', c = false, d = null, e = [/* empty */], f = [,], g = {k: 1}, h = [], i = -1, plain}) {}" );
    let p = &patterns[0];
    assert_eq!(admit(p), ["a", "b", "c", "d", "e", "h", "plain"]);
    assert!(!p.elements.last().unwrap().defaulted);
}

#[test]
fn erased_wrappers_are_peeled_only_on_default_values() {
    let patterns = facts("function f({a = (({} as object) satisfies object)!, b = <unknown>[], c = (seed as object), d = 1}: T = ({} as T)!) {}", Language::TypeScript);
    assert_eq!(admit(&patterns[0]), ["a", "b", "d"]);
    let patterns = facts("function f({a}: T = ({} satisfies T)!) {}", Language::Tsx);
    assert_eq!(admit(&patterns[0]), ["a"]);
}

#[test]
fn legacy_alias_owns_earlier_formal_default() {
    let patterns = js("function f(input, {a: local} = input, {b}) {}");
    assert!(patterns[0].alias_skipped);
    assert!(patterns[0].elements.is_empty());
    assert_eq!(patterns[0].position, 1);
    assert_eq!(admit(&patterns[1]), ["b"]);
}

#[test]
fn legacy_alias_owns_outer_variable_but_not_wrapped_or_forward_default() {
    let patterns =
        js("const seed = {}; function f({a} = seed, {b} = (seed), {c} = forward, forward) {}");
    assert!(patterns[0].alias_skipped);
    assert!(patterns[0].elements.is_empty());
    for p in &patterns[1..] {
        assert!(!p.alias_skipped);
        assert_eq!(p.elements.len(), 1);
        assert!(admit(p).is_empty());
    }
}

#[test]
fn bound_names_in_excluded_branches_still_compete() {
    let patterns = js("function f({a, b, rename: keep, k: {d} = effect()}, [a], {other: b, d, keep: different}, c) {}" );
    assert_eq!(admit(&patterns[0]), ["keep"]);
    assert_eq!(admit(&patterns[1]), ["different"]);
    let patterns = js("function f({a, b}, a) {}");
    assert_eq!(admit(&patterns[0]), ["b"]);
}

#[test]
fn escaped_bindings_and_recovery_preclude_new_facts() {
    let patterns = js(r"function f({a, independent}, [\u0061]) {}");
    assert_eq!(admit(&patterns[0]), ["independent"]);
    let patterns = js(r"function f({live, \u0071}, [\u007a]) { return live; }");
    assert_eq!(admit(&patterns[0]), ["live"]);
    let patterns = js(r"function f({live}, [\uD800]) {}");
    assert_eq!(patterns.len(), 1);
    assert!(admit(&patterns[0]).is_empty());
    assert!(js("function f({a}, b { return a; }").is_empty());
}

#[test]
fn root_array_and_rest_supply_no_object_target() {
    let patterns = js("function f([{x}], {keep}, ...{length}) {}");
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].position, 1);
    assert_eq!(admit(&patterns[0]), ["keep"]);
    assert!(js("const f = ordinary => ordinary;").is_empty());
}

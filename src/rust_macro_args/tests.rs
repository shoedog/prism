use super::*;
use crate::languages::Language;

/// Parse `src` (must contain exactly one top-level `macro_invocation` in
/// a function body) and return the parsed file plus that node.
fn parse_one_macro(src: &str) -> ParsedFile {
    ParsedFile::parse("scratch.rs", src, Language::Rust).expect("parse")
}

fn find_macro_invocation<'a>(node: Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "macro_invocation" {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_macro_invocation(child) {
            return Some(found);
        }
    }
    None
}

fn extract(src: &str) -> (ParsedFile, Vec<(String, Option<String>)>, MacroArgFacts) {
    let pf = parse_one_macro(src);
    // Leak-free indirection: re-run extract_calls with a fresh borrow
    // scope so callers can inspect owned (name, qualifier) pairs without
    // fighting the `CallSiteMeta<'a>` lifetime.
    let root = pf.tree.root_node();
    let macro_node = find_macro_invocation(root).expect("must contain a macro_invocation");
    let (sites, facts) = extract_calls(&pf, macro_node);
    let simplified: Vec<(String, Option<String>)> = sites
        .iter()
        .map(|s| (s.callee_name.clone(), s.qualifier.clone()))
        .collect();
    (pf, simplified, facts)
}

// ---- allowlist / last-segment ----

#[test]
fn last_segment_strips_leading_path() {
    assert_eq!(last_path_segment("std::assert"), "assert");
    assert_eq!(last_path_segment("assert"), "assert");
}

#[test]
fn allowlist_matches_on_last_segment() {
    assert!(is_transparent_arg_macro("assert"));
    assert!(is_transparent_arg_macro("std::assert"));
    assert!(!is_transparent_arg_macro("matches"));
    assert!(!is_transparent_arg_macro("stringify"));
    assert!(!is_transparent_arg_macro("quote"));
    assert!(!is_transparent_arg_macro("lazy_static"));
    assert!(!is_transparent_arg_macro("select")); // tokio::select!
}

// ---- punctuation-atom splitter (pure function; spec-review MAJOR) ----

#[test]
fn splitter_keeps_ne_as_one_atom() {
    assert_eq!(split_punct_atoms("!="), vec!["!="]);
}

#[test]
fn splitter_keeps_range_as_one_atom() {
    assert_eq!(split_punct_atoms(".."), vec![".."]);
}

#[test]
fn splitter_keeps_range_inclusive_as_one_atom() {
    assert_eq!(split_punct_atoms("..="), vec!["..="]);
}

#[test]
fn splitter_splits_question_dot_into_two_atoms() {
    // `?.` is not itself a Rust token; a merged run must split into `?`, `.`.
    assert_eq!(split_punct_atoms("?."), vec!["?", "."]);
}

#[test]
fn splitter_keeps_fat_arrow_as_one_atom() {
    assert_eq!(split_punct_atoms("=>"), vec!["=>"]);
}

#[test]
fn splitter_splits_turbofish_colons_from_angle() {
    // `::<` is not itself a Rust token; must split into `::`, `<`.
    assert_eq!(split_punct_atoms("::<"), vec!["::", "<"]);
}

#[test]
fn splitter_handles_bang_alone() {
    assert_eq!(split_punct_atoms("!"), vec!["!"]);
}

#[test]
fn splitter_handles_dot_alone() {
    assert_eq!(split_punct_atoms("."), vec!["."]);
}

// ---- extractor: free / qualified calls ----

#[test]
fn free_call_is_minted() {
    let (_pf, sites, facts) = extract("fn f() { assert!(check(x)); }");
    assert_eq!(sites, vec![("check".to_string(), None)]);
    assert_eq!(facts.calls_recorded, 1);
}

#[test]
fn qualified_path_call_is_minted_with_joined_name() {
    let (_pf, sites, facts) = extract("fn f() { assert_eq!(util::compute(1), 2); }");
    assert_eq!(sites, vec![("util::compute".to_string(), None)]);
    assert_eq!(facts.calls_recorded, 1);
}

#[test]
fn lowercase_final_segment_qualified_ctor_style_call_still_mints() {
    // T::new(1): only the FINAL segment is tested for the ctor guard.
    let (_pf, sites, facts) = extract("fn f() { assert!(T::new(1)); }");
    assert_eq!(sites, vec![("T::new".to_string(), None)]);
    assert_eq!(facts.calls_recorded, 1);
    assert_eq!(facts.ctor_skips, 0);
}

#[test]
fn method_call_after_paren_group_mints_both_unknown_receiver() {
    // f(x).is_none() — method on a call result: unknown receiver (None).
    let (_pf, sites, facts) = extract("fn f() { assert!(f(x).is_none()); }");
    assert_eq!(
        sites,
        vec![("f".to_string(), None), ("is_none".to_string(), None),]
    );
    assert_eq!(facts.calls_recorded, 2);
}

#[test]
fn method_call_with_identifier_receiver_derives_qualifier() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(v.contains(x)); }");
    assert_eq!(sites, vec![("contains".to_string(), Some("v".to_string()))]);
}

#[test]
fn vec_macro_mints_two_free_calls() {
    let (_pf, sites, facts) = extract("fn f() { vec![compute(1), compute(2)]; }");
    assert_eq!(
        sites,
        vec![("compute".to_string(), None), ("compute".to_string(), None),]
    );
    assert_eq!(facts.calls_recorded, 2);
}

#[test]
fn nested_transparent_macro_recurses_and_mints_inner_call() {
    let (_pf, sites, _facts) = extract(r#"fn f() { assert!(v.contains(&format!("{}", f(1)))); }"#);
    // v.contains(...) [method, qualifier "v"] and f(1) [free, nested 2
    // levels inside the transparent format! macro]. `format` itself is
    // never minted — it's the nested macro's own name, not a value call.
    assert!(sites.contains(&("contains".to_string(), Some("v".to_string()))));
    assert!(sites.contains(&("f".to_string(), None)));
    assert!(!sites.iter().any(|(name, _)| name == "format"));
}

#[test]
fn non_allowlisted_nested_macro_is_not_walked_but_outer_method_still_mints() {
    // assert!(stringify!(g()).len() > 0): g must NEVER mint (nested
    // stringify! is not allowlisted); len() must still mint (method call
    // on the stringify! result, outside its token-soup boundary).
    let (_pf, sites, facts) = extract("fn f() { assert!(stringify!(g()).len() > 0); }");
    assert!(!sites.iter().any(|(name, _)| name == "g"));
    assert!(sites.contains(&("len".to_string(), None)));
    assert_eq!(facts.skipped_macros, 1);
}

#[test]
fn matches_macro_is_fully_skipped() {
    let (_pf, sites, facts) = extract("fn f() { matches!(x, Some(y)); }");
    assert!(sites.is_empty());
    // Some(y) is a call-shaped token pattern -> counts toward the
    // allowlist-growth signal even though nothing is minted.
    assert_eq!(facts.skipped_macros, 1);
}

#[test]
fn nontransparent_undefined_macro_is_skipped_and_counted() {
    let (_pf, sites, facts) = extract("fn f() { my_macro!(check(x)); }");
    assert!(sites.is_empty());
    assert_eq!(facts.skipped_macros, 1);
}

#[test]
fn stringify_never_mints_the_call_inside_it() {
    let (_pf, sites, facts) = extract("fn f() { stringify!(check(x)); }");
    assert!(sites.is_empty());
    assert_eq!(facts.skipped_macros, 1);
}

#[test]
fn struct_literal_lookalike_is_not_a_call() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(Ident { a: 1 }.a == 1); }");
    assert!(sites.is_empty());
}

#[test]
fn keyword_guard_skips_move_as_callee() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(move(x)); }");
    assert!(sites.is_empty());
}

#[test]
fn keyword_guard_skips_dyn_as_callee() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(dyn(x)); }");
    assert!(sites.is_empty());
}

#[test]
fn uppercase_constructor_is_skipped_and_counted() {
    let (_pf, sites, facts) = extract("fn f() { assert!(Foo(1)); }");
    assert!(sites.is_empty());
    assert_eq!(facts.ctor_skips, 1);
}

#[test]
fn uppercase_constructor_inside_call_args_does_not_block_finding_nested_calls() {
    let (_pf, sites, facts) = extract("fn f() { assert!(Foo(check(1))); }");
    assert!(!sites.iter().any(|(name, _)| name == "Foo"));
    assert!(sites.contains(&("check".to_string(), None)));
    assert_eq!(facts.ctor_skips, 1);
    assert_eq!(facts.calls_recorded, 1);
}

#[test]
fn turbofish_call_is_deliberately_not_minted() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(f::<T>(x)); }");
    assert!(sites.is_empty());
}

#[test]
fn punctuation_run_ne_does_not_trigger_nested_macro() {
    // `a != b`: no identifier is followed by "!" + token_tree, so no
    // nested-macro (mis)detection and no mint (a, b are bare values).
    let (_pf, sites, _facts) = extract("fn f() { assert!(a != b); }");
    assert!(sites.is_empty());
}

#[test]
fn punctuation_run_range_does_not_trigger_method_call() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(a..b); }");
    assert!(sites.is_empty());
}

#[test]
fn punctuation_run_range_inclusive_does_not_trigger_method_call() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(a..=b); }");
    assert!(sites.is_empty());
}

#[test]
fn minted_sites_carry_call_kind_and_macro_arg_origin() {
    let pf = parse_one_macro("fn f() { assert!(check(x)); }");
    let root = pf.tree.root_node();
    let macro_node = find_macro_invocation(root).unwrap();
    let (sites, _facts) = extract_calls(&pf, macro_node);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].kind_override, Some(CallKind::Call));
    assert_eq!(sites[0].origin_override, Some(CallSiteOrigin::MacroArg));
    assert_eq!(sites[0].arg_count, None);
    assert!(!sites[0].arg_spread);
}

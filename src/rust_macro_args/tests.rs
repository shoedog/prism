use super::*;
use crate::languages::Language;
use std::collections::BTreeSet;

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
    extract_with_shadow(src, &BTreeSet::new())
}

fn extract_with_shadow(
    src: &str,
    shadow: &BTreeSet<String>,
) -> (ParsedFile, Vec<(String, Option<String>)>, MacroArgFacts) {
    let pf = parse_one_macro(src);
    // Leak-free indirection: re-run extract_calls with a fresh borrow
    // scope so callers can inspect owned (name, qualifier) pairs without
    // fighting the `CallSiteMeta<'a>` lifetime.
    let root = pf.tree.root_node();
    let macro_node = find_macro_invocation(root).expect("must contain a macro_invocation");
    let (sites, facts) = extract_calls(&pf, macro_node, shadow);
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
    let shadow = BTreeSet::new();
    assert!(is_transparent_arg_macro("assert", &shadow));
    assert!(is_transparent_arg_macro("std::assert", &shadow));
    assert!(!is_transparent_arg_macro("matches", &shadow));
    assert!(!is_transparent_arg_macro("stringify", &shadow));
    assert!(!is_transparent_arg_macro("quote", &shadow));
    assert!(!is_transparent_arg_macro("lazy_static", &shadow));
    assert!(!is_transparent_arg_macro("select", &shadow)); // tokio::select!
}

// ---- F1 BLOCKER: definition-aware transparency (shadow set + qualifier) ----

#[test]
fn shadow_set_blocks_transparency_for_shadowed_name() {
    // A user `macro_rules! assert` shares its name with the real std macro
    // but is NOT known to be argument-transparent -- the shadow set must
    // withhold transparency even though "assert" is in the allowlist.
    let shadow: BTreeSet<String> = ["assert".to_string()].into_iter().collect();
    assert!(!is_transparent_arg_macro("assert", &shadow));
}

#[test]
fn shadow_set_does_not_block_unrelated_names() {
    let shadow: BTreeSet<String> = ["assert".to_string()].into_iter().collect();
    assert!(is_transparent_arg_macro("vec", &shadow));
}

#[test]
fn qualified_by_std_core_alloc_is_still_transparent() {
    let shadow = BTreeSet::new();
    assert!(is_transparent_arg_macro("std::assert", &shadow));
    assert!(is_transparent_arg_macro("core::assert", &shadow));
    assert!(is_transparent_arg_macro("alloc::vec", &shadow));
}

#[test]
fn qualified_by_arbitrary_path_is_not_transparent() {
    // `my::assert!`/`crate::assert!` are a DIFFERENT macro than the real
    // `std::assert!` (name resolution is out of scope here) -- any
    // qualifier other than std/core/alloc is NOT transparent, regardless of
    // the last segment or the shadow set.
    let shadow = BTreeSet::new();
    assert!(!is_transparent_arg_macro("my::assert", &shadow));
    assert!(!is_transparent_arg_macro("crate::assert", &shadow));
}

#[test]
fn user_defined_vec_macro_in_another_file_mints_nothing() {
    // The shadow set is repo-wide: a `macro_rules! vec` defined in ANY
    // indexed file must withhold transparency for `vec![...]` anywhere,
    // even in a different file. Simulated here by constructing the shadow
    // set directly (the cross-file plumbing is exercised at the
    // `collect_macro_shadow_set`/populator level in separate tests).
    let shadow: BTreeSet<String> = ["vec".to_string()].into_iter().collect();
    let (_pf, sites, facts) = extract_with_shadow("fn f() { vec![check(1)]; }", &shadow);
    assert!(sites.is_empty());
    assert_eq!(facts.skipped_macros, 1);
}

#[test]
fn qualified_my_assert_mints_nothing() {
    let (_pf, sites, _facts) = extract("fn f() { my::assert!(check(x)); }");
    assert!(sites.is_empty());
}

#[test]
fn qualified_std_assert_still_mints() {
    let (_pf, sites, facts) = extract("fn f() { std::assert!(check(x)); }");
    assert_eq!(sites, vec![("check".to_string(), None)]);
    assert_eq!(facts.calls_recorded, 1);
}

#[test]
fn collect_macro_shadow_set_finds_macro_rules_across_files() {
    use std::collections::BTreeMap;
    let mut files = BTreeMap::new();
    files.insert(
        "a.rs".to_string(),
        ParsedFile::parse("a.rs", "fn f() {}\n", Language::Rust).unwrap(),
    );
    files.insert(
        "b.rs".to_string(),
        ParsedFile::parse("b.rs", "macro_rules! vec { () => {}; }\n", Language::Rust).unwrap(),
    );
    let shadow = collect_macro_shadow_set(&files);
    assert!(shadow.contains("vec"));
    assert_eq!(shadow.len(), 1);
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
    let (sites, _facts) = extract_calls(&pf, macro_node, &BTreeSet::new());
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].kind_override, Some(CallKind::Call));
    assert_eq!(sites[0].origin_override, Some(CallSiteOrigin::MacroArg));
    assert_eq!(sites[0].arg_count, None);
    assert!(!sites[0].arg_spread);
}

// ---- F2 MAJOR: qualifier derivation must not truncate chained receivers ----

#[test]
fn chained_receiver_method_call_mints_with_no_qualifier() {
    // a.b.c(x): `b` is itself a chained receiver, NOT a simple identifier
    // receiver -- deriving qualifier=Some("b") would let a local `b: B`
    // falsely resolve `.c()` via receiver recovery. Must be qualifier=None.
    let (_pf, sites, _facts) = extract("fn f() { assert!(a.b.c(x)); }");
    assert_eq!(sites, vec![("c".to_string(), None)]);
}

#[test]
fn simple_receiver_method_call_still_derives_qualifier() {
    let (_pf, sites, _facts) = extract("fn f() { assert!(x.m(1)); }");
    assert_eq!(sites, vec![("m".to_string(), Some("x".to_string()))]);
}

#[test]
fn method_call_on_call_result_still_has_no_qualifier() {
    let (_pf, sites, facts) = extract("fn f() { assert!(f(x).is_none()); }");
    assert_eq!(
        sites,
        vec![("f".to_string(), None), ("is_none".to_string(), None)]
    );
    assert_eq!(facts.calls_recorded, 2);
}

#[test]
fn simple_receiver_preceded_by_non_dot_token_still_derives_qualifier() {
    // check(a.m(x)): `a` is a simple receiver -- nothing but the opening
    // paren of check's own arg list precedes it, so qualifier=Some("a").
    let (_pf, sites, _facts) = extract("fn f() { assert!(check(a.m(x))); }");
    assert!(sites.contains(&("m".to_string(), Some("a".to_string()))));
}

//! Slice R unit rows: the value spans `js_ts_written_value_lines` hands the resolver (rule 2),
//! and `js_ts_text_alias_agrees`'s two conditions (rule 3). The site is the LAST `name(`.
use crate::ast::js_binding::JsBindingCache;
use crate::ast::js_binding_helper_tests::parse;

const ANY: [&str; 3] = ["a.js", "a.ts", "a.tsx"];

fn site_of<'a>(p: &'a crate::ast::ParsedFile, src: &str, name: &str) -> tree_sitter::Node<'a> {
    let at = src.rfind(&format!("{name}(")).expect("site");
    let site = p
        .tree
        .root_node()
        .descendant_for_byte_range(at, at + name.len())
        .unwrap();
    assert_eq!(site.kind(), "identifier");
    site
}

fn lines(path: &str, src: &str, name: &str) -> Vec<(usize, usize)> {
    let p = parse(path, src);
    let mut cache = JsBindingCache::default();
    p.js_ts_written_value_lines(site_of(&p, src, name), name, &mut cache)
}

#[test]
fn value_lines_cover_the_initializer_and_each_assigned_right_hand_side() {
    let rows: [(&str, &str, Vec<(usize, usize)>); 9] = [
        (
            "initializer",
            "function r(o) {\n  const h = o.pick;\n  return h();\n}\n",
            vec![(2, 2)],
        ),
        (
            "multi-line initializer",
            "function r() {\n  const h = wrap(() => {\n    return 1;\n  });\n  return h();\n}\n",
            vec![(2, 4)],
        ),
        (
            "bare let has no value",
            "function r() {\n  let h;\n  return h();\n}\n",
            vec![],
        ),
        (
            "later assignment",
            "function r() {\n  let h;\n  h = () => {\n    return 1;\n  };\n  return h();\n}\n",
            vec![(3, 5)],
        ),
        (
            "initializer plus reassignment, sorted",
            "function r(o) {\n  let h = o.a;\n  if (o.b) h = o.c;\n  return h();\n}\n",
            vec![(2, 2), (3, 3)],
        ),
        (
            "destructured declarator: its source expression",
            "function r(o) {\n  const { h } = o;\n  return h();\n}\n",
            vec![(2, 2)],
        ),
        (
            "destructuring assignment",
            "function r(o) {\n  let h;\n  ({ h } = o.pick);\n  return h();\n}\n",
            vec![(3, 3)],
        ),
        (
            "a function declaration is its own value",
            "function h() {\n  return 0;\n}\nh = wrap(h);\nh();\n",
            vec![(1, 3), (4, 4)],
        ),
        (
            "an assignment to a shadowing binding elsewhere is not a write to this one",
            "function r(o) {\n  let h = o.a;\n  function q() {\n    let h = 1;\n    h = o.z;\n  }\n  return h();\n}\n",
            vec![(2, 2)],
        ),
    ];
    for path in ANY {
        for (id, src, want) in &rows {
            assert_eq!(&lines(path, src, "h"), want, "{id} {path}");
        }
    }
}

#[test]
fn value_lines_are_empty_when_the_walk_does_not_end_at_a_plain_scope() {
    // Unbound: nothing to hand over.
    assert!(lines("a.js", "function r() {\n  return h();\n}\n", "h").is_empty());
    // A parameter without default has no visible value.
    assert!(lines("a.js", "function r(h) {\n  return h();\n}\n", "h").is_empty());
}

fn agrees(path: &str, src: &str, alias: &str, target: &str) -> bool {
    let p = parse(path, src);
    let at = src.rfind(&format!("{alias}(")).expect("site");
    let function_start = src.find("function r").unwrap_or(0);
    let mut cache = JsBindingCache::default();
    p.js_ts_text_alias_agrees(
        function_start,
        at,
        at + alias.len() + 3,
        alias,
        target,
        &mut cache,
    )
}

#[test]
fn rule3_text_alias_needs_one_match_bound_alike_on_both_names() {
    let rows: [(&str, &str, bool); 6] = [
        (
            "same-file function, let alias",
            "function f() {}\nfunction r() {\n  let h = f;\n  return h(1);\n}\n",
            true,
        ),
        (
            "import, assignment",
            "import { f } from './m';\nfunction r() {\n  let h;\n  h = f;\n  return h(1);\n}\n",
            true,
        ),
        (
            "right-hand name is the caller's parameter",
            "function f() {}\nfunction r(f) {\n  let h = f;\n  return h(1);\n}\n",
            false,
        ),
        (
            "right-hand name holds a non-function",
            "const f = 1;\nfunction r() {\n  let h = f;\n  return h(1);\n}\n",
            false,
        ),
        (
            "alias rebound by a catch parameter",
            "function f() {}\nfunction r() {\n  let h = f;\n  try {\n    throw 1;\n  } catch (h) {\n    return h(1);\n  }\n}\n",
            false,
        ),
        (
            "alias declared in a sibling block",
            "function f() {}\nfunction r(h, c) {\n  if (c) {\n    let h = f;\n  }\n  return h(1);\n}\n",
            false,
        ),
    ];
    for path in ANY {
        for (id, src, want) in &rows {
            assert_eq!(agrees(path, src, "h", "f"), *want, "{id} {path}");
        }
    }
}

#[test]
fn rule3_two_matches_or_a_comment_only_match_agree_with_nothing() {
    // Two structural matches: the scan itself fails closed on a second assignment, and the
    // rule never picks one.
    let two = "function f() {}\nfunction r() {\n  let h = f;\n  h = f;\n  return h(1);\n}\n";
    assert!(!agrees("a.js", two, "h", "f"));
    // The text scan would match a comment line; there is no declaration to agree with.
    let comment = "function f() {}\nfunction r() {\n  // h = f\n  return h(1);\n}\n";
    assert!(!agrees("a.js", comment, "h", "f"));
}

#[test]
fn rule3_does_not_apply_to_a_callee_that_is_not_a_bare_identifier() {
    let src = "function f() {}\nfunction r(o) {\n  let h = f;\n  return o.h(1);\n}\n";
    let p = parse("a.js", src);
    let at = src.rfind("o.h(").unwrap();
    let mut cache = JsBindingCache::default();
    let end = src.rfind("o.h(1)").unwrap() + "o.h(1)".len();
    assert!(p.js_ts_text_alias_agrees(0, at, end, "h", "f", &mut cache));
}

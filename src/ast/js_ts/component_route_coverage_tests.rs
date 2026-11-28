//! Refusal-branch checks for the component route (lane 3 S2b, codecov patch). Each refusal row is
//! paired with a control that differs only in the guarded fact and proves the route.

use super::component_route::{js_ts_component_route, Route, SiteKind};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

fn visit<'a>(node: Node<'a>, predicate: &impl Fn(Node<'a>) -> bool) -> Option<Node<'a>> {
    if predicate(node) {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find_map(|child| visit(child, predicate));
    found
}

fn parse(name: &str, language: Language, source: &str) -> ParsedFile {
    let file = ParsedFile::parse(name, source, language).unwrap();
    assert!(!file.tree.root_node().has_error(), "{source}");
    file
}

fn marked_site<'a>(file: &'a ParsedFile, kind: SiteKind) -> Node<'a> {
    visit(file.tree.root_node(), &|node| {
        let text = file.node_text(&node);
        match kind {
            SiteKind::Literal => node.kind() == "call_expression" && text.ends_with("({a: x})"),
            SiteKind::Jsx => node.kind() == "jsx_self_closing_element" && text.ends_with("a={x}/>"),
        }
    })
    .expect("marked evaluated site")
}

/// The lexical site is in `caller`; the selected callable is the only one with a pattern formal.
fn same_file(name: &str, language: Language, source: &str, kind: SiteKind) -> Route {
    let file = parse(name, language, source);
    let candidates: Vec<_> = file
        .all_functions()
        .into_iter()
        .filter(|node| !file.js_ts_pattern_params(node).is_empty())
        .collect();
    assert_eq!(candidates.len(), 1, "one selected pattern body: {source}");
    let site = marked_site(&file, kind);
    js_ts_component_route(kind, &file, site, &file, candidates[0], false)
}

fn tsx(source: &str, kind: SiteKind) -> Route {
    same_file("route.tsx", Language::Tsx, source, kind)
}

/// Rows of `(refused source, route, control source)`; the control must be `Direct`.
fn assert_rows(language: Language, kind: SiteKind, rows: &[(&str, Route, &str)]) {
    let mut failures = Vec::new();
    let name = if language == Language::JavaScript {
        "route.js"
    } else {
        "route.tsx"
    };
    for (refused, want, control) in rows {
        let got = same_file(name, language, refused, kind);
        if got != *want {
            failures.push(format!("refusal {refused}: wanted {want:?}, got {got:?}"));
        }
        let kept = same_file(name, language, control, kind);
        if kept != Route::Direct {
            failures.push(format!("control {control}: {kept:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const APP: &str = "function App(x) { return D({a: x}); }";

#[test]
fn route_refuses_a_site_whose_syntax_does_not_match_its_kind() {
    let source =
        "function D({a}) { return a; }\nfunction App(x) { D({a: x}); return <D a={x}/>; }\n";
    let file = parse("route.tsx", Language::Tsx, source);
    let selected = file
        .all_functions()
        .into_iter()
        .find(|node| !file.js_ts_pattern_params(node).is_empty())
        .unwrap();
    let call = marked_site(&file, SiteKind::Literal);
    let element = marked_site(&file, SiteKind::Jsx);
    let route = |kind, site| js_ts_component_route(kind, &file, site, &file, selected, false);
    assert_eq!(route(SiteKind::Literal, call), Route::Direct);
    assert_eq!(route(SiteKind::Jsx, element), Route::Direct);
    assert_eq!(route(SiteKind::Jsx, call), Route::UnprovedIdentity);
    assert_eq!(route(SiteKind::Literal, element), Route::UnprovedIdentity);
    // A fragment opener has no name to prove.
    let fragment = parse(
        "frag.tsx",
        Language::Tsx,
        "function D({a}) { return a; }\nconst view = <></>;\n",
    );
    let opener = visit(fragment.tree.root_node(), &|node| {
        node.kind() == "jsx_opening_element"
    })
    .expect("fragment opener");
    assert!(opener.child_by_field_name("name").is_none());
    let selected = fragment.all_functions()[0];
    assert_eq!(
        js_ts_component_route(SiteKind::Jsx, &fragment, opener, &fragment, selected, false),
        Route::UnprovedIdentity
    );
}

#[test]
fn route_refuses_declarations_that_are_not_one_selected_implementation() {
    let other = "function Other({a}) { return a; }";
    // Overload signatures alone prove nothing; with the implementation selected they do.
    assert_rows(
        Language::Tsx,
        SiteKind::Literal,
        &[
            (
                &format!("function D(p: unknown): unknown;\n{other}\n{APP}"),
                Route::UnprovedIdentity,
                &format!(
                    "function D(p: unknown): unknown;\nfunction D({{a}}) {{ return a; }}\n{APP}"
                ),
            ),
            // An ambient overload signature declares the name but has no implementation to select.
            (
                &format!("declare function D(p: unknown): unknown;\n{other}\n{APP}"),
                Route::UnprovedIdentity,
                &format!("function D({{a}}) {{ return a; }}\n{APP}"),
            ),
            // A second runtime declaration of the name leaves no unique callee.
            (
                &format!("function D({{a}}) {{ return a; }}\nfunction D(p) {{ return p; }}\n{APP}"),
                Route::UnprovedIdentity,
                &format!("function D({{a}}) {{ return a; }}\n{APP}"),
            ),
            // The declaration found is not the selected callable.
            (
                &format!("function D(p) {{ return p; }}\n{other}\n{APP}"),
                Route::UnprovedIdentity,
                &format!("function D({{a}}) {{ return a; }}\n{APP}"),
            ),
            // A later write leaves the binding non-callable.
            (
                &format!("function D({{a}}) {{ return a; }}\nD = other;\n{APP}"),
                Route::UnprovedIdentity,
                &format!("function D({{a}}) {{ return a; }}\n{APP}"),
            ),
            // A declarator without an initializer carries no value to follow.
            (
                &format!("let D;\n{other}\n{APP}"),
                Route::UnprovedIdentity,
                &format!("const D = ({{a}}) => a;\n{APP}"),
            ),
        ],
    );
}

#[test]
fn route_refuses_wrapped_private_self_unless_the_result_binding_proves() {
    let react = "import {memo, useCallback as cb} from 'react';\n";
    let site = "return Self({a: x});";
    let literal = |declaration: &str| tsx(&format!("{react}{declaration}"), SiteKind::Literal);
    // A memo result never routes a literal; a hook result does.
    assert_eq!(
        literal(&format!(
            "const D = memo(function Self({{a}}) {{ {site} }});"
        )),
        Route::UnknownWrapper
    );
    assert_eq!(
        literal(&format!(
            "const D = cb(function Self({{a}}) {{ {site} }}, []);"
        )),
        Route::LocalHookOperand
    );
    // The wrapped call must be the initializer of a declarator.
    assert_eq!(
        literal(&format!("wrap(function Self({{a}}) {{ {site} }}, []);")),
        Route::UnknownWrapper
    );
    // Control for the shapes below: the plain named expression proves directly.
    assert_eq!(
        literal(&format!("const D = function Self({{a}}) {{ {site} }};")),
        Route::Direct
    );
    // The result must be one plain identifier, declared once, found by the walk.
    assert_eq!(
        literal(&format!(
            "const {{D}} = cb(function Self({{a}}) {{ {site} }}, []);"
        )),
        Route::UnknownWrapper
    );
    assert_eq!(
        literal(&format!(
            "var D = cb(function Self({{a}}) {{ {site} }}, []);\nvar D = 2;"
        )),
        Route::UnknownWrapper
    );
}

#[test]
fn route_jsx_render_operand_needs_a_program_level_declaration() {
    let react = "import {memo} from 'react';\n";
    let app = "function App(x) { return <D a={x}/>; }";
    let jsx = |declaration: &str| tsx(&format!("{react}{declaration}\n{app}"), SiteKind::Jsx);
    assert_eq!(jsx("const D = memo(({a}) => a);"), Route::JsxRenderOperand);
    // An exported declaration is still at program level.
    assert_eq!(
        jsx("export const D = memo(({a}) => a);"),
        Route::JsxRenderOperand
    );
    // The wrapper call is still found through transparent wrappers around the operand, but the
    // first argument is then not the selected callable itself: refused as an unknown wrapper.
    assert_eq!(
        jsx("const D = memo((({a}) => a) as any);"),
        Route::UnknownWrapper
    );
    assert_eq!(jsx("const D = memo((({a}) => a)!);"), Route::UnknownWrapper);
    // A declaration inside a function is not at program level.
    assert_eq!(
        tsx(
            &format!("{react}function Outer() {{ const D = memo(({{a}}) => a); {app} }}"),
            SiteKind::Jsx
        ),
        Route::UnknownWrapper
    );
    // A plain stored arrow is not a wrapper result at all.
    assert_eq!(jsx("const D = ({a}) => a;"), Route::Direct);
}

#[test]
fn route_refuses_an_annex_b_block_function_name() {
    // Sloppy block-level function declarations are Annex B names: unproved.
    assert_rows(
        Language::JavaScript,
        SiteKind::Literal,
        &[(
            "if (c) { function D({a}) { return a; } }\nfunction App(x) { return D({a: x}); }",
            Route::UnprovedIdentity,
            "function D({a}) { return a; }\nfunction App(x) { return D({a: x}); }",
        )],
    );
}

/// The lexical site is in `caller.tsx`; the selected callable is the only pattern-formal body of
/// `target.tsx`, with the export proof given by the test.
fn cross_file(caller: &str, target: &str, kind: SiteKind, export_proved: bool) -> Route {
    let from = parse("caller.tsx", Language::Tsx, caller);
    let to = parse("target.tsx", Language::Tsx, target);
    let candidates: Vec<_> = to
        .all_functions()
        .into_iter()
        .filter(|node| !to.js_ts_pattern_params(node).is_empty())
        .collect();
    assert_eq!(candidates.len(), 1, "one selected pattern body: {target}");
    let site = marked_site(&from, kind);
    js_ts_component_route(kind, &from, site, &to, candidates[0], export_proved)
}

const IMPORT_D: &str = "import {D} from './target';\n";
const CALL_D: &str = "function App(x) { return D({a: x}); }";
const VIEW_D: &str = "function App(x) { return <D a={x}/>; }";
const REACT: &str = "import {memo, useCallback as cb} from 'react';\n";

#[test]
fn route_checks_the_wrapper_in_the_target_file_of_an_imported_site() {
    let caller = |site| format!("{IMPORT_D}{site}");
    let target = |declaration: &str| format!("{REACT}{declaration}");
    let route = |declaration: &str, site, kind, proved| {
        cross_file(&caller(site), &target(declaration), kind, proved)
    };
    // Control: an unwrapped export proves, and only with the export proof.
    assert_eq!(
        route(
            "export const D = ({a}) => a;",
            CALL_D,
            SiteKind::Literal,
            true
        ),
        Route::Direct
    );
    assert_eq!(
        route(
            "export const D = ({a}) => a;",
            CALL_D,
            SiteKind::Literal,
            false
        ),
        Route::UnprovedIdentity
    );
    // The same operand under a wrapper call is judged by that call in the target file.
    assert_eq!(
        route(
            "export const D = memo(({a}) => a);",
            CALL_D,
            SiteKind::Literal,
            true
        ),
        Route::UnknownWrapper
    );
    assert_eq!(
        route(
            "export const D = memo(({a}) => a);",
            VIEW_D,
            SiteKind::Jsx,
            true
        ),
        Route::JsxRenderOperand
    );
    assert_eq!(
        route(
            "export const D = cb(({a}) => a, []);",
            CALL_D,
            SiteKind::Literal,
            true
        ),
        Route::LocalHookOperand
    );
    assert_eq!(
        route(
            "export const D = cb(({a}) => a, []);",
            VIEW_D,
            SiteKind::Jsx,
            true
        ),
        Route::UnknownWrapper
    );
    // A wrapped operand that is not a declarator initializer is refused.
    let default = format!("{REACT}export default memo(({{a}}) => a);");
    assert_eq!(
        cross_file(
            &format!("import D from './target';\n{CALL_D}"),
            &default,
            SiteKind::Literal,
            true
        ),
        Route::UnknownWrapper
    );
    assert_eq!(
        cross_file(
            &format!("import D from './target';\n{CALL_D}"),
            "export default ({a}) => a;",
            SiteKind::Literal,
            true
        ),
        Route::Direct
    );
}

#[test]
fn route_member_sites_need_an_import_namespace_qualifier() {
    let target = "export function D({a}) { return a; }";
    let route = |caller: &str, proved| cross_file(caller, target, SiteKind::Literal, proved);
    let namespace = "import * as ns from './target';\nfunction App(x) { return ns.D({a: x}); }";
    assert_eq!(route(namespace, true), Route::Direct);
    // Without the export proof the member form proves nothing.
    assert_eq!(route(namespace, false), Route::UnprovedIdentity);
    // A parameter, a local, or a default import is no namespace import.
    for caller in [
        "function App(ns, x) { return ns.D({a: x}); }",
        "const ns = make();\nfunction App(x) { return ns.D({a: x}); }",
        "import ns from './target';\nfunction App(x) { return ns.D({a: x}); }",
    ] {
        assert_eq!(route(caller, true), Route::UnprovedIdentity, "{caller}");
    }
}

#[test]
fn route_refuses_a_hook_result_declared_under_with() {
    let hook = "import {useCallback as cb} from 'react';\n";
    let route = |body: &str| {
        same_file(
            "route.js",
            Language::JavaScript,
            &format!("{hook}{body}"),
            SiteKind::Literal,
        )
    };
    let wrapped = "cb(function Self({a}) { return Self({a: x}); }, [])";
    assert_eq!(
        route(&format!("const D = {wrapped};")),
        Route::LocalHookOperand
    );
    // A `var` hoists past the `with` block, so the walk leaves through the with body: the result
    // binding is then not provable and the wrapper is refused. The later callable check refuses
    // a `var` result too, so this row pins the route's answer, not that line in isolation.
    assert_eq!(
        route(&format!("with (o) {{ var D = {wrapped}; }}")),
        Route::UnknownWrapper
    );
}

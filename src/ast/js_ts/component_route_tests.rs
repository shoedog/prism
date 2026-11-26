//! Direct proof tests inject the selected callable without changing resolution.

use super::component_route::{js_ts_component_route, wrapper_census_counts, Route, SiteKind};
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

fn classify(declarations: &str, caller: &str, kind: SiteKind) -> Route {
    let source = format!("{declarations}\n{caller}\n");
    let file = ParsedFile::parse("route.tsx", &source, Language::Tsx).unwrap();
    assert!(!file.tree.root_node().has_error(), "{source}");
    let candidates: Vec<_> = file
        .all_functions()
        .into_iter()
        .filter(|node| !file.js_ts_pattern_params(node).is_empty())
        .collect();
    assert_eq!(
        candidates.len(),
        1,
        "one independently selected pattern body"
    );
    let site = visit(file.tree.root_node(), &|node| {
        let text = file.node_text(&node);
        match kind {
            SiteKind::Literal => node.kind() == "call_expression" && text.ends_with("({a: x})"),
            SiteKind::Jsx => node.kind() == "jsx_self_closing_element" && text.ends_with("a={x}/>"),
        }
    })
    .expect("marked evaluated site");
    js_ts_component_route(kind, &file, site, &file, candidates[0], false)
}

#[test]
fn component_route_direct_hoisted_deferred_alias_and_private_self() {
    let mut failures = Vec::new();
    for (declarations, caller) in [
        (
            "function D({a}) { return a; }",
            "function App(x) { return D({a: x}); }",
        ),
        (
            "function D({a}) { return a; } const Alias = D;",
            "function App(x) { return Alias({a: x}); }",
        ),
        (
            "function App(x) { return D({a: x}); }",
            "const D = ({a}) => a;",
        ),
        ("const D = function Self({a}) { return Self({a: x}); };", ""),
        (
            "function D(p: unknown): unknown; function D({a}) { return a; }",
            "function App(x) { return D({a: x}); }",
        ),
    ] {
        let actual = classify(declarations, caller, SiteKind::Literal);
        if actual != Route::Direct {
            failures.push((declarations, caller, actual));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn component_route_refuses_unproved_lexical_value() {
    for (declarations, caller) in [
        (
            "function D({a}) { return a; }",
            "function App(D, x) { return D({a: x}); }",
        ),
        (
            "function D({a}) { return a; }",
            "function App(x) { return Missing({a: x}); }",
        ),
        (
            "function D({a}) { return a; } let Alias = D; Alias = other;",
            "function App(x) { return Alias({a: x}); }",
        ),
        (
            "function D({a}) { return a; } const Alias = cond ? D : other;",
            "function App(x) { return Alias({a: x}); }",
        ),
        (
            "function D({a}) { return a; } const A = B; const B = A;",
            "function App(x) { return A({a: x}); }",
        ),
        (
            "function F() { function D({a}) { return a; } }",
            "function App(x) { return D({a: x}); }",
        ),
        ("const premature = D({a: x});", "const D = ({a}) => a;"),
        (
            "(function Run(x) { D({a: x}); })(1);",
            "const D = ({a}) => a;",
        ),
        ("((x) => D({a: x})).call(null, 1);", "const D = ({a}) => a;"),
        (
            "function D({a}) { return a; } const Pick = factory(D);",
            "function App(x) { return Pick({a: x}); }",
        ),
        (
            "function D({a}) { return a; } const Pick = require('./d');",
            "function App(x) { return Pick({a: x}); }",
        ),
        (
            "function D({a}) { return a; } const Pick = require('./d').D;",
            "function App(x) { return Pick({a: x}); }",
        ),
        (
            "import type {D} from './d'; function Other({a}) { return a; }",
            "function App(x) { return D({a: x}); }",
        ),
    ] {
        assert_eq!(
            classify(declarations, caller, SiteKind::Literal),
            Route::UnprovedIdentity,
            "{declarations}\n{caller}"
        );
    }
}

#[test]
fn component_route_refuses_unadmitted_inline_operands() {
    let mut failures = Vec::new();
    for wrap in [
        "withDefaults(({a}) => a)",
        "debounce(({a}) => a, 10)",
        "passThrough(({a}) => a)",
        "outer(inner(({a}) => a))",
        "memo(({a}) => a, compare)",
        "memo(other, ({a}) => a)",
        "lazy(({a}) => a)",
    ] {
        let declaration = format!("import {{memo, lazy}} from 'react'; const D = {wrap};");
        let actual = classify(
            &declaration,
            "function App(x) { return D({a: x}); }",
            SiteKind::Literal,
        );
        if actual != Route::UnknownWrapper {
            failures.push((wrap, actual));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn component_route_react_hook_and_render_are_site_kind_specific() {
    let hook = "import {useCallback as cb} from 'react';";
    let calls = [
        (
            hook,
            "function App(x) { const D = cb(({a}) => a, []); return D({a: x}); }",
            SiteKind::Literal,
            Route::LocalHookOperand,
        ),
        (
            hook,
            "function App(cb, x) { const D = cb(({a}) => a, []); return D({a: x}); }",
            SiteKind::Literal,
            Route::UnknownWrapper,
        ),
        (
            hook,
            "function App(x) { const D = cb(({a}) => a, []); return <D a={x}/>; }",
            SiteKind::Jsx,
            Route::UnknownWrapper,
        ),
        (
            "import {memo} from 'react'; const D = memo(({a}) => a);",
            "function App(x) { return <D a={x}/>; }",
            SiteKind::Jsx,
            Route::JsxRenderOperand,
        ),
        (
            "import {memo} from 'react'; const D = memo(({a}) => a);",
            "function App(x) { return D({a: x}); }",
            SiteKind::Literal,
            Route::UnknownWrapper,
        ),
    ];
    let failures: Vec<_> = calls
        .into_iter()
        .filter_map(|(decl, caller, kind, want)| {
            let got = classify(decl, caller, kind);
            (got != want).then_some((caller, kind, want, got))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn component_route_named_and_default_imports_require_selected_identity() {
    let exported = ParsedFile::parse(
        "target.tsx",
        "export default function D({a}) { return a; }\nexport { D };",
        Language::Tsx,
    )
    .unwrap();
    let impostor = ParsedFile::parse(
        "impostor.tsx",
        "export default function D({a}) { return a; }",
        Language::Tsx,
    )
    .unwrap();
    let mut failures = Vec::new();
    for syntax in [
        "import Pick from './target';",
        "import {D as Pick} from './target';",
    ] {
        let source = format!("{syntax}\nfunction App(x) {{ return Pick({{a: x}}); }}");
        let from = ParsedFile::parse("caller.tsx", &source, Language::Tsx).unwrap();
        let site = visit(from.tree.root_node(), &|node| {
            node.kind() == "call_expression"
        })
        .unwrap();
        for (target, proved, want) in [
            (&exported, true, Route::Direct),
            (&impostor, false, Route::UnprovedIdentity),
        ] {
            let candidates = target.all_functions();
            assert_eq!(candidates.len(), 1);
            let got = js_ts_component_route(
                SiteKind::Literal,
                &from,
                site,
                target,
                candidates[0],
                proved,
            );
            if got != want {
                failures.push((syntax, target.path.clone(), want, got));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn component_route_literal_wrapper_attempt_has_zero_f9_weight() {
    let literal = (SiteKind::Literal, true, Route::UnknownWrapper);
    let jsx_identity = (SiteKind::Jsx, true, Route::UnprovedIdentity);
    let jsx_wrapper = (SiteKind::Jsx, true, Route::UnknownWrapper);
    let not_exact = (SiteKind::Jsx, false, Route::UnknownWrapper);
    assert_eq!(wrapper_census_counts(&[literal, not_exact]), (0, 0));
    assert_eq!(
        wrapper_census_counts(&[literal, not_exact, jsx_identity]),
        (0, 1)
    );
    assert_eq!(
        wrapper_census_counts(&[literal, not_exact, jsx_identity, jsx_wrapper]),
        (1, 2)
    );
}

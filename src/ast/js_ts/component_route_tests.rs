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
    classify_in("route.tsx", Language::Tsx, declarations, caller, kind)
}

fn classify_in(
    name: &str,
    language: Language,
    declarations: &str,
    caller: &str,
    kind: SiteKind,
) -> Route {
    let source = format!("{declarations}\n{caller}\n");
    let file = ParsedFile::parse(name, &source, language).unwrap();
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
            "const keep = (function Later(x) { return D({a: x}); });",
            "const D = ({a}) => a;",
        ),
        (
            "function D({a}) { return a; } const Alias = D;",
            "function App(x) { return Alias({a: x}); }",
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
            "function F() { function D({a}) { return a; } }",
            "function App(x) { return D({a: x}); }",
        ),
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

const LITERAL: &str = "((x) => D({a: x}))";
const LATER_D: &str = "const D = ({a}) => a;";
const STORED: &str = "const stored = #F#;";
const RETURNED: &str = "function make() { return #F#; }";
const JSX_ATTRIBUTE: &str = "const view = <Widget render={#F#}/>;";

fn literal_route(template: &str, kind: SiteKind) -> Route {
    classify(&template.replace("#F#", LITERAL), LATER_D, kind)
}

/// Each refusal reports the timing reason and is paired with a deferred control
/// whose route proves `D` independently.
fn timing_failures(rows: &[(&str, &str)]) -> Vec<String> {
    let mut failures = Vec::new();
    for (prefix, control) in rows {
        let refused = literal_route(prefix, SiteKind::Literal);
        if refused != Route::PreInitializerUnproved {
            failures.push(format!("refusal {prefix}: {refused:?}"));
        }
        let kept = literal_route(control, SiteKind::Literal);
        if kept != Route::Direct {
            failures.push(format!("control {control} for {prefix}: {kept:?}"));
        }
    }
    failures
}

fn assert_timing_rows(rows: &[(&str, &str)]) {
    let failures = timing_failures(rows);
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn component_route_timing_refuses_every_unlisted_delivery_form() {
    assert_timing_rows(&[
        ("(#F#.bind)(null)(1);", STORED),
        ("Reflect.apply(#F#, null, [1]);", RETURNED),
        ("Function.prototype.call.call(#F#, null, 1);", RETURNED),
        (
            "const bound = #F#.bind(null, 1); const alias = bound; alias();",
            STORED,
        ),
        ("[1].forEach(#F#);", JSX_ATTRIBUTE),
        ("new Promise(#F#);", STORED),
        ("#F#.call(null, 1);", STORED),
        ("#F#.apply(null, [1]);", RETURNED),
        ("#F#.bind(null)(1);", STORED),
        ("#F#.bind(null).call(null, 1);", STORED),
        ("const keep = #F#.bind(null);", STORED),
        ("const outer = () => #F#(1);", RETURNED),
        ("function App(x) { return D({a: x}); }", STORED),
    ]);
}

#[test]
fn component_route_timing_refuses_assignment_property_comma_and_statement_uses() {
    assert_timing_rows(&[
        ("let held; held = #F#;", STORED),
        ("const holder = { render: #F# };", STORED),
        ("const holder = { render() { return D({a: x}); } };", STORED),
        ("const pair = (0, #F#);", RETURNED),
        ("#F#;", STORED),
        ("(#F#)(1);", STORED),
        ("new (#F#)();", RETURNED),
        ("const [first] = [#F#];", STORED),
        ("const picked = cond ? #F# : null;", STORED),
    ]);
}

#[test]
fn component_route_timing_refuses_member_access_through_wrappers_and_brackets() {
    let mut failures = timing_failures(&[
        ("(#F# as any).bind(null)(1);", STORED),
        ("#F#!.call(null, 1);", STORED),
        ("(#F# satisfies unknown).apply(null, [1]);", RETURNED),
        ("#F#['call'](null, 1);", STORED),
        ("#F#['bind'](null);", STORED),
        ("const keep = #F#['bind'](null);", STORED),
        ("#F#?.call(null, 1);", RETURNED),
        ("#F#?.['apply'](null, [1]);", STORED),
        ("const keep = (#F#)?.bind(null);", STORED),
        ("const keep = ((#F# as any)!)[`bind`](null);", RETURNED),
        ("const member = #F#.name;", STORED),
    ]);
    let angle = |template: &str| {
        classify_in(
            "route.ts",
            Language::TypeScript,
            &template.replace("#F#", LITERAL),
            LATER_D,
            SiteKind::Literal,
        )
    };
    for (template, want) in [
        ("(<any>#F#).bind(null)(1);", Route::PreInitializerUnproved),
        ("const kept = <any>#F#;", Route::Direct),
        ("function make() { return <any>#F#; }", Route::Direct),
    ] {
        let got = angle(template);
        if got != want {
            failures.push(format!("angle {template}: wanted {want:?}, got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn component_route_timing_admits_exactly_the_three_delivery_forms() {
    let mut failures = Vec::new();
    for (name, prefix, kind) in [
        ("stored", "const later = #F#;", SiteKind::Literal),
        ("let", "let later = #F#;", SiteKind::Literal),
        ("var", "var later = #F#;", SiteKind::Literal),
        ("returned", RETURNED, SiteKind::Literal),
        (
            "returned in nested make",
            "function make(x) { return () => D({a: x}); }",
            SiteKind::Literal,
        ),
        (
            "concise arrow return",
            "const make = x => () => D({a: x});",
            SiteKind::Literal,
        ),
        (
            "concise arrow body",
            "const make = () => #F#;",
            SiteKind::Literal,
        ),
        ("jsx attribute", JSX_ATTRIBUTE, SiteKind::Literal),
        (
            "jsx attribute inside a call argument",
            "render(<Widget render={#F#}/>);",
            SiteKind::Literal,
        ),
        (
            "as wrapper",
            "const later = (#F# as any);",
            SiteKind::Literal,
        ),
        ("non-null wrapper", "const later = #F#!;", SiteKind::Literal),
        (
            "satisfies wrapper",
            "const later = (#F# satisfies unknown);",
            SiteKind::Literal,
        ),
        (
            "wrapped return",
            "function make() { return (#F# as any)!; }",
            SiteKind::Literal,
        ),
        (
            "wrapped jsx attribute",
            "const view = <Widget render={#F# as any}/>;",
            SiteKind::Literal,
        ),
        (
            "async arrow",
            "const later = async (x) => D({a: x});",
            SiteKind::Literal,
        ),
        (
            "function expression",
            "const later = function (x) { return D({a: x}); };",
            SiteKind::Literal,
        ),
        (
            "generator function expression",
            "const later = function* (x) { yield D({a: x}); };",
            SiteKind::Literal,
        ),
        (
            "parameter default of a stored arrow",
            "const later = (x, y = D({a: x})) => y;",
            SiteKind::Literal,
        ),
        (
            "jsx site in a jsx attribute arrow",
            "const view = <Widget render={x => <D a={x}/>}/>;",
            SiteKind::Jsx,
        ),
        (
            "jsx site in a stored arrow",
            "const later = x => <D a={x}/>;",
            SiteKind::Jsx,
        ),
    ] {
        let got = classify(&prefix.replace("#F#", LITERAL), LATER_D, kind);
        if got != Route::Direct {
            failures.push(format!("{name}: {got:?}"));
        }
    }
    let jsx_refused = classify("[1].forEach(x => <D a={x}/>);", LATER_D, SiteKind::Jsx);
    if jsx_refused != Route::PreInitializerUnproved {
        failures.push(format!("jsx call-argument refusal: {jsx_refused:?}"));
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn component_route_timing_checks_reads_inside_alias_initializers() {
    let alias_use = "function App(x) { return Alias({a: x}); }";
    let mut failures = Vec::new();
    let mut row = |label: String, got: Route, want: Route| {
        if got != want {
            failures.push(format!("{label}: wanted {want:?}, got {got:?}"));
        }
    };
    for (prefix, want) in [
        ("const Alias = D;", Route::PreInitializerUnproved),
        ("const Alias = (D);", Route::PreInitializerUnproved),
        ("const Alias = (y) => D(y);", Route::UnprovedIdentity),
    ] {
        row(
            prefix.to_owned(),
            classify(
                prefix,
                &format!("{LATER_D}\n{alias_use}"),
                SiteKind::Literal,
            ),
            want,
        );
    }
    // Every const cycle reads one binding before its declarator ends, so the
    // timing reason is reported before the cycle is reached.
    row(
        "const cycle".to_owned(),
        classify(
            "function D({a}) { return a; } const A = B; const B = A;",
            "function App(x) { return A({a: x}); }",
            SiteKind::Literal,
        ),
        Route::PreInitializerUnproved,
    );
    row(
        "control: alias declared after D".to_owned(),
        classify(
            &format!("{LATER_D}\nconst Alias = D;"),
            alias_use,
            SiteKind::Literal,
        ),
        Route::Direct,
    );
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn component_route_timing_gates_the_result_binding_of_a_proved_local_hook() {
    let hook = "import {useCallback as cb} from 'react';";
    let mut failures = Vec::new();
    for (caller, want) in [
        (
            "function App(x) { D({a: x}); const D = cb(({a}) => a, []); }",
            Route::PreInitializerUnproved,
        ),
        (
            "function App(x) { const D = cb(({a}) => a, []); D({a: x}); }",
            Route::LocalHookOperand,
        ),
        (
            "function App(x) { const later = () => D({a: x}); const D = cb(({a}) => a, []); }",
            Route::LocalHookOperand,
        ),
        (
            "function App(x) { (() => D({a: x}))(); const D = cb(({a}) => a, []); }",
            Route::PreInitializerUnproved,
        ),
    ] {
        let got = classify(hook, caller, SiteKind::Literal);
        if got != want {
            failures.push(format!("{caller}: wanted {want:?}, got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

use super::{certify, syntax, Certificate};
use crate::{ast::ParsedFile, languages::Language};

fn fixture(ext: &str, source: &str, _own: bool, _accepted: bool) -> Certificate {
    let language = if ext == "tsx" {
        Language::Tsx
    } else {
        Language::JavaScript
    };
    let parsed = ParsedFile::parse(&format!("fixture.{ext}"), source, language).unwrap();
    let operand = source.find("<D a={").unwrap() + "<D a={".len();
    let end = operand + source[operand..].find('}').unwrap();
    let root = syntax::operand_root(syntax::exact_node(&parsed, operand..end).unwrap()).unwrap();
    let owner = syntax::execution_owner(&parsed, root).unwrap();
    certify(&parsed, root, owner)
}

fn app(ext: &str, formals: &str, body: &str) -> Certificate {
    fixture(
        ext,
        &format!("function App({formals}) {{ {body} return <D a={{x}} />; }}"),
        false,
        true,
    )
}

#[track_caller]
fn assert_p2(cert: Certificate, failed: &[&str], form: Option<&str>, position: Option<&str>) {
    assert_eq!(cert.failed, failed, "{cert:?}");
    assert_eq!(cert.a2_form, form, "{cert:?}");
    assert_eq!(cert.b2_position.as_deref(), position, "{cert:?}");
    assert_eq!(cert.keeps(), failed.is_empty());
}

#[test]
fn p2_a2_keeps_const_and_formal_leaves() {
    for ext in ["jsx", "tsx"] {
        for (formals, body) in [
            ("", "const x = 7;"),
            ("", "const {x = 7} = get();"),
            ("", "const [x = 7] = get();"),
            ("", "const {p: {x = 7}} = get();"),
            ("", "const {...x} = get();"),
            ("", "const [a, ...x] = get();"),
            ("x", ""),
            ("x = 7", ""),
            ("{x}", ""),
            ("{p: x}", ""),
            ("{x = 7}", ""),
            ("{x = 7} = get()", ""),
            ("[...x]", ""),
            ("{...x}", ""),
        ] {
            assert_p2(
                app(ext, formals, &format!("{body} see(x);")),
                &[],
                None,
                None,
            );
        }
        assert_p2(
            fixture(ext, "const App = x => <D a={x} />;", false, true),
            &[],
            None,
            None,
        );
    }
    assert_p2(app("tsx", "x: number = 7", "see(x);"), &[], None, None);
    assert_p2(
        fixture(
            "jsx",
            "function App(undefined) { see(undefined); return <D a={undefined} />; }",
            false,
            true,
        ),
        &[],
        None,
        None,
    );
}

#[test]
fn p2_a2_refuses_ineligible_declaration_forms() {
    for ext in ["jsx", "tsx"] {
        for (formals, body, form) in [
            ("", "let x = 7;", "let"),
            ("", "var x = 7;", "var"),
            ("", "function x() {}", "function"),
            ("", "const f = function x() {};", "function"),
            ("", "class x {}", "class"),
            ("", "const C = class x {};", "class"),
            ("", "try {} catch ({x}) { see(x); }", "catch"),
            ("", "for (const {x} of values) { see(x); }", "loop_head"),
            ("", "for (let x = 7; ok;) { see(x); }", "loop_head"),
            (
                "",
                "do { const {x} = get(); see(x); } while (ok);",
                "const_in_loop",
            ),
            ("", "const x;", "const_without_initializer"),
            ("", "", "none_found"),
            ("...x", "", "param_non_leaf"),
            ("", "function nested(x) { see(x); }", "param_not_owner"),
        ] {
            assert_p2(
                app(ext, formals, &format!("{body} see(x);")),
                &["a2"],
                Some(form),
                None,
            );
        }
        assert_p2(
            app(ext, "x", r"class C { m() { const {p: \u0078} = get(); } }"),
            &["a2", "b2"],
            Some("multiple_declarations"),
            Some(if ext == "jsx" {
                "formal_parameters/<none>"
            } else {
                "required_parameter/pattern"
            }),
        );
    }
    assert_p2(
        app("tsx", "", "enum x { A }; see(x);"),
        &["a2"],
        Some("enum"),
        None,
    );
    for root in ["this", "super", "arguments"] {
        let code = format!("class C extends B {{ m() {{ return <D a={{{root}.p}} />; }} }}");
        let failed: &[&str] = if root == "arguments" {
            &["c", "a2"]
        } else {
            &["a2"]
        };
        assert_p2(
            fixture("jsx", &code, false, true),
            failed,
            Some("this_super_arguments"),
            None,
        );
    }
}

#[test]
fn p2_b2_ignores_non_variable_names_and_keeps_listed_reads() {
    for ext in ["jsx", "tsx"] {
        let source = "function App(title) {
            title: { break title; }
            class C { #title = 7; title() { return this.#title; } }
            see(<title title={title} ns:title={title} title-long={title} />);
            see(o.title, {title: 1}, {title}, (title).p.q, o[title]);
            o[title] = 1;
            return <D a={title} />;
        }";
        assert_p2(fixture(ext, source, false, true), &[], None, None);
    }
    assert_p2(
        app(
            "tsx",
            "x",
            "
            type x = typeof x.p;
            type Queried = typeof f(class x {});
            type Tuple = [x: number];
            type F = (x: number) => x is number;
            interface I extends x.Type { [x: string]: string; }
            declare module.exports: import(x++).T;
            const n: x.Type = 0;
            see<x.Type>();
        ",
        ),
        &[],
        None,
        None,
    );
}

#[test]
fn p2_b2_refuses_writes_and_unknown_reference_positions() {
    for ext in ["jsx", "tsx"] {
        for (body, position) in [
            ("x = 1;", "assignment_expression/left"),
            ("x++;", "update_expression/argument"),
            ("delete x.a;", "member_expression/object"),
            ("(x.a)[k].p = 1;", "member_expression/object"),
            ("[x] = v;", "array_pattern/<none>"),
            ("({x} = v);", "object_pattern/<none>"),
            ("for (x of v) {}", "for_in_statement/left"),
            ("for (see(x); ok;) {}", "arguments/<none>"),
            (r"\u0078++; delete x.a;", "update_expression/argument"),
            ("const o = {[x]: 1};", "computed_property_name/<none>"),
            ("see(<x.Component />);", "member_expression/object"),
        ] {
            assert_p2(app(ext, "x", body), &["b2"], None, Some(position));
        }
        assert_p2(
            fixture(
                ext,
                "function App(X) { see(<X />); return <D a={X} />; }",
                false,
                true,
            ),
            &["b2"],
            None,
            Some("jsx_self_closing_element/name"),
        );
    }
}

#[test]
fn clause_c_and_failure_order() {
    for body in [
        "function nested() { see(arguments); }",
        r"see(\u0065val);",
        "with (other) { see(1); }",
        "class C { m() { let y = ; } }",
        "see(o.eval);",
    ] {
        assert_p2(app("jsx", "x", body), &["c"], None, None);
    }
    assert_p2(app("jsx", "x", "see('eval arguments');"), &[], None, None);
    let source = "function App() { let x = get(); eval(''); x++; return <D a={x} />; }";
    assert_p2(
        fixture("jsx", source, false, true),
        &["c", "a2", "b2"],
        Some("let"),
        Some("update_expression/argument"),
    );
}

#[test]
fn astra_nine() {
    for (id, ext, source, failed, form, position) in [
        ("var iteration", "jsx",
            "function App() { for (var x of [7,0]) { if (x) see(x); else return <D a={x} />; } }",
            &["a2"][..], Some("loop_head"), None),
        ("var iteration, formal", "jsx",
            "function App(x) { for (x of [7,0]) { if (x) see(x); else return <D a={x} />; } }",
            &["b2"], None, Some("for_in_statement/left")),
        ("super receiver", "jsx",
            "class App extends Base { render() { see(this.p); super.p=0; return <D a={this.p} />; } }",
            &["a2"], Some("this_super_arguments"), None),
        ("super receiver, formal alias", "jsx",
            "class App extends Base { render(x) { see(x.p); super.p=0; return <D a={x.p} />; } }",
            &[], None, None),
        ("TS instantiation", "tsx",
            "function App(x: { <T>(): T; p: number }) { see(x.p); (x<string>).p=0; return <D a={x.p} />; }",
            &["b2"], None, Some("instantiation_expression/<none>")),
        ("escaped catch-var", "jsx",
            r"function App() { try { throw 7; } catch (x) { see(x); var \u0078=0; return <D a={x} />; } }",
            &["a2", "b2"], Some("multiple_declarations"), Some("catch_clause/parameter")),
        ("escaped catch-var, formal", "jsx",
            r"function App(x) { see(x); try { throw 7; } catch (e) { var \u0078=0; } return <D a={x} />; }",
            &["a2", "b2"], Some("multiple_declarations"), Some("formal_parameters/<none>")),
        ("first enum install", "tsx",
            "function App() { enum E { A=(see(E.B),1), B=2, C=(take(<D a={E.B} />),3) } }",
            &["a2"], Some("enum"), None),
        ("first enum install, const getter", "tsx",
            "function App() { const x = { get B() { return E.B; } }; enum E { A=(see(x.B),1), B=2, C=(take(<D a={x.B} />),3) } }",
            &[], None, None),
        ("computed key strictness", "jsx",
            r#"const o = { [(function App(x) { see(x); arguments[0]=0; return <D a={x} />; })(7)]() { "use strict"; } };"#,
            &["c"], None, None),
        ("formal to arguments", "jsx",
            "function App(x) { see(arguments); x=0; return <D a={arguments} />; }",
            &["c", "a2"], Some("this_super_arguments"), None),
        ("formal to arguments, const alias", "jsx",
            "function App(x) { const a=arguments; see(a[0]); x=0; return <D a={a[0]} />; }",
            &["c"], None, None),
        ("arguments to formal", "jsx",
            "function App(x) { see(x); arguments[0]=0; return <D a={x} />; }",
            &["c"], None, None),
        ("explicit arguments control", "jsx",
            "function App(x, arguments) { see(x); arguments[0]=0; return <D a={x} />; }",
            &["c"], None, None),
        ("formal-list identity", "jsx",
            "function App(x,y) { const a=x,b=y; see(a.p); b.p=0; return <D a={a.p} />; }",
            &[], None, None),
        ("private-name collision", "jsx",
            "class Outer { #p=7; render() { class Inner extends Outer { #p=7; set() { t.#p=0; } } const t=new Inner(); see(t.#p); t.set(); return <D a={t.#p} />; } }",
            &["b2"], None, Some("member_expression/object")),
    ] {
        let (before, operand) = source.split_once("<D a={").unwrap();
        let (operand, _) = operand.split_once('}').unwrap();
        assert!(before.contains(&format!("see({operand})")), "{id}");
        assert_p2(fixture(ext, source, false, true), failed, form, position);
    }
}

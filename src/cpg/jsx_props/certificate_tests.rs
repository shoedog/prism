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

// Round-5 design check (astra): a closed list must fail closed on every spelling it cannot read.
#[test]
fn an_undecodable_escape_fails_clause_c() {
    // Valid JavaScript escapes with more than six hex digits spell `x`, `eval` and `arguments`.
    for body in [
        r"see(x); \u{00000078} = 0;",
        r#"see(x); \u{00000065}val("x = 0");"#,
        r"see(x); \u{00000061}rguments[0] = 0;",
    ] {
        assert_p2(app("jsx", "x", body), &["c"], None, None);
    }
    // Control: the short escape decodes and is judged as a write (b2).
    assert_p2(
        app("jsx", "x", r"see(x); \u0078 = 0;"),
        &["b2"],
        None,
        Some("update_expression/argument")
            .filter(|_| false)
            .or(Some("assignment_expression/left")),
    );
}

#[test]
fn a_type_query_call_argument_is_executable_code() {
    // `typeof f(..)` parses as a type query, but its arguments run when the code is erased.
    let body = "function change() { type T = typeof Math.min/*\n*/(x = 0); } see(x); change();";
    assert_p2(
        app("tsx", "x", body),
        &["b2"],
        None,
        Some("assignment_expression/left"),
    );
    // Control: a type query without call arguments names no runtime value.
    assert_p2(
        app("tsx", "x", "type T = typeof x.p; see(x);"),
        &[],
        None,
        None,
    );
}

#[test]
fn a_result_position_follows_the_value_to_its_consumer() {
    for ext in ["jsx", "tsx"] {
        for (body, position) in [
            ("(0, x).p = 0;", "sequence_expression/<none>"),
            ("(c ? x : y).p = 0;", "ternary_expression/consequence"),
            ("(c ? y : x).p = 0;", "ternary_expression/alternative"),
            ("(x || y).p = 0;", "binary_expression/left"),
            ("(y ?? x).p = 0;", "binary_expression/right"),
            ("(c && x).p = 0;", "binary_expression/right"),
        ] {
            assert_p2(
                app(ext, "x", &format!("see(x); {body}")),
                &["b2"],
                None,
                Some(position),
            );
        }
        // Controls: the same result positions feeding a consumer that does not store, a discarded
        // sequence operand, a store through another root, and the condition operand.
        for body in [
            "see((0, x), c ? x : y, x || y, y ?? x, c && x);",
            "(x, y).p = 0;",
            "(0, y).p = 0;",
            "(c ? y : y).p = 0;",
            "(x ? y : y).p = 0;",
            "return_(x ? 1 : 2);",
        ] {
            assert_p2(app(ext, "x", &format!("see(x); {body}")), &[], None, None);
        }
    }
}

// Round-4 review: line terminators the grammars do not share with the language, `await`, and
// `as`/`satisfies` around a type query.
#[test]
fn line_terminator_corners_fail_clause_c() {
    for body in [
        "const g = () => { let v: number\r[x] = [0]; }; see(x); g();",
        "const g = () => { let v: number\u{2028}[x] = [0]; }; see(x); g();",
        "const g = () => { <!-- marker\r x = 0;\n }; see(x); g();",
    ] {
        let body = body.replace("\\r", "\r");
        assert_eq!(
            app("tsx", "x", &body).failed.first(),
            Some(&"c"),
            "{body:?}"
        );
    }
    // Controls: CRLF files and plain LF are not divergences.
    assert!(app("tsx", "x", "see(x);\r\n see(x);").keeps());
}

#[test]
fn await_and_wrapped_type_queries() {
    assert_p2(
        app("jsx", "x", "see(x); (await x).p = 0;"),
        &["b2"],
        None,
        Some("await_expression/<none>"),
    );
    assert_p2(app("jsx", "x", "see(await x);"), &[], None, None);
    for wrapper in ["as", "satisfies"] {
        let body = format!(
            "function change() {{ const n = Math.min {wrapper} typeof Math.min/*\n*/(x = 0); }} see(x); change();"
        );
        assert_p2(
            app("tsx", "x", &body),
            &["b2"],
            None,
            Some("assignment_expression/left"),
        );
    }
}

// Clause c, the structural rule (DESIGN 5.3.4): the grammar attaches a `[..]` or `(..)` that starts a
// new line to an arrow function or an `as`/`satisfies` expression, where the language ends the
// statement and the bracket starts a destructuring write. One rule at the choke point: every
// subscript, call, member and tagged template of the owner needs an object on a closed list.
const ARROW_THEN_WRITE: &str = "const g = () => {}\n[x] = [0]; see(x); g();";
const AS_THEN_WRITE: &str = "const v = [] as const\n[x] = [0]; see(x); v;";

#[test]
fn an_object_the_grammar_attaches_across_a_line_fails_clause_c() {
    for ext in ["jsx", "tsx"] {
        assert_eq!(
            app(ext, "x", ARROW_THEN_WRITE).failed.first(),
            Some(&"c"),
            "arrow {ext}"
        );
    }
    // (`satisfies T` followed by a line starting `[` is a parse error in the grammar, which clause c
    // already refuses: a type can take `[..]`, so that shape never reaches the structural rule.)
    for body in [AS_THEN_WRITE] {
        assert_eq!(app("tsx", "x", body).failed.first(), Some(&"c"), "{body:?}");
    }
    // The call form of the same attachment.
    assert_eq!(
        app("jsx", "x", "const g = () => {}\n(x); see(x);")
            .failed
            .first(),
        Some(&"c")
    );
    // A member and a tagged template on an arrow are the same kind of object.
    assert_eq!(
        app("jsx", "x", "const g = () => {}\n.p; see(x);")
            .failed
            .first(),
        Some(&"c")
    );
}

#[test]
fn a_parenthesized_object_is_the_language_reading_and_stays_certified() {
    assert!(app("jsx", "x", "const g = (() => {})\n[x]; see(x); g;").keeps());
    assert!(app("tsx", "x", "const v = ([] as const)\n[x]; see(x); v;").keeps());
    assert!(app(
        "tsx",
        "x",
        "const v = ([] satisfies object)\n[x]; see(x); v;"
    )
    .keeps());
    assert!(app("jsx", "x", "see(x); (() => {})[x]; (() => {})(x);").keeps());
}

#[test]
fn every_listed_object_kind_stays_certified() {
    // One row per kind on the closed list; each is a genuine subscript, call, member or template.
    for (kind, body) in [
        ("identifier", "see(x); a[x]; a(x); a.b;"),
        ("member_expression", "see(x); a.b[x]; a.b(x);"),
        ("subscript_expression", "see(x); a[0][x]; a[0](x); a[0].b;"),
        ("call_expression", "see(x); f()[x]; f()(x); f().b;"),
        ("parenthesized_expression", "see(x); (a)[x]; (a)(x); (a).b;"),
        ("this", "see(x); this[x]; this(x); this.b;"),
        ("string", "see(x); 's'[x]; 's'.length;"),
        ("template_string", "see(x); `t`[x]; `t`.length;"),
        ("number", "see(x); 1[x]; (1).b;"),
        ("array", "see(x); [1][x]; [1].length;"),
        ("object", "see(x); const o = {a: 1}[x]; const p = {a: 1}.a;"),
        ("regex", "see(x); /a/[x]; /a/.test(x);"),
        ("new_expression", "see(x); new A()[x]; new A().b;"),
        ("tagged template", "see(x); tag`t${x}`; a.tag`t`;"),
        ("undefined", "see(x); undefined.b; undefined[x];"),
        ("null, true, false", "see(x); null.b; true.b; false[x];"),
        ("meta_property", "see(x); new.target[x]; new.target.b;"),
    ] {
        assert!(app("jsx", "x", body).keeps(), "{kind}: {body:?}");
    }
    assert!(
        app("tsx", "x", "see(x); a![x]; a!(x); a!.b;").keeps(),
        "non_null_expression"
    );
    let module = "function App(x) { import.meta[x]; import.meta.env; import(x).then(); return <D a={x} />; }";
    assert!(
        fixture("jsx", module, false, true).keeps(),
        "import, import.meta"
    );
    let method = "class A extends B { m(x) { super.m(x); super[x]; return <D a={x} />; } }";
    assert!(fixture("jsx", method, false, true).keeps(), "super");
}

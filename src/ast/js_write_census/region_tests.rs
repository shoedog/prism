//! Round-1 fold rows: `delete` operands, evaluation boundaries by child position, `using`,
//! `Site.initialized`. Each source names its grammar explicitly.

use super::harness::{owners, Fx};
use super::walk::{census, Form, Refusal, Site};
use crate::languages::Language::{self, JavaScript as JS, Tsx, TypeScript as TS};

fn body(line: &str) -> String {
    format!("function f(a) {{\n  log();\n  {line}\n}}")
}

/// Non-parameter-formal sites of `a` (everything but the owner's own `a` declaration).
fn a_sites(fx: &Fx) -> Vec<Site> {
    fx.first_census()
        .sites
        .into_iter()
        .filter(|s| s.path.base == "a" && !(s.form == Form::Declare && s.region.own_params))
        .collect()
}

fn rows_in(lang: Language, line: &str) -> Vec<Site> {
    a_sites(&Fx::in_lang(&body(line), lang))
}

#[test]
fn delete_operand_children_are_walked_as_reads() {
    for lang in [JS, TS, Tsx] {
        let s = rows_in(lang, "delete g(a++);");
        assert!(
            s.len() == 1 && s[0].form == Form::ReadWrite,
            "{lang:?} call argument: {s:?}"
        );
        let s = rows_in(lang, "delete (a = 1);");
        assert!(
            s.len() == 1 && s[0].form == Form::Write,
            "{lang:?} parenthesized assignment: {s:?}"
        );
        let s = rows_in(lang, "delete (x, a = 1);");
        assert!(
            s.len() == 1 && s[0].form == Form::Write,
            "{lang:?} sequence operand: {s:?}"
        );
        let s = rows_in(lang, "delete g(() => { a = 1; });");
        assert!(
            s.len() == 1 && s[0].form == Form::Write && s[0].region.nested,
            "{lang:?} closure write: {s:?}"
        );
        let s = rows_in(lang, "delete o[a = 1];");
        assert!(
            s.len() == 1 && s[0].form == Form::Write,
            "{lang:?} index of a deleted member: {s:?}"
        );
        // Controls: a deleted member is a Delete site of its path; `delete a` writes nothing.
        let fx = Fx::in_lang(&body("delete o.x;"), lang);
        let o: Vec<_> = fx
            .first_census()
            .sites
            .into_iter()
            .filter(|s| s.path.base == "o")
            .collect();
        assert!(o.len() == 1 && o[0].form == Form::Delete, "{lang:?} {o:?}");
        assert!(
            rows_in(lang, "delete a;").is_empty(),
            "{lang:?}: delete of a name writes nothing"
        );
    }
}

fn nested_flags(lang: Language, src: &str) -> Vec<(bool, bool, bool)> {
    let fx = Fx::in_lang(&body(src), lang);
    a_sites(&fx)
        .iter()
        .map(|s| (s.region.nested, s.region.nested_params, s.region.own_params))
        .collect()
}

#[test]
fn method_keys_and_decorators_keep_the_enclosing_region() {
    let eager = vec![(false, false, false)];
    for lang in [JS, TS, Tsx] {
        assert_eq!(
            nested_flags(lang, "const C = class { [a++]() {} };"),
            eager,
            "{lang:?} class key"
        );
        assert_eq!(
            nested_flags(lang, "const o = { [a = 1]() {} };"),
            eager,
            "{lang:?} object key"
        );
        assert_eq!(
            nested_flags(lang, "const o = { get [a = 1]() { return 1; } };"),
            eager,
            "{lang:?} getter key"
        );
        assert_eq!(
            nested_flags(lang, "const o = { set [a = 1](v) {} };"),
            eager,
            "{lang:?} setter key"
        );
        // Controls: body and default are the method's region.
        assert_eq!(
            nested_flags(lang, "const C = class { m() { a++; } };"),
            vec![(true, false, false)],
            "{lang:?} body"
        );
        assert_eq!(
            nested_flags(lang, "const C = class { m(z = a++) {} };"),
            vec![(true, true, false)],
            "{lang:?} default"
        );
        // A key inside an already deferred callable stays deferred through its ancestor.
        assert_eq!(
            nested_flags(lang, "g(() => { const C = class { [a++]() {} }; });"),
            vec![(true, false, false)],
            "{lang:?}"
        );
    }
    // Decorator placement differs by grammar; the flags must not.
    assert_eq!(
        nested_flags(JS, "const C = class { @dec(a++) m() {} };"),
        eager,
        "JS method decorator"
    );
    assert_eq!(
        nested_flags(TS, "const C = class { @dec(a++) m() {} };"),
        eager,
        "TS class_body decorator"
    );
    assert_eq!(
        nested_flags(Tsx, "const C = class { @dec(a++) m() {} };"),
        eager,
        "TSX class_body decorator"
    );
}

#[test]
fn typescript_parameter_decorators_run_in_the_enclosing_scope() {
    for lang in [TS, Tsx] {
        // The outer `a` is written; the decorator is neither a parameter-region write nor made inner.
        let fx = Fx::in_lang(&body("class C { constructor(@dec(a = 1) a) {} }"), lang);
        let c = fx.first_census();
        let w = c
            .sites
            .iter()
            .find(|s| s.form == Form::Write && s.path.base == "a")
            .expect("decorator write is a site");
        assert!(
            !w.region.nested && !w.region.nested_params && !w.region.own_params,
            "{lang:?} {:?}",
            w.region
        );
        // The parameter `a` itself is declared in the constructor's parameter list.
        let p = c
            .sites
            .iter()
            .find(|s| s.form == Form::Declare && s.path.base == "a" && s.region.nested_params);
        assert!(p.is_some(), "{lang:?}");
        // Control: a true parameter default is a parameter-region write.
        assert_eq!(
            nested_flags(lang, "class C { constructor(z = (a = 1)) {} }"),
            vec![(true, true, false)],
            "{lang:?}"
        );
    }
}

#[test]
fn a_method_owner_does_not_walk_its_own_key_or_decorators() {
    for lang in [JS, TS] {
        let fx = Fx::in_lang("class C { [a = 1](a) { return a; } }", lang);
        let o = owners(&fx.parsed, &fx.t)[0];
        assert_eq!(o.kind(), "method_definition");
        let c = census(&fx.parsed, &fx.t, o);
        let a: Vec<_> = c.sites.iter().filter(|s| s.path.base == "a").collect();
        assert!(
            a.len() == 1 && a[0].form == Form::Declare,
            "{lang:?}: only the parameter: {a:?}"
        );
    }
}

#[test]
fn assignment_form_using_refuses_the_owner() {
    let src = "function f(a) {\n  using r = (a++, res());\n}";
    for lang in [TS, Tsx] {
        let c = Fx::in_lang(src, lang).first_census();
        assert_eq!(
            c.refusals,
            vec![Refusal::Target("using".into())],
            "{lang:?}"
        );
        assert!(c.sites.is_empty() || c.refused());
    }
    // JS 0.23.1 does not parse it at all (no callable owner is even found).
    assert!(Fx::in_lang(src, JS).parsed.tree.root_node().has_error());
    // Control: the word as a property name is not the keyword token (it is an identifier-like name
    // in `.using`; the TS grammars lex a bare `using` identifier as the keyword, so none exists).
    let c = Fx::in_lang("function f(a) {\n  o.using = 1;\n}", TS).first_census();
    assert!(
        c.refusals.is_empty()
            && c.sites
                .iter()
                .any(|s| s.path.render() == "o.using" && s.form == Form::Write),
        "{c:?}"
    );
}

#[test]
fn only_var_without_a_value_stores_nothing() {
    let init = |line: &str, base: &str| {
        let fx = Fx::in_lang(&body(line), JS);
        let s = fx
            .first_census()
            .sites
            .into_iter()
            .find(|s| s.path.base == base && s.form == Form::Declare)
            .unwrap();
        s.initialized
    };
    assert!(!init("var b;", "b"));
    assert!(init("let b;", "b"), "let b; initializes to undefined");
    assert!(init("var b = 1;", "b"));
    assert!(init("const b = 1;", "b"));
    assert!(init("for (var b of xs) g();", "b"));
}

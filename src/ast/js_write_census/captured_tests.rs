//! #660 S1c: which declarations scope a write in a region (DESIGN.md 6.2 item 1), one row per
//! declaration kind, positive and negative. A row is a region body (`a++` inside a nested callable)
//! and the source text of the declarations the census names as scoping that write.
use super::completion::completion;
use super::harness::{owners, Fx};
use crate::access_path::AccessPath;
use crate::languages::Language::{self, JavaScript as JS, Tsx, TypeScript as TS};

/// The text at the start of each declaration scoping each region write of `a`, per write.
fn scoping(lang: Language, src: &str) -> Vec<Vec<String>> {
    let fx = Fx::in_lang(src, lang);
    let owner = owners(&fx.parsed, &fx.t)[0];
    let a = AccessPath::simple("a");
    let done = completion(&fx.parsed, owner, &[(&a, 0, false)]);
    done.captured
        .iter()
        .filter(|c| c.path == a)
        .map(|c| {
            c.shadows
                .iter()
                .map(|d| {
                    let rest = &src[d.start..];
                    let end = rest
                        .find(|ch: char| !ch.is_alphanumeric() && ch != '_')
                        .unwrap_or(rest.len());
                    rest[..end].to_string()
                })
                .collect()
        })
        .collect()
}

/// `function f(a) { <line> }` with `line` holding the region.
fn row(line: &str) -> Vec<Vec<String>> {
    scoping(JS, &format!("function f(a) {{\n  {line}\n}}"))
}

fn scoped(line: &str) -> bool {
    let rows = row(line);
    assert_eq!(rows.len(), 1, "one region write in {line}: {rows:?}");
    !rows[0].is_empty()
}

#[test]
fn a_parameter_scopes_its_body_and_its_own_list() {
    assert!(scoped("g(function (a) { a++; });"));
    assert!(scoped("g((a) => { a++; });"));
    assert!(scoped("g(a => { a++; });"));
    assert!(scoped("g(({ a }) => { a++; });"));
    assert!(scoped("g(([a]) => { a++; });"));
    assert!(scoped("g((a, z = (a++)) => z);"));
    assert!(scoped("g((z = (a++), a) => z);"));
    assert!(scoped("o = { m(a) { a++; } };"));
    assert!(scoped("o = { set v(a) { a++; } };"));
}

#[test]
fn a_parameter_of_another_list_scopes_nothing() {
    // a nested callable's own list scopes its own default only; a sibling callable's parameter does not.
    assert!(!scoped("g((a) => 1, () => { a++; });"));
    assert!(!scoped("g((z = (a++)) => z, (a) => 1);"));
}

#[test]
fn a_body_declaration_never_scopes_a_parameter_default() {
    // a10, a11: the parameter environment precedes the body's var and function declarations.
    assert!(!scoped("g(function (z = (a++)) { var a; });"));
    assert!(!scoped("g(function (z = (a++)) { function a() {} });"));
    assert!(!scoped("g(function (z = (a++)) { let a; });"));
    assert!(!scoped("g((z = (a++)) => { var a; });"));
}

#[test]
fn a_var_scopes_the_nearest_callable_body() {
    assert!(scoped("g(() => { var a; a++; });"));
    assert!(scoped("g(() => { { var a; } a++; });"));
    assert!(scoped("g(() => { a++; var a; });"));
    assert!(scoped("g(() => { for (var a = 0;;) {} a++; });"));
    assert!(scoped("g(() => { for (var a of xs) {} a++; });"));
    assert!(scoped("g(() => { for (var a in o) {} a++; });"));
    // a var of a callable inside the region does not reach out of it
    assert!(!scoped("g(() => { h(() => { var a; }); a++; });"));
}

#[test]
fn a_lexical_declaration_scopes_its_block_head_or_switch_body() {
    assert!(scoped("g(() => { let a; a++; });"));
    assert!(scoped("g(() => { const a = 1; a++; });"));
    assert!(scoped("g(() => { { let a; a++; } });"));
    assert!(scoped("g(() => { for (let a = 0;;) { a++; } });"));
    assert!(scoped("g(() => { for (let a = 0;; a++) {} });"));
    assert!(scoped("g(() => { for (let a of xs) { a++; } });"));
    assert!(scoped("g(() => { for (const a in o) { a++; } });"));
    assert!(scoped("g(() => { switch (k) { case 1: let a; a++; } });"));
    assert!(scoped(
        "g(() => { switch (k) { case 1: let a; break; case 2: a++; } });"
    ));
    // a sibling block, a sibling statement's head, and a later block do not scope it
    assert!(!scoped("g(() => { { let a; } a++; });"));
    assert!(!scoped("g(() => { for (let a = 0;;) {} a++; });"));
    assert!(!scoped("g(() => { switch (k) { case 1: let a; } a++; });"));
    assert!(!scoped("g(() => { if (c) { let a; } else { a++; } });"));
}

#[test]
fn a_function_or_class_declaration_scopes_its_block() {
    assert!(scoped("g(() => { a++; function a() {} });"));
    assert!(scoped("g(() => { a++; function* a() {} });"));
    assert!(scoped("g(() => { l: function a() {} a++; });"));
    assert!(scoped("g(() => { { function a() {} a++; } });"));
    assert!(scoped(
        "g(() => { switch (k) { case 1: function a() {} a++; } });"
    ));
    assert!(scoped("g(() => { { class a {} a++; } });"));
    // a14: a block function scopes only its block, in both modes
    assert!(!scoped("g(() => { { function a() {} } a++; });"));
    assert!(!scoped("g(() => { { class a {} } a++; });"));
    assert!(!scoped("g(() => { if (c) function a() {} a++; });"));
}

#[test]
fn a_named_function_expression_scopes_its_body_only() {
    assert!(scoped("g(function a() { a++; });"));
    assert!(scoped("g(function* a() { a++; });"));
    assert!(!scoped("g(function a(z = (a++)) {});"));
    // a class expression's own name is not in the design's list
    assert!(!scoped("g(class a { m() { a++; } });"));
}

#[test]
fn a_catch_parameter_scopes_its_clause() {
    assert!(scoped("g(() => { try { h(); } catch (a) { a++; } });"));
    assert!(scoped("g(() => { try { h(); } catch ({ a }) { a++; } });"));
    assert!(!scoped("g(() => { try { h(); } catch (e) { a++; } });"));
    assert!(!scoped("g(() => { try { h(); } catch (a) {} a++; });"));
}

#[test]
fn a_declaration_in_an_enclosing_callable_scopes_a_nested_write() {
    let rows = row("g(() => { var a; h(() => { a++; }); });");
    assert_eq!(rows, vec![vec!["a".to_string()]]);
    let rows = row("g((a) => { h(() => { a++; }); });");
    assert_eq!(rows, vec![vec!["a".to_string()]]);
    // the owner's own declaration is eager: never a region declaration
    assert_eq!(row("g(() => { a++; });"), vec![Vec::<String>::new()]);
}

#[test]
fn a_field_initializer_is_a_region_and_its_arrow_is_another() {
    let rows = row("const C = class { x = (a++); };");
    assert_eq!(rows, vec![Vec::<String>::new()]);
    assert!(scoped("const C = class { x = (a) => { a++; }; };"));
    assert!(!scoped("const C = class { x = () => { a++; }; };"));
}

#[test]
fn typescript_parameter_properties_and_tsx_handlers() {
    let ts = scoping(
        TS,
        "function f(a: number) {\n  class C { constructor(public a = 1) { a++; } }\n}",
    );
    assert_eq!(ts, vec![vec!["a".to_string()]]);
    let ts = scoping(
        TS,
        "function f(a: number) {\n  class C { constructor(public x = (a++)) {} }\n}",
    );
    assert_eq!(ts, vec![Vec::<String>::new()]);
    let tsx = scoping(
        Tsx,
        "function f(a: number) {\n  return <div onClick={(a: number) => { a++; }} />;\n}",
    );
    assert_eq!(tsx, vec![vec!["a".to_string()]]);
}

#[test]
fn a_write_of_another_base_or_in_the_owner_is_not_captured() {
    // only the owner's recorded bases are listed, and only region writes
    assert!(row("b++; g(() => { b++; });").is_empty());
    assert!(row("a++;").is_empty());
    // `this` of a non-arrow callable is another object
    let fx = Fx::new(
        "function f() {\n  g(function () { this.x = 1; });\n  g(() => { this.x = 2; });\n}",
    );
    let owner = owners(&fx.parsed, &fx.t)[0];
    let this = AccessPath::simple("this");
    let done = completion(&fx.parsed, owner, &[(&this, 0, false)]);
    assert_eq!(done.captured.len(), 1, "{:?}", done.captured);
}

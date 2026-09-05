use prism::{
    ast::ParsedFile,
    call_graph::CallGraph,
    languages::Language,
    resolution::{ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;

fn check(source: &str, expected: bool, lang: Language) {
    let ext = if lang == Language::JavaScript {
        "js"
    } else {
        "ts"
    };
    let app = format!("app.{ext}");
    let owner = format!("client.{ext}");
    let decoy = format!("decoy.{ext}");
    let files = BTreeMap::from([
        (
            app.clone(),
            ParsedFile::parse(
                &app,
                &format!("import Client from './client'; {source}"),
                lang,
            )
            .unwrap(),
        ),
        (
            owner.clone(),
            ParsedFile::parse(
                &owner,
                "class Client { m = () => {}; } export default Client;",
                lang,
            )
            .unwrap(),
        ),
        (
            decoy.clone(),
            ParsedFile::parse(&decoy, "class Client { m() {} }", lang).unwrap(),
        ),
    ]);
    for cg in [
        CallGraph::build(&files),
        CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
    ] {
        let sites: Vec<_> = cg
            .calls
            .values()
            .flat_map(|s| s)
            .filter(|s| s.callee_name == "m")
            .collect();
        assert!(!sites.is_empty(), "zero sites: {source}");
        for site in sites {
            let exact: Vec<_> = cg
                .resolve_call_site(site)
                .into_iter()
                .filter(|e| e.confidence == ResolutionConfidence::Exact)
                .collect();
            assert_eq!(
                exact.len(),
                usize::from(expected),
                "{source}: {site:?} {exact:?}"
            );
            if expected {
                assert_eq!(exact[0].target.file, owner);
                assert_eq!(exact[0].kind, ResolutionKind::FieldTyped);
            }
        }
    }
}

#[test]
fn constructor_field_positive() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for source in [
            "class App { constructor() { this.client = new Client(); } run() { this.client.m(); } }",
            "class App { client; constructor() { this.client = new Client(this); } run = () => this.client.m(); }",
            "class App { constructor() { this.client = new Client(); this.client.m(); } }",
            "class App { constructor() { this.client = new Client(); } run() { const nested = () => this.client.m(); } }",
            "class App extends Base { client; constructor() { super(); this.client = new Client(); } run() { this.client.m(); } }",
            "class App { static client = other; constructor() { this.client = new Client(); } run() { this.other = value; this.client.m(); } }",
            "class App { constructor() { const f = () => { return other; }; this.client = new Client(); } run() { ({x = this.client} = other); Object.assign(this.other, value); this.client.m(); } }",
        ] { check(source, true, lang); }
    }
    for lang in [Language::TypeScript, Language::Tsx] {
        check("class App { public client: Props['client']; constructor() { this.client = new Client(); } run() { this.client.m(); } }", true, lang);
    }
}

#[test]
fn constructor_field_shape_barriers() {
    let mut failures = Vec::new();
    for source in [
        "class App { client: Client; run() { this.client.m(); } }",
        "class App { client = new Client(); run() { this.client.m(); } }",
        "class App { constructor() { if(ok) this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client.m(); this.client = new Client(); } }",
        "class App { constructor() { this.client = new Client(this.client.m()); } }",
        "class App { constructor() { if(ok) return; this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); return other; } run() { this.client.m(); } }",
        "class App { client; client; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { get client() { return other; } constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { [key] = other; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { client = other; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor(Client) { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { const Client = Other; this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new NS.Client(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = factory(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); } static run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); } run() { function nested() { this.client.m(); } } }",
        "class App { constructor() { this.client = new Client(); } run(this: Other) { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); } run() { this['client'].m(); } }",
        "class App { constructor() { this.client = new Client(); } } class Sub extends App { run() { this.client.m(); } }",
        "class App { private client; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); } constructor() {} run() { this.client.m(); } }",
        "import type Client from './client'; class App { constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { client?; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { protected client; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { declare client; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "@decorate class App { constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { @decorate client; constructor() { this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); this.client = new Client(); } run() { this.client.m(); } }",
        "class App { constructor() { { this.client = new Client(); } } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(this.client = other); } run() { this.client.m(); } }",
        "class App { constructor() { this.client = new Client(); } static run = () => this.client.m(); }",
        "class Base { get client() { return other; } set client(value) {} } class App extends Base { constructor() { super(); this.client = new Client(); } run() { this.client.m(); } }",
    ] { if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() { failures.push(source); } }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn constructor_field_write_barriers() {
    let mut failures = Vec::new();
    for write in [
        "this.client = other;",
        "delete this.client;",
        "this.client++;",
        "this[key] = other;",
        "this.client.m = other;",
        "delete (this.client).m;",
        "this . client . m = other;",
        "this['client'].m = other;",
        "({x: this.client} = other);",
        "for(this.client of others) {}",
        "Object.assign(this, other);",
        "Object.assign((this), other);",
        "Object['assign'](this, other);",
        "Reflect.set(this, 'client', other);",
        "Reflect.defineProperty((this.client), 'm', other);",
        "(this).client = other;",
        "(this).client.m = other;",
        "({x: [this.client.m]} = other);",
    ] {
        // Reflective helper is an explicit exclusion, not general alias analysis.
        let source = format!("class App {{ constructor() {{ this.client = new Client(); }} change() {{ {write} }} run() {{ this.client.m(); }} }}");
        if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() {
            failures.push(source);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

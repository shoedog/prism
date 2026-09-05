use prism::{
    ast::ParsedFile, call_graph::CallGraph, languages::Language, resolution::ResolutionConfidence,
};
use std::collections::BTreeMap;

fn graph(owner: &str, caller: &str, lang: Language) -> CallGraph {
    let files = BTreeMap::from([
        (
            "client.ts".into(),
            ParsedFile::parse("client.ts", owner, lang).unwrap(),
        ),
        (
            "app.ts".into(),
            ParsedFile::parse("app.ts", caller, lang).unwrap(),
        ),
    ]);
    let full = CallGraph::build(&files);
    let subset = CallGraph::build_direct_subset(&files, &files.keys().cloned().collect());
    assert_eq!(full.calls, subset.calls);
    for site in full.calls.values().flat_map(|s| s) {
        assert_eq!(
            format!("{:?}", full.resolve_call_site(site)),
            format!("{:?}", subset.resolve_call_site(site))
        );
    }
    full
}

fn assert_exact(cg: &CallGraph, expected: bool) {
    let site = cg
        .calls
        .iter()
        .filter(|(id, _)| id.name == "run")
        .flat_map(|(_, s)| s)
        .find(|s| s.callee_name == "m")
        .unwrap();
    let edges = cg.resolve_call_site(site);
    let exact: Vec<_> = edges
        .iter()
        .filter(|e| e.confidence == ResolutionConfidence::Exact)
        .collect();
    assert_eq!(exact.len(), usize::from(expected), "{site:?}: {edges:?}");
    if expected {
        assert_eq!(exact[0].target.file, "client.ts");
    }
}

#[test]
fn arrow_field_receiver_positive() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for field in [
            "m = () => {};",
            "m = async () => {};",
            "m = () => 42;",
            "m = () => {}; n = () => {};",
            "static n = () => {}; m = () => {};",
            "m = () => {}; change() { this.n = other; }",
        ] {
            assert_exact(&graph(&format!("class Client {{ {field} }} export default Client;"), "import Alias from './client'; function run() { const x = new Alias(); x.m(); }", lang), true);
        }
    }
}

#[test]
fn arrow_field_receiver_barriers() {
    let mut failures = Vec::new();
    for member in [
        "static m = () => {};",
        "m = () => {}; m = other;",
        "m = () => {}; m() {}",
        "m = () => {}; get m() { return other; }",
        "m = () => {}; constructor() { this.m = other; }",
        "m = () => {}; change() { this.m = other; }",
        "m = () => {}; change() { this['m'] = other; }",
        "m = () => {}; change() { this[key] = other; }",
        "m = () => {}; change() { delete this.m; }",
        "m = () => {}; change() { ({m: this.m} = other); }",
        "m = () => {}; change() { for (this.m of xs) {} }",
        "m = () => {}; change() { (this).m = other; }",
        "m = () => {}; change() { [this.m] = xs; }",
        "m = () => {}; [key] = other;",
        "@decorate m = () => {};",
        "private m = () => {};",
        "m = function() {};",
    ] {
        let cg = graph(
            &format!("class Client {{ {member} }} export default Client;"),
            "import Alias from './client'; function run() { const x = new Alias(); x.m(); }",
            Language::TypeScript,
        );
        if std::panic::catch_unwind(|| assert_exact(&cg, false)).is_err() {
            failures.push(member);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn arrow_field_receiver_local_write_barriers() {
    let mut failures = Vec::new();
    for write in [
        "x.m = other;",
        "x[key] = other;",
        "delete x.m;",
        "({m: x.m} = other);",
        "for (x.m of xs) {}",
        "(x).m = other;",
        "[x.m] = xs;",
    ] {
        let cg = graph("class Client { m = () => {}; } export default Client;", &format!("import Alias from './client'; function run() {{ const x = new Alias(); {write} x.m(); }}"), Language::TypeScript);
        if std::panic::catch_unwind(|| assert_exact(&cg, false)).is_err() {
            failures.push(write);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn arrow_field_receiver_write_edges() {
    for body in [
        "function run() { const x = new Alias(); function change(x) { x.m = other; } x.m(); }",
        "function run() { const x = new Alias(); const {v = x.m} = other; x.m(); }",
        "function run() { const x = new Alias(); x.m(); x.m = other; }",
        "function run(x: Alias) { x.m(); }",
    ] {
        assert_exact(
            &graph(
                "class Client { readonly m = () => {}; } export default Client;",
                &format!("import type Alias from './client'; {body}").replace(
                    "import type Alias",
                    if body.contains("new Alias") {
                        "import Alias"
                    } else {
                        "import type Alias"
                    },
                ),
                Language::TypeScript,
            ),
            true,
        );
    }
    assert_exact(&graph("class Client { m = () => {}; } export default Client;", "import Alias from './client'; function run() { const x = new Alias(); while (ok) { x.m(); x.m = other; } }", Language::TypeScript), false);
}

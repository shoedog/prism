use prism::{
    ast::ParsedFile,
    call_graph::CallGraph,
    languages::Language,
    resolution::{ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;

fn check(source: &str, expected: bool, language: Language) {
    let files = BTreeMap::from([
        (
            "client.ts".into(),
            ParsedFile::parse(
                "client.ts",
                "class Client { m = () => {}; } export default Client;",
                language,
            )
            .unwrap(),
        ),
        (
            "app.tsx".into(),
            ParsedFile::parse(
                "app.tsx",
                &format!("import type Client from './client'; {source}"),
                language,
            )
            .unwrap(),
        ),
        (
            "decoy.ts".into(),
            ParsedFile::parse("decoy.ts", "class Client { m() {} }", language).unwrap(),
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
        assert!(!sites.is_empty(), "{source}");
        for site in sites {
            let edges = cg.resolve_call_site(site);
            let exact: Vec<_> = edges
                .iter()
                .filter(|e| e.confidence == ResolutionConfidence::Exact)
                .collect();
            assert_eq!(
                exact.len(),
                usize::from(expected),
                "{source}: {site:?}: {edges:?}"
            );
            if expected {
                assert_eq!(exact[0].target.file, "client.ts");
                assert_eq!(exact[0].kind, ResolutionKind::TypedParam);
                assert!(!cg
                    .import_bindings
                    .get("app.tsx")
                    .is_some_and(|bs| bs.iter().any(|b| b.local == "Client")));
            }
        }
    }
}

#[test]
fn inline_prop_receiver_positive() {
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "function run({client}: {client: Client}) { client.m(); }",
            "const run = ({client}: {client: Client}) => { client.m(); };",
            "const run = memo(({client}: {client: Client}) => { client.m(); });",
            "function run({client: x}: {client: Client}) { x.m(); }",
            "function outer({client}: {client: Client}) { function run() { client.m(); } }",
            "function run({client, label}: {readonly client: Client; label?: string}) { client.m(); }",
            "function run({client}: {client: Client}) { function change(client) { client.m = other; } client.m(); }",
            "function run({client: x}: {client: Client}) { x.m(); x = other; }",
        ] { check(source, true, language); }
    }
}

#[test]
fn inline_prop_receiver_shape_barriers() {
    let mut failures = Vec::new();
    for source in [
        "function run({client}: {client?: Client}) { client.m(); }",
        "function run({client = other}: {client: Client}) { client.m(); }",
        "function run({client}: {client: Client} = other) { client.m(); }",
        "function run({client, ...rest}: {client: Client}) { client.m(); }",
        "function run({nested: {client}}: {nested: {client: Client}}) { client.m(); }",
        "function run({['client']: client}: {client: Client}) { client.m(); }",
        "function run({client}: {client: Client; client: Other}) { client.m(); }",
        "function run({client}: {other: Client}) { client.m(); }",
        "function run({client}: {client: Client | Other}) { client.m(); }",
        "function run({client}: {client: Client[]}) { client.m(); }",
        "function run({client}: {client: NS.Client}) { client.m(); }",
        "function run({client}: Props) { client.m(); }",
        "const run: React.FC<{client: Client}> = ({client}) => { client.m(); };",
        "function run() { const {client} = useApp(); client.m(); }",
        "function run({client, client: client}: {client: Client}) { client.m(); }",
        "function run({client}: {client: Client; [key: string]: Other}) { client.m(); }",
        "function run({client}: {get client(): Client}) { client.m(); }",
        "function run({client, missing = other}: {client: Client}) { client.m(); }",
        "function run({client}: {client: Client; other: number; other: string}) { client.m(); }",
    ] {
        if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() {
            failures.push(source);
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn inline_prop_receiver_shadow_and_write_barriers() {
    let mut failures = Vec::new();
    for source in [
        "function run<Client>({client}: {client: Client}) { client.m(); }",
        "function outer({client}: {client: Client}) { function run(client) { client.m(); } }",
        "function run({client}: {client: Client}) { client = other; client.m(); }",
        "function run({client}: {client: Client}) { client.m = other; client.m(); }",
        "function run({client}: {client: Client}) { delete client.m; client.m(); }",
        "function run({client}: {client: Client}) { while(ok) { client.m(); client = other; } }",
        "function run({client}: {client: Client}, client: Other) { client.m(); }",
        "type Client = Other; function run({client}: {client: Client}) { client.m(); }",
        "import type Client from './decoy'; function run({client}: {client: Client}) { client.m(); }",
    ] {
        if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

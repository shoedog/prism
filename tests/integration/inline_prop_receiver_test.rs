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
fn contextual_prop_receiver_positive() {
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "const run: (props: {client: Client}) => void = ({client}) => { client.m(); };",
            "const run: (props: {client: Client}) => void = function ({client}) { client.m(); };",
            "const run: (props: {client: Client}) => void = function named({client}) { client.m(); };",
            "const run: (props: {client: Client}) => void = async ({client: x}) => { x.m(); };",
            "const run: (props: {readonly client: Client; label?: string}) => void = ({client, label}) => { client.m(); };",
            "const run: (props: {client: Client}) => void = ({client}) => { function inner() { client.m(); } };",
            "const run: (props: {client: Client}) => void = ({client}) => { client.m(); client = other; };",
            "const run: (props: {client: Client}) => void = ({client}) => { function change(client) { client.m = other; } client.m(); };",
            "export const run: (/* context */ props: {client: Client}) => void = (/* binding */ {client}) => client.m();",
            "let run: (props: {client: Client}) => void = ({client}) => { client.m(); };",
        ] { check(source, true, language); }
    }
}

#[test]
fn contextual_prop_receiver_shape_barriers() {
    let mut failures = Vec::new();
    for source in [
        "const run: React.FC<{client: Client}> = ({client}) => { client.m(); };",
        "declare namespace React { type FC<P> = (p: {client: Other}) => void; } const run: React.FC<{client: Client}> = ({client}) => { client.m(); };",
        "const run: { (p: {client: Client}): void } = ({client}) => { client.m(); };",
        "const run: ((p: {client: Client}) => void) | Other = ({client}) => { client.m(); };",
        "const run: (p: {client?: Client}) => void = ({client}) => { client.m(); };",
        "const run: (p?: {client: Client}) => void = ({client}) => { client.m(); };",
        "const run: (...p: {client: Client}[]) => void = ({client}) => { client.m(); };",
        "const run: (p: {client: Client}, other: Other) => void = ({client}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({client}, other) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({client}: {client: Other}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({client} = other) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({client = other}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({client, ...rest}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({['client']: client}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({nested: {client}}) => { client.m(); };",
        "const run: (p: {client: Client; client: Other}) => void = ({client}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = ({client, client: client}) => { client.m(); };",
        "const run: (p: {client: Client | Other}) => void = ({client}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = memo(({client}) => { client.m(); });",
        "function outer() { const run = (({client}) => { client.m(); }) as (p: {client: Client}) => void; }",
        "const run: <Client>(p: {client: Client}) => void = ({client}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = function <T>({client}) { client.m(); };",
        "function outer<Client>() { const run: (p: {client: Client}) => void = ({client}) => { client.m(); }; }",
        "type Props = {client: Client}; const run: (p: Props) => void = ({client}) => { client.m(); };",
        "const run: ({client}: {client: Client}) => void = ({client}) => { client.m(); };",
        "const run: (p: {client: Client}) => void = client => { client.m(); };",
        "function outer() { const run: (p: {client: Client}) => void = function* ({client}) { client.m(); }; }",
        "const run: (p: {client: Client}) => void = ({client}) => { client.m(); const broken = ; };",
        "const run: (p: {client: Client; [key: string]: Other}) => void = ({client}) => { client.m(); };",
        "const run: (p: {get client(): Client}) => void = ({client}) => { client.m(); };",
    ] {
        if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn contextual_prop_receiver_write_barriers() {
    for source in [
        "const run: (p: {client: Client}) => void = ({client}) => { client = other; client.m(); };",
        "const run: (p: {client: Client}) => void = ({client}) => { client.m = other; client.m(); };",
        "const run: (p: {client: Client}) => void = ({client}) => { delete client.m; client.m(); };",
        "const run: (p: {client: Client}) => void = ({client}) => { while(ok) { client.m(); client = other; } };",
        "const run: (p: {client: Client}) => void = ({client}) => { function inner(client) { client.m(); } };",
    ] { check(source, false, Language::Tsx); }
}

#[test]
fn local_contextual_alias_positive() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "type F = (p: {client: Client}) => void; const run: F = ({client}) => client.m();",
            "export type F = (p: {client: Client}) => void; export const run: F = function named({client: x}) { x.m(); };",
            "const run: F = ({client}) => client.m(); type F = (p: {client: Client}) => void;",
            "type F = (/* sig */ p: {readonly client: Client; label?: string}) => void; let run: F = async ({client, label}) => client.m();",
            "type F = (p: {client: Client}) => void; function outer() { const run: F = ({client}) => { function inner() { client.m(); } }; }",
            "type F = (p: {client: Client}) => void; function outer<Client>() { const run: F = ({client}) => client.m(); }",
            "type F = (p: {client: Client}) => void; let F = other; F = value; const run: F = ({client}) => client.m();",
            "type F = (p: {client: Client}) => void; function sibling() { type F = Other; } const run: F = ({client}) => { client.m(); client = other; };",
            "type F = (p: {client: Client}) => void; function F() {} const run: F = ({client}) => client.m();",
            "type F = (p: {client: Client}) => void; export { F }; const run: F = ({client}) => client.m();",
        ] {
            if std::panic::catch_unwind(|| check(source, true, language)).is_err() { failures.push((language, source)); }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_contextual_alias_declaration_barriers() {
    let signature = "(p: {client: Client}) => void";
    let mut failures = Vec::new();
    for source in [
        format!("type F = {signature}; type F = {signature}; const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; interface F {{}} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; class F {{}} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; enum F {{ A }} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; namespace F {{}} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; namespace F.Inner {{}} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; export interface F {{}} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; declare class F {{}} const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; import * as F from './other'; const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; import F = NS.Other; const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; import F from './other'; const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; import type F from './other'; const run: F = ({{client}}) => client.m();"),
        format!("declare type F = {signature}; const run: F = ({{client}}) => client.m();"),
        format!("function outer() {{ type F = {signature}; const run: F = ({{client}}) => client.m(); }}"),
        format!("type F = {signature}; function outer<F>() {{ const run: F = ({{client}}) => client.m(); }}"),
        format!("type F = {signature}; function outer() {{ type F = Other; const run: F = ({{client}}) => client.m(); }}"),
        format!("type F = {signature}; function outer() {{ const run: F = ({{client}}) => client.m(); interface F {{}} }}"),
        format!("type F = {signature}; class Outer<F> {{ run() {{ const inner: F = ({{client}}) => client.m(); }} }}"),
        format!("type F = {signature}; function outer() {{ {{ class F {{}} const run: F = ({{client}}) => client.m(); }} }}"),
        format!("type F<T> = {signature}; const run: F<Client> = ({{client}}) => client.m();"),
        format!("type F<T = Client> = {signature}; const run: F = ({{client}}) => client.m();"),
        format!("type F = <Client>{signature}; const run: F = ({{client}}) => client.m();"),
        format!("type G = {signature}; type F = G; const run: F = ({{client}}) => client.m();"),
        "type F = F; const run: F = ({client}) => client.m();".into(),
        "import type F from './other'; const run: F = ({client}) => client.m();".into(),
        "type F = { (p: {client: Client}): void }; const run: F = ({client}) => client.m();".into(),
        format!("type F = ({signature}) | Other; const run: F = ({{client}}) => client.m();"),
        format!("type F = ({signature}) & Other; const run: F = ({{client}}) => client.m();"),
        format!("type F = ({signature}); const run: F = ({{client}}) => client.m();"),
        format!("type F = {signature}; const run: F = ({{client}}) => {{ client.m(); const broken = ; }};"),
    ] {
        if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_contextual_alias_receiver_barriers() {
    for source in [
        "type F = (p: {client?: Client}) => void; const run: F = ({client}) => client.m();",
        "type F = (p: {client: Client; client: Other}) => void; const run: F = ({client}) => client.m();",
        "type F = (p?: {client: Client}) => void; const run: F = ({client}) => client.m();",
        "type F = (p: {client: Client}) => void; const run: F = ({client}: {client: Other}) => client.m();",
        "type F = (p: {client: Client}) => void; const run: F = ({client}) => { client = other; client.m(); };",
        "type F = (p: {client: Client}) => void; const run: F = ({client}) => { delete client.m; client.m(); };",
        "type F = (p: {client: Client}) => void; const run: F = ({client}) => { while(ok) { client.m(); client = other; } };",
        "type F = (p: {client: Client}) => void; const run: F = ({client}) => { function inner(client) { client.m(); } };",
        "type Client = Other; type F = (p: {client: Client}) => void; const run: F = ({client}) => client.m();",
        "type F = (p: {client: Client}, q: Other) => void; const run: F = ({client}) => client.m();",
        "type F = (...p: {client: Client}[]) => void; const run: F = ({client}) => client.m();",
        "type F = (p: {client: Client}) => void; const run: F = ({client} = other) => client.m();",
        "type F = (p: {client: Client}) => void; const run: F = ({client, ...rest}) => client.m();",
        "type F = (p: {client: Client}) => void; const run: F = ({client}) => { ({m: client.m} = other); client.m(); };",
    ] { check(source, false, Language::Tsx); }
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

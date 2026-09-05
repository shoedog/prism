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
fn local_callable_alias_positive() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "type F = { (p: {client: Client}): void }; const run: F = ({client}) => client.m();",
            "type Props = {client: Client}; type F = { (p: Props): void }; const run: F = ({client}) => client.m();",
            "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => client.m();",
            "type Props = {client: Client}; type F<P> = { (p: P): void }; const run: F<Props> = ({client}) => client.m();",
            "export type F = { (p: {client: Client}): void }; export const run: F = function named({client: x}) { x.m(); };",
            "type F<P> = { (p: P): void }; const run: F<{readonly client: Client; label?: string}> = async ({client, label}) => client.m();",
            "const run: F<Props> = ({client}) => client.m(); type Props = {client: Client}; type F<P> = { (p: P): void };",
            "type F</* binder */ P,> = { /* member */ (/* param */ p: P): void; /* end */ }; const run: F<{client: Client},> = ({client}) => client.m();",
            "type F = { (p: {client: Client}): void }; function outer<Client>() { const run: F = ({client}) => client.m(); }",
            "type F<Client> = { (p: Client): void }; const run: F<{client: Client}> = ({client}) => client.m();",
            "type Props = {client: Client}; type F<Props> = { (p: Props): void }; function outer<Client>() { const run: F<Props> = ({client}) => client.m(); }",
            "type F<P> = { (p: P): void }; let F = other; F = value; const run: F<{client: Client}> = ({client}) => { type Client = Other; client.m(); };",
            "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { function inner() { client.m(); } };",
            "type F<P> = { (p: P) }; const run: F<{client: Client}> = ({client: x}) => { x.m(); x = other; };",
            "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { function change(client) { delete client.m; } client.m(); };",
            "type F = { (p: {client: Client}): void }; const run: F = ({client}) => { interface Client {} client.m(); };",
            "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}: {client: Client}) => client.m();",
        ] {
            if std::panic::catch_unwind(|| check(source, true, language)).is_err() { failures.push((language, source)); }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_callable_alias_shape_barriers() {
    let mut failures = Vec::new();
    for (binder, parameter, reference) in [
        ("", "{client: Client}", "F"),
        ("<P>", "P", "F<{client: Client}>"),
    ] {
        for members in [
            "",
            "(p: $): void; (p: Other): void",
            "(p: Other): void; (p: $): void",
            "(p: $): void; (p: $): void",
            "(p: $): void; label: string",
            "label?: string; (p: $): void",
            "(p: $): void; get label(): string",
            "(p: $): void; run(): void",
            "(p: $): void; [Symbol.iterator](): Other",
            "(p: $): void; [key: string]: Other",
            "(p: $): void; new(p: $): Other",
            "new(p: $): Other",
            "run(p: $): void",
            "call: (p: $) => void",
            "<Q>(p: $): void",
            "<P>(p: $): void",
            "(p?: $): void",
            "(...p: $[]): void",
            "({client}: $): void",
            "(p: $, q: Other): void",
        ] {
            let source = format!(
                "type F{binder} = {{{}}}; const run: {reference} = ({{client}}) => client.m();",
                members.replace('$', parameter)
            );
            if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() {
                failures.push(source);
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_callable_alias_authority_and_receiver_barriers() {
    let mut failures = Vec::new();
    for source in [
        "const run: { (p: {client: Client}): void } = ({client}) => client.m();",
        "declare type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; interface F {} const run: F<{client: Client}> = ({client}) => client.m();",
        "type F = { (p: {client: Client}): void }; type F = Other; const run: F = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; import type F from './other'; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; import * as F from './other'; const run: F<{client: Client}> = ({client}) => client.m();",
        "function outer() { type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => client.m(); }",
        "type F<P> = { (p: P): void }; function outer<F>() { const run: F<{client: Client}> = ({client}) => client.m(); }",
        "type F<P> = { (p: P): void }; function outer<Client>() { const run: F<{client: Client}> = ({client}) => client.m(); }",
        "type Props = {client: Client}; type F<P> = { (p: P): void }; function outer<Props>() { const run: F<Props> = ({client}) => client.m(); }",
        "type F<P = Other> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P extends Other> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<in P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}, Other> = ({client}) => client.m();",
        "type F<P> = { (p: {client: P}): void }; const run: F<Client> = ({client}) => client.m();",
        "type F<P> = { (p: Other): void }; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P> = ({ (p: P): void }); const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void } & Other; const run: F<{client: Client}> = ({client}) => client.m();",
        "type G<P> = { (p: P): void }; type F<P> = G<P>; const run: F<{client: Client}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client?: Client}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client; client: Other}> = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}: Missing) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}: {client: Other}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client = other}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client, ...rest}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { delete client.m; client.m(); };",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { while(ok) { client.m(); client = other; } };",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { function inner(client) { client.m(); } };",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = memo(({client}) => client.m());",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { client.m(); const broken = ; };",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}) => { ({m: client.m} = other); client.m(); };",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client} = other) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client, client: client}) => client.m();",
        "type F<P> = { (p: P): void }; const run: F<{client: Client}> = ({client}, other) => client.m();",
        "type F<P> = { (p: P): void }; function outer() { const run: F<{client: Client}> = ({client}) => client.m(); interface F {} }",
        "type F = { (p: Props): void }; type Props = {client: Client}; interface Props {} const run: F = ({client}) => client.m();",
        "type F<P> = { (p: P): void }; type Props = {client: Client}; function outer() { const run: F<Props> = ({client}) => client.m(); interface Props {} }",
        "type F = { (p: {client: Client}): void } | Other; const run: F = ({client}) => client.m();",
    ] {
        if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_callable_interface_module_boundary() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for (marker, expected) in [("", false), ("export {};", true)] {
            let files = BTreeMap::from([
                ("app.ts".to_string(), ParsedFile::parse("app.ts", &format!("{marker} class Client {{ m() {{}} }} interface F {{ (p: {{client: Client}}): void }} const run: F = ({{client}}) => client.m();"), language).unwrap()),
                ("global.ts".to_string(), ParsedFile::parse("global.ts", "interface F { (p: {client: Other}): void }", language).unwrap()),
            ]);
            for cg in [
                CallGraph::build(&files),
                CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
            ] {
                let site = cg
                    .calls
                    .values()
                    .flatten()
                    .find(|s| s.callee_name == "m")
                    .unwrap();
                let edges = cg.resolve_call_site(site);
                let exact: Vec<_> = edges
                    .iter()
                    .filter(|e| e.confidence == ResolutionConfidence::Exact)
                    .collect();
                if exact.len() != usize::from(expected)
                    || (expected && exact[0].target.file != "app.ts")
                {
                    failures.push(format!("{language:?} {marker}: {site:?} {edges:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_callable_interface_positive() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "interface F { (p: {client: Client}): void } const run: F = ({client}) => client.m();",
            "type Props = {client: Client}; interface F { (p: Props): void } const run: F = ({client}) => client.m();",
            "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
            "type Props = {client: Client}; interface F<P> { (p: P): void } const run: F<Props> = ({client}) => client.m();",
            "interface F { (p: {client: Client}): void } export const run: F = function named({client: x}) { x.m(); };",
            "interface F { (p: {client: Client}): void } export type {F} from './other'; const run: F = ({client}) => client.m();",
            "interface F<P> { (p: P): void } const run: F<{readonly client: Client; label?: string}> = async ({client, label}) => client.m();",
            "const run: F<Props> = ({client}) => client.m(); type Props = {client: Client}; interface F<P> { (p: P): void }",
            "interface F</* binder */ P,> { /* member */ (/* param */ p: P): void; /* end */ } const run: F<{client: Client},> = ({client}) => client.m();",
            "interface F { (p: {client: Client}): void } function outer<Client>() { const run: F = ({client}) => client.m(); }",
            "interface F<Client> { (p: Client): void } const run: F<{client: Client}> = ({client}) => client.m();",
            "type Props = {client: Client}; interface F<Props> { (p: Props): void } function outer<Client>() { const run: F<Props> = ({client}) => client.m(); }",
            "interface F<P> { (p: P): void } let F = other; F = value; const run: F<{client: Client}> = ({client}) => { type Client = Other; client.m(); };",
            "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { function inner() { client.m(); } };",
            "interface F<P> { (p: P) } const run: F<{client: Client}> = ({client: x}) => { x.m(); x = other; };",
            "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { function change(client) { delete client.m; } client.m(); };",
            "interface F { (p: {client: Client}): void } const run: F = ({client}) => { interface Client {} client.m(); };",
            "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}: {client: Client}) => client.m();",
        ] {
            if std::panic::catch_unwind(|| check(source, true, language)).is_err() { failures.push((language, source)); }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_callable_interface_shape_barriers() {
    let mut failures = Vec::new();
    for (binder, parameter, reference) in [
        ("", "{client: Client}", "F"),
        ("<P>", "P", "F<{client: Client}>"),
    ] {
        for members in [
            "",
            "(p: $): void; (p: Other): void",
            "(p: Other): void; (p: $): void",
            "(p: $): void; (p: $): void",
            "(p: $): void; label: string",
            "label?: string; (p: $): void",
            "(p: $): void; get label(): string",
            "(p: $): void; run(): void",
            "(p: $): void; [Symbol.iterator](): Other",
            "(p: $): void; [key: string]: Other",
            "(p: $): void; new(p: $): Other",
            "new(p: $): Other",
            "run(p: $): void",
            "call: (p: $) => void",
            "<Q>(p: $): void",
            "<P>(p: $): void",
            "(p?: $): void",
            "(...p: $[]): void",
            "({client}: $): void",
            "(p: $, q: Other): void",
        ] {
            let source = format!(
                "interface F{binder} {{{}}} const run: {reference} = ({{client}}) => client.m();",
                members.replace('$', parameter)
            );
            if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() {
                failures.push(source);
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_callable_interface_authority_and_receiver_barriers() {
    let mut failures = Vec::new();
    for source in [
        "export interface F { (p: {client: Client}): void } const run: F = ({client}) => client.m();",
        "interface F<P> { (p: P): void } export type {F}; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } export {type F as Other}; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } export {F as Other}; const run: F<{client: Client}> = ({client}) => client.m();",
        "export default interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } export default F; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface Base {} interface F<P> extends Base { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F extends Other { (p: {client: Client}): void } const run: F = ({client}) => client.m();",
        "interface F<P> { (p: P): void } interface F<P> {} const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F {} interface F { (p: {client: Client}): void } const run: F = ({client}) => client.m();",
        "interface F<P> { (p: P): void } type F = Other; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } class F {} const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } namespace F.Inner {} const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } import type F from './other'; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } import * as F from './other'; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } import F = NS.Other; const run: F<{client: Client}> = ({client}) => client.m();",
        "declare interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "export declare interface F { (p: {client: Client}): void } const run: F = ({client}) => client.m();",
        "function outer() { interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m(); }",
        "interface F<P> { (p: P): void } function outer<F>() { const run: F<{client: Client}> = ({client}) => client.m(); }",
        "interface F<P> { (p: P): void } function outer<Client>() { const run: F<{client: Client}> = ({client}) => client.m(); }",
        "type Props = {client: Client}; interface F<P> { (p: P): void } function outer<Props>() { const run: F<Props> = ({client}) => client.m(); }",
        "interface F<P> { (p: P): void } function outer() { const run: F<{client: Client}> = ({client}) => client.m(); interface F {} }",
        "interface F<P extends Other> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P = Other> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<in P> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P, Q> { (p: P): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } const run: F = ({client}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}, Other> = ({client}) => client.m();",
        "interface F { (p: {client: Client}): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: {client: P}): void } const run: F<Client> = ({client}) => client.m();",
        "interface F<P> { (p: Other): void } const run: F<{client: Client}> = ({client}) => client.m();",
        "interface G<P> { (p: P): void } type F<P> = G<P>; const run: F<{client: Client}> = ({client}) => client.m();",
        "interface Props {client: Client} interface F<P> { (p: P): void } const run: F<Props> = ({client}) => client.m();",
        "interface Props {client: Client} interface F { (p: Props): void } const run: F = ({client}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client?: Client}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client; client: Other}> = ({client}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}: Missing) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}: {client: Other}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client = other}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client, ...rest}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { client = other; client.m(); };",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { delete client.m; client.m(); };",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { while(ok) { client.m(); client = other; } };",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { function inner(client) { client.m(); } };",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = memo(({client}) => client.m());",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { client.m(); const broken = ; };",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}) => { ({m: client.m} = other); client.m(); };",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client} = other) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client, client: client}) => client.m();",
        "interface F<P> { (p: P): void } const run: F<{client: Client}> = ({client}, other) => client.m();",
    ] {
        if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_generic_alias_positive() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "type F<P> = (p: P) => void; const run: F<{client: Client}> = ({client}) => client.m();",
            "type Props = {client: Client}; type F<P> = (p: P) => void; const run: F<Props> = ({client}) => client.m();",
            "export type F<P> = (p: P) => void; export const run: F<{client: Client}> = function named({client: x}) { x.m(); };",
            "const run: F<Props> = ({client}) => client.m(); type F<P> = (p: P) => void; type Props = {client: Client};",
            "type F<P> = (p: P) => P; const run: F<{client: Client}> = async ({client}) => client.m();",
            "type F</* binder */ P> = (/* param */ p: P) => void; const run: F</* arg */ {readonly client: Client; label?: string}> = ({client, label}) => client.m();",
            "type F<Client> = (p: Client) => void; const run: F<{client: Client}> = ({client}) => client.m();",
            "type Props = {client: Client}; type F<Props> = (p: Props) => void; const run: F<Props> = ({client}) => client.m();",
            "type Props = {client: Client}; type F<P> = (p: P) => void; function outer<Client>() { const run: F<Props> = ({client}) => client.m(); }",
            "type F<P> = (p: P) => void; const run: F<{client: Client}> = ({client}) => { type Client = Other; client.m(); };",
            "type F<P> = (p: P) => void; let F = other; F = value; const run: F<{client: Client}> = ({client}) => client.m();",
            "type F<P> = (p: P) => void; const run: F<{client: Client}> = ({client}) => { function inner() { client.m(); } };",
            "type F<P> = (p: P) => void; const run: F<{client: Client}> = ({client: x}) => { x.m(); x = other; };",
            "type F<P> = (p: P) => void; const run: F<{client: Client}> = ({client}) => { function change(client) { delete client.m; } client.m(); };",
            "type F<P,> = (p: P) => void; const run: F<{client: Client},> = ({client}) => client.m();",
            "type Props = {client: Client}; type F<Props> = (p: Props) => void; function outer<Client>() { const run: F<Props> = ({client}) => client.m(); }",
            "type F<P> = (p: P) => void; const run: F<{client: Client}> = ({client}: {client: Client}) => client.m();",
        ] {
            if std::panic::catch_unwind(|| check(source, true, language)).is_err() { failures.push((language, source)); }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_generic_alias_declaration_barriers() {
    let mut failures = Vec::new();
    for declaration in [
        "",
        "type F<P> = (p: P) => void; type F<P> = (p: P) => void;",
        "type F<P> = (p: P) => void; interface F {}",
        "type F<P> = (p: P) => void; class F {}",
        "type F<P> = (p: P) => void; enum F { A }",
        "type F<P> = (p: P) => void; namespace F.Inner {}",
        "type F<P> = (p: P) => void; import type F from './other';",
        "type F<P> = (p: P) => void; import F from './other';",
        "type F<P> = (p: P) => void; import F = NS.Other;",
        "type F<P> = (p: P) => void; import * as F from './other';",
        "type F<P> = (p: P) => void; declare class F {}",
        "declare type F<P> = (p: P) => void;",
        "type F = (p: {client: Client}) => void;",
        "type F<P, Q> = (p: P) => void;",
        "type F<P, P> = (p: P) => void;",
        "type F<P extends {client: Client}> = (p: P) => void;",
        "type F<P = {client: Client}> = (p: P) => void;",
        "type F<in P> = (p: P) => void;",
        "type F<out P> = (p: P) => void;",
        "type F<const P> = (p: P) => void;",
        "type G<P> = (p: P) => void; type F<P> = G<P>;",
        "type F<P> = F<P>;",
        "type F<P> = ((p: P) => void);",
        "type F<P> = ((p: P) => void) | Other;",
        "type F<P> = <P>(p: P) => void;",
        "type F<P> = <Q>(p: P) => void;",
        "type F<P> = (p: Other) => void;",
        "type F<P> = (p: {client: P}) => void;",
        "type F<P> = (p: P & {client: Client}) => void;",
        "type F<P> = (p: P, other: Other) => void;",
        "type F<P> = (p?: P) => void;",
        "type F<P> = (...p: P[]) => void;",
        "type F<P> = ({client}: P) => void;",
    ] {
        for argument in ["{client: Client}", "Props"] {
            let source = format!("type Props = {{client: Client}}; {declaration} const run: F<{argument}> = ({{client}}) => client.m();");
            if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() {
                failures.push(source);
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_generic_alias_argument_and_receiver_barriers() {
    let mut failures = Vec::new();
    for consumer in [
        "const run: F = ({client}) => client.m();",
        "const run: F<> = ({client}) => client.m();",
        "const run: F<{client: Client}, Other> = ({client}) => client.m();",
        "const run: NS.F<{client: Client}> = ({client}) => client.m();",
        "const run: F<Missing> = ({client}) => client.m();",
        "type Props = {client: Client}; interface Props {} const run: F<Props> = ({client}) => client.m();",
        "import type Props from './other'; const run: F<Props> = ({client}) => client.m();",
        "type Props<P> = {client: P}; const run: F<Props<Client>> = ({client}) => client.m();",
        "type Props = {client: Client}; type Alias = Props; const run: F<Alias> = ({client}) => client.m();",
        "const run: F<({client: Client})> = ({client}) => client.m();",
        "const run: F<{client: Client} | Other> = ({client}) => client.m();",
        "const run: F<{client?: Client}> = ({client}) => client.m();",
        "const run: F<{client: Client; client: Other}> = ({client}) => client.m();",
        "const run: F<{get client(): Client}> = ({client}) => client.m();",
        "const run: F<{client: Client; [key: string]: Other}> = ({client}) => client.m();",
        "function outer<Client>() { const run: F<{client: Client}> = ({client}) => client.m(); }",
        "type Props = {client: Client}; function outer<Props>() { const run: F<Props> = ({client}) => client.m(); }",
        "function outer<F>() { const run: F<{client: Client}> = ({client}) => client.m(); }",
        "function outer() { const run: F<{client: Client}> = ({client}) => client.m(); interface F {} }",
        "function outer() { type Props = {client: Client}; const run: F<Props> = ({client}) => client.m(); }",
        "const run: F<{client: Client}> = ({client}: Missing) => client.m();",
        "const run: F<{client: Client}> = ({client}: {client: Other}) => client.m();",
        "const run: F<{client: Client}> = <T>({client}) => client.m();",
        "const run: F<{client: Client}> = ({client = other}) => client.m();",
        "const run: F<{client: Client}> = ({client, ...rest}) => client.m();",
        "const run: F<{client: Client}> = ({client} = other) => client.m();",
        "const run: F<{client: Client}> = ({client}) => { client = other; client.m(); };",
        "const run: F<{client: Client}> = ({client}) => { delete client.m; client.m(); };",
        "const run: F<{client: Client}> = ({client}) => { while(ok) { client.m(); client = other; } };",
        "const run: F<{client: Client}> = ({client}) => { function inner(client) { client.m(); } };",
        "const run: F<{client: Client}> = memo(({client}) => client.m());",
        "function outer() { type G<P> = (p: P) => void; const run: G<{client: Client}> = ({client}) => client.m(); }",
        "const run: F<{client: Client}> = ({client}) => { client.m(); const broken = ; };",
        "const run: F<{client: Client}> = ({client}) => { ({m: client.m} = other); client.m(); };",
        "const run: F<{client: Client}> = ({client, client: client}) => client.m();",
        "const run: F<{client: Client}> = ({client}, other) => client.m();",
        "type C = Client; const run: F<{client: C}> = ({client}) => client.m();",
        "function outer() { const run: F<{client: Client}> = ({client}) => client.m(); type Client = Other; }",
        "type Props = {client: Client}; function outer() { const run: F<Props> = ({client}) => client.m(); interface Props {} }",
    ] {
        let source = format!("type F<P> = (p: P) => void; {consumer}");
        if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_props_alias_positive() {
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "type Props = {client: Client}; function run({client}: Props) { client.m(); }",
            "type Props = {client: Client}; const run: (p: Props) => void = ({client}) => client.m();",
            "type Props = {client: Client}; type F = (p: Props) => void; const run: F = ({client}) => client.m();",
            "export type Props = {client: Client}; export const run = function named({client: x}: Props) { x.m(); };",
            "function run({client}: Props) { client.m(); } type Props = {client: Client};",
            "type F = (p: Props) => void; const run: F = ({client}) => client.m(); type Props = {client: Client};",
            "type Props = {readonly client: Client; label?: string}; const run = async ({client, label}: Props) => client.m();",
            "type Props = {client: Client}; function run<Client>({client}: Props) { client.m(); }",
            "type Props = {client: Client}; type F = (p: Props) => void; function outer<Props, Client>() { const run: F = ({client}) => client.m(); }",
            "type Props = {client: Client}; let Props = other; Props = value; function run({client}: Props) { client.m(); }",
            "type Props = {client: Client}; function run({client}: Props) { function inner() { client.m(); } }",
            "type Props = {client: Client}; function run({client: x}: Props) { x.m(); x = other; }",
            "type Props = {client: Client}; const run = memo(({client}: Props) => client.m());",
            "type Props = {client: Client}; function run({client}: Props) { type Client = Other; client.m(); }",
            "type Props = {client: Client}; const run: (p: Props) => void = ({client}) => { function change(client) { delete client.m; } client.m(); };",
        ] {
            if std::panic::catch_unwind(|| check(source, true, language)).is_err() { failures.push((language, source)); }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_props_alias_declaration_barriers() {
    let mut failures = Vec::new();
    for declaration in [
        "",
        "type Props = {client: Client}; type Props = {client: Client};",
        "type Props = {client: Client}; interface Props {}",
        "type Props = {client: Client}; class Props {}",
        "type Props = {client: Client}; enum Props { A }",
        "type Props = {client: Client}; namespace Props.Inner {}",
        "type Props = {client: Client}; import Props from './other';",
        "type Props = {client: Client}; import type Props from './other';",
        "type Props = {client: Client}; import * as Props from './other';",
        "type Props = {client: Client}; import Props = NS.Other;",
        "type Props = {client: Client}; declare class Props {}",
        "declare type Props = {client: Client};",
        "type Props<T = Client> = {client: T};",
        "type Other = {client: Client}; type Props = Other;",
        "type Props = Props;",
        "type Props = Other; type Other = Props;",
        "interface Props { client: Client }",
        "type Props = {client: Client} | Other;",
        "type Props = {client: Client} & Other;",
        "type Props = ({client: Client});",
        "type Props = { [K in Key]: Client };",
        "type Props = T extends U ? {client: Client} : Other;",
        "type Props = (p: {client: Client}) => void;",
    ] {
        for consumer in [
            "function run({client}: Props) { client.m(); }",
            "const run: (p: Props) => void = ({client}) => client.m();",
            "type F = (p: Props) => void; const run: F = ({client}) => client.m();",
        ] {
            let source = format!("{declaration} {consumer}");
            if std::panic::catch_unwind(|| check(&source, false, Language::Tsx)).is_err() {
                failures.push(source);
            }
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn local_props_alias_receiver_barriers() {
    let mut failures = Vec::new();
    for source in [
        "type Props = {client?: Client}; function run({client}: Props) { client.m(); }",
        "type Props = {client: Client; client: Other}; function run({client}: Props) { client.m(); }",
        "type Props = {get client(): Client}; function run({client}: Props) { client.m(); }",
        "type Props = {client: Client; [key: string]: Other}; function run({client}: Props) { client.m(); }",
        "type Props = {client: Client | Other}; function run({client}: Props) { client.m(); }",
        "type C = Client; type Props = {client: C}; function run({client}: Props) { client.m(); }",
        "type Props = {client: Client}; function run<Props>({client}: Props) { client.m(); }",
        "type Props = {client: Client}; function outer() { type Props = Other; const run: (p: Props) => void = ({client}) => client.m(); }",
        "function outer() { type Props = {client: Client}; function run({client}: Props) { client.m(); } }",
        "type Props = {client: Client}; function run({client = other}: Props) { client.m(); }",
        "type Props = {client: Client}; function run({client, ...rest}: Props) { client.m(); }",
        "type Props = {client: Client}; function run({client}: Props = other) { client.m(); }",
        "type Props = {client: Client}; const run: (p: Props) => void = ({client}: Missing) => client.m();",
        "type Props = {client: Other}; const run: (p: {client: Client}) => void = ({client}: Props) => client.m();",
        "type Props = {client: Client}; function run({client}: Props) { client = other; client.m(); }",
        "type Props = {client: Client}; function run({client}: Props) { delete client.m; client.m(); }",
        "type Props = {client: Client}; function run({client}: Props) { while(ok) { client.m(); client = other; } }",
        "type Props = {client: Client}; function run({client}: Props) { function inner(client) { client.m(); } }",
        "type Props = {client: Client}; function run() { const {client}: Props = other; client.m(); }",
        "type Props = {client: Client}; function run({client}: Props) { client.m(); const broken = ; }",
        "type Props = {client: Client}; type F = (p: Props) => void; const run: F = ({client}) => { delete client.m; client.m(); };",
        "type Props = {client: Client}; const run: (p: Props) => void = ({client}) => { ({m: client.m} = other); client.m(); };",
        "type Props = {client: Client}; type F = (p: Props) => void; const run: F = ({client}: Missing) => client.m();",
        "type Props = {client: Client}; function outer() { interface Props {} type F = (p: Props) => void; const run: F = ({client}) => client.m(); }",
        "type Props = {client: Client}; function outer() { const run: (p: Props) => void = ({client}) => client.m(); interface Props {} }",
    ] {
        if std::panic::catch_unwind(|| check(source, false, Language::Tsx)).is_err() { failures.push(source); }
    }
    assert!(failures.is_empty(), "{failures:?}");
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

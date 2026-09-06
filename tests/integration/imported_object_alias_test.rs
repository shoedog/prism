use prism::{
    ast::ParsedFile,
    call_graph::CallGraph,
    languages::Language,
    resolution::{ResolutionConfidence, ResolutionKind},
};
use std::collections::{BTreeMap, BTreeSet};

fn sources(
    app: &str,
    props: &str,
    extra: &[(&str, &str)],
    lang: Language,
) -> BTreeMap<String, ParsedFile> {
    let mut texts = BTreeMap::from([
        ("client.ts", "export default class Client { m() {} }"),
        ("props.ts", props),
        ("app.ts", app),
    ]);
    texts.extend(extra.iter().copied());
    texts
        .into_iter()
        .map(|(p, s)| (p.into(), ParsedFile::parse(p, s, lang).unwrap()))
        .collect()
}
const APP: &str = "import type {Props} from './props'; class DeclaredClient { m() {} } export function run({client}: Props) { client.m(); }";
const PROPS: &str =
    "import type DeclaredClient from './client'; export type Props = {client: DeclaredClient};";

fn check(cg: &CallGraph, expected: Option<&str>) {
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.file == "app.ts" && s.callee_name == "m")
        .collect();
    assert!(!sites.is_empty(), "probe must contain a receiver call");
    for site in sites {
        let exact: Vec<_> = cg
            .resolve_call_site(site)
            .into_iter()
            .filter(|e| e.confidence == ResolutionConfidence::Exact)
            .collect();
        assert_eq!(
            exact.len(),
            usize::from(expected.is_some()),
            "{site:?}: {exact:?}"
        );
        if let Some(file) = expected {
            assert_eq!(exact[0].target.file, file);
            assert_eq!(exact[0].kind, ResolutionKind::TypedParam);
        }
    }
}

#[test]
fn imported_object_alias_defining_scope() {
    for lang in [Language::TypeScript, Language::Tsx] {
        let files = sources(APP, PROPS, &[], lang);
        check(&CallGraph::build(&files), Some("client.ts"));
        check(
            &CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
            Some("client.ts"),
        );
    }
}

#[test]
fn imported_object_alias_augmentation_transition() {
    let a = sources(APP, PROPS, &[], Language::Tsx);
    let b = sources(
        APP,
        PROPS,
        &[(
            "augment.d.ts",
            "export {}; declare module './props' { interface Extra {} }",
        )],
        Language::Tsx,
    );
    for (from, to, expected) in [(&a, &b, None), (&b, &a, Some("client.ts"))] {
        let mut graph = CallGraph::build(from);
        let changed = BTreeSet::from(["augment.d.ts".to_string()]);
        graph.remove_files(&changed);
        graph.merge(CallGraph::build_direct_subset(to, &changed));
        check(&graph, expected);
        check(&CallGraph::build(to), expected);
    }
}

#[test]
fn imported_object_alias_positive_forms() {
    for lang in [Language::TypeScript, Language::Tsx] {
        for app in [
            "import {Props} from './props'; function run({client}: Props) { client.m(); }",
            "import {type Props as P} from './props'; const run = ({client: x}: P) => x.m();",
            "import type {Props as P} from './props'; const run = async function named({client}: P) { client.m(); };",
            "import type {Props} from './props'; function run<DeclaredClient>({client}: Props) { client.m(); }",
            "import type {Props} from './props'; function run({client}: Props) { function inner() { client.m(); } }",
            "import type {Props} from './props'; function run({client}: Props) { client.m(); client = other; }",
            "import type {Props} from './props'; function sibling() { type Props = Other; } function run({client}: Props) { client.m(); }",
        ] {
            let files = sources(app, PROPS, &[], lang);
            check(&CallGraph::build(&files), Some("client.ts"));
            check(&CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()), Some("client.ts"));
        }
    }
}

#[test]
fn imported_object_alias_negative_population() {
    let mut variants: Vec<(String, String, Vec<(&str, &str)>)> = Vec::new();
    for app in [
        "import type {Props} from './missing'; function run({client}: Props) { client.m(); }",
        "import type {Props} from 'props'; function run({client}: Props) { client.m(); }",
        "import type Props from './props'; function run({client}: Props) { client.m(); }",
        "import type * as ns from './props'; function run({client}: ns.Props) { client.m(); }",
        "function run({client}: import('./props').Props) { client.m(); }",
        "import type {Props} from './props'; import type {Props} from './props'; function run({client}: Props) { client.m(); }",
        "import type {Props} from './props'; interface Props {} function run({client}: Props) { client.m(); }",
        "import type {Props} from './props'; function run<Props>({client}: Props) { client.m(); }",
        "import type {Props} from './props'; function outer() { type Props = Other; function run({client}: Props) { client.m(); } }",
        "import type {Props} from './props'; function run({client}: Props) { client = other; client.m(); }",
        "import type {Props} from './props'; function run({client}: Props) { client.m = other; client.m(); }",
        "import type {Props} from './props'; function run({client}: Props) { delete client.m; client.m(); }",
        "import type {Props} from './props'; function run({client}: Props) { ({x: client.m} = other); client.m(); }",
        "import type {Props} from './props'; function run({client}: Props) { while(ok) { client.m(); client = other; } }",
        "import type {Props} from './props'; function run({client}: Props) { function inner(client) { client.m(); } }",
        "import type {Props} from './props'; function run({client}: Props, client) { client.m(); }",
        "import type {Props} from './props'; function run({client = other}: Props) { client.m(); }",
        "import type {Props} from './props'; function run({client, ...rest}: Props) { client.m(); }",
        "import type {Props} from './props'; function run({['client']: client}: Props) { client.m(); }",
        "import type {Props} from './props'; function run({client, client: x}: Props) { client.m(); }",
        "import type {Props} from './props'; function run({client}: Props = other) { client.m(); }",
        "import type {Props} from './props'; const run: (p: Props) => void = ({client}) => client.m();",
        "import type {Props} from './props'; const run: React.FC<Props> = ({client}) => client.m();",
        "import type {Props} from './props'; function run() { const {client}: Props = other; client.m(); }",
        "import type {Props} from './props'; function run({client}: Props<Other>) { client.m(); }",
    ] { variants.push((app.into(), PROPS.into(), vec![])); }
    for declaration in [
        "export interface Props {client: DeclaredClient}",
        "type Props = {client: DeclaredClient}; export {Props};",
        "export type Props<T> = {client: DeclaredClient};",
        "export type Props = {client?: DeclaredClient};",
        "export type Props = {client: DeclaredClient | Other};",
        "export type Props = {client: DeclaredClient; client: DeclaredClient};",
        "export type Props = {client: DeclaredClient; focus(): void};",
        "export type Props = {client: DeclaredClient; [key: string]: unknown};",
        "export type Props = {client: DeclaredClient}; interface Props {}",
        "export type Props = {client: DeclaredClient}; export type Props = {client: Other};",
        "export type Props = Other;",
        "export type Props = {client: DeclaredClient}; export {Other as Props};",
        "export type Props = {client: DeclaredClient}; export * from './other';",
        "export type {Props} from './other';",
    ] {
        variants.push((
            APP.into(),
            format!("import type DeclaredClient from './client'; {declaration}"),
            vec![],
        ));
    }
    for extra in [
        vec![("props.tsx", PROPS)],
        vec![("props/index.ts", PROPS)],
        vec![("client.tsx", "export default class Client { m() {} }")],
        vec![(
            "augment.d.ts",
            "export {}; declare module './client' { interface Client {extra: string} }",
        )],
        vec![(
            "global.ts",
            "export {}; declare global { interface Window { extra: string } }",
        )],
        vec![(
            "ambient.d.ts",
            "declare module 'external' { interface Props {} }",
        )],
        vec![("broken.ts", "const x = ;")],
        vec![("client.ts", "export default class Client { static m() {} }")],
        vec![("client.ts", "export default class Client { m() {} m() {} }")],
        vec![(
            "client.ts",
            "export default class Client { m() {} } Client.prototype.m = other;",
        )],
        vec![(
            "patch.js",
            "import Client from './client'; const escaped = Client['prototype'];",
        )],
        vec![(
            "patch.js",
            "Object.defineProperty(target, 'm', {value: other});",
        )],
        vec![("patch.js", "Reflect['set'](target, 'm', other);")],
    ] {
        variants.push((APP.into(), PROPS.into(), extra));
    }
    let mut failures = Vec::new();
    for (i, (app, props, extra)) in variants.iter().enumerate() {
        for lang in [Language::TypeScript, Language::Tsx] {
            if std::panic::catch_unwind(|| {
                let files = sources(app, props, extra, lang);
                check(&CallGraph::build(&files), None);
                check(
                    &CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
                    None,
                );
            })
            .is_err()
            {
                failures.push((i, lang, app, props, extra));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "complete negative population: {failures:?}"
    );
}

#[test]
fn imported_object_alias_owner_replacement_and_missing_source() {
    let a = sources(
        APP,
        PROPS,
        &[("other.ts", "export default class Other { m() {} }")],
        Language::Tsx,
    );
    let mut b = a.clone();
    b.insert(
        "props.ts".into(),
        ParsedFile::parse(
            "props.ts",
            &PROPS.replace("'./client'", "'./other'"),
            Language::Tsx,
        )
        .unwrap(),
    );
    for (from, to, expected) in [(&a, &b, "other.ts"), (&b, &a, "client.ts")] {
        let mut graph = CallGraph::build(from);
        let changed = BTreeSet::from(["props.ts".to_string()]);
        graph.remove_files(&changed);
        graph.merge(CallGraph::build_direct_subset(to, &changed));
        check(&graph, Some(expected));
        check(&CallGraph::build(to), Some(expected));
    }
    let mut missing = a.clone();
    missing.remove("props.ts");
    for (from, to, expected) in [(&a, &missing, None), (&missing, &a, Some("client.ts"))] {
        let mut graph = CallGraph::build(from);
        let changed = BTreeSet::from(["props.ts".to_string()]);
        graph.remove_files(&changed);
        graph.merge(CallGraph::build_direct_subset(to, &changed));
        check(&graph, expected);
    }
}

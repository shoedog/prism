//! Endpoint checks at pre-outgoing-guard Step 5b.
use super::*;
use crate::resolution::go::satisfiers::CpgCallResolver;

struct Pool<'a>(&'a CallGraph, Vec<ResolvedCallee<'a>>);

impl CpgCallResolver for Pool<'_> {
    fn call_graph(&self) -> &CallGraph {
        self.0
    }

    fn resolve_call_site(&self, _: &CallSite) -> Vec<ResolvedCallee<'_>> {
        self.1.clone()
    }
}

fn span(source: &str, text: &str) -> (usize, usize) {
    let (start, _) = source.match_indices(text).next().unwrap();
    assert_eq!(source.matches(text).count(), 1, "{text:?} in {source:?}");
    (start, start + text.len())
}

pub(super) fn site<'a>(
    cpg: &'a CodePropertyGraph,
    file: &ParsedFile,
    call: &str,
    callee: &str,
) -> &'a CallSite {
    let bytes = span(&file.source, call);
    let mut sites = cpg.call_graph.calls.values().flatten().filter(|site| {
        site.caller.file == file.path
            && (site.start_byte, site.end_byte) == bytes
            && site.callee_name == callee
    });
    let site = sites
        .next()
        .unwrap_or_else(|| panic!("missing {call} at {bytes:?}"));
    assert!(sites.next().is_none(), "ambiguous {call} at {bytes:?}");
    site
}

pub(super) fn target<'a>(
    cpg: &'a CodePropertyGraph,
    file: &ParsedFile,
    owner: &str,
    implementation: &str,
) -> &'a FunctionId {
    let bytes = span(&file.source, implementation);
    let function = file
        .all_functions()
        .into_iter()
        .find(|n| (n.start_byte(), n.end_byte()) == bytes)
        .expect("declaration byte span");
    let expected = crate::anon_owner::identity(file, &function, &file.path).unwrap();
    assert_eq!(expected.name, owner);
    cpg.call_graph.functions[owner]
        .iter()
        .find(|target| **target == expected)
        .expect("independently identified declaration")
}

fn run(
    cpg: &CodePropertyGraph,
    files: &BTreeMap<String, ParsedFile>,
    site: &CallSite,
    pool: Vec<ResolvedCallee<'_>>,
) -> Vec<PendingArgumentFlow> {
    CodePropertyGraph::step5b_edges_for_caller_with_exact(
        &site.caller,
        &BTreeSet::from([site.clone()]),
        &Pool(&cpg.call_graph, pool),
        &cpg.var_index,
        &CodePropertyGraph::exact_var_index_from_graph(&cpg.graph),
        &cpg.graph,
        &ParamCache::new(files),
        spread_args::for_call,
    )
}

pub(super) type Endpoint<'a> = (
    &'a FunctionId,
    &'a str,
    (usize, usize),
    VarAccess,
    FlowConfidence,
);

pub(super) fn assert_bindings(
    cpg: &CodePropertyGraph,
    site: &CallSite,
    caller: &ParsedFile,
    edges: &[PendingArgumentFlow],
    expected: &[Endpoint<'_>],
) {
    let text = &caller.source[site.start_byte..site.end_byte];
    let argument = site.start_byte + text.rfind('v').unwrap();
    assert_eq!(
        edges.len(),
        expected.len(),
        "{text} at {}:{}: {:?}",
        caller.path,
        site.start_byte,
        edges
            .iter()
            .map(|flow| {
                let (from, to, ref edge) = flow.edge;
                (cpg.node(from), cpg.node(to), edge)
            })
            .collect::<Vec<_>>()
    );
    for (flow, &(owner, name, bytes, access, label)) in edges.iter().zip(expected) {
        let (from, to, ref edge) = flow.edge;
        assert_eq!(
            *edge,
            CpgEdge::DataFlow(label),
            "{text}: {:?}",
            cpg.node(to)
        );
        assert!(matches!(cpg.node(from), CpgNode::Variable {
            file, function, path, start_byte, end_byte, access: VarAccess::Use, ..
        } if file == &caller.path && function == &site.caller.name && path.base == "v"
            && (*start_byte, *end_byte) == (argument, argument + 1)));
        assert!(
            matches!(cpg.node(to), CpgNode::Variable {
            file, function, function_start_line, path, start_byte, end_byte, access: got, ..
        } if file == &owner.file && function == &owner.name && *function_start_line == owner.start_line
            && path.base == name && *got == access && (*start_byte, *end_byte) == bytes),
            "expected {owner:?} {name} {bytes:?} {access:?}, got {:?}",
            cpg.node(to)
        );
    }
}

pub(super) const BARE: &str = "p => { use(p.a); use(p.b); }";
const PAREN: &str = "(p) => { use(p.a); use(p.b); }";

pub(super) fn endpoint<'a>(parsed: &ParsedFile, owner: &'a FunctionId, leaf: bool) -> Endpoint<'a> {
    let (name, bytes, access) = if leaf {
        let start = span(&parsed.source, BARE).0 + BARE.find("p.a").unwrap();
        ("p", (start, start + 3), VarAccess::Use)
    } else {
        let start = span(&parsed.source, BARE).0;
        ("p", (start, start + 1), VarAccess::Def)
    };
    let label = if leaf {
        FlowConfidence::NameOnly(FlowDoubt::SameLine)
    } else {
        FlowConfidence::Exact
    };
    (owner, name, bytes, access, label)
}

#[test]
fn foreign_free_single_exact_refuses_the_ordinary_endpoint() {
    check_foreign_free_single(false);
}

#[test]
fn foreign_free_single_exact_refuses_the_leaf_endpoint() {
    check_foreign_free_single(true);
}

fn check_foreign_free_single(leaf: bool) {
    let files = parse(&BTreeMap::from([
        (
            "lib.js".into(),
            format!("export const take = {BARE};\nexport const fixed = {PAREN};"),
        ),
        (
            "loose.js".into(),
            "export function loose(v) { take(v); take({ a: v }); fixed(v); fixed({ a: v }); }"
                .into(),
        ),
        (
            "proved.js".into(),
            "import { take } from './lib';\nexport function proved(v) { take(v); take({ a: v }); }"
                .into(),
        ),
    ]));
    let cpg = CodePropertyGraph::build(&files);
    let lib = &files["lib.js"];
    let bare = target(&cpg, lib, "take", BARE);
    let fixed = target(&cpg, lib, "fixed", PAREN);
    for (file, callee, owner, route, admitted) in [
        ("loose.js", "take", bare, ResolutionKind::FreeSingle, false),
        (
            "proved.js",
            "take",
            bare,
            ResolutionKind::ImportMember,
            true,
        ),
        ("loose.js", "fixed", fixed, ResolutionKind::FreeSingle, true),
    ] {
        let caller = &files[file];
        let arg = if leaf { "{ a: v }" } else { "v" };
        let call = format!("{callee}({arg})");
        let site = site(&cpg, caller, &call, callee);
        let actual = cpg.call_graph.resolve_call_site(site);
        assert_eq!(actual.len(), 1, "{call}: {actual:?}");
        assert_eq!(
            (actual[0].target, actual[0].kind, actual[0].confidence),
            (owner, route, ResolutionConfidence::Exact)
        );
        let mut expected = Vec::new();
        if owner == fixed {
            let start = span(&lib.source, PAREN).0;
            let bytes = if leaf {
                let p = start + PAREN.find("p.a").unwrap();
                (p, p + 3)
            } else {
                (start + 1, start + 2)
            };
            let access = if leaf { VarAccess::Use } else { VarAccess::Def };
            let label = if leaf {
                FlowConfidence::NameOnly(FlowDoubt::SameLine)
            } else {
                FlowConfidence::Exact
            };
            expected.push((fixed, "p", bytes, access, label));
        } else if admitted {
            expected.push(endpoint(lib, bare, leaf));
        }
        assert_bindings(
            &cpg,
            site,
            caller,
            &run(&cpg, &files, site, actual),
            &expected,
        );
    }
}

#[test]
fn complete_exact_pool_refuses_the_local_ordinary_endpoint() {
    check_exact_pool(ResolutionKind::LocalDef, "take", false);
}

#[test]
fn complete_exact_pool_refuses_the_local_leaf_endpoint() {
    check_exact_pool(ResolutionKind::LocalDef, "take", true);
}

#[test]
fn complete_exact_pool_refuses_the_namespace_ordinary_endpoint() {
    check_exact_pool(ResolutionKind::ImportQualified, "api.take", false);
}

#[test]
fn complete_exact_pool_refuses_the_namespace_leaf_endpoint() {
    check_exact_pool(ResolutionKind::ImportQualified, "api.take", true);
}

fn check_exact_pool(route: ResolutionKind, spelling: &str, leaf: bool) {
    const OTHER: &str = "q => { use(q.a); use(q.b); }";
    let decl = format!("export const take = {BARE};");
    let (prefix, lib) = if route == ResolutionKind::LocalDef {
        (decl.clone(), "export {};".into())
    } else {
        ("import * as api from './lib';".into(), decl.clone())
    };
    let files = parse(&BTreeMap::from([
        ("m.js".into(), format!("{prefix}\nconst other = {OTHER};\nfunction main(v) {{ {spelling}(v); {spelling}({{ a: v }}); }}")),
        ("lib.js".into(), lib),
    ]));
    let cpg = CodePropertyGraph::build(&files);
    let caller = &files["m.js"];
    let definition = if route == ResolutionKind::LocalDef {
        caller
    } else {
        &files["lib.js"]
    };
    let callee = target(&cpg, definition, "take", BARE);
    let other = target(&cpg, caller, "other", OTHER);
    let arg = if leaf { "{ a: v }" } else { "v" };
    let site = site(&cpg, caller, &format!("{spelling}({arg})"), "take");
    let actual = cpg.call_graph.resolve_call_site(site);
    assert_eq!(actual.len(), 1, "{actual:?}");
    let one = actual[0].clone();
    assert_eq!(
        (one.target, one.kind, one.confidence),
        (callee, route, ResolutionConfidence::Exact)
    );
    let many = one.clone();
    let two = ResolvedCallee {
        target: other,
        ..one.clone()
    };
    let name_only = ResolvedCallee {
        confidence: ResolutionConfidence::NameOnly,
        ..two.clone()
    };
    let q = span(&caller.source, OTHER).0;
    let mixed = [(
        other,
        "q",
        (q, q + 1),
        VarAccess::Def,
        ResolutionConfidence::NameOnly.into(),
    )];
    for (pool, expected) in [
        (vec![one.clone()], &[endpoint(definition, callee, leaf)][..]),
        (vec![one.clone(), two], &[][..]),
        (vec![one.clone(), many], &[][..]),
        (
            vec![one, name_only],
            if leaf { &[][..] } else { &mixed[..] },
        ),
    ] {
        assert_bindings(&cpg, site, caller, &run(&cpg, &files, site, pool), expected);
    }
}

#[test]
fn namespace_export_witness_refuses_the_ordinary_endpoint() {
    check_namespace_export_witness(false);
}

#[test]
fn namespace_export_witness_refuses_the_leaf_endpoint() {
    check_namespace_export_witness(true);
}

fn check_namespace_export_witness(leaf: bool) {
    for (exports, admitted) in [
        ("export { readFile };", true),
        ("export { live as readFile };", false),
        ("export {};", false),
        ("export { readFile }; export { live as readFile };", false),
    ] {
        let files = parse(&BTreeMap::from([
            ("m.js".into(), "import * as api from './lib';\nfunction main(v) { api.readFile(v); api.readFile({ a: v }); }".into()),
            ("lib.js".into(), format!("const readFile = {BARE};\nfunction live() {{}}\n{exports}")),
        ]));
        let cpg = CodePropertyGraph::build(&files);
        let caller = &files["m.js"];
        let lib = &files["lib.js"];
        let owner = target(&cpg, lib, "readFile", BARE);
        let arg = if leaf { "{ a: v }" } else { "v" };
        let site = site(&cpg, caller, &format!("api.readFile({arg})"), "readFile");
        let actual = cpg.call_graph.resolve_call_site(site);
        assert_eq!(actual.len(), 1, "{exports}: {actual:?}");
        assert_eq!(
            (actual[0].target, actual[0].kind, actual[0].confidence),
            (
                owner,
                ResolutionKind::ImportQualified,
                ResolutionConfidence::Exact
            ),
            "{exports}"
        );
        let expected = [endpoint(lib, owner, leaf)];
        assert_bindings(
            &cpg,
            site,
            caller,
            &run(&cpg, &files, site, actual),
            if admitted { &expected } else { &[] },
        );
    }
}

#[test]
fn ordinary_bare_name_only_requires_the_same_file() {
    for (same_file, parenthesised, import) in [
        (true, false, false),
        (false, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        let arrow = if parenthesised { PAREN } else { BARE };
        let decl = if import {
            format!("export const take = {arrow};")
        } else {
            format!("export class Box {{ take = {arrow}; }}")
        };
        let call = if import { "take(v)" } else { "r.take(v)" };
        let prefix = if same_file {
            decl.as_str()
        } else if import {
            "import { take } from './lib';"
        } else {
            ""
        };
        let files = parse(&BTreeMap::from([
            (
                "m.js".into(),
                format!("{prefix}\nfunction main(r, v) {{ {call}; }}"),
            ),
            (
                "lib.js".into(),
                if same_file { "export {};".into() } else { decl },
            ),
        ]));
        let cpg = CodePropertyGraph::build(&files);
        let caller = &files["m.js"];
        let lib = if same_file { caller } else { &files["lib.js"] };
        let owner = target(&cpg, lib, "take", arrow);
        let site = site(&cpg, caller, call, "take");
        let actual = cpg.call_graph.resolve_call_site(site);
        assert_eq!(
            actual.len(),
            1,
            "{same_file}/{parenthesised}/{import}: {actual:?}"
        );
        let (kind, confidence) = if import {
            (ResolutionKind::ImportMember, ResolutionConfidence::Exact)
        } else {
            (
                ResolutionKind::R6SingleOwner,
                ResolutionConfidence::NameOnly,
            )
        };
        assert_eq!(
            (actual[0].target, actual[0].kind, actual[0].confidence),
            (owner, kind, confidence)
        );
        let start = span(&lib.source, arrow).0 + usize::from(parenthesised);
        let expected = [(
            owner,
            "p",
            (start, start + 1),
            VarAccess::Def,
            confidence.into(),
        )];
        assert_bindings(
            &cpg,
            site,
            caller,
            &run(&cpg, &files, site, actual),
            if same_file || parenthesised || import {
                &expected
            } else {
                &[]
            },
        );
    }
}

#[path = "param_binding_bare_identity_tests.rs"]
mod identity;

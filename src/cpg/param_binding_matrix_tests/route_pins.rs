//! Resolver prerequisites captured on the Slice C base, before formal lookup.
use super::*;
use crate::ast::ParsedFile;
use crate::resolution::{
    DropReason, DropReason::UnknownName, ResolutionConfidence, ResolutionConfidence::*,
};

type Target = (
    &'static str,
    &'static str,
    &'static str,
    ResolutionConfidence,
    (usize, usize),
);
type Route = (
    &'static str,
    &'static str,
    (usize, usize),
    &'static str,
    &'static [Target],
    Option<DropReason>,
);

const ROUTES: &[Route] = &[
    (
        "k01a",
        "m.js",
        (46, 50),
        "g",
        &[("m.js", "g", "local_def", Exact, (10, 23))],
        None,
    ),
    (
        "k01b",
        "m.js",
        (52, 56),
        "g",
        &[("m.js", "g", "local_def", Exact, (10, 29))],
        None,
    ),
    (
        "k05a",
        "m.js",
        (55, 66),
        "f",
        &[("m.js", "f", "local_def", Exact, (10, 32))],
        None,
    ),
    (
        "k06a",
        "b.js",
        (39, 50),
        "parseInt",
        &[("a.js", "parseInt", "free_single", Exact, (7, 54))],
        None,
    ),
    (
        "k06b",
        "b.js",
        (54, 63),
        "escape",
        &[("a.js", "escape", "free_single", Exact, (77, 99))],
        None,
    ),
    (
        "k06c",
        "b.js",
        (67, 83),
        "escape",
        &[("a.js", "escape", "free_single", Exact, (77, 99))],
        None,
    ),
    (
        "k07a",
        "b.js",
        (86, 95),
        "escape",
        &[("a.js", "escape", "import_member", Exact, (77, 99))],
        None,
    ),
    (
        "k07a",
        "b.js",
        (99, 115),
        "escape",
        &[("a.js", "escape", "import_member", Exact, (77, 99))],
        None,
    ),
    (
        "k07b",
        "m.js",
        (123, 132),
        "escape",
        &[("m.js", "escape", "local_def", Exact, (63, 85))],
        None,
    ),
    (
        "k08a",
        "m.js",
        (70, 79),
        "id",
        &[("m.js", "id", "local_def", Exact, (11, 31))],
        None,
    ),
    (
        "k08b",
        "m.js",
        (79, 97),
        "id",
        &[("m.js", "id", "local_def", Exact, (11, 31))],
        None,
    ),
    (
        "t4-4",
        "m.tsx",
        (83, 96),
        "render",
        &[("m.tsx", "render", "local_def", Exact, (15, 56))],
        None,
    ),
    (
        "s1-9",
        "m.js",
        (110, 119),
        "id",
        &[("m.js", "id", "local_def", Exact, (76, 106))],
        None,
    ),
    (
        "s1-10",
        "main.js",
        (72, 85),
        "invoke",
        &[("a.js", "project", "import_member", Exact, (23, 47))],
        None,
    ),
    (
        "s1-11",
        "m.js",
        (118, 132),
        "project",
        &[("m.js", "project", "local_def", Exact, (66, 90))],
        None,
    ),
    (
        "s2-8",
        "b.js",
        (54, 67),
        "invoke",
        &[("a.js", "project", "import_member", Exact, (16, 40))],
        None,
    ),
    ("s3-7", "b.js", (53, 65), "apply", &[], Some(UnknownName)),
    (
        "s3-8",
        "b.js",
        (56, 74),
        "project",
        &[("a.js", "project", "import_qualified", Exact, (23, 47))],
        None,
    ),
    (
        "s3-9",
        "b.js",
        (25, 39),
        "project",
        &[("a.js", "project", "free_single", Exact, (14, 38))],
        None,
    ),
    (
        "s4-6",
        "m.js",
        (120, 131),
        "sync",
        &[("m.js", "sync", "local_def", Exact, (13, 39))],
        None,
    ),
    (
        "s4-6",
        "m.js",
        (133, 146),
        "later",
        &[("m.js", "later", "local_def", Exact, (55, 85))],
        None,
    ),
    (
        "s4-7",
        "caller.js",
        (63, 77),
        "project",
        &[("api.js", "project", "import_member", Exact, (16, 40))],
        None,
    ),
    (
        "s4-8",
        "b.ts",
        (127, 141),
        "project",
        &[],
        Some(UnknownName),
    ),
    (
        "s5-9",
        "m.js",
        (144, 166),
        "project",
        &[
            ("m.js", "project", "self_receiver", NameOnly, (31, 63)),
            ("m.js", "project", "self_receiver", NameOnly, (77, 107)),
        ],
        None,
    ),
    (
        "s5-9",
        "m.js",
        (172, 196),
        "project",
        &[
            ("m.js", "project", "qualifier_owner", NameOnly, (31, 63)),
            ("m.js", "project", "qualifier_owner", NameOnly, (77, 107)),
        ],
        None,
    ),
    (
        "s5-11",
        "m.js",
        (67, 86),
        "proj\\u0065ct",
        &[],
        Some(UnknownName),
    ),
    (
        "s5-12",
        "m.js",
        (119, 133),
        "project",
        &[("m.js", "project", "local_def", Exact, (67, 91))],
        None,
    ),
    (
        "x2-6",
        "m.js",
        (31, 67),
        "(item => {\n    use(item);\n  })",
        &[],
        Some(UnknownName),
    ),
    (
        "x2-6",
        "m.js",
        (69, 106),
        "(item => {\n    use(item);\n  })",
        &[],
        Some(UnknownName),
    ),
    (
        "x2-7",
        "m.js",
        (92, 103),
        "local",
        &[("lib.js", "parseInt", "import_member", Exact, (7, 47))],
        None,
    ),
    (
        "x2-7",
        "m.js",
        (153, 167),
        "parseInt",
        &[("lib.js", "parseInt", "free_single", Exact, (7, 47))],
        None,
    ),
    (
        "x3-6",
        "m.js",
        (92, 131),
        "take",
        &[("m.js", "take", "local_def", Exact, (13, 60))],
        None,
    ),
    (
        "x3-7",
        "m.ts",
        (165, 176),
        "send",
        &[("m.ts", "send", "local_def", Exact, (79, 118))],
        None,
    ),
    (
        "x3-7",
        "m.ts",
        (244, 255),
        "send",
        &[
            ("m.ts", "send", "local_def", Exact, (13, 48)),
            ("m.ts", "send", "local_def", Exact, (79, 118)),
        ],
        None,
    ),
    (
        "x4-5",
        "m.js",
        (68, 79),
        "send",
        &[("source.js", "visit", "import_member", Exact, (21, 45))],
        None,
    ),
    (
        "x4-6",
        "m.ts",
        (104, 116),
        "visit",
        &[("lib.ts", "visit", "import_member", Exact, (21, 45))],
        None,
    ),
];

#[test]
fn param_binding_matrix_slice_c_routes() {
    let rows = read_rows(ROWS).unwrap();
    for &(id, file, bytes, callee, pool, drop) in ROUTES {
        let row = rows.iter().find(|row| row.id == id).unwrap();
        for ext in &row.langs {
            let path_for = |key: &str| format!("{}.{ext}", key.rsplit_once('.').unwrap().0);
            let files = row
                .files
                .iter()
                .map(|(key, source)| {
                    let path = path_for(key);
                    let parsed = ParsedFile::parse(&path, source, language(ext).unwrap()).unwrap();
                    (path, parsed)
                })
                .collect::<BTreeMap<_, _>>();
            let path = path_for(file);
            let cg = crate::call_graph::CallGraph::build(&files);
            let mut sites = cg.calls.values().flatten().filter(|site| {
                site.caller.file == path
                    && (site.start_byte, site.end_byte) == bytes
                    && site.callee_name == callee
            });
            let context = format!("{id}/{ext} {path}:{bytes:?} {callee}");
            let site = sites.next().unwrap_or_else(|| panic!("missing {context}"));
            assert!(sites.next().is_none(), "duplicate {context}");
            let actual = cg.resolve_call_site_full(site);
            assert_eq!(actual.drop, drop, "{context}");
            assert_eq!(
                actual.resolved.len(),
                pool.len(),
                "{context}: {:?}",
                actual.resolved
            );
            for (resolved, &(file, owner, route, confidence, declaration)) in
                actual.resolved.iter().zip(pool)
            {
                let file = path_for(file);
                assert_eq!(
                    (
                        &resolved.target.file,
                        resolved.target.name.as_str(),
                        resolved.kind.as_str(),
                        resolved.confidence
                    ),
                    (&file, owner, route, confidence),
                    "{context}"
                );
                let parsed = &files[&file];
                let spans: Vec<_> = parsed
                    .all_functions()
                    .into_iter()
                    .filter(|node| {
                        crate::anon_owner::identity(parsed, node, &file).as_ref()
                            == Some(resolved.target)
                    })
                    .map(|node| (node.start_byte(), node.end_byte()))
                    .collect();
                assert_eq!(spans, [declaration], "{context}: {file}:{owner}");
            }
        }
    }
}

#[test]
fn param_binding_matrix_s5_9_crossed_edges_are_nameonly() {
    let rows = read_rows(ROWS).unwrap();
    let row = rows.iter().find(|row| row.id == "s5-9").unwrap();
    for ext in &row.langs {
        let fixture = row.fixture(ext);
        for ordinal in [5, 6] {
            let assertion = &row.expect[ordinal - 1];
            let Check::NoEdge {
                from,
                to: Some(to),
                label: None,
            } = &assertion.check
            else {
                panic!("s5-9 #{ordinal} must assert unconditional absence");
            };
            assert!(assertion.loss.is_some(), "s5-9 #{ordinal} must stay a loss");
            assert_eq!(
                fixture.edge_labels(Some(from), Some(to), true).unwrap(),
                ["nameonly/call_nameonly"],
                "s5-9/{ext} #{ordinal}"
            );
        }
    }
}

#[test]
fn param_binding_matrix_k08b_has_a_represented_surplus_use() {
    let rows = read_rows(ROWS).unwrap();
    let row = rows.iter().find(|row| row.id == "k08b").unwrap();
    let source = &row.files["m.js"];
    let start = source.rfind("unbound").unwrap();
    for ext in &row.langs {
        let fixture = row.fixture(ext);
        let uses: Vec<_> = fixture
            .cpg
            .graph
            .node_indices()
            .filter(|&idx| {
                fixture.cpg.to_var_location(idx).is_some_and(|loc| {
                    loc.function == "main"
                        && loc.kind == VarAccessKind::Use
                        && loc.path.base == "unbound"
                        && (loc.start_byte, loc.end_byte) == (start, start + "unbound".len())
                })
            })
            .collect();
        assert_eq!(uses.len(), 1, "{ext}: missing surplus RHS Use");
        assert!(
            !fixture.cpg.graph.edges(uses[0]).any(|edge| {
                edge.weight().is_data_flow()
                    && fixture
                        .cpg
                        .to_var_location(edge.target())
                        .is_some_and(|loc| loc.function == "id")
            }),
            "{ext}: surplus Use bound to id"
        );
    }
}

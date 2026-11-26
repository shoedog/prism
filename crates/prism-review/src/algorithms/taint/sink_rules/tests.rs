use super::evidence::{
    Bound, Detector, InlineRoute, InlineVisit, Occurrence, PathProof, PathRoute, Provenance, Role,
};
use super::{collect_rules, FindingKey, NameSinks, Selection};
use crate::algorithms::taint::cleansing::{flow_path_cleansed_for_sink_call, node_in_ranges};
use prism::access_path::AccessPath;
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
use prism::frameworks::{SanitizerCategory, SinkPattern};
use prism::languages::Language;
use prism::rule_registry::{CONFIGURED_SINK, FORMAT_WRAPPER, JS_EVAL, TAINT_WRITE_DATA};
use prism::slice::{attach_rules, SliceFinding};
use std::collections::BTreeSet;
use std::iter::successors;
use tree_sitter::Node;

const FLAT: Detector<'static> = Detector::Flat("fixture");
const STRUCTURED: Detector<'static> = Detector::Structured {
    table: "fixture",
    entry: 0,
};

fn node<'a>(parsed: &'a ParsedFile, scope: Node<'a>, text: &str) -> Node<'a> {
    parsed
        .node_text(&scope)
        .match_indices(text)
        .find_map(|(offset, _)| {
            let start = scope.start_byte() + offset;
            let range = start..start + text.len();
            scope
                .named_descendant_for_byte_range(range.start, range.end)
                .filter(|candidate| candidate.byte_range() == range)
        })
        .unwrap_or_else(|| panic!("fixture node {text:?} in {:?}", parsed.node_text(&scope)))
}

fn site<'a>(
    parsed: &'a ParsedFile,
    operation: &str,
    target: &str,
    operand: &str,
    role: Role<'a>,
) -> Bound<'a> {
    let operation = node(parsed, parsed.tree.root_node(), operation);
    Bound::from_binder(
        operation,
        node(parsed, operation, target),
        node(parsed, operation, operand),
        role,
        FLAT,
    )
}

fn rebind<'a>(bound: Bound<'a>, operand: Node<'a>, role: Role<'a>) -> Bound<'a> {
    Bound::from_binder(
        bound.operation(),
        bound.target(),
        operand,
        role,
        bound.detector(),
    )
}

fn owner<'a>(parsed: &'a ParsedFile, operation: Node<'a>) -> Node<'a> {
    let boundaries = parsed.language.callable_boundary_node_types();
    successors(Some(operation), Node::parent)
        .find(|candidate| boundaries.contains(&candidate.kind()))
        .unwrap_or_else(|| parsed.tree.root_node())
}

fn receipt<'a, 'p>(
    parsed: &'a ParsedFile,
    line: usize,
    site: Bound<'a>,
    reached: Node<'a>,
    detector: Detector<'a>,
    provenance: Provenance<'a, 'p>,
) -> Occurrence<'a, 'p> {
    let bound = Bound::from_binder(
        site.operation(),
        site.target(),
        site.operand(),
        site.role(),
        detector,
    );
    Occurrence::from_adapter(
        parsed,
        (&parsed.path, line),
        owner(parsed, site.operation()),
        bound,
        reached,
        provenance,
    )
}

fn owner_line(parsed: &ParsedFile, node: Node<'_>) -> usize {
    let owner = owner(parsed, node);
    if owner == parsed.tree.root_node() {
        1
    } else {
        owner.start_position().row + 1
    }
}

fn raw_from<'a, 'p>(
    parsed: &'a ParsedFile,
    key: FindingKey<'a>,
    owner: Node<'a>,
    site: Bound<'a>,
    reached: Node<'a>,
    path: &'p FlowPath,
) -> Occurrence<'a, 'p> {
    Occurrence::from_adapter(parsed, key, owner, site, reached, destination(path))
}

fn path_to(parsed: &ParsedFile, definition: Node<'_>, reached: Node<'_>) -> FlowPath {
    let location = |node: Node<'_>, kind| VarLocation {
        file: parsed.path.clone(),
        function: "f".into(),
        function_start_line: owner_line(parsed, node),
        line: node.start_position().row + 1,
        path: AccessPath::simple(parsed.node_text(&definition)),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        kind,
    };
    FlowPath {
        edges: vec![FlowEdge {
            from: location(definition, VarAccessKind::Def),
            to: location(reached, VarAccessKind::Use),
        }],
        cleansed_for: BTreeSet::new(),
    }
}

fn destination<'a, 'p>(path: &'p FlowPath) -> Provenance<'a, 'p> {
    Provenance::Path {
        path,
        edge_index: 0,
        route: PathRoute::Destination,
        proof: PathProof::ExactPoint,
    }
}

fn admitted(parsed: &ParsedFile, lines: &[usize]) -> BTreeSet<(String, usize)> {
    lines
        .iter()
        .map(|&line| (parsed.path.clone(), line))
        .collect()
}

fn select_eval(_: &Occurrence<'_, '_>, rows: &mut Vec<Selection>) {
    rows.push((&JS_EVAL, false));
}

#[test]
fn combines_paths_and_inline_on_final_admitted_key() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { db.query('select ?', inner(x), y); db.query('select ?', {bind: x}); inline(source()); }",
        Language::JavaScript,
    ).unwrap();
    let inner = site(&parsed, "inner(x)", "inner", "x", Role::Argument(0));
    let query = site(
        &parsed,
        "db.query('select ?', inner(x), y)",
        "query",
        "inner(x)",
        Role::Argument(1),
    );
    let y = rebind(
        query,
        node(&parsed, query.operation(), "y"),
        Role::Argument(2),
    );
    let bound = site(
        &parsed,
        "db.query('select ?', {bind: x})",
        "query",
        "{bind: x}",
        Role::Argument(1),
    );
    let inline = site(
        &parsed,
        "inline(source())",
        "inline",
        "source()",
        Role::Argument(0),
    );
    let x_def = node(&parsed, parsed.tree.root_node(), "x");
    let y_def = node(&parsed, parsed.tree.root_node(), "y");
    let x = path_to(&parsed, x_def, inner.operand());
    let y_live = path_to(&parsed, y_def, y.operand());
    let mut y_cleansed = y_live.clone();
    y_cleansed.cleansed_for.insert(SanitizerCategory::Sqli);
    let bound_read = node(&parsed, bound.operand(), "x");
    let bound_path = path_to(&parsed, x_def, bound_read);
    let make_witnesses = || {
        [
            receipt(&parsed, 1, inner, inner.operand(), FLAT, destination(&x)),
            receipt(&parsed, 1, inner, inner.operand(), FLAT, destination(&x)),
            receipt(
                &parsed,
                1,
                query,
                inner.operand(),
                STRUCTURED,
                destination(&x),
            ),
            receipt(
                &parsed,
                1,
                y,
                y.operand(),
                STRUCTURED,
                destination(&y_cleansed),
            ),
            receipt(&parsed, 1, y, y.operand(), STRUCTURED, destination(&y_live)),
            receipt(
                &parsed,
                1,
                bound,
                bound_read,
                STRUCTURED,
                destination(&bound_path),
            ),
            receipt(
                &parsed,
                1,
                inline,
                inline.operand(),
                STRUCTURED,
                Provenance::Inline {
                    visit: InlineVisit::from_visit(
                        InlineRoute::JsYaml,
                        inline.operation(),
                        (&parsed.path, 1),
                        (&parsed.path, 1),
                    ),
                },
            ),
        ]
    };
    assert!(collect_rules(&parsed, 1, &BTreeSet::new(), make_witnesses(), select_eval).is_empty());

    static QUERY: SinkPattern = SinkPattern {
        call_path: "query",
        category: SanitizerCategory::Sqli,
        tainted_arg_indices: &[1, 2],
        semantic_check: None,
    };
    let cpg = CodePropertyGraph::empty();
    let mut paths = Vec::new();
    let rows = collect_rules(
        &parsed,
        1,
        &admitted(&parsed, &[1]),
        make_witnesses(),
        |witness, rows| {
            let site = witness.site();
            match witness.provenance() {
                Provenance::Path { path, .. } => {
                    paths.push(path as *const FlowPath);
                    if witness.detector() == FLAT {
                        let callee = (site.target().start_byte(), site.target().end_byte());
                        let operand = (site.operand().start_byte(), site.operand().end_byte());
                        assert!(node_in_ranges(&site.target(), &[callee]));
                        assert!(!node_in_ranges(&site.target(), &[operand]));
                        rows.push((&JS_EVAL, true));
                    } else if site.operand() != bound.operand()
                        && !flow_path_cleansed_for_sink_call(
                            &parsed,
                            &cpg,
                            path,
                            1,
                            &QUERY,
                            &site.operation(),
                        )
                    {
                        rows.push((&TAINT_WRITE_DATA, false));
                    }
                }
                Provenance::Inline { .. } => {
                    assert_eq!(parsed.node_text(&witness.reached()), "source()");
                    rows.push((&JS_EVAL, false));
                }
            }
        },
    );
    assert_eq!(
        paths,
        [&x, &x, &x, &y_cleansed, &y_live, &bound_path].map(std::ptr::from_ref)
    );
    assert_eq!(rows.len(), 5);
    let mut finding = SliceFinding {
        severity: "warning".into(),
        ..SliceFinding::default()
    };
    attach_rules(&mut finding, rows);
    assert_eq!(
        finding.rules,
        [JS_EVAL.rule_ref(), TAINT_WRITE_DATA.rule_ref()]
    );
}

#[test]
fn configured_and_wrapper_origins_remain_independent_and_ordered() {
    let parsed = ParsedFile::parse(
        "a.py",
        "def f(x):\n    x.report('ok')\n    report_extra(x); report('%s', x)\n",
        Language::Python,
    )
    .unwrap();
    let configured = ["report".into(), "=report".into()];
    let wrappers = ["=report".into()];
    let names = NameSinks::new(&configured, &wrappers);
    assert_eq!(
        names.admission_patterns().collect::<Vec<_>>(),
        ["report", "=report", "=report"]
    );
    let extra = site(
        &parsed,
        "report_extra(x)",
        "report_extra",
        "x",
        Role::Argument(0),
    );
    let wrapper = site(&parsed, "report('%s', x)", "report", "x", Role::Argument(1));
    let receiver = site(
        &parsed,
        "x.report('ok')",
        "report",
        "x",
        Role::ReceiverValue,
    );
    let def = node(&parsed, parsed.tree.root_node(), "x");
    let path = path_to(&parsed, def, extra.operand());
    let wrapper_path = path_to(&parsed, def, wrapper.operand());
    let receiver_path = path_to(&parsed, def, receiver.operand());
    let rows = collect_rules(
        &parsed,
        2,
        &admitted(&parsed, &[2, 3]),
        [receipt(
            &parsed,
            2,
            receiver,
            receiver.operand(),
            Detector::Configured("report"),
            destination(&receiver_path),
        )],
        |witness, rows| names.select(witness, rows),
    );
    assert!(rows.is_empty());

    let mut wrong_file = path.clone();
    wrong_file.edges[0].to.file = "other.py".into();
    let rows = collect_rules(
        &parsed,
        3,
        &admitted(&parsed, &[2, 3]),
        [
            receipt(
                &parsed,
                3,
                extra,
                extra.operand(),
                Detector::Configured("report"),
                destination(&wrong_file),
            ),
            receipt(
                &parsed,
                3,
                extra,
                extra.operand(),
                Detector::Configured("report"),
                destination(&path),
            ),
            receipt(
                &parsed,
                3,
                extra,
                extra.operand(),
                Detector::Configured("=report"),
                destination(&path),
            ),
            receipt(
                &parsed,
                3,
                wrapper,
                wrapper.operand(),
                Detector::Configured("=report"),
                destination(&wrapper_path),
            ),
            receipt(
                &parsed,
                3,
                wrapper,
                wrapper.operand(),
                Detector::Wrapper("=report"),
                destination(&wrapper_path),
            ),
            receipt(
                &parsed,
                3,
                wrapper,
                wrapper.operand(),
                Detector::Wrapper("report"),
                destination(&wrapper_path),
            ),
        ],
        |witness, rows| names.select(witness, rows),
    );
    assert_eq!(
        rows.iter()
            .map(|(row, weak)| (row.id(), *weak))
            .collect::<Vec<_>>(),
        [
            (CONFIGURED_SINK.id(), false),
            (CONFIGURED_SINK.id(), false),
            (FORMAT_WRAPPER.id(), false)
        ]
    );
}

#[test]
fn receipt_checks_parse_key_range_owner_and_operand_membership() {
    let source = "function f(x) { audit('safe', x); other(x); audit(() => x); }\n";
    let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let other_parse = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let copied_tree = parsed.tree.clone();
    let audit = site(&parsed, "audit('safe', x)", "audit", "x", Role::Argument(1));
    let sibling = site(&parsed, "other(x)", "other", "x", Role::Argument(0));
    let deferred = site(
        &parsed,
        "audit(() => x)",
        "audit",
        "() => x",
        Role::Argument(0),
    );
    let wrong_slot = rebind(
        audit,
        node(&parsed, audit.operation(), "'safe'"),
        Role::Argument(0),
    );
    let def = node(&parsed, parsed.tree.root_node(), "x");
    let path = path_to(&parsed, def, audit.operand());
    let key = (parsed.path.as_str(), 1);
    let callable = owner(&parsed, audit.operation());
    let range = audit.operand().byte_range();
    let cloned_read = copied_tree
        .root_node()
        .named_descendant_for_byte_range(range.start, range.end)
        .unwrap();
    assert_eq!(cloned_read.id(), audit.operand().id());
    let shifted_root = parsed
        .tree
        .root_node_with_offset(0, tree_sitter::Point::new(1, 0));
    let shifted_read = shifted_root
        .named_descendant_for_byte_range(range.start, range.end)
        .unwrap();
    assert_eq!(shifted_read.id(), audit.operand().id());
    assert_ne!(shifted_read.range(), audit.operand().range());
    let mut truncated = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let truncated_root = truncated.tree.root_node();
    truncated.source.truncate(12);
    let in_truncated_tree = |node: Node<'_>| {
        let range = node.byte_range();
        truncated_root
            .named_descendant_for_byte_range(range.start, range.end)
            .unwrap()
    };
    let from = |key, owner, site, reached| raw_from(&parsed, key, owner, site, reached, &path);
    let candidates = [
        from(("other.js", 1), callable, audit, audit.operand()),
        from((&parsed.path, 2), callable, audit, audit.operand()),
        from(key, parsed.tree.root_node(), audit, audit.operand()),
        from(key, callable, audit, sibling.operand()),
        from(key, callable, wrong_slot, audit.operand()),
        from(
            key,
            callable,
            deferred,
            node(&parsed, deferred.operand(), "x"),
        ),
        from(key, callable, audit, cloned_read),
        receipt(
            &other_parse,
            1,
            site(
                &other_parse,
                "audit('safe', x)",
                "audit",
                "x",
                Role::Argument(1),
            ),
            node(&other_parse, other_parse.tree.root_node(), "x"),
            FLAT,
            destination(&path),
        ),
    ];
    assert!(collect_rules(
        &parsed,
        1,
        &admitted(&parsed, &[1]),
        candidates,
        select_eval
    )
    .is_empty());
    let mut shifted_path = path.clone();
    shifted_path.edges[0].to.line = 2;
    let shifted = receipt(
        &parsed,
        2,
        audit,
        shifted_read,
        FLAT,
        destination(&shifted_path),
    );
    assert!(collect_rules(&parsed, 2, &admitted(&parsed, &[2]), [shifted], select_eval).is_empty());
    let truncated_site = Bound::from_binder(
        in_truncated_tree(audit.operation()),
        in_truncated_tree(audit.target()),
        in_truncated_tree(audit.operand()),
        audit.role(),
        audit.detector(),
    );
    let invalid = receipt(
        &truncated,
        1,
        truncated_site,
        truncated_site.operand(),
        FLAT,
        destination(&path),
    );
    assert!(collect_rules(
        &truncated,
        1,
        &admitted(&truncated, &[1]),
        [invalid],
        select_eval
    )
    .is_empty());
    assert_eq!(
        collect_rules(
            &parsed,
            1,
            &admitted(&parsed, &[1]),
            [from(key, callable, audit, audit.operand())],
            select_eval
        )
        .len(),
        1
    );
}

mod callee;
mod certificate_rows;
mod fixtures;
mod receipt;
mod routes;

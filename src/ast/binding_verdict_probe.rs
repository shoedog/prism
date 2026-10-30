//! Operation-only repetition probe. No clocks or process/resource sampling.
use super::*;
use crate::access_path::AccessPath;
use crate::ast::member_reference_index;
use crate::languages::Language;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;

#[derive(Default)]
struct Counts {
    total: usize,
    distinct: std::collections::HashSet<(usize, u8, String)>,
}
thread_local! {
    static COUNTS: RefCell<Option<HashMap<&'static str, Counts>>> = const { RefCell::new(None) };
}
pub(crate) fn observe(kind: &'static str, node: usize, region: u8, name: &str) {
    COUNTS.with(|slot| {
        if let Some(counts) = slot.borrow_mut().as_mut() {
            let c = counts.entry(kind).or_default();
            c.total += 1;
            c.distinct.insert((node, region, name.into()));
        }
    });
}
pub(crate) fn active() -> bool {
    COUNTS.with(|slot| slot.borrow().is_some())
}
pub(crate) fn count_body_walks(run: impl FnOnce()) -> usize {
    count_walks("function_body", run)
}
pub(crate) fn count_scope_walks(run: impl FnOnce()) -> usize {
    count_walks("scope_names", run)
}
fn count_walks(kind: &'static str, run: impl FnOnce()) -> usize {
    struct Restore(Option<HashMap<&'static str, Counts>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            COUNTS.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(COUNTS.with(|slot| slot.replace(Some(HashMap::new()))));
    run();
    COUNTS.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|counts| counts.get(kind))
            .map_or(0, |counts| counts.total)
    })
}
pub(crate) fn gate(kind: &'static str, node: usize, name: impl FnOnce() -> String) {
    if active() {
        observe(kind, node, 0, &name());
    }
}

#[test]
fn gate_identity_is_lazy_and_active_probe_still_records() {
    gate("gate", 7, || panic!("inactive probe allocated an identity"));
    COUNTS.with(|slot| *slot.borrow_mut() = Some(HashMap::new()));
    gate("gate", 7, || "identity".into());
    let counts = COUNTS.with(|slot| slot.borrow_mut().take().unwrap());
    assert_eq!(counts["gate"].total, 1);
    assert_eq!(counts["gate"].distinct.len(), 1);
}
pub(crate) fn scope(p: &ParsedFile, scope: Node<'_>, boundaries: &[&str], name: &str, byte: usize) {
    if !active() {
        return;
    }
    let region = if boundaries.contains(&scope.kind()) {
        let inside =
            |n: Option<Node<'_>>| n.is_some_and(|n| n.start_byte() <= byte && byte < n.end_byte());
        u8::from(inside(scope.child_by_field_name("body")))
            | u8::from(inside(p.parameter_binding_region(&scope))) << 1
            | u8::from(inside(scope.child_by_field_name("name"))) << 2
            | u8::from(inside(scope.child_by_field_name("return_type"))) << 3
    } else {
        0
    };
    observe("scope_all", scope.id(), region, name);
    if boundaries.contains(&scope.kind())
        || matches!(
            scope.kind(),
            "statement_block"
                | "switch_body"
                | "enum_body"
                | "class"
                | "class_declaration"
                | "abstract_class_declaration"
                | "class_static_block"
                | "for_statement"
                | "for_in_statement"
                | "catch_clause"
        )
    {
        observe("scope", scope.id(), region, name);
    }
}

#[test]
#[ignore = "explicit operation-count fixture replay; PRISM_BINDING_FIXTURE required"]
fn repetition_probe() {
    let path = std::env::var("PRISM_BINDING_FIXTURE").unwrap();
    let source = std::fs::read_to_string(&path).unwrap();
    let p = ParsedFile::parse(&path, &source, Language::TypeScript).unwrap();
    COUNTS.with(|slot| *slot.borrow_mut() = Some(HashMap::new()));
    let search = member_reference_index::MemberReferenceSearch::new(&p);
    let limit = std::env::var("PRISM_BINDING_LIMIT")
        .ok()
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(usize::MAX);
    let mut searches = 0;
    let mut gates = 0;
    'owners: for owner in p.all_functions() {
        if std::env::var("PRISM_BINDING_OWNER")
            .ok()
            .is_some_and(|name| p.dfg_owner_name(&owner).as_deref() != Some(&name))
        {
            continue;
        }
        let (start, end) = p.node_line_range(&owner);
        let spans = p.assignment_lvalue_spans_on_lines(&owner, &(start..=end).collect());
        for span in spans {
            let within =
                crate::profile::gates::def_scope(&p, &owner, &span.path.base, span.start_byte);
            let refs = search.find_path_reference_spans_fenced_in(
                &owner,
                &span.path,
                span.line,
                span.start_byte,
                within,
                false,
            );
            gates += refs.len();
            searches += 1;
            if searches >= limit {
                break 'owners;
            }
        }
    }
    let counts = COUNTS.with(|slot| slot.borrow_mut().take().unwrap());
    let rows: BTreeMap<_, _> = counts
        .into_iter()
        .map(|(k, c)| {
            (
                k,
                serde_json::json!({"total":c.total,"distinct":c.distinct.len()}),
            )
        })
        .collect();
    println!(
        "{}",
        serde_json::json!({"fixture":path,"searches":searches,"admitted_lines":gates,"operations":rows})
    );
    assert!(searches > 0);
}

#[test]
fn binding_work_is_per_scope_not_per_gate() {
    for n in [32, 128] {
        let source = format!(
            "function owner(obj) {{\n obj.x = 1;\n{}\n}}",
            " consume(obj.x);\n".repeat(n)
        );
        let p = ParsedFile::parse("scale.ts", &source, Language::TypeScript).unwrap();
        let owner = p.all_functions()[0];
        let search = member_reference_index::MemberReferenceSearch::new(&p);
        COUNTS.with(|slot| *slot.borrow_mut() = Some(HashMap::new()));
        for _ in 0..2 {
            let refs = search.find_path_reference_spans_fenced_in(
                &owner,
                &AccessPath::from_expr("obj.x"),
                2,
                source.find("obj.x").unwrap(),
                None,
                false,
            );
            assert_eq!(refs.len(), n);
        }
        let counts = COUNTS.with(|slot| slot.borrow_mut().take().unwrap());
        assert_eq!(counts["fence"].total, 2 * n);
        assert!(
            counts["scope"].total <= 2,
            "{n} reads: {} binder evaluations for {} gate calls",
            counts["scope"].total,
            counts["fence"].total
        );
        assert!(counts["seam"].total <= 1);
    }
}

#[test]
fn negative_sub_results_are_cached() {
    let p = ParsedFile::parse(
        "negative.ts",
        "function f(x) { let y = x; return y; }",
        Language::TypeScript,
    )
    .unwrap();
    let owner = p.all_functions()[0];
    let body = owner.child_by_field_name("body").unwrap();
    let byte = p.source.rfind('y').unwrap();
    let boundaries = p.language.callable_boundary_node_types();
    let _session = Session::new(&p);
    COUNTS.with(|slot| *slot.borrow_mut() = Some(HashMap::new()));
    let check = || {
        assert_eq!(p.js_ts_binding_scope_at(byte, "missing"), None);
        assert!(!p.js_ts_scope_binds(owner, &boundaries, "missing", byte + 1));
        assert!(!p.js_ts_scope_binds(body, &boundaries, "missing", byte + 1));
        assert!(!p.js_ts_seam_binding(&owner, "missing"));
        assert!(!p.scope_has_declaration(body, "missing"));
    };
    check();
    let first: BTreeMap<_, _> = COUNTS.with(|slot| {
        slot.borrow()
            .as_ref()
            .unwrap()
            .iter()
            .map(|(k, v)| (*k, v.total))
            .collect()
    });
    for _ in 0..16 {
        check();
    }
    let counts = COUNTS.with(|slot| slot.borrow_mut().take().unwrap());
    let last: BTreeMap<_, _> = counts.iter().map(|(k, v)| (*k, v.total)).collect();
    assert_eq!(
        last, first,
        "negative hits must not evaluate any binder internals"
    );
    for key in ["binding_at", "scope", "seam", "declaration"] {
        assert!(first[key] > 0);
    }
}

#[test]
fn simple_wrappers_reuse_binding_work() {
    for route in [1, 2, 3, 0] {
        let mut first = None;
        for n in [8, 16, 32] {
            let reads = " use(x);\n".repeat(n);
            let source = if route == 0 {
                format!("function owner(x) {{\n {{\n{reads} }}\n}}\n")
            } else {
                format!("function owner(x) {{\n{reads}}}\n")
            };
            let p = ParsedFile::parse("wrapper.ts", &source, Language::TypeScript).unwrap();
            assert_eq!(p.parse_error_count, 0);
            let owner = p.all_functions()[0];
            let path = AccessPath::simple("x");
            COUNTS.with(|slot| *slot.borrow_mut() = Some(HashMap::new()));
            let refs = match route {
                0 => p.find_path_references_scoped(&owner, &path, 1).len(),
                1 => p
                    .find_path_references_fenced(&owner, &path, 1, source.find('x').unwrap())
                    .len(),
                _ => p
                    .find_path_reference_spans_fenced_in(
                        &owner,
                        &path,
                        1,
                        source.find('x').unwrap(),
                        None,
                        route == 3,
                    )
                    .len(),
            };
            let counts = COUNTS.with(|slot| slot.borrow_mut().take().unwrap());
            assert_eq!(refs, n + usize::from(route != 3));
            if route == 0 {
                let declarations = &counts["declaration"];
                assert!(declarations.total > 0);
                assert_eq!(
                    declarations.total,
                    declarations.distinct.len(),
                    "each node in a declaration scan must be visited once per call"
                );
                continue;
            }
            let work: BTreeMap<_, _> = ["scope", "seam"]
                .into_iter()
                .map(|kind| (kind, counts.get(kind).map_or(0, |c| c.total)))
                .collect();
            assert!(
                work.values().sum::<usize>() > 0,
                "probe must observe binding work"
            );
            println!("wrapper route={route} reads={n} refs={refs} expensive={work:?}");
            assert_eq!(
                work,
                *first.get_or_insert_with(|| work.clone()),
                "route={route}: expensive scope work must be constant as reads grow"
            );
        }
    }
}

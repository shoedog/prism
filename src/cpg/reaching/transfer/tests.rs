use super::*;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

fn fixture(source: &str) -> ParsedFile {
    ParsedFile::parse("test.go", source, Language::Go).unwrap()
}

fn lua_fixture(source: &str) -> ParsedFile {
    ParsedFile::parse("test.lua", source, Language::Lua).unwrap()
}

fn real_defs(parsed: &ParsedFile, function: &Node<'_>) -> Vec<DefSite> {
    let (start, end) = parsed.node_line_range(function);
    parsed
        .assignment_lvalue_spans_on_lines(function, &(start..=end).collect())
        .into_iter()
        .enumerate()
        .map(|(index, span)| DefSite {
            id: DefId(index as u32),
            path: span.path,
            line: span.line,
            start_byte: span.start_byte,
            alias_derived: false,
            synthetic: false,
        })
        .collect()
}

type LineGraph = (BTreeMap<Line, usize>, Vec<Vec<(usize, bool)>>);

fn line_graph() -> LineGraph {
    let line_index = BTreeMap::from([(2, 1), (3, 2), (4, 3), (6, 4)]);
    let successors = vec![
        vec![(1, false)],
        vec![(2, false)],
        vec![(3, false), (4, false)],
        vec![(2, false)],
        vec![],
    ];
    (line_index, successors)
}

#[test]
fn synthetic_defs_have_binding_entries() {
    let parsed = fixture(
            "func f(items []int) {\n  x := source()\n  for _, x = range items {\n    observe(x)\n  }\n  sink(x)\n}\n",
        );
    let function = parsed.all_functions()[0];
    let mut defs = real_defs(&parsed, &function);
    let real_count = defs.len();
    let binders = discover(&parsed, &function, &defs);
    assert_eq!(binders.len(), 1);
    let (line_index, mut successors) = line_graph();
    let original_successors = successors.clone();
    let cfg = CfgView {
        line_index: &line_index,
        successors: &original_successors,
    };
    let mut predecessors = vec![Vec::new(); successors.len()];
    let mut mapped_defs = vec![Some(1); defs.len()];
    let applied = apply(
        &parsed,
        &binders,
        &line_index,
        &mut successors,
        &mut predecessors,
        &mut defs,
        &mut mapped_defs,
    );
    assert_eq!(applied.synthetic_nodes.len(), 1);
    let facts = BindingFacts::with_cfg_view(&parsed, &function, &defs, &cfg);
    assert_eq!(facts.def_binding_count(), defs.len());
    for synthetic in real_count..defs.len() {
        for existing in 0..real_count {
            let _ = facts.same_def_binding(synthetic, existing);
        }
    }
    assert!(facts.same_def_binding(real_count, 0));
}

#[test]
fn go_short_range_does_not_synthesize_a_definition() {
    let parsed = fixture(
            "func f(items []int) {\n  x := source()\n  for _, x := range items {\n    observe(x)\n  }\n  sink(x)\n}\n",
        );
    let function = parsed.all_functions()[0];
    let mut defs = real_defs(&parsed, &function);
    let (line_index, mut successors) = line_graph();
    let mut predecessors = vec![Vec::new(); successors.len()];
    let mut mapped_defs = vec![Some(1); defs.len()];
    let binders = discover(&parsed, &function, &defs);
    let materialized =
        materialize_existing_def_sites(&binders, &line_index, &mut defs, &mut mapped_defs);
    assert_eq!(materialized.len(), 1);
    let before = defs.len();
    let applied = apply(
        &parsed,
        &binders,
        &line_index,
        &mut successors,
        &mut predecessors,
        &mut defs,
        &mut mapped_defs,
    );
    assert!(applied.synthetic_nodes.is_empty());
    assert_eq!(defs.len(), before);
}

#[test]
fn lua_assignments_reuse_existing_defs_and_locals_are_not_reuse_binders() {
    let parsed = lua_fixture(
        "function f(c)\n  x = source()\n  local y = source()\n  x = clean()\n  sink(x)\nend\n",
    );
    let function = parsed.all_functions()[0];
    let defs = real_defs(&parsed, &function);
    let binders = discover(&parsed, &function, &defs);

    assert_eq!(
        binders
            .iter()
            .map(|binder| (
                binder.node_line,
                binder.path.base.as_str(),
                binder.missing_def
            ))
            .collect::<Vec<_>>(),
        vec![(2, "x", false), (4, "x", false)]
    );
}

#[test]
fn no_entry_edge_fallback_is_ownership_uncertain() {
    let parsed = fixture(
            "func f(items []int) {\n  x := source()\n  for _, x = range items {\n    observe(x)\n  }\n  sink(x)\n}\n",
        );
    let function = parsed.all_functions()[0];
    let defs = real_defs(&parsed, &function);
    let line_index = BTreeMap::from([(2, 1), (3, 2), (4, 3), (6, 4)]);
    let successors = vec![
        vec![(1, false)],
        vec![(2, false)],
        vec![(4, false)],
        vec![(2, false)],
        vec![],
    ];
    let cfg = CfgView {
        line_index: &line_index,
        successors: &successors,
    };
    let facts = BindingFacts::with_cfg_view(&parsed, &function, &defs, &cfg);
    assert_eq!(facts.no_entry_binders.get("x"), Some(&BTreeSet::from([3])));
    let use_byte = parsed
        .identifiers_on_line(4)
        .into_iter()
        .find(|node| parsed.node_text(node) == "x")
        .expect("body use")
        .start_byte();
    let label = facts.no_entry_use("x", use_byte).map(|construct_line| {
        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: construct_line as u32,
        })
    });
    assert_eq!(
        label,
        Some(FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: 3,
        }))
    );
}

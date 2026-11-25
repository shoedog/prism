//! Run the original matrix assertion on a graph with a supplied call site.

use super::*;

pub(in crate::cpg) fn assert_supplied_site_reaches_use(
    cpg: CodePropertyGraph,
    id: &str,
    ext: &str,
    ordinal: usize,
) {
    let row = read_rows(ROWS)
        .unwrap()
        .into_iter()
        .find(|row| row.id == id)
        .unwrap();
    let check = &row.expect[ordinal - 1].check;
    let Check::Reaches { to, .. } = check else {
        panic!("{id}/{ext} #{ordinal}: expected a reachability assertion");
    };
    let mut fixture = Fixture {
        cpg,
        files: &row.files,
        ext,
    };
    let (passed, actual) = fixture.check(check).unwrap();
    assert!(passed, "{id}/{ext} #{ordinal}: {actual}");

    let target = fixture.node(to, VarAccessKind::Use).unwrap();
    let graph = &mut fixture.cpg.graph;
    while let Some(edge) = graph
        .edges_directed(target, petgraph::Incoming)
        .find(|edge| matches!(edge.weight(), CpgEdge::DataFlow(_)))
        .map(|edge| edge.id())
    {
        graph.remove_edge(edge);
    }
    let (passed, actual) = fixture.check(check).unwrap();
    assert!(
        !passed,
        "{id}/{ext} #{ordinal}: disconnected body Use: {actual}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "expected a reachability assertion")]
    fn supplied_site_requires_a_reachability_assertion() {
        // c01 #1 is a `def` check, which cannot be disconnected and re-asserted.
        let cpg = crate::cpg::object_literal_arg_tests::build(
            Language::JavaScript,
            &[("m", "function f(a) {\n  use(a);\n}\n")],
            "js",
        );
        assert_supplied_site_reaches_use(cpg, "c01", "js", 1);
    }
}

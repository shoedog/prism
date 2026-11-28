use super::*;

fn count(parsed: &ParsedFile, line: usize, witness: Occurrence<'_, '_>) -> usize {
    collect_rules(
        parsed,
        line,
        &admitted(parsed, &[line]),
        [witness],
        select_eval,
    )
    .len()
}

fn flask_visit<'a>(
    parsed: &'a ParsedFile,
    site: Bound<'a>,
    operation: Node<'_>,
    source: usize,
    insertion: usize,
    line: usize,
) -> Occurrence<'a, 'static> {
    let key = |line| (parsed.path.as_str(), line);
    let visit = InlineVisit::from_visit(InlineRoute::Flask, operation, key(source), key(insertion));
    receipt(
        parsed,
        line,
        site,
        site.operand(),
        STRUCTURED,
        Provenance::Inline { visit },
    )
}

#[test]
fn path_edge_binds_the_exact_read_and_owner() {
    // (source, genuine operation, text the path really ends at)
    let cases = [
        (
            "function f(x) { res.send(x) } function g(y) { audit(y) }",
            "audit(y)",
            "res.send(x)",
        ),
        (
            "function f(x, y) { audit(x); audit(y); }",
            "audit(y)",
            "audit(x)",
        ),
        (
            "function f(x){ audit(x); const g = () => exec(x); }",
            "exec(x)",
            "audit(x)",
        ),
    ];
    for (source, op, path_end) in cases {
        let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
        let root = parsed.tree.root_node();
        let operand = &op[op.len() - 2..op.len() - 1];
        let genuine = site(
            &parsed,
            op,
            op.split('(').next().unwrap(),
            operand,
            Role::Argument(0),
        );
        let letter = &path_end[path_end.len() - 2..path_end.len() - 1];
        let elsewhere = node(&parsed, node(&parsed, root, path_end), letter);
        let def = node(&parsed, root, "x");
        // The path ends at another read; the witness claims this operation's read.
        let wrong = path_to(&parsed, def, elsewhere);
        let witness = receipt(
            &parsed,
            1,
            genuine,
            genuine.operand(),
            FLAT,
            destination(&wrong),
        );
        assert_eq!(count(&parsed, 1, witness), 0, "{source}");
        let right = path_to(&parsed, def, genuine.operand());
        let witness = receipt(
            &parsed,
            1,
            genuine,
            genuine.operand(),
            FLAT,
            destination(&right),
        );
        assert_eq!(count(&parsed, 1, witness), 1, "{source}");
        // Owner: the same bytes under another callable start line are refused.
        let mut moved = right.clone();
        moved.edges[0].to.function_start_line += 1;
        let witness = receipt(
            &parsed,
            1,
            genuine,
            genuine.operand(),
            FLAT,
            destination(&moved),
        );
        let owner_is_root = owner(&parsed, genuine.operation()) == root;
        assert_eq!(
            count(&parsed, 1, witness),
            usize::from(owner_is_root),
            "{source}"
        );
    }
    // A zero-width location is a point that must lie inside the read.
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { audit(x); audit(y); }",
        Language::JavaScript,
    )
    .unwrap();
    let audit = site(&parsed, "audit(y)", "audit", "y", Role::Argument(0));
    let def = node(&parsed, parsed.tree.root_node(), "x");
    let mut point = path_to(&parsed, def, audit.operand());
    point.edges[0].to.end_byte = point.edges[0].to.start_byte;
    let witness = receipt(
        &parsed,
        1,
        audit,
        audit.operand(),
        FLAT,
        destination(&point),
    );
    assert_eq!(count(&parsed, 1, witness), 1);
    point.edges[0].to.start_byte -= 3;
    point.edges[0].to.end_byte -= 3;
    let witness = receipt(
        &parsed,
        1,
        audit,
        audit.operand(),
        FLAT,
        destination(&point),
    );
    assert_eq!(count(&parsed, 1, witness), 0);
}

#[test]
fn flask_inline_record_is_bound_to_the_visit() {
    let parsed = ParsedFile::parse(
        "a.py",
        "def f():\n    pickle.loads(\n        request.data,\n    ); audit(x)\n",
        Language::Python,
    )
    .unwrap();
    let call = site(
        &parsed,
        "pickle.loads(\n        request.data,\n    )",
        "loads",
        "request.data",
        Role::Argument(0),
    );
    let other = site(&parsed, "audit(x)", "audit", "x", Role::Argument(0));
    // (site, visited operation, source key line, insertion key line, finding line)
    let inline = |site, operation, source, insertion, line| {
        flask_visit(&parsed, site, operation, source, insertion, line)
    };
    let op = call.operation();
    // The true visit used the middle line as both keys, and verifies there only.
    assert_eq!(count(&parsed, 3, inline(call, op, 3, 3, 3)), 1);
    assert_eq!(count(&parsed, 4, inline(call, op, 3, 3, 4)), 0);
    // A fabricated record naming the closing line as the source key is not the visit's.
    assert_eq!(count(&parsed, 4, inline(call, op, 4, 3, 4)), 0);
    // A source key outside the operation's span, or a visit of another operation.
    assert_eq!(count(&parsed, 3, inline(call, op, 5, 3, 3)), 0);
    assert_eq!(
        count(&parsed, 3, inline(call, other.operation(), 3, 3, 3)),
        0
    );
    // Flask may insert at a different line of the call than its source key.
    assert_eq!(count(&parsed, 3, inline(call, op, 2, 3, 3)), 1);
    assert_eq!(count(&parsed, 4, inline(call, op, 3, 4, 4)), 1);
}

use super::*;

fn leaked<'a>(
    parsed: &'a ParsedFile,
    line: usize,
    site: Bound<'a>,
    reached: Node<'a>,
    def: Node<'a>,
) -> Occurrence<'a, 'static> {
    let path = Box::leak(Box::new(path_to(parsed, def, reached)));
    receipt(parsed, line, site, reached, FLAT, destination(path))
}

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
fn receipt_binds_role_to_the_operand_slot() {
    let parsed = ParsedFile::parse(
        "a.py",
        "def f(x):\n    x.report('ok')\n    audit('safe', x)\n    pair(report('ok'), other(x))\n    report(k=x)\n",
        Language::Python,
    )
    .unwrap();
    let def = node(&parsed, parsed.tree.root_node(), "x");
    let make = |line, site, reached| leaked(&parsed, line, site, reached, def);
    let receiver = site(
        &parsed,
        "x.report('ok')",
        "report",
        "x",
        Role::ReceiverValue,
    );
    let as_argument = rebind(receiver, receiver.operand(), Role::Argument(0));
    assert_eq!(count(&parsed, 2, make(2, receiver, receiver.operand())), 1);
    assert_eq!(
        count(&parsed, 2, make(2, as_argument, receiver.operand())),
        0
    );

    let audit = site(&parsed, "audit('safe', x)", "audit", "x", Role::Argument(1));
    let safe = node(&parsed, audit.operation(), "'safe'");
    let first_slot = rebind(audit, safe, Role::Argument(1));
    let honest_first = rebind(audit, safe, Role::Argument(0));
    assert_eq!(count(&parsed, 3, make(3, audit, audit.operand())), 1);
    assert_eq!(count(&parsed, 3, make(3, honest_first, safe)), 1);
    assert_eq!(count(&parsed, 3, make(3, first_slot, safe)), 0);
    for role in [
        Role::ReceiverValue,
        Role::AssignmentValue,
        Role::AttributeValue,
    ] {
        let in_arguments = rebind(audit, audit.operand(), role);
        assert_eq!(count(&parsed, 3, make(3, in_arguments, audit.operand())), 0);
    }
    let second_as_first = rebind(audit, audit.operand(), Role::Argument(0));
    assert_eq!(
        count(&parsed, 3, make(3, second_as_first, audit.operand())),
        0
    );

    let pair = node(
        &parsed,
        parsed.tree.root_node(),
        "pair(report('ok'), other(x))",
    );
    let other = node(&parsed, pair, "other(x)");
    let read = node(&parsed, other, "x");
    let substituted = Bound::from_binder(
        pair,
        node(&parsed, pair, "report"),
        other,
        Role::Argument(1),
        FLAT,
    );
    assert_eq!(count(&parsed, 4, make(4, substituted, read)), 0);
    let genuine = Bound::from_binder(
        pair,
        node(&parsed, pair, "pair"),
        other,
        Role::Argument(1),
        FLAT,
    );
    assert_eq!(count(&parsed, 4, make(4, genuine, read)), 1);

    let keyword = site(
        &parsed,
        "report(k=x)",
        "report",
        "x",
        Role::KeywordArgument("k"),
    );
    let wrong_name = rebind(keyword, keyword.operand(), Role::KeywordArgument("j"));
    let positional = rebind(keyword, keyword.operand(), Role::Argument(0));
    assert_eq!(count(&parsed, 5, make(5, keyword, keyword.operand())), 1);
    assert_eq!(count(&parsed, 5, make(5, wrong_name, keyword.operand())), 0);
    assert_eq!(count(&parsed, 5, make(5, positional, keyword.operand())), 0);
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
        "def f():\n    pickle.loads(\n        request.get_data(),\n    ); audit(x)\n",
        Language::Python,
    )
    .unwrap();
    let call = site(
        &parsed,
        "pickle.loads(\n        request.get_data(),\n    )",
        "loads",
        "request.get_data()",
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

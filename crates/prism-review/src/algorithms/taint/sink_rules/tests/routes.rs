use super::*;

#[test]
fn receipt_accepts_go_originating_path_at_opening_line() {
    let parsed = ParsedFile::parse(
        "a.go",
        "package p\nfunc f() { x := c.Param(\"f\"); os.Remove(\n    x,\n); other(x) }\n",
        Language::Go,
    )
    .unwrap();
    let remove = site(
        &parsed,
        "os.Remove(\n    x,\n)",
        "Remove",
        "x",
        Role::Argument(0),
    );
    let other = site(&parsed, "other(x)", "other", "x", Role::Argument(0));
    let def = node(&parsed, parsed.tree.root_node(), "x");
    let path = path_to(&parsed, def, remove.operand);
    let mut wrong_origin = path.clone();
    wrong_origin.edges[0].from.line = 1;
    let mut wrong_read_line = path.clone();
    wrong_read_line.edges[0].to.line = 2;
    let origin = |path| Provenance::Path {
        path,
        edge_index: 0,
        route: PathRoute::GoOriginating,
        proof: PathProof::StableLocal,
    };
    let admitted = admitted(&parsed, &[2, 3, 4]);
    let read = remove.operand;
    let witnesses = [
        receipt(&parsed, 2, remove, read, STRUCTURED, origin(&path)),
        receipt(&parsed, 2, remove, read, STRUCTURED, destination(&path)),
        receipt(&parsed, 2, remove, read, FLAT, origin(&path)),
        receipt(&parsed, 2, remove, read, STRUCTURED, origin(&wrong_origin)),
        receipt(
            &parsed,
            2,
            remove,
            read,
            STRUCTURED,
            origin(&wrong_read_line),
        ),
        receipt(&parsed, 2, remove, other.operand, STRUCTURED, origin(&path)),
    ];
    let missing_edge = Provenance::Path {
        path: &path,
        edge_index: 1,
        route: PathRoute::GoOriginating,
        proof: PathProof::StableLocal,
    };
    assert!(collect_rules(
        &parsed,
        2,
        &admitted,
        [receipt(&parsed, 2, remove, read, STRUCTURED, missing_edge)],
        select_eval,
    )
    .is_empty());
    let rows = collect_rules(&parsed, 2, &admitted, witnesses, |witness, rows| {
        assert_eq!(witness.reached().start_position().row + 1, 3);
        assert_eq!(witness.site().operation.start_position().row + 1, 2);
        select_eval(witness, rows);
    });
    assert_eq!(rows.len(), 1);
    assert!(collect_rules(
        &parsed,
        3,
        &admitted,
        [
            receipt(&parsed, 3, remove, read, STRUCTURED, destination(&path)),
            receipt(&parsed, 3, remove, read, STRUCTURED, origin(&path))
        ],
        select_eval
    )
    .is_empty());
}

#[test]
fn inline_routes_keep_insertion_keys_and_selected_operands() {
    let parsed = ParsedFile::parse(
        "a.py",
        "other()\ninline(\n    source(),\n    'clean'\n)\n",
        Language::Python,
    )
    .unwrap();
    let call = site(
        &parsed,
        "inline(\n    source(),\n    'clean'\n)",
        "inline",
        "source()",
        Role::Argument(0),
    );
    let wrong_slot = Site {
        operand: node(&parsed, call.operation, "'clean'"),
        role: Role::Argument(1),
        ..call
    };
    let witness = |line, source_key, site, detector, route| {
        receipt(
            &parsed,
            line,
            site,
            call.operand,
            detector,
            Provenance::Inline {
                key: source_key,
                route,
            },
        )
    };
    let key = |line| (parsed.path.as_str(), line);
    let keys = admitted(&parsed, &[1, 2, 3, 4]);
    assert!(collect_rules(
        &parsed,
        1,
        &keys,
        [witness(1, key(1), call, STRUCTURED, InlineRoute::Flask)],
        select_eval
    )
    .is_empty());
    assert!(collect_rules(
        &parsed,
        3,
        &keys,
        [witness(3, key(2), call, STRUCTURED, InlineRoute::Flask)],
        select_eval
    )
    .is_empty());
    let rows = collect_rules(
        &parsed,
        2,
        &keys,
        [
            witness(2, key(2), call, STRUCTURED, InlineRoute::Flask),
            witness(2, key(2), wrong_slot, STRUCTURED, InlineRoute::Flask),
            witness(2, key(2), call, FLAT, InlineRoute::Flask),
            witness(2, key(2), call, STRUCTURED, InlineRoute::JsYaml),
            witness(2, ("other.py", 2), call, STRUCTURED, InlineRoute::Flask),
        ],
        |witness, rows| {
            assert_eq!(parsed.node_text(&witness.reached()), "source()");
            assert_eq!(witness.site().role, Role::Argument(0));
            select_eval(witness, rows);
        },
    );
    assert_eq!(rows.len(), 1);

    let go = ParsedFile::parse(
        "a.go",
        "package p\nfunc f() { c.File(\n    c.Param(\"f\"),\n) }\n",
        Language::Go,
    )
    .unwrap();
    let call = site(
        &go,
        "c.File(\n    c.Param(\"f\"),\n)",
        "File",
        "c.Param(\"f\")",
        Role::Argument(0),
    );
    let keys = admitted(&go, &[2, 3]);
    for (line, count) in [(2, 1), (3, 0)] {
        let inline = receipt(
            &go,
            line,
            call,
            call.operand,
            STRUCTURED,
            Provenance::Inline {
                key: (&go.path, line),
                route: InlineRoute::Go,
            },
        );
        assert_eq!(
            collect_rules(&go, line, &keys, [inline], select_eval).len(),
            count
        );
    }
}

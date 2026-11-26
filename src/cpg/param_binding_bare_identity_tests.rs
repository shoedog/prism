//! Step 5b controls for bare owner identity and entry Def eligibility.
use super::*;

#[test]
fn supplied_same_line_anonymous_bare_owners_bind_by_position_and_range() {
    let source = "register(p => { use(p); }); register(q => { use(q); }); register((r) => { use(r); });\nfunction main(v) { invoke(v); }";
    let files = parse(&BTreeMap::from([("m.js".into(), source.into())]));
    let cpg = CodePropertyGraph::build(&files);
    let parsed = &files["m.js"];
    let site = site(&cpg, parsed, "invoke(v)", "invoke");
    let real = cpg.call_graph.resolve_call_site_full(site);
    assert!(real.resolved.is_empty(), "{:?}", real.resolved);
    assert_eq!(real.drop, Some(crate::resolution::DropReason::UnknownName));

    for (implementation, param, admitted) in [
        ("p => { use(p); }", "p", true),
        ("q => { use(q); }", "q", true),
        ("(r) => { use(r); }", "r", false),
    ] {
        let (start, end) = span(source, implementation);
        let owner = FunctionId {
            file: "m.js".into(),
            name: crate::anon_owner::anonymous_owner_name(1, start + 1),
            start_line: 1,
            end_line: 1,
        };
        let independent = parsed
            .all_functions()
            .into_iter()
            .find(|node| node.byte_range() == (start..end))
            .unwrap();
        assert_eq!(
            crate::anon_owner::identity(parsed, &independent, "m.js"),
            Some(owner.clone())
        );
        let resolved = ResolvedCallee {
            target: &owner,
            kind: ResolutionKind::LocalDef,
            confidence: ResolutionConfidence::Exact,
        };
        let expected = [(
            &owner,
            param,
            (start, start + 1),
            VarAccess::Def,
            FlowConfidence::Exact,
        )];
        assert_bindings(
            &cpg,
            site,
            parsed,
            &run(&cpg, &files, site, vec![resolved.clone()]),
            if admitted { &expected } else { &[] },
        );

        let wrong_end = FunctionId {
            end_line: 2,
            ..owner.clone()
        };
        assert_bindings(
            &cpg,
            site,
            parsed,
            &run(
                &cpg,
                &files,
                site,
                vec![ResolvedCallee {
                    target: &wrong_end,
                    ..resolved
                }],
            ),
            &[],
        );
    }
}

#[test]
fn named_bare_locator_refuses_same_range_collisions() {
    for (gap, admits) in [("\n", true), (" ", false)] {
        let source = format!("var take = {BARE};{gap}var take = q => {{ use(q); }};\nfunction main(v) {{ take(v); }}");
        let files = parse(&BTreeMap::from([("m.js".into(), source)]));
        let cpg = CodePropertyGraph::build(&files);
        let parsed = &files["m.js"];
        let owner = target(&cpg, parsed, "take", BARE);
        let site = site(&cpg, parsed, "take(v)", "take");
        let expected = [endpoint(parsed, owner, false)];
        let supplied = ResolvedCallee {
            target: owner,
            kind: ResolutionKind::LocalDef,
            confidence: ResolutionConfidence::Exact,
        };
        assert_bindings(
            &cpg,
            site,
            parsed,
            &run(&cpg, &files, site, vec![supplied]),
            if admits { &expected } else { &[] },
        );
    }
}

#[test]
fn bare_slot_requires_the_entry_def_bytes() {
    let files = parse(&BTreeMap::from([(
        "m.js".into(),
        format!("const take = {BARE};\nfunction main(v) {{ take(v); }}"),
    )]));
    let mut cpg = CodePropertyGraph::build(&files);
    let parsed = &files["m.js"];
    let site = site(&cpg, parsed, "take(v)", "take").clone();
    let owner = target(&cpg, parsed, "take", BARE).clone();
    let resolved = ResolvedCallee {
        target: &owner,
        kind: ResolutionKind::LocalDef,
        confidence: ResolutionConfidence::Exact,
    };
    let expected = [endpoint(parsed, &owner, false)];
    assert_bindings(
        &cpg,
        &site,
        parsed,
        &run(&cpg, &files, &site, vec![resolved.clone()]),
        &expected,
    );

    let start = expected[0].2 .0;
    let def = cpg.graph.node_indices().find(|&index| {
        matches!(cpg.node(index), CpgNode::Variable { function, access: VarAccess::Def, start_byte, .. }
            if function == "take" && *start_byte == start)
    }).expect("the original entry Def");
    let CpgNode::Variable {
        start_byte,
        end_byte,
        ..
    } = &mut cpg.graph[def]
    else {
        unreachable!()
    };
    *start_byte += 1;
    *end_byte += 1;
    assert_bindings(
        &cpg,
        &site,
        parsed,
        &run(&cpg, &files, &site, vec![resolved.clone()]),
        &[],
    );

    let CpgNode::Variable {
        start_byte,
        end_byte,
        ..
    } = &mut cpg.graph[def]
    else {
        unreachable!()
    };
    (*start_byte, *end_byte) = (start, start + 1);
    cpg.var_index.hide([def]);
    assert_bindings(
        &cpg,
        &site,
        parsed,
        &run(&cpg, &files, &site, vec![resolved]),
        &[],
    );
}

#[test]
pub(super) fn enclosing_call_refuses_the_inferred_ordinary_endpoint() {
    check_enclosing_call("take(v)", false);
}

#[test]
fn enclosing_call_refuses_the_inferred_leaf_endpoint() {
    check_enclosing_call("take({ a: v })", true);
}

fn check_enclosing_call(call: &str, leaf: bool) {
    for enclosed in [false, true] {
        let value = if enclosed {
            format!("discard({BARE})")
        } else {
            BARE.into()
        };
        let files = parse(&BTreeMap::from([(
            "m.js".into(),
            format!("function discard(fn) {{ return () => 0; }}\nconst take = {value};\nfunction main(v) {{ take(v); take({{ a: v }}); }}"),
        )]));
        let cpg = CodePropertyGraph::build(&files);
        let parsed = &files["m.js"];
        let owner = target(&cpg, parsed, "take", BARE);
        let site = site(&cpg, parsed, call, "take");
        let actual = cpg.call_graph.resolve_call_site(site);
        assert!(object_literal_args::exact_singleton(&actual));
        assert_eq!(
            (actual[0].target, actual[0].kind),
            (owner, ResolutionKind::LocalDef)
        );
        let expected = [endpoint(parsed, owner, leaf)];
        assert_bindings(
            &cpg,
            site,
            parsed,
            &run(&cpg, &files, site, actual),
            if enclosed { &[] } else { &expected },
        );
    }
}

#[test]
fn unsupported_bare_occurrence_cannot_be_supplied_by_a_colliding_def() {
    let source = "const take = p => {\n  function shadow(p) { use(p); }\n  use(p);\n};\nfunction main(v) { take(v); }";
    let files = parse(&BTreeMap::from([("m.js".into(), source.into())]));
    let mut cpg = CodePropertyGraph::build(&files);
    let parsed = &files["m.js"];
    let site = site(&cpg, parsed, "take(v)", "take").clone();
    let owner = target(
        &cpg,
        parsed,
        "take",
        "p => {\n  function shadow(p) { use(p); }\n  use(p);\n}",
    )
    .clone();
    let start = span(source, "p =>").0;
    // D11 withholds the entry Def. Supply an otherwise matching coordinate to
    // isolate eligibility from the separate entry-Def check.
    let fake = cpg.graph.add_node(CpgNode::variable_occurrence(
        AccessPath::simple("p"),
        owner.file.clone(),
        owner.name.clone(),
        owner.start_line,
        owner.start_line,
        VarAccess::Def,
        start,
        start + 1,
    ));
    assert_eq!(cpg.var_index.insert_first_wins(&cpg.graph, fake), fake);
    let actual = cpg.call_graph.resolve_call_site(&site);
    assert!(object_literal_args::exact_singleton(&actual));
    assert_eq!(actual[0].target, &owner);
    assert_bindings(&cpg, &site, parsed, &run(&cpg, &files, &site, actual), &[]);
}

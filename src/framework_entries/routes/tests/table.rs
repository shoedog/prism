use super::*;

#[test]
fn route_table_partitions_receivers_and_counts_candidates_independently() {
    let mut unproven = literal_route("'a'", "/a", 12);
    unproven.receiver = Receiver::Unproven {
        reason: Reason::ReceiverUnproven,
    };
    unproven.paths = vec![exposure(Some(router(10)), full("/pretend"))];
    unproven.candidate = Some(false);

    let mut grounded_candidate = literal_route("'b'", "/b", 22);
    grounded_candidate.candidate = Some(true);
    grounded_candidate.reason = Some(Reason::UnsupportedRegistration);

    let mut grounded = literal_route("'c'", "/c", 32);
    grounded.candidate = Some(false);
    let unspecified = literal_route("'d'", "/d", 42);
    let table = RouteTable::new(
        vec![unspecified, grounded, grounded_candidate, unproven],
        vec![],
        vec![],
        vec![],
        [Framework::Flask, Framework::Express, Framework::Flask],
    );
    let [a, b, c, d] = table.routes() else {
        panic!("expected four routes")
    };
    assert_eq!(
        [a.candidate, b.candidate, c.candidate, d.candidate],
        [Some(true), Some(true), Some(false), None]
    );
    assert!(a.paths.is_empty());
    assert_eq!(b.reason, Some(Reason::UnsupportedRegistration));
    assert_wire(
        table.summary().clone(),
        json!({
            "enabled_adapters":["express","flask"],"registrations":4,"proven":3,
            "unproven":1,"candidates":2,"mounts":0,"diagnostics":[],"failed_roots":[]
        }),
    );
    assert_eq!(
        table.summary().proven + table.summary().unproven,
        table.routes().len()
    );
}

#[test]
fn route_table_blocks_only_failed_root_instances_and_orders_exposures() {
    let mut failed = router(10);
    failed.invocations = vec![Invocation::Call {
        file: "app.py".into(),
        start_byte: 20,
        end_byte: 24,
    }];
    let mut sibling = failed.clone();
    sibling.invocations = vec![Invocation::Call {
        file: "app.py".into(),
        start_byte: 30,
        end_byte: 34,
    }];
    let mut affected = exposure(Some(failed.clone()), full("/z"));
    affected.binding_blocked = Some(MethodSet::known(["GET"]));
    let mut unaffected = exposure(Some(sibling), full("/a"));
    let already_blocked = MethodSet::known(["POST"]);
    unaffected.binding_blocked = Some(already_blocked.clone());
    let suffix = exposure(
        Some(failed.clone()),
        ExposureStatus::Suffix {
            value: "/0".into(),
            reason: Reason::VariablePrefix,
        },
    );
    let orphan = exposure(
        None,
        ExposureStatus::Dynamic {
            reason: Reason::NonLiteralPath,
        },
    );
    let mut route = literal_route("path", "/a", 4);
    route.paths = vec![orphan.clone(), affected, suffix, unaffected];
    let failure = FailedRoot {
        root: failed,
        site: site(30),
        reason: Reason::DispatchConflict,
    };
    let table = RouteTable::new(vec![route], vec![], vec![], vec![failure.clone()], []);
    let paths = &table.routes()[0].paths;
    assert_eq!(
        paths
            .iter()
            .map(|path| path.status.value())
            .collect::<Vec<_>>(),
        [Some("/a"), Some("/z"), Some("/0"), None]
    );
    assert_eq!(paths[0].binding_blocked, Some(already_blocked));
    assert_eq!(paths[1].binding_blocked, Some(MethodSet::ANY));
    assert_eq!(paths[2].binding_blocked, Some(MethodSet::ANY));
    assert_eq!(paths[3], orphan);
    assert_eq!(table.summary().failed_roots, [failure]);
}

#[test]
fn route_table_orders_normalized_paths_then_literal_and_raw_fallbacks() {
    let raw = literal_route("`/b`", "/b", 0);
    let literal = literal_route("'/b'", "/b", 2);
    let mut normalized = literal_route("'/z'", "/z", 4);
    let mut path = exposure(Some(router(10)), full("/z"));
    path.normalized = Some("/a".into());
    normalized.paths = vec![path];
    let mut decoded = literal_route("'/0'", "/0", 6);
    decoded.paths = vec![exposure(Some(router(10)), full("/e"))];
    let mut dynamic = literal_route("expression", "", 8);
    dynamic.declaration.as_mut().unwrap().path.literal = None;
    dynamic.paths = vec![exposure(
        None,
        ExposureStatus::Dynamic {
            reason: Reason::NonLiteralPath,
        },
    )];
    let table = RouteTable::new(
        vec![raw, dynamic, decoded, literal, normalized],
        vec![],
        vec![],
        vec![],
        [],
    );
    assert_eq!(
        table
            .routes()
            .iter()
            .map(|route| route.site.start_byte)
            .collect::<Vec<_>>(),
        [4, 2, 0, 6, 8]
    );
}

#[test]
fn route_table_orders_mount_ordinals_and_keeps_all_reported_failures() {
    let early = Diagnostic {
        site: site(10),
        reason: Reason::UnsupportedRegistration,
    };
    let late = Diagnostic {
        site: site(20),
        reason: Reason::HapiRegistrationUnproven,
    };
    let first = FailedRoot {
        root: router(10),
        site: site(30),
        reason: Reason::DispatchConflict,
    };
    let second = FailedRoot {
        root: router(20),
        site: site(20),
        reason: Reason::HapiRegistrationUnproven,
    };
    let table = RouteTable::new(
        vec![],
        vec![
            mount(30, 0, 0),
            mount(20, 1, 0),
            mount(20, 0, 1),
            mount(20, 0, 0),
        ],
        vec![late.clone(), early.clone()],
        vec![second.clone(), first.clone()],
        [],
    );
    assert_eq!(
        table
            .mounts()
            .iter()
            .map(|edge| (
                edge.site.start_byte,
                edge.alternative_ordinal,
                edge.child_ordinal
            ))
            .collect::<Vec<_>>(),
        [(20, 0, 0), (20, 0, 1), (20, 1, 0), (30, 0, 0)]
    );
    let summary = table.summary();
    assert_eq!(summary.mounts, 4);
    assert_eq!(summary.registrations, 0);
    assert_eq!(summary.diagnostics, [early, late]);
    assert_eq!(summary.failed_roots, [first, second]);
    assert_wire(
        summary.diagnostics[0].clone(),
        json!({"site":{"file":"app.py","line":3,"start_byte":10,"end_byte":16},
               "reason":"unsupported_registration"}),
    );
    assert_wire(
        summary.failed_roots[0].clone(),
        json!({"root":{"file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
               "site":{"file":"app.py","line":3,"start_byte":30,"end_byte":36},"reason":"dispatch_conflict"}),
    );
}

#[test]
fn table_suppresses_callable_identity_for_unproven_receivers() {
    let mut route = literal_route("'/x'", "/x", 12);
    route.receiver = Receiver::Unproven {
        reason: Reason::ImportedUnproven,
    };
    let identity = CallableId {
        file: "app.js".into(),
        name: "handler".into(),
        start_line: 1,
        end_line: 1,
        start_byte: 0,
        end_byte: 7,
    };
    route.callbacks = [
        CallbackStatus::Resolved {
            identity: identity.clone(),
            argument_text: "alias".into(),
        },
        CallbackStatus::Anonymous {
            identity: identity.clone(),
            argument_text: "() => 1".into(),
        },
        CallbackStatus::Unresolved {
            text: "wrap(h)".into(),
            reason: Reason::WrapperCall,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(position, status)| Callback {
        position,
        status,
        delegate: Some(identity.clone()),
        role: Some(CallbackRole::Handler),
    })
    .collect();
    let table = RouteTable::new(vec![route], vec![], vec![], vec![], []);
    use crate::navigation::routes::RouteFormat;
    for format in [RouteFormat::Json, RouteFormat::Text] {
        let listing = format.render(Ok(table.clone()), false);
        assert!(listing.contains("wrap(h)") && listing.contains("imported_unproven"));
        assert!(!listing.contains("\"name\"") && !listing.contains("\"identity\""));
    }
    let callbacks = serde_json::to_value(&table.routes()[0].callbacks).unwrap();
    let expected = ["alias", "() => 1", "wrap(h)"];
    for (position, text) in expected.into_iter().enumerate() {
        assert_eq!(
            callbacks[position],
            json!({
                "position":position,"status":"unresolved","text":text,"reason":"imported_unproven",
                "delegate":null,"role":"handler"
            })
        );
    }
}

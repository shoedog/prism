use super::*;

#[test]
fn priority_on_mount_blocks_only_the_colliding_child() {
    let table = mount_contest("first", false);
    assert_eq!(
        origin(&table, 1).paths[0].binding_blocked,
        Some(MethodSet::known(["GET"]))
    );
    assert_eq!(origin(&table, 2).paths[0].binding_blocked, None);
    assert_eq!(origin(&table, 2).paths[0].shadowed, None);
    assert!(origin(&table, 2).conflicts.is_empty());
}

#[test]
fn unknown_mount_installation_leaves_disjoint_child_full() {
    let table = mount_contest("append", false);
    assert!(matches!(
        origin(&table, 1).paths[0].status,
        ExposureStatus::Suffix {
            reason: Reason::DispatchConflict,
            ..
        }
    ));
    assert_eq!(origin(&table, 2).paths[0].status, full("/api/y"));
    assert_eq!(origin(&table, 2).paths[0].binding_blocked, None);
}

#[test]
fn overwrite_cuts_one_key_of_an_earlier_mount() {
    let table = mount_contest("overwrite", true);
    assert!(origin(&table, 1).paths.is_empty());
    assert_eq!(origin(&table, 2).paths[0].status, full("/api/y"));
    assert_eq!(origin(&table, 2).paths[0].binding_blocked, None);
    assert_eq!(
        origin(&table, 1).superseded_keys[0].path.as_deref(),
        Some("/api/x")
    );
}

#[test]
fn rejecting_one_mount_key_keeps_disjoint_historical_exposure() {
    let table = mount_contest("reject", false);
    assert!(origin(&table, 1).paths.is_empty());
    assert_eq!(origin(&table, 2).paths[0].status, full("/api/y"));
    assert_eq!(
        origin(&table, 2).paths[0].binding_blocked,
        Some(MethodSet::ANY)
    );
    assert_eq!(table.summary().failed_roots.len(), 1);
}

fn mount_contest(policy: &'static str, mount_first: bool) -> RouteTable {
    let mounts = vec![edge(1, 2, 8, prefix("/api"))];
    let mut scoped = install(0, &mounts[0], MountEpoch::Live);
    scoped.target = Target::Mount {
        record: 0,
        epoch: MountEpoch::Live,
        key: Some(MountKey(1)),
    };
    scoped.key = ("x", if mount_first { "co-install" } else { policy });
    scoped.call = Some(7);
    let mut remainder = install(0, &mounts[0], MountEpoch::Live);
    remainder.call = scoped.call;
    let route = leaf(
        0,
        1,
        "/api/x",
        ("x", if mount_first { policy } else { "co-install" }),
    );
    let pair = if mount_first {
        [scoped, remainder, route]
    } else {
        [route, scoped, remainder]
    };
    let children = [
        leaf(1, 2, "/x", ("child-x", "co-install")),
        leaf(2, 2, "/y", ("child-y", "co-install")),
    ];
    let events: Vec<_> = if policy == "reject" {
        children.into_iter().chain(pair).collect()
    } else {
        pair.into_iter().chain(children).collect()
    };
    compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    )
}

#[test]
fn effective_leaf_methods_gate_mounts_without_narrowing_unrestricted_wire_guards() {
    let mut browse = edge(1, 2, 10, prefix("/browse"));
    browse.method_guard = Some(MethodSet::known(["GET", "HEAD"]));
    let mounts = vec![browse, edge(1, 2, 11, prefix("/manage"))];
    let events = [
        install(0, &mounts[0], MountEpoch::Live),
        install(1, &mounts[1], MountEpoch::Live),
        leaf(0, 2, "/items", ("GET", "co-install")),
        leaf(1, 2, "/items", ("POST", "co-install")),
    ];
    let mut routes = catalog(&events);
    routes[0].declaration.as_mut().unwrap().methods = Some(MethodSet::known(["GET"]));
    routes[0].default_methods = Some(MethodSet::known(["HEAD"]));
    routes[1].declaration.as_mut().unwrap().methods = Some(MethodSet::known(["POST"]));
    let table = compose(routes, mounts, &[router(1)], &events, &Synthetic::default());
    assert_eq!(literals(&table, 0), ["/browse/items", "/manage/items"]);
    assert_eq!(
        origin(&table, 0).paths[0].method_guard,
        MethodSet::known(["GET", "HEAD"])
    );
    assert_eq!(literals(&table, 1), ["/manage/items"]);
    assert_eq!(origin(&table, 1).paths[0].method_guard, MethodSet::ANY);
}

#[test]
fn root_can_be_reached_directly_and_through_historical_copies() {
    let mounts = vec![
        edge(2, 1, 10, prefix("/first")),
        edge(1, 2, 11, prefix("/back")),
    ];
    let events = [
        leaf(0, 1, "/x", ("x", "co-install")),
        install(0, &mounts[0], MountEpoch::Snapshot),
        install(1, &mounts[1], MountEpoch::Snapshot),
    ];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert_eq!(literals(&table, 0), ["/back/first/x", "/x"]);
}

#[test]
fn selected_root_live_self_mount_reports_cycle_beside_direct_path() {
    let mounts = vec![edge(1, 1, 10, prefix("/loop"))];
    let events = [
        leaf(0, 1, "/x", ("x", "co-install")),
        install(0, &mounts[0], MountEpoch::Live),
    ];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let paths = &origin(&table, 0).paths;
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].status, full("/x"));
    assert_eq!(
        paths[1].status,
        ExposureStatus::Suffix {
            value: "/loop/x".into(),
            reason: Reason::MountCycle
        }
    );
}

#[test]
fn skip_against_possibly_absent_slot_is_conditional() {
    let events = [
        leaf(0, 1, "/x", ("x", "co-install")),
        leaf(1, 1, "/x", ("x", "skip")),
        leaf(2, 1, "/y", ("y", "co-install")),
    ];
    let mut events = events;
    let Target::Route { path, .. } = &mut events[0].target else {
        unreachable!()
    };
    path.setup_guard = Some("maybe".into());
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert!(matches!(
        origin(&table, 1).paths[0].status,
        ExposureStatus::Suffix {
            reason: Reason::DispatchConflict,
            ..
        }
    ));
    assert_eq!(origin(&table, 2).paths[0].status, full("/y"));
}

#[test]
fn possibly_rejected_write_cannot_definitely_supersede_another_slot() {
    let mut events = [
        leaf(0, 1, "/x", ("a", "co-install")),
        leaf(1, 1, "/y", ("b", "co-install")),
        leaf(2, 1, "/x", ("write", "mix")),
    ];
    let Target::Route { path, .. } = &mut events[1].target else {
        unreachable!()
    };
    path.setup_guard = Some("maybe".into());
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    for i in [0, 2] {
        assert!(matches!(
            origin(&table, i).paths[0].status,
            ExposureStatus::Suffix {
                reason: Reason::DispatchConflict,
                ..
            }
        ));
        assert!(origin(&table, i).superseded_keys.is_empty());
    }
    assert_eq!(origin(&table, 1).paths[0].status, full("/y"));
    assert!(table.summary().failed_roots.is_empty());
}

#[test]
fn possible_later_winner_restricts_an_earlier_certain_install() {
    let events = [
        leaf(0, 1, "/x", ("x", "co-install")),
        leaf(1, 1, "/x", ("x", "append")),
    ];
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let earlier = &origin(&table, 0).paths[0];
    assert_eq!(earlier.status, full("/x"));
    assert_eq!(earlier.binding_blocked, Some(MethodSet::known(["GET"])));
}

#[test]
fn first_event_can_reject_with_its_own_reason() {
    let mut events = [
        leaf(0, 1, "/x", ("invalid", "handler-conflict")),
        leaf(1, 1, "/sibling", ("sibling", "co-install")),
        leaf(2, 1, "/other", ("other", "co-install")),
    ];
    events[0].rejection = Some(Reason::HapiHandlerConflict);
    events[0].call = Some(7);
    events[1].call = Some(7);
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert!(origin(&table, 0).paths.is_empty());
    assert_eq!(origin(&table, 0).reason, Some(Reason::HapiHandlerConflict));
    assert!(origin(&table, 1).paths.is_empty());
    assert_eq!(origin(&table, 1).reason, Some(Reason::HapiHandlerConflict));
    assert_eq!(
        origin(&table, 2).paths[0].binding_blocked,
        Some(MethodSet::ANY)
    );
    assert_eq!(
        table.summary().failed_roots,
        [FailedRoot {
            root: router(1),
            site: site(100),
            reason: Reason::HapiHandlerConflict
        }]
    );
}

fn guarded_call(guards: [Option<&str>; 2]) -> [Event<(&'static str, &'static str)>; 2] {
    let mut events = [
        leaf(0, 1, "/x", ("invalid", "handler-conflict")),
        leaf(1, 1, "/ok", ("ok", "co-install")),
    ];
    events[0].rejection = Some(Reason::HapiHandlerConflict);
    for (event, guard) in events.iter_mut().zip(guards) {
        event.call = Some(7);
        let Target::Route { path, .. } = &mut event.target else {
            unreachable!()
        };
        path.setup_guard = guard.map(Into::into);
    }
    events
}

#[test]
fn guarded_within_call_rejection_leaves_the_sibling_no_full_exposure() {
    // `if (flag) server.route([invalid, valid])`: /ok is never a certain install.
    let events = guarded_call([Some("flag"), Some("flag")]);
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert!(origin(&table, 0).paths.is_empty());
    assert_eq!(
        origin(&table, 1).paths[0].binding_blocked,
        Some(MethodSet::ANY)
    );
    assert!(table.summary().failed_roots.is_empty());
}

#[test]
fn equal_guard_text_is_not_one_evaluation_so_the_sibling_is_not_refused() {
    // `flag ? [item] : []` evaluated in two slots can differ: the first false,
    // the second true leaves only /ok in the call. It must not be refused.
    let events = guarded_call([Some("flag"), Some("flag")]);
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let ok = origin(&table, 1);
    assert_ne!(ok.reason, Some(Reason::HapiHandlerConflict));
    assert_eq!(ok.paths.len(), 1);
}

#[test]
fn guarded_rejection_only_possibly_aborts_a_sibling_under_another_guard() {
    for guards in [[Some("flag"), None], [Some("flag"), Some("other")]] {
        let events = guarded_call(guards);
        let table = compose(
            catalog(&events),
            vec![],
            &[router(1)],
            &events,
            &Synthetic::default(),
        );
        assert_eq!(
            origin(&table, 1).paths[0].binding_blocked,
            Some(MethodSet::ANY)
        );
        assert!(table.summary().failed_roots.is_empty());
    }
}

#[test]
fn unattributed_rejection_fails_closed_across_selected_roots() {
    let mounts = vec![edge(1, 2, 10, prefix("/api"))];
    let events = [
        install(0, &mounts[0], MountEpoch::Live),
        leaf(0, 2, "/x", ("x", "co-install")),
        leaf(1, 2, "/x", ("x", "reject")),
        leaf(2, 3, "/independent", ("i", "co-install")),
    ];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1), router(3)],
        &events,
        &Synthetic::default(),
    );
    assert_eq!(table.summary().failed_roots.len(), 2);
    assert_eq!(
        table.summary().diagnostics,
        [Diagnostic {
            site: site(101),
            reason: Reason::DispatchConflict
        }]
    );
    for i in [0, 2] {
        assert_eq!(
            origin(&table, i).paths[0].binding_blocked,
            Some(MethodSet::ANY)
        );
    }
}

#[test]
fn mismatched_route_receiver_cannot_install() {
    let events = [leaf(0, 1, "/x", ("x", "co-install"))];
    let mut routes = catalog(&events);
    routes[0].receiver = router_receiver(2);
    let table = compose(routes, vec![], &[router(1)], &events, &Synthetic::default());
    assert!(origin(&table, 0).paths.is_empty());
}

#[test]
fn skip_retains_a_refusal_reason() {
    let events = [
        leaf(0, 1, "/x", ("x", "co-install")),
        leaf(1, 1, "/x", ("x", "skip")),
    ];
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert!(origin(&table, 1).paths.is_empty());
    assert_eq!(origin(&table, 1).reason, Some(Reason::DispatchConflict));
}

#[test]
fn variable_outer_mount_avoids_exponential_exact_pass() {
    let mounts = binary_mounts(
        18,
        MountPrefix::Unevaluated {
            text: "config.prefix".into(),
        },
    );
    let (table, joins) = compose_binary(mounts, 18);
    let paths = &origin(&table, 0).paths;
    assert_eq!(paths.len(), 65);
    assert!(paths[..64].iter().all(|p| matches!(
        p.status,
        ExposureStatus::Suffix {
            reason: Reason::VariablePrefix,
            ..
        }
    )));
    assert!(joins < 500, "{joins} joins");
}

#[test]
fn impossible_outer_edge_spends_bounded_work_and_marks_incomplete_search() {
    let mut mounts = binary_mounts(16, prefix("/end"));
    mounts[0].end_guard = Some(true);
    let (table, joins) = compose_binary(mounts, 16);
    assert_eq!(
        origin(&table, 0)
            .paths
            .iter()
            .map(|p| &p.status)
            .collect::<Vec<_>>(),
        [&ExposureStatus::Suffix {
            value: "/leaf".into(),
            reason: Reason::ExpansionTruncated
        }],
    );
    assert!(joins <= 4096, "{joins} joins");
}

fn binary_mounts(depth: usize, outer: MountPrefix) -> Vec<Mount> {
    let mut mounts = vec![edge(1, 2, 0, outer)];
    for parent in 2..depth + 2 {
        for part in ["/a", "/b"] {
            mounts.push(edge(parent, parent + 1, mounts.len(), prefix(part)));
        }
    }
    mounts
}

fn compose_binary(mounts: Vec<Mount>, depth: usize) -> (RouteTable, usize) {
    let mut events: Vec<_> = mounts
        .iter()
        .enumerate()
        .map(|(i, mount)| install(i, mount, MountEpoch::Live))
        .collect();
    events.push(leaf(0, depth + 2, "/leaf", ("leaf", "co-install")));
    let relation = Synthetic::default();
    let table = compose(catalog(&events), mounts, &[router(1)], &events, &relation);
    (table, relation.joins.get())
}

#[test]
fn dynamic_seed_truncation_stays_dynamic() {
    let events = [Event {
        target: Target::Route {
            record: 0,
            path: exposure(
                None,
                ExposureStatus::Dynamic {
                    reason: Reason::RegexPath,
                },
            ),
            effective_methods: MethodSet::ANY,
        },
        ..leaf(0, 1, "", ("x", "co-install"))
    }];
    let mut mounts = vec![];
    let mut events = Vec::from(events);
    let roots: Vec<_> = (2..67).map(router).collect();
    for parent in 2..67 {
        mounts.push(edge(parent, 1, parent, prefix("/api")));
        events.push(install(
            mounts.len() - 1,
            mounts.last().unwrap(),
            MountEpoch::Live,
        ));
    }
    let table = compose(
        catalog(&events),
        mounts,
        &roots,
        &events,
        &Synthetic::default(),
    );
    let paths = &origin(&table, 0).paths;
    assert_eq!(paths.len(), 65);
    assert!(paths.iter().any(|p| p.status
        == ExposureStatus::Dynamic {
            reason: Reason::ExpansionTruncated
        }));
}

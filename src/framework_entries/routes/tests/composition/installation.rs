use super::*;

#[test]
fn composition_resolves_installation_per_slot_and_instance() {
    let mut events = vec![
        leaf(0, 1, "/a", ("a:get", "co-install")),
        leaf(1, 1, "/a", ("a:post", "co-install")),
        leaf(2, 1, "/a", ("a:get", "overwrite")),
        leaf(3, 1, "/b", ("b", "co-install")),
        leaf(4, 1, "/b", ("b", "reject")),
        leaf(5, 1, "/skipped", ("disjoint", "co-install")),
        leaf(6, 1, "/later", ("later", "co-install")),
        leaf(7, 1, "/b", ("b", "skip")),
        leaf(8, 1, "/c", ("c", "co-install")),
    ];
    events[4].call = Some(7);
    events[5].call = Some(7);
    let mut healthy = leaf(9, 1, "/b", ("b", "reject"));
    healthy.dispatcher.invocations.push(Invocation::Call {
        file: "app.py".into(),
        start_byte: 4,
        end_byte: 9,
    });
    let roots = [router(1), healthy.dispatcher.clone()];
    events.push(healthy);
    let table = compose(
        catalog(&events),
        vec![],
        &roots,
        &events,
        &Synthetic::default(),
    );
    assert_eq!(table.routes().len(), 10);
    assert_eq!(origin(&table, 0).callbacks.len(), 1);
    assert!(origin(&table, 0).paths.is_empty());
    assert_eq!(origin(&table, 0).reason, Some(Reason::Superseded));
    assert_eq!(origin(&table, 0).superseded_keys[0].site, site(102));
    for i in [4, 5, 7] {
        assert!(origin(&table, i).paths.is_empty(), "origin {i}");
    }
    for i in [1, 2, 3, 6, 8] {
        let path = &origin(&table, i).paths[0];
        assert!(matches!(path.status, ExposureStatus::Full { .. }));
        assert_eq!(path.binding_blocked, Some(MethodSet::ANY));
    }
    assert_eq!(
        table.summary().failed_roots,
        [FailedRoot {
            root: router(1),
            site: site(104),
            reason: Reason::DispatchConflict,
        }]
    );
    assert_eq!(origin(&table, 9).paths[0].root, Some(roots[1].clone()));
    assert_eq!(origin(&table, 9).paths[0].binding_blocked, None);
}

#[test]
fn composition_distinguishes_installation_uncertainty_and_dispatch_priority() {
    let mut events: Vec<_> = [
        ("/unknown", "a", "co-install"),
        ("/unknown", "a", "unknown"),
        ("/append", "b", "co-install"),
        ("/append", "b", "append"),
        ("/contest", "c", "co-install"),
        ("/contest", "c", "contest"),
        ("/first", "d", "co-install"),
        ("/first", "d", "first"),
        ("/partial", "e", "co-install"),
        ("/partial", "e", "partial"),
        ("/later", "f", "co-install"),
        ("/claim", "implicit", "co-install"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (path, key, policy))| leaf(i, 1, path, (key, policy)))
    .collect();
    let routes = catalog(&events);
    events.push(Event {
        target: Target::Internal,
        ..leaf(12, 1, "/claim", ("implicit", "contest"))
    });
    let table = compose(routes, vec![], &[router(1)], &events, &Synthetic::default());
    assert_eq!(table.routes().len(), 12);
    assert!(table.summary().failed_roots.is_empty());
    for i in [0, 1, 3] {
        assert!(matches!(
            origin(&table, i).paths[0].status,
            ExposureStatus::Suffix {
                reason: Reason::DispatchConflict,
                ..
            }
        ));
        assert!(!origin(&table, i).conflicts.is_empty());
    }
    for i in [2, 4, 5, 6, 7, 8, 9, 10, 11] {
        assert!(matches!(
            origin(&table, i).paths[0].status,
            ExposureStatus::Full { .. }
        ));
    }
    for i in [4, 5, 8, 11] {
        assert_eq!(
            origin(&table, i).paths[0].binding_blocked,
            Some(MethodSet::known(["GET"]))
        );
    }
    assert_eq!(origin(&table, 4).paths[0].shadowed, Some(None));
    assert_eq!(origin(&table, 6).paths[0].shadowed, Some(Some(false)));
    assert_eq!(origin(&table, 7).paths[0].shadowed, Some(Some(true)));
    assert_ne!(origin(&table, 8).paths[0].shadowed, Some(Some(true)));
    for i in [2, 9, 10] {
        assert_eq!(origin(&table, i).paths[0].binding_blocked, None);
    }
}

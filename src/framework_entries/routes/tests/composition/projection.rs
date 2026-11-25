use super::*;

#[test]
fn composition_uses_live_and_nested_copy_epochs() {
    let mounts = vec![
        edge(1, 2, 1, prefix("/live")),
        edge(3, 2, 2, prefix("/inner")),
        edge(1, 3, 3, prefix("/snapshot")),
        edge(1, 2, 4, prefix("/deferred")),
        edge(1, 2, 5, prefix("/never")),
        edge(4, 1, 6, prefix("/early")),
    ];
    let events = vec![
        install(0, &mounts[0], MountEpoch::Live),
        leaf(0, 2, "/old", ("a", "co-install")),
        install(1, &mounts[1], MountEpoch::Snapshot),
        leaf(1, 2, "/new", ("a", "overwrite")),
        install(2, &mounts[2], MountEpoch::Snapshot),
        leaf(2, 2, "/late", ("b", "co-install")),
        install(5, &mounts[5], MountEpoch::Snapshot),
        install(3, &mounts[3], MountEpoch::Live),
    ];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1), router(4)],
        &events,
        &Synthetic::default(),
    );
    assert_eq!(table.summary().mounts, 6);
    assert_eq!(
        literals(&table, 0),
        ["/early/snapshot/inner/old", "/snapshot/inner/old"]
    );
    assert_eq!(
        literals(&table, 1),
        ["/deferred/new", "/early/live/new", "/live/new"]
    );
    assert_eq!(
        literals(&table, 2),
        ["/deferred/late", "/early/live/late", "/live/late"]
    );
}

#[test]
fn composition_keeps_guards_suffixes_cycles_and_impossible_chains_distinct() {
    let mut conditional = edge(3, 2, 1, prefix("/inner"));
    conditional.setup_guard = Some("settings.enabled".into());
    conditional.method_guard = Some(MethodSet::known(["GET", "HEAD"]));
    let mut exact = edge(1, 3, 2, prefix("/exact"));
    exact.method_guard = Some(MethodSet::known(["GET", "PUT"]));
    let mut impossible = edge(1, 4, 6, prefix("/end"));
    impossible.end_guard = Some(true);
    let mounts = vec![
        conditional,
        exact,
        edge(
            1,
            3,
            3,
            MountPrefix::Unevaluated {
                text: "config.prefix".into(),
            },
        ),
        edge(3, 3, 4, prefix("/cycle")),
        edge(3, 2, 5, prefix("/inner")),
        impossible,
    ];
    let mut events: Vec<_> = mounts
        .iter()
        .enumerate()
        .map(|(i, mount)| install(i, mount, MountEpoch::Live))
        .collect();
    events.extend([
        leaf(0, 2, "/{literal}", ("a", "co-install")),
        leaf(1, 4, "/denied", ("b", "co-install")),
        leaf(2, 5, "/orphan", ("c", "co-install")),
        Event {
            target: Target::Route {
                record: 3,
                path: exposure(
                    None,
                    ExposureStatus::Dynamic {
                        reason: Reason::RegexPath,
                    },
                ),
                effective_methods: MethodSet::ANY,
            },
            ..leaf(3, 1, "", ("d", "co-install"))
        },
    ]);
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let paths = &origin(&table, 0).paths;
    assert_eq!(paths.len(), 6);
    assert!(paths
        .iter()
        .any(|p| p.status == full("/exact/inner/{literal}")
            && p.normalized.as_deref() == Some("/exact/inner/{literal}")
            && p.method_guard == MethodSet::known(["GET"])
            && p.setup_guard.as_deref() == Some("settings.enabled")));
    assert!(paths.iter().any(|p| p.root == Some(router(1))
        && p.status
            == ExposureStatus::Suffix {
                value: "/inner/{literal}".into(),
                reason: Reason::VariablePrefix
            }));
    assert!(paths.iter().any(|p| p.root.is_none()
        && p.status
            == ExposureStatus::Suffix {
                value: "/cycle/inner/{literal}".into(),
                reason: Reason::MountCycle
            }));
    assert!(origin(&table, 1).paths.is_empty());
    assert_eq!(
        origin(&table, 2).paths[0].status,
        ExposureStatus::Suffix {
            value: "/orphan".into(),
            reason: Reason::NoKnownMount
        }
    );
    assert_eq!(
        origin(&table, 3).paths[0].status,
        ExposureStatus::Dynamic {
            reason: Reason::RegexPath
        }
    );
}

#[test]
fn composition_bounds_distinct_expansion_before_materializing_the_product() {
    let mut mounts = vec![edge(
        1,
        2,
        0,
        MountPrefix::Unevaluated {
            text: "opaque".into(),
        },
    )];
    for child in 2..20 {
        for alternative in 0..2 {
            mounts.push(edge(
                child - 1,
                child,
                child * 10 + alternative,
                prefix(if alternative == 0 { "/a" } else { "/b" }),
            ));
        }
    }
    mounts.push(mounts[1].clone());
    let mut events: Vec<_> = mounts
        .iter()
        .enumerate()
        .rev()
        .map(|(i, mount)| install(i, mount, MountEpoch::Live))
        .collect();
    events.push(leaf(0, 19, "/leaf", ("a", "co-install")));
    let synthetic = Synthetic::default();
    let table = compose(
        catalog(&events),
        mounts.clone(),
        &[router(0), router(1)],
        &events,
        &synthetic,
    );
    let paths = &origin(&table, 0).paths;
    assert_eq!(table.summary().mounts, mounts.len());
    assert_eq!(paths.len(), 65);
    assert_eq!(paths[0].status, full(&format!("{}/leaf", "/a".repeat(18))));
    assert!(paths[..64]
        .iter()
        .all(|p| matches!(p.status, ExposureStatus::Full { .. })));
    assert_eq!(
        paths[64].status,
        ExposureStatus::Suffix {
            value: "/leaf".into(),
            reason: Reason::ExpansionTruncated
        }
    );
    assert!(synthetic.joins.get() < 500, "{}", synthetic.joins.get());
}

#[test]
fn composition_ignores_unrelated_historical_epochs_when_reporting_suffixes() {
    let mounts = vec![
        edge(2, 2, 1, prefix("/cycle")),
        edge(3, 4, 2, prefix("/former")),
        edge(6, 5, 3, prefix("/copy")),
    ];
    let mut events = vec![
        install(0, &mounts[0], MountEpoch::Live),
        install(1, &mounts[1], MountEpoch::Live),
        leaf(0, 2, "/old", ("a", "co-install")),
        leaf(1, 4, "/detached", ("b", "co-install")),
        install(2, &mounts[2], MountEpoch::Snapshot),
        leaf(2, 2, "/new", ("a", "overwrite")),
    ];
    let routes = catalog(&events);
    events.push(Event {
        target: Target::Internal,
        ..leaf(3, 3, "", ("mount", "overwrite"))
    });
    let table = compose(routes, mounts, &[router(6)], &events, &Synthetic::default());
    assert_eq!(
        [
            literals(&table, 0),
            literals(&table, 1),
            literals(&table, 2)
        ],
        [vec![], vec!["/detached"], vec!["/cycle/new"]]
    );
    assert_eq!(origin(&table, 0).reason, Some(Reason::Superseded));
    assert_eq!(
        origin(&table, 1).paths[0].status,
        ExposureStatus::Suffix {
            value: "/detached".into(),
            reason: Reason::NoKnownMount
        }
    );
    assert_eq!(
        origin(&table, 2).paths[0].status,
        ExposureStatus::Suffix {
            value: "/cycle/new".into(),
            reason: Reason::MountCycle
        }
    );
}

#[test]
fn composition_coalesces_equivalent_acyclic_chains_before_expanding_them() {
    let depth = 12;
    let mut mounts = vec![];
    for level in 0..depth {
        for parent in [level * 2, level * 2 + 1] {
            for child in [level * 2 + 2, level * 2 + 3] {
                mounts.push(edge(parent, child, mounts.len(), prefix("/same")));
            }
        }
    }
    let mut events: Vec<_> = mounts
        .iter()
        .enumerate()
        .map(|(i, mount)| install(i, mount, MountEpoch::Live))
        .collect();
    events.push(leaf(0, depth * 2, "/leaf", ("a", "co-install")));
    let synthetic = Synthetic::default();
    let table = compose(
        catalog(&events),
        mounts,
        &[router(0), router(1)],
        &events,
        &synthetic,
    );
    let expected = format!("{}/leaf", "/same".repeat(depth));
    assert_eq!(literals(&table, 0), [expected.as_str(), expected.as_str()]);
    assert!(
        synthetic.joins.get() < 250,
        "performed {} joins",
        synthetic.joins.get()
    );
}

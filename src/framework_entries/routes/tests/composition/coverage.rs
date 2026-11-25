use super::*;

/// A relation that keeps the trait's default `mount_key` and lets a test choose
/// the interaction and whether path text is tokenized into several literals.
struct Scripted {
    interact: fn(&Event<Key>, &Event<Key>) -> Interaction,
    split_paths: bool,
}

impl Relation<Key> for Scripted {
    fn interact(&self, earlier: &Event<Key>, later: &Event<Key>) -> Interaction {
        (self.interact)(earlier, later)
    }

    fn join(&self, mount: &Mount, path: &mut Exposure, methods: &MethodSet) -> bool {
        Relation::join(&Synthetic::default(), mount, path, methods)
    }

    fn path_tokens(&self, value: &str) -> Option<Vec<PathToken>> {
        if !self.split_paths {
            return Some(vec![PathToken::Literal(value.into())]);
        }
        // "/users/{id}" becomes "/", "users", "/", parameter: adjacent
        // literals must be merged by the composer.
        let mut tokens = vec![];
        for (i, segment) in value.split('/').enumerate() {
            if i > 0 {
                tokens.push(PathToken::Literal("/".into()));
            }
            if segment.starts_with('{') {
                tokens.push(PathToken::Parameter);
            } else if !segment.is_empty() {
                tokens.push(PathToken::Literal(segment.into()));
            }
        }
        Some(tokens)
    }
}

fn co_install(_: &Event<Key>, _: &Event<Key>) -> Interaction {
    Interaction::CoInstall
}

fn route_of(event: &mut Event<Key>) -> (&mut Exposure, &mut MethodSet) {
    let Target::Route {
        path,
        effective_methods,
        ..
    } = &mut event.target
    else {
        unreachable!()
    };
    (path, effective_methods)
}

fn dynamic_leaf(i: usize, dispatcher: usize) -> Event<Key> {
    Event {
        target: Target::Route {
            record: i,
            path: exposure(
                None,
                ExposureStatus::Dynamic {
                    reason: Reason::RegexPath,
                },
            ),
            effective_methods: MethodSet::ANY,
        },
        ..leaf(i, dispatcher, "", ("a", "co-install"))
    }
}

fn truncated(value: &str, root: Option<RouterId>) -> Exposure {
    Exposure {
        root,
        binding_blocked: Some(MethodSet::ANY),
        path_tokens: Some(vec![PathToken::Literal(value.into())]),
        normalized: Some(value.into()),
        ..exposure(
            None,
            ExposureStatus::Suffix {
                value: value.into(),
                reason: Reason::ExpansionTruncated,
            },
        )
    }
}

#[test]
fn mount_with_an_unproven_child_is_inventoried_but_exposes_nothing() {
    let mut mounts = vec![edge(1, 2, 1, prefix("/api"))];
    mounts[0].child.receiver = Receiver::Unproven {
        reason: Reason::MountChildUnproven,
    };
    let events = [
        install(0, &mounts[0], MountEpoch::Live),
        leaf(0, 2, "/x", ("a", "co-install")),
    ];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert_eq!(table.summary().mounts, 1);
    let [path] = origin(&table, 0).paths.as_slice() else {
        panic!("expected one path")
    };
    assert_eq!(path.root, None);
    assert_eq!(
        path.status,
        ExposureStatus::Suffix {
            value: "/x".into(),
            reason: Reason::NoKnownMount
        }
    );
}

#[test]
fn duplicate_seeds_emit_one_exposure_and_non_literal_seeds_follow_literal_ones() {
    let events = vec![
        leaf(0, 1, "/x", ("a", "co-install")),
        Event {
            site: site(300),
            ..dynamic_leaf(0, 1)
        },
        Event {
            site: site(301),
            ..leaf(0, 1, "/x", ("a", "co-install"))
        },
    ];
    let routes = catalog(&events[..1]);
    let table = compose(routes, vec![], &[router(1)], &events, &Synthetic::default());
    let paths = &origin(&table, 0).paths;
    assert_eq!(paths.len(), 2);
    assert_eq!(
        paths
            .iter()
            .filter(|p| p.status == full("/x") && p.root == Some(router(1)))
            .count(),
        1
    );
    assert!(paths.iter().any(|p| p.status
        == ExposureStatus::Dynamic {
            reason: Reason::RegexPath
        }
        && p.root == Some(router(1))));
}

#[test]
fn seed_whose_path_guard_excludes_its_effective_methods_exposes_nothing() {
    let mut events = [leaf(0, 1, "/x", ("GET", "co-install"))];
    route_of(&mut events[0]).0.method_guard = MethodSet::known(["POST"]);
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    assert!(origin(&table, 0).paths.is_empty());
    assert_eq!(origin(&table, 0).reason, None);
}

#[test]
fn join_merges_adjacent_literal_tokens_and_normalizes_parameters() {
    let mounts = vec![edge(1, 2, 1, prefix("/api"))];
    let events = [
        install(0, &mounts[0], MountEpoch::Live),
        leaf(0, 2, "/users/{id}/posts", ("a", "co-install")),
    ];
    let relation = Scripted {
        interact: co_install,
        split_paths: true,
    };
    let table = compose(catalog(&events), mounts, &[router(1)], &events, &relation);
    let [path] = origin(&table, 0).paths.as_slice() else {
        panic!("expected one path")
    };
    assert_eq!(path.status, full("/api/users/{id}/posts"));
    assert_eq!(
        path.path_tokens,
        Some(vec![
            PathToken::Literal("/api/users/".into()),
            PathToken::Parameter,
            PathToken::Literal("/posts".into()),
        ])
    );
    assert_eq!(path.normalized.as_deref(), Some("/api/users/{}/posts"));
}

#[test]
fn guards_join_by_text_and_impossible_method_chains_drop() {
    // (leaf setup guard, leaf methods, leaf effective methods, mount setup
    // guard, mount methods)
    type Case = (
        Option<&'static str>,
        MethodSet,
        MethodSet,
        Option<&'static str>,
        Option<MethodSet>,
    );
    let cases: Vec<Case> = vec![
        (Some("b"), MethodSet::ANY, MethodSet::ANY, Some("a"), None),
        (Some("a"), MethodSet::ANY, MethodSet::ANY, Some("a"), None),
        (
            None,
            MethodSet::unknown("x"),
            MethodSet::ANY,
            None,
            Some(MethodSet::known(["GET"])),
        ),
        (
            None,
            MethodSet::known(["POST"]),
            MethodSet::known(["POST"]),
            None,
            Some(MethodSet::unknown("y")),
        ),
        (
            None,
            MethodSet::unknown("z"),
            MethodSet::ANY,
            None,
            Some(MethodSet::unknown("z")),
        ),
        (
            None,
            MethodSet::unknown("x"),
            MethodSet::ANY,
            None,
            Some(MethodSet::unknown("y")),
        ),
        (
            None,
            MethodSet::known(["POST"]),
            MethodSet::known([] as [&str; 0]),
            None,
            Some(MethodSet::known(["GET"])),
        ),
    ];
    let mut mounts = vec![];
    let mut events = vec![];
    for (i, (leaf_setup, leaf_methods, effective, mount_setup, mount_methods)) in
        cases.into_iter().enumerate()
    {
        let mut mount = edge(1, 10 + i, i, prefix(&format!("/m{i}")));
        mount.setup_guard = mount_setup.map(str::to_owned);
        mount.method_guard = mount_methods;
        events.push(install(i, &mount, MountEpoch::Live));
        mounts.push(mount);
        let mut seed = leaf(i, 10 + i, "/x", ("k", "co-install"));
        let (path, methods) = route_of(&mut seed);
        path.setup_guard = leaf_setup.map(str::to_owned);
        path.method_guard = leaf_methods;
        *methods = effective;
        events.push(seed);
    }
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let guards = |i: usize| {
        let [path] = origin(&table, i).paths.as_slice() else {
            panic!("case {i}: expected one path")
        };
        assert_eq!(path.status, full(&format!("/m{i}/x")), "case {i}");
        (path.setup_guard.clone(), path.method_guard.clone())
    };
    assert_eq!(guards(0), (Some("(a) && (b)".into()), MethodSet::ANY));
    assert_eq!(guards(1), (Some("a".into()), MethodSet::ANY));
    assert_eq!(guards(2), (None, MethodSet::unknown("(GET) && (x)")));
    assert_eq!(guards(3), (None, MethodSet::unknown("(y) && (POST)")));
    assert_eq!(guards(4), (None, MethodSet::unknown("z")));
    assert_eq!(guards(5), (None, MethodSet::unknown("(y) && (x)")));
    assert!(origin(&table, 6).paths.is_empty());
    assert_eq!(origin(&table, 6).reason, None);
}

#[test]
fn unevaluated_prefix_leaves_a_non_literal_exposure_unchanged() {
    let mounts = vec![edge(
        1,
        2,
        1,
        MountPrefix::Unevaluated {
            text: "config.prefix".into(),
        },
    )];
    let events = [install(0, &mounts[0], MountEpoch::Live), dynamic_leaf(0, 2)];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let [path] = origin(&table, 0).paths.as_slice() else {
        panic!("expected one path")
    };
    assert_eq!(
        path.status,
        ExposureStatus::Dynamic {
            reason: Reason::RegexPath
        }
    );
    assert_eq!(path.root, Some(router(1)));
}

#[test]
fn proved_aborts_fail_only_the_root_that_runs_the_setup() {
    let mut events: Vec<_> = (0..4)
        .map(|i| leaf(i, 1, "/x", ("a", "co-install")))
        .collect();
    for event in &mut events {
        event.rejection = Some(Reason::UnsupportedRegistration);
    }
    // Setup runs in a selected root other than the dispatcher.
    events[0].setup_root = Some(router(9));
    // Setup runs in a root that was not selected.
    events[1].setup_root = Some(router(77));
    // A guarded abort fails no root.
    route_of(&mut events[3]).0.setup_guard = Some("maybe".into());
    let table = compose(
        catalog(&events),
        vec![],
        &[router(1), router(9)],
        &events,
        &Synthetic::default(),
    );
    let reason = Reason::UnsupportedRegistration;
    assert_eq!(
        table.summary().failed_roots.iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            &FailedRoot {
                root: router(9),
                site: site(100),
                reason
            },
            &FailedRoot {
                root: router(1),
                site: site(102),
                reason
            },
        ])
    );
    for i in 0..4 {
        assert!(origin(&table, i).paths.is_empty(), "origin {i}");
        assert_eq!(origin(&table, i).reason, Some(reason), "origin {i}");
    }
    assert!(table.summary().diagnostics.is_empty());
}

#[test]
fn build_only_routes_never_install_or_contest() {
    let events = [
        leaf(0, 1, "/x", ("a", "co-install")),
        leaf(1, 1, "/x", ("a", "overwrite")),
    ];
    let mut routes = catalog(&events);
    routes[0].reason = Some(Reason::BuildOnly);
    let table = compose(routes, vec![], &[router(1)], &events, &Synthetic::default());
    assert!(origin(&table, 0).paths.is_empty());
    assert_eq!(origin(&table, 0).reason, Some(Reason::BuildOnly));
    assert!(origin(&table, 0).superseded_keys.is_empty());
    let [path] = origin(&table, 1).paths.as_slice() else {
        panic!("expected one path")
    };
    assert_eq!(path.status, full("/x"));
    assert_eq!(path.binding_blocked, None);
    assert!(origin(&table, 1).conflicts.is_empty());
}

#[test]
fn identical_dispatch_cuts_are_recorded_once_per_route() {
    let events = [
        leaf(0, 1, "/x", ("x", "co-install")),
        leaf(1, 1, "/x", ("x", "reject")),
        Event {
            site: site(500),
            ..leaf(1, 1, "/x", ("x", "reject"))
        },
    ];
    let routes = catalog(&events[..2]);
    let table = compose(routes, vec![], &[router(1)], &events, &Synthetic::default());
    assert_eq!(
        origin(&table, 1).conflicts,
        [DispatchCut {
            root: Some(router(1)),
            method_guard: MethodSet::known(["GET"]),
            path: Some("/x".into()),
            site: site(100),
            reason: Reason::DispatchConflict,
        }]
    );
    assert_eq!(table.summary().failed_roots.len(), 2);
    assert!(origin(&table, 1).paths.is_empty());
}

#[test]
fn concurrent_contests_accumulate_every_losing_method_set() {
    fn contests(_: &Event<Key>, later: &Event<Key>) -> Interaction {
        let methods = match later.key.1 {
            "get" => MethodSet::known(["GET"]),
            "post" => MethodSet::known(["POST"]),
            _ => return Interaction::CoInstall,
        };
        Interaction::Priority {
            methods,
            priority: Priority::Unknown,
        }
    }
    let events = [
        leaf(0, 1, "/x", ("k", "co-install")),
        leaf(1, 1, "/x", ("k", "get")),
        leaf(2, 1, "/x", ("k", "post")),
    ];
    let relation = Scripted {
        interact: contests,
        split_paths: false,
    };
    let table = compose(catalog(&events), vec![], &[router(1)], &events, &relation);
    let blocked = |i: usize| origin(&table, i).paths[0].binding_blocked.clone();
    assert_eq!(blocked(0), Some(MethodSet::known(["GET", "POST"])));
    assert_eq!(blocked(1), Some(MethodSet::known(["GET", "POST"])));
    assert_eq!(blocked(2), Some(MethodSet::known(["POST"])));
}

#[test]
fn mount_slot_without_a_matching_request_key_exposes_nothing() {
    // The relation keys this joined exposure (`/api/x`, GET at site 8), but
    // the adapter installed only the unkeyed remainder.
    let mounts = vec![edge(1, 2, 8, prefix("/api"))];
    let events = [
        install(0, &mounts[0], MountEpoch::Live),
        leaf(0, 2, "/x", ("child-x", "co-install")),
    ];
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let [path] = origin(&table, 0).paths.as_slice() else {
        panic!("expected one path")
    };
    assert_eq!(path.root, None);
    assert_eq!(
        path.status,
        ExposureStatus::Suffix {
            value: "/x".into(),
            reason: Reason::NoKnownMount
        }
    );
}

fn keyed_mount(rejection: Option<Reason>) -> RouteTable {
    let mounts = vec![edge(1, 2, 8, prefix("/api"))];
    let mut scoped = install(0, &mounts[0], MountEpoch::Live);
    scoped.target = Target::Mount {
        record: 0,
        epoch: MountEpoch::Live,
        key: Some(MountKey(1)),
    };
    scoped.rejection = rejection;
    let events = [
        scoped,
        install(0, &mounts[0], MountEpoch::Live),
        leaf(0, 2, "/x", ("child-x", "co-install")),
    ];
    compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    )
}

#[test]
fn keyed_mount_slot_without_cuts_exposes_the_child() {
    let table = keyed_mount(None);
    let [path] = origin(&table, 0).paths.as_slice() else {
        panic!("expected one path")
    };
    assert_eq!(path.status, full("/api/x"));
    assert_eq!(path.root, Some(router(1)));
    assert_eq!(path.binding_blocked, None);
    assert!(table.summary().failed_roots.is_empty());
}

#[test]
fn refused_keyed_mount_slot_hides_the_child_without_a_no_mount_marker() {
    let reason = Reason::UnsupportedRegistration;
    let table = keyed_mount(Some(reason));
    assert!(origin(&table, 0).paths.is_empty());
    assert_eq!(origin(&table, 0).reason, None);
    assert_eq!(
        table.summary().failed_roots,
        [FailedRoot {
            root: router(1),
            site: site(8),
            reason
        }]
    );
}

fn fan_out(extra: impl FnOnce(&mut Vec<Mount>)) -> (Vec<Mount>, Vec<Event<Key>>) {
    let mut mounts: Vec<_> = (0..64)
        .map(|i| edge(1, 2, i, prefix(&format!("/p{i}"))))
        .collect();
    extra(&mut mounts);
    let mut events: Vec<_> = mounts
        .iter()
        .enumerate()
        .map(|(i, mount)| install(i, mount, MountEpoch::Live))
        .collect();
    events.push(leaf(0, 2, "/leaf", ("a", "co-install")));
    (mounts, events)
}

fn exposures_by_kind(table: &RouteTable) -> (usize, Vec<&Exposure>) {
    let (full_paths, rest): (Vec<_>, Vec<_>) = origin(table, 0)
        .paths
        .iter()
        .partition(|p| matches!(p.status, ExposureStatus::Full { .. }));
    (full_paths.len(), rest)
}

#[test]
fn exposure_cap_during_no_known_mount_search_marks_the_origin_truncated() {
    // The leaf router is also mounted under a router that no selected root
    // contains and that nothing mounts: its prefix is reported only as a suffix.
    let (mounts, events) = fan_out(|mounts| mounts.push(edge(3, 2, 100, prefix("/q"))));
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let (full_count, rest) = exposures_by_kind(&table);
    assert_eq!(full_count, 64);
    assert_eq!(rest, [&truncated("/leaf", None)]);
}

#[test]
fn exposure_cap_during_cycle_reporting_marks_the_origin_truncated() {
    let (mounts, events) = fan_out(|mounts| mounts.push(edge(1, 1, 200, prefix("/loop"))));
    let table = compose(
        catalog(&events),
        mounts,
        &[router(1)],
        &events,
        &Synthetic::default(),
    );
    let (full_count, rest) = exposures_by_kind(&table);
    assert_eq!(full_count, 64);
    assert_eq!(rest, [&truncated("/leaf", Some(router(1)))]);
}

#[test]
fn work_budget_ends_a_deep_chain_at_any_step_with_a_truncation_marker() {
    const DEPTH: usize = 1500;
    // Dead parallel mounts shift which step (visit, edge or version) of the
    // search spends the last unit of work.
    for dead in 0..4 {
        let mut mounts: Vec<_> = (0..DEPTH)
            .map(|level| edge(level + 1, level, level, prefix("/a")))
            .collect();
        for i in 0..dead {
            let mut impossible = edge(1, 0, DEPTH + i, prefix("/d"));
            impossible.end_guard = Some(true);
            mounts.push(impossible);
        }
        let mut events: Vec<_> = mounts
            .iter()
            .enumerate()
            .map(|(i, mount)| install(i, mount, MountEpoch::Live))
            .collect();
        events.push(leaf(0, 0, "/leaf", ("a", "co-install")));
        let table = compose(
            catalog(&events),
            mounts,
            &[router(DEPTH)],
            &events,
            &Synthetic::default(),
        );
        assert_eq!(
            origin(&table, 0).paths,
            [truncated("/leaf", Some(router(DEPTH)))],
            "{dead} dead mounts"
        );
    }
}

use super::*;
use serde_json::json;

mod wire;

fn router(byte: usize) -> RouterId {
    RouterId {
        file: "app.py".into(),
        binding: "app".into(),
        construction_byte: byte,
        invocations: vec![],
    }
}

#[test]
fn selectors_choose_only_a_unique_evidenced_identity() {
    let first = router(10);
    let second = router(20);
    let evidence = [
        RootEvidence {
            identity: first.clone(),
            selectors: vec![EntryKind::parse("app:app").unwrap()],
        },
        RootEvidence {
            identity: second.clone(),
            selectors: vec![EntryKind::parse("app:app").unwrap()],
        },
    ];
    fn select<'a>(entry: &str, evidence: &'a [RootEvidence]) -> Result<&'a RouterId, EntryError> {
        RootSelector::parse(entry).unwrap().resolve(evidence)
    }
    assert_eq!(select("app:app", &evidence[..1]).unwrap(), &first);
    assert_eq!(
        select("app:app", &evidence).unwrap_err().reason,
        Reason::EntryUnproven
    );
    assert!(select("app:missing", &evidence).is_err());
    let serialized = serde_json::to_string(&second).unwrap();
    assert_eq!(select(&serialized, &evidence).unwrap(), &second);
    assert!(select(&serialized, &evidence[..1]).is_err());
}

#[test]
fn selectors_preserve_spelling_and_reject_invalid_shapes() {
    for entry in [
        "app:factory()",
        "package.urls:urlpatterns",
        "./app.ts",
        "src/app.cjs#named",
    ] {
        assert!(RootSelector::parse(entry).is_ok(), "{entry}");
    }
    for entry in [
        "", "app:fake.member", "app:factory(1)", "app.js##name", "file.py", "/app.js", "a/../app.ts",
        "{\"file\":\"app.py\",\"binding\":\"app\",\"construction_byte\":4}",
        "{\"file\":\"app.py\",\"binding\":\"app\",\"construction_byte\":4,\"invocations\":[{\"file\":\"app.py\",\"start_byte\":9,\"end_byte\":8}]}",
    ] {
        let error = RootSelector::parse(entry).unwrap_err();
        assert_eq!(error.reason, Reason::EntryUnproven);
        assert_eq!(error.entry, entry);
    }
}

#[test]
fn serialized_identity_selects_exact_invocation_chain() {
    let mut first = router(10);
    first.invocations = vec![
        Invocation::Entry {
            entry: "app:create_app()".into(),
        },
        Invocation::Call {
            file: "app.py".into(),
            start_byte: 20,
            end_byte: 24,
        },
    ];
    let mut second = first.clone();
    second.invocations[1] = Invocation::Call {
        file: "app.py".into(),
        start_byte: 42,
        end_byte: 46,
    };
    let selector = RootSelector::parse(&serde_json::to_string(&first).unwrap()).unwrap();
    let roots = [
        RootEvidence {
            identity: second,
            selectors: vec![],
        },
        RootEvidence {
            identity: first.clone(),
            selectors: vec![],
        },
    ];
    assert_eq!(selector.resolve(&roots).unwrap(), &first);
    assert!(selector.resolve(&roots[..1]).is_err());
}

#[test]
fn wire_distinguishes_null_omitted_and_closed_variants() {
    assert_wire(MethodSet::known([] as [&str; 0]), json!({"known":[]}));
    assert_wire(MethodSet::ANY, json!("any"));
    assert_wire(
        MethodSet::unknown("settings.methods"),
        json!({"status":"unknown","text":"settings.methods"}),
    );
    assert_wire(
        MethodSet::known(["PUT", "GET", "PUT"]),
        json!({"known":["GET","PUT"]}),
    );
    assert!(serde_json::from_value::<Reason>(json!("unknown_reason")).is_err());
    assert!(serde_json::from_value::<PathToken>(json!(["parameter", "id"])).is_err());
    assert!(serde_json::from_value::<MethodSet>(json!({"known":["GET"],"text":"expr"})).is_err());
    assert_wire(PathToken::Parameter, json!(["parameter"]));
    assert_wire(
        PathToken::Literal("{id}".into()),
        json!(["literal", "{id}"]),
    );
}

#[test]
fn native_identifiers_can_select_evidenced_roots() {
    for entry in ["app:किरण", "किरण.urls:ऐप", "app.js#किरण", "app.js#\u{2118}"]
    {
        let kind = EntryKind::parse(entry).unwrap_or_else(|| panic!("{entry}"));
        let root = router(10);
        let evidence = [RootEvidence {
            identity: root.clone(),
            selectors: vec![kind],
        }];
        assert_eq!(
            RootSelector::parse(entry).unwrap().resolve(&evidence),
            Ok(&root)
        );
    }
    for entry in ["app:कि-रण", "app:किरण ", "app.js#नाम/अ", "app:१नाम"]
    {
        assert!(RootSelector::parse(entry).is_err(), "{entry}");
    }
}

#[test]
fn serialized_paths_use_repo_paths_instead_of_shorthand_delimiters() {
    let root = RouterId {
        file: "routes#v2.js".into(),
        invocations: vec![Invocation::Call {
            file: "src/config:dev\\local.js".into(),
            start_byte: 23,
            end_byte: 38,
        }],
        ..router(10)
    };
    let evidence = [RootEvidence {
        identity: root.clone(),
        selectors: vec![],
    }];
    let entry = serde_json::to_string(&root).unwrap();
    assert_eq!(
        RootSelector::parse(&entry).unwrap().resolve(&evidence),
        Ok(&root)
    );
    for file in [
        "/abs.js",
        "../escape.js",
        "src/../../escape.js",
        "line\nbreak.js",
        "nul\0.js",
        "control\u{7f}.js",
        "line\u{2028}break.js",
    ] {
        let mut invalid = root.clone();
        invalid.file = file.into();
        assert!(
            RootSelector::parse(&serde_json::to_string(&invalid).unwrap()).is_err(),
            "{file:?}"
        );
    }
}

fn assert_wire<T>(record: T, expected: serde_json::Value)
where
    T: std::fmt::Debug + PartialEq + Serialize + serde::de::DeserializeOwned,
{
    assert_eq!(serde_json::to_value(&record).unwrap(), expected);
    assert_eq!(serde_json::from_value::<T>(expected).unwrap(), record);
}

fn site(start_byte: usize) -> Site {
    Site {
        file: "app.py".into(),
        line: 3,
        start_byte,
        end_byte: start_byte + 6,
    }
}

fn router_receiver(byte: usize) -> Receiver {
    Receiver::Router {
        identity: router(byte),
        proof: None,
    }
}

fn literal_route(raw: &str, literal: &str, start_byte: usize) -> Route {
    Route {
        framework: Framework::Flask,
        protocol: Protocol::Http,
        site: site(start_byte),
        element_site: None,
        install_site: None,
        element_ordinal: None,
        receiver: router_receiver(10),
        origin_plugin: None,
        declaration: Some(Declaration {
            methods: None,
            path: DeclaredPath {
                raw: raw.into(),
                literal: Some(literal.into()),
            },
        }),
        default_methods: None,
        endpoint: None,
        candidate: None,
        reason: None,
        superseded_keys: vec![],
        conflicts: vec![],
        paths: vec![],
        callbacks: vec![],
    }
}

fn full(value: &str) -> ExposureStatus {
    ExposureStatus::Full {
        value: value.into(),
    }
}

fn exposure(root: Option<RouterId>, status: ExposureStatus) -> Exposure {
    Exposure {
        root,
        status,
        path_tokens: None,
        normalized: None,
        host: HostConstraint {
            host_kind: HostKind::Unconstrained,
            hosts: vec![],
        },
        method_guard: MethodSet::ANY,
        binding_blocked: None,
        version: None,
        strict_slashes: None,
        setup_guard: None,
        shadowed: None,
        mount_refusal: None,
    }
}

fn mount(start_byte: usize, alternative_ordinal: usize, child_ordinal: usize) -> Mount {
    Mount {
        parent: router_receiver(10),
        child: MountChild {
            receiver: router_receiver(20),
            realm: None,
        },
        site: site(start_byte),
        prefix: MountPrefix::Absent,
        guard_pattern: None,
        end_guard: None,
        method_guard: None,
        host: None,
        setup_guard: None,
        transform: None,
        alternative_ordinal,
        child_ordinal,
    }
}

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
fn identity_invocations_are_exact_and_only_an_initial_factory_entry_is_allowed() {
    let mut identity = router(10);
    identity.file = "src/../app.py".into();
    identity.invocations = vec![
        Invocation::Entry {
            entry: "app:create_app()".into(),
        },
        Invocation::Call {
            file: "./app.py".into(),
            start_byte: 20,
            end_byte: 24,
        },
    ];
    let wire = json!({
        "file":"src/../app.py","binding":"app","construction_byte":10,
        "invocations":[{"entry":"app:create_app()"},{"file":"./app.py","start_byte":20,"end_byte":24}]
    });
    assert_wire(identity.clone(), wire.clone());
    assert_eq!(
        EntryKind::parse(&wire.to_string()),
        Some(EntryKind::Identity(identity))
    );

    for invalid in [
        json!([{"entry":"app:app"}]),
        json!([{"file":"app.py","start_byte":20,"end_byte":24},{"entry":"app:create_app()"}]),
        json!([{"file":"../outside.py","start_byte":20,"end_byte":24}]),
        json!([{"file":"app.py","start_byte":20,"end_byte":20}]),
        json!([{"entry":"app:create_app()","file":"app.py"}]),
    ] {
        let mut selector = wire.clone();
        selector["invocations"] = invalid;
        let entry = selector.to_string();
        assert_eq!(
            RootSelector::parse(&entry),
            Err(EntryError {
                entry,
                reason: Reason::EntryUnproven
            })
        );
    }
}

#[test]
fn shorthand_matches_duplicate_evidence_but_filters_invalid_identities() {
    let shorthand = "./src/app.ts#choose\u{200c}$";
    let kind = EntryKind::JavaScript {
        file: "src/app.ts".into(),
        export: Some("choose\u{200c}$".into()),
    };
    assert_eq!(EntryKind::parse(shorthand), Some(kind.clone()));
    assert_eq!(
        EntryKind::parse("app.js#\u{0e33}export"),
        Some(EntryKind::JavaScript {
            file: "app.js".into(),
            export: Some("\u{0e33}export".into())
        })
    );
    assert_eq!(EntryKind::parse("app:\u{0e33}export"), None);
    assert_eq!(
        EntryKind::parse("src/app.ts"),
        Some(EntryKind::JavaScript {
            file: "src/app.ts".into(),
            export: None
        })
    );
    let first = router(10);
    let invalid = RouterId {
        binding: " ".into(),
        ..router(20)
    };
    let evidence = [
        RootEvidence {
            identity: invalid,
            selectors: vec![kind.clone()],
        },
        RootEvidence {
            identity: first.clone(),
            selectors: vec![kind.clone()],
        },
        RootEvidence {
            identity: first.clone(),
            selectors: vec![kind],
        },
    ];
    let selector = RootSelector::parse(shorthand).unwrap();
    assert_eq!(selector.resolve(&evidence), Ok(&first));
    assert_eq!(
        selector.resolve(&evidence[..1]),
        Err(EntryError {
            entry: shorthand.into(),
            reason: Reason::EntryUnproven
        })
    );
    for invalid in [
        "app.js#",
        "src//app.js",
        "src/./app.ts",
        "src\\app.js",
        "src/../app.js",
        ":factory()",
    ] {
        assert_eq!(EntryKind::parse(invalid), None, "{invalid}");
    }
}

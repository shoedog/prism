use super::*;

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
    use serde_json::{from_value, json, to_value};
    assert_eq!(
        to_value(MethodSet::known([] as [&str; 0])).unwrap(),
        json!({"known":[]})
    );
    assert_eq!(to_value(MethodSet::ANY).unwrap(), "any");
    assert_eq!(
        to_value(MethodSet::unknown("settings.methods")).unwrap(),
        json!({"status":"unknown","text":"settings.methods"})
    );
    assert!(from_value::<Reason>(json!("unknown_reason")).is_err());
    assert!(from_value::<PathToken>(json!(["parameter", "id"])).is_err());
    assert_eq!(
        to_value(PathToken::Parameter).unwrap(),
        json!(["parameter"])
    );
    assert_eq!(
        to_value(PathToken::Literal("{id}".into())).unwrap(),
        json!(["literal", "{id}"])
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

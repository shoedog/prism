use super::*;

#[test]
fn reason_spellings_are_closed_and_round_trip() {
    use Reason::*;
    for (reason, spelling) in [
        (ReceiverUnproven, "receiver_unproven"),
        (ImportedUnproven, "imported_unproven"),
        (MountChildUnproven, "mount_child_unproven"),
        (TableConsumerUnproven, "table_consumer_unproven"),
        (NoKnownMount, "no_known_mount"),
        (VariablePrefix, "variable_prefix"),
        (NonLiteralPath, "non_literal_path"),
        (RegexPath, "regex_path"),
        (MountCycle, "mount_cycle"),
        (ExpansionTruncated, "expansion_truncated"),
        (CallbackUnproven, "callback_unproven"),
        (MemberHandler, "member_handler"),
        (WrapperCall, "wrapper_call"),
        (InnerDecorator, "inner_decorator"),
        (ClassView, "class_view"),
        (AnonymousUnindexed, "anonymous_unindexed"),
        (SpreadCallback, "spread_callback"),
        (HandlerObject, "handler_object"),
        (
            MethodValueReceiverUnproven,
            "method_value_receiver_unproven",
        ),
        (
            MethodValueReceiverShadowed,
            "method_value_receiver_shadowed",
        ),
        (MethodValueMethodUnproven, "method_value_method_unproven"),
        (MethodValueInterface, "method_value_interface"),
        (MethodValueExternal, "method_value_external"),
        (FuncFieldValue, "func_field_value"),
        (QualifiedFunctionValue, "qualified_function_value"),
        (MethodExpression, "method_expression"),
        (HapiRegistrationUnproven, "hapi_registration_unproven"),
        (HapiPluginUnproven, "hapi_plugin_unproven"),
        (HapiPluginUnregistered, "hapi_plugin_unregistered"),
        (HapiServerParamUnproven, "hapi_server_param_unproven"),
        (HapiRouteNotLiteral, "hapi_route_not_literal"),
        (HapiHandlerObject, "hapi_handler_object"),
        (HapiHandlerConflict, "hapi_handler_conflict"),
        (
            RuntimeGeneratedRegistration,
            "runtime_generated_registration",
        ),
        (UnsupportedRegistration, "unsupported_registration"),
        (BuildOnly, "build_only"),
        (Superseded, "superseded"),
        (DispatchConflict, "dispatch_conflict"),
        (EntryUnproven, "entry_unproven"),
    ] {
        assert_wire(reason, json!(spelling));
    }
    assert_wire(
        EntryError {
            reason: EntryUnproven,
            entry: "app:missing".into(),
        },
        json!({"reason":"entry_unproven","entry":"app:missing"}),
    );
}

#[test]
fn framework_protocol_and_constraint_variants_round_trip() {
    for (framework, spelling) in [
        (Framework::Chi, "chi"),
        (Framework::Django, "django"),
        (Framework::Express, "express"),
        (Framework::FastApi, "fastapi"),
        (Framework::Flask, "flask"),
        (Framework::Gin, "gin"),
        (Framework::Hapi, "hapi"),
        (Framework::NestJs, "nestjs"),
        (Framework::NetHttp, "net/http"),
        (Framework::ObjectTable, "object-table"),
    ] {
        assert_wire(framework, json!(spelling));
    }
    assert_wire(Protocol::Http, json!("http"));
    assert_wire(Protocol::Websocket, json!("websocket"));
    for (kind, spelling) in [
        (HostKind::Unconstrained, "unconstrained"),
        (HostKind::Literal, "literal"),
        (HostKind::Parameterized, "parameterized"),
        (HostKind::Regex, "regex"),
        (HostKind::Unknown, "unknown"),
    ] {
        assert_wire(kind, json!(spelling));
    }
    for (version, expected) in [
        (
            Version::Header {
                header: "X-Version".into(),
                value: "1".into(),
            },
            json!({"kind":"header","header":"X-Version","value":"1"}),
        ),
        (
            Version::Media {
                value: Some("2".into()),
            },
            json!({"kind":"media","value":"2"}),
        ),
        (
            Version::Custom { value: None },
            json!({"kind":"custom","value":null}),
        ),
        (
            Version::Unknown { value: None },
            json!({"kind":"unknown","value":null}),
        ),
    ] {
        assert_wire(version, expected);
    }
    assert!(serde_json::from_value::<Version>(json!({"kind":"uri","value":"1"})).is_err());
}

#[test]
fn exposures_encode_full_suffix_dynamic_and_nullable_fields() {
    let mut rooted = exposure(Some(router(10)), full("/users/<id>"));
    rooted.path_tokens = Some(vec![
        PathToken::Literal("/users/".into()),
        PathToken::Parameter,
    ]);
    rooted.normalized = Some("/users/{id}".into());
    rooted.host = HostConstraint {
        host_kind: HostKind::Literal,
        hosts: vec!["api.example".into()],
    };
    rooted.method_guard = MethodSet::known(["POST", "GET"]);
    rooted.binding_blocked = Some(MethodSet::known(["POST"]));
    rooted.setup_guard = Some("is_enabled".into());
    rooted.strict_slashes = Some(None);
    assert_eq!(rooted.status.value(), Some("/users/<id>"));
    assert_wire(
        rooted,
        json!({
            "root":{"file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
            "status":"full","value":"/users/<id>","path_tokens":[["literal","/users/"],["parameter"]],
            "normalized":"/users/{id}","host_kind":"literal","hosts":["api.example"],
            "method_guard":{"known":["GET","POST"]},"binding_blocked":{"known":["POST"]},
            "version":null,"strict_slashes":null,"setup_guard":"is_enabled"
        }),
    );

    let mut suffix = exposure(
        None,
        ExposureStatus::Suffix {
            value: "/reports".into(),
            reason: Reason::NoKnownMount,
        },
    );
    suffix.mount_refusal = Some(Reason::HapiRegistrationUnproven);
    suffix.shadowed = Some(None);
    assert_eq!(suffix.status.value(), Some("/reports"));
    assert_wire(
        suffix,
        json!({
            "root":null,"status":"suffix","value":"/reports","reason":"no_known_mount",
            "path_tokens":null,"normalized":null,"host_kind":"unconstrained","hosts":[],
            "method_guard":"any","version":null,"shadowed":null,
            "mount_refusal":"hapi_registration_unproven"
        }),
    );

    let mut dynamic = exposure(
        None,
        ExposureStatus::Dynamic {
            reason: Reason::RegexPath,
        },
    );
    dynamic.shadowed = Some(Some(false));
    assert_eq!(dynamic.status.value(), None);
    assert_wire(
        dynamic,
        json!({
            "root":null,"status":"dynamic","reason":"regex_path","path_tokens":null,"normalized":null,
            "host_kind":"unconstrained","hosts":[],"method_guard":"any","version":null,"shadowed":false
        }),
    );
}

#[test]
fn callbacks_preserve_identity_role_order_and_forwarding() {
    let identity = CallableId {
        file: "app.js".into(),
        name: "guard".into(),
        start_line: 4,
        end_line: 4,
        start_byte: 8,
        end_byte: 18,
    };
    let anonymous = CallableId {
        name: "<anonymous>".into(),
        start_byte: 22,
        end_byte: 37,
        ..identity.clone()
    };
    assert_wire(
        vec![
            Callback {
                position: 0,
                status: CallbackStatus::Resolved {
                    identity: identity.clone(),
                    argument_text: String::new(),
                },
                delegate: None,
                role: Some(CallbackRole::Pre { group: 1 }),
            },
            Callback {
                position: 1,
                status: CallbackStatus::Anonymous {
                    identity: anonymous,
                    argument_text: String::new(),
                },
                delegate: Some(identity),
                role: Some(CallbackRole::Handler),
            },
            Callback {
                position: 2,
                status: CallbackStatus::Unresolved {
                    text: "wrap(serve)".into(),
                    reason: Reason::WrapperCall,
                },
                delegate: None,
                role: None,
            },
        ],
        json!([
            {"position":0,"status":"resolved","file":"app.js","name":"guard",
             "start_line":4,"end_line":4,"start_byte":8,"end_byte":18,"delegate":null,"role":"pre","group":1},
            {"position":1,"status":"anonymous","file":"app.js","name":"<anonymous>",
             "start_line":4,"end_line":4,"start_byte":22,"end_byte":37,"role":"handler",
             "delegate":{"file":"app.js","name":"guard","start_line":4,"end_line":4,"start_byte":8,"end_byte":18}},
            {"position":2,"status":"unresolved","text":"wrap(serve)","reason":"wrapper_call","delegate":null}
        ]),
    );
}

#[test]
fn route_records_preserve_declaration_and_candidate_presence() {
    let mut route = literal_route("'/status'", "/status", 12);
    route.default_methods = Some(MethodSet::known(["GET"]));
    route.endpoint = Some("status".into());
    let expected = json!({
        "framework":"flask","protocol":"http",
        "site":{"file":"app.py","line":3,"start_byte":12,"end_byte":18},
        "receiver":{"status":"router","file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
        "methods":null,"path":{"raw":"'/status'","literal":"/status"},
        "default_methods":{"known":["GET"]},"endpoint":"status","paths":[],"callbacks":[]
    });
    assert_wire(route.clone(), expected.clone());
    route.candidate = Some(false);
    assert_eq!(serde_json::to_value(route).unwrap(), expected);
    assert_eq!(
        serde_json::from_value::<Route>(expected).unwrap().candidate,
        None
    );

    let mut opaque = literal_route("descriptor", "", 12);
    opaque.framework = Framework::Hapi;
    opaque.install_site = Some(site(32));
    opaque.element_ordinal = Some(2);
    opaque.receiver = Receiver::Unproven {
        reason: Reason::HapiServerParamUnproven,
    };
    opaque.origin_plugin = Some(PluginOrigin {
        file: "plugin.js".into(),
        object_byte: 11,
        name: None,
    });
    opaque.declaration = None;
    opaque.candidate = Some(true);
    opaque.reason = Some(Reason::HapiRouteNotLiteral);
    assert_wire(
        opaque,
        json!({
            "framework":"hapi","protocol":"http",
            "site":{"file":"app.py","line":3,"start_byte":12,"end_byte":18},
            "install_site":{"file":"app.py","line":3,"start_byte":32,"end_byte":38},"element_ordinal":2,
            "receiver":{"status":"unproven","reason":"hapi_server_param_unproven"},
            "origin_plugin":{"file":"plugin.js","object_byte":11,"name":null},
            "candidate":true,"reason":"hapi_route_not_literal","paths":[],"callbacks":[]
        }),
    );
}

#[test]
fn dispatch_cuts_keep_root_scope_and_distinguish_conflict_from_supersession() {
    let mut route = literal_route("'/items'", "/items", 2);
    route.framework = Framework::ObjectTable;
    route.element_site = Some(site(5));
    route.element_ordinal = Some(0);
    route.superseded_keys.push(DispatchCut {
        root: Some(router(10)),
        method_guard: MethodSet::known(["GET"]),
        path: Some("/items".into()),
        site: site(40),
        reason: Reason::Superseded,
    });
    route.conflicts.push(DispatchCut {
        root: None,
        method_guard: MethodSet::ANY,
        path: None,
        site: site(50),
        reason: Reason::DispatchConflict,
    });
    assert_wire(
        route,
        json!({
            "framework":"object-table","protocol":"http",
            "site":{"file":"app.py","line":3,"start_byte":2,"end_byte":8},
            "element_site":{"file":"app.py","line":3,"start_byte":5,"end_byte":11},"element_ordinal":0,
            "receiver":{"status":"router","file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
            "methods":null,"path":{"raw":"'/items'","literal":"/items"},"paths":[],"callbacks":[],
            "superseded_keys":[{"root":{"file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
                                "method_guard":{"known":["GET"]},"path":"/items","reason":"superseded",
                                "site":{"file":"app.py","line":3,"start_byte":40,"end_byte":46}}],
            "conflicts":[{"root":null,"method_guard":"any","path":null,"reason":"dispatch_conflict",
                          "site":{"file":"app.py","line":3,"start_byte":50,"end_byte":56}}]
        }),
    );
}

#[test]
fn mount_records_preserve_prefix_variants_guards_and_realm() {
    let absent = mount(30, 0, 0);
    let mut expected = json!({
        "parent":{"status":"router","file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
        "child":{"status":"router","file":"app.py","binding":"app","construction_byte":20,"invocations":[]},
        "site":{"file":"app.py","line":3,"start_byte":30,"end_byte":36},
        "prefix":{"status":"absent"},"alternative_ordinal":0,"child_ordinal":0
    });
    assert_wire(absent.clone(), expected.clone());

    let mut literal = absent.clone();
    literal.prefix = MountPrefix::Literal {
        value: "/api/".into(),
    };
    literal.guard_pattern = Some("GET api.example/api/".into());
    literal.method_guard = Some(MethodSet::known(["GET", "HEAD"]));
    literal.host = Some(HostConstraint {
        host_kind: HostKind::Literal,
        hosts: vec!["api.example".into()],
    });
    literal.transform = Some(MountTransform::StripPrefix);
    expected["prefix"] = json!({"status":"literal","value":"/api/"});
    expected["guard_pattern"] = json!("GET api.example/api/");
    expected["method_guard"] = json!({"known":["GET","HEAD"]});
    expected["host_kind"] = json!("literal");
    expected["hosts"] = json!(["api.example"]);
    expected["transform"] = json!("strip_prefix");
    assert_wire(literal, expected);
    assert_wire(MountTransform::Identity, json!("identity"));

    let mut regex = absent.clone();
    regex.prefix = MountPrefix::Unevaluated {
        text: "r'^api/$'".into(),
    };
    regex.guard_pattern = Some("^api/$".into());
    regex.end_guard = Some(true);
    let mut expected_regex = serde_json::to_value(&absent).unwrap();
    expected_regex["prefix"] = json!({"status":"unevaluated","text":"r'^api/$'"});
    expected_regex["guard_pattern"] = json!("^api/$");
    expected_regex["end_guard"] = json!(true);
    assert_wire(regex, expected_regex);

    let mut plugin = absent;
    plugin.child = MountChild {
        receiver: Receiver::Router {
            identity: router(20),
            proof: Some(ReceiverProof::PluginContract),
        },
        realm: Some(vec![RealmFrame {
            file: "plugin.js".into(),
            start_byte: 40,
            end_byte: 60,
            item_ordinal: 2,
        }]),
    };
    plugin.prefix = MountPrefix::Unevaluated {
        text: "settings.prefix".into(),
    };
    plugin.setup_guard = Some("enabled".into());
    assert_wire(
        plugin,
        json!({
            "parent":{"status":"router","file":"app.py","binding":"app","construction_byte":10,"invocations":[]},
            "child":{"status":"router","file":"app.py","binding":"app","construction_byte":20,"invocations":[],
                     "proof":"plugin_contract","realm":[{"file":"plugin.js","start_byte":40,"end_byte":60,"item_ordinal":2}]},
            "site":{"file":"app.py","line":3,"start_byte":30,"end_byte":36},
            "prefix":{"status":"unevaluated","text":"settings.prefix"},"setup_guard":"enabled",
            "alternative_ordinal":0,"child_ordinal":0
        }),
    );
}

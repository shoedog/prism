use super::*;
use crate::languages::Language;

const BLUEPRINT: &str = "from flask import Blueprint\nbp = Blueprint('bp', __name__)\n";
const SHORTCUT: &str = "@bp.get(r\"/x\")\ndef view(): pass\n";
const FACTORY: &str = r#"from flask import Flask
from extensions import db
from views import bp
def create_app():
    app = Flask(__name__)
    db.init_app(app)
    app.register_blueprint(bp)
    return app
"#;
const FORMAL: &str = r#"from extensions import external
def view(): pass
@external.record_once
def configure(bp):
    bp.add_url_rule("/formal", view_func=view, methods=["POST"])
"#;

fn listing(sources: &[(&str, &str)]) -> RouteTable {
    let files = sources
        .iter()
        .map(|&(path, source)| {
            (
                path.to_owned(),
                ParsedFile::parse(path, source, Language::Python).unwrap(),
            )
        })
        .collect();
    routes(
        &files,
        &CallGraph::empty(),
        &ManifestSnapshot::default(),
        None,
    )
    .unwrap()
}

#[track_caller]
fn only_row(table: &RouteTable) -> &Route {
    let [row] = table.routes() else {
        panic!(
            "expected one retained Flask registration, got {}",
            table.routes().len()
        );
    };
    assert_eq!(
        (row.framework, row.protocol),
        (Framework::Flask, Protocol::Http)
    );
    row
}

#[track_caller]
fn assert_site(row: &Route, file: &str, source: &str, spelling: &str) {
    let start_byte = source.find(spelling).unwrap();
    assert_eq!(
        row.site,
        Site {
            file: file.into(),
            line: source[..start_byte].split('\n').count(),
            start_byte,
            end_byte: start_byte + spelling.len(),
        }
    );
}

#[track_caller]
fn assert_declaration(row: &Route, raw: &str, literal: &str, method: Option<&str>) {
    assert_eq!(
        row.declaration,
        Some(Declaration {
            path: DeclaredPath {
                raw: raw.into(),
                literal: Some(literal.into()),
            },
            methods: method.map(|verb| MethodSet::known([verb])),
        })
    );
}

#[track_caller]
fn assert_blueprint(row: &Route, file: &str) {
    assert!(
        matches!(&row.receiver, Receiver::Router { identity, proof: None }
            if identity.file == file && identity.binding == "bp"),
        "expected a proved Blueprint from {file}, got {:?}",
        row.receiver
    );
    assert_ne!(row.candidate, Some(true));
    assert_eq!(row.reason, None);
}

#[track_caller]
fn assert_withheld_receiver(row: &Route, reason: Reason) {
    assert_eq!(row.receiver, Receiver::Unproven { reason });
    assert_eq!(row.candidate, Some(true));
    assert_eq!(row.reason, Some(reason));
    assert!(row.paths.is_empty());
}

#[track_caller]
fn assert_unresolved(row: &Route, text: &str, reason: Reason) {
    assert_eq!(
        row.callbacks,
        [Callback {
            position: 0,
            status: CallbackStatus::Unresolved {
                text: text.into(),
                reason
            },
            delegate: None,
            role: None,
        }]
    );
}

#[track_caller]
fn assert_resolved_view(row: &Route) {
    assert!(matches!(
        row.callbacks.as_slice(),
        [Callback {
            position: 0,
            status: CallbackStatus::Resolved { identity, .. },
            delegate: None,
            ..
        }] if identity.name == "view"
    ));
}

#[test]
fn a_decorated_view_keeps_registration_when_callback_is_withheld() {
    let source = format!(
        "{BLUEPRINT}from guards import protect\n@bp.get(\"/x\")\n@protect\ndef view(): pass\n"
    );
    let control = source.replace("@protect\n", "");
    let table = listing(&[("views.py", &source)]);
    let clear = listing(&[("views.py", &control)]);
    let row = only_row(&table);
    assert_site(row, "views.py", &source, r#"@bp.get("/x")"#);
    assert_blueprint(row, "views.py");
    assert_declaration(row, r#""/x""#, "/x", Some("GET"));
    assert_unresolved(row, "view", Reason::InnerDecorator);
    let clear = only_row(&clear);
    assert_eq!(clear.declaration, row.declaration);
    assert_eq!(clear.receiver, row.receiver);
    assert_resolved_view(clear);
}

#[test]
fn b_escaped_factory_mount_keeps_imported_blueprint_rule() {
    let views = format!("{BLUEPRINT}{SHORTCUT}");
    let clean_factory = FACTORY.replace("db.init_app(app)", "pass");
    let table = listing(&[("app.py", FACTORY), ("views.py", &views)]);
    let clean = listing(&[("app.py", &clean_factory), ("views.py", &views)]);
    let row = only_row(&table);
    assert_site(row, "views.py", &views, r#"@bp.get(r"/x")"#);
    assert_declaration(row, r#"r"/x""#, "/x", Some("GET"));
    assert_withheld_receiver(row, Reason::ReceiverUnproven);
    assert_unresolved(row, "view", Reason::ReceiverUnproven);
    let clean = only_row(&clean);
    assert_eq!(clean.site, row.site);
    assert_eq!(clean.declaration, row.declaration);
    assert_blueprint(clean, "views.py");
    assert_resolved_view(clean);
}

#[test]
fn c_terminal_reexport_keeps_its_specific_refusal() {
    let init = "from .owner import bp\n";
    let via_package = format!("from pkg import bp\n{SHORTCUT}");
    let direct = via_package.replace("from pkg import", "from pkg.owner import");
    let list = |views: &str| {
        listing(&[
            ("pkg/__init__.py", init),
            ("pkg/owner.py", BLUEPRINT),
            ("views.py", views),
        ])
    };
    let table = list(&via_package);
    let direct_table = list(&direct);
    let row = only_row(&table);
    assert_site(row, "views.py", &via_package, r#"@bp.get(r"/x")"#);
    assert_declaration(row, r#"r"/x""#, "/x", Some("GET"));
    assert_withheld_receiver(row, Reason::TerminalReexport);
    assert_unresolved(row, "view", Reason::TerminalReexport);
    let direct_row = only_row(&direct_table);
    assert_eq!(direct_row.declaration, row.declaration);
    assert_blueprint(direct_row, "pkg/owner.py");
    assert_resolved_view(direct_row);
}

#[test]
fn d_class_view_keeps_registration_without_claiming_a_callback() {
    let source = format!(
        "{BLUEPRINT}from flask.views import View\nclass C(View):\n    def dispatch_request(self): pass\ndef view(): pass\nbp.add_url_rule(\"/c\", view_func=C.as_view(\"c\"))\n"
    );
    let plain = source.replace(r#"C.as_view("c")"#, "view");
    let table = listing(&[("views.py", &source)]);
    let plain_table = listing(&[("views.py", &plain)]);
    let row = only_row(&table);
    assert_site(
        row,
        "views.py",
        &source,
        r#"bp.add_url_rule("/c", view_func=C.as_view("c"))"#,
    );
    assert_blueprint(row, "views.py");
    assert_declaration(row, r#""/c""#, "/c", None);
    assert_unresolved(row, r#"C.as_view("c")"#, Reason::ClassView);
    let plain_row = only_row(&plain_table);
    assert_eq!(plain_row.declaration, row.declaration);
    assert_eq!(plain_row.receiver, row.receiver);
    assert_resolved_view(plain_row);
}

#[test]
fn e_unproved_formal_keeps_evidence_without_registration_credit() {
    let source = format!("{BLUEPRINT}{FORMAL}");
    let hooked = source.replace("@external.record_once", "@bp.record_once");
    let table = listing(&[("views.py", &source)]);
    let hooked_table = listing(&[("views.py", &hooked)]);
    let row = only_row(&table);
    assert_site(
        row,
        "views.py",
        &source,
        r#"bp.add_url_rule("/formal", view_func=view, methods=["POST"])"#,
    );
    assert_declaration(row, r#""/formal""#, "/formal", Some("POST"));
    assert_withheld_receiver(row, Reason::ReceiverUnproven);
    assert_unresolved(row, "view", Reason::ReceiverUnproven);
    assert_eq!((table.summary().proven, table.summary().unproven), (0, 1));
    let hooked_row = only_row(&hooked_table);
    assert_eq!(hooked_row.declaration, row.declaration);
    assert_blueprint(hooked_row, "views.py");
    assert_resolved_view(hooked_row);
    assert_eq!(
        (
            hooked_table.summary().proven,
            hooked_table.summary().unproven
        ),
        (1, 0)
    );
}

#[test]
fn cr1_same_suffix_keeps_distinct_origins_without_prefix_completion() {
    let views = format!("{BLUEPRINT}{SHORTCUT}");
    let sources = [("other.py", views.as_str()), ("views.py", views.as_str())];
    let clean = listing(&sources);
    let table = listing(&[sources[0], sources[1], ("app.py", FACTORY)]);
    let [orphan, escaped] = table.routes() else {
        panic!(
            "expected two same-suffix Flask registrations, got {}",
            table.routes().len()
        );
    };
    let [clean_orphan, clean_child] = clean.routes() else {
        panic!("expected two independent Blueprint declarations without the factory");
    };
    for (row, control, file) in [
        (orphan, clean_orphan, "other.py"),
        (escaped, clean_child, "views.py"),
    ] {
        assert_site(row, file, &views, r#"@bp.get(r"/x")"#);
        assert_declaration(row, r#"r"/x""#, "/x", Some("GET"));
        assert_eq!(row.site, control.site);
        assert!(row.paths.iter().all(|path| {
            path.root.is_none() && !matches!(path.status, ExposureStatus::Full { .. })
        }));
        assert_blueprint(control, file);
    }
    assert_blueprint(orphan, "other.py");
    assert_withheld_receiver(escaped, Reason::ReceiverUnproven);
}

#[test]
fn cr2_omitted_methods_serialize_as_null() {
    let source = format!("{BLUEPRINT}@bp.route(\"/x\")\ndef view(): pass\n");
    let explicit = source.replace(r#"route("/x")"#, r#"route("/x", methods=["GET"])"#);
    let table = listing(&[("views.py", &source)]);
    let explicit_table = listing(&[("views.py", &explicit)]);
    let row = only_row(&table);
    assert_declaration(row, r#""/x""#, "/x", None);
    let wire = serde_json::to_value(row).unwrap();
    assert_eq!(wire.get("methods"), Some(&serde_json::Value::Null));
    assert_eq!(wire["default_methods"], json!({"known": ["GET"]}));
    let explicit_row = only_row(&explicit_table);
    assert_declaration(explicit_row, r#""/x""#, "/x", Some("GET"));
    assert_eq!(
        serde_json::to_value(explicit_row).unwrap()["methods"],
        json!({"known": ["GET"]})
    );
}

#[test]
fn summary_proven_counts_receivers_independently_of_callbacks() {
    let views = format!(
        "{BLUEPRINT}from guards import protect\n@bp.get('/clear')\ndef view(): pass\n@bp.get('/wrapped')\n@protect\ndef wrapped(): pass\n"
    );
    let formal = format!("{BLUEPRINT}{FORMAL}");
    let table = listing(&[("views.py", &views), ("formal.py", &formal)]);
    assert_eq!(
        serde_json::to_value(table.summary()).unwrap(),
        json!({
            "enabled_adapters": ["flask"], "registrations": 3, "proven": 2,
            "unproven": 1, "candidates": 1, "mounts": 0, "diagnostics": [], "failed_roots": []
        })
    );
    let handler_proved = |table: &RouteTable| {
        let registrations = table
            .routes()
            .iter()
            .filter(|row| matches!(row.receiver, Receiver::Router { .. }));
        assert_eq!(registrations.clone().count(), table.summary().proven);
        registrations
            .filter(|row| {
                !row.callbacks.is_empty()
                    && row.callbacks.iter().all(|callback| {
                        matches!(
                            callback.status,
                            CallbackStatus::Resolved { .. } | CallbackStatus::Anonymous { .. }
                        )
                    })
            })
            .count()
    };
    assert_eq!(handler_proved(&table), 1);

    let clear = views.replace("@protect\n", "");
    let control = listing(&[("views.py", &clear), ("formal.py", &formal)]);
    assert_eq!(control.summary(), table.summary());
    assert_eq!(handler_proved(&control), 2);
}

#[test]
fn fold1_unproved_receiver_has_no_flask_default() {
    let table = listing(&[(
        "app.py",
        "from flask import Flask\napp = Flask(__name__)\napp = wrap(app)\napp.add_url_rule('/x', endpoint='x')\n",
    )]);
    let row = only_row(&table);
    assert_withheld_receiver(row, Reason::ReceiverUnproven);
    assert_eq!(row.default_methods, None);
    assert_eq!(row.endpoint.as_deref(), Some("x"));
    assert_declaration(row, "'/x'", "/x", None);
}

#[test]
fn fold1_public_counted_refusals_use_original_spans_and_predicate_order() {
    let source = concat!(
        "from flask import Flask\napp = Flask(__name__)\n",
        "from views import f as first\n",
        "from helpers import patch\n",
        "patch(first)\npatch((first))\n",
        "from broken import router\n",
        "router.add_url_rule('/lost')\n",
        "app.add_url_rule('/x', view_func=first)\n",
        "router = None\n",
    );
    let table = listing(&[
        ("app.py", source),
        ("views.py", "def f(): pass\n"),
        ("helpers.py", "def patch(v): pass\ndef patch(v): pass\n"),
        (
            "broken.py",
            "def __getattr__(name): raise AttributeError(name)\nrouter = None\n",
        ),
    ]);
    let row = only_row(&table);
    assert_eq!(row.default_methods, None);
    let diagnostics = serde_json::to_value(&table.summary().diagnostics).unwrap();
    let expected: Vec<_> = [
        ("patch(first)", "patch(first)", "multi_write"),
        ("patch((first))", "patch((first))", "multi_write"),
        ("from broken import router", "router", "getattr_module"),
    ]
    .into_iter()
    .map(|(enclosing, span, reason)| {
        let at = source.find(enclosing).unwrap() + enclosing.rfind(span).unwrap();
        serde_json::json!({
            "site": {"file": "app.py", "start_byte": at, "end_byte": at + span.len(),
                     "line": source[..at].bytes().filter(|&b| b == b'\n').count() + 1},
            "reason": reason,
        })
    })
    .collect();
    assert_eq!(diagnostics, serde_json::json!(expected));
}

#[test]
fn fold1_scope_directives_keep_traced_view_escapes() {
    for (import, setup, callback) in [
        ("from views import f\n", "def configure():\n    global f\n    patch(f)\n", "f"),
        ("import views\n", "def configure():\n    from views import f\n    def inner():\n        nonlocal f\n        patch(f)\n    inner()\n", "views.f"),
    ] {
        for (patch, expected) in [
            ("def patch(v): v.methods = ['POST']\n", None),
            ("def patch(v): pass\n", Some(MethodSet::known(["GET"]))),
        ] {
            let source = format!(
                "from flask import Flask\napp = Flask(__name__)\n{import}from helpers import patch\n{setup}configure()\napp.add_url_rule('/x', view_func={callback})\n"
            );
            let table = listing(&[
                ("app.py", &source),
                ("views.py", "def f(): pass\n"),
                ("helpers.py", patch),
            ]);
            let row = only_row(&table);
            assert_eq!(row.default_methods, expected, "{setup} {patch}");
            assert!(matches!(row.callbacks[0].status, CallbackStatus::Resolved { .. }));
            assert!(table.summary().diagnostics.is_empty());
        }
    }
}

#[test]
fn fold1_comprehension_body_and_later_iterable_are_not_direct_registrations() {
    let table = listing(&[(
        "app.py",
        concat!(
            "from flask import Flask\napp = Flask(__name__)\n",
            "def f(): pass\n",
            "[app.add_url_rule('/body', view_func=f) for x in xs]\n",
            "[None for x in xs for y in (app.add_url_rule('/later', view_func=f),)]\n",
            "[None for x in (lambda: app.add_url_rule('/lambda', view_func=f),)]\n",
        ),
    )]);
    assert!(table.routes().is_empty());
}

fn import_diagnostic(source: &str, enclosing: &str, span: &str, reason: Reason) -> Diagnostic {
    let start_byte = source.find(enclosing).unwrap() + enclosing.find(span).unwrap();
    Diagnostic {
        site: Site {
            file: "app.py".into(),
            start_byte,
            end_byte: start_byte + span.len(),
            line: source[..start_byte].split('\n').count(),
        },
        reason,
    }
}

#[test]
fn fold1_census_applies_to_receivers_and_escape_callees() {
    let source = concat!(
        "from flask import Flask\napp = Flask(__name__)\n",
        "from views import bp as router, f\nfrom helpers import patch\n",
        "def swap():\n    global router\n    from other import bp as router\n",
        "swap()\nrouter.add_url_rule('/lost', view_func=f)\n",
        "patch(f)\napp.add_url_rule('/x', view_func=f)\n",
        "patch = alternate\n",
    );
    let bp = "from flask import Blueprint\nbp = Blueprint('b', __name__)\ndef f(): pass\n";
    let table = listing(&[
        ("app.py", source),
        ("views.py", bp),
        ("other.py", bp),
        ("helpers.py", "def patch(v): pass\n"),
    ]);
    let row = only_row(&table);
    assert_eq!(row.default_methods, None);
    assert!(matches!(
        row.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    let refused_call = "router.add_url_rule('/lost', view_func=f)";
    // The receiver capture and the failed callee check have distinct spans.
    assert_eq!(
        table.summary().diagnostics,
        [
            import_diagnostic(
                source,
                "from views import bp as router, f",
                "bp",
                Reason::GlobalWrite
            ),
            import_diagnostic(source, refused_call, refused_call, Reason::GlobalWrite),
            import_diagnostic(source, "patch(f)", "patch(f)", Reason::MultiWrite),
        ]
    );
}

#[test]
fn fold1_refused_receiver_cannot_exempt_view_or_mount_child_escapes() {
    let source = concat!(
        "from flask import Flask\napp = Flask(__name__)\n",
        "from owner import upstream, child\nfrom views import f\n",
        "def replace():\n    global upstream\n    upstream = supplied_router\n",
        "replace()\n",
        "upstream.add_url_rule('/dropped', view_func=f)\n",
        "upstream.register_blueprint(child)\n",
        "app.add_url_rule('/view', view_func=f)\n",
    );
    let owners = concat!(
        "from flask import Flask, Blueprint\n",
        "upstream = Flask(__name__)\nchild = Blueprint('b', __name__)\n",
        "@child.get('/child')\ndef view(): pass\n",
    );
    let table = listing(&[
        ("app.py", source),
        ("owner.py", owners),
        ("views.py", "def f(): pass\n"),
    ]);
    let [first, second] = table.routes() else {
        panic!("expected only the grounded app rule and the child candidate");
    };
    let (view, child) = if first.site.file == "app.py" {
        (first, second)
    } else {
        (second, first)
    };
    assert_site(
        view,
        "app.py",
        source,
        "app.add_url_rule('/view', view_func=f)",
    );
    assert_declaration(view, "'/view'", "/view", None);
    assert!(matches!(view.receiver, Receiver::Router { .. }));
    assert!(matches!(
        view.callbacks[0].status,
        CallbackStatus::Resolved { .. }
    ));
    assert_eq!(view.default_methods, None);
    assert_site(child, "owner.py", owners, "@child.get('/child')");
    assert_declaration(child, "'/child'", "/child", Some("GET"));
    assert_withheld_receiver(child, Reason::ReceiverUnproven);
    let refused_call = "upstream.add_url_rule('/dropped', view_func=f)";
    // The registration and mount share the import span. Only the registration
    // supplies a view argument for the separate no-effect callee check.
    assert_eq!(
        table.summary().diagnostics,
        [
            import_diagnostic(
                source,
                "from owner import upstream, child",
                "upstream",
                Reason::GlobalWrite
            ),
            import_diagnostic(source, refused_call, refused_call, Reason::GlobalWrite),
        ]
    );
}

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

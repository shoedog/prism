//! Capture-side Stage D rows. Installation and dispatch belong to checkpoint 2.
use super::*;

const VIEWS: &str = "def f(): return 'first'\ndef g(): return 'second'\n";
const REG: &str = "app.add_url_rule('/x', endpoint='x', view_func=handler, methods=['GET'])";
const IMPORT: &str = "from views import f as handler";

fn captured(body: &str, diag: Option<(ImportRefusalReason, &str, &str)>, written: &'static str) {
    let files = files(&[("app.py", &app(body)), ("views.py", VIEWS)]);
    assert_capture(&files, diag, written);
}

fn assert_capture(
    files: &Files,
    diag: Option<(ImportRefusalReason, &str, &str)>,
    written: &'static str,
) {
    let (spec, diags) = match diag {
        Some((reason, statement, name)) => (
            withheld_view("/x", written),
            vec![(reason, "app.py", leak(statement.into()), leak(name.into()))],
        ),
        None => (proven_view("/x", "views.py", "f"), vec![]),
    };
    let spec = spec.endpoint("x").declared(&["GET"]);
    // Match written methods even when view metadata is withheld. The effective
    // method set can separately depend on the unresolved view.
    check(files, "app.py", rules(&[spec]), &diags);
}

fn register(name: &str) -> String {
    REG.replace("view_func=handler", &format!("view_func={name}"))
}

fn method_global(competing: bool) {
    let name = if competing { "__handler" } else { "alternate" };
    let import = "from views import f as _C__handler";
    captured(
        &format!("{import}\nclass C:\n    def install(self):\n        global {name}\n        from views import g as {name}\nC().install()\n{}\n", register("_C__handler")),
        competing.then_some((GlobalWrite, import, "f")),
        "_C__handler",
    );
}

#[test]
fn fk_86() {
    method_global(true);
}

#[test]
fn fk_c86() {
    method_global(false);
}

fn method_local(written: &'static str, competing: Option<&str>) {
    let name = competing.unwrap_or("alternate");
    let local = format!("from views import g as {name}");
    captured(
        &format!("from views import f as {written}\nclass C:\n    def install(self):\n        {local}\n        {}\nC().install()\n", register(written)),
        competing.map(|_| (MultiWrite, local.as_str(), "g")),
        written,
    );
}

#[test]
fn fk_87() {
    method_local("handler", Some("handler"));
}

#[test]
fn fk_c87() {
    method_local("handler", None);
}

#[test]
fn fk_88() {
    method_local("_C__handler", Some("__handler"));
}

#[test]
fn fk_c88() {
    method_local("_C__handler", None);
}

fn subsequent_assignment(competing: bool) {
    let name = if competing { "handler" } else { "alternate" };
    captured(
        &format!("{IMPORT}\nfrom views import g\n{name} = g\n{REG}\n"),
        competing.then_some((MultiWrite, IMPORT, "f")),
        "handler",
    );
}

#[test]
fn fk_89() {
    subsequent_assignment(true);
}

#[test]
fn fk_c89() {
    subsequent_assignment(false);
}

fn global_rebind(name: &str) -> String {
    format!("def replace():\n    global {name}\n    from views import g as {name}\nreplace()\n")
}

fn function_global(competing: bool) {
    let name = if competing { "handler" } else { "alternate" };
    captured(
        &format!("{IMPORT}\n{}{REG}\n", global_rebind(name)),
        competing.then_some((GlobalWrite, IMPORT, "f")),
        "handler",
    );
}

#[test]
fn fk_90() {
    function_global(true);
}

#[test]
fn fk_c90() {
    function_global(false);
}

#[test]
fn fk_91() {
    let files = plus(
        pkg_routes("pkg", "", "/routes"),
        &[
            (
                "app.py",
                &mount_app("import pkg\nimport pkg.routes\n", "pkg.routes.bp"),
            ),
            ("tests/test_unrelated.py", "import pkg.routes\n"),
        ],
    );
    check(
        &files,
        "app.py",
        mount_unproven(),
        &[(
            MultiWrite,
            "app.py",
            "app.register_blueprint(pkg.routes.bp)",
            "pkg.routes.bp",
        )],
    );
}

fn first_iterable(competing: bool) {
    let name = if competing { "handler" } else { "other" };
    for (left, right) in [
        ("[None", "]"),
        ("(None", ")"),
        ("{None", "}"),
        ("{0: None", "}"),
    ] {
        captured(
            &format!("{IMPORT}\n{left} for {name} in ({REG},){right}\n"),
            competing.then_some((MultiWrite, IMPORT, "f")),
            "handler",
        );
    }
}

#[test]
fn fk_92() {
    first_iterable(true);
}

#[test]
fn fk_c92() {
    first_iterable(false);
}

fn assignment_alias(global: &str) {
    captured(
        &format!(
            "{IMPORT}\n{}callback = handler\n{}\n",
            global_rebind(global),
            register("callback")
        ),
        None,
        "callback",
    );
}

#[test]
fn fk_93() {
    assignment_alias("handler");
}

#[test]
fn fk_c93() {
    assignment_alias("alternate");
}

fn wildcard(export: &str) {
    let files = files(&[
        (
            "app.py",
            &app(&format!("{IMPORT}\nfrom support import *\n{REG}\n")),
        ),
        ("views.py", VIEWS),
        (
            "support.py",
            &format!("from views import g as {export}\n__all__ = ['{export}']\n"),
        ),
    ]);
    assert_capture(&files, None, "handler");
}

#[test]
fn fk_94() {
    wildcard("handler");
}

#[test]
fn fk_c94() {
    wildcard("alternate");
}

fn type_parameter(name: &str) {
    captured(
        &format!("{IMPORT}\ndef install[{name}]():\n    {REG}\ninstall()\n"),
        None,
        "handler",
    );
}

#[test]
fn fk_95() {
    type_parameter("handler");
}

#[test]
fn fk_c95() {
    type_parameter("other");
}

#[test]
fn closed_binding_forms_count_after_the_read_and_in_nested_scopes() {
    for tail in [
        "import other as handler",
        "def handler(): pass",
        "class handler: pass",
        "def local(handler): pass",
        "def local(*handler: object): pass",
        "consume(lambda handler, /: None)",
        "type handler = object",
        "type handler[T] = list[T]",
        "ｈａｎｄｌｅｒ = None",
        "handler += other",
        "handler: object = other",
        "consume((handler := other))",
        "for handler in other: pass",
        "[None for handler in other]",
        "a, *handler = other",
        "with other() as (a, handler): pass",
        "try: pass\nexcept Error as handler: pass",
        "try: pass\nexcept* Error as handler: pass",
        "match other:\n    case {'result': handler}: pass",
        "del handler",
        "def local():\n    handler = None",
    ] {
        captured(
            &format!("{IMPORT}\n{REG}\n{tail}\n"),
            Some((MultiWrite, IMPORT, "f")),
            "handler",
        );
    }
}

#[test]
fn annotation_and_directives_do_not_count_as_capturing_writes() {
    captured(
        &format!("{IMPORT}\n{REG}\nhandler: object\ndef local():\n    global handler\n"),
        None,
        "handler",
    );
}

#[test]
fn higher_priority_source_refusal_beats_hidden_capture_write() {
    let files = files(&[
        (
            "app.py",
            &app(&format!(
                "{IMPORT}\nfrom views import g\nhandler = g\n{REG}\n"
            )),
        ),
        (
            "views.py",
            &format!("{VIEWS}\ndef __getattr__(name): raise AttributeError(name)\n"),
        ),
    ]);
    assert_capture(&files, Some((GetattrModule, IMPORT, "f")), "handler");
}

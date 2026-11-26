//! Shapes of the one open review finding (rev-static-sol-5 finding 1, rev-static-opus-5 N1):
//! the capturing module's OWN other bindings of the captured as-name are not checked by
//! `global_write` / `multi_write`. Examples are a `global` write in a method, a privately
//! declared alias that mangles to the captured name and a method-local import. No pinned
//! Stage D row has this shape (fk-20, fk-35 and fk-69 write the SOURCE module, not the
//! capturing one).
//!
//! Each shape has a mangled form (where it applies), an unmangled twin and a clean control.
//! Every test asserts both outcomes of the owner decision through
//! [`CAPTURING_MODULE_BINDINGS_CHECKED`], so only that constant changes when the owner decides.
//!
//! Skipped clauses (checkpoint 2): dispatch binding and exposure. The private-alias shapes
//! register inside a method, which `flask_facts` does not visit, so they assert the refusal
//! multiset only.
use super::*;

/// Owner decision pending. `true`: the rule withholds the capture with `global_write` or
/// `multi_write` on the captured span when the capturing module binds the captured as-name
/// elsewhere (through `global`, a mangled private declaration or a local import). `false`:
/// those bindings are declared out of model, prism proves the written capture and each shape
/// is a known limit (native Python binds the competing import).
const CAPTURING_MODULE_BINDINGS_CHECKED: bool = false;

const VIEWS: &str =
    "def _C__f(): return \"mangled\"\ndef f(): return \"first\"\ndef g(): return \"second\"\n";

fn global_files(first: &str, class_or_def: &str, register: &str) -> Files {
    files(&[
        (
            "app.py",
            &app(&format!(
                "from views import _C__f as {first}\n{class_or_def}{register}\n"
            )),
        ),
        ("views.py", VIEWS),
    ])
}

/// Decide the outcome for a registration of `name` captured through `from views import _C__f`.
fn global_outcome(files: &Files, name: &'static str, competing: bool) {
    if competing && CAPTURING_MODULE_BINDINGS_CHECKED {
        let enclosing = leak(format!("from views import _C__f as {name}"));
        check(
            files,
            "app.py",
            rules(&[withheld_view("/x", name)]),
            &[(GlobalWrite, "app.py", enclosing, "_C__f")],
        );
    } else {
        // Out of model, or no competing binding: the written capture is proved.
        check(
            files,
            "app.py",
            rules(&[proven_view("/x", "views.py", "_C__f")]),
            &[],
        );
    }
}

const METHOD_GLOBAL: &str =
    "class C:\n    def m(self):\n        global __h\n        from views import f as __h\nC().m()\n";

#[test]
fn global_write_in_method_to_mangled_alias() {
    // Native: the method's store goes to the global `_C__h`; the endpoint binds `views.f`.
    let files = global_files(
        "_C__h",
        METHOD_GLOBAL,
        "app.add_url_rule('/x', view_func=_C__h)",
    );
    global_outcome(&files, "_C__h", true);
}

#[test]
fn global_write_in_method_unmangled_twin() {
    let class =
        "class C:\n    def m(self):\n        global h\n        from views import f as h\nC().m()\n";
    let files = global_files("h", class, "app.add_url_rule('/x', view_func=h)");
    global_outcome(&files, "h", true);
}

#[test]
fn global_write_in_method_clean_control() {
    // The method writes a different global (`alt`): nothing competes with the capture.
    let class = "class C:\n    def m(self):\n        global alt\n        from views import f as alt\nC().m()\n";
    let files = global_files("h", class, "app.add_url_rule('/x', view_func=h)");
    global_outcome(&files, "h", false);
}

#[test]
fn global_write_in_method_mangled_clean_control() {
    // `__k` mangles to `_C__k`, a different name from the captured `_C__h`.
    let class = "class C:\n    def m(self):\n        global __k\n        from views import f as __k\nC().m()\n";
    let files = global_files("_C__h", class, "app.add_url_rule('/x', view_func=_C__h)");
    global_outcome(&files, "_C__h", false);
}

#[test]
fn global_write_in_function() {
    let def = "def m():\n    global h\n    from views import f as h\nm()\n";
    let files = global_files("h", def, "app.add_url_rule('/x', view_func=h)");
    global_outcome(&files, "h", true);
}

#[test]
fn global_write_in_function_clean_control() {
    let def = "def m():\n    global alt\n    from views import f as alt\nm()\n";
    let files = global_files("h", def, "app.add_url_rule('/x', view_func=h)");
    global_outcome(&files, "h", false);
}

/// The private-alias shape: a module-level `from views import f as <alias>` and a method that
/// reads `<alias>` after the method-local import `local_import`.
fn alias_files(alias: &str, local_import: &str) -> Files {
    files(&[
        (
            "app.py",
            &app(&format!(
                "from views import f as {alias}\nclass C:\n    def install(self):\n{local_import}        app.add_url_rule('/x', view_func={alias})\nC().install()\n"
            )),
        ),
        ("views.py", VIEWS),
    ])
}

fn alias_outcome(files: &Files, competing: bool) {
    if competing && CAPTURING_MODULE_BINDINGS_CHECKED {
        // The method-local `__handler` mangles to `_C__handler` and binds `views.g`.
        check(
            files,
            "app.py",
            nothing(),
            &[(
                MultiWrite,
                "app.py",
                "from views import f as _C__handler",
                "f",
            )],
        );
    } else {
        check(files, "app.py", nothing(), &[]);
    }
}

#[test]
fn private_alias_shadows_captured_name_in_method() {
    // Native: Python mangles `__handler` to `_C__handler`, so Flask binds `views.g`.
    let files = alias_files("_C__handler", "        from views import g as __handler\n");
    alias_outcome(&files, true);
}

#[test]
fn private_alias_unmangled_twin() {
    // `handler` is a plain local: the read resolves lexically to the method-local import of
    // `g`, which is no competing module binding in either decision.
    let files = alias_files("handler", "        from views import g as handler\n");
    check(&files, "app.py", nothing(), &[]);
}

#[test]
fn private_alias_different_name_clean_control() {
    // `_handler` is not `_C__handler`: nothing competes with the capture.
    let files = alias_files("_C__handler", "        from views import g as _handler\n");
    alias_outcome(&files, false);
}

#[test]
fn private_alias_clean_control() {
    alias_outcome(&alias_files("_C__handler", ""), false);
}

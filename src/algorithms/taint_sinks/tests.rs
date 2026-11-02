//! Rule-table checks: each rule's name and CWE tag, read straight off the classifier.

use super::*;

/// Classify the first identifier named `name` on `line` of `source`.
fn hit(language: Language, source: &str, line: usize, name: &str) -> Option<SinkHit> {
    let parsed = ParsedFile::parse("t", source, language).unwrap();
    let ids = parsed.identifiers_on_line(line);
    let id = ids
        .iter()
        .find(|n| parsed.node_text(n) == name)
        .unwrap_or_else(|| panic!("no identifier {name} on line {line}"));
    SinkClassifier::default().classify("t", &parsed, *id, name, &["exec", "get_user"])
}

fn tag(h: Option<SinkHit>) -> Option<(&'static str, &'static str)> {
    h.map(|h| (h.rule, h.cwe.unwrap_or("none")))
}

#[test]
fn js_rules_carry_cwe_tags() {
    let js = Language::JavaScript;
    let cases = [
        ("(0, eval)(x);", "eval", Some(("js-eval", "CWE-95"))),
        (
            "new Function(x);",
            "Function",
            Some(("js-function-ctor", "CWE-95")),
        ),
        ("db.query(x);", "query", Some(("js-db-call", "CWE-89"))),
        (
            "res.redirect(x);",
            "redirect",
            Some(("js-redirect", "CWE-601")),
        ),
        (
            "window.location.href = x;",
            "location",
            Some(("js-location", "CWE-601")),
        ),
        (
            "el.innerHTML = x;",
            "innerHTML",
            Some(("js-dom-html", "CWE-79")),
        ),
        ("res.send(x);", "send", Some(("js-response", "CWE-79"))),
        ("const sanitizedQuery = x;", "sanitizedQuery", None),
        ("const t = el.innerHTML;", "innerHTML", None),
        ("re.exec(x);", "exec", None),
    ];
    for (src, name, want) in cases {
        assert_eq!(tag(hit(js, src, 1, name)), want, "{src}");
    }
}

#[test]
fn js_child_process_binding_forms_are_tagged() {
    let js = Language::JavaScript;
    let bound = [
        "const sh = require('node:child_process');\nsh.execSync(x);\n",
        "import * as sh from 'child_process';\nsh.execSync(x);\n",
        "import sh = require('child_process');\nsh.execSync(x);\n",
    ];
    for src in bound {
        let lang = if src.contains("= require") {
            Language::TypeScript
        } else {
            js
        };
        assert_eq!(
            tag(hit(lang, src, 2, "execSync")),
            Some(("js-child-process", "CWE-78")),
            "{src}"
        );
    }
    let unbound = "const sh = makeShell();\nsh.execSync(x);\n";
    assert_eq!(tag(hit(js, unbound, 2, "execSync")), None);
}

#[test]
fn python_and_go_rules_carry_cwe_tags() {
    let py = Language::Python;
    assert_eq!(
        tag(hit(py, "os.system(x)\n", 1, "system")),
        Some(("py-os-command", "CWE-78"))
    );
    assert_eq!(
        tag(hit(py, "cur.execute(x)\n", 1, "execute")),
        Some(("py-db-call", "CWE-89"))
    );
    assert_eq!(
        tag(hit(py, "q.filter(x)\n", 1, "filter")),
        Some(("py-sql-clause", "CWE-89"))
    );
    assert_eq!(tag(hit(py, "getattr(o, 'a')\n", 1, "getattr")), None);
    let go = Language::Go;
    let src = "package p\nfunc h(w http.ResponseWriter, f string) {\n\tfmt.Fprintf(w, \"%s\", x)\n\tfmt.Sprintf(f, x)\n\tfmt.Sprintf(\"%d\", x)\n}\n";
    assert_eq!(
        tag(hit(go, src, 3, "Fprintf")),
        Some(("go-writer-output", "CWE-79"))
    );
    assert_eq!(
        tag(hit(go, src, 4, "Sprintf")),
        Some(("go-format-string", "CWE-134"))
    );
    assert_eq!(tag(hit(go, src, 5, "Sprintf")), None);
}

#[test]
fn legacy_languages_gate_bash_names_and_get_user() {
    let c = "void f(void) { get_user(x); get_user_id(x); }\n";
    // Only the exact kernel name fires in C.
    assert!(hit(Language::C, c, 1, "get_user").is_some());
    assert!(hit(Language::C, c, 1, "get_user_id").is_none());
    // `exec` stays a substring pattern for legacy languages; `=eval` is Bash-only.
    let parsed = ParsedFile::parse("t", "void f(void) { eval(x); }\n", Language::C).unwrap();
    let ids = parsed.identifiers_on_line(1);
    let id = ids.iter().find(|n| parsed.node_text(n) == "eval").unwrap();
    let patterns = ["=eval"];
    let classifier = SinkClassifier::default();
    assert!(classifier
        .classify("t", &parsed, *id, "eval", &patterns)
        .is_none());
}

#[test]
fn js_receiver_and_position_rules() {
    let js = Language::JavaScript;
    let cases = [
        // Position: only the callee counts, never a bare read or an object.
        ("obj.eval(x);", "eval", None),
        ("query.trim(x);", "query", None),
        ("foo(eval);", "eval", None),
        // Receiver: a response object sends, any other receiver does not.
        ("res.send(x);", "send", Some(("js-response", "CWE-79"))),
        ("socket.send(x);", "send", None),
        // Location: global-rooted assignment targets only, never reads or data fields.
        ("item.location = x;", "location", None),
        ("const t = window.location.href;", "location", None),
        (
            "location = x;",
            "location",
            Some(("js-location", "CWE-601")),
        ),
        (
            "res.setHeader('Location', x);",
            "setHeader",
            Some(("js-location", "CWE-601")),
        ),
        ("res.setHeader('X-Trace', x);", "setHeader", None),
        // A bare command-runner call needs no import in this file; spawn does.
        ("exec(x);", "exec", Some(("js-child-process", "CWE-78"))),
        ("spawn(x);", "spawn", None),
    ];
    for (src, name, want) in cases {
        assert_eq!(tag(hit(js, src, 1, name)), want, "{src}");
    }
    let destructured = "const { exec: run } = require('child_process');\nrun(x);\n";
    assert_eq!(
        tag(hit(js, destructured, 2, "run")),
        Some(("js-child-process", "CWE-78"))
    );
    let promisified =
        "const cp = require('child_process');\nconst run = util.promisify(cp.exec);\nrun(x);\n";
    assert_eq!(
        tag(hit(js, promisified, 3, "run")),
        Some(("js-child-process", "CWE-78"))
    );
}

#[test]
fn python_rule_guards() {
    let py = Language::Python;
    let runner = "runner.run(x)\n";
    assert_eq!(tag(hit(py, runner, 1, "run")), None);
    let sub = "import subprocess\nsubprocess.run(x)\n";
    assert_eq!(
        tag(hit(py, sub, 2, "run")),
        Some(("py-subprocess", "CWE-78"))
    );
    assert_eq!(tag(hit(py, "q.filter(pk=x)\n", 1, "filter")), None);
    assert_eq!(
        tag(hit(py, "q.filter(a == x)\n", 1, "filter")),
        Some(("py-sql-clause", "CWE-89"))
    );
    let literal_text = "import sqlalchemy as sa\nsa.text('select 1')\n";
    assert_eq!(tag(hit(py, literal_text, 2, "text")), None);
    let dynamic_text = "import sqlalchemy as sa\nsa.text('select ' + x)\n";
    assert_eq!(
        tag(hit(py, dynamic_text, 2, "text")),
        Some(("py-sql-clause", "CWE-89"))
    );
}

#[test]
fn go_rule_guards() {
    let go = Language::Go;
    let src = "package p\nconst f = \"%d\"\nfunc h(file *os.File, w http.ResponseWriter) {\n\tfmt.Sprintf(f, x)\n\tfmt.Fprintf(file, \"%s\", x)\n\tcache.Query(x)\n\tresponse.HTML(w, r, x)\n\tdb.Exec(x)\n\ttemplate.HTML(x)\n}\n";
    assert_eq!(tag(hit(go, src, 4, "Sprintf")), None);
    assert_eq!(tag(hit(go, src, 5, "Fprintf")), None);
    assert_eq!(tag(hit(go, src, 6, "Query")), None);
    assert_eq!(tag(hit(go, src, 7, "HTML")), None);
    let pkgs = "package p\nimport \"a/constant\"\nfunc h() {\n\tfmt.Sprintf(constant.Prompt, x)\n\tfmt.Sprintf(req.Format, x)\n}\n";
    assert_eq!(tag(hit(go, pkgs, 4, "Sprintf")), None);
    assert_eq!(
        tag(hit(go, pkgs, 5, "Sprintf")),
        Some(("go-format-string", "CWE-134"))
    );
    assert_eq!(tag(hit(go, src, 8, "Exec")), Some(("go-db-call", "CWE-89")));
    assert_eq!(
        tag(hit(go, src, 9, "HTML")),
        Some(("go-template-html", "CWE-79"))
    );
}

/// The structured `exec`/`execSync` sinks match any receiver by tail name; the
/// semantic check lets only command runs through.
#[test]
fn structured_js_exec_check_rejects_regex_receivers() {
    fn check(source: &str) -> bool {
        let parsed = ParsedFile::parse("t", source, Language::JavaScript).unwrap();
        let mut found = None;
        callee::walk(parsed.tree.root_node(), &mut |n| {
            if n.kind() == "call_expression" {
                found = Some(n);
            }
        });
        let call_node = found.expect("a call");
        js::exec_call_is_command(&crate::frameworks::CallSite {
            call_node,
            source: &parsed.source,
        })
    }
    assert!(!check("/a+/.exec(x);\n"));
    assert!(!check("re.exec(x);\n"));
    assert!(check("const cp = require('child_process');\ncp.exec(x);\n"));
    assert!(check("exec(x);\n"));
}

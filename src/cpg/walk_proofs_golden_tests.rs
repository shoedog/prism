use std::io::Write;
use std::path::Path;

use super::object_literal_arg_tests::parse_files;
use super::trace_descent_tests::occurrence;
use super::trace_proofs::with_walk_proofs;
use super::{CodePropertyGraph, ReturnFlowMode, VarAccess};
use crate::languages::Language;
use crate::reasoning::order::AstOrderView;

#[test]
fn off_trace_debug_matches_54fe1586() {
    use ReturnFlowMode::{Off, On};
    use VarAccess::{Def, Use};

    let gf = "function g(x) { sink(x); }\nfunction f(t) {\n  g(t);\n}\n";
    let w05 = include_str!("../../tests/taint_parity/w05/a.js");
    let l02 = include_str!("../../tests/taint_parity/l02/a.py");
    let l03 = include_str!("../../tests/taint_parity/l03/main.go");
    let local = "function h(a) {\n  const b = a;\n  sink(b);\n}\n";
    let recursive = "function f(x) {\n  f(x);\n}\n";
    let call = |source: &str, text: &str| source.find(text).unwrap() + 2;
    let cases: &[(_, _, _, _, &[(usize, VarAccess)], _, _)] = &[
        (
            "gf",
            Language::JavaScript,
            "js",
            gf,
            &[(call(gf, "g(t)"), Use)],
            Off,
            true,
        ),
        (
            "two",
            Language::JavaScript,
            "js",
            gf,
            &[(call(gf, "g(t)"), Use), (call(gf, "f(t)"), Def)],
            Off,
            true,
        ),
        (
            "local",
            Language::JavaScript,
            "js",
            local,
            &[(call(local, "h(a"), Def)],
            Off,
            true,
        ),
        (
            "w05",
            Language::JavaScript,
            "js",
            w05,
            &[(call(w05, "f(y)"), Use)],
            On,
            true,
        ),
        (
            "w05v",
            Language::JavaScript,
            "js",
            w05,
            &[(w05.find("v = p").unwrap(), Def)],
            On,
            false,
        ),
        (
            "l01",
            Language::JavaScript,
            "js",
            include_str!("../../tests/taint_parity/l01/a.js"),
            &[(166, Use)],
            Off,
            false,
        ),
        (
            "l02",
            Language::Python,
            "py",
            l02,
            &[(call(l02, "g(t)"), Use)],
            Off,
            false,
        ),
        (
            "l03",
            Language::Go,
            "go",
            l03,
            &[(call(l03, "g(t)"), Use)],
            Off,
            false,
        ),
        (
            "f01",
            Language::JavaScript,
            "js",
            include_str!("../../tests/taint_parity/f01/a.js"),
            &[(180, Def)],
            Off,
            false,
        ),
        (
            "self-param",
            Language::JavaScript,
            "js",
            recursive,
            &[(recursive.rfind("f(x)").unwrap() + 2, Use)],
            Off,
            false,
        ),
        (
            "self-return",
            Language::JavaScript,
            "js",
            include_str!("../../tests/taint_parity/s1-js-self-return/a.js"),
            &[(83, Def)],
            On,
            false,
        ),
        (
            "x01",
            Language::JavaScript,
            "js",
            include_str!("../../tests/taint_parity/x01/a.js"),
            &[(82, Def)],
            On,
            false,
        ),
    ];
    let mut debug = String::new();
    with_walk_proofs(None, || {
        for &(name, language, ext, source, positions, mode, ordered) in cases {
            let files = parse_files(language, &[("a", source)], ext);
            let cpg = CodePropertyGraph::build(&files);
            let roots: Vec<_> = positions
                .iter()
                .map(|&(at, access)| occurrence(&cpg, at, access))
                .collect();
            let view = AstOrderView::new(&cpg, &files);
            let trace = cpg.taint_trace_nodes_with_mode(&roots, ordered.then_some(&view), mode);
            debug.push_str(&format!("{name} {trace:?}\n"));
        }
        use super::taint_policy_return_tests as policy;
        let cpg = policy::build(&[
            (policy::MAIN, policy::MAIN_SRC),
            (policy::LIB, policy::LIB_SRC),
        ]);
        let trace = policy::trace(&cpg, &policy::sources(policy::MAIN, 3), None);
        debug.push_str(&format!("return-policy {trace:?}\n"));
    });
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg/walk_proofs_off_54fe1586.txt");
    if std::env::var_os("PRISM_IWALK_RECORD_BASE").is_some_and(|value| value == "1") {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&golden)
            .unwrap()
            .write_all(debug.as_bytes())
            .unwrap();
    } else {
        let expected = std::fs::read_to_string(&golden).unwrap_or_else(|error| {
            panic!(
                "{}: missing switch-off base recording: {error}",
                golden.display()
            )
        });
        assert_eq!(debug, expected);
    }
}

use super::*;

#[test]
fn c_header_keeps_wave_g_scope_labels_like_c_source() {
    let source = "void sink(int);\nstatic inline void f(int x) {\n  sink(x);\n  {\n    int x = 1;\n    sink(x);\n  }\n  sink(x);\n}\n";
    let mut observed = Vec::new();

    for extension in ["c", "h"] {
        let parsed = ParsedFile::parse(&format!("plain.{extension}"), source, Language::C)
            .expect("C fixture must parse");
        let func = function(&parsed);
        let parameter = def(&parsed, 0, "x", 2, 0, false);
        let inner_local = def(&parsed, 1, "x", 5, 0, false);
        let defs = vec![parameter.clone(), inner_local.clone()];
        let parameter_to_exit = edge(&parsed, &func, &parameter, 8);
        let local_to_exit = edge(&parsed, &func, &inner_local, 8);
        let outcome = run(
            &parsed,
            &defs,
            &[parameter_to_exit.clone(), local_to_exit.clone()],
        )
        .0;

        assert_label(&outcome, &parameter_to_exit, FlowConfidence::Exact);
        assert_label(
            &outcome,
            &local_to_exit,
            FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 }),
        );
        let RdOutcome::Available(result) = outcome else {
            unreachable!("assert_label checked availability")
        };
        observed.push((
            result.labels[&(parameter_to_exit.from, parameter_to_exit.to)],
            result.labels[&(local_to_exit.from, local_to_exit.to)],
        ));
    }

    assert_eq!(observed[0], observed[1], ".c and .h must use wave G labels");
}

use crate::common::{build_cpg, Language};
use prism::cpg::{CpgNode, VarAccess};

#[test]
fn assignment_target_keys_have_uses_on_the_write_line() {
    let owners = [
        ("declaration", "function put", "", "}"),
        ("method", "const setters = {\n  put", "", "}\n};"),
        ("bound arrow", "const put = ", " =>", "};"),
        ("anonymous callback", "visit(function ", "", "});"),
    ];
    let writes: [(&str, &[&str]); 4] = [
        ("o[k] = v;", &["k"]),
        ("a[k1][k2] = v;", &["k1", "k2"]),
        ("t[key] = s[key];", &["key"]),
        ("o[keys[keys.length - 1]] = v;", &["keys"]),
    ];

    let file = "key_use.js";
    for (owner, prefix, after_parameters, suffix) in owners {
        let write_line = prefix.lines().count() + 1;
        for (write, keys) in writes {
            let source = format!(
                "{prefix}(o, k, v, a, k1, k2, t, key, s, keys){after_parameters} {{\n    {write}\n{suffix}\n"
            );
            let cpg = build_cpg(file, &source, Language::JavaScript);
            let nodes = cpg.nodes_at(file, write_line);
            for key in keys {
                assert!(
                    nodes.iter().any(|&node| matches!(
                        cpg.node(node),
                        CpgNode::Variable { path, access: VarAccess::Use, .. }
                            if path.base == *key
                    )),
                    "{owner}: expected a Use of `{key}` on line {write_line} for `{write}`"
                );
            }
        }
    }
}

#[test]
fn selection_step_keys_have_exact_or_declarator_anchor_uses() {
    let source = "function put(o, keys, users, id) {\n  for (const s of keys) {\n    o = o[s];\n  }\n  const u = users[id];\n  return u;\n}\n";
    let file = "key_read.js";
    let cpg = build_cpg(file, source, Language::JavaScript);
    let step = source.find("o[s]").unwrap() + 2;
    let anchor = source.find("  const u").unwrap();
    for (key, line, start, end) in [("s", 3, step, step + 1), ("id", 5, anchor, anchor)] {
        let nodes = cpg.nodes_at(file, line);
        assert!(
            nodes.iter().any(|&node| matches!(
                cpg.node(node),
                CpgNode::Variable {
                    path, access: VarAccess::Use, function, function_start_line: 1,
                    line: actual_line, start_byte, end_byte, ..
                } if path.base == key && function == "put" && *actual_line == line
                    && (*start_byte, *end_byte) == (start, end)
            )),
            "missing {key} Use in put at line {line}, bytes {start}..{end}: {:?}",
            nodes.iter().map(|&node| cpg.node(node)).collect::<Vec<_>>(),
        );
    }
}

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

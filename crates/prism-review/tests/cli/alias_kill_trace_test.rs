//! #783: the trace names an edge a write through an alias moved (`kill:alias` for the may-kill, `alias-d3` for
//! an endpoint-node order, `alias-d2c` for a write in a nested callable), through the real producer defs
//! (twin included), which the in-crate solver tests cannot feed it. Each row is a source, the edge, and the
//! mechanism the trace must name for it ("" : none of the alias mechanisms).
use serde_json::Value;

const ROWS: &[(&str, &str, &str, &str)] = &[
    ("twin_assign", "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}\n", "o.x@3->o.x@5", "kill:alias"),
    ("census_incr", "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}\n", "o.x@3->o.x@5", "kill:alias"),
    ("param_alias", "function f(o, u = o) {\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}\n", "o.x@2->o.x@4", "kill:alias"),
    ("use_node_write", "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1; return sink(o.x);\n}\n", "o.x@3->o.x@4", "alias-d3"),
    ("closure_write", "function g(o) {\n  const u = o;\n  o.x = 0;\n  const f = () => {\n    u.x++;\n  };\n  return sink(o.x, f);\n}\n", "o.x@3->o.x@7", "alias-d2c"),
    ("fallthrough_branch", "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    u.x = 1;\n  }\n  return sink(o.x);\n}\n", "o.x@3->o.x@7", ""),
    ("reassigned_before", "function g(o) {\n  let u = o;\n  o.x = 0;\n  u = {};\n  u.x = 1;\n  return sink(o.x);\n}\n", "o.x@3->o.x@6", ""),
    ("read_before_write", "function g(o) {\n  const u = o;\n  o.x = 0;\n  return (sink(o.x), u.x = 1);\n}\n", "o.x@3->o.x@4", ""),
    ("declaration_twin", "function g(o) {\n  o.name = 0;\n  const { name } = o;\n  return sink(o.name, name);\n}\n", "o.name@2->o.name@4", ""),
];

fn run(source: &str) -> (Vec<String>, Vec<Value>) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.js"), source).unwrap();
    let out = crate::prism_std()
        .env("PRISM_RD_CENSUS_TRACE", "1")
        .args(["nav", "--no-cache", "dfg-stats", "--edges", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let trace = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with("rd-census\t"))
        .map(String::from)
        .collect();
    let edges = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    (trace, edges)
}

fn place(side: &Value) -> String {
    let path = &side["path"];
    let mut s = path["base"].as_str().unwrap().to_string();
    for f in path["fields"].as_array().unwrap() {
        s.push('.');
        s.push_str(f.as_str().unwrap());
    }
    format!("{s}@{}", side["line"])
}

#[test]
fn the_trace_names_the_alias_mechanism_of_each_moved_edge() {
    let mut failures = Vec::new();
    for (name, source, edge, want) in ROWS {
        let (trace, edges) = run(source);
        let (from, to) = edge.split_once("->").unwrap();
        let named: Vec<&str> = trace
            .iter()
            .filter(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                f.len() == 6
                    && f[4].starts_with(&format!("{from}:"))
                    && f[5].starts_with(&format!("{to}:"))
            })
            .map(|l| l.split('\t').nth(1).unwrap())
            .filter(|m| ["kill:alias", "alias-d3", "alias-d2c"].contains(m))
            .collect();
        let label = edges
            .iter()
            .find(|e| place(&e["from"]) == from && place(&e["to"]) == to)
            .map(|e| e["confidence"].as_str().unwrap().to_string());
        let expect: Vec<&str> = if want.is_empty() { vec![] } else { vec![want] };
        if named != expect {
            failures.push(format!(
                "{name} {edge}: want trace {expect:?}, got {named:?}"
            ));
        }
        if !want.is_empty() && label.as_deref() != Some("nameonly") {
            failures.push(format!(
                "{name} {edge}: label {label:?} does not match the trace"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

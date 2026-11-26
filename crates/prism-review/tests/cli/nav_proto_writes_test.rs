use prism::api::{nav_session, NavOptions};
use prism::navigation::proto_writes;
use serde_json::{json, Value};
use std::{fs, path::Path};

fn listing(root: &Path) -> Value {
    let out = crate::prism_bin()
        .unwrap()
        .args(["nav", "proto-writes", "--repo"])
        .arg(root)
        .args(["--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&out).unwrap()
}

#[test]
fn json_matches_core_query_in_file_line_and_target_order() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("z.ts"), "function put(o: any, k: string, p: string, v: any) {\n const child = o[k];\n child[p] = v;\n}\n").unwrap();
    fs::write(
        root.path().join("a.js"),
        "function put(a, k, p) {\n a[k][p] = external; a[k][p] = 1;\n}\n",
    )
    .unwrap();
    let got = listing(root.path());
    let mut options = NavOptions::default();
    options.no_cache = true;
    let session = nav_session(root.path(), &options).unwrap();
    assert_eq!(
        got,
        serde_json::to_value(proto_writes::proto_writes(&session)).unwrap()
    );
    let facts = got["facts"].as_array().unwrap();
    assert_eq!(facts.len(), 3);
    assert_eq!(facts[0]["file"], "a.js");
    assert_eq!(facts[1]["file"], "a.js");
    assert_eq!(facts[2]["file"], "z.ts");
    assert!(facts[0]["start_byte"].as_u64() < facts[1]["start_byte"].as_u64());
    let lines = got["name_reads"].as_array().unwrap();
    assert_eq!(
        lines
            .iter()
            .map(|line| (
                line["file"].as_str().unwrap(),
                line["line"].as_u64().unwrap()
            ))
            .collect::<Vec<_>>(),
        [("a.js", 2), ("z.ts", 2), ("z.ts", 3)]
    );
    let unknown = lines[0]["reads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|read| read["base"] == "external")
        .unwrap();
    assert_eq!(unknown["binding"], Value::Null);
    assert_eq!(unknown["resolution_complete"], false);
}

#[test]
fn json_has_two_empty_arrays_for_other_languages() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("app.py"),
        "def put(o, k, p):\n    o[k][p] = 1\n",
    )
    .unwrap();
    assert_eq!(
        listing(root.path()),
        json!({ "facts": [], "name_reads": [] })
    );
}

#[test]
fn default_text_shares_json_spellings() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("app.js"),
        "function put(o,k,p) { const child=o[k]; child[p]=1; }\n",
    )
    .unwrap();
    let output = crate::prism_bin()
        .unwrap()
        .args(["nav", "proto-writes", "--repo"])
        .arg(root.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    let value = listing(root.path());
    let fact = &value["facts"][0];
    assert!(text.contains(&format!(
        " {} guard={}",
        fact["class"].as_str().unwrap(),
        fact["guard"].as_str().unwrap()
    )));
    for operand in fact["key_operands"].as_array().unwrap() {
        assert!(text.contains(&format!(
            "  {} {}:",
            operand["role"].as_str().unwrap(),
            operand["name"].as_str().unwrap()
        )));
    }
}

#[test]
fn json_keeps_a_target_line_with_no_reads() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("app.js"),
        "function put(o,k,p) {\n o[k][\n\n p\n ] = 1;\n}\n",
    )
    .unwrap();
    let value = listing(root.path());
    assert_eq!(value["facts"].as_array().unwrap().len(), 1);
    assert!(value["name_reads"]
        .as_array()
        .unwrap()
        .contains(&json!({ "file": "app.js", "line": 3, "reads": [] })));
}

#[test]
fn fold_refusal_reason_is_listed_in_json_and_text() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("app.js"),
        "function put(o,k,eval){ eval('k=null');\n o[k].x=1; use(external); }",
    )
    .unwrap();
    let got = listing(root.path());
    let reads = got["name_reads"][0]["reads"].as_array().unwrap();
    assert!(reads.iter().any(|read| {
        read["base"] == "k"
            && read["resolution_complete"] == false
            && read["incomplete_reason"] == "direct_eval"
    }));
    assert!(reads.iter().any(|read| {
        read["base"] == "external"
            && read["resolution_complete"] == false
            && read.get("incomplete_reason").is_none()
    }));
    let out = crate::prism_bin()
        .unwrap()
        .args(["nav", "proto-writes", "--repo"])
        .arg(root.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("k resolution_complete=false incomplete_reason=direct_eval"));
}

#[test]
fn unclosed_interface_with_error_recovery_does_not_panic() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("a.ts"),
        "interface i4 {\n    [p];\n    [p2: string, p3: number];\ninterface i5 extends i1 { }\ninterface i6 extends i2 { }\ninterface i7 extends i3 { }\ninterface i8 extends i4 { }\n",
    )
    .unwrap();
    let value = listing(root.path());
    assert!(value["facts"].as_array().is_some());
}

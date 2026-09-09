use super::*;

fn parsed(file: &str, source: &str) -> ParsedFile {
    ParsedFile::parse(file, source, Language::TypeScript).unwrap()
}

#[test]
fn input_census_rejects_aliases_paths_languages_empty_and_parse_recovery() {
    for file in [
        "",
        "/app.ts",
        "./app.ts",
        "src/../app.ts",
        "src//app.ts",
        "src\\app.ts",
        "C:/app.ts",
        ".git/app.ts",
    ] {
        assert!(!relative(file), "{file}");
    }
    assert!(owned_inputs(BTreeMap::new()).is_err());
    let mut p = parsed("a.ts", "export class C {}");
    p.path = "b.ts".into();
    assert!(owned_inputs(BTreeMap::from([("a.ts".into(), p)])).is_err());
    let mut p = parsed("a.ts", "export class C {}");
    p.language = Language::Rust;
    assert!(owned_inputs(BTreeMap::from([("a.ts".into(), p)])).is_err());
    assert!(owned_inputs(BTreeMap::from([(
        "a.ts".into(),
        parsed("a.ts", "class C {")
    )]))
    .is_err());
}

#[test]
fn public_mutable_parsed_file_fields_do_not_authenticate_an_old_tree() {
    let mut p = parsed("a.ts", "class Wrong { other() {} }");
    p.source = "class Correct { m() {} }".into();
    let files = owned_inputs(BTreeMap::from([("a.ts".into(), p)])).unwrap();
    let names: Vec<_> = files["a.ts"]
        .functions()
        .iter()
        .filter_map(|f| f.name.as_deref())
        .collect();
    assert!(names.contains(&"m"));
    assert!(!names.contains(&"other"));
}

#[test]
fn input_byte_budget_is_checked_before_reparsing() {
    let mut p = parsed("a.ts", "");
    p.source = " ".repeat(8 * 1024 * 1024 + 1);
    assert!(
        matches!(owned_inputs(BTreeMap::from([("a.ts".into(),p)])),Err(e) if e=="input_budget")
    );
}

#[test]
fn anchors_require_digest_domain_parser_range_and_original_utf_encodings() {
    let source = "// 🦊\r\nclass C { m() {} }";
    let files = BTreeMap::from([("a.ts".into(), parsed("a.ts", source))]);
    let start = source.find("class").unwrap();
    let a = Anchor {
        file: "project/a.ts".into(),
        sha256: hash(source.as_bytes()),
        kind: "ClassKeyword".into(),
        start_byte: start,
        end_byte: start + 5,
        start_utf16: source[..start].encode_utf16().count(),
        end_utf16: source[..start + 5].encode_utf16().count(),
    };
    assert!(validate_anchor(&a, "ClassKeyword", &files).is_ok());
    let mutations: [fn(&mut Anchor); 10] = [
        |a| a.sha256 = "0".repeat(64),
        |a| a.kind = "Identifier".into(),
        |a| a.file = "compiler/a.ts".into(),
        |a| a.file = "project/other.ts".into(),
        |a| a.start_byte = a.end_byte,
        |a| a.end_byte = 1000,
        |a| a.start_byte = 4,
        |a| a.start_utf16 += 1,
        |a| a.end_utf16 += 1,
        |a| {
            a.start_byte += 1;
            a.start_utf16 += 1;
        },
    ];
    for mutate in mutations {
        let mut changed = a.clone();
        mutate(&mut changed);
        assert!(validate_anchor(&changed, "ClassKeyword", &files).is_err());
    }
}

#[test]
fn wire_observations_reject_unknown_fields_and_negative_ranges() {
    for text in [
        r#"{"schema":"prism.detached-owner/1","authorizes_runtime_edge":true}"#,
        r#"{"schema":"prism.detached-owner/1","authorizes_runtime_edge":false,"packet":{},"sources":[],"candidates":[],"refusals":[],"epoch":123}"#,
    ] {
        assert!(serde_json::from_str::<Observation>(text).is_err());
    }
    assert!(serde_json::from_str::<Anchor>(r#"{"file":"project/a.ts","sha256":"a","kind":"Identifier","start_utf16":0,"end_utf16":1,"start_byte":-1,"end_byte":1}"#).is_err());
}

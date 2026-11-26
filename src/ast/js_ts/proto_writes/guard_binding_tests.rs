use std::{fs, path::Path};

const RAW_LOOKUPS: &[&str] = &[
    ".read(",
    "::read(",
    ".read_at(",
    "::read_at(",
    ".declaration(",
    "::declaration(",
    ".binding(",
    "::binding(",
    ".has_binding(",
    "::has_binding(",
    ".lookup(",
    "::lookup(",
    ".member_read(",
    "::member_read(",
];

fn raw_lookups(source: &str) -> Vec<&'static str> {
    let compact: String = source
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect();
    RAW_LOOKUPS
        .iter()
        .flat_map(|&lookup| compact.match_indices(lookup).map(move |_| lookup))
        .collect()
}

fn exclude_lookup(source: &str, method: &str, lookup: &str) -> String {
    let (before, rest) = source.split_once(&format!("fn {method}(")).unwrap();
    let (body, after) = rest.split_once("\n    }\n").unwrap();
    assert_eq!(raw_lookups(body), [lookup], "{method}");
    [before, after].concat()
}

#[test]
fn proto_writes_guard_raw_binding_lookups_stay_in_adapter() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ast/js_ts/proto_writes");
    let adapter = dir.join("guards/bindings.rs");
    let mut paths = vec![dir.join("guards.rs"), dir.join("guards")];
    while let Some(path) = paths.pop() {
        if path.is_dir() {
            paths.extend(
                fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let mut source = fs::read_to_string(&path).unwrap();
        if path == adapter {
            source = exclude_lookup(&source, "bare_read", ".read(");
            // Shadow checks and the lexical census have separate identifier-only queries.
            source = exclude_lookup(&source, "is_shadowed", ".has_binding(");
            source = exclude_lookup(&source, "lexical_entry", ".lookup(");
        }
        let lookups = raw_lookups(&source);
        assert!(lookups.is_empty(), "{}: {lookups:?}", path.display());
    }
}

#[test]
fn proto_writes_guard_lookup_scan_detects_raw_calls() {
    let source = "
        // analysis.read(node)
        bindings.bare_read(node);
        bindings.bare_declaration(node);
        read.binding.as_ref();
    ";
    assert!(raw_lookups(source).is_empty());
    let source = "
        self.analysis
            // An intervening comment must not hide the call.
            .read (node);
        Analysis::read_at(&analysis, &span);
        self.declaration(node);
    ";
    assert_eq!(
        raw_lookups(source),
        [".read(", "::read_at(", ".declaration("]
    );
}

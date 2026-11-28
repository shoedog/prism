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

/// Trimmed non-comment lines of the guard sources that compare or look up a
/// name by its SPELLING. Every one is classified; a new one fails the scan
/// until it is classified here (and in the lane LEDGER).
///
/// PROPERTY: a member, field or label name, not a variable binding.
/// LITERAL: the text of a literal token.
/// BINDING: a spelling filter whose candidates are then resolved to the
///   binding (`lexical_entry` / `bare_read` equality).
/// NATIVE: a global's spelling selects the candidate; the sufficient tier is
///   proved by `native` (canonical name plus `is_shadowed`).
/// NATIVE-PARTIAL: as NATIVE, but a shadowed global keeps Partial (known
///   limit, constructed only: see the shadow tests).
const SPELLING_READS: &[(&str, &str, &str)] = &[
    (
        "finite.rs",
        r#".is_some_and(|constructor| self.text(constructor) == "Set")"#,
        "NATIVE",
    ),
    (
        "finite.rs",
        "&& self.bindings.canonical(self.text(node)) == name",
        "BINDING",
    ),
    (
        "flow.rs",
        ".is_some_and(|name| self.text(name) == self.text(label)))",
        "PROPERTY",
    ),
    (
        "names.rs",
        ".is_some_and(|(a, b)| self.text(a) == self.text(b))",
        "PROPERTY",
    ),
    (
        "names.rs",
        r#"|| self.bindings.canonical(self.text(callee)) != "String""#,
        "NATIVE",
    ),
    (
        "names.rs",
        r#"return matches!(self.text(base), "Object" | "Function" | "Array")"#,
        "NATIVE",
    ),
    (
        "names.rs",
        r#"&& field(node, "constructor").is_some_and(|callee| self.text(callee) == "Set")"#,
        "NATIVE-PARTIAL",
    ),
    ("names.rs", r#"&& self.text(offset) == "-1""#, "LITERAL"),
    (
        "names.rs",
        r#"&& field(receiver, "property").is_some_and(|p| self.text(p) == "hasOwnProperty")"#,
        "PROPERTY",
    ),
    (
        "own.rs",
        r#""hasOwn" if identifier(receiver) && self.text(receiver) == "Object" => {"#,
        "NATIVE-PARTIAL",
    ),
    (
        "own.rs",
        r#".is_some_and(|part| self.text(part) == "hasOwnProperty") =>"#,
        "PROPERTY",
    ),
    (
        "own.rs",
        r#"if self.text(field(prototype, "property")?) != "prototype""#,
        "PROPERTY",
    ),
    (
        "own.rs",
        r#"|| self.text(namespace) != "Object""#,
        "NATIVE-PARTIAL",
    ),
    (
        "own.rs",
        r#"&& field(ancestor, "label").is_some_and(|name| self.text(name) == self.text(label))"#,
        "PROPERTY",
    ),
    (
        "receiver.rs",
        r#"if constructor.kind() != "new_expression" || self.text(native) != "WeakSet" {"#,
        "NATIVE-PARTIAL",
    ),
    (
        "receiver.rs",
        ".is_some_and(|field| self.text(field) == brand_name)",
        "PROPERTY",
    ),
    (
        "receiver.rs",
        "&& self.text(node) == self.text(name)",
        "BINDING",
    ),
    (
        "receiver.rs",
        ".is_some_and(|name| self.text(name) == self.text(method))",
        "PROPERTY",
    ),
    (
        "header.rs",
        r#"|| !field(capture, "index").is_some_and(|index| self.text(index) == "1")"#,
        "LITERAL",
    ),
    (
        "header.rs",
        r#"if regex.kind() != "regex" || self.text(field(regex, "pattern")?) != r"^\[([^\]]+)\]$" {"#,
        "LITERAL",
    ),
    (
        "sequences.rs",
        r#"&& field(value, "index").is_some_and(|index| self.text(index) == "0")"#,
        "LITERAL",
    ),
    (
        "sequences.rs",
        r#"&& field(value, "constructor").is_some_and(|ctor| self.text(ctor) == "Set")"#,
        "NATIVE",
    ),
];

/// The only lines outside the adapter that may mention the analysis type or
/// a local called `analysis`: the constructors that hand it to `Bindings`.
const ANALYSIS_MENTIONS: &[(&str, &str)] = &[
    ("guards.rs", "name_reads::{Analysis, Origin},"),
    ("guards.rs", "analysis: &Analysis<'a>,"),
    ("guards.rs", "bindings: Bindings::new(analysis),"),
    ("natives.rs", "analysis: &Analysis<'a>,"),
    ("natives.rs", "bindings: Bindings::new(analysis),"),
];

fn is_spelling_read(line: &str) -> bool {
    (line.contains("text(") || line.contains("canonical("))
        && (line.contains("==") || line.contains("!=") || line.contains("matches!("))
}

fn mentions_analysis(line: &str) -> bool {
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == "analysis" || word == "Analysis")
}

fn guard_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ast/js_ts/proto_writes");
    let mut paths = vec![dir.join("guards.rs"), dir.join("guards")];
    let mut sources = Vec::new();
    while let Some(path) = paths.pop() {
        if path.is_dir() {
            paths.extend(
                fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            sources.push((name, fs::read_to_string(&path).unwrap()));
        }
    }
    sources
}

/// No guard proof reads a name by spelling unless the read is classified
/// above, and nothing outside the adapter touches the analysis (so a method
/// used as a value, a field access or a new analysis method cannot bypass
/// `Bindings::bare_read` either).
#[test]
fn proto_writes_guard_spelling_reads_are_classified() {
    let mut seen = Vec::new();
    let mut unclassified = Vec::new();
    for (file, source) in guard_sources() {
        for line in source.lines().map(str::trim) {
            if line.starts_with("//") {
                continue;
            }
            if is_spelling_read(line) {
                if SPELLING_READS
                    .iter()
                    .any(|(f, l, _)| *f == file && *l == line)
                {
                    seen.push((file.clone(), line.to_owned()));
                } else {
                    unclassified.push(format!("{file}: {line}"));
                }
            }
            if file != "bindings.rs"
                && mentions_analysis(line)
                && !ANALYSIS_MENTIONS
                    .iter()
                    .any(|(f, l)| *f == file && *l == line)
            {
                unclassified.push(format!("{file}: analysis used outside the adapter: {line}"));
            }
        }
    }
    assert!(unclassified.is_empty(), "{unclassified:#?}");
    for (file, line, class) in SPELLING_READS {
        assert!(
            seen.iter().any(|(f, l)| f == file && l == line),
            "stale {class} entry: {file}: {line}"
        );
    }
}

#[test]
fn proto_writes_guard_spelling_scan_detects_new_reads() {
    assert!(is_spelling_read("&& self.text(a) == self.text(b)"));
    assert!(is_spelling_read(
        "self.bindings.canonical(self.text(c)) != name"
    ));
    assert!(is_spelling_read(
        "return matches!(self.text(base), \"Object\")"
    ));
    assert!(!is_spelling_read("let text = self.text(node);"));
    assert!(mentions_analysis("self.analysis.origins.get(&k)"));
    assert!(mentions_analysis(".map(Analysis::read)"));
    assert!(!mentions_analysis("self.bindings.bare_read(node)"));
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

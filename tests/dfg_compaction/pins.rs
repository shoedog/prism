use super::common::*;
use sha2::{Digest, Sha256};
fn parse_pins(manifest: &str) -> BTreeMap<&str, &str> {
    let mut pins = BTreeMap::new();
    for line in manifest.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let digest = fields.next().unwrap();
        let name = fields.next().expect("missing parent pin case name");
        assert!(fields.next().is_none(), "extra parent pin fields: {name}");
        assert!(
            digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid parent pin sha256: {name}"
        );
        assert!(
            pins.insert(name, digest).is_none(),
            "duplicate parent pin: {name}"
        );
    }
    pins
}

pub(super) fn parent_pins() -> BTreeMap<&'static str, &'static str> {
    parse_pins(include_str!("pins.sha256"))
}

fn assert_parent_pin(name: &str, bytes: &[u8]) {
    let pins = parent_pins();
    let expected = pins
        .get(name)
        .unwrap_or_else(|| panic!("missing parent pin: {name}"));
    assert_eq!(
        format!("{:x}", Sha256::digest(bytes)),
        *expected,
        "parent pin {name}"
    );
}

#[test]
fn pin_parser_accepts_comments_and_blank_lines() {
    let digest = "a".repeat(64);
    let manifest = format!("# parent provenance\n\n  {digest}  example  \n");
    assert_eq!(
        parse_pins(&manifest),
        BTreeMap::from([("example", digest.as_str())])
    );
}

#[test]
fn pin_parser_rejects_invalid_entries() {
    let digest = "a".repeat(64);
    for (manifest, message) in [
        (digest.clone(), "missing parent pin case name"),
        ("bad example".into(), "invalid parent pin sha256: example"),
        (
            format!("{} example", "z".repeat(64)),
            "invalid parent pin sha256: example",
        ),
        (
            format!("{digest} example extra"),
            "extra parent pin fields: example",
        ),
        (
            format!("{digest} example\n{digest} example"),
            "duplicate parent pin: example",
        ),
    ] {
        let panic = std::panic::catch_unwind(|| parse_pins(&manifest)).unwrap_err();
        let text = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(
            text.contains(message),
            "{text:?} should contain {message:?}"
        );
    }
}

#[test]
#[should_panic(expected = "missing parent pin: missing-case")]
fn missing_parent_pin_fails_loudly() {
    assert_parent_pin("missing-case", b"");
}

#[test]
#[should_panic(expected = "parent pin generator-python")]
fn changed_parent_payload_is_rejected() {
    assert_parent_pin("generator-python", b"corrupted payload");
}

pub(super) fn index_pins() -> BTreeMap<&'static str, &'static str> {
    parse_pins(include_str!("indexes.sha256"))
}
#[test]
fn all_payload_and_lookup_pins() {
    prism::build_pool::install(|| {
        let indexes = index_pins();
        let mut seen = BTreeSet::new();
        super::cases::visit(|name, files| {
            let g = CodePropertyGraph::build(&files);
            let bytes = super::outputs::cpg_bytes(&g);
            assert_parent_pin(&name, &bytes);
            assert_eq!(
                format!(
                    "{:x}",
                    Sha256::digest(prism::cpg::dfg_compaction_support::lookup_indexes(&g))
                ),
                indexes[&*name],
                "lookup pin {name}"
            );
            seen.insert(name);
        });
        assert_eq!(seen, parent_pins().keys().map(|s| s.to_string()).collect());
        assert_eq!(seen, indexes.keys().map(|s| s.to_string()).collect());
    });
}

// Census: in `pins::` because a fixture added on main changes the population, which the
// controller rebinds with the hashes; CI's parity job skips `pins::` (review round 2, Opus S2).
#[test]
fn census_every_fixture_is_registered() {
    let mut paths = Vec::new();
    super::corpus::fixture_paths(Path::new("tests/fixtures"), &mut paths);
    let registered: BTreeSet<_> = super::corpus::REGISTERED
        .iter()
        .map(std::path::PathBuf::from)
        .collect();
    assert_eq!(
        paths.into_iter().collect::<BTreeSet<_>>(),
        registered,
        "register every fixture as a named case"
    );
}

#[test]
fn census_registered_cases_match_capture_population() {
    prism::build_pool::install(|| {
        let expected = super::cases::registered_names();
        let mut actual = BTreeSet::new();
        super::cases::visit(|name, _| {
            actual.insert(name);
        });
        assert_eq!(
            expected, actual,
            "capture population must cover every parity case"
        );
    });
}

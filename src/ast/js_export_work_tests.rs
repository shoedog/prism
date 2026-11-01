use super::{js_module_forwarding, ParsedFile};
use crate::languages::Language;

#[test]
fn repeated_ambient_exports_do_not_repeat_module_walks() {
    for tail in ["", "exports = other;", "module.exports = other;"] {
        let mut source = String::new();
        for i in 0..128 {
            source.push_str(&format!("exports.e{i} = \"value\";\n"));
        }
        source.push_str(tail);
        let parsed = ParsedFile::parse("work.js", &source, Language::JavaScript).unwrap();
        js_module_forwarding::tests::reset_visits();
        let facts = parsed.extract_js_ts_export_facts();
        assert_eq!(facts.named.len(), usize::from(tail.starts_with("module")));
        assert_eq!(
            facts.skipped_expr_count,
            if tail.is_empty() { 128 } else { 0 }
        );
        let visits = js_module_forwarding::tests::visits();
        assert!(
            visits <= 4 * parsed.parse_node_count,
            "{tail:?}: {visits} module-scan visits for {} tree nodes",
            parsed.parse_node_count
        );
    }
}

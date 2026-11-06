//! How `is_capture` places a use: real bytes first, the line-keyed reference identity only for a
//! zero-width line anchor (#660 S1a round 1).
use super::*;
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;

fn loc(path: &str, line: usize, start: usize, len: usize, kind: VarAccessKind) -> VarLocation {
    VarLocation {
        file: "a.js".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::from_expr(path),
        start_byte: start,
        end_byte: start + len,
        kind,
    }
}

/// `from` is the owner's parameter `o` (byte 11); the use is `path` at (line, start, len).
fn capture(src: &str, path: &str, line: usize, start: usize, len: usize) -> bool {
    let file = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    let owner = file.tree.root_node().child(0).unwrap();
    let facts = capture_facts_with(&file, owner, false);
    let edge = FlowEdge {
        from: loc("o", 1, 11, 1, VarAccessKind::Def),
        to: loc(path, line, start, len, VarAccessKind::Use),
    };
    is_capture(&edge, &facts)
}

const SRC: &str = "function f(o) {\n  g((z = o) => 1); use(o);\n}\n";

#[test]
fn a_precise_use_in_a_region_is_placed_by_its_bytes_whatever_its_spelling() {
    let start = SRC.find("z = o").unwrap() + 4;
    // `zzz` is spelled nowhere on the line: only the bytes can place it.
    assert!(capture(SRC, "zzz", 2, start, 1));
    assert!(capture(SRC, "o", 2, start, 1));
}

#[test]
fn a_precise_use_outside_every_region_is_eager_even_when_its_line_names_a_region_read() {
    let start = SRC.rfind("o)").unwrap();
    assert!(!capture(SRC, "o", 2, start, 1));
}

#[test]
fn a_zero_width_anchor_is_placed_by_its_line() {
    let line_start = SRC.find("  g(").unwrap();
    assert!(
        capture(SRC, "o", 2, line_start, 0),
        "named on the line by a region read"
    );
    assert!(!capture(SRC, "zzz", 2, line_start, 0), "named by nothing");
}

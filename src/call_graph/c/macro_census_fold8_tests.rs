//! #630 slice 6d-1 fold 8: a block comment holding a newline is handled by ONE policy only
//! when its line's status is PROVEN (the line began outside any literal and comment, and its
//! first token is a directive leader, or no leader byte sequence precedes the comment).
//! Anything else reads both variants (comment as one space; one space plus its newlines),
//! facts and refusals unioned.
use super::macro_census::{takes_second_comment_variant as second, CMacroCensus};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;

fn parsed(name: &str, src: &str) -> CMacroCensus {
    let lang = if name.ends_with(".c") || name.ends_with(".h") {
        Language::C
    } else {
        Language::Cpp
    };
    let mut m = BTreeMap::new();
    m.insert(
        name.to_string(),
        ParsedFile::parse(name, src, lang).unwrap(),
    );
    CMacroCensus::from_files(&m)
}

fn m_hidden(c: &CMacroCensus) {
    let m = c.get("M").expect("M read");
    assert!(m.free.contains("hidden"), "M.free = {:?}", m.free);
}

fn safe(c: &CMacroCensus) -> bool {
    c.incomplete() || c.get("M").is_some_and(|m| m.free.contains("hidden"))
}

/// sol's round-8 input: a raw string whose content line starts with `lead`, then a comment on
/// that very line, then a genuine directive after `*/`.
fn round8(lead: &str, def: &str) -> String {
    format!(
        "const char *s = R\"(a\\\nb\n{lead})\"; /* comment\n*/ {def} M hidden\n\nint f(int hidden) {{\n    hidden = M;\n    return hidden + 1;\n}}\n"
    )
}

#[test]
fn raw_string_content_line_with_a_directive_leader_is_no_directive_line() {
    for (lead, def) in [("#", "#define"), ("%:", "%:define")] {
        let src = round8(lead, def);
        for text in [src.clone(), src.replace('\n', "\r\n")] {
            // C++: the literal closes where written, `M` is a real definition.
            let c = parsed("a.cpp", &text);
            assert!(!c.incomplete(), "{lead}: {:?}", c.reasons);
            m_hidden(&c);
            // C: a spliced `R"` candidate is uncertain (fold 4); either way never lost.
            assert!(safe(&parsed("a.c", &text)), "{lead} .c {text:?}");
        }
    }
}

#[test]
fn directive_after_a_leader_led_raw_string_keeps_its_body() {
    for lead in ["#", "%:"] {
        for def in ["#define", "%:define"] {
            let src = format!("const char *s = R\"(a\\\nb\n{lead})\";\n{def} N a /* x\ny */ b\n");
            let c = parsed("a.cpp", &src);
            assert!(!c.incomplete(), "{lead} {def}: {:?}", c.reasons);
            let n = c.get("N").expect("N read");
            assert!(
                n.free.contains("a") && n.free.contains("b"),
                "{lead} {def}: {:?}",
                n.free
            );
        }
    }
}

#[test]
fn line_started_in_a_comment_tail_or_with_a_comment_leader_reads_the_define() {
    // The comment starts on a line that began inside an earlier comment.
    m_hidden(&parsed("a.c", "/* a\n*/ /* b\n*/ #define M hidden\n"));
    // A `#` inside an earlier comment of the same line counts as "before the comment".
    m_hidden(&parsed("a.c", "/* # */ /* c\n*/ #define M hidden\n"));
    m_hidden(&parsed(
        "a.cpp",
        "/* a\n*/ s = \"#\"; /* b\n*/ #define M hidden\n",
    ));
}

#[test]
fn proven_lines_read_one_variant() {
    for src in [
        // License header, ordinary code, a directive with a multi-line comment.
        "/* License\n * text\n */\n#include <a.h>\nint x; /* c\n d */\n",
        "#define A x /* c\n */ y\n",
        "  # define A x /* c\n */ y\n",
        "%:define A x /* c\n */ y\n",
        "/* a\n*/\n/* b\n*/\nint x; /* c\n*/\n",
        // Comments without a newline never matter, whatever the line holds.
        "int a; /* one line */ const char *s = \"#\"; /* also one line */\n",
        // The protobuf shape: a raw string, no comment on its lines.
        "void f() {\n  p->Emit(R\"cc(\n#if (a) && \\\n  b\n#endif\n)cc\");\n}\n",
        // A `//` comment holding `#` ends its line.
        "int a; // # not a directive\n/* c\n*/ int b;\n",
    ] {
        assert!(!second(src), "{src:?} must read one variant");
    }
}

#[test]
fn unproven_lines_read_both_variants() {
    for src in [
        // A `#` (or digraph, or trigraph) inside a literal before the comment.
        "const char *h = \"#\"; /* c\n*/ int v;\n",
        "const char *h = '#'; /* c\n*/ int v;\n",
        "x = \"%:\"; /* c\n*/ int v;\n",
        "x = \"??=\"; /* c\n*/ int v;\n",
        // The line began inside a raw string, with no leader byte at all.
        "const char *s = R\"(a\nb)\"; /* c\n*/ int v;\n",
        // The line began inside an earlier comment.
        "/* a\n*/ /* b\n*/ int v;\n",
        // A leader inside an earlier comment of the same line.
        "/* # */ /* c\n*/ int v;\n",
        "/* %: */ /* c\n*/ int v;\n",
    ] {
        assert!(second(src), "{src:?} must read both variants");
    }
}

fn typical_unit(i: usize) -> String {
    format!(
        "/* function {i}\n * does a thing */\nint f{i}(int a) {{ /* body\n  note */ return a + {i}; }}\n#define D{i} f{i}(1) /* c\n d */ + 1\n"
    )
}

#[test]
fn typical_large_file_takes_no_second_variant_and_is_fast() {
    let src: String = (0..6000).map(typical_unit).collect();
    assert!(!second(&src));
    let t = std::time::Instant::now();
    let c = parsed("a.c", &src);
    assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
    assert!(c.get("D5999").is_some_and(|d| d.free.contains("f5999")));
    assert!(!c.incomplete(), "{:?}", c.reasons);
}

#[test]
fn unproven_large_file_is_linear() {
    // Thousands of lines with multi-line comments and literals, every comment unproven.
    let mut src = String::new();
    for i in 0..3000 {
        src.push_str(&format!("const char *h{i} = \"#\"; /* c\n*/ int v{i};\n"));
        src.push_str(&format!("const char *r{i} = R\"(a\nb)\"; /* c\n d */\n"));
        src.push_str(&format!("/* t\n*/ /* u\n*/ int w{i};\n"));
        src.push_str(&format!("#define D{i} v{i} /* c\n d */ w\n"));
    }
    assert!(second(&src));
    let t = std::time::Instant::now();
    let c = parsed("a.cpp", &src);
    assert!(t.elapsed().as_secs_f64() < 5.0, "{:?}", t.elapsed());
    assert!(c.get("D2999").is_some_and(|d| d.free.contains("w")));
}

/// Measurement, not a gate: how many C-like files under the directories named by
/// `PRISM_CENSUS_VARIANT_DIRS` (colon-separated) take the second comment variant. Prints the
/// total, the count and the names; run with `--ignored --nocapture`.
#[test]
#[ignore = "reads the directories named by PRISM_CENSUS_VARIANT_DIRS"]
fn second_variant_count_over_directories() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() || matches!(name.as_str(), ".git" | "secbench" | "frontend-portal") {
                continue;
            }
            if ft.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| {
                matches!(
                    x.to_ascii_lowercase().as_str(),
                    "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "inc" | "inl"
                )
            }) {
                out.push(p);
            }
        }
    }
    let dirs = std::env::var("PRISM_CENSUS_VARIANT_DIRS").unwrap_or_default();
    let mut files = Vec::new();
    for d in dirs.split(':').filter(|d| !d.is_empty()) {
        walk(std::path::Path::new(d), &mut files);
    }
    let (mut total, mut hits) = (0usize, Vec::new());
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        total += 1;
        if second(&String::from_utf8_lossy(&bytes)) {
            hits.push(f.display().to_string());
        }
    }
    println!("second-variant: {} of {} files", hits.len(), total);
    for h in &hits {
        println!("second-variant file: {h}");
    }
}

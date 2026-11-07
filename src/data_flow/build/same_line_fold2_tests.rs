//! Same-line own-span judging (#630 slice 5, fold 2): shared by Go and Python, it adds edges,
//! and because the fallback cap counts survivors an owner at the cap can cross it (#634/#630).
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

fn run(name: &str, source: &str, language: Language, cap: Option<usize>) -> FileRefs {
    let parsed = ParsedFile::parse(name, source, language).unwrap();
    Producer::file_refs_with_cap(&parsed.path, &parsed, cap)
}

/// An RD-unavailable owner (4,100 padding statements add no candidates) whose only
/// difference is a `;`-joined redefinition before a use.
fn cap_pair(language: Language) -> (String, String, &'static str) {
    let (name, head, base, extra, tail) = match language {
        Language::Go => (
            "a.go",
            "package p\nfunc f(a int) {\n",
            "a = 0\nconsume(a)\n",
            "x := 1; x = 2\nconsume(x)\n",
            "}\n",
        ),
        _ => (
            "a.py",
            "def f(a):\n",
            "    a = 0\n    consume(a)\n",
            "    x = 1; x = 2\n    consume(x)\n",
            "",
        ),
    };
    let pad = match language {
        Language::Go => "g()\n".repeat(4100),
        _ => "    g()\n".repeat(4100),
    };
    let build = |body: &str| format!("{head}{body}{pad}{tail}");
    (build(base), build(&format!("{base}{extra}")), name)
}

#[test]
fn same_line_survivor_can_cross_the_fallback_cap_in_both_languages() {
    for language in [Language::Go, Language::Python] {
        let (plain, joined, name) = cap_pair(language);
        let base = run(name, &plain, language, None);
        let with = run(name, &joined, language, None);
        assert_eq!(base.rd_function_stats.functions_over_cap, 1, "{language:?}");
        assert_eq!(with.rd_function_stats.functions_over_cap, 1, "{language:?}");
        let (n_base, n_with) = (base.edges.len(), with.edges.len());
        assert!(n_base > 0, "{language:?}: the independent `a` flow exists");
        // `x = 1; x = 2` / use: the second def's own span is inside D's window and
        // survives; the coalesced bytes used to drop both candidates.
        assert!(
            n_with > n_base,
            "{language:?}: the same-line survivor adds an edge"
        );
        // The cap counts survivors: at the survivor count of the plain owner the plain owner
        // emits its edges, the owner with the added survivor is over the cap and emits none
        // (its independent `a` flow is gone too). Owner-accepted under #634/#630.
        let at_cap = run(name, &plain, language, Some(n_base));
        assert_eq!(at_cap.edges.len(), n_base, "{language:?}");
        assert_eq!(at_cap.rd_function_stats.fallback_functions_capped, 0);
        let over = run(name, &joined, language, Some(n_base));
        assert!(over.edges.is_empty(), "{language:?}");
        assert_eq!(over.rd_function_stats.fallback_functions_capped, 1);
        assert_eq!(
            over.rd_function_stats.fallback_edges_capped, n_with,
            "{language:?}: the cap counts every survivor"
        );
        let fits = run(name, &joined, language, Some(n_with));
        assert_eq!(fits.edges.len(), n_with, "{language:?}");
        assert_eq!(fits.rd_function_stats.fallback_functions_capped, 0);
    }
}

/// The alias-twin emission site judges the twin's own span, not the coalesced bytes of the
/// first twin on the line (`ptr.f = 1` is before the dominating `s.f = 2`, the last one after).
#[test]
fn alias_twin_same_line_edge_is_judged_at_its_own_span() {
    let source = "package p\ntype S struct { f int }\nfunc f(s *S) {\nptr := s\nptr.f = 1; s.f = 2; ptr.f = 3\nuse(s.f)\n}\n";
    let first_twin = source.find("ptr.f = 1").unwrap();
    let parsed = ParsedFile::parse("t.go", source, Language::Go).unwrap();
    let refs = Producer::file_refs(&parsed.path, &parsed);
    // The twin edge is emitted from the coalesced location, the first twin's bytes.
    let twin_edge = refs.edges.iter().any(|e| {
        e.from.path.to_string() == "s.f"
            && e.from.line == 5
            && e.from.start_byte == first_twin
            && e.to.line == 6
    });
    assert!(
        twin_edge,
        "the later twin lies in the dominating write's window: {:?}",
        refs.edges
            .iter()
            .map(|e| (
                e.from.path.to_string(),
                e.from.line,
                e.from.start_byte,
                e.to.line
            ))
            .collect::<Vec<_>>()
    );
}

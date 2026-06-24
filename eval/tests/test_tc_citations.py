from tier_c.citations import parse_citations

def test_parses_file_line_and_file_line_symbol():
    text = "The bug is in `src/glob.rs:42` in fn `compile` (src/glob.rs:42:compile)."
    cites = parse_citations(text)
    assert ("src/glob.rs", 42, None) in [(c.file, c.line, c.symbol) for c in cites]
    assert ("src/glob.rs", 42, "compile") in [(c.file, c.line, c.symbol) for c in cites]

def test_ignores_non_code_colons():
    assert parse_citations("see http://x.com:80/page") == []

def test_dedupes():
    cites = parse_citations("src/a.py:1 and again src/a.py:1")
    assert len(cites) == 1

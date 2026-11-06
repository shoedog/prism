//! Census sites for every `variant-matrix.tsv` fixture, against a committed expectation
//! (`tests/fixtures/js_write_census/sites.tsv`). Each fixture's source is the matrix row's `source`
//! column. Re-record with `PRISM_JS_CENSUS_BLESS=1` only for a reviewed change to the walker.

use super::harness::{owners, Fx};
use super::walk::{census, Census, Site};
use crate::languages::Language;
use std::collections::BTreeSet;

const MATRIX: &str = include_str!("../../../tests/fixtures/js_write_census/variant-matrix.tsv");
const EXPECTED: &str = include_str!("../../../tests/fixtures/js_write_census/sites.tsv");

/// (fixture, slice, source) of each distinct fixture, source unescaped (`\n` only).
pub(crate) fn fixtures() -> Vec<(String, String, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for line in MATRIX.lines().skip(1) {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 8, "matrix row: {line}");
        if seen.insert(cols[0].to_string()) {
            out.push((
                cols[0].to_string(),
                cols[5].to_string(),
                cols[7].replace("\\n", "\n"),
            ));
        }
    }
    out
}

/// The grammar each fixture is parsed with. Everything not listed is JavaScript; the listed rows use
/// TypeScript (types, `as`, `!`, `enum`, parameter properties, assignment-form `using`) or TSX (JSX
/// next to types). No fixture is parsed by guessing.
pub(crate) fn lang_of(fixture: &str) -> Language {
    match fixture.split('_').next().unwrap_or("") {
        "t05" | "t01" | "t02" | "t03" | "t04" | "o20" | "o22" | "o23" | "o24" | "o25" | "o26" => {
            Language::TypeScript
        }
        "t26" | "o34" => Language::Tsx,
        _ => Language::JavaScript,
    }
}

fn parse_fixture(name: &str, src: &str) -> Fx {
    let fx = Fx::in_lang(src, lang_of(name));
    assert!(
        !fx.parsed.tree.root_node().has_error(),
        "{name} parses cleanly as {:?}",
        lang_of(name)
    );
    fx
}

pub(super) fn fixture(name: &str) -> Fx {
    parse_fixture(name, &source_of(name))
}

pub(super) fn source_of(fixture: &str) -> String {
    fixtures()
        .into_iter()
        .find(|f| f.0 == fixture)
        .unwrap_or_else(|| panic!("no fixture {fixture}"))
        .2
}

fn flags(s: &Site) -> String {
    let r = s.region;
    let mut f = Vec::new();
    for (on, name) in [
        (r.own_params, "own_params"),
        (r.nested, "nested"),
        (r.nested_params, "nested_params"),
        (r.instance_field, "instance_field"),
        (r.with_body, "with_body"),
        (s.for_head, "for_head"),
        (s.chain.is_none(), "unmapped"),
    ] {
        if on {
            f.push(name);
        }
    }
    f.join(",")
}

/// 1-indexed line of a byte offset.
fn line_of(src: &str, byte: usize) -> usize {
    src[..byte].matches('\n').count() + 1
}

fn site_line(s: &Site, src: &str) -> String {
    let node = match s.span {
        Some((a, _)) => format!("node@{}", line_of(src, a)),
        None => "node@-".to_string(),
    };
    let chain = s.chain.as_ref().map_or("-".to_string(), |c| c.join(">"));
    format!(
        "{}@{}:{} {:?}{}/{} m{} [{}] {node} {chain}",
        s.path.render(),
        s.line,
        s.target_start,
        s.form,
        if s.decl.is_empty() {
            String::new()
        } else {
            format!(
                ":{}{}",
                s.decl,
                if s.initialized { "" } else { "(no value)" }
            )
        },
        s.construct_kind.unwrap_or("-"),
        s.moment,
        flags(s)
    )
}

fn describe(c: &Census, src: &str) -> String {
    let refusal = c
        .refusals
        .iter()
        .map(|r| r.label())
        .collect::<Vec<_>>()
        .join(",");
    let sites = c
        .sites
        .iter()
        .map(|s| site_line(s, src))
        .collect::<Vec<_>>()
        .join(" ; ");
    format!(
        "{}\t{}",
        if refusal.is_empty() { "-" } else { &refusal },
        sites
    )
}

fn render() -> String {
    let mut out = String::from("# fixture\towner\trefusal\tsites (path@line:start_byte form/construct moment [region flags] E18 node line, chain of kinds up to the mapped statement)\n");
    for (name, _slice, src) in fixtures() {
        let fx = parse_fixture(&name, &src);
        for o in owners(&fx.parsed, &fx.t) {
            let c = census(&fx.parsed, &fx.t, o);
            out.push_str(&format!(
                "{name}\t{}@{}\t{}\n",
                o.kind(),
                o.start_position().row + 1,
                describe(&c, &src)
            ));
        }
    }
    out
}

#[test]
fn matrix_has_every_fixture_and_the_expected_slices() {
    let all = fixtures();
    assert_eq!(all.len(), 176, "distinct fixtures");
    for want in ["S1b-1", "S1b-2", "S1c", "S1d"] {
        assert!(all.iter().any(|f| f.1.contains(want)), "{want}");
    }
    assert!(
        all.iter()
            .any(|f| f.0.starts_with("s15") || f.0.starts_with("s14")),
        "one-node first_hazard rows s01-s15 present"
    );
}

#[test]
fn census_sites_match_committed_expectation() {
    let got = render();
    if std::env::var_os("PRISM_JS_CENSUS_BLESS").is_some() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/js_write_census/sites.tsv"
            ),
            &got,
        )
        .unwrap();
        return;
    }
    assert_eq!(
        got, EXPECTED,
        "census sites drifted; review and re-bless with PRISM_JS_CENSUS_BLESS=1"
    );
}

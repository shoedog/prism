//! Hand-written rows on the fields S1 consumes: the E18 mapped statement, the chain, parameter
//! regions and the one-sided order case. Independent of the blessed `sites.tsv`.

use super::harness::{writes, Fx};
use super::order::{precedes, Position};
use super::walk::Form;
use crate::languages::Language::{JavaScript as JS, Tsx, TypeScript as TS};

fn line_of(src: &str, byte: usize) -> usize {
    src[..byte].matches('\n').count() + 1
}

#[test]
fn a_site_maps_to_the_innermost_span_not_the_outermost() {
    for lang in [JS, TS, Tsx] {
        for (src, kind) in [
            (
                "function f(a) {\n  log();\n  while (c()) {\n    a++;\n  }\n}",
                "while",
            ),
            (
                "function f(a) {\n  log();\n  if (c()) {\n    a++;\n  }\n}",
                "if",
            ),
        ] {
            let fx = Fx::in_lang(src, lang);
            let c = fx.first_census();
            let w = writes(&c, "a")[0];
            let (start, _) = w.span.expect("mapped");
            assert_eq!(
                line_of(src, start),
                4,
                "{lang:?} {kind}: the body statement's own line"
            );
            assert_eq!(
                w.chain.as_deref(),
                Some(&["expression_statement"][..]),
                "{lang:?} {kind}"
            );
        }
    }
}

#[test]
fn a_read_maps_to_the_innermost_span_not_the_outermost() {
    for lang in [JS, TS, Tsx] {
        let src = "function f(a) {\n  while (c()) {\n    sink(a);\n  }\n}";
        let fx = Fx::in_lang(src, lang);
        let c = fx.first_census();
        let r = &fx.reads(&c, "a")[0].1;
        assert_eq!(line_of(src, r.span.unwrap().0), 3, "{lang:?}");
        assert_eq!(
            r.chain.as_deref().and_then(|c| c.last()),
            Some(&"expression_statement"),
            "{lang:?}"
        );
    }
}

#[test]
fn only_delete_makes_a_unary_operand_a_write() {
    for lang in [JS, TS, Tsx] {
        let fx = Fx::line_in(lang, "!o.x, typeof o.x, void o.x, -a, +a, ~a;");
        let c = fx.first_census();
        assert!(
            c.sites
                .iter()
                .all(|s| s.path.base != "o" && s.form == Form::Declare),
            "{lang:?}: {:?}",
            c.sites
        );
        let fx = Fx::line_in(lang, "delete o.x;");
        let n = fx
            .first_census()
            .sites
            .iter()
            .filter(|s| s.path.base == "o" && s.form == Form::Delete)
            .count();
        assert_eq!(n, 1, "{lang:?}");
    }
}

#[test]
fn a_lone_arrow_parameter_is_a_parameter_list() {
    for lang in [JS, TS, Tsx] {
        let fx = Fx::line_in(lang, "g(x => { x = 1; });");
        let c = fx.first_census();
        let p = c
            .sites
            .iter()
            .find(|s| s.path.base == "x" && s.form == Form::Declare)
            .unwrap();
        assert!(
            p.region.nested && p.region.nested_params && !p.region.own_params,
            "{lang:?} {:?}",
            p.region
        );
        let w = c
            .sites
            .iter()
            .find(|s| s.path.base == "x" && s.form == Form::Write)
            .unwrap();
        assert!(
            w.region.nested && !w.region.nested_params,
            "{lang:?}: the body write is not a parameter write"
        );
    }
}

#[test]
fn an_unknown_kind_on_either_chain_alone_refuses() {
    for lang in [JS, TS, Tsx] {
        // Only the write sits under an `unknown` kind (a pattern default); the read's chain is all `seq`.
        let fx = Fx::line_in(lang, "(sink(a), [b = (a = 1)] = c);");
        let c = fx.first_census();
        let w = Position::of_site(writes(&c, "a")[0]);
        let r = fx.reads(&c, "a")[0].1.clone();
        assert!(
            r.chain
                .as_ref()
                .unwrap()
                .iter()
                .all(|k| fx.t.order_class(k) == Some("seq")),
            "{lang:?}"
        );
        assert!(
            !precedes(&fx.t, &r, &w),
            "{lang:?}: the second position's chain alone refuses"
        );
        assert!(!precedes(&fx.t, &w, &r), "{lang:?}");
        // Control: with no unknown kind the same read -> write pair is proven.
        let fx = Fx::line_in(lang, "(sink(a), (a = 1));");
        let c = fx.first_census();
        let w = Position::of_site(writes(&c, "a")[0]);
        let r = fx.reads(&c, "a")[0].1.clone();
        assert!(precedes(&fx.t, &r, &w), "{lang:?}");
    }
}

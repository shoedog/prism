//! Rule R (positive certificate P2, DESIGN 5.3.4) through the product: every row builds the real
//! graph and compares the whole outcome (the `refused_source_ambiguous` counter and the number of
//! caller-to-`D` data-flow hops). Refusal rows pair with a control that differs only in the
//! guarded fact and flows; the three shapes P2 cannot see are pinned as known limits (#783).

use super::props_generated_tests::parse;
use super::{CodePropertyGraph, CpgNode};
use petgraph::visit::EdgeRef;
use std::collections::BTreeMap;

const D: &str = "function D({ a }) { return use(a); }\n";

/// (`refused_source_ambiguous` count, caller-to-D data-flow hops) for one grammar.
fn run(ext: &str, source: &str) -> (usize, usize) {
    let path = format!("m.{ext}");
    let files = parse(&BTreeMap::from([(path.clone(), format!("{D}{source}"))]));
    assert_eq!(files[&path].parse_error_count, 0, "{source}");
    let graph = CodePropertyGraph::build(&files);
    let hops = graph
        .graph
        .edge_references()
        .filter(|edge| edge.weight().is_data_flow())
        .filter(|edge| {
            matches!(graph.node(edge.source()), CpgNode::Variable { function, .. } if function != "D")
                && matches!(graph.node(edge.target()), CpgNode::Variable { function, .. } if function == "D")
        })
        .count();
    let ambiguous = graph
        .dfg_label_stats
        .dfg_props
        .get("refused_source_ambiguous")
        .copied()
        .unwrap_or_default();
    (ambiguous, hops)
}

#[track_caller]
fn refused(ext: &str, source: &str) {
    assert_eq!(run(ext, source), (1, 0), "{ext}: {source}");
}

#[track_caller]
fn bound(ext: &str, source: &str) {
    assert_eq!(run(ext, source), (0, 1), "{ext}: {source}");
}

#[test]
fn a_mapped_arguments_or_eval_write_refuses_the_representative() {
    for ext in ["jsx", "tsx"] {
        // Sloppy mapped `arguments` writes the formal; direct eval can write any local (clause c).
        refused(
            ext,
            "function App(x) { see(x); arguments[0] = 0; return <D a={x} />; }",
        );
        refused(
            ext,
            "function App(x) { see(x); eval('x = 0'); return <D a={x} />; }",
        );
        refused(
            ext,
            r"function App(x) { see(x); eval('x = 0'); return <D a={x} />; }",
        );
        // The formal's own mapped alias, written the other way round (a2 and c both fail).
        refused(
            ext,
            "function App(x) { see(arguments); x = 0; return <D a={arguments} />; }",
        );
        // Controls: neither spelling occurs, or the spelling is only text.
        bound(
            ext,
            "function App(x) { see(x); keep(1); return <D a={x} />; }",
        );
        bound(
            ext,
            "function App(x) { see(x); keep('eval arguments'); return <D a={x} />; }",
        );
    }
}

#[test]
fn the_six_shapes_astra_named_are_refused_by_a_clause() {
    refused(
        "jsx",
        "function App() { for (var x of [7, 0]) { if (x) see(x); else return <D a={x} />; } }",
    );
    refused(
        "tsx",
        "function App(x: { <T>(): T; p: number }) { see(x.p); (x<string>).p = 0; return <D a={x.p} />; }",
    );
    refused(
        "jsx",
        r"function App() { try { throw 7; } catch (x) { see(x); var x = 0; return <D a={x} />; } }",
    );
    refused(
        "jsx",
        r#"const o = { [(function App(x) { see(x); arguments[0] = 0; return <D a={x} />; })(7)]() { "use strict"; } };"#,
    );
    refused(
        "jsx",
        "function App(x) { see(x); arguments[0] = 0; return <D a={x} />; }",
    );
    refused(
        "jsx",
        "class Outer { #p = 7; render() { class Inner extends Outer { #p = 7; set() { t.#p = 0; } } const t = new Inner(); see(t.#p); t.set(); return <D a={t.#p} />; } }",
    );
}

/// Known limits (DESIGN 5.3.4, #783): the certificate reads spelled occurrences only, so these
/// three shapes are admitted although a runtime value can differ. Each sits beside a control
/// that differs in the one guarded fact.
#[test]
fn the_three_shapes_p2_cannot_see_are_pinned_as_known_limits() {
    for ext in ["jsx", "tsx"] {
        // (1) A `super` store through a formal receiver; a store spelled through `x` refuses.
        bound(
            ext,
            "class App extends Base { render(x) { see(x.p); super.p = 0; return <D a={x.p} />; } }",
        );
        refused(
            ext,
            "class App extends Base { render(x) { see(x.p); x.p = 0; return <D a={x.p} />; } }",
        );
        // (2) A const that reads an enum member during enum construction is certified, and the
        // enum root is refused: both pinned at the certificate (`certificate_tests::astra_nine`);
        // the product builds no props edge for an enum member operand.
        // (3) A store through another formal that may alias the same object.
        bound(
            ext,
            "function App(x, y) { see(x); y.p = 0; return <D a={x} />; }",
        );
        refused(
            ext,
            "function App(x, y) { see(x); x.p = 0; return <D a={x} />; }",
        );
    }
}

#[test]
fn the_round_5_design_check_shapes_are_refused_with_controls() {
    for ext in ["jsx", "tsx"] {
        // A padded escape (`\u{00000078} = 0`) is pinned at the certificate (clause c). The
        // product refuses such an owner earlier (`refused_not_exact_singleton`), so no edge row.
        // A listed result position that supplies the receiver of a store.
        refused(
            ext,
            "function App(x) { see(x); (0, x).p = 0; return <D a={x} />; }",
        );
        refused(
            ext,
            "function App(x, c, y) { see(x); (c ? x : y).p = 0; return <D a={x} />; }",
        );
        refused(
            ext,
            "function App(x, y) { see(x); (x || y).p = 0; return <D a={x} />; }",
        );
        // Controls: the store goes through another root.
        bound(
            ext,
            "function App(x, y) { see(x); (0, y).p = 0; return <D a={x} />; }",
        );
        bound(
            ext,
            "function App(x, y) { see(x); (y || y).p = 0; return <D a={x} />; }",
        );
    }
    // A type query call argument is erased code only in the type, but runs once erased.
    refused(
        "tsx",
        "function App(x: number) { function change() { type T = typeof Math.min/*\n*/(x = 0); } see(x); change(); return <D a={x} />; }",
    );
    bound(
        "tsx",
        "function App(x: number) { function change() { type T = typeof Math.min; } see(x); change(); return <D a={x} />; }",
    );
}

#[test]
fn an_object_the_grammar_attaches_across_a_line_is_refused_with_controls() {
    // The grammar reads the second line as a subscript of the arrow (or of the `as` expression);
    // the language ends the statement there and runs a destructuring write of `x`. The operand sits
    // on a line of its own with `see(x)`, so the read of `x` stands for it (Rule R decides).
    for ext in ["jsx", "tsx"] {
        refused(
            ext,
            "function App(x) { const g = () => {}\n[x] = [0]; g;\nsee(x); return <D a={x} />; }",
        );
        // Controls: parentheses close the arrow, so the language reads the same subscript.
        bound(
            ext,
            "function App(x) { const g = (() => {})\n[x]; g;\nsee(x); return <D a={x} />; }",
        );
    }
    // (`satisfies T` then a line starting `[` is a parse error in the grammar, so it never reaches
    // the rule; only its parenthesized control is a valid source.)
    refused(
        "tsx",
        "function App(x) { const v = [] as const\n[x] = [0]; v;\nsee(x); return <D a={x} />; }",
    );
    for wrapper in ["as const", "satisfies object"] {
        bound(
            "tsx",
            &format!(
                "function App(x) {{ const v = ([] {wrapper})\n[x]; v;\nsee(x); return <D a={{x}} />; }}"
            ),
        );
    }
}

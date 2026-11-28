//! Refusal-branch checks for the S3 JSX source and syntax proofs (lane 3, codecov patch). Each
//! refusal is paired with a control that differs only in the guarded fact and flows. Counters are
//! compared as whole maps, so an extra or a missing refusal fails the case.

use super::props_generated_tests::parse;
use super::{CodePropertyGraph, CpgNode};
use petgraph::visit::EdgeRef;
use std::collections::BTreeMap;

const P_A: &str = "function P(p) {\n  return use(p.a);\n}\n";
const EXTS: [&str; 2] = ["jsx", "tsx"];

fn build(ext: &str, source: &str) -> CodePropertyGraph {
    let path = format!("m.{ext}");
    let files = parse(&BTreeMap::from([(path.clone(), source.to_owned())]));
    assert_eq!(files[&path].parse_error_count, 0, "{source}");
    CodePropertyGraph::build(&files)
}

/// Whole-map counters and the number of caller-to-callee data-flow hops into `P`.
fn outcome(graph: &CodePropertyGraph) -> (BTreeMap<String, usize>, usize) {
    let hops = graph
        .graph
        .edge_references()
        .filter(|edge| edge.weight().is_data_flow())
        .filter(|edge| {
            matches!(graph.node(edge.source()), CpgNode::Variable { function, .. } if function == "App" || function == "C")
                && matches!(graph.node(edge.target()), CpgNode::Variable { function, .. } if function == "P")
        })
        .count();
    (graph.dfg_label_stats.dfg_props.clone(), hops)
}

fn expect(source: &str, ext: &str, counters: &[(&str, usize)], hops: usize) {
    let (actual, actual_hops) = outcome(&build(ext, source));
    let want: BTreeMap<String, usize> = counters
        .iter()
        .map(|(k, v)| ((*k).to_owned(), *v))
        .collect();
    assert_eq!((actual, actual_hops), (want, hops), "{ext}: {source}");
}

fn jsx(callee: &str, jsx: &str) -> String {
    format!("{callee}function App(x) {{\n  return {jsx};\n}}\n")
}

/// `body` runs before `return out`; `formal` is App's parameter list.
fn app(formal: &str, body: &str) -> String {
    format!("{P_A}function App({formal}) {{\n  {body}\n  return out;\n}}\n")
}

#[test]
fn a_braced_attribute_that_is_not_a_spread_binds_nothing() {
    for ext in EXTS {
        for opening in ["<P {x}/>", "<P {}/>"] {
            // The fold counts the recovered-syntax drop (an admitted site with no entries).
            expect(
                &jsx(P_A, opening),
                ext,
                &[("sites_jsx", 1), ("refused_jsx_site_syntax", 1)],
                0,
            );
        }
        // Control: the braced spread (and a named attribute) is still read.
        let spread = jsx(P_A, "<P {...x} a={x}/>");
        expect(
            &spread,
            ext,
            &[
                ("sites_jsx", 1),
                ("spread_sources_unbound", 1),
                ("edges_to_use", 1),
            ],
            1,
        );
    }
}

#[test]
fn a_character_reference_child_adds_an_opaque_children_prop() {
    for ext in EXTS {
        let opaque = [
            ("sites_jsx", 1),
            ("edges_to_use", 1),
            ("refused_value_not_data", 1),
        ];
        expect(&jsx(P_A, "<P a={x}>&amp;</P>"), ext, &opaque, 1);
        expect(&jsx(P_A, "<P a={x}>text</P>"), ext, &opaque, 1);
        // Controls: an empty element and a whitespace-only multi-line body have no children.
        let none = [("sites_jsx", 1), ("edges_to_use", 1)];
        expect(&jsx(P_A, "<P a={x}></P>"), ext, &none, 1);
        expect(&jsx(P_A, "<P a={x}>\n  </P>"), ext, &none, 1);
    }
}

#[test]
fn a_this_operand_is_a_source_attempt_in_jsx_and_opaque_in_a_literal() {
    for ext in EXTS {
        let class =
            |inner: &str| format!("{P_A}class C {{\n  render() {{\n    {inner}\n  }}\n}}\n");
        let sited = |counter| [("sites_jsx", 1), (counter, 1)];
        // `this` is a data operand: no Use node answers it, so the source is refused, not opaque.
        expect(
            &class("return <P a={this}/>;"),
            ext,
            &sited("refused_source_not_found"),
            0,
        );
        expect(
            &class("return <P a={this.a}/>;"),
            ext,
            &sited("refused_source_unconnected"),
            0,
        );
        // An arrow keeps the method's `this`: the owner walk skips it to the method.
        let arrow = class("const f = () => <P a={this.a}/>;\n    return f();");
        // The fold also counts the execution-owner mismatch (arrow vs method) once.
        expect(
            &arrow,
            ext,
            &[
                ("sites_jsx", 1),
                ("refused_jsx_caller_owner", 1),
                ("refused_source_not_found", 1),
            ],
            0,
        );
        // Control: a string value is opaque data, never a source.
        expect(
            &class("return <P a={'s'}/>;"),
            ext,
            &sited("refused_value_not_data"),
            0,
        );
        // The same `this` in an object-literal argument stays opaque (JSX-only rule).
        let literal = "function D(p) {\n  return use(p.a);\n}\nclass C {\n  render() {\n    return D({a: this});\n  }\n}\n";
        let counters = &build(ext, literal).dfg_label_stats.dfg_props;
        assert_eq!(counters.get("refused_value_not_data"), Some(&1), "{ext}");
        assert_eq!(counters.get("refused_source_not_found"), None, "{ext}");
    }
}

#[test]
fn a_wrapper_inside_a_member_chain_contributes_its_inner_line() {
    for ext in EXTS {
        let bound = [("sites_jsx", 1), ("edges_to_use", 1)];
        let multi = |operand: &str| app("x", &format!("const out = <P a={{{operand}}}/>;"));
        // A wrapper around the root, spread over lines, still binds: the chain's line walk
        // descends the wrapper to `x`. (The base fallback at the root line would also find it.)
        expect(&multi("(\n    x\n  ).a"), ext, &bound, 1);
        expect(&multi("((\n    x\n  )).a"), ext, &bound, 1);
        // Control: the unwrapped multi-line chain binds the same way.
        expect(&multi("x\n    .a"), ext, &bound, 1);
    }
    // An `as` wrapper is outside the read list (clause b2): the representative is refused.
    let tsx = app("x", "const out = <P a={(\n    x as any\n  ).a}/>;");
    expect(
        &tsx,
        "tsx",
        &[("sites_jsx", 1), ("refused_source_ambiguous", 1)],
        0,
    );
}

#[test]
fn a_source_with_no_declaration_is_unconnected_and_a_declared_one_binds() {
    for ext in EXTS {
        // A representative of a name with no declaration in the owner fails clause a2
        // (`none_found`), so it counts as ambiguous, ahead of the unconnected proof.
        let refused = [("sites_jsx", 1), ("refused_source_ambiguous", 1)];
        expect(
            &app("", "see(g); const out = <P a={g}/>;"),
            ext,
            &refused,
            0,
        );
        expect(
            &app("", "for (g of arr) { see(g); }\n  const out = <P a={g}/>;"),
            ext,
            &[("sites_jsx", 1), ("refused_source_ambiguous", 1)],
            0,
        );
        // An initialized redeclaration of the formal is a write under Rule R.
        expect(
            &app("x", "var x = f(); const out = <P a={x}/>;"),
            ext,
            &[("sites_jsx", 1), ("refused_source_ambiguous", 1)],
            0,
        );
        // Controls: a parameter, and a block-scoped redeclaration, are single declarations.
        let bound = [("sites_jsx", 1), ("edges_to_use", 1)];
        expect(&app("g", "see(g); const out = <P a={g}/>;"), ext, &bound, 1);
        expect(
            &app("x", "const q = f(); const out = <P a={x}/>;"),
            ext,
            &bound,
            1,
        );
    }
}

#[test]
fn a_write_between_the_anchor_and_the_operand_makes_the_source_ambiguous() {
    let body = |write: &str| format!("see(x); {write} const out = <P a={{x.a}}/>;");
    for ext in EXTS {
        let ambiguous = [("sites_jsx", 1), ("refused_source_ambiguous", 1)];
        for write in [
            "x = g();",
            "x.a = 1;",
            "x[0] = 1;",
            "x[k] = 1;",
            "[x] = z;",
            "({k: x} = z);",
            "({k: x.a} = z);",
            "for ([x] of z) {}",
        ] {
            expect(&app("x", &body(write)), ext, &ambiguous, 0);
        }
        // Controls: a write to another base does not touch `x`. Rule R (P2) is not
        // field-sensitive: a store to a sibling field is a store through the root and refuses.
        let bound = [("sites_jsx", 1), ("edges_to_use", 1)];
        for write in ["w[0] = 1;", "[q] = z;"] {
            expect(&app("x", &body(write)), ext, &bound, 1);
        }
        for write in ["x.z = 1;", "({k: x.z} = z);"] {
            expect(&app("x", &body(write)), ext, &ambiguous, 0);
        }
    }
}

#[test]
fn a_zero_width_anchor_is_judged_by_the_reads_on_its_line() {
    for ext in EXTS {
        // `x` is mixed (a member read and a bare read), so its declarator read is a line anchor.
        let anchored = |tail: &str| {
            app(
                "x",
                &format!("see(x.k);\n  const q = x; {tail} const out = <P a={{x}}/>;"),
            )
        };
        let ambiguous = [("sites_jsx", 1), ("refused_source_ambiguous", 1)];
        expect(&anchored("x.y = 1;"), ext, &ambiguous, 0);
        expect(&anchored("x = g();"), ext, &ambiguous, 0);
        expect(
            &anchored(""),
            ext,
            &[("sites_jsx", 1), ("edges_to_use", 1)],
            1,
        );
    }
}

#[test]
fn props_s3_owner_write_declarations_and_own_sources() {
    for ext in EXTS {
        let ambiguous = [("sites_jsx", 1), ("refused_source_ambiguous", 1)];
        let bound = [("sites_jsx", 1), ("edges_to_use", 1)];
        expect(
            &app("x", "see(x); const out = <P a={x}/>; var x = f();"),
            ext,
            &ambiguous,
            0,
        );
        expect(
            &app("", "var x;\n  see(x); const out = <P a={x}/>; var x = f();"),
            ext,
            &ambiguous,
            0,
        );
        expect(
            &app(
                "x",
                "see(x); const out = <P a={x}/>; for (var x of items) {}",
            ),
            ext,
            &ambiguous,
            0,
        );
        // Clause a2: a `var` local is never an eligible declaration, written or not.
        expect(
            &app("", "var x = f();\n  see(x); const out = <P a={x}/>;"),
            ext,
            &ambiguous,
            0,
        );
        expect(
            &app("x", "try { f(); } catch (x) { see(x); take(<P a={x}/>); }"),
            ext,
            &ambiguous,
            0,
        );
        // A catch parameter spelled like the formal is a second declaration (a2).
        expect(
            &app(
                "x",
                "try { f(); } catch (x) {}\n  see(x); take(<P a={x}/>);",
            ),
            ext,
            &ambiguous,
            0,
        );
        expect(
            &app(
                "x",
                "try { f(); } catch (e) {}\n  see(x); take(<P a={x}/>);",
            ),
            ext,
            &bound,
            1,
        );
        expect(&app("x", "x = f();\n  return <P a={x}/>;"), ext, &bound, 1);
    }
    expect(
        &app("x", "see(x); const out = <P a={x}/>; using other = f();"),
        "tsx",
        &[("sites_jsx", 1), ("edges_to_use", 1)],
        1,
    );
}

#[test]
fn props_s3_owner_write_counts_once_for_each_refused_leaf() {
    for ext in EXTS {
        let source = |writes: &str| {
            format!("function P({{a, b, c}}) {{ return use(a, b, c); }}\nfunction App(x, y) {{\n  see(x, y); const out = <P a={{x}} b={{x}} c={{y}}/>; {writes}\n  return out;\n}}\n")
        };
        expect(
            &source("[x = 0] = values; ++x;"),
            ext,
            &[
                ("sites_jsx", 1),
                ("refused_source_ambiguous", 2),
                ("edges_to_def", 1),
            ],
            1,
        );
        expect(
            &source(""),
            ext,
            &[("sites_jsx", 1), ("edges_to_def", 3)],
            3,
        );
    }
}

#[test]
fn a_multiline_operand_falls_back_to_the_base_use_by_line() {
    for ext in EXTS {
        let local = |body: &str| app("", &format!("const box = make();\n  {body}"));
        let ambiguous = [("sites_jsx", 1), ("refused_source_ambiguous", 1)];
        let bound = [("sites_jsx", 1), ("edges_to_use", 1)];
        // A write to the base before the operand's line is a conflict the fallback must see.
        let written = local("box.value = x; see(box); const out = <P a={box\n    .value}/>;");
        expect(&written, ext, &ambiguous, 0);
        let clean = local("see(box); const out = <P a={box\n    .value}/>;");
        expect(&clean, ext, &bound, 1);
        // An undeclared base has a base Use on the root line (or a later line) but no proof.
        let unconnected = [("sites_jsx", 1), ("refused_source_ambiguous", 1)];
        expect(
            &app("", "see(g); const out = <P a={g\n    .value}/>;"),
            ext,
            &unconnected,
            0,
        );
        expect(
            &app("", "const out = <P a={g\n    .value}/>; see(g);"),
            ext,
            &unconnected,
            0,
        );
    }
}

#[test]
fn a_captured_source_is_attributed_to_the_enclosing_owner_only_when_it_is_proved_there() {
    for ext in EXTS {
        let enclosing = [("sites_jsx", 1), ("refused_source_in_enclosing_owner", 1)];
        let not_found = [("sites_jsx", 1), ("refused_source_not_found", 1)];
        let arrow = "const f = () => <P a={x}/>; const out = f();";
        // The outer Use names the same declaration: the capture is the enclosing owner's.
        expect(&app("x", &format!("see(x); {arrow}")), ext, &enclosing, 0);
        // A same-spelled shadow in the outer owner is another declaration: nothing proves it.
        expect(
            &app("x", &format!("{{ let x = 1; see(x); }} {arrow}")),
            ext,
            &not_found,
            0,
        );
        // A zero-width outer anchor needs its own contributor proof; a redeclared formal has none.
        let anchored = |extra: &str| {
            format!("{P_A}function App(x) {{\n  see(x.k);\n  {extra}const q = x; const f = () => <P a={{x}}/>;\n  return f;\n}}\n")
        };
        expect(&anchored(""), ext, &enclosing, 0);
        expect(&anchored("var x; "), ext, &not_found, 0);
        // Control: the same operand without an enclosing callable binds in its own owner.
        expect(
            &app("x", "see(x); const out = <P a={x}/>;"),
            ext,
            &[("sites_jsx", 1), ("edges_to_use", 1)],
            1,
        );
    }
}

#[test]
fn a_namespace_tag_object_is_not_a_read_of_the_operand_it_shares_a_name_with() {
    let ui = "export function P(p) {\n  return use(p.a);\n}\n";
    for ext in EXTS {
        let run = |attribute: &str| {
            let caller = format!("import * as ns from './ui.{ext}';\nfunction App(x) {{\n  return <ns.P a={{{attribute}}}/>;\n}}\n");
            let files = parse(&BTreeMap::from([
                (format!("m.{ext}"), caller),
                (format!("ui.{ext}"), ui.to_owned()),
            ]));
            outcome(&CodePropertyGraph::build(&files))
        };
        // The `ns` of `<ns.P ..>` sits on the operand's line but is a tag, not a value read;
        // the operand `ns` is the import itself, which no Def connects. This pins the outcome.
        let (counters, hops) = run("ns");
        let want = BTreeMap::from([
            ("sites_jsx".to_owned(), 1),
            ("refused_source_ambiguous".to_owned(), 1),
        ]);
        assert_eq!((counters, hops), (want, 0), "{ext}");
        // Control: the same tag with a formal operand binds.
        let (counters, hops) = run("x");
        let want = BTreeMap::from([("sites_jsx".to_owned(), 1), ("edges_to_use".to_owned(), 1)]);
        assert_eq!((counters, hops), (want, 1), "{ext}");
    }
}

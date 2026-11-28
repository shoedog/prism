//! `literal_arguments` skips sites and spans that cannot carry an object-literal argument (lane 3
//! S2b, codecov patch). Each skip is paired with the unmodified control that yields the literal.

use super::super::spread_args::for_call;
use super::*;
use crate::cpg::object_literal_arg_tests::parse_files;
use crate::languages::Language;

#[test]
fn literal_argument_collection_skips_non_call_sites_and_spans_that_are_not_one_node() {
    let source = "function D({a}) {\n  return use(a);\n}\nfunction App(x) {\n  D({a: x});\n}\n";
    let parsed = parse_files(Language::JavaScript, &[("m", source)], "js");
    let graph = CodePropertyGraph::build(&parsed);
    let caller = &parsed["m.js"];
    let call = graph
        .call_graph
        .calls
        .iter()
        .flat_map(|(_, sites)| sites)
        .find(|site| site.callee_name == "D")
        .expect("recorded site")
        .clone();
    let (args, _) = for_call(caller, &call);
    assert_eq!(args.len(), 1);
    assert_eq!(literal_arguments(caller, &call, &args).len(), 1);
    // A JSX element or a macro invocation carries no object-literal argument.
    let mut jsx = call.clone();
    jsx.jsx_element = true;
    assert!(literal_arguments(caller, &jsx, &args).is_empty());
    let mut macro_site = call.clone();
    macro_site.kind = crate::call_graph::CallKind::MacroInvocation;
    assert!(literal_arguments(caller, &macro_site, &args).is_empty());
    // A span past the end of the file, or one that is only part of a node, is no argument.
    let (text, span) = args[0].clone();
    let past = vec![(text.clone(), source.len() + 10..source.len() + 20)];
    assert!(literal_arguments(caller, &call, &past).is_empty());
    let partial = vec![(text, span.start..span.end - 1)];
    assert!(literal_arguments(caller, &call, &partial).is_empty());
}

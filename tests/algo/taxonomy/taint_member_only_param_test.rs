//! PR-C open point (design of record, step 0 section 4): why `--algorithm taint` findings on an
//! Express handler do not change when a framework parameter that is read only through member
//! paths (`req.query.cmd`) gains a base Def.
//!
//! Traced answer, pinned below:
//! 1. `detect_js_ts_framework_sources` seeds `TaintSeed::target(file, <fn start line>, "req")`.
//! 2. `synthesize_target_seed_paths` resolves it with `dfg_forward_reachable`, keyed by the
//!    owner NAME. For a named handler the owner is `handler`, so the new base Def now fans out
//!    to every `req.<field>` read (the accepted cost). For an anonymous callback the algorithm
//!    passes the literal owner `<anonymous>`, but the CPG owner is the synthetic `<cb@L:C>`
//!    identity, so the lookup finds nothing and the seed never fans out. That mismatch predates
//!    PR-C and is a gap, not a design choice.
//! 3. Either way the findings are unchanged, because every `req.<field>` read is already its own
//!    request-source line, detected without the parameter seed.

use crate::common::*;
use prism::data_flow::{VarAccessKind, VarLocation};

const NAMED: &str = "const express = require('express');\nconst { exec } = require('child_process');\nconst app = express();\nfunction handler(req, res) {\n  exec(req.query.cmd);\n  res.send(req.headers.agent);\n}\napp.get('/run', handler);\n";
const ANONYMOUS: &str = "const express = require('express');\nconst { exec } = require('child_process');\nconst app = express();\napp.get('/run', (req, res) => {\n  exec(req.query.cmd);\n  res.send(req.headers.agent);\n});\n";

fn parse(source: &str) -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([(
        "app.js".to_string(),
        ParsedFile::parse("app.js", source, Language::JavaScript).unwrap(),
    )])
}

fn findings(source: &str) -> BTreeSet<(usize, String)> {
    let lines = source.lines().count();
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "app.js".into(),
            modify_type: ModifyType::Modified,
            diff_lines: (1..=lines).collect(),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &parse(source),
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    result
        .findings
        .iter()
        .map(|f| (f.line, f.description.clone()))
        .collect()
}

fn seed_fan_out(source: &str, owner: &str) -> BTreeSet<(String, usize)> {
    let cpg = CodePropertyGraph::build(&parse(source));
    let from = VarLocation {
        file: "app.js".into(),
        function: owner.into(),
        function_start_line: 4,
        line: 4,
        path: AccessPath::simple("req"),
        start_byte: 0,
        end_byte: 0,
        kind: VarAccessKind::Def,
    };
    cpg.dfg_forward_reachable(&from)
        .into_iter()
        .map(|l| (l.path.to_string(), l.line))
        .collect()
}

#[test]
fn named_handler_parameter_seed_fans_out_to_every_field_read() {
    assert_eq!(
        seed_fan_out(NAMED, "handler"),
        BTreeSet::from([
            ("req.query.cmd".to_string(), 5),
            ("req.headers.agent".to_string(), 6),
        ])
    );
}

#[test]
fn anonymous_callback_parameter_seed_does_not_fan_out_owner_name_mismatch() {
    // The algorithm builds the seed with the literal owner "<anonymous>".
    assert!(seed_fan_out(ANONYMOUS, "<anonymous>").is_empty());
    // The real owner is the synthetic callback identity, which does hold the Def and fans out.
    let arrow_col = ANONYMOUS.lines().nth(3).unwrap().find("(req").unwrap() + 1;
    let owner = format!("<cb@4:{arrow_col}>");
    assert_eq!(
        seed_fan_out(ANONYMOUS, &owner),
        BTreeSet::from([
            ("req.query.cmd".to_string(), 5),
            ("req.headers.agent".to_string(), 6),
        ])
    );
}

#[test]
fn taint_findings_are_identical_for_named_and_anonymous_handlers() {
    let named = findings(NAMED);
    let anonymous = findings(ANONYMOUS);
    assert!(
        named.iter().any(|(line, _)| *line == 5) && named.iter().any(|(line, _)| *line == 6),
        "both direct request-source sinks are reported: {named:?}"
    );
    assert_eq!(
        named, anonymous,
        "the base-Def fan-out of a named handler adds no finding over the anonymous one"
    );
    // The fan-out reaches only lines that are already request-source sinks; no finding is
    // attributed to the signature line.
    assert!(!named.iter().any(|(line, _)| *line == 4), "{named:?}");
}

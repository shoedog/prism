//! PR-C R1 (design of record): member-only parameter Defs must not let a field-specific seed
//! reach a sibling field. The whole-parameter seed reaching every `p.<field>` read is the
//! accepted cost and is pinned here too.
//!
//! `nav taint-reaches` and MCP `taint_reaches` both call `reasoning::taint_reaches`, so these
//! cases cover both entry points; the binary entry points have their own tests in tests/cli and
//! tests/mcp.

use prism::navigation::types::Reachability;
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::reasoning::seeds::SeedSpec;
use prism::reasoning::taint_reaches::taint_reaches;
use prism::repo_loader::load_repo;
use std::sync::Arc;

const HANDLER_JS: &str = "function handler(req) {\n  sinkBody(req.body);\n  sinkHeaders(req.headers);\n  sinkQuery(req.query);\n}\n";
const HANDLER_TS: &str = "function handler(req: any) {\n  sinkBody(req.body);\n  sinkHeaders(req.headers);\n  sinkQuery(req.query);\n}\n";
const HANDLER_TSX: &str = HANDLER_TS;

fn session(file: &str, source: &str) -> (tempfile::TempDir, NavigationSession) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(file), source).unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    (dir, NavigationSession { repo, index })
}

/// Reachability of the sink variable endpoints on `line` for the given seed.
fn reach(
    session: &NavigationSession,
    file: &str,
    seed: SeedSpec,
    sink_line: usize,
) -> Vec<Reachability> {
    let evidence = taint_reaches(
        session,
        &[seed],
        Some(&[SeedSpec::Loc {
            file: file.into(),
            line: sink_line,
        }]),
    )
    .expect("taint_reaches");
    evidence
        .reasoning
        .as_ref()
        .expect("reasoning summary")
        .per_sink
        .iter()
        .map(|sink| sink.reachability)
        .collect()
}

fn cases() -> [(&'static str, &'static str); 3] {
    [
        ("h.js", HANDLER_JS),
        ("h.ts", HANDLER_TS),
        ("h.tsx", HANDLER_TSX),
    ]
}

#[test]
fn a_req_body_seed_never_reaches_sibling_fields() {
    for (file, source) in cases() {
        let (_dir, session) = session(file, source);
        let seed = || SeedSpec::Loc {
            file: file.into(),
            line: 2,
        };
        // Sanity: the seed reaches its own line (not vacuous).
        assert!(
            reach(&session, file, seed(), 2).contains(&Reachability::Reached),
            "{file}: the body seed reaches the body sink"
        );
        for (sink_line, name) in [(3, "headers"), (4, "query")] {
            let got = reach(&session, file, seed(), sink_line);
            assert!(
                !got.contains(&Reachability::Reached),
                "{file}: req.body seed must not reach req.{name}: {got:?}"
            );
        }
    }
}

#[test]
fn a_whole_parameter_seed_reaches_every_field_read_accepted_cost() {
    for (file, source) in cases() {
        let (_dir, session) = session(file, source);
        for sink_line in [2, 3, 4] {
            let got = reach(
                &session,
                file,
                SeedSpec::Symbol {
                    name: "handler".into(),
                    file: Some(file.into()),
                },
                sink_line,
            );
            assert!(
                got.contains(&Reachability::Reached),
                "{file}: whole-parameter seed reaches line {sink_line}: {got:?}"
            );
        }
    }
}

#[test]
fn a_symbol_seed_on_a_function_without_a_parameter_read_stays_not_found() {
    let (_dir, session) = session("n.js", "function handler(req) {\n  return 1;\n}\n");
    let err = taint_reaches(
        &session,
        &[SeedSpec::Symbol {
            name: "handler".into(),
            file: Some("n.js".into()),
        }],
        None,
    )
    .expect_err("an unused formal gives no seed");
    assert!(
        format!("{err:?}").contains("SymbolNotFound"),
        "unused formal must still be SymbolNotFound: {err:?}"
    );
}

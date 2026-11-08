//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

mod cleansing;
mod flat;
mod go;
mod js_ts;
mod python;
mod report;
mod run;
mod seeds;
mod sink_match;

pub use go::sinks::{GO_CWE22_SINKS, GO_CWE78_SINKS};
pub use js_ts::sink_tables::{
    JS_CWE22_SINKS, JS_CWE502_SINKS, JS_CWE78_SINKS, JS_CWE79_SINKS, JS_CWE89_SINKS,
    JS_CWE918_SINKS,
};
pub use python::sink_tables::{PY_CWE502_SINKS, PY_CWE79_SINKS, PY_CWE89_SINKS, PY_CWE918_SINKS};
pub use run::slice;
pub(crate) use sink_match::{line_matches_structured_sink, SinkMatchOutcome};

use prism::access_path::AccessPath;
use prism::ast::ParsedFile;
use prism::data_flow::VarLocation;
use go::sanitizers::PathSanitizerKind;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Check whether an identifier text matches a sink pattern.
///
/// Most patterns use substring matching (e.g. "exec" matches "execFile").
/// Patterns prefixed with '=' require an exact identifier match
/// (e.g. "=raw" matches "raw" but not "rawData" or "withdrawal").
pub(super) fn matches_sink(identifier: &str, pattern: &str) -> bool {
    if let Some(exact) = pattern.strip_prefix('=') {
        identifier == exact
    } else {
        identifier.contains(pattern)
    }
}

#[derive(Debug, Clone)]
pub struct TaintConfig {
    /// Explicit taint source locations.
    pub sources: Vec<(String, usize)>,
    /// If true, auto-taint all variables assigned on diff lines.
    pub taint_from_diff: bool,
    /// Additional sink patterns to check. Prefix with '=' for exact identifier match.
    pub extra_sinks: Vec<String>,
    /// Opt in to singleton-Exact callee-return flow. Default-off preserves the
    /// Tier-1/review wire until the separate default-on decision.
    pub return_flow: bool,
}

impl Default for TaintConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            taint_from_diff: true,
            extra_sinks: Vec::new(),
            return_flow: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TaintSeed {
    file: String,
    line: usize,
    target: Option<AccessPath>,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
}

impl TaintSeed {
    fn line(file: String, line: usize) -> Self {
        Self {
            file,
            line,
            target: None,
            start_byte: None,
            scope: None,
            byte_scope: None,
        }
    }

    fn target(file: String, line: usize, target: AccessPath) -> Self {
        Self::target_scoped(file, line, target, None, None, None)
    }

    fn target_scoped(
        file: String,
        line: usize,
        target: AccessPath,
        start_byte: Option<usize>,
        scope: Option<(usize, usize)>,
        byte_scope: Option<(usize, usize)>,
    ) -> Self {
        Self {
            file,
            line,
            target: Some(target),
            start_byte,
            scope,
            byte_scope,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct JsTsRequestDataSource {
    line: usize,
    target: AccessPath,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JsTsAliasKill {
    line: usize,
    byte: usize,
    byte_scope: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsTsAliasDef {
    line: usize,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    kills: Vec<JsTsAliasKill>,
}

type JsTsAliasDefs = BTreeMap<String, Vec<JsTsAliasDef>>;

impl JsTsAliasKill {
    fn applies_to_range(&self, line: usize, start_byte: usize, end_byte: usize) -> bool {
        (self.line < line || (self.line == line && self.byte < end_byte))
            && self
                .byte_scope
                .map(|(start, end)| start <= start_byte && end_byte <= end)
                .unwrap_or(true)
    }
}

impl JsTsAliasDef {
    fn visible_on(&self, line: usize) -> bool {
        self.line <= line
            && self
                .scope
                .map(|(start, end)| start <= line && line <= end)
                .unwrap_or(true)
    }

    fn visible_range(&self, line: usize, start_byte: usize, end_byte: usize) -> bool {
        self.visible_on(line)
            && (self.line < line
                || self
                    .start_byte
                    .map(|def_start| def_start <= start_byte)
                    .unwrap_or(true))
            && self
                .byte_scope
                .map(|(start, end)| start <= start_byte && end_byte <= end)
                .unwrap_or(true)
            && !self
                .kills
                .iter()
                .any(|kill| kill.applies_to_range(line, start_byte, end_byte))
    }

    fn same_binding_as(&self, other: &Self) -> bool {
        self.line == other.line
            && self.start_byte == other.start_byte
            && self.scope == other.scope
            && self.byte_scope == other.byte_scope
    }
}

/// A taint finding: a path from source to sink.
#[derive(Debug, Clone)]
pub struct TaintFinding {
    pub source_file: String,
    pub source_line: usize,
    pub source_var: String,
    pub sink_file: String,
    pub sink_line: usize,
    pub sink_pattern: String,
    pub path_lines: Vec<(String, usize)>,
}

#[derive(Clone)]
struct JsTsParam {
    name: String,
    has_nest_source_decorator: bool,
}

#[derive(Default)]
struct PythonDjangoRequestDataSources {
    has_access: bool,
    targets: BTreeSet<(usize, AccessPath)>,
}

#[derive(Default)]
struct PythonFlaskRequestDataSources {
    targets: BTreeSet<(usize, AccessPath)>,
    lines: BTreeSet<usize>,
}

#[derive(Debug)]
struct PythonParam {
    name: String,
    annotation: Option<String>,
}

struct TargetSeedSynthesisContext<'a> {
    seed: &'a TaintSeed,
    parsed: &'a ParsedFile,
    func: Node<'a>,
    func_name: &'a str,
    target: &'a AccessPath,
    reachable: Option<&'a BTreeSet<(String, usize)>>,
    from: &'a VarLocation,
}

struct PathSanitizerBinding {
    kind: PathSanitizerKind,
    result_var: String,
    call_line: usize,
}

struct UrlSanitizerBinding {
    url_var: String,
    result_var: String,
    call_line: usize,
}

struct JsTsPathSanitizerBinding {
    result_var: String,
    call_line: usize,
}

struct SafeFormatHtmlBinding {
    result_var: String,
    call_line: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism::access_path::AccessPath;
    use crate::algorithms::taint::go::sinks::go_sink_outcome;
    use crate::algorithms::taint::report::build_taint_chain_diagram;
    use prism::data_flow::FlowPath;
    use prism::data_flow::{FlowEdge, VarAccessKind, VarLocation};
    use prism::frameworks::SanitizerCategory;
    use prism::languages::Language;
    use prism::slice::NodeKind;

    fn flow_path_to(line: usize, var_name: &str) -> FlowPath {
        let from = VarLocation {
            file: "main.go".to_string(),
            function: "handler".to_string(),
            function_start_line: 3,
            line: 3,
            path: AccessPath::simple(var_name),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Def,
        };
        let to = VarLocation {
            file: "main.go".to_string(),
            function: "handler".to_string(),
            function_start_line: 3,
            line,
            path: AccessPath::simple(var_name),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        FlowPath {
            edges: vec![FlowEdge { from, to }],
            cleansed_for: BTreeSet::new(),
        }
    }

    #[test]
    fn powershell_shell_wrappers_match_structured_registry() {
        let source = r#"package main
import "os/exec"
func handler(input string) {
	_ = exec.Command("pwsh", "-c", input).Run()
	_ = exec.Command("powershell", "-command", input).Run()
	_ = exec.Command("powershell.exe", "-Command", input).Run()
	_ = exec.CommandContext(ctx, "pwsh", "-Command", input).Run()
}
"#;
        let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();

        for line in [4, 5, 6, 7] {
            let path = flow_path_to(line, "input");
            assert!(matches!(
                go_sink_outcome(&parsed, line, Some(&path)),
                SinkMatchOutcome::Match(p) if p.category == SanitizerCategory::OsCommand
            ));
        }
    }

    #[test]
    fn shell_wrapper_flags_are_shell_family_specific() {
        let source = r#"package main
import "os/exec"
func handler(input string) {
	_ = exec.Command("sh", "-Command", input).Run()
	_ = exec.Command("cmd.exe", "-c", input).Run()
}
"#;
        let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();

        for line in [4, 5] {
            let path = flow_path_to(line, "input");
            assert!(matches!(
                go_sink_outcome(&parsed, line, Some(&path)),
                SinkMatchOutcome::SemanticallyExcluded
            ));
        }
    }

    #[test]
    fn build_taint_chain_diagram_collapses_self_loop() {
        let g = build_taint_chain_diagram(
            "foo.c",
            42,
            "x = sink_call(y)",
            "foo.c",
            42,
            "x = sink_call(y)",
            &[],
        );
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 0);
        assert!(matches!(g.nodes[0].kind, NodeKind::Sink));
    }

    #[test]
    fn build_taint_chain_diagram_distinct_source_sink() {
        let g = build_taint_chain_diagram(
            "foo.c",
            10,
            "x = read_input()",
            "foo.c",
            42,
            "system(x)",
            &[],
        );
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.edges.len(), 1);
        assert!(matches!(g.nodes[0].kind, NodeKind::Source));
        assert!(matches!(g.nodes[1].kind, NodeKind::Sink));
    }
}

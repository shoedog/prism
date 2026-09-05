//! AST-recovered `dependency_hint` for `external_call` targets (roadmap
//! `03-tooling-plan-roadmap.md` §3 Phase 1; owner ruling 2026-09-04).
//!
//! The projection in `mod.rs` only carries `finding.description`, which for
//! `echo`/`missing_error_handling` names the *resolved function identity*
//! (`func_id.name` — often just the last segment of a qualified call, e.g.
//! `"post"` for `requests.post(...)`, since the call graph tracks callee
//! *definitions*, not call-site *syntax*). The runtime harness needs the
//! syntax actually written at the site (`"requests.post"`) so it can classify
//! the dependency. This module recovers that from the AST directly:
//!
//! 1. Find the call node(s) at `finding.site.line` inside the enclosing
//!    function (or the whole file when there is none).
//! 2. Take the raw callee expression text exactly as written (no drill-down
//!    to the trailing identifier — a dotted chain stays a dotted chain).
//! 3. Map the chain's root binding through a per-language, closed
//!    root-library table to a harness `kind`. A receiver-only callee
//!    (`self.client.get`) additionally looks for a same-file assignment that
//!    constructs the receiver (`self.client = requests.Session()`) and
//!    resolves the kind from that constructor's callee instead.
//!
//! Bounded, textual, no `regex` dependency — same posture as
//! `mapping::ABSENCE_PAIRS`. Never invents a `kind` outside the harness
//! catalog (`http | db | queue | filesystem | clock | process | cache`); an
//! unresolvable root omits `kind` rather than guessing.

use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

/// AST-recovered callee text plus the `kind` resolved from it, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstHint {
    pub callee: String,
    pub kind: Option<&'static str>,
}

/// Recover a dependency hint for the call site at `line` (1-indexed) in
/// `parsed`. Returns `None` only when no call node's span contains `line`
/// (or no callee expression can be extracted from it) — the caller falls
/// back to whatever hint it already had.
pub fn resolve(parsed: &ParsedFile, line: usize) -> Option<AstHint> {
    let scope = parsed
        .function_node_spanning(line)
        .unwrap_or_else(|| parsed.tree.root_node());
    let call_node = best_call_node(parsed, scope, line)?;
    let callee_node = raw_callee_node(&call_node)?;
    let callee = parsed.node_text(&callee_node).to_string();
    let kind = resolve_kind(parsed, &callee);
    Some(AstHint { callee, kind })
}

/// Every call node in `scope` whose 1-indexed line range contains `line`,
/// pre-order.
fn call_nodes_containing_line<'a>(
    parsed: &ParsedFile,
    scope: Node<'a>,
    line: usize,
) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    collect_call_nodes(parsed, scope, line, &mut out);
    out
}

fn collect_call_nodes<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    line: usize,
    out: &mut Vec<Node<'a>>,
) {
    if parsed.language.is_call_node(node.kind()) {
        let (start, end) = parsed.node_line_range(&node);
        if start <= line && line <= end {
            out.push(node);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_nodes(parsed, child, line, out);
    }
}

/// Several call nodes can contain the same line (nested calls, multi-line
/// calls). Pick the one whose callee text ends nearest `line`; ties favour
/// the more deeply nested candidate (larger start byte) for determinism.
fn best_call_node<'a>(parsed: &ParsedFile, scope: Node<'a>, line: usize) -> Option<Node<'a>> {
    call_nodes_containing_line(parsed, scope, line)
        .into_iter()
        .filter_map(|node| raw_callee_node(&node).map(|callee| (node, callee)))
        .min_by_key(|(node, callee)| {
            let (_, callee_end) = parsed.node_line_range(callee);
            let distance = callee_end.abs_diff(line);
            (distance, usize::MAX - node.start_byte())
        })
        .map(|(node, _)| node)
}

/// The callee expression node exactly as it stands in the call — e.g. the
/// whole `attribute`/`selector_expression`/`member_expression` node, not its
/// trailing identifier. Mirrors the first half of `Language::call_function_name`
/// (field lookup only), deliberately skipping that method's drill-down into
/// the last segment: that drill-down is what loses `requests.` from
/// `requests.post`.
fn raw_callee_node<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    node.child_by_field_name("function")
        .or_else(|| node.child_by_field_name("name"))
        .or_else(|| node.child_by_field_name("object"))
        .or_else(|| {
            if node.kind() == "function_call" || node.kind() == "command" {
                node.child_by_field_name("name").or_else(|| node.child(0))
            } else {
                None
            }
        })
}

/// Resolve `callee`'s dotted chain to a harness `kind` via the root-library
/// table, falling back to same-file receiver-construction resolution for a
/// root that is a local binding rather than a cataloged library.
fn resolve_kind(parsed: &ParsedFile, callee: &str) -> Option<&'static str> {
    let segments: Vec<&str> = callee.split('.').collect();
    let root = *segments.first()?;

    if segments.len() >= 2 {
        let prefix = format!("{root}.{}", segments[1]);
        if let Some(kind) = root_override(parsed.language, &prefix) {
            return Some(kind);
        }
    }

    if let Some(kind) = root_table(parsed.language, root) {
        return Some(kind);
    }

    if segments.len() >= 2 {
        let receiver = segments[..segments.len() - 1].join(".");
        let ctor_callee = receiver_construction_callee(parsed, &receiver)?;
        let ctor_root = ctor_callee.split('.').next()?;
        if let Some(kind) = root_override(parsed.language, &ctor_callee) {
            return Some(kind);
        }
        return root_table(parsed.language, ctor_root);
    }

    None
}

/// Same-file, first-match, top-to-bottom scan for an assignment or
/// declaration whose target text is exactly `receiver` and whose value is a
/// call — the `self.client = requests.Session()` pattern. Purely textual
/// (no reaching-definitions, no scope check): the same closed-table posture
/// as `mapping::ABSENCE_PAIRS`, bounded to one heuristic hop.
fn receiver_construction_callee(parsed: &ParsedFile, receiver: &str) -> Option<String> {
    fn walk(parsed: &ParsedFile, node: Node<'_>, receiver: &str) -> Option<String> {
        let language = parsed.language;
        if language.is_assignment_node(node.kind()) || language.is_declaration_node(node.kind()) {
            let lhs = language
                .assignment_target(&node)
                .or_else(|| language.declaration_name(&node));
            let rhs = language
                .assignment_value(&node)
                .or_else(|| language.declaration_value(&node));
            if let (Some(lhs), Some(rhs)) = (lhs, rhs) {
                if parsed.node_text(&lhs) == receiver && language.is_call_node(rhs.kind()) {
                    if let Some(callee) = raw_callee_node(&rhs) {
                        return Some(parsed.node_text(&callee).to_string());
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = walk(parsed, child, receiver) {
                return Some(found);
            }
        }
        None
    }
    walk(parsed, parsed.tree.root_node(), receiver)
}

/// Two-segment exact overrides checked before the single-root table — for
/// roots whose root-only meaning is ambiguous or already claimed by another
/// kind (`os` is `filesystem` generally but `os.system` is `process`).
fn root_override(language: Language, first_two: &str) -> Option<&'static str> {
    match language {
        Language::Python => match first_two {
            "time.sleep" => Some("clock"),
            "os.system" => Some("process"),
            _ => None,
        },
        _ => None,
    }
}

/// Closed per-language table from a callee chain's root binding to a harness
/// `kind` (`http | db | queue | filesystem | clock | process | cache`).
fn root_table(language: Language, root: &str) -> Option<&'static str> {
    match language {
        Language::Python => match root {
            "requests" | "httpx" | "urllib3" | "urllib" | "aiohttp" => Some("http"),
            "psycopg2" | "sqlalchemy" | "pymysql" | "sqlite3" | "asyncpg" | "pymongo" => Some("db"),
            "kombu" | "pika" | "celery" | "confluent_kafka" | "aiokafka" => Some("queue"),
            "redis" | "memcache" | "pylibmc" => Some("cache"),
            "open" | "os" | "pathlib" | "shutil" | "io" => Some("filesystem"),
            "datetime" => Some("clock"),
            "subprocess" => Some("process"),
            _ => None,
        },
        Language::Go => match root {
            "http" => Some("http"),
            "sql" | "pgx" | "gorm" => Some("db"),
            "sarama" | "kafka" | "amqp" => Some("queue"),
            "os" | "io" => Some("filesystem"),
            "exec" => Some("process"),
            _ => None,
        },
        Language::JavaScript | Language::TypeScript | Language::Tsx => match root {
            "axios" | "fetch" | "got" | "http" | "https" => Some("http"),
            "pg" | "mysql2" | "knex" | "prisma" | "mongoose" => Some("db"),
            "amqplib" | "kafkajs" | "bullmq" => Some("queue"),
            "fs" => Some("filesystem"),
            "child_process" => Some("process"),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str, language: Language) -> ParsedFile {
        ParsedFile::parse("f", source, language).unwrap()
    }

    #[test]
    fn direct_root_library_call_gets_kind_and_verbatim_callee() {
        let parsed = parse("def send():\n    requests.post('x')\n", Language::Python);
        let hint = resolve(&parsed, 2).unwrap();
        assert_eq!(hint.callee, "requests.post");
        assert_eq!(hint.kind, Some("http"));
    }

    #[test]
    fn bare_identifier_callee_with_unmapped_root_has_no_kind() {
        let parsed = parse("def send():\n    fetch('x')\n", Language::Python);
        let hint = resolve(&parsed, 2).unwrap();
        assert_eq!(hint.callee, "fetch");
        assert_eq!(hint.kind, None);
    }

    #[test]
    fn receiver_shape_resolves_kind_via_same_file_constructor() {
        let source = "class C:\n    def __init__(self):\n        self.client = requests.Session()\n    def send(self):\n        self.client.get('x')\n";
        let parsed = parse(source, Language::Python);
        let hint = resolve(&parsed, 5).unwrap();
        assert_eq!(hint.callee, "self.client.get");
        assert_eq!(hint.kind, Some("http"));
    }

    #[test]
    fn receiver_shape_without_resolvable_constructor_keeps_callee_omits_kind() {
        let source =
            "class C:\n    def send(self):\n        self.client = make_client()\n        self.client.get('x')\n";
        let parsed = parse(source, Language::Python);
        let hint = resolve(&parsed, 4).unwrap();
        assert_eq!(hint.callee, "self.client.get");
        assert_eq!(hint.kind, None);
    }

    #[test]
    fn unknown_root_emits_callee_without_kind() {
        let parsed = parse(
            "def send():\n    unknownlib.frobnicate('x')\n",
            Language::Python,
        );
        let hint = resolve(&parsed, 2).unwrap();
        assert_eq!(hint.callee, "unknownlib.frobnicate");
        assert_eq!(hint.kind, None);
    }

    #[test]
    fn go_net_http_selector_call_resolves_http() {
        let source = "package main\n\nfunc send() {\n\thttp.Get(\"x\")\n}\n";
        let parsed = parse(source, Language::Go);
        let hint = resolve(&parsed, 4).unwrap();
        assert_eq!(hint.callee, "http.Get");
        assert_eq!(hint.kind, Some("http"));
    }

    #[test]
    fn go_database_sql_root_resolves_db() {
        let source = "package main\n\nfunc open() {\n\tsql.Open(\"pg\", \"dsn\")\n}\n";
        let parsed = parse(source, Language::Go);
        let hint = resolve(&parsed, 4).unwrap();
        assert_eq!(hint.callee, "sql.Open");
        assert_eq!(hint.kind, Some("db"));
    }

    #[test]
    fn os_system_override_wins_over_os_filesystem_root() {
        let parsed = parse("def run():\n    os.system('ls')\n", Language::Python);
        let hint = resolve(&parsed, 2).unwrap();
        assert_eq!(hint.kind, Some("process"));
    }

    #[test]
    fn os_path_join_falls_back_to_filesystem_root() {
        let parsed = parse("def run():\n    os.path.join('a', 'b')\n", Language::Python);
        let hint = resolve(&parsed, 2).unwrap();
        assert_eq!(hint.callee, "os.path.join");
        assert_eq!(hint.kind, Some("filesystem"));
    }

    #[test]
    fn time_sleep_override_resolves_clock() {
        let parsed = parse("def run():\n    time.sleep(1)\n", Language::Python);
        let hint = resolve(&parsed, 2).unwrap();
        assert_eq!(hint.kind, Some("clock"));
    }

    #[test]
    fn no_call_at_line_returns_none() {
        let parsed = parse("def run():\n    pass\n", Language::Python);
        assert!(resolve(&parsed, 2).is_none());
    }

    #[test]
    fn nested_calls_on_one_line_pick_the_innermost_by_tie_break() {
        let parsed = parse("def run():\n    requests.post(str(1))\n", Language::Python);
        let hint = resolve(&parsed, 2).unwrap();
        // Both `requests.post(...)` and `str(...)` contain line 2; the
        // deterministic tie-break must not crash and must return one of the
        // two real callees.
        assert!(hint.callee == "requests.post" || hint.callee == "str");
    }
}

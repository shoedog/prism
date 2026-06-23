use super::input::{parse_nodes_at, GroupPolicy, SnippetPolicy, ViewFormat, ViewOptions};
use super::output::{shape_result, McpToolResult, Verbosity};
use crate::navigation::types::{
    Evidence, EvidenceItem, GraphNode, Location, Reason, SymbolRef, Warning,
};
use crate::navigation::NavigationSession;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const VIEW_SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone)]
pub enum NavigationViewKind {
    NodesAt,
    Callers,
    Callees {
        depth: usize,
        seed_file: Option<String>,
    },
    EgoGraph,
    ModuleDeps,
    RepoMap,
}

#[derive(Debug, Serialize)]
struct EvidenceView {
    query: String,
    profile: String,
    summary: ViewSummary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    groups: Vec<ViewGroup>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<ViewItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graph: Option<ViewGraph>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<Warning>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    next_queries: Vec<NextQuery>,
    meta: ViewMeta,
}

#[derive(Debug, Serialize)]
struct ViewSummary {
    visible_items: usize,
    total_items: usize,
    canonical_items: usize,
    truncated: bool,
    warnings: usize,
}

#[derive(Debug, Serialize)]
struct ViewGroup {
    key: String,
    item_count: usize,
    items: Vec<ViewItem>,
}

#[derive(Debug, Clone, Serialize)]
struct ViewItem {
    loc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    score: f32,
    reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
}

#[derive(Debug, Serialize)]
struct ViewGraph {
    visible_nodes: usize,
    total_nodes: usize,
    edges: usize,
    nodes: Vec<ViewNode>,
}

#[derive(Debug, Serialize)]
struct ViewNode {
    loc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
}

#[derive(Debug, Serialize)]
struct NextQuery {
    tool: &'static str,
    arguments: Value,
}

#[derive(Debug, Serialize)]
struct ViewMeta {
    schema_version: &'static str,
    content_text_format: &'static str,
    snippets: &'static str,
    group_by: &'static str,
    clipped_to_fit: bool,
}

pub fn shape_navigation_result(
    session: &NavigationSession,
    full: &Evidence,
    canonical: Evidence,
    total: usize,
    max_results_clipped: bool,
    verbosity: Verbosity,
    cap: usize,
    view: ViewOptions,
    kind: NavigationViewKind,
) -> McpToolResult {
    let canonical_result = shape_result(canonical, total, max_results_clipped, verbosity, cap);
    if !view.agent_requested() || canonical_result.is_error {
        return canonical_result;
    }

    let budget = super::transport::payload_budget(cap);
    let full_count = view_count(full);
    let canonical_items = structured_count(&canonical_result);
    let mut best = None;
    let mut lo = 0usize;
    let mut hi = full_count;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let candidate = compose_view_result(
            session,
            full,
            canonical_result.clone_like(),
            view,
            &kind,
            mid,
            total,
            canonical_items,
            mid < full_count,
        );
        if candidate.content_text.len() <= view.max_view_bytes
            && candidate.serialized_len() <= budget
        {
            best = Some(candidate);
            lo = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            hi = mid - 1;
        }
    }

    if let Some(result) = best {
        return result;
    }

    let mut fallback = canonical_result;
    fallback.content_text = bounded_notice(view.max_view_bytes);
    fallback.meta.insert(
        "prism/view_schema_version".into(),
        Value::String(VIEW_SCHEMA_VERSION.into()),
    );
    fallback.meta.insert(
        "prism/content_text_format".into(),
        Value::String(view.format.as_str().into()),
    );
    fallback
        .meta
        .insert("prism/view_clipped".into(), Value::Bool(true));
    fallback
}

fn compose_view_result(
    session: &NavigationSession,
    full: &Evidence,
    mut canonical_result: McpToolResult,
    view: ViewOptions,
    kind: &NavigationViewKind,
    item_limit: usize,
    total: usize,
    canonical_items: usize,
    clipped_to_fit: bool,
) -> McpToolResult {
    let evidence_view = build_view(
        session,
        full,
        view,
        kind,
        item_limit,
        total,
        canonical_items,
        clipped_to_fit,
    );
    canonical_result.content_text = match view.format {
        ViewFormat::AgentMarkdown => render_markdown(&evidence_view),
        ViewFormat::AgentJson => {
            serde_json::to_string_pretty(&evidence_view).unwrap_or_else(|_| "{}".into())
        }
        ViewFormat::CanonicalJson => canonical_result.content_text,
    };
    canonical_result.meta.insert(
        "prism/view_schema_version".into(),
        Value::String(VIEW_SCHEMA_VERSION.into()),
    );
    canonical_result.meta.insert(
        "prism/content_text_format".into(),
        Value::String(view.format.as_str().into()),
    );
    canonical_result.meta.insert(
        "prism/view_profile".into(),
        Value::String(view.profile.as_str().into()),
    );
    canonical_result.meta.insert(
        "prism/view_clipped".into(),
        Value::Bool(clipped_to_fit || item_limit < view_count(full)),
    );
    canonical_result
}

fn build_view(
    session: &NavigationSession,
    full: &Evidence,
    view: ViewOptions,
    kind: &NavigationViewKind,
    item_limit: usize,
    total: usize,
    canonical_items: usize,
    clipped_to_fit: bool,
) -> EvidenceView {
    let items = build_items(session, full, view, kind, item_limit);
    let graph = build_graph(full, item_limit);
    let groups = group_items(&items, view.group_by);
    let loose_items = if groups.is_empty() { items } else { Vec::new() };
    let visible_items = graph
        .as_ref()
        .map(|graph| graph.visible_nodes)
        .unwrap_or_else(|| {
            loose_items.len() + groups.iter().map(|group| group.items.len()).sum::<usize>()
        });
    EvidenceView {
        query: full.query.clone(),
        profile: view.profile.as_str().into(),
        summary: ViewSummary {
            visible_items,
            total_items: total,
            canonical_items,
            truncated: full.truncated || item_limit < view_count(full),
            warnings: full.warnings.len(),
        },
        groups,
        items: loose_items,
        graph,
        warnings: full.warnings.clone(),
        next_queries: next_queries(full, item_limit),
        meta: ViewMeta {
            schema_version: VIEW_SCHEMA_VERSION,
            content_text_format: view.format.as_str(),
            snippets: view.snippets.as_str(),
            group_by: view.group_by.as_str(),
            clipped_to_fit,
        },
    }
}

fn build_items(
    session: &NavigationSession,
    full: &Evidence,
    view: ViewOptions,
    kind: &NavigationViewKind,
    item_limit: usize,
) -> Vec<ViewItem> {
    full.items
        .iter()
        .take(item_limit)
        .map(|item| ViewItem {
            loc: format_location(&item.location),
            symbol: item.symbol.as_ref().map(symbol_label),
            score: item.score,
            reason: reason_label(item),
            snippet: snippet_for_item(session, item, view.snippets, kind),
        })
        .collect()
}

fn build_graph(full: &Evidence, item_limit: usize) -> Option<ViewGraph> {
    let graph = full.graph.as_ref()?;
    let nodes: Vec<_> = graph.nodes.iter().take(item_limit).map(view_node).collect();
    Some(ViewGraph {
        visible_nodes: nodes.len(),
        total_nodes: graph.nodes.len(),
        edges: graph.edges.len(),
        nodes,
    })
}

fn group_items(items: &[ViewItem], group_by: GroupPolicy) -> Vec<ViewGroup> {
    if matches!(group_by, GroupPolicy::None) {
        return Vec::new();
    }
    let mut grouped: BTreeMap<String, Vec<ViewItem>> = BTreeMap::new();
    for item in items {
        let key = match group_by {
            GroupPolicy::None => unreachable!(),
            GroupPolicy::File => item
                .loc
                .split_once(':')
                .map(|(file, _)| file.to_string())
                .unwrap_or_else(|| item.loc.clone()),
            GroupPolicy::Symbol => item.symbol.clone().unwrap_or_else(|| "(no symbol)".into()),
        };
        grouped.entry(key).or_default().push(item.clone());
    }
    grouped
        .into_iter()
        .map(|(key, items)| ViewGroup {
            item_count: items.len(),
            key,
            items,
        })
        .collect()
}

fn render_markdown(view: &EvidenceView) -> String {
    let mut out = String::new();
    out.push_str("# Prism Evidence\n");
    out.push_str(&format!(
        "query: `{}`\nprofile: `{}`\nitems: {} of {}\n",
        view.query, view.profile, view.summary.visible_items, view.summary.total_items
    ));
    if view.summary.truncated {
        out.push_str("truncated: true\n");
    }
    if !view.warnings.is_empty() {
        out.push_str("\n## Warnings\n");
        for warning in &view.warnings {
            out.push_str(&format!("- {}\n", warning.message));
        }
    }
    if let Some(graph) = &view.graph {
        out.push_str("\n## Graph\n");
        out.push_str(&format!(
            "nodes: {} of {}, edges: {}\n",
            graph.visible_nodes, graph.total_nodes, graph.edges
        ));
        for node in &graph.nodes {
            out.push_str(&format!(
                "- `{}` {}\n",
                node.loc,
                node.symbol.as_deref().unwrap_or("")
            ));
        }
    }
    for group in &view.groups {
        out.push_str(&format!("\n## {}\n", group.key));
        for item in &group.items {
            render_markdown_item(&mut out, item);
        }
    }
    if !view.items.is_empty() {
        out.push_str("\n## Items\n");
        for item in &view.items {
            render_markdown_item(&mut out, item);
        }
    }
    if !view.next_queries.is_empty() {
        out.push_str("\n## Next Queries\n");
        for query in &view.next_queries {
            out.push_str(&format!("- {} {}\n", query.tool, query.arguments));
        }
    }
    out
}

fn render_markdown_item(out: &mut String, item: &ViewItem) {
    out.push_str(&format!(
        "- `{}` {} score={:.2}; {}\n",
        item.loc,
        item.symbol.as_deref().unwrap_or(""),
        item.score,
        item.reason
    ));
    if let Some(snippet) = &item.snippet {
        out.push_str("  ");
        out.push_str(snippet);
        out.push('\n');
    }
}

fn snippet_for_item(
    session: &NavigationSession,
    item: &EvidenceItem,
    policy: SnippetPolicy,
    kind: &NavigationViewKind,
) -> Option<String> {
    match policy {
        SnippetPolicy::None => None,
        SnippetPolicy::SymbolHeader => {
            source_line(session, &item.location.file, item.location.start_line)
        }
        SnippetPolicy::Line => line_snippet(session, item, kind),
    }
}

fn line_snippet(
    session: &NavigationSession,
    item: &EvidenceItem,
    kind: &NavigationViewKind,
) -> Option<String> {
    match kind {
        NavigationViewKind::Callers => item.why.iter().find_map(|reason| match reason {
            Reason::CalledBy { call_site_line, .. } => {
                source_line(session, &item.location.file, *call_site_line)
            }
            _ => None,
        }),
        NavigationViewKind::Callees { depth, seed_file } if *depth == 1 => {
            let file = seed_file.as_deref()?;
            item.why.iter().find_map(|reason| match reason {
                Reason::Calls { call_site_line, .. } => source_line(session, file, *call_site_line),
                _ => None,
            })
        }
        NavigationViewKind::Callees { .. } => None,
        _ => source_line(session, &item.location.file, item.location.start_line),
    }
}

fn source_line(session: &NavigationSession, file: &str, line: usize) -> Option<String> {
    let parsed = session.repo.files.get(file)?;
    let text = parsed.source.lines().nth(line.checked_sub(1)?)?.trim_end();
    Some(format!("{line}: {text}"))
}

fn next_queries(full: &Evidence, item_limit: usize) -> Vec<NextQuery> {
    full.items
        .iter()
        .take(item_limit)
        .take(3)
        .filter_map(|item| nodes_at_query(&item.location))
        .collect()
}

fn nodes_at_query(location: &Location) -> Option<NextQuery> {
    let arguments = json!({
        "file": location.file,
        "line": location.start_line,
    });
    debug_assert!(parse_nodes_at(&arguments).is_ok());
    Some(NextQuery {
        tool: "nav_nodes_at",
        arguments,
    })
}

fn view_node(node: &GraphNode) -> ViewNode {
    ViewNode {
        loc: format_location(&node.location),
        symbol: node.symbol.as_ref().map(symbol_label),
    }
}

fn view_count(full: &Evidence) -> usize {
    full.graph
        .as_ref()
        .map(|graph| graph.nodes.len())
        .unwrap_or_else(|| full.items.len())
}

fn structured_count(result: &McpToolResult) -> usize {
    let Some(structured) = &result.structured else {
        return 0;
    };
    structured
        .get("graph")
        .and_then(|graph| graph.get("nodes"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .or_else(|| {
            structured
                .get("items")
                .and_then(Value::as_array)
                .map(Vec::len)
        })
        .unwrap_or(0)
}

fn format_location(location: &Location) -> String {
    if location.start_line == location.end_line {
        format!("{}:{}", location.file, location.start_line)
    } else {
        format!(
            "{}:{}-{}",
            location.file, location.start_line, location.end_line
        )
    }
}

fn symbol_label(symbol: &SymbolRef) -> String {
    match symbol {
        SymbolRef::Function {
            file,
            name,
            start_line,
            ..
        } => {
            format!("function {name} @ {file}:{start_line}")
        }
        SymbolRef::Statement {
            file, line, kind, ..
        } => {
            format!("statement {kind} @ {file}:{line}")
        }
        SymbolRef::Variable {
            file,
            function,
            line,
            path,
            access,
            ..
        } => format!("variable {path} {access} in {function} @ {file}:{line}"),
    }
}

fn reason_label(item: &EvidenceItem) -> String {
    let Some(reason) = item.why.first() else {
        return "matched evidence".into();
    };
    match reason {
        Reason::Calls {
            callee,
            call_site_line,
            qualifier,
        } => match qualifier {
            Some(qualifier) => format!("calls {qualifier}.{callee} at line {call_site_line}"),
            None => format!("calls {callee} at line {call_site_line}"),
        },
        Reason::CalledBy {
            caller,
            call_site_line,
        } => format!("called by {caller} at line {call_site_line}"),
        Reason::Resolution { kind } => format!("resolution {kind}"),
        Reason::EnclosingFunction { function } => {
            format!("inside {}", symbol_label(function))
        }
        Reason::Containment { parent } => format!("contained by {}", symbol_label(parent)),
        Reason::ResolvedImport {
            module,
            target_file,
        } => format!("import {module} resolves to {target_file}"),
        Reason::UnresolvedImport { module } => format!("unresolved import {module}"),
        Reason::Reasoning(reason) => format!("{reason:?}"),
    }
}

fn bounded_notice(max_view_bytes: usize) -> String {
    let notice = "Evidence view clipped to fit the MCP result cap; inspect structuredContent for canonical Evidence.";
    if notice.len() <= max_view_bytes {
        return notice.into();
    }
    let end = notice
        .char_indices()
        .map(|(idx, _)| idx)
        .take_while(|idx| *idx <= max_view_bytes)
        .last()
        .unwrap_or(0);
    notice[..end].into()
}

trait CloneMcpToolResult {
    fn clone_like(&self) -> McpToolResult;
}

impl CloneMcpToolResult for McpToolResult {
    fn clone_like(&self) -> McpToolResult {
        McpToolResult {
            content_text: self.content_text.clone(),
            structured: self.structured.clone(),
            is_error: self.is_error,
            meta: self.meta.clone(),
        }
    }
}

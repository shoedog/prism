use super::input::{
    parse_callees, parse_callers, parse_module_deps, parse_nodes_at, EvidenceProfile, GroupPolicy,
    SnippetPolicy, ViewFormat, ViewOptions,
};
use super::output::{shape_result, McpToolResult, Verbosity};
use crate::navigation::types::{
    Evidence, EvidenceItem, GraphNode, Location, Reason, Source, SymbolRef, Warning,
};
use crate::navigation::NavigationSession;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const VIEW_SCHEMA_VERSION: &str = "0.2";
const MAX_NEXT_QUERIES: usize = 5;

#[derive(Debug, Clone)]
pub enum NavigationViewKind {
    NodesAt,
    Callers,
    Callees {
        depth: usize,
        seed_file: Option<String>,
    },
    EgoGraph,
    ModuleDeps {
        file: String,
    },
    RepoMap,
}

#[derive(Debug, Clone, Copy)]
struct ProfilePolicy {
    default_group_by: GroupPolicy,
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
    visible_files: usize,
    exact: usize,
    fallback: usize,
    unresolved: usize,
    heuristic: usize,
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
    trust: &'static str,
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
    reason: &'static str,
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
    let policy = profile_policy(view.profile);
    let group_by = effective_group_by(view, full, policy);
    let items = build_items(session, full, view, kind, item_limit);
    let graph = build_graph(full, item_limit);
    let groups = group_items(&items, group_by);
    let loose_items = if groups.is_empty() { items } else { Vec::new() };
    let visible_items = graph
        .as_ref()
        .map(|graph| graph.visible_nodes)
        .unwrap_or_else(|| {
            loose_items.len() + groups.iter().map(|group| group.items.len()).sum::<usize>()
        });
    let trust_counts = trust_counts(&loose_items, &groups);
    EvidenceView {
        query: full.query.clone(),
        profile: view.profile.as_str().into(),
        summary: ViewSummary {
            visible_items,
            total_items: total,
            canonical_items,
            truncated: full.truncated || item_limit < view_count(full),
            warnings: full.warnings.len(),
            visible_files: visible_file_count(full, item_limit),
            exact: trust_counts.exact,
            fallback: trust_counts.fallback,
            unresolved: trust_counts.unresolved,
            heuristic: trust_counts.heuristic,
        },
        groups,
        items: loose_items,
        graph,
        warnings: full.warnings.clone(),
        next_queries: next_queries(
            full,
            item_limit,
            view,
            kind,
            full.truncated || item_limit < view_count(full),
        ),
        meta: ViewMeta {
            schema_version: VIEW_SCHEMA_VERSION,
            content_text_format: view.format.as_str(),
            snippets: view.snippets.as_str(),
            group_by: group_by.as_str(),
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
            trust: trust_label(item),
            reason: reason_label(item, view.profile),
            snippet: snippet_for_item(session, item, view.snippets, kind),
        })
        .collect()
}

fn profile_policy(profile: EvidenceProfile) -> ProfilePolicy {
    let default_group_by = match profile {
        EvidenceProfile::Impact => GroupPolicy::Symbol,
        EvidenceProfile::Dependencies => GroupPolicy::File,
        EvidenceProfile::Orientation
        | EvidenceProfile::EditContext
        | EvidenceProfile::Audit
        | EvidenceProfile::Seed
        | EvidenceProfile::Graph => GroupPolicy::None,
    };
    ProfilePolicy { default_group_by }
}

fn effective_group_by(view: ViewOptions, full: &Evidence, policy: ProfilePolicy) -> GroupPolicy {
    if view.group_by_explicit {
        return view.group_by;
    }
    match policy.default_group_by {
        GroupPolicy::Symbol if full.items.iter().any(|item| item.symbol.is_some()) => {
            GroupPolicy::Symbol
        }
        GroupPolicy::Symbol => GroupPolicy::File,
        other => other,
    }
}

#[derive(Default)]
struct TrustCounts {
    exact: usize,
    fallback: usize,
    unresolved: usize,
    heuristic: usize,
}

fn trust_counts(items: &[ViewItem], groups: &[ViewGroup]) -> TrustCounts {
    let mut counts = TrustCounts::default();
    for item in items
        .iter()
        .chain(groups.iter().flat_map(|group| group.items.iter()))
    {
        match item.trust {
            "fallback" => counts.fallback += 1,
            "unresolved" => counts.unresolved += 1,
            "heuristic" => counts.heuristic += 1,
            _ => counts.exact += 1,
        }
    }
    counts
}

fn visible_file_count(full: &Evidence, item_limit: usize) -> usize {
    let mut files = BTreeSet::new();
    for item in full.items.iter().take(item_limit) {
        files.insert(item.location.file.as_str());
    }
    if let Some(graph) = &full.graph {
        for node in graph.nodes.iter().take(item_limit) {
            files.insert(node.location.file.as_str());
        }
    }
    files.len()
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
        "query: `{}`\nprofile: `{}`\nitems: {} of {}\nfiles: {}\ntrust: exact={} fallback={} unresolved={} heuristic={}\n",
        view.query,
        view.profile,
        view.summary.visible_items,
        view.summary.total_items,
        view.summary.visible_files,
        view.summary.exact,
        view.summary.fallback,
        view.summary.unresolved,
        view.summary.heuristic
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
            out.push_str(&format!(
                "- {} reason={} {}\n",
                query.tool, query.reason, query.arguments
            ));
        }
    }
    out
}

fn render_markdown_item(out: &mut String, item: &ViewItem) {
    out.push_str(&format!(
        "- `{}` {} score={:.2}; trust={}; {}\n",
        item.loc,
        item.symbol.as_deref().unwrap_or(""),
        item.score,
        item.trust,
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

fn next_queries(
    full: &Evidence,
    item_limit: usize,
    view: ViewOptions,
    kind: &NavigationViewKind,
    truncated: bool,
) -> Vec<NextQuery> {
    let mut queries = Vec::new();
    let mut seen = BTreeSet::new();
    match view.profile {
        EvidenceProfile::Impact if matches!(kind, NavigationViewKind::Callers) => {
            add_callers_hints(full, item_limit, &mut queries, &mut seen);
        }
        EvidenceProfile::Dependencies => match kind {
            NavigationViewKind::Callees { .. } => {
                add_callees_hints(full, item_limit, kind, &mut queries, &mut seen);
            }
            NavigationViewKind::ModuleDeps { .. } => {
                add_module_deps_hints(full, item_limit, kind, &mut queries, &mut seen);
            }
            _ => add_item_locator_hints(full, item_limit, "edit_locator", &mut queries, &mut seen),
        },
        EvidenceProfile::Orientation => match kind {
            NavigationViewKind::RepoMap => {
                add_repo_map_hints(full, item_limit, &mut queries, &mut seen);
            }
            NavigationViewKind::ModuleDeps { .. } => {
                add_module_deps_hints(full, item_limit, kind, &mut queries, &mut seen);
            }
            _ => add_graph_locator_hints(full, item_limit, &mut queries, &mut seen),
        },
        EvidenceProfile::Seed | EvidenceProfile::EditContext => {
            add_item_locator_hints(full, item_limit, "edit_locator", &mut queries, &mut seen);
        }
        EvidenceProfile::Graph => {
            add_graph_locator_hints(full, item_limit, &mut queries, &mut seen)
        }
        EvidenceProfile::Audit => {
            add_item_locator_hints(full, item_limit, "edit_locator", &mut queries, &mut seen);
        }
        _ => add_item_locator_hints(full, item_limit, "edit_locator", &mut queries, &mut seen),
    }

    if truncated {
        add_truncation_hint(full, item_limit, &mut queries, &mut seen);
    }
    queries
}

fn add_callers_hints(
    full: &Evidence,
    item_limit: usize,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    for item in full.items.iter().take(item_limit) {
        for reason in &item.why {
            if let Reason::CalledBy { call_site_line, .. } = reason {
                push_nodes_at_query(
                    queries,
                    seen,
                    "call_site",
                    &item.location.file,
                    *call_site_line,
                );
            }
        }
        push_symbol_query(queries, seen, "nav_callers", "caller_symbol", &item.symbol);
    }
}

fn add_callees_hints(
    full: &Evidence,
    item_limit: usize,
    kind: &NavigationViewKind,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    for item in full.items.iter().take(item_limit) {
        if let NavigationViewKind::Callees {
            depth: 1,
            seed_file: Some(seed_file),
        } = kind
        {
            for reason in &item.why {
                if let Reason::Calls { call_site_line, .. } = reason {
                    push_nodes_at_query(queries, seen, "call_site", seed_file, *call_site_line);
                }
            }
        }
        let reason = if item.symbol.is_some() {
            "callee_definition"
        } else {
            "edit_locator"
        };
        push_nodes_at_location(queries, seen, reason, &item.location);
        push_symbol_query(queries, seen, "nav_callees", "callee_symbol", &item.symbol);
    }
}

fn add_module_deps_hints(
    full: &Evidence,
    item_limit: usize,
    kind: &NavigationViewKind,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    let source_file = match kind {
        NavigationViewKind::ModuleDeps { file } => Some(file.as_str()),
        _ => None,
    };
    for item in full.items.iter().take(item_limit) {
        if let Some(source_file) = source_file {
            for reason in &item.why {
                if let Reason::Calls { call_site_line, .. } = reason {
                    push_nodes_at_query(queries, seen, "call_site", source_file, *call_site_line);
                }
            }
        }
        push_nodes_at_location(queries, seen, "dependency_target", &item.location);
    }
}

fn add_repo_map_hints(
    full: &Evidence,
    item_limit: usize,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    if let Some(graph) = &full.graph {
        for node in graph.nodes.iter().take(item_limit) {
            let arguments = json!({ "file": node.location.file });
            push_query(
                queries,
                seen,
                "nav_module_deps",
                "inspect_module",
                arguments,
            );
        }
    }
}

fn add_item_locator_hints(
    full: &Evidence,
    item_limit: usize,
    reason: &'static str,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    for item in full.items.iter().take(item_limit) {
        push_nodes_at_location(queries, seen, reason, &item.location);
    }
}

fn add_graph_locator_hints(
    full: &Evidence,
    item_limit: usize,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    if let Some(graph) = &full.graph {
        for node in graph.nodes.iter().take(item_limit) {
            push_nodes_at_location(queries, seen, "edit_locator", &node.location);
        }
    }
}

fn add_truncation_hint(
    full: &Evidence,
    item_limit: usize,
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
) {
    if let Some(item) = full.items.iter().take(item_limit).next() {
        push_nodes_at_location(queries, seen, "result_truncated", &item.location);
        return;
    }
    if let Some(node) = full
        .graph
        .as_ref()
        .and_then(|graph| graph.nodes.iter().take(item_limit).next())
    {
        push_nodes_at_location(queries, seen, "result_truncated", &node.location);
    }
}

fn push_nodes_at_location(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    reason: &'static str,
    location: &Location,
) {
    push_nodes_at_query(queries, seen, reason, &location.file, location.start_line);
}

fn push_nodes_at_query(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    reason: &'static str,
    file: &str,
    line: usize,
) {
    let arguments = json!({
        "file": file,
        "line": line,
    });
    push_query(queries, seen, "nav_nodes_at", reason, arguments);
}

fn push_symbol_query(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    tool: &'static str,
    reason: &'static str,
    symbol: &Option<SymbolRef>,
) {
    let Some(SymbolRef::Function { file, name, .. }) = symbol else {
        return;
    };
    let arguments = json!({
        "seed": {
            "kind": "symbol",
            "name": name,
            "file": file,
        }
    });
    push_query(queries, seen, tool, reason, arguments);
}

fn push_query(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    tool: &'static str,
    reason: &'static str,
    arguments: Value,
) {
    if queries.len() >= MAX_NEXT_QUERIES || !query_arguments_valid(tool, &arguments) {
        return;
    }
    let Ok(arguments_key) = serde_json::to_string(&arguments) else {
        return;
    };
    let key = format!("{tool}:{reason}:{arguments_key}");
    if seen.insert(key) {
        queries.push(NextQuery {
            tool,
            reason,
            arguments,
        });
    }
}

fn query_arguments_valid(tool: &str, arguments: &Value) -> bool {
    match tool {
        "nav_nodes_at" => parse_nodes_at(arguments).is_ok(),
        "nav_callers" => parse_callers(arguments).is_ok(),
        "nav_callees" => parse_callees(arguments).is_ok(),
        "nav_module_deps" => parse_module_deps(arguments).is_ok(),
        _ => false,
    }
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

fn trust_label(item: &EvidenceItem) -> &'static str {
    if item
        .why
        .iter()
        .any(|reason| matches!(reason, Reason::UnresolvedImport { .. }))
        || (item.symbol.is_none()
            && item
                .why
                .iter()
                .any(|reason| matches!(reason, Reason::Calls { .. }))
            && !item
                .why
                .iter()
                .any(|reason| matches!(reason, Reason::Resolution { .. })))
    {
        "unresolved"
    } else if item.fallback {
        "fallback"
    } else if !matches!(item.source, Source::PrismCpg) {
        "heuristic"
    } else {
        "exact"
    }
}

fn reason_label(item: &EvidenceItem, profile: EvidenceProfile) -> String {
    let Some(reason) = item.why.first() else {
        return "matched evidence".into();
    };
    let base = match reason {
        Reason::Calls {
            callee,
            call_site_line,
            qualifier,
        } => {
            let callee = match qualifier {
                Some(qualifier) => format!("{qualifier}.{callee}"),
                None => callee.clone(),
            };
            if matches!(profile, EvidenceProfile::Dependencies) && trust_label(item) == "unresolved"
            {
                format!("unresolved call `{callee}` at line {call_site_line}")
            } else {
                format!("calls `{callee}` at line {call_site_line}")
            }
        }
        Reason::CalledBy {
            caller,
            call_site_line,
        } if matches!(profile, EvidenceProfile::Impact) => {
            format!("caller `{caller}` at call site line {call_site_line}")
        }
        Reason::CalledBy {
            caller,
            call_site_line,
        } => format!("called by `{caller}` at line {call_site_line}"),
        Reason::Resolution { kind } => format!("resolution {kind}"),
        Reason::EnclosingFunction { function } => {
            format!("inside {}", symbol_label(function))
        }
        Reason::Containment { parent } => format!("contained by {}", symbol_label(parent)),
        Reason::ResolvedImport {
            module,
            target_file,
        } => format!("import `{module}` resolves to `{target_file}`"),
        Reason::UnresolvedImport { module } => format!("unresolved import `{module}`"),
        Reason::Reasoning(reason) => format!("{reason:?}"),
    };
    match profile {
        EvidenceProfile::Audit => format!(
            "{base}; source={}; fallback={}",
            source_label(&item.source),
            item.fallback
        ),
        EvidenceProfile::EditContext => format!("edit locator; {base}"),
        _ => base,
    }
}

fn source_label(source: &Source) -> &'static str {
    match source {
        Source::PrismCpg => "prism_cpg",
        Source::HeuristicImport => "heuristic_import",
        Source::ExternalIndex { .. } => "external_index",
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

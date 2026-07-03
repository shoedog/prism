use super::input::{
    parse_callees, parse_callers, parse_module_deps, parse_nodes_at, EvidenceProfile, GroupPolicy,
    SnippetPolicy, ViewFormat, ViewOptions,
};
use super::output::{shape_result, McpToolResult, Verbosity};
use crate::navigation::code_context::{classify_file, CodeRole};
use crate::navigation::types::{
    Evidence, EvidenceItem, GraphNode, Location, Reason, ReasoningReason, Source, SymbolRef,
    Warning,
};
use crate::navigation::NavigationSession;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const VIEW_SCHEMA_VERSION: &str = "0.4";
const VIEW_INDEXING_POLICY: &str = "code_role_v1";
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
    production: usize,
    test: usize,
}

#[derive(Debug, Serialize)]
struct ViewGroup {
    key: String,
    item_count: usize,
    file_count: usize,
    trust: TrustCounts,
    code: CodeCounts,
    representative_locations: Vec<ViewLocation>,
    items: Vec<ViewItem>,
}

#[derive(Debug, Clone, Serialize)]
struct ViewItem {
    loc: String,
    location: ViewLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol_ref: Option<SymbolRef>,
    score: f32,
    trust: &'static str,
    reason: String,
    reasons: Vec<ViewReason>,
    code: ViewCodeContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ViewCodeContext {
    role: &'static str,
    is_test: bool,
    reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct ViewLocation {
    file: String,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
    display: String,
}

#[derive(Debug, Clone, Serialize)]
struct ViewReason {
    kind: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
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
    location: ViewLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    code: ViewCodeContext,
}

#[derive(Debug, Serialize)]
struct NextQuery {
    tool: &'static str,
    reason: &'static str,
    arguments: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_location: Option<ViewSourceLocation>,
}

#[derive(Debug, Clone, Serialize)]
struct ViewSourceLocation {
    kind: &'static str,
    file: String,
    line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_byte: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_byte: Option<usize>,
    display: String,
}

#[derive(Debug, Serialize)]
struct ViewMeta {
    schema_version: &'static str,
    content_text_format: &'static str,
    snippets: &'static str,
    group_by: &'static str,
    clipped_to_fit: bool,
    indexing_policy: &'static str,
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
    concise_shape_mode: super::concise_shape::ConciseShapeMode,
) -> McpToolResult {
    let canonical_result = shape_result(canonical, total, max_results_clipped, verbosity, cap);
    if !view.agent_requested() || canonical_result.is_error {
        // S3: apply the Concise item-slimming transform ONLY here, on the copy that becomes the
        // FINAL default-path response — never on `canonical_result` itself before this branch,
        // since the agent-view path below clones it (`clone_like`) into structuredContent, which
        // must stay the full canonical shape regardless of `PRISM_MCP_CONCISE_SHAPE`.
        return super::concise_shape::apply_concise_shape(
            canonical_result,
            verbosity,
            concise_shape_mode,
        );
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
    fallback.meta.insert(
        "prism/view_profile".into(),
        Value::String(view.profile.as_str().into()),
    );
    fallback.meta.insert(
        "prism/view_indexing_policy".into(),
        Value::String(VIEW_INDEXING_POLICY.into()),
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
        "prism/view_indexing_policy".into(),
        Value::String(VIEW_INDEXING_POLICY.into()),
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
    let code_counts = graph
        .as_ref()
        .map(|graph| code_counts_for_nodes(&graph.nodes))
        .unwrap_or_else(|| code_counts(&loose_items, &groups));
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
            production: code_counts.production,
            test: code_counts.test,
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
            indexing_policy: VIEW_INDEXING_POLICY,
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
        .map(|item| {
            let location = view_location(&item.location);
            ViewItem {
                loc: location.display.clone(),
                location,
                symbol: item.symbol.as_ref().map(symbol_label),
                symbol_ref: item.symbol.clone(),
                score: item.score,
                trust: trust_label(item),
                reason: reason_label(item, view.profile),
                reasons: view_reasons(item, view.profile),
                code: view_code_context(&item.location.file),
                snippet: snippet_for_item(session, item, view.snippets, kind),
            }
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

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct TrustCounts {
    exact: usize,
    fallback: usize,
    unresolved: usize,
    heuristic: usize,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct CodeCounts {
    production: usize,
    test: usize,
}

fn trust_counts(items: &[ViewItem], groups: &[ViewGroup]) -> TrustCounts {
    trust_counts_for_items(
        items
            .iter()
            .chain(groups.iter().flat_map(|group| group.items.iter())),
    )
}

fn trust_counts_for_items<'a>(items: impl IntoIterator<Item = &'a ViewItem>) -> TrustCounts {
    let mut counts = TrustCounts::default();
    for item in items {
        match item.trust {
            "fallback" => counts.fallback += 1,
            "unresolved" => counts.unresolved += 1,
            "heuristic" => counts.heuristic += 1,
            _ => counts.exact += 1,
        }
    }
    counts
}

fn code_counts(items: &[ViewItem], groups: &[ViewGroup]) -> CodeCounts {
    code_counts_for_items(
        items
            .iter()
            .chain(groups.iter().flat_map(|group| group.items.iter())),
    )
}

fn code_counts_for_items<'a>(items: impl IntoIterator<Item = &'a ViewItem>) -> CodeCounts {
    let mut counts = CodeCounts::default();
    for item in items {
        counts.add(&item.code);
    }
    counts
}

fn code_counts_for_nodes(nodes: &[ViewNode]) -> CodeCounts {
    let mut counts = CodeCounts::default();
    for node in nodes {
        counts.add(&node.code);
    }
    counts
}

impl CodeCounts {
    fn add(&mut self, code: &ViewCodeContext) {
        if code.is_test {
            self.test += 1;
        } else {
            self.production += 1;
        }
    }
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
            GroupPolicy::File => item.location.file.clone(),
            GroupPolicy::Symbol => item.symbol.clone().unwrap_or_else(|| "(no symbol)".into()),
        };
        grouped.entry(key).or_default().push(item.clone());
    }
    grouped
        .into_iter()
        .map(|(key, items)| ViewGroup {
            item_count: items.len(),
            file_count: file_count_for_items(&items),
            trust: trust_counts_for_items(items.iter()),
            code: code_counts_for_items(items.iter()),
            representative_locations: items
                .iter()
                .take(3)
                .map(|item| item.location.clone())
                .collect(),
            key,
            items,
        })
        .collect()
}

fn file_count_for_items(items: &[ViewItem]) -> usize {
    items
        .iter()
        .map(|item| item.location.file.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn render_markdown(view: &EvidenceView) -> String {
    let mut out = String::new();
    out.push_str("# Prism Evidence\n");
    out.push_str(&format!(
        "query: `{}`\nprofile: `{}`\nitems: {} of {}\nfiles: {}\ntrust: exact={} fallback={} unresolved={} heuristic={}\ncode: production={} test={}\n",
        view.query,
        view.profile,
        view.summary.visible_items,
        view.summary.total_items,
        view.summary.visible_files,
        view.summary.exact,
        view.summary.fallback,
        view.summary.unresolved,
        view.summary.heuristic,
        view.summary.production,
        view.summary.test
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
                "- `{}` {} code={}\n",
                node.loc,
                node.symbol.as_deref().unwrap_or(""),
                node.code.role
            ));
        }
    }
    for group in &view.groups {
        let trust = markdown_trust_counts(&group.trust);
        let code = markdown_code_counts(&group.code);
        let counts = join_non_empty([trust.as_str(), code.as_str()]);
        if counts.is_empty() {
            out.push_str(&format!(
                "\n## {} (items={} files={})\n",
                group.key, group.item_count, group.file_count
            ));
        } else {
            out.push_str(&format!(
                "\n## {} (items={} files={} {})\n",
                group.key, group.item_count, group.file_count, counts
            ));
        }
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

fn markdown_trust_counts(counts: &TrustCounts) -> String {
    let mut parts = Vec::new();
    if counts.exact > 0 {
        parts.push(format!("exact={}", counts.exact));
    }
    if counts.fallback > 0 {
        parts.push(format!("fallback={}", counts.fallback));
    }
    if counts.unresolved > 0 {
        parts.push(format!("unresolved={}", counts.unresolved));
    }
    if counts.heuristic > 0 {
        parts.push(format!("heuristic={}", counts.heuristic));
    }
    parts.join(" ")
}

fn markdown_code_counts(counts: &CodeCounts) -> String {
    let mut parts = Vec::new();
    if counts.production > 0 {
        parts.push(format!("production={}", counts.production));
    }
    if counts.test > 0 {
        parts.push(format!("test={}", counts.test));
    }
    parts.join(" ")
}

fn join_non_empty<const N: usize>(parts: [&str; N]) -> String {
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_markdown_item(out: &mut String, item: &ViewItem) {
    out.push_str(&format!(
        "- `{}` {} score={:.2}; trust={}; code={}; {}\n",
        item.loc,
        item.symbol.as_deref().unwrap_or(""),
        item.score,
        item.trust,
        item.code.role,
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
                    Some(call_site_source_location(
                        &item.location.file,
                        *call_site_line,
                    )),
                );
            }
        }
        push_symbol_query(
            queries,
            seen,
            "nav_callers",
            "caller_symbol",
            &item.symbol,
            Some(item_source_location(item)),
        );
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
                    push_nodes_at_query(
                        queries,
                        seen,
                        "call_site",
                        seed_file,
                        *call_site_line,
                        Some(call_site_source_location(seed_file, *call_site_line)),
                    );
                }
            }
        }
        let reason = if item.symbol.is_some() {
            "callee_definition"
        } else {
            "edit_locator"
        };
        push_nodes_at_item(queries, seen, reason, item);
        push_symbol_query(
            queries,
            seen,
            "nav_callees",
            "callee_symbol",
            &item.symbol,
            Some(item_source_location(item)),
        );
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
                    push_nodes_at_query(
                        queries,
                        seen,
                        "call_site",
                        source_file,
                        *call_site_line,
                        Some(call_site_source_location(source_file, *call_site_line)),
                    );
                }
            }
        }
        push_nodes_at_item(queries, seen, "dependency_target", item);
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
                Some(graph_source_location(node)),
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
        push_nodes_at_item(queries, seen, reason, item);
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
            push_nodes_at_graph_node(queries, seen, "edit_locator", node);
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
        push_nodes_at_item(queries, seen, "result_truncated", item);
        return;
    }
    if let Some(node) = full
        .graph
        .as_ref()
        .and_then(|graph| graph.nodes.iter().take(item_limit).next())
    {
        push_nodes_at_graph_node(queries, seen, "result_truncated", node);
    }
}

fn push_nodes_at_item(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    reason: &'static str,
    item: &EvidenceItem,
) {
    push_nodes_at_query(
        queries,
        seen,
        reason,
        &item.location.file,
        item.location.start_line,
        Some(item_source_location(item)),
    );
}

fn push_nodes_at_graph_node(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    reason: &'static str,
    node: &GraphNode,
) {
    push_nodes_at_query(
        queries,
        seen,
        reason,
        &node.location.file,
        node.location.start_line,
        Some(graph_source_location(node)),
    );
}

fn push_nodes_at_query(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    reason: &'static str,
    file: &str,
    line: usize,
    source_location: Option<ViewSourceLocation>,
) {
    let arguments = json!({
        "file": file,
        "line": line,
    });
    push_query(
        queries,
        seen,
        "nav_nodes_at",
        reason,
        arguments,
        source_location,
    );
}

fn push_symbol_query(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    tool: &'static str,
    reason: &'static str,
    symbol: &Option<SymbolRef>,
    source_location: Option<ViewSourceLocation>,
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
    push_query(queries, seen, tool, reason, arguments, source_location);
}

fn push_query(
    queries: &mut Vec<NextQuery>,
    seen: &mut BTreeSet<String>,
    tool: &'static str,
    reason: &'static str,
    arguments: Value,
    source_location: Option<ViewSourceLocation>,
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
            source_location,
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
    let location = view_location(&node.location);
    ViewNode {
        loc: location.display.clone(),
        code: view_code_context(&node.location.file),
        location,
        symbol: node.symbol.as_ref().map(symbol_label),
    }
}

fn view_code_context(file: &str) -> ViewCodeContext {
    let context = classify_file(file);
    ViewCodeContext {
        role: role_label(context.role),
        is_test: context.is_test,
        reasons: context.reasons,
    }
}

fn role_label(role: CodeRole) -> &'static str {
    match role {
        CodeRole::Production => "production",
        CodeRole::Test => "test",
    }
}

fn view_location(location: &Location) -> ViewLocation {
    ViewLocation {
        file: location.file.clone(),
        start_line: location.start_line,
        end_line: location.end_line,
        start_byte: location.start_byte,
        end_byte: location.end_byte,
        display: format_location(location),
    }
}

fn item_source_location(item: &EvidenceItem) -> ViewSourceLocation {
    evidence_source_location("item", &item.location)
}

fn graph_source_location(node: &GraphNode) -> ViewSourceLocation {
    evidence_source_location("graph_node", &node.location)
}

fn evidence_source_location(kind: &'static str, location: &Location) -> ViewSourceLocation {
    ViewSourceLocation {
        kind,
        file: location.file.clone(),
        line: location.start_line,
        start_byte: Some(location.start_byte),
        end_byte: Some(location.end_byte),
        display: format_location(location),
    }
}

fn call_site_source_location(file: &str, line: usize) -> ViewSourceLocation {
    ViewSourceLocation {
        kind: "call_site",
        file: file.into(),
        line,
        start_byte: None,
        end_byte: None,
        display: format!("{file}:{line}"),
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

fn symbol_file_line(symbol: &SymbolRef) -> (String, usize) {
    match symbol {
        SymbolRef::Function {
            file, start_line, ..
        } => (file.clone(), *start_line),
        SymbolRef::Statement { file, line, .. } | SymbolRef::Variable { file, line, .. } => {
            (file.clone(), *line)
        }
    }
}

fn view_reasons(item: &EvidenceItem, profile: EvidenceProfile) -> Vec<ViewReason> {
    let mut reasons = Vec::new();
    if matches!(profile, EvidenceProfile::Audit) {
        reasons.extend(item.why.iter().map(view_reason));
    } else if let Some(first) = item.why.first() {
        reasons.push(view_reason(first));
        if !matches!(first, Reason::Resolution { .. }) {
            if let Some(resolution) = item
                .why
                .iter()
                .find(|reason| matches!(reason, Reason::Resolution { .. }))
            {
                reasons.push(view_reason(resolution));
            }
        }
    }
    if reasons.is_empty() {
        reasons.push(ViewReason {
            kind: "matched_evidence".into(),
            label: "matched evidence".into(),
            detail: None,
            target: None,
            file: None,
            line: None,
        });
    }
    reasons
}

fn view_reason(reason: &Reason) -> ViewReason {
    match reason {
        Reason::Calls {
            callee,
            call_site_line,
            qualifier,
        } => {
            let display_callee = qualifier
                .as_ref()
                .map(|qualifier| format!("{qualifier}.{callee}"))
                .unwrap_or_else(|| callee.clone());
            ViewReason {
                kind: "calls".into(),
                label: format!("calls `{display_callee}` at line {call_site_line}"),
                detail: qualifier
                    .as_ref()
                    .map(|qualifier| format!("qualifier={qualifier}")),
                target: Some(callee.clone()),
                file: None,
                line: Some(*call_site_line),
            }
        }
        Reason::CalledBy {
            caller,
            call_site_line,
        } => ViewReason {
            kind: "called_by".into(),
            label: format!("called by `{caller}` at line {call_site_line}"),
            detail: None,
            target: Some(caller.clone()),
            file: None,
            line: Some(*call_site_line),
        },
        Reason::Resolution { kind } => ViewReason {
            kind: "resolution".into(),
            label: format!("resolution {kind}"),
            detail: Some(kind.clone()),
            target: None,
            file: None,
            line: None,
        },
        Reason::EnclosingFunction { function } => {
            let target = symbol_label(function);
            let (file, line) = symbol_file_line(function);
            ViewReason {
                kind: "enclosing_function".into(),
                label: format!("enclosing function {target}"),
                detail: None,
                target: Some(target),
                file: Some(file),
                line: Some(line),
            }
        }
        Reason::Containment { parent } => {
            let target = symbol_label(parent);
            let (file, line) = symbol_file_line(parent);
            ViewReason {
                kind: "containment".into(),
                label: format!("contained by {target}"),
                detail: None,
                target: Some(target),
                file: Some(file),
                line: Some(line),
            }
        }
        Reason::ResolvedImport {
            module,
            target_file,
        } => ViewReason {
            kind: "resolved_import".into(),
            label: format!("import `{module}` resolves to `{target_file}`"),
            detail: None,
            target: Some(module.clone()),
            file: Some(target_file.clone()),
            line: None,
        },
        Reason::UnresolvedImport { module } => ViewReason {
            kind: "unresolved_import".into(),
            label: format!("unresolved import `{module}`"),
            detail: None,
            target: Some(module.clone()),
            file: None,
            line: None,
        },
        Reason::Reasoning(ReasoningReason::TaintedBy {
            source,
            sanitizers_present_in_source_fn,
            path_proven,
        }) => {
            let target = symbol_label(source);
            let (file, line) = symbol_file_line(source);
            ViewReason {
                kind: "reasoning".into(),
                label: format!("tainted by {target}"),
                detail: Some(format!(
                    "path_proven={} sanitizers={}",
                    path_proven,
                    sanitizers_present_in_source_fn.len()
                )),
                target: Some(target),
                file: Some(file),
                line: Some(line),
            }
        }
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
        Reason::Reasoning(_) => view_reason(reason).label,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn function_symbol(name: &str, file: &str, start_line: usize) -> SymbolRef {
        SymbolRef::Function {
            file: file.into(),
            name: name.into(),
            start_line,
            end_line: start_line + 1,
            start_byte: 10,
            end_byte: 20,
            ordinal: 0,
        }
    }

    fn statement_symbol(kind: &str, file: &str, line: usize) -> SymbolRef {
        SymbolRef::Statement {
            file: file.into(),
            line,
            kind: kind.into(),
            start_byte: 30,
            end_byte: 40,
            ordinal: 0,
        }
    }

    #[test]
    fn view_reason_maps_reasoning_without_debug_output() {
        let reason = Reason::Reasoning(ReasoningReason::TaintedBy {
            source: function_symbol("source", "a.py", 3),
            sanitizers_present_in_source_fn: vec!["clean".into(), "escape".into()],
            path_proven: true,
        });

        let mapped = view_reason(&reason);
        assert_eq!(mapped.kind, "reasoning");
        assert!(mapped.label.contains("tainted by function source @ a.py:3"));
        assert!(!mapped.label.contains("TaintedBy"));
        assert_eq!(mapped.file.as_deref(), Some("a.py"));
        assert_eq!(mapped.line, Some(3));
        assert_eq!(
            mapped.detail.as_deref(),
            Some("path_proven=true sanitizers=2")
        );
    }

    #[test]
    fn view_reason_extracts_symbol_location_fields() {
        let enclosing = view_reason(&Reason::EnclosingFunction {
            function: function_symbol("outer", "a.py", 11),
        });
        assert_eq!(enclosing.kind, "enclosing_function");
        assert_eq!(
            enclosing.label,
            "enclosing function function outer @ a.py:11"
        );
        assert_eq!(enclosing.file.as_deref(), Some("a.py"));
        assert_eq!(enclosing.line, Some(11));

        let containment = view_reason(&Reason::Containment {
            parent: statement_symbol("if_statement", "b.py", 7),
        });
        assert_eq!(containment.kind, "containment");
        assert_eq!(
            containment.label,
            "contained by statement if_statement @ b.py:7"
        );
        assert_eq!(containment.file.as_deref(), Some("b.py"));
        assert_eq!(containment.line, Some(7));
    }

    #[test]
    fn call_site_source_location_is_line_only() {
        let location = call_site_source_location("main.py", 4);
        assert_eq!(location.kind, "call_site");
        assert_eq!(location.display, "main.py:4");
        assert_eq!(location.start_byte, None);
        assert_eq!(location.end_byte, None);
    }
}

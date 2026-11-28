#[cfg(test)]
use super::super::props_s2_test_support::{LiteralAttempt, LiteralWitness, Observer};
use super::{
    jsx_props::{
        jsx_plain_read, FormalReadContext, PlainRead, ReachedRead, ReadRefusal, SourceRefusal,
    },
    object_literal_args::is_transparent_wrapper,
    *,
};
use crate::ast::js_ts::component_route::SiteKind;
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct Counters {
    counts: BTreeMap<String, usize>,
    pairs: BTreeSet<(NodeIndex, NodeIndex)>,
    containers: BTreeSet<(NodeIndex, NodeIndex)>,
    #[cfg(test)]
    pub(super) attempts: Vec<LiteralAttempt>,
    #[cfg(test)]
    witnesses: Vec<LiteralWitness>,
}

impl Counters {
    pub(super) fn add(&mut self, name: &str, count: usize) {
        if count != 0 {
            *self.counts.entry(name.to_owned()).or_default() += count;
        }
    }

    pub(super) fn edge(
        &mut self,
        pair: (NodeIndex, NodeIndex),
        class: &str,
        container: bool,
    ) -> bool {
        if container && self.containers.insert(pair) {
            self.add("edges_to_container", 1);
        }
        let fresh = self.pairs.insert(pair);
        if fresh {
            self.add(class, 1);
        }
        fresh
    }
}

#[derive(Default)]
pub(super) struct Step5bOutput {
    pub(super) flows: Vec<PendingArgumentFlow>,
    pub(super) props: Counters,
}

impl Step5bOutput {
    pub(super) fn merge(&mut self, caller: Self) {
        self.flows.extend(caller.flows);
        #[cfg(test)]
        {
            self.props.attempts.extend(caller.props.attempts);
            self.props.witnesses.extend(caller.props.witnesses);
        }
        for (name, count) in caller.props.counts {
            self.props.add(&name, count);
        }
    }

    pub(super) fn into_flows(
        self,
        stats: &mut DfgLabelStats,
        #[cfg(test)] observer: Option<&Observer>,
    ) -> Vec<PendingArgumentFlow> {
        #[cfg(test)]
        if let Some(observer) = observer {
            observer.literal_proofs(self.props.attempts, self.props.witnesses);
        }
        stats.dfg_props = self.props.counts;
        self.flows
    }
}

pub(in crate::cpg) fn literal_site_refusal(
    failures: &[(usize, bool, u8, &'static str)],
) -> Option<&'static str> {
    failures
        .iter()
        .filter(|(_, safe, _, _)| *safe)
        .min_by_key(|(position, _, check, _)| (*check, *position))
        .map(|(_, _, _, reason)| *reason)
}

#[cfg(test)]
pub(in crate::cpg) fn literal_pattern_pairs(
    witnesses: &[(NodeIndex, NodeIndex, bool)],
) -> (Vec<(NodeIndex, NodeIndex)>, BTreeMap<String, usize>) {
    let mut stats = Counters::default();
    let edges = witnesses
        .iter()
        .filter_map(|&(from, to, container)| {
            stats
                .edge((from, to), "edges_to_def", container)
                .then_some((from, to))
        })
        .collect();
    (edges, stats.counts)
}

/// Entries retain source order and legacy visibility at each object level.
pub(super) enum PropsEntry {
    Named(PropsWriter),
    NumericWildcard,
    Wildcard,
    Spread,
}

pub(super) struct PropsWriter {
    pub(super) key: String,
    pub(super) legacy: bool,
    pub(super) value: PropsValue,
}

pub(super) enum PropsValue {
    Operand {
        span: std::ops::Range<usize>,
        path: Option<AccessPath>,
    },
    Object(Vec<PropsEntry>),
    Opaque,
    Numeric,
}

#[derive(Default)]
struct Projection {
    leaves: Vec<Leaf>,
    terminals: usize,
    opaque: usize,
}

impl Projection {
    fn append(&mut self, mut value: Self) {
        self.leaves.append(&mut value.leaves);
        self.terminals += value.terminals;
        self.opaque += value.opaque;
    }
}

pub(super) fn rich_leaves(entries: &[PropsEntry], counters: &mut Counters) -> Vec<Leaf> {
    let projection = fold_writers(entries, &mut Vec::new(), Some(counters));
    counters.add("refused_value_not_data", projection.opaque);
    projection.leaves
}

pub(super) fn legacy_leaves(entries: &[PropsEntry]) -> Vec<Leaf> {
    fold_writers(entries, &mut Vec::new(), None).leaves
}

fn fold_writers(
    entries: &[PropsEntry],
    prefix: &mut Vec<String>,
    mut counters: Option<&mut Counters>,
) -> Projection {
    let rich = counters.is_some();
    let mut last = BTreeMap::<&str, usize>::new();
    let mut values = BTreeMap::<usize, Projection>::new();
    let mut displaced = 0;
    for (position, entry) in entries.iter().enumerate() {
        match entry {
            PropsEntry::Named(writer) if rich || writer.legacy => {
                prefix.push(writer.key.clone());
                let value = match &writer.value {
                    PropsValue::Operand { span, path } if !rich || path.is_some() => Projection {
                        leaves: vec![Leaf {
                            keys: prefix.clone(),
                            span: span.clone(),
                            path: if rich { path.clone() } else { None },
                        }],
                        terminals: 1,
                        opaque: 0,
                    },
                    PropsValue::Object(entries) => {
                        fold_writers(entries, prefix, counters.as_deref_mut())
                    }
                    // A numeric-key writer supplies no leaf, and no static read
                    // names its key, so a surviving terminal counts as opaque.
                    PropsValue::Numeric | PropsValue::Opaque | PropsValue::Operand { .. } => {
                        Projection {
                            terminals: 1,
                            opaque: 1,
                            ..Projection::default()
                        }
                    }
                };
                prefix.pop();
                if let Some(prior) = last.insert(&writer.key, position) {
                    displaced += values.remove(&prior).expect("live writer").terminals;
                }
                values.insert(position, value);
            }
            PropsEntry::Wildcard | PropsEntry::Spread => {
                if matches!(entry, PropsEntry::Spread) {
                    if let Some(counters) = counters.as_deref_mut() {
                        counters.add("spread_sources_unbound", 1);
                    }
                }
                displaced += values.values().map(|value| value.terminals).sum::<usize>();
                last.clear();
                values.clear();
            }
            PropsEntry::NumericWildcard if rich => {
                last.retain(|key, position| {
                    let masks = key.starts_with(|c: char| c.is_ascii_digit()) || *key == "Infinity";
                    if masks {
                        displaced += values.remove(position).expect("live writer").terminals;
                    }
                    !masks
                });
            }
            _ => {}
        }
    }
    if let Some(counters) = counters {
        counters.add("refused_overwritten", displaced);
    }
    let mut projection = Projection::default();
    for value in values.into_values() {
        projection.append(value);
    }
    projection
}

/// A data-carrying leaf of an object literal: its static key path and value span.
pub(super) struct Leaf {
    pub(super) keys: Vec<String>,
    pub(super) span: std::ops::Range<usize>,
    pub(super) path: Option<AccessPath>,
}

/// The static member chain read through the identifier at `[start, end)`: `p.a.b` -> `[a, b]`,
/// bare or computed uses -> `[]`. `None` when the span is not that identifier.
fn read_chain(parsed: &ParsedFile, start: usize, end: usize) -> Option<Vec<String>> {
    let ident = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)?;
    if ident.kind() != "identifier" || ident.start_byte() != start || ident.end_byte() != end {
        return None;
    }
    let mut chain = Vec::new();
    let mut current = ident;
    while let Some(parent) = current.parent() {
        if is_transparent_wrapper(parent.kind()) {
            if first_expression(parent).map(|n| n.id()) == Some(current.id()) {
                current = parent;
                continue;
            }
            break;
        }
        if parent.kind() == "member_expression"
            && parent.child_by_field_name("object").map(|n| n.id()) == Some(current.id())
        {
            let property = parent.child_by_field_name("property")?;
            if property.kind() != "property_identifier" {
                break;
            }
            chain.push(parsed.node_text(&property).to_string());
            current = parent;
            continue;
        }
        break;
    }
    // A write target is not special-cased here: its Use is refused through the callee's
    // member writes (`FormalInfo`), which classify with the member-only lane's own rules.
    Some(chain)
}

/// Prove the actual selected Use, including base-only receiver Uses.
fn selected_read(
    parsed: &ParsedFile,
    owner: Node<'_>,
    path: &AccessPath,
    start: usize,
    end: usize,
) -> Option<bool> {
    if start >= end {
        return None;
    }
    let mut node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)?;
    if node.byte_range() != (start..end) || node.has_error() {
        return None;
    }
    let mut member = false;
    loop {
        node = crate::ast::js_ts::props_keys::strip_transparent_wrappers(node)?;
        if !matches!(node.kind(), "member_expression" | "subscript_expression") {
            break;
        }
        member = true;
        node = node.child_by_field_name("object")?;
    }
    if node.kind() != "identifier"
        || parsed.node_text(&node) != path.base
        || parsed.js_ts_identifier_is_not_a_read(node)
        || !parsed.js_ts_occurrence_binds_formal(&owner, &path.base, node.start_byte())
    {
        return None;
    }
    while let Some(parent) = node.parent() {
        if matches!(parent.kind(), "member_expression" | "subscript_expression")
            && parent.child_by_field_name("object") == Some(node)
        {
            member = true;
        } else if !is_transparent_wrapper(parent.kind()) || first_expression(parent) != Some(node) {
            break;
        }
        node = parent;
    }
    Some(!member)
}

/// A zero-width representative can come from a line that only writes the
/// formal's member. Such a line has no read that the width check could refuse.
fn only_member_writes_on_line(
    parsed: &ParsedFile,
    owner: Node<'_>,
    name: &str,
    line: usize,
) -> bool {
    let mut pending = vec![owner];
    let mut write = false;
    while let Some(node) = pending.pop() {
        let (lo, hi) = parsed.node_line_range(&node);
        if !(lo..=hi).contains(&line) || node.is_extra() {
            continue;
        }
        if node.kind() == "identifier" || node.kind() == "shorthand_property_identifier" {
            let text = parsed.node_text(&node);
            if text.contains('\\') {
                return false;
            }
            if text == name {
                let mut current = node;
                while let Some(parent) = current.parent().filter(|parent| {
                    matches!(parent.kind(), "member_expression" | "subscript_expression")
                        && parent.child_by_field_name("object") == Some(current)
                }) {
                    current = parent;
                }
                if current == node
                    || !current.parent().is_some_and(|parent| {
                        parent.kind() == "assignment_expression"
                            && parent.child_by_field_name("left") == Some(current)
                    })
                    || !parsed.js_ts_occurrence_binds_formal(&owner, name, node.start_byte())
                {
                    return false;
                }
                write = true;
            }
        }
        pending.extend(node.named_children(&mut node.walk()));
    }
    write
}

#[derive(Default)]
pub(super) struct SiteReadRefusals {
    zero_width: bool,
    unproved: BTreeSet<NodeIndex>,
    bracket: BTreeSet<NodeIndex>,
}

impl SiteReadRefusals {
    pub(super) fn record(self, counters: &mut Counters) {
        counters.add("refused_zero_width_read", usize::from(self.zero_width));
        counters.add("refused_unproved_read", self.unproved.len());
        counters.add("refused_bracket_read", self.bracket.len());
    }
}

/// The expression a transparent wrapper wraps, one level.
pub(super) fn first_expression(wrapper: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = wrapper.walk();
    let mut inner = wrapper
        .named_children(&mut cursor)
        .filter(|c| !c.is_extra());
    if wrapper.kind() == "type_assertion" {
        inner.last()
    } else {
        inner.next()
    }
}

/// A leaf and a read overlap when one key chain is a prefix of the other.
fn comparable(leaf: &[String], read: &[String]) -> bool {
    let n = leaf.len().min(read.len());
    leaf[..n] == read[..n]
}

/// One read of the formal that its definition node reaches.
struct Read {
    node: NodeIndex,
    chain: Vec<String>,
    bare: bool,
    /// The definition edge's own label.
    label: FlowConfidence,
}

/// The reads of a formal and, for a mixed formal, the callee's member writes to it.
struct FormalInfo {
    reads: Vec<Read>,
    base: String,
    /// JSX always checks writes. The literal lane checks them for mixed formals.
    facts: Option<crate::ast::FormalMemberFacts>,
}

impl FormalInfo {
    /// The read is not fed: its path is written, has a written prefix or a deeper write.
    fn read_suppressed(&self, read: &Read) -> bool {
        self.facts.as_ref().is_some_and(|facts| {
            facts.is_killed(&AccessPath::with_fields(
                self.base.clone(),
                read.chain.clone(),
            ))
        })
    }

    /// The leaf's key was replaced by a write at it or above it.
    fn leaf_overwritten(&self, leaf: &Leaf) -> bool {
        self.facts
            .as_ref()
            .is_some_and(|facts| facts.overwrites_at_or_above(&self.base, &leaf.keys))
    }
}

/// Reads of the formal that its definition node reaches.
fn formal_info(
    graph: &DiGraph<CpgNode, CpgEdge>,
    callee_parsed: &ParsedFile,
    def: NodeIndex,
    refusals: &mut SiteReadRefusals,
    kind: SiteKind,
) -> Option<FormalInfo> {
    let CpgNode::Variable {
        path: formal,
        access: VarAccess::Def,
        start_byte: def_start,
        end_byte: def_end,
        ..
    } = &graph[def]
    else {
        return None;
    };
    let mut owner = callee_parsed
        .tree
        .root_node()
        .descendant_for_byte_range(*def_start, *def_end)?;
    let callable_kinds = callee_parsed.language.callable_boundary_node_types();
    while !callable_kinds.contains(&owner.kind()) {
        owner = owner.parent()?;
    }
    let mut reads: BTreeMap<NodeIndex, Read> = BTreeMap::new();
    let mut mixed = false;
    for edge in graph.edges(def) {
        let CpgEdge::DataFlow(label) = *edge.weight() else {
            continue;
        };
        let CpgNode::Variable {
            path,
            line,
            access: VarAccess::Use,
            start_byte,
            end_byte,
            ..
        } = &graph[edge.target()]
        else {
            continue;
        };
        if path.base != formal.base {
            continue;
        }
        mixed |= path.fields.is_empty();
        let (bare, chain, label) = if kind == SiteKind::Jsx {
            let endpoint = ReachedRead {
                node: edge.target(),
                path,
                line: *line,
                range: *start_byte..*end_byte,
                label,
            };
            match jsx_plain_read(callee_parsed, &endpoint, FormalReadContext { owner }) {
                Ok(PlainRead::Whole) => (true, Vec::new(), endpoint.label),
                Ok(PlainRead::Dot(chain)) => (false, chain, endpoint.label),
                Err(reason) => {
                    match reason {
                        ReadRefusal::ZeroWidth => refusals.zero_width = true,
                        ReadRefusal::Unproved => {
                            refusals.unproved.insert(endpoint.node);
                        }
                        ReadRefusal::Bracket => {
                            refusals.bracket.insert(endpoint.node);
                        }
                    }
                    continue;
                }
            }
        } else {
            if path.fields.is_empty() && start_byte == end_byte {
                refusals.zero_width |=
                    !only_member_writes_on_line(callee_parsed, owner, &formal.base, *line);
                continue;
            }
            let Some(bare) = selected_read(callee_parsed, owner, path, *start_byte, *end_byte)
            else {
                refusals.unproved.insert(edge.target());
                continue;
            };
            let chain = if path.fields.is_empty() {
                match read_chain(callee_parsed, *start_byte, *end_byte) {
                    Some(chain) => chain,
                    None => continue,
                }
            } else {
                path.fields.clone()
            };
            (bare, chain, label)
        };
        reads
            .entry(edge.target())
            .and_modify(|read| read.label = read.label.worst(label))
            .or_insert(Read {
                node: edge.target(),
                bare,
                chain,
                label,
            });
    }
    // The callee's member writes come from the same walk the member-only lane uses; fail
    // closed without them. A mixed formal's reads are not field-aware in the DFG, so those
    // facts also drive the per-read suppression; a member-only formal's DFG already has it.
    let all_facts = callee_parsed.formal_member_facts_at(*def_start, *def_end, &formal.base)?;
    // A computed write (`p[k] = v`) may replace any key: a wildcard writer, fail closed.
    if all_facts.has_computed_write() {
        return None;
    }
    let facts = (mixed || kind == SiteKind::Jsx).then_some(all_facts);
    Some(FormalInfo {
        reads: reads.into_values().collect(),
        base: formal.base.clone(),
        facts,
    })
}

pub(super) enum PropsTarget<'a, T> {
    Plain { def: NodeIndex, bare_literal: bool },
    Pattern(&'a callee_params::PatternPosition<T>),
}

pub(super) struct BindingSite<'a> {
    pub(super) kind: SiteKind,
    #[cfg(test)]
    pub(super) bytes: (usize, usize),
    pub(super) graph: &'a DiGraph<CpgNode, CpgEdge>,
    pub(super) callee: &'a ParsedFile,
    pub(super) cross_file_free_single_exact: bool,
}

pub(super) fn bind<T>(
    output: &mut Step5bOutput,
    refusals: &mut SiteReadRefusals,
    site: &BindingSite<'_>,
    target: PropsTarget<'_, T>,
    leaves: &[Leaf],
    rich: &[Leaf],
    mut source: impl FnMut(&Leaf) -> Result<Vec<NodeIndex>, SourceRefusal>,
    resolve: impl Fn(&T) -> Option<NodeIndex>,
) {
    match target {
        PropsTarget::Plain { def, bare_literal } => {
            let info = formal_info(site.graph, site.callee, def, refusals, site.kind);
            let mut matched = BTreeSet::new();
            for leaf in leaves {
                let Some(info) = info.as_ref().filter(|info| !info.leaf_overwritten(leaf)) else {
                    continue;
                };
                let mut reads = info
                    .reads
                    .iter()
                    .filter(|read| {
                        comparable(&leaf.keys, &read.chain) && !info.read_suppressed(read)
                    })
                    .peekable();
                if reads.peek().is_none() {
                    continue;
                }
                matched.insert((&leaf.keys, leaf.span.start, leaf.span.end));
                let sources = match source(leaf) {
                    Ok(sources) => sources,
                    Err(reason) => {
                        output.props.add(reason.counter(), 1);
                        continue;
                    }
                };
                for read in reads {
                    let class = if read.bare {
                        "edges_to_bare_use"
                    } else {
                        "edges_to_use"
                    };
                    for &from in &sources {
                        emit(
                            output,
                            site,
                            leaf,
                            (from, read.node),
                            class,
                            false,
                            FlowConfidence::Exact.worst(read.label),
                            bare_literal,
                        );
                    }
                }
            }
            let unmatched = rich
                .iter()
                .filter(|leaf| !matched.contains(&(&leaf.keys, leaf.span.start, leaf.span.end)))
                .count();
            output.props.add("refused_key_not_read", unmatched);
        }
        PropsTarget::Pattern(position) => {
            bind_pattern(output, site, position, leaves, source, resolve);
        }
    }
}

fn bind_pattern<T>(
    output: &mut Step5bOutput,
    site: &BindingSite<'_>,
    position: &callee_params::PatternPosition<T>,
    leaves: &[Leaf],
    mut source: impl FnMut(&Leaf) -> Result<Vec<NodeIndex>, SourceRefusal>,
    resolve: impl Fn(&T) -> Option<NodeIndex>,
) {
    let syntax = &position.syntax;
    let mut unknown = syntax
        .levels
        .iter()
        .map(|level| level.unknown_entries)
        .sum();
    let destinations: Vec<_> = syntax
        .elements
        .iter()
        .zip(&position.targets)
        .filter_map(|(entry, target)| {
            let path = entry.path.as_ref()?;
            let rest = entry.rest_level.and_then(|level| syntax.levels.get(level));
            if rest.is_some_and(|level| level.unknown_sibling) {
                unknown += 1;
                return None;
            }
            Some((path, rest, resolve(target.as_ref()?)?))
        })
        .collect();
    output.props.add("refused_key_unknown", unknown);
    for leaf in leaves {
        let mut matching = destinations
            .iter()
            .copied()
            .filter(|&(path, rest, _)| {
                rest.map_or_else(
                    || comparable(&leaf.keys, path),
                    |level| level.accepts_rest_leaf(&leaf.keys),
                )
            })
            .peekable();
        if matching.peek().is_none() {
            output.props.add("refused_key_not_read", 1);
            continue;
        }
        let sources = match source(leaf) {
            Ok(sources) => sources,
            Err(reason) => {
                output.props.add(reason.counter(), 1);
                continue;
            }
        };
        for (path, rest, to) in matching {
            let class = if rest.is_some() {
                "edges_to_rest"
            } else {
                "edges_to_def"
            };
            let container = rest.is_none() && leaf.keys.len() > path.len();
            for &from in &sources {
                emit(
                    output,
                    site,
                    leaf,
                    (from, to),
                    class,
                    container,
                    FlowConfidence::Exact,
                    false,
                );
            }
        }
    }
}

fn emit(
    output: &mut Step5bOutput,
    site: &BindingSite<'_>,
    _leaf: &Leaf,
    pair: (NodeIndex, NodeIndex),
    class: &'static str,
    container: bool,
    label: FlowConfidence,
    bare_literal: bool,
) {
    #[cfg(test)]
    output.props.witnesses.push(LiteralWitness {
        kind: site.kind,
        site: site.bytes,
        operand: _leaf.span.clone(),
        keys: _leaf.keys.clone(),
        endpoints: pair,
        class,
        container,
    });
    if output.props.edge(pair, class, container) {
        output.flows.push(PendingArgumentFlow {
            edge: (pair.0, pair.1, CpgEdge::DataFlow(label)),
            cross_file_free_single_exact: site.cross_file_free_single_exact,
            bare_literal,
        });
    }
}

use super::{
    object_literal_args::{is_transparent_wrapper, SiteInputs},
    *,
};
use tree_sitter::Node;

/// A data-carrying leaf of an object literal: its static key path and value span.
pub(super) struct Leaf {
    pub(super) keys: Vec<String>,
    pub(super) span: std::ops::Range<usize>,
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

/// The expression a transparent wrapper wraps, one level.
fn first_expression(wrapper: Node<'_>) -> Option<Node<'_>> {
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
    /// The definition edge's own label.
    label: FlowConfidence,
}

/// The reads of a formal and, for a mixed formal, the callee's member writes to it.
struct FormalInfo {
    reads: Vec<Read>,
    base: String,
    /// Present exactly when the reads are base `p` Uses (a mixed formal).
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
    let mut reads: BTreeMap<NodeIndex, Read> = BTreeMap::new();
    let mut mixed = false;
    for edge in graph.edges(def) {
        let CpgEdge::DataFlow(label) = *edge.weight() else {
            continue;
        };
        let CpgNode::Variable {
            path,
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
        let chain = if path.fields.is_empty() {
            mixed = true;
            match read_chain(callee_parsed, *start_byte, *end_byte) {
                Some(chain) => chain,
                None => continue,
            }
        } else {
            path.fields.clone()
        };
        reads
            .entry(edge.target())
            .and_modify(|read| read.label = read.label.worst(label))
            .or_insert(Read {
                node: edge.target(),
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
    let facts = mixed.then_some(all_facts);
    Some(FormalInfo {
        reads: reads.into_values().collect(),
        base: formal.base.clone(),
        facts,
    })
}

pub(super) fn bind(
    out: &mut Vec<PendingArgumentFlow>,
    seen: &mut BTreeSet<(NodeIndex, NodeIndex)>,
    site: &SiteInputs<'_>,
    def: NodeIndex,
    leaves: &[Leaf],
    bare_literal: bool,
) {
    let Some(info) = formal_info(site.graph, site.callee_parsed, def) else {
        return;
    };
    for leaf in leaves {
        if info.leaf_overwritten(leaf) {
            continue;
        }
        let value_text = site
            .caller_parsed
            .source
            .get(leaf.span.clone())
            .unwrap_or("")
            .trim();
        // Same lookup as a plain argument: the field path, then the base.
        let full = site
            .caller_parsed
            .bounded_argument_member_path_at(leaf.span.start, leaf.span.end)
            .unwrap_or_else(|| AccessPath::from_expr(value_text));
        let mut paths = vec![full.clone()];
        let base = AccessPath::simple(&full.base);
        if base != full {
            paths.push(base);
        }
        let sources: Vec<NodeIndex> = paths
            .into_iter()
            .filter_map(|path| {
                CodePropertyGraph::argument_var_node_in_span_with_exact(
                    site.caller_id,
                    site.caller_parsed,
                    &path,
                    &leaf.span,
                    site.var_index,
                    site.exact_var_index,
                    site.graph,
                )
            })
            .collect();
        for read in &info.reads {
            if !comparable(&leaf.keys, &read.chain) || info.read_suppressed(read) {
                continue;
            }
            for &from in &sources {
                if seen.insert((from, read.node)) {
                    out.push(PendingArgumentFlow {
                        edge: (
                            from,
                            read.node,
                            CpgEdge::DataFlow(FlowConfidence::Exact.worst(read.label)),
                        ),
                        cross_file_free_single_exact: site.cross_file_free_single_exact,
                        bare_literal,
                    });
                }
            }
        }
    }
}

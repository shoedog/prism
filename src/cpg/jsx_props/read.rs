use super::*;

pub(in crate::cpg::build) struct ReachedRead<'a> {
    pub(in crate::cpg::build) node: NodeIndex,
    pub(in crate::cpg::build) path: &'a AccessPath,
    pub(in crate::cpg::build) line: usize,
    pub(in crate::cpg::build) range: Range<usize>,
    pub(in crate::cpg::build) label: FlowConfidence,
}

pub(in crate::cpg::build) struct FormalReadContext<'a> {
    pub(in crate::cpg::build) owner: Node<'a>,
}

pub(in crate::cpg::build) enum PlainRead {
    Whole,
    Dot(Vec<String>),
}

pub(in crate::cpg::build) enum ReadRefusal {
    ZeroWidth,
    Bracket,
    Unproved,
}

pub(in crate::cpg::build) fn jsx_plain_read(
    parsed: &ParsedFile,
    endpoint: &ReachedRead<'_>,
    formal: FormalReadContext<'_>,
) -> Result<PlainRead, ReadRefusal> {
    if endpoint.range.is_empty() && endpoint.path.fields.is_empty() {
        return Err(ReadRefusal::ZeroWidth);
    }
    let prove = |root: Node<'_>| {
        if !syntax::is_value_read(parsed, root)
            || parsed.node_text(&root) != endpoint.path.base
            || !parsed.js_ts_occurrence_binds_formal(
                &formal.owner,
                &endpoint.path.base,
                root.start_byte(),
            )
        {
            return Err(ReadRefusal::Unproved);
        }
        let (_, fields, bracket) = syntax::receiver_chain(parsed, root);
        if bracket {
            return Err(ReadRefusal::Bracket);
        }
        if !fields.starts_with(&endpoint.path.fields) {
            return Err(ReadRefusal::Unproved);
        }
        Ok(if fields.is_empty() {
            PlainRead::Whole
        } else {
            PlainRead::Dot(fields)
        })
    };
    if let Some(root) = syntax::exact_node(parsed, endpoint.range.clone())
        .filter(|node| !node.has_error())
        .and_then(syntax::operand_root)
    {
        return prove(root);
    }
    // A projected anchor must select one syntactic receiver on its recorded line.
    if endpoint.path.fields.is_empty() {
        return Err(ReadRefusal::Unproved);
    }
    let mut selected = None;
    syntax::walk(vec![formal.owner], |node| {
        let (start, end) = parsed.node_line_range(&node);
        if !(start..=end).contains(&endpoint.line) {
            return false;
        }
        if start == endpoint.line
            && syntax::is_value_read(parsed, node)
            && parsed.node_text(&node) == endpoint.path.base
            && syntax::receiver_chain(parsed, node)
                .1
                .starts_with(&endpoint.path.fields)
        {
            selected = Some(if selected.is_some() {
                Err(ReadRefusal::Unproved)
            } else {
                prove(node)
            });
        }
        true
    });
    selected.unwrap_or(Err(ReadRefusal::Unproved))
}

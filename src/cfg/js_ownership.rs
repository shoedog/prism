//! JS/TS/TSX CFG edge ownership records (#693, #697; option B, slice 1).
//!
//! OBSERVATIONAL. While a JS-family owner's CFG is built, every edge the builders push also gets one
//! record: which builder emitted it (`Producer`), what it means (`Role`), the syntax it was drawn from
//! and to (`Anchor`s: byte ranges and node kinds), the construct whose completion it depends on, and
//! whether that ownership is known (`Own`). The records change no edge, flag, order or label: they are
//! kept in lockstep with `EdgeSink::edges` (`live[i]` describes `edges[i]`), the edges the loop
//! rewrite drops move to `dropped`, and a loop edge the rewrite skipped as already present moves to
//! `suppressed`. They are transient (never cached or serialized) and are read only by
//! `js_disposition` (the per-loop-tree inventory, the test helper and `PRISM_CFG_OWNERSHIP_TRACE=1`).
//!
//! Full records come from the emitters that can touch a loop tree's lines, its enclosing arm's
//! completion or a line shared with a loop boundary: the sequential builder (statement identity is
//! resolved later from the pre-dedup universe), the `if` branch / then-end builder, the loop back and
//! exit builder and `js_loops`. Every other emitter (`try`, switch fall-through) is recorded with role
//! `Legacy` and ownership `Unknown(LegacyEmitter)`.
use super::CfgEdge;
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// A syntax node as the records name it: a half-open byte range and the node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: &'static str,
}

impl Anchor {
    pub(crate) fn of(node: Node<'_>) -> Self {
        Self {
            start: node.start_byte(),
            end: node.end_byte(),
            kind: node.kind(),
        }
    }

    pub(crate) fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    pub(crate) fn same_range(self, other: Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

/// The builder that emitted an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Producer {
    /// `build_function_cfg`'s fall-through loop over the sorted, line-deduplicated universe.
    Sequential,
    /// An `if` condition's edge to an arm or past the `if`.
    BranchEntry,
    /// The end of an `if`'s consequence to the statement after the `if`.
    ThenEnd,
    /// `collect_loop_back_edges`: the end of a body to the header.
    LoopBack,
    /// `collect_loop_back_edges`: the header past the loop.
    LoopExit,
    /// `js_loops`: the replacement edges of a rewired loop tree.
    JsLoops,
    /// `js_loops`: the edge of a `break` or `continue`.
    Jump,
    /// `build_try_catch_edges`.
    Try,
    /// Switch dispatch or fall-through.
    Switch,
    Other,
}

/// What an edge means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    BranchEntry,
    /// A statement or arm completes into what follows.
    NormalCompletion,
    /// A body completion reaches the header (or a `do`'s condition reaches its body).
    Repetition,
    LoopExit,
    Break,
    Continue,
    /// An emitter this slice does not model.
    Legacy,
}

/// Why an edge's ownership is not established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Doubt {
    /// The emitter is not modelled (`Role::Legacy`).
    LegacyEmitter,
    /// The edge leaves a compound statement's ENTRY line where a completion is meant (then-end).
    WrapperEntry,
    /// The construct the edge was drawn for sits inside a callable nested in the owner.
    NestedCallable,
    /// An endpoint line holds several statements (resolved from the pre-dedup universe).
    SharedLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Own {
    Known,
    Unknown(Doubt),
    Ambiguous(Doubt),
}

/// One edge's record. `from`/`to`/`depends_on` are what the emitter named (`None` when it worked from
/// lines only: the sequential builder, a "next statement" target); `from_kept`/`to_kept` are the
/// statements the line nodes keep after start-line deduplication (filled by `js_disposition`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EdgeRecord {
    /// Emission order within the owner's build; never reused, so it survives the rewrite's drops.
    pub(crate) id: usize,
    /// The owner callable's byte range.
    pub(crate) callable: (usize, usize),
    pub(crate) from_line: usize,
    pub(crate) to_line: usize,
    pub(crate) producer: Producer,
    pub(crate) role: Role,
    pub(crate) own: Own,
    pub(crate) from: Option<Anchor>,
    pub(crate) to: Option<Anchor>,
    pub(crate) depends_on: Option<Anchor>,
    pub(crate) from_kept: Option<(usize, usize)>,
    pub(crate) to_kept: Option<(usize, usize)>,
    /// For a `suppressed` record: the live record that already held the same line pair.
    pub(crate) duplicate_of: Option<usize>,
}

#[cfg(test)]
std::thread_local! {
    static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with no records built, to compare the edges with and without them (tests only).
#[cfg(test)]
pub(crate) fn without_records<T>(f: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            OFF.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(OFF.with(|c| c.replace(true)));
    f()
}

/// Whether records are built for a JS-family owner. Always, outside tests: slices 2 and 3 consume them.
pub(super) fn wanted() -> bool {
    #[cfg(test)]
    {
        !OFF.with(|c| c.get())
    }
    #[cfg(not(test))]
    {
        true
    }
}

/// What an emitter tells the recorder about one edge.
pub(crate) struct Emit<'t> {
    producer: Producer,
    role: Role,
    own: Own,
    /// The construct the edge belongs to: its callable decides `NestedCallable`.
    site: Node<'t>,
    from: Option<Node<'t>>,
    to: Option<Node<'t>>,
    depends_on: Option<Node<'t>>,
}

impl<'t> Emit<'t> {
    pub(crate) fn new(producer: Producer, role: Role, site: Node<'t>) -> Self {
        Self {
            producer,
            role,
            own: Own::Known,
            site,
            from: None,
            to: None,
            depends_on: None,
        }
    }

    pub(crate) fn from(mut self, node: Node<'t>) -> Self {
        self.from = Some(node);
        self
    }

    pub(crate) fn to(mut self, node: Option<Node<'t>>) -> Self {
        self.to = node;
        self
    }

    pub(crate) fn depends_on(mut self, node: Node<'t>) -> Self {
        self.depends_on = Some(node);
        self
    }

    pub(crate) fn own(mut self, own: Own) -> Self {
        self.own = own;
        self
    }
}

/// The records of one owner's build. `live` is index-aligned with `EdgeSink::edges`.
pub(crate) struct Recorder {
    pub(crate) callable: (usize, usize),
    owner_id: usize,
    boundaries: Vec<&'static str>,
    next_id: usize,
    /// The producer of an edge pushed without an `Emit` (a legacy emitter).
    phase: Producer,
    pub(crate) live: Vec<EdgeRecord>,
    pub(crate) dropped: Vec<EdgeRecord>,
    pub(crate) suppressed: Vec<EdgeRecord>,
}

impl Recorder {
    pub(crate) fn new(owner: Node<'_>, parsed: &ParsedFile) -> Self {
        Self {
            callable: (owner.start_byte(), owner.end_byte()),
            owner_id: owner.id(),
            boundaries: parsed.language.callable_boundary_node_types(),
            next_id: 0,
            phase: Producer::Other,
            live: Vec::new(),
            dropped: Vec::new(),
            suppressed: Vec::new(),
        }
    }

    pub(crate) fn set_phase(&mut self, producer: Producer) {
        self.phase = producer;
    }

    fn make(&mut self, edge: &CfgEdge, emit: Emit<'_>) -> EdgeRecord {
        let id = self.next_id;
        self.next_id += 1;
        let own = match emit.own {
            Own::Known if self.in_nested_callable(emit.site) => {
                Own::Ambiguous(Doubt::NestedCallable)
            }
            own => own,
        };
        EdgeRecord {
            id,
            callable: self.callable,
            from_line: edge.from_line,
            to_line: edge.to_line,
            producer: emit.producer,
            role: emit.role,
            own,
            from: emit.from.map(Anchor::of),
            to: emit.to.map(Anchor::of),
            depends_on: emit.depends_on.map(Anchor::of),
            from_kept: None,
            to_kept: None,
            duplicate_of: None,
        }
    }

    /// Whether `node` is inside a callable other than the owner.
    fn in_nested_callable(&self, node: Node<'_>) -> bool {
        let mut at = Some(node);
        while let Some(n) = at {
            if n.id() == self.owner_id {
                return false;
            }
            if self.boundaries.contains(&n.kind()) {
                return true;
            }
            at = n.parent();
        }
        false
    }

    pub(crate) fn push(&mut self, edge: &CfgEdge, emit: Emit<'_>) {
        let record = self.make(edge, emit);
        self.live.push(record);
    }

    /// A record from a builder without syntax anchors: the sequential loop, or a legacy emitter.
    pub(crate) fn push_unanchored(&mut self, edge: &CfgEdge, sequential: bool) {
        let (producer, role, own) = if sequential {
            (Producer::Sequential, Role::NormalCompletion, Own::Known)
        } else {
            (self.phase, Role::Legacy, Own::Unknown(Doubt::LegacyEmitter))
        };
        let id = self.next_id;
        self.next_id += 1;
        self.live.push(EdgeRecord {
            id,
            callable: self.callable,
            from_line: edge.from_line,
            to_line: edge.to_line,
            producer,
            role,
            own,
            from: None,
            to: None,
            depends_on: None,
            from_kept: None,
            to_kept: None,
            duplicate_of: None,
        });
    }

    /// An emission the rewrite skipped because its line pair is already an edge.
    pub(crate) fn suppress(&mut self, edge: &CfgEdge, emit: Emit<'_>) {
        let mut record = self.make(edge, emit);
        record.duplicate_of = self
            .live
            .iter()
            .find(|r| (r.from_line, r.to_line) == (edge.from_line, edge.to_line))
            .map(|r| r.id);
        self.suppressed.push(record);
    }

    /// Mirror `EdgeSink::retain`: `keep[i]` is the verdict on `live[i]`.
    pub(crate) fn retain(&mut self, keep: &[bool]) {
        let all = std::mem::take(&mut self.live);
        for (record, &k) in all.into_iter().zip(keep) {
            if k {
                self.live.push(record);
            } else {
                self.dropped.push(record);
            }
        }
    }
}

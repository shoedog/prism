//! Shared typed contract for Tier-2 reasoning evidence.

use crate::cpg::OrderingUnavailableReason;
use crate::data_flow::VarLocation;
use crate::navigation::types::{Location, SymbolRef};
use std::cmp::Ordering;

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub enum ReasoningReason {
    TaintedBy {
        source: SymbolRef,
        sanitizers_present_in_source_fn: Vec<String>,
        path_proven: bool,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeHonestyUnavailableReason {
    UnsupportedLanguage,
    MissingParsedFile,
    UnmappedTraceNode,
    UnmappedFunction,
    MissingCatalog,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub enum ReasoningWarning {
    SeedUnresolved {
        seed: String,
    },
    /// The field is named `sink` for the experimental-v1 wire shape, but the value is the boundary
    /// variable reached at an interprocedural crossing.
    InterproceduralBoundary {
        sink: String,
    },
    Cleansed {
        source_function: String,
    },
    UnmodeledConstruct {
        language: String,
        construct: String,
        function: String,
    },
    ScopeHonestyUnavailable {
        reason: ScopeHonestyUnavailableReason,
        file: String,
        function: Option<String>,
    },
    OrderingUnavailable {
        file: String,
        line: usize,
        path: String,
        reason: OrderingUnavailableReason,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize, PartialEq, Eq)]
pub enum Reachability {
    Reached,
    NotReached,
    BoundaryExited,
    /// P10: the raw trace connects source to sink (`Reached`), but the witness chain proves a
    /// contiguous transition through a recognized, non-`paired_check` sanitizer call — see
    /// `src/reasoning/sanitizer_walk.rs`. This is a DOWNGRADE computed after raw reachability, never
    /// a value `reachability_for_node_from_ordered`/`shape.rs` produce directly (source of truth:
    /// `taint_reaches.rs`'s witness_mode, after the chain walk and before the witness-graph gate).
    Sanitized,
}

impl Reachability {
    pub fn severity_rank(self) -> u8 {
        match self {
            Self::Reached => 0,
            Self::BoundaryExited => 1,
            Self::Sanitized => 2,
            Self::NotReached => 3,
        }
    }

    /// Priority order (most to least severe): `Reached` > `BoundaryExited` > `Sanitized` >
    /// `NotReached`. Spelled out per-variant (not a generic `min_by_key(severity_rank)`) so a future
    /// 5th variant is a non-exhaustive-match compile error here, not a silent aggregate mis-rank.
    pub fn aggregate<I>(values: I) -> Option<Self>
    where
        I: IntoIterator<Item = Self>,
    {
        let mut best = None;
        for value in values {
            best = Some(match (best, value) {
                (Some(Self::Reached), _) | (_, Self::Reached) => Self::Reached,
                (Some(Self::BoundaryExited), _) | (_, Self::BoundaryExited) => Self::BoundaryExited,
                (Some(Self::Sanitized), _) | (_, Self::Sanitized) => Self::Sanitized,
                (Some(Self::NotReached), Self::NotReached) | (None, Self::NotReached) => {
                    Self::NotReached
                }
            });
        }
        best
    }
}

/// P10's discriminating fact: a sanitizer call site the witness chain-window walk proved sits ON
/// the source->sink path (not just present somewhere in the source function's body — see
/// `src/reasoning/sanitizer_walk.rs`). `category` is the lowercase `SanitizerCategory` string (same
/// vocabulary as `sanitizers_present_in_source_fn`, e.g. `"xss"`).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SanitizerSite {
    pub category: String,
    pub callee_text: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SinkSourceResult {
    pub source: SymbolRef,
    pub reachability: Reachability,
    /// Index of this source's reached sink endpoint in the union witness graph. This is not the
    /// source node; `merge_witness` returns the witness path's terminal sink node after remapping.
    pub graph_node: Option<usize>,
    pub sanitizers_present_in_source_fn: Vec<String>,
    /// Path-proven sanitizer sites (P10) — populated ONLY when `reachability == Sanitized`.
    /// Additive/verdict-bearing: `compact_non_verdict_detail` must NOT clear this (unlike the
    /// advisory `sanitizers_present_in_source_fn`). Skip-if-empty keeps non-sanitized wire output
    /// byte-identical to pre-P10.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sanitized_by: Vec<SanitizerSite>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceWarningKey {
    pub(crate) file: String,
    pub(crate) function: String,
    pub(crate) line: usize,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

impl SourceWarningKey {
    pub(crate) fn from_var_location(loc: &VarLocation) -> Self {
        Self {
            file: loc.file.clone(),
            function: loc.function.clone(),
            line: loc.line,
            start_byte: loc.start_byte,
            end_byte: loc.end_byte,
        }
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn from_symbol(symbol: &SymbolRef) -> Option<Self> {
        match symbol {
            SymbolRef::Variable {
                file,
                function,
                line,
                start_byte,
                end_byte,
                ..
            } => Some(Self {
                file: file.clone(),
                function: function.clone(),
                line: *line,
                start_byte: *start_byte,
                end_byte: *end_byte,
            }),
            _ => None,
        }
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn from_warning(source_function: &str, location: &Location) -> Self {
        Self {
            file: location.file.clone(),
            function: source_function.into(),
            line: location.start_line,
            start_byte: location.start_byte,
            end_byte: location.end_byte,
        }
    }

    pub(crate) fn location(&self) -> Location {
        Location {
            file: self.file.clone(),
            start_line: self.line,
            end_line: self.line,
            start_byte: self.start_byte,
            end_byte: self.end_byte,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct SinkResult {
    pub sink: SymbolRef,
    pub reachability: Reachability,
    pub sources: Vec<SinkSourceResult>,
    pub sources_omitted: usize,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ReasoningSummary {
    /// Aggregate reachability across `per_sink`. `None` means frontier mode or an indeterminate
    /// aggregate after sink truncation; `per_sink` remains authoritative when not truncated.
    pub reachability: Option<Reachability>,
    pub per_sink: Vec<SinkResult>,
    pub source_count: usize,
    pub frontier_count: usize,
    pub sinks_omitted: usize,
}

impl ReasoningSummary {
    pub fn aggregate(per_sink: &[SinkResult], sinks_omitted: usize) -> Option<Reachability> {
        if sinks_omitted > 0 || per_sink.is_empty() {
            return None;
        }
        Reachability::aggregate(per_sink.iter().map(|sink| sink.reachability))
    }

    pub fn repair_after_clip(&mut self, kept_graph_nodes: usize) {
        for sink in &mut self.per_sink {
            for source in &mut sink.sources {
                if source.graph_node.is_some_and(|idx| idx >= kept_graph_nodes) {
                    source.graph_node = None;
                }
            }
        }
    }

    pub fn compact_non_verdict_detail(&mut self) {
        for sink in &mut self.per_sink {
            for source in &mut sink.sources {
                source.graph_node = None;
                source.sanitizers_present_in_source_fn.clear();
            }
        }
    }

    pub fn truncate_sinks(&mut self, retained: usize) {
        if retained >= self.per_sink.len() {
            return;
        }
        self.sinks_omitted += self.per_sink.len() - retained;
        self.per_sink.truncate(retained);
        self.reachability = None;
    }

    pub fn truncate_sources(&mut self, retained: usize) {
        for sink in &mut self.per_sink {
            if retained >= sink.sources.len() {
                continue;
            }
            sink.sources.sort_by(source_result_cmp);
            sink.sources_omitted += sink.sources.len() - retained;
            sink.sources.truncate(retained);
        }
    }
}

fn source_result_cmp(a: &SinkSourceResult, b: &SinkSourceResult) -> Ordering {
    a.reachability
        .severity_rank()
        .cmp(&b.reachability.severity_rank())
        .then_with(|| symbol_ref_cmp(&a.source, &b.source))
}

fn symbol_ref_cmp(a: &SymbolRef, b: &SymbolRef) -> Ordering {
    symbol_ref_rank(a)
        .cmp(&symbol_ref_rank(b))
        .then_with(|| match (a, b) {
            (
                SymbolRef::Variable {
                    file: a_file,
                    function: a_function,
                    line: a_line,
                    path: a_path,
                    access: a_access,
                    start_byte: a_start,
                    end_byte: a_end,
                    ordinal: a_ordinal,
                },
                SymbolRef::Variable {
                    file: b_file,
                    function: b_function,
                    line: b_line,
                    path: b_path,
                    access: b_access,
                    start_byte: b_start,
                    end_byte: b_end,
                    ordinal: b_ordinal,
                },
            ) => a_file
                .cmp(b_file)
                .then_with(|| a_function.cmp(b_function))
                .then_with(|| a_line.cmp(b_line))
                .then_with(|| a_start.cmp(b_start))
                .then_with(|| a_end.cmp(b_end))
                .then_with(|| a_path.cmp(b_path))
                .then_with(|| a_access.cmp(b_access))
                .then_with(|| a_ordinal.cmp(b_ordinal)),
            (
                SymbolRef::Function {
                    file: a_file,
                    name: a_name,
                    start_line: a_start_line,
                    end_line: a_end_line,
                    start_byte: a_start,
                    end_byte: a_end,
                    ordinal: a_ordinal,
                },
                SymbolRef::Function {
                    file: b_file,
                    name: b_name,
                    start_line: b_start_line,
                    end_line: b_end_line,
                    start_byte: b_start,
                    end_byte: b_end,
                    ordinal: b_ordinal,
                },
            ) => a_file
                .cmp(b_file)
                .then_with(|| a_name.cmp(b_name))
                .then_with(|| a_start_line.cmp(b_start_line))
                .then_with(|| a_end_line.cmp(b_end_line))
                .then_with(|| a_start.cmp(b_start))
                .then_with(|| a_end.cmp(b_end))
                .then_with(|| a_ordinal.cmp(b_ordinal)),
            (
                SymbolRef::Statement {
                    file: a_file,
                    line: a_line,
                    kind: a_kind,
                    start_byte: a_start,
                    end_byte: a_end,
                    ordinal: a_ordinal,
                },
                SymbolRef::Statement {
                    file: b_file,
                    line: b_line,
                    kind: b_kind,
                    start_byte: b_start,
                    end_byte: b_end,
                    ordinal: b_ordinal,
                },
            ) => a_file
                .cmp(b_file)
                .then_with(|| a_line.cmp(b_line))
                .then_with(|| a_start.cmp(b_start))
                .then_with(|| a_end.cmp(b_end))
                .then_with(|| a_kind.cmp(b_kind))
                .then_with(|| a_ordinal.cmp(b_ordinal)),
            _ => Ordering::Equal,
        })
}

fn symbol_ref_rank(symbol: &SymbolRef) -> u8 {
    match symbol {
        SymbolRef::Function { .. } => 0,
        SymbolRef::Statement { .. } => 1,
        SymbolRef::Variable { .. } => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(name: &str) -> SymbolRef {
        SymbolRef::Variable {
            file: "a.py".into(),
            function: "f".into(),
            line: 1,
            path: name.into(),
            access: "use".into(),
            start_byte: 0,
            end_byte: 0,
            ordinal: 0,
        }
    }

    #[test]
    fn aggregate_uses_reached_boundary_not_reached_order() {
        assert_eq!(
            Reachability::aggregate([Reachability::NotReached, Reachability::BoundaryExited]),
            Some(Reachability::BoundaryExited)
        );
        assert_eq!(
            Reachability::aggregate([Reachability::BoundaryExited, Reachability::Reached]),
            Some(Reachability::Reached)
        );
    }

    #[test]
    fn severity_rank_order_is_reached_boundary_sanitized_not_reached() {
        assert!(
            Reachability::Reached.severity_rank() < Reachability::BoundaryExited.severity_rank()
        );
        assert!(
            Reachability::BoundaryExited.severity_rank() < Reachability::Sanitized.severity_rank()
        );
        assert!(Reachability::Sanitized.severity_rank() < Reachability::NotReached.severity_rank());
    }

    #[test]
    fn aggregate_every_mix_with_sanitized() {
        use Reachability::*;
        // Reached + anything -> Reached.
        assert_eq!(Reachability::aggregate([Reached, Sanitized]), Some(Reached));
        assert_eq!(Reachability::aggregate([Sanitized, Reached]), Some(Reached));
        assert_eq!(
            Reachability::aggregate([Reached, BoundaryExited]),
            Some(Reached)
        );
        assert_eq!(
            Reachability::aggregate([Reached, NotReached]),
            Some(Reached)
        );
        assert_eq!(Reachability::aggregate([Reached, Reached]), Some(Reached));

        // Sanitized + BoundaryExited -> BoundaryExited (BoundaryExited outranks Sanitized).
        assert_eq!(
            Reachability::aggregate([Sanitized, BoundaryExited]),
            Some(BoundaryExited)
        );
        assert_eq!(
            Reachability::aggregate([BoundaryExited, Sanitized]),
            Some(BoundaryExited)
        );

        // Sanitized + NotReached -> Sanitized (Sanitized outranks NotReached).
        assert_eq!(
            Reachability::aggregate([Sanitized, NotReached]),
            Some(Sanitized)
        );
        assert_eq!(
            Reachability::aggregate([NotReached, Sanitized]),
            Some(Sanitized)
        );

        // all-Sanitized -> Sanitized.
        assert_eq!(
            Reachability::aggregate([Sanitized, Sanitized, Sanitized]),
            Some(Sanitized)
        );

        // all-NotReached -> NotReached (baseline, unaffected by the new variant).
        assert_eq!(
            Reachability::aggregate([NotReached, NotReached]),
            Some(NotReached)
        );
    }

    #[test]
    fn truncate_sources_keeps_most_severe_source_verdicts() {
        let mut summary = ReasoningSummary {
            reachability: Some(Reachability::Reached),
            per_sink: vec![SinkResult {
                sink: sym("sink"),
                reachability: Reachability::Reached,
                sources: vec![
                    SinkSourceResult {
                        source: sym("clean"),
                        reachability: Reachability::NotReached,
                        graph_node: None,
                        sanitizers_present_in_source_fn: vec![],
                        sanitized_by: vec![],
                    },
                    SinkSourceResult {
                        source: sym("boundary"),
                        reachability: Reachability::BoundaryExited,
                        graph_node: None,
                        sanitizers_present_in_source_fn: vec![],
                        sanitized_by: vec![],
                    },
                    SinkSourceResult {
                        source: sym("culprit"),
                        reachability: Reachability::Reached,
                        graph_node: Some(0),
                        sanitizers_present_in_source_fn: vec![],
                        sanitized_by: vec![],
                    },
                ],
                sources_omitted: 0,
            }],
            source_count: 3,
            frontier_count: 3,
            sinks_omitted: 0,
        };

        summary.truncate_sources(1);

        assert_eq!(summary.per_sink[0].sources_omitted, 2);
        assert_eq!(
            summary.per_sink[0].sources[0].reachability,
            Reachability::Reached
        );
        assert!(matches!(
            &summary.per_sink[0].sources[0].source,
            SymbolRef::Variable { path, .. } if path == "culprit"
        ));
    }

    #[test]
    fn truncate_sources_preserves_source_order_when_not_truncating() {
        let mut summary = ReasoningSummary {
            reachability: Some(Reachability::Reached),
            per_sink: vec![SinkResult {
                sink: sym("sink"),
                reachability: Reachability::Reached,
                sources: vec![
                    SinkSourceResult {
                        source: sym("second"),
                        reachability: Reachability::NotReached,
                        graph_node: None,
                        sanitizers_present_in_source_fn: vec![],
                        sanitized_by: vec![],
                    },
                    SinkSourceResult {
                        source: sym("first"),
                        reachability: Reachability::Reached,
                        graph_node: Some(0),
                        sanitizers_present_in_source_fn: vec![],
                        sanitized_by: vec![],
                    },
                ],
                sources_omitted: 0,
            }],
            source_count: 2,
            frontier_count: 2,
            sinks_omitted: 0,
        };

        summary.truncate_sources(2);

        assert!(matches!(
            &summary.per_sink[0].sources[0].source,
            SymbolRef::Variable { path, .. } if path == "second"
        ));
        assert!(matches!(
            &summary.per_sink[0].sources[1].source,
            SymbolRef::Variable { path, .. } if path == "first"
        ));
        assert_eq!(summary.per_sink[0].sources_omitted, 0);
    }

    #[test]
    fn repair_after_clip_clears_nested_graph_nodes() {
        let mut summary = ReasoningSummary {
            reachability: Some(Reachability::Reached),
            per_sink: vec![SinkResult {
                sink: sym("sink"),
                reachability: Reachability::Reached,
                sources: vec![SinkSourceResult {
                    source: sym("source"),
                    reachability: Reachability::Reached,
                    graph_node: Some(3),
                    sanitizers_present_in_source_fn: vec!["html".into()],
                    sanitized_by: vec![],
                }],
                sources_omitted: 0,
            }],
            source_count: 1,
            frontier_count: 1,
            sinks_omitted: 0,
        };
        summary.repair_after_clip(3);
        assert_eq!(summary.per_sink[0].sources[0].graph_node, None);
    }
}

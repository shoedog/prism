//! Per-file extraction. Body moved from the baseline Rayon closure.
use super::{FileRefs, Producer};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::{
    reaching_definitions_with_exact, FlowConfidence, FlowDoubt, RdFileStats, RdOutcome,
};
use crate::data_flow::{ExactFlowEdge, FlowEdge, VarAccessKind, VarLocation};
use std::collections::{BTreeMap, BTreeSet};

impl Producer {
    // Preserve the extracted closure's line breaks for the indentation-only byte control.
    #[rustfmt::skip]
    pub(super) fn file_refs(file_path: &String, parsed: &ParsedFile) -> FileRefs {
        let _profile_file = crate::profile::Phase::file("dfg.file_extract", file_path);
        let mut defs: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
            BTreeMap::new();
        let mut uses: BTreeMap<(String, String, usize, AccessPath), Vec<VarLocation>> =
            BTreeMap::new();
        let mut edges = Vec::new();
        let mut labels = BTreeMap::new();
        let mut exact_labels = BTreeMap::new();
        let mut rd_function_stats = RdFileStats::default();

        let function_nodes = parsed.all_functions();
        let member_refs = crate::ast::member_reference_index::MemberReferenceSearch::new(parsed);
        // PR-B perf: one file-level CFG edge build shared by every pass.
        let file_cfg_edges = std::cell::OnceCell::new();
        let mut owner_counts = BTreeMap::new();
        for func_node in &function_nodes {
            // js-param-defs PR-B: anonymous JS/TS/TSX callables own a
            // synthetic, non-referenceable `<anon@L:C>` DFG identity.
            if let Some(name) = parsed.dfg_owner_name(func_node) {
                let key = (name, parsed.node_line_range(func_node).0);
                *owner_counts.entry(key).or_insert(0usize) += 1;
            }
        }
        for func_node in function_nodes {
            let func_name = match parsed.dfg_owner_name(&func_node) {
                Some(n) => n,
                None => continue,
            };
            // PR-B: a synthetic pass owns only its own scope (bytes inside
            // the callable, outside nested callables). Legacy named passes
            // keep their line-based coverage byte-identically.
            let synthetic = crate::ast::is_synthetic_owner(&func_name);
            let (start, end) = parsed.node_line_range(&func_node);
            let mut profile_owner = crate::profile::Owner::begin(file_path, parsed, func_node, &func_name);
            let owner_is_unique = owner_counts
                .get(&(func_name.clone(), start))
                .is_some_and(|count| *count == 1);
            let all_lines: BTreeSet<usize> = (start..=end).collect();
            let param_occurrences = parsed.function_parameter_occurrences(&func_node);
            let refused_formals: BTreeSet<_> = parsed
                .js_ts_parameter_refusals(&func_node)
                .into_iter()
                .filter(|(_, seam, eval, args)| synthetic && (*seam || *eval || *args))
                .map(|(name, _, _, _)| name)
                .collect();
            let refused_occurrence = |path: &AccessPath, byte| {
                refused_formals.contains(path.base.as_str())
                    && parsed.js_ts_occurrence_binds_formal(&func_node, &path.base, byte)
            };

            // Phase 3: Build local alias map for this function.
            // Tracks `ptr = dev` so that `ptr->field` resolves to `dev->field`.
            // Also tracks destructuring: `const { name } = obj` → name resolves to obj.name.
            let mut lvalue_spans =
                parsed.assignment_lvalue_spans_on_lines(&func_node, &all_lines);
            let mut nested_write_spans = Vec::new();
            if synthetic {
                lvalue_spans.retain(|span| {
                    // W4: object-backed writes are not proved lexical
                    // Defs, and cannot contribute even kill-only input.
                    if parsed.js_ts_span_in_with_body(span.start_byte, span.end_byte)
                        || refused_occurrence(&span.path, span.start_byte)
                        || span.path.base == "this"
                            && parsed.js_ts_span_this_rebound_between(
                                span.start_byte,
                                span.end_byte,
                                &func_node,
                            )
                    {
                        return false;
                    }
                    // An uninitialised body var supplies no value after
                    // the refused seam copy. Keep actual assignments.
                    if parsed.js_ts_seam_binding(&func_node, &span.path.base)
                        && parsed
                            .tree
                            .root_node()
                            .descendant_for_byte_range(span.start_byte, span.end_byte)
                            .and_then(|n| n.parent())
                            .is_some_and(|n| {
                                n.kind() == "variable_declarator"
                                    && n.child_by_field_name("value").is_none()
                            })
                    {
                        return false;
                    }
                    let own = parsed.js_ts_span_in_own_scope(
                        &func_node,
                        span.start_byte,
                        span.end_byte,
                    );
                    if !own
                        && func_node.start_byte() <= span.start_byte
                        && span.end_byte <= func_node.end_byte()
                    {
                        nested_write_spans.push(span.clone());
                    }
                    own
                });
            }
            let mut alias_targets_by_byte = BTreeMap::new();
            let (alias_map, raw_aliases) = if synthetic {
                // PR-B D11: a synthetic pass derives its aliases from its own
                // own-scope lvalue identifiers only (plain `=`, declarators,
                // flat object destructuring); outer/nested/augmented
                // assignments never mint an alias here (fail closed).
                let raw: Vec<_> = lvalue_spans
                    .iter()
                    .filter(|span| span.path.is_simple())
                    .filter_map(|span| {
                        parsed
                            .js_ts_own_alias_target(span.start_byte, span.end_byte)
                            .map(|target| {
                                alias_targets_by_byte
                                    .insert(span.start_byte, target.clone());
                                (span.path.base.clone(), target, span.line)
                            })
                    })
                    .collect();
                (Self::resolve_alias_chain(&raw), raw)
            } else {
                Self::build_alias_map(parsed, &func_node, &all_lines)
            };
            let occurrence_byte = |byte| if synthetic { byte } else { 0 };
            let mut def_locs_by_occurrence: BTreeMap<
                (AccessPath, usize, usize),
                VarLocation,
            > = BTreeMap::new();
            let mut resolved_def_locs_by_occurrence: BTreeMap<
                (AccessPath, usize, usize),
                VarLocation,
            > = BTreeMap::new();
            let mut param_ref_jobs = Vec::new();
            let mut member_only_params: Vec<(AccessPath, VarLocation)> = Vec::new();
            let mut rd_defs = Vec::new();
            let function_edge_start = edges.len();

            // Register function parameters as Defs at the function start line.
            // Parameters are variable definitions that receive values from callers.
            // Without this, interprocedural data flow edges have no target.
            //
            // Skip parameters that are only used via field access (e.g. `dev.name`)
            // to preserve field isolation — a base-only Def would let taint on
            // `dev.name` leak to unrelated fields like `dev.id`.
            for (param_name, param_start_byte, param_end_byte) in &param_occurrences {
                if refused_formals.contains(param_name.as_str()) {
                    continue;
                }
                let path = AccessPath::simple(param_name.clone());
                // Skip parameters only used via field access (e.g. `dev.name`)
                // to preserve field isolation.
                let use_kind = parsed.classify_parameter_use(&func_node, param_name);
                if use_kind == crate::ast::ParameterUse::Unused {
                    continue;
                }
                let member_only = use_kind == crate::ast::ParameterUse::MemberOnly;
                let refs = if member_only {
                    BTreeMap::new()
                } else {
                    crate::profile::search_refs(parsed, &member_refs,
                        &func_node,
                        &path,
                        start,
                        *param_start_byte,
                        None,
                        synthetic,
                    )
                };
                let param_decl_line = parsed.line_for_byte(*param_start_byte);

                // Parameter Defs are pinned to the function's `start` line for stable lookup.
                // `cpg/trace.rs` treats the arg→param DataFlow edge itself as the boundary for
                // recursive/self calls, so body locals on the signature line are not classified
                // as parameters.
                let loc = VarLocation {
                    file: file_path.clone(),
                    function: func_name.clone(),
                    function_start_line: start,
                    line: start,
                    path: path.clone(),
                    start_byte: *param_start_byte,
                    end_byte: *param_end_byte,
                    kind: VarAccessKind::Def,
                };
                Self::push_rd_def(&mut rd_defs, &loc, false);
                defs.entry((file_path.clone(), func_name.clone(), start, path.clone()))
                    .or_default()
                    .push(loc.clone());
                if member_only {
                    member_only_params.push((path.clone(), loc.clone()));
                }
                param_ref_jobs.push((path, loc, refs, param_decl_line));
            }

            // Find all definitions (L-values) with structured access paths
            for span in &lvalue_spans {
                let loc = VarLocation {
                    file: file_path.clone(),
                    function: func_name.clone(),
                    function_start_line: start,
                    line: span.line,
                    path: span.path.clone(),
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                    kind: VarAccessKind::Def,
                };
                Self::push_rd_def(&mut rd_defs, &loc, false);
                defs.entry((
                    file_path.clone(),
                    func_name.clone(),
                    start,
                    span.path.clone(),
                ))
                .or_default()
                .push(loc.clone());
                def_locs_by_occurrence
                    .entry((
                        span.path.clone(),
                        span.line,
                        occurrence_byte(span.start_byte),
                    ))
                    .or_insert(loc.clone());

                // Phase 3: If this path's base is aliased, also register a def under
                // the resolved path so edges connect through aliases.
                // For field paths (ptr->field → dev->field): resolves base through alias.
                // For simple paths from destructuring (name → device.name): creates
                // a field-qualified def so taint connects through destructured variables.
                // PR-B D11: in a synthetic pass an alias twin exists only on the
                // line that establishes the alias, to that line's target;
                // re-assigning the alias name elsewhere breaks the alias, it does
                // not write the target (flow-insensitive twins: legacy E11).
                let twin = if synthetic {
                    Self::resolve_on_alias_span(
                        &alias_targets_by_byte,
                        &span.path,
                        span.start_byte,
                    )
                } else {
                    Self::resolve_path(&alias_map, &span.path)
                };
                if let Some(resolved) = twin {
                    if resolved != span.path.clone() {
                        let resolved_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: span.line,
                            path: resolved.clone(),
                            start_byte: span.start_byte,
                            end_byte: span.end_byte,
                            kind: VarAccessKind::Def,
                        };
                        Self::push_rd_def(&mut rd_defs, &resolved_loc, true);
                        defs.entry((
                            file_path.clone(),
                            func_name.clone(),
                            start,
                            resolved.clone(),
                        ))
                        .or_default()
                        .push(resolved_loc.clone());
                        resolved_def_locs_by_occurrence
                            .entry((resolved, span.line, occurrence_byte(span.start_byte)))
                            .or_insert(resolved_loc);
                    }
                }
            }

            // Register defs for alias-resolved paths that did not surface
            // through lvalue spans. When they do surface, the lvalue loop
            // above already registered the resolved def with the raw span.
            for (alias, _target, alias_line) in &raw_aliases {
                if lvalue_spans.iter().any(|span| {
                    span.line == *alias_line
                        && span.path.is_simple()
                        && span.path.base == *alias
                }) {
                    continue;
                }
                if let Some(resolved_str) = alias_map.get(alias) {
                    let resolved_ap = AccessPath::from_expr(resolved_str);
                    if resolved_ap.has_fields() {
                        let anchor = parsed.line_start_byte(*alias_line);
                        let loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: *alias_line,
                            path: resolved_ap.clone(),
                            start_byte: anchor,
                            end_byte: anchor,
                            kind: VarAccessKind::Def,
                        };
                        debug_assert_eq!(loc.start_byte, loc.end_byte);
                        Self::push_rd_def(&mut rd_defs, &loc, true);
                        defs.entry((
                            file_path.clone(),
                            func_name.clone(),
                            start,
                            resolved_ap.clone(),
                        ))
                        .or_default()
                        .push(loc.clone());
                        resolved_def_locs_by_occurrence
                            .entry((resolved_ap, *alias_line, 0))
                            .or_insert(loc);
                    }
                }
            }

            // R-values: insert real-span occurrences before any line
            // anchors so CPG var-node dedup keeps the real span.
            let mut rvalue_spans =
                parsed.rvalue_identifier_spans_on_lines(&func_node, &all_lines);
            if synthetic {
                // PR-B D9: new passes never count non-reference
                // positions (keys, attribute/property names) as Uses.
                rvalue_spans.retain(|span| {
                    !parsed.js_ts_span_is_non_reference(span.start_byte, span.end_byte)
                        && !parsed.js_ts_span_is_not_a_read(span.start_byte, span.end_byte)
                        && !parsed.js_ts_span_in_with_body(span.start_byte, span.end_byte)
                        && !refused_occurrence(&span.path, span.start_byte)
                });
            }
            let mut preferred_use_locs: BTreeMap<(AccessPath, usize), VarLocation> =
                BTreeMap::new();
            for span in &rvalue_spans {
                let use_loc = VarLocation {
                    file: file_path.clone(),
                    function: func_name.clone(),
                    function_start_line: start,
                    line: span.line,
                    path: span.path.clone(),
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                    kind: VarAccessKind::Use,
                };
                preferred_use_locs
                    .entry((span.path.clone(), span.line))
                    .or_insert(use_loc.clone());
                uses.entry((
                    file_path.clone(),
                    func_name.clone(),
                    start,
                    span.path.clone(),
                ))
                .or_default()
                .push(use_loc);
            }

            let mut get_use =
                |path: &AccessPath,
                 ref_line: usize,
                 allow: bool,
                 admitted: (usize, usize)| {
                    if synthetic {
                        if refused_occurrence(path, admitted.0) {
                            return None;
                        }
                        // The reference walk has already proved this occurrence's
                        // binding and read role for the current Def.
                        let use_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: ref_line,
                            path: path.clone(),
                            start_byte: admitted.0,
                            end_byte: admitted.1,
                            kind: VarAccessKind::Use,
                        };
                        uses.entry((
                            file_path.clone(),
                            func_name.clone(),
                            start,
                            path.clone(),
                        ))
                        .or_default()
                        .push(use_loc.clone());
                        return Some(use_loc);
                    }
                    if let Some(existing) =
                        preferred_use_locs.get(&(path.clone(), ref_line))
                    {
                        return Some(existing.clone());
                    }
                    if !allow {
                        return None;
                    }
                    let anchor = parsed.line_start_byte(ref_line);
                    let use_loc = VarLocation {
                        file: file_path.clone(),
                        function: func_name.clone(),
                        function_start_line: start,
                        line: ref_line,
                        path: path.clone(),
                        start_byte: anchor,
                        end_byte: anchor,
                        kind: VarAccessKind::Use,
                    };
                    debug_assert_eq!(use_loc.start_byte, use_loc.end_byte);
                    uses.entry((file_path.clone(), func_name.clone(), start, path.clone()))
                        .or_default()
                        .push(use_loc.clone());
                    preferred_use_locs.insert((path.clone(), ref_line), use_loc.clone());
                    Some(use_loc)
                };

            // Create edges from param def to all uses in the function body.
            for (path, loc, refs, param_decl_line) in &param_ref_jobs {
                for (ref_line, admitted) in refs {
                    let in_signature_line =
                        *ref_line >= start && *ref_line <= *param_decl_line;
                    if let Some(use_loc) =
                        get_use(path, *ref_line, !in_signature_line, *admitted)
                    {
                        edges.push(FlowEdge {
                            from: loc.clone(),
                            to: use_loc,
                        });
                    }
                }
            }

            // Find all uses of each defined variable/path.
            // Cross-line references remain line-anchored unless a real
            // rvalue span already exists for the same identity key.
            for span in &lvalue_spans {
                let path = &span.path;
                let def_line = span.line;
                // R1: every JS/TS Def, including writes, stays in its
                // binding environment; body vars cannot reach defaults.
                let def_scope =
                    crate::profile::gates::def_scope(parsed, &func_node, &path.base, span.start_byte);
                let refs = crate::profile::search_refs(parsed, &member_refs,
                    &func_node,
                    path,
                    def_line,
                    span.start_byte,
                    def_scope,
                    synthetic,
                );
                for (ref_line, admitted) in &refs {
                    if *ref_line == def_line
                        || (synthetic
                            && parsed.js_ts_seam_binding(&func_node, &path.base)
                            && admitted.0 < span.start_byte)
                    {
                        continue; // Skip self-reference
                    }
                    let Some(use_loc) = get_use(path, *ref_line, true, *admitted) else {
                        continue;
                    };
                    let def_loc = def_locs_by_occurrence
                        .get(&(path.clone(), def_line, occurrence_byte(span.start_byte)))
                        .cloned()
                        .unwrap_or_else(|| VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: def_line,
                            path: path.clone(),
                            start_byte: span.start_byte,
                            end_byte: span.end_byte,
                            kind: VarAccessKind::Def,
                        });
                    edges.push(FlowEdge {
                        from: def_loc,
                        to: use_loc,
                    });
                }

                // Phase 3: Also create edges for the alias-resolved path.
                let twin = if synthetic {
                    Self::resolve_on_alias_span(
                        &alias_targets_by_byte,
                        path,
                        span.start_byte,
                    )
                } else {
                    Self::resolve_path(&alias_map, path)
                };
                if let Some(resolved) = twin {
                    if resolved != *path {
                        let resolved_refs = crate::profile::search_refs(parsed, &member_refs,
                            &func_node,
                            &resolved,
                            def_line,
                            span.start_byte,
                            crate::profile::gates::def_scope(parsed,
                                &func_node,
                                &resolved.base,
                                span.start_byte,
                            ),
                            synthetic,
                        );
                        for (ref_line, admitted) in &resolved_refs {
                            if *ref_line == def_line {
                                continue;
                            }
                            let Some(use_loc) =
                                get_use(&resolved, *ref_line, true, *admitted)
                            else {
                                continue;
                            };
                            let def_loc = resolved_def_locs_by_occurrence
                                .get(&(
                                    resolved.clone(),
                                    def_line,
                                    occurrence_byte(span.start_byte),
                                ))
                                .cloned()
                                .unwrap_or_else(|| VarLocation {
                                    file: file_path.clone(),
                                    function: func_name.clone(),
                                    function_start_line: start,
                                    line: def_line,
                                    path: resolved.clone(),
                                    start_byte: span.start_byte,
                                    end_byte: span.end_byte,
                                    kind: VarAccessKind::Def,
                                });
                            edges.push(FlowEdge {
                                from: def_loc,
                                to: use_loc,
                            });
                        }
                    }
                }
            }

            drop(get_use);
            // PR-C: projection edges Def(p) -> Use(p.<path>) for
            // member-only formals. Never between two different field
            // paths. One body walk per formal collects the admitted read
            // occurrences and the binding-proved member writes; a write to
            // a prefix of a path, to the path, or below it suppresses it
            // (narrow choice, flow-insensitive). Every Use is built at the
            // exact admitted span, for named and synthetic owners alike.
            for (base_path, base_loc) in &member_only_params {
                let facts = parsed.formal_member_facts(
                    &func_node,
                    &base_path.base,
                    base_loc.start_byte,
                );
                for field_path in &facts.outermost {
                    if facts.is_killed(field_path) {
                        continue;
                    }
                    let Some(occurrences) = facts.reads.get(field_path) else {
                        continue;
                    };
                    for (ref_line, (start_byte, end_byte)) in occurrences {
                        let use_loc = VarLocation {
                            file: file_path.clone(),
                            function: func_name.clone(),
                            function_start_line: start,
                            line: *ref_line,
                            path: field_path.clone(),
                            start_byte: *start_byte,
                            end_byte: *end_byte,
                            kind: VarAccessKind::Use,
                        };
                        uses.entry((
                            file_path.clone(),
                            func_name.clone(),
                            start,
                            field_path.clone(),
                        ))
                        .or_default()
                        .push(use_loc.clone());
                        edges.push(FlowEdge {
                            from: base_loc.clone(),
                            to: use_loc,
                        });
                    }
                }
            }

            // F1: captured writes inside nested callables contribute to
            // RD kills, but never become emitted Defs or edge sources.
            for span in &nested_write_spans {
                let Some(node) = parsed
                    .tree
                    .root_node()
                    .descendant_for_byte_range(span.start_byte, span.end_byte)
                else {
                    continue;
                };
                let same_binding = rd_defs.iter().any(|d| {
                    d.path == span.path
                        && !parsed.js_ts_reference_fenced(
                            &node,
                            &func_node,
                            &span.path.base,
                            d.start_byte,
                        )
                });
                if !same_binding
                    || (span.path.base == "this"
                        && parsed.js_ts_this_rebound_between(&node, &func_node))
                {
                    continue;
                }
                Self::push_rd_def(
                    &mut rd_defs,
                    &VarLocation {
                        file: file_path.clone(),
                        function: func_name.clone(),
                        function_start_line: start,
                        line: span.line,
                        path: span.path.clone(),
                        start_byte: span.start_byte,
                        end_byte: span.end_byte,
                        kind: VarAccessKind::Def,
                    },
                    false,
                );
            }
            let function_edges = &edges[function_edge_start..];
            let exact_candidates = Self::exact_read_candidates(
                parsed,
                &func_node,
                owner_is_unique,
                &param_occurrences,
                &raw_aliases,
                &lvalue_spans,
                &rvalue_spans,
                function_edges,
            );
            let cfg = file_cfg_edges.get_or_init(|| crate::profile::file_cfg(&mut profile_owner, file_path, || crate::cfg::build_cfg_edges_with_arms(parsed)));
            crate::profile::Owner::reaching(&mut profile_owner, rd_defs.len(), lvalue_spans.len());
            match reaching_definitions_with_exact(
                parsed,
                &func_node,
                &rd_defs,
                function_edges,
                &exact_candidates,
                cfg,
            ) {
                RdOutcome::Available(result) => {
                    Self::store_supplemental_exact(
                        &mut exact_labels,
                        function_edges,
                        &result.labels,
                        result.exact_labels,
                    );
                    for (key, label) in result.labels {
                        Self::insert_label(&mut labels, key, label);
                    }
                    // Task 5 owns loop-carried telemetry. Consume the
                    // classified set here without changing edge shape.
                    drop(result.loop_carried_edges);
                }
                RdOutcome::Unavailable(reason) => {
                    if reason.is_def_cap() || reason.is_line_cap() {
                        rd_function_stats.record_over_cap(func_name.clone(), start);
                    } else {
                        rd_function_stats.record_without_cfg(func_name.clone(), start);
                    }
                    let fallback = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
                    for edge in function_edges {
                        Self::insert_label(
                            &mut labels,
                            (edge.from.clone(), edge.to.clone()),
                            fallback,
                        );
                    }
                    let classified = exact_candidates
                        .iter()
                        .map(|edge| (ExactFlowEdge::from_legacy(edge), fallback))
                        .collect();
                    let fallback_labels: BTreeMap<_, _> = function_edges
                        .iter()
                        .map(|edge| ((edge.from.clone(), edge.to.clone()), fallback))
                        .collect();
                    Self::store_supplemental_exact(
                        &mut exact_labels,
                        function_edges,
                        &fallback_labels,
                        classified,
                    );
                }
            }
        }

        FileRefs {
            file_path: file_path.clone(),
            defs,
            uses,
            edges,
            labels,
            exact_labels,
            rd_function_stats,
        }
    }
}

// Consuming boundary: expanded scratch dies on this worker before parallel collection.
impl Producer {
    pub(super) fn compact_file_refs(
        file: &String,
        parsed: &ParsedFile,
    ) -> crate::data_flow::compact::CompactDfg {
        let _profile_file = crate::profile::Phase::file("dfg.file", file);
        let parts = Self::file_refs(file, parsed).into_parts();
        #[cfg(test)]
        let legacy_adjacency_rows = parts
            .adjacency
            .as_ref()
            .map_or(0, |(f, b)| f.len() + b.len());
        let graph = crate::data_flow::compact::CompactDfg::from_parts(parts)
            .expect("DFG exceeds u32 location capacity");
        #[cfg(test)]
        if let Some(records) = FILE_RESULTS.lock().unwrap().as_mut() {
            if let Some(record) = records.get_mut(file) {
                *record = (
                    graph.arena.rows().len(),
                    graph.edges().len(),
                    legacy_adjacency_rows,
                    crate::data_flow::test_support::compact::storage(&graph),
                );
            }
        }
        graph
    }
}
#[cfg(test)]
pub(in crate::data_flow) static FILE_RESULTS: std::sync::Mutex<
    Option<BTreeMap<String, (usize, usize, usize, Vec<u8>)>>,
> = std::sync::Mutex::new(None);

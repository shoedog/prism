//! Frozen main (3916fea7) builder: independent test-only differential oracle.
use super::*;

pub(super) fn build<'a, I>(files: I) -> ProofIndex
where
    I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
{
    tests::record_oracle();
    if !crate::anon_owner::EMIT_ANONYMOUS_OWNERS && !cfg!(test) {
        return ProofIndex::default();
    }
    let mut result = ProofIndex::default();
    for (file, parsed) in files {
        if !matches!(
            parsed.language,
            crate::languages::Language::JavaScript
                | crate::languages::Language::TypeScript
                | crate::languages::Language::Tsx
        ) {
            continue;
        }
        let file_supported = parsed.anonymous_file_environment().supported();
        let mut proof = FileProofs {
            program_supported: file_supported,
            ..Default::default()
        };
        let mut cache = JsBindingCache::for_sites();
        for owner in crate::anon_owner::candidate_nodes(parsed, true) {
            let Some(id) = crate::anon_owner::identity(parsed, &owner, file) else {
                continue;
            };
            // Inferred property names identify owners, never lexical declarations.
            // A named function expression still binds its own explicit name.
            if owner.kind() == "method_definition"
                || crate::languages::js_inferred_name::js_inferred_name_source(owner)
                    .is_some_and(|inferred| inferred.is_property(&parsed.source))
            {
                proof.property_methods.insert(id.clone());
            }
            proof.expression_scopes.record(owner, &id);
            let supported = file_supported && parsed.anonymous_evaluation_supported(owner);
            proof.owners.insert(id.clone(), supported);
            cache.proof_trace = Some(Vec::new());
            parsed.js_ts_return_facts(owner, &mut cache);
            let (safe, imports) = dependencies(parsed, &mut cache);
            proof.returns.insert(id, (supported && safe, imports));
        }
        let mut stack = vec![parsed.tree.root_node()];
        while let Some(node) = stack.pop() {
            if matches!(
                node.kind(),
                "call_expression" | "jsx_opening_element" | "jsx_self_closing_element"
            ) {
                let callee = node.child_by_field_name(if node.kind() == "call_expression" {
                    "function"
                } else {
                    "name"
                });
                if let Some(callee) = callee {
                    let name = if callee.kind() == "member_expression" {
                        callee
                            .child_by_field_name("property")
                            .map(|n| parsed.node_text(&n))
                    } else {
                        Some(parsed.node_text(&callee))
                    };
                    if let Some(name) = name {
                        cache.proof_trace = Some(Vec::new());
                        let binding = CallGraph::js_local_binding_at(
                            parsed,
                            node.start_byte(),
                            node.end_byte(),
                            name,
                            &mut cache,
                        );
                        let (safe, imports) = dependencies(parsed, &mut cache);
                        proof.sites.insert(
                            (node.start_byte(), node.end_byte()),
                            BindingEvidence {
                                binding,
                                dependencies_supported: file_supported && safe,
                                imports,
                            },
                        );
                    }
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        result.0.insert(file.clone(), proof);
    }
    result
}

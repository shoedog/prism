use super::*;
use crate::anon_owner::{candidate_nodes, identity, is_anonymous_owner_name, S4_CANDIDATES};
use crate::languages::Language;

fn parse(source: &str, language: Language) -> ParsedFile {
    ParsedFile::parse("original.ts", source, language).unwrap()
}

fn anonymous(parsed: &ParsedFile) -> Vec<(FunctionId, (usize, usize))> {
    candidate_nodes(parsed, true)
        .into_iter()
        .filter_map(|n| {
            let id = identity(parsed, &n, "alias.ts")?;
            (is_anonymous_owner_name(&id.name)
                && !crate::anon_owner::is_module_owner_name(&id.name))
            .then_some((id, (n.start_byte(), n.end_byte())))
        })
        .collect()
}

#[test]
fn enumerates_once_per_index_and_is_lazy_for_named_and_empty() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for count in [1, 32, 128] {
            let p = parse(
                &format!(
                    "function named(){{}} consume({});",
                    vec!["() => f()"; count].join(",")
                ),
                language,
            );
            let expected = anonymous(&p);
            assert_eq!(expected.len(), count);
            let index = SourceIndex::new(&p);
            S4_CANDIDATES.with(|c| c.set(0));
            let named = FunctionId {
                file: "other".into(),
                name: "named".into(),
                start_line: 1,
                end_line: 99,
            };
            assert_eq!(index.source_bytes(&named), source_bytes(&p, &named));
            assert_eq!(S4_CANDIDATES.with(|c| c.get()), 0);
            for _ in 0..2 {
                for (id, span) in &expected {
                    assert_eq!(index.source_bytes(id), Some(*span));
                }
            }
            assert_eq!(
                S4_CANDIDATES.with(|c| c.get()),
                1,
                "one enumeration per borrowed index, owners={count}, {language:?}"
            );
        }
        let empty = parse("", language);
        let index = SourceIndex::new(&empty);
        S4_CANDIDATES.with(|c| c.set(0));
        assert_eq!(S4_CANDIDATES.with(|c| c.get()), 0);
        let absent = FunctionId {
            file: "alias".into(),
            name: "<anon@1:1>".into(),
            start_line: 1,
            end_line: 1,
        };
        for _ in 0..3 {
            assert_eq!(index.source_bytes(&absent), None);
        }
        assert_eq!(S4_CANDIDATES.with(|c| c.get()), 1);
    }
}

#[test]
fn exact_identity_aliases_missing_generators_and_nested_callbacks() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let p = parse("function outer(){ consume((p) => {\n return consume((q) => p + q);\n}, function*(){ yield 1; }, function* named(){ yield 2; }); }\nfunction named(){ return 3; }", language);
        let index = SourceIndex::new(&p);
        let expected = anonymous(&p);
        assert_eq!(expected.len(), 3);
        for (id, span) in expected {
            assert_eq!(index.source_bytes(&id), Some(span));
            assert_eq!(source_bytes(&p, &id), Some(span));
            let mut alias = id.clone();
            alias.file = "different-file.js".into();
            assert_eq!(index.source_bytes(&alias), Some(span));
            for field in 0..3 {
                let mut missing = id.clone();
                match field {
                    0 => missing.name.push('x'),
                    1 => missing.start_line += 1,
                    _ => missing.end_line += 1,
                }
                assert_eq!(index.source_bytes(&missing), None, "{missing:?}");
            }
        }
        let generators = parse(
            "function* declared(p){ yield p; }\nconsume(function* expressed(p){ yield p; });",
            language,
        );
        let declaration = candidate_nodes(&generators, true)
            .into_iter()
            .find(|node| node.kind() == "generator_function_declaration")
            .unwrap();
        let mut declared = identity(&generators, &declaration, "generator-alias").unwrap();
        declared.end_line += 99;
        let expressed = FunctionId {
            file: "generator-alias".into(),
            name: "expressed".into(),
            start_line: 2,
            end_line: 2,
        };
        let named_index = SourceIndex::new(&generators);
        S4_CANDIDATES.with(|c| c.set(0));
        assert_eq!(
            named_index.source_bytes(&declared),
            Some((declaration.start_byte(), declaration.end_byte()))
        );
        assert_eq!(
            named_index.source_bytes(&declared),
            source_bytes(&generators, &declared)
        );
        assert_eq!(source_bytes(&generators, &expressed), None);
        assert_eq!(named_index.source_bytes(&expressed), None);
        assert_eq!(
            S4_CANDIDATES.with(|c| c.get()),
            0,
            "named generators keep the eager fast path, including missing expressions"
        );

        // Named lookup intentionally ignores end_line, even in duplicate arrangements.
        for node in candidate_nodes(&p, true) {
            let Some(mut id) = identity(&p, &node, "requested-alias") else {
                continue;
            };
            if is_anonymous_owner_name(&id.name) {
                continue;
            }
            id.end_line += 100;
            assert_eq!(index.source_bytes(&id), source_bytes(&p, &id));
        }
    }
}

#[test]
fn fresh_parse_and_replaced_tree_cannot_reuse_spans() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let mut p = parse("consume(() => 1);", language);
        let (id, first) = anonymous(&p).remove(0);
        {
            let index = SourceIndex::new(&p);
            assert_eq!(index.source_bytes(&id), Some(first));
        }
        let replacement = parse("consume(() => longerName);", language);
        p.source = replacement.source;
        p.tree = replacement.tree;
        let index = SourceIndex::new(&p);
        let second = source_bytes(&p, &id).unwrap();
        assert_ne!(first, second);
        assert_eq!(index.source_bytes(&id), Some(second));
        let fresh = parse("consume(() => differentLengthAgain);", language);
        assert_eq!(
            SourceIndex::new(&fresh).source_bytes(&id),
            source_bytes(&fresh, &id)
        );
    }
}

#[test]
fn duplicate_identity_retains_first_candidate_and_named_lookup_stays_legacy() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let p = parse("consume(function same(){ return 1; }, function same(){ return 222; }, function* same(){ yield 3333; }, () => 0); function same(){ return 44444; }", language);
        let index = SourceIndex::new(&p);
        let mut duplicates = Vec::new();
        for node in candidate_nodes(&p, true) {
            let Some(id) = identity(&p, &node, "alias") else {
                continue;
            };
            if id.name == "same" {
                duplicates.push((id, (node.start_byte(), node.end_byte())));
            }
        }
        assert!(duplicates.len() >= 3);
        let (id, first) = &duplicates[0];
        assert!(duplicates
            .iter()
            .skip(1)
            .any(|(other, span)| other == id && span != first));
        assert_eq!(
            index
                .anonymous_spans()
                .get(&(id.name.clone(), id.start_line, id.end_line)),
            Some(first)
        );
        for (mut id, _) in duplicates {
            id.file = "another-alias".into();
            id.end_line = 999;
            assert_eq!(index.source_bytes(&id), source_bytes(&p, &id));
        }
    }
}

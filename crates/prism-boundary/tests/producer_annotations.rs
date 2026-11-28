//! P2-1 rows: swaggo annotation bindings. Each test names its row and DESIGN clause (5.1-5.3).
//! Row ids: g = PLAN P2 seed rows, X = prototype carry-over, A = annotation grammar extensions.

#[allow(dead_code)]
#[path = "support/producer.rs"]
mod support;

use prism_boundary::model::{AnnotationDisposition as D, EdgeGrade};
use serde_json::json;
use support::*;

type Edge = (String, Option<EdgeGrade>, String);

fn bound(op: &str, handler: &str) -> Edge {
    (op.into(), Some(EdgeGrade::Bound), handler.into())
}

fn candidate(op: &str, handler: &str) -> Edge {
    (op.into(), Some(EdgeGrade::Candidate), handler.into())
}

fn inline(source: &str, ops: &[(&str, &str)]) -> prism_boundary::model::BindingsDocument {
    run(&[("i.go", source.as_bytes())], &index(ops))
}

#[test]
fn g01_one_router_line_binds_the_method() {
    // DESIGN 5.3 swaggo_annotation: annotation, handler and schema are located separately.
    let src = go("g01");
    let doc = one("g01", &[("GET", "/x")]);
    assert_eq!(edges(&doc), [bound("op:GET /x", "H")]);
    let row = &rows(&doc)[0];
    let handler = row.handler.as_ref().unwrap();
    assert_eq!(
        (
            handler.package.as_deref(),
            handler.receiver.as_deref(),
            handler.receiver_type.as_deref()
        ),
        (Some("p"), Some("*T"), Some("T"))
    );
    assert_eq!(spanned(&src, handler.site.span), "func (*T) H() {}");
    assert_eq!(handler.site.line, 8);
    assert_eq!(
        row.sites.len(),
        1,
        "the binding carries the annotation site, no schema site"
    );
    let site = &row.sites[0];
    assert_eq!(spanned(&src, site.span), "// @Router /x [get]");
    assert_eq!(site.line, line_of(&src, site.span));
    assert_eq!(
        (site.tree.as_str(), site.file.as_str(), site.blob.as_str()),
        ("T1", "g01.go", "blob:g01.go")
    );
    assert_eq!(
        (row.original_method.as_str(), row.original_path.as_str()),
        ("get", "/x")
    );
    assert_eq!(row.provenance, "proven");
    assert!(
        row.success_statuses.is_empty() && row.message_roots.is_empty() && row.fields.is_empty()
    );
    assert_eq!(file_region(&doc, "g01.go").reason, None);
    assert_eq!(dispositions(&doc), [D::Bound]);
    assert_eq!(record(&doc, "/x").operation_ids, ["op:GET /x"]);
    assert_wire_round_trip(&doc);
}

#[test]
fn g02_two_router_lines_bind_one_function_twice() {
    // DESIGN 5.3 / 9: one edge per operation, each with its own annotation span.
    let src = go("g02");
    let doc = one("g02", &[("GET", "/x"), ("POST", "/y")]);
    assert_eq!(
        edges(&doc),
        [bound("op:GET /x", "H"), bound("op:POST /y", "H")]
    );
    let (a, b) = (&rows(&doc)[0], &rows(&doc)[1]);
    assert_eq!(a.handler, b.handler);
    assert_eq!(spanned(&src, a.sites[0].span), "// @Router /x [get]");
    assert_eq!(spanned(&src, b.sites[0].span), "// @Router /y [post]");
}

#[test]
fn g03_detached_annotation_is_a_missing_binding_with_no_stale_claim() {
    // DESIGN 5.3: detached annotation is recorded, never bound, and warns of nothing.
    let doc = one("g03", &[("GET", "/x")]);
    assert!(edges(&doc).is_empty());
    let record = record(&doc, "/x");
    assert_eq!(
        (record.disposition, record.reason.as_deref()),
        (D::Unattached, Some("not_attached_to_function"))
    );
    assert_eq!(
        entries(&doc, "g03.go"),
        [(
            "annotation_unattached".to_string(),
            vec!["op:GET /x".to_string()]
        )]
    );
    assert_eq!(
        file_region(&doc, "g03.go").reason,
        None,
        "the file itself was fully read"
    );
    assert!(!serde_json::to_string(&doc).unwrap().contains("stale"));
    // Control: without the blank line the same text binds.
    let attached = String::from_utf8(go("g03"))
        .unwrap()
        .replace("\n\nfunc", "\nfunc");
    let doc = inline(&attached, &[("GET", "/x")]);
    assert_eq!(edges(&doc), [bound("op:GET /x", "H")]);
    assert!(doc.entries.is_empty());
}

#[test]
fn g04_annotation_without_operation_in_a_complete_inventory() {
    // DESIGN 5.3: schema absence is a complete claim only when the inventory is complete.
    let doc = inline(
        "package p\n\n// @Router /new [get]\nfunc H() {}\n",
        &[("GET", "/old")],
    );
    assert!(edges(&doc).is_empty());
    assert_eq!(dispositions(&doc), [D::AnnotationWithoutOperation]);
    assert!(doc.entries.is_empty() && doc.unaccounted_scopes.is_empty());
    assert!(!serde_json::to_string(&doc).unwrap().contains("stale"));
    // Control: an operation the reader could not project makes the absence unproven.
    let gap =
        br#"{"swagger":"2.0","basePath":"/a","basePath":"/b","info":{"title":"t","version":"1"},
        "paths":{"/old":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let schema = index_at(gap, TREE, REV);
    let source = "package p\n\n// @Router /new [get]\nfunc H() {}\n";
    let doc = run(&[("i.go", source.as_bytes())], &schema);
    let record = record(&doc, "/new");
    assert_eq!(
        (record.disposition, record.reason.as_deref()),
        (D::NotChecked, Some("schema_operations_incomplete"))
    );
    // The base path is ambiguous (duplicate member), so the projected ids cannot be listed:
    // the gap is keyless and blocks every negative.
    assert_eq!(
        entries(&doc, "i.go"),
        [("annotation_not_checked".to_string(), Vec::<String>::new())]
    );
}

#[test]
fn g05_the_same_operation_on_two_functions_is_two_candidates() {
    // DESIGN 5.5: a tie grants no unique binding.
    let doc = one("g05", &[("GET", "/x")]);
    assert_eq!(
        edges(&doc),
        [candidate("op:GET /x", "H"), candidate("op:GET /x", "K")]
    );
    assert!(rows(&doc)
        .iter()
        .all(|row| row.evidence == ["competing_handlers"]));
    assert_eq!(dispositions(&doc), [D::Candidate, D::Candidate]);
    let single = one("g01", &[("GET", "/x")]);
    assert!(
        rows(&single)[0].evidence.is_empty(),
        "control: a unique handler stays bound"
    );
}

#[test]
fn g10_generated_file_keeps_its_bindings_and_notes_it() {
    // PLAN g10: bindings kept, a note in evidence; only the header before `package` counts.
    let doc = one("g10", &[("GET", "/x")]);
    assert_eq!(edges(&doc), [bound("op:GET /x", "H")]);
    assert_eq!(rows(&doc)[0].evidence, ["generated_file"]);
    assert_eq!(record(&doc, "/x").notes, ["generated_file"]);
    let late = one("g10_marker_after_package", &[("GET", "/x")]);
    assert_eq!(edges(&late), [bound("op:GET /x", "H")]);
    assert!(
        rows(&late)[0].evidence.is_empty(),
        "a marker after the package clause is no header"
    );
}

#[test]
fn x01_path_matching_never_collapses_slashes() {
    // DESIGN 5.1: the prototype collapsed `/x//y/` to `/x/y`; the typed path must not.
    let ops = [("GET", "/x//y/"), ("GET", "/x/y")];
    let doc = one("x01", &ops);
    assert_eq!(edges(&doc), [bound("op:GET /x//y/", "H")]);
    let src = String::from_utf8(go("x01"))
        .unwrap()
        .replace("/x//y/", "/x/y/");
    let doc = inline(&src, &ops);
    assert_eq!(
        dispositions(&doc),
        [D::AnnotationWithoutOperation],
        "a trailing slash is significant"
    );
}

#[test]
fn x09_a_missing_annotation_target_is_not_exported_as_a_binding() {
    // PLAN g04 / DESIGN 5.3: zero bindings; the method is part of the key.
    let doc = one("x09", &[("GET", "/old")]);
    assert!(edges(&doc).is_empty() && rows(&doc).is_empty());
    assert_eq!(
        dispositions(&doc),
        [D::AnnotationWithoutOperation, D::AnnotationWithoutOperation]
    );
    let doc = one("x09", &[("GET", "/new"), ("POST", "/old")]);
    assert_eq!(
        edges(&doc),
        [
            bound("op:GET /new", "New"),
            bound("op:POST /old", "WrongMethod")
        ]
    );
}

#[test]
fn a01_bare_comment_separators_stay_in_the_comment_run() {
    let src = go("a01");
    let doc = one("a01", &[("GET", "/x")]);
    assert_eq!(edges(&doc), [bound("op:GET /x", "H")]);
    assert_eq!(
        rows(&doc)[0].sites[0].line,
        line_of(&src, rows(&doc)[0].sites[0].span)
    );
}

#[test]
fn a02_block_comments_are_an_unsupported_form_not_a_guess() {
    // DESIGN 5.3: only `//` lines attach. A block comment is kept raw, with its span, as a gap.
    let src = go("a02");
    let doc = one("a02", &[("GET", "/x"), ("GET", "/y"), ("GET", "/z")]);
    // Fold 1: a keyless gap in the document could hide a competitor, so the observed edge is
    // kept as a candidate (competitor_unread) rather than a unique binding.
    assert_eq!(edges(&doc), [candidate("op:GET /z", "Line")]);
    let block = record(&doc, "/x");
    assert_eq!(
        (block.disposition, block.reason.as_deref()),
        (D::UnsupportedForm, Some("block_comment"))
    );
    assert!(spanned(&src, block.site.span).starts_with("/*\n@Router /x [get]\n*/"));
    let inline_block = record(&doc, "/y");
    assert_eq!(inline_block.disposition, D::UnsupportedForm);
    assert_eq!(
        spanned(&src, inline_block.site.span),
        "/* @Router /y [get] */"
    );
    let entries = entries(&doc, "a02.go");
    assert_eq!(
        entries,
        vec![("annotation_unsupported_form".to_string(), vec![]); 2]
    );
    let evidence = doc.entries[0].evidence.as_ref().unwrap();
    assert_eq!(evidence["span"], json!(block.site.span));
}

#[test]
fn a03_a_split_directive_is_malformed_and_its_sibling_still_binds() {
    let doc = one("a03", &[("GET", "/x"), ("GET", "/y")]);
    // Fold 1: a keyless gap in the document could hide a competitor, so the observed edge is
    // kept as a candidate (competitor_unread) rather than a unique binding.
    assert_eq!(edges(&doc), [candidate("op:GET /y", "Sibling")]);
    let split = record_of(&doc, "Split");
    assert_eq!(
        (split.disposition, split.reason.as_deref()),
        (D::Malformed, Some("malformed_directive"))
    );
    assert_eq!(
        entries(&doc, "a03.go"),
        [("annotation_malformed".to_string(), vec![])]
    );
}

#[test]
fn a04_directives_outside_a_declaration_comment_block_are_unattached() {
    // DESIGN 5.3 "directly above a function": inline, in a body, above a var and below a func.
    let doc = one("a04", &[("GET", "/w")]);
    assert_eq!(edges(&doc), [bound("op:GET /w", "Attached")]);
    let reason = |needle: &str| {
        let record = record(&doc, needle);
        assert_eq!(record.disposition, D::Unattached, "{needle}");
        record.reason.clone().unwrap()
    };
    assert_eq!(reason("/x"), "trailing_comment");
    assert_eq!(reason("/y"), "inside_function_body");
    assert_eq!(reason("/z"), "not_attached_to_function");
    assert_eq!(reason("/v"), "not_attached_to_function");
    assert!(
        records(&doc).iter().all(|r| !r.raw.contains("/s")),
        "a string literal is data"
    );
    let partial: Vec<_> = entries(&doc, "a04.go")
        .into_iter()
        .filter(|e| e.0 == "annotation_unattached")
        .collect();
    assert_eq!(partial.len(), 4);
}

#[test]
fn a05_only_the_canonical_spelling_binds() {
    // Case, slashes and trailing text are not guessed; Go whitespace around the directive is fine.
    let doc = one("a05", &[("GET", "/d"), ("GET", "/e")]);
    // Fold 1: a keyless gap in the document could hide a competitor, so the observed edge is
    // kept as a candidate (competitor_unread) rather than a unique binding.
    assert_eq!(
        edges(&doc),
        [
            candidate("op:GET /d", "Indented"),
            candidate("op:GET /e", "TabAfter")
        ]
    );
    for needle in ["/a [get]", "/b [get]", "/c [get]"] {
        let record = record(&doc, needle);
        assert_eq!(
            (record.disposition, record.reason.as_deref()),
            (D::Malformed, Some("malformed_directive"))
        );
    }
}

#[test]
fn a06_methods_fold_by_case_and_other_forms_are_gaps() {
    let doc = one("a06", &[("GET", "/x")]);
    // Fold 1: a keyless gap in the document could hide a competitor, so the observed edge is
    // kept as a candidate (competitor_unread) rather than a unique binding.
    assert_eq!(edges(&doc), [candidate("op:GET /x", "Fold")]);
    assert_eq!(
        rows(&doc)[0].original_method,
        "GeT",
        "the raw spelling is preserved"
    );
    let reason = |handler: &str| record_of(&doc, handler).reason.clone().unwrap();
    for handler in ["List", "Star", "Custom", "Open"] {
        assert_eq!(reason(handler), "malformed_directive", "{handler}");
    }
    assert_eq!(reason("Trace"), "unsupported_method");
    assert_eq!(
        doc.entries
            .iter()
            .filter(|e| e.possible_keys.is_empty())
            .count(),
        5
    );
}

#[test]
fn a07_brace_capture_and_literal_colon_are_different_paths() {
    let both = [("GET", "/x/{id}"), ("GET", "/x/:id")];
    let doc = one("a07", &both);
    assert_eq!(
        edges(&doc),
        [
            bound("op:GET /x/:id", "Colon"),
            bound("op:GET /x/{}", "Brace")
        ]
    );
    let doc = one("a07", &[("GET", "/x/{id}")]);
    assert_eq!(edges(&doc), [bound("op:GET /x/{}", "Brace")]);
    assert_eq!(
        record(&doc, "/x/:id").disposition,
        D::AnnotationWithoutOperation
    );
}

#[test]
fn a08_capture_names_and_repeats_are_compared_not_just_printed_ids() {
    let doc = one(
        "a08",
        &[("GET", "/x/{a}/{b}"), ("GET", "/x/file-{id}.{ext}")],
    );
    assert_eq!(edges(&doc), [bound("op:GET /x/file-{}.{}", "Mixed")]);
    let repeated = record(&doc, "{id}/{id}");
    assert_eq!(
        (repeated.disposition, repeated.reason.as_deref()),
        (D::NotChecked, Some("typed_path_differs"))
    );
    assert_eq!(entries(&doc, "a08.go")[0].1, ["op:GET /x/{}/{}"]);
}

#[test]
fn a09_only_absolute_literal_paths_are_read() {
    let doc = one("a09", &[("GET", "/x/{id}"), ("GET", "/x/%7Bid%7D")]);
    // Fold 1: a keyless gap in the document could hide a competitor, so the observed edge is
    // kept as a candidate (competitor_unread) rather than a unique binding.
    assert_eq!(edges(&doc), [candidate("op:GET /x/%7Bid%7D", "Encoded")]);
    for handler in ["Relative", "Absolute", "Query", "Fragment"] {
        let record = record_of(&doc, handler);
        assert_eq!(
            (record.disposition, record.reason.as_deref()),
            (D::Malformed, Some("invalid_path")),
            "{handler}"
        );
        assert!(
            record.operation_ids.is_empty(),
            "no repaired id for {handler}"
        );
    }
}

#[test]
fn a10_a_base_path_is_joined_once_and_never_stripped() {
    // DESIGN 5.1: authored path joins the authored base; a pre-prefixed annotation is not repaired.
    let schema = index_at(&swagger(Some("/api"), &[("GET", "/x")]), TREE, REV);
    let doc = run(
        &[("i.go", b"package p\n\n// @Router /x [get]\nfunc H() {}\n")],
        &schema,
    );
    assert_eq!(edges(&doc), [bound("op:GET /api/x", "H")]);
    let doc = run(
        &[(
            "i.go",
            b"package p\n\n// @Router /api/x [get]\nfunc H() {}\n",
        )],
        &schema,
    );
    assert!(edges(&doc).is_empty());
    let record = record(&doc, "/api/x");
    assert_eq!(
        (record.disposition, record.reason.as_deref()),
        (D::NotChecked, Some("annotation_path_includes_base"))
    );
}

#[test]
fn a11_every_server_projection_of_the_declaration_is_bound() {
    let schema = br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},"servers":[{"url":"/north"},{"url":"/south"}],
        "paths":{"/x":{"get":{"responses":{"200":{"description":"ok"}}}}}}"#;
    let doc = run(
        &[("i.go", b"package p\n\n// @Router /x [get]\nfunc H() {}\n")],
        &index_at(schema, TREE, REV),
    );
    assert_eq!(
        edges(&doc),
        [bound("op:GET /north/x", "H"), bound("op:GET /south/x", "H")]
    );
    assert_eq!(records(&doc).len(), 1);
    assert_eq!(
        records(&doc)[0].operation_ids,
        ["op:GET /north/x", "op:GET /south/x"]
    );
}

#[test]
fn a12_root_path_and_letter_case_are_exact() {
    let schema = index_at(&swagger(Some("/api/"), &[("GET", "/")]), TREE, REV);
    let id = schema.operations[0].id.to_string();
    let doc = run(
        &[("i.go", b"package p\n\n// @Router / [get]\nfunc H() {}\n")],
        &schema,
    );
    assert_eq!(edges(&doc), [bound(&id, "H")]);
    let doc = inline(
        "package p\n\n// @Router /X [get]\nfunc H() {}\n",
        &[("GET", "/x")],
    );
    assert_eq!(dispositions(&doc), [D::AnnotationWithoutOperation]);
}

#[test]
fn a13_a_repeated_annotation_is_one_edge_with_two_sites() {
    let doc = one("a13", &[("GET", "/x")]);
    assert_eq!(edges(&doc), [bound("op:GET /x", "Twice")]);
    assert_eq!(rows(&doc)[0].sites.len(), 2);
    assert_eq!(dispositions(&doc), [D::Bound, D::Bound]);
}

#[test]
fn a14_receiver_spelling_and_bodyless_declarations_are_kept() {
    let src = go("a14");
    let doc = one("a14", &[("GET", "/x"), ("GET", "/y"), ("GET", "/z")]);
    assert_eq!(
        edges(&doc),
        [
            bound("op:GET /x", "Ptr"),
            bound("op:GET /y", "Value"),
            bound("op:GET /z", "Bodyless")
        ]
    );
    let handler = |name: &str| {
        rows(&doc)
            .iter()
            .find_map(|r| r.handler.as_ref().filter(|h| h.name == name))
            .unwrap()
    };
    assert_eq!(
        (
            handler("Ptr").receiver.as_deref(),
            handler("Ptr").receiver_type.as_deref()
        ),
        (Some("*T[A]"), Some("T"))
    );
    assert_eq!(
        (
            handler("Value").receiver.as_deref(),
            handler("Value").receiver_type.as_deref()
        ),
        (Some("T[A]"), Some("T"))
    );
    assert_eq!(
        spanned(&src, handler("Bodyless").site.span),
        "func Bodyless()"
    );
    let bodyless = rows(&doc)
        .iter()
        .find(|r| r.operation_id == "op:GET /z")
        .unwrap();
    assert_eq!(bodyless.evidence, ["bodyless_declaration"]);
}

#[test]
fn a16_a_broken_body_keeps_the_attachment_and_a_broken_gap_does_not() {
    let doc = one("a16_header_intact", &[("GET", "/x")]);
    assert_eq!(
        edges(&doc),
        [bound("op:GET /x", "H")],
        "the header is intact"
    );
    let region = file_region(&doc, "a16_header_intact.go");
    assert_eq!(
        (region.status, region.reason.as_deref()),
        (
            prism_boundary::model::ExtractionStatus::Partial,
            Some("parse_error")
        )
    );
    assert!(
        region.possible_keys.is_empty(),
        "unknown domain blocks every dependent negative"
    );
    let doc = one("a16_error_between", &[("GET", "/x")]);
    assert!(
        edges(&doc).is_empty(),
        "an error between comment and declaration confirms no attachment"
    );
    assert_eq!(record(&doc, "/x").disposition, D::Unattached);
    assert_eq!(
        file_region(&doc, "a16_error_between.go").reason.as_deref(),
        Some("parse_error")
    );
}

#[test]
fn a17_offsets_are_physical_bytes_of_the_admitted_input() {
    // Multi-byte text, CRLF and a `//line` directive neither shift nor remap the site.
    let src = "package p\r\n\r\n// h\u{e9}llo\r\n//line other.go:100\r\n// @Router /x [get]\r\nfunc H() {}\r\n";
    let doc = run(&[("u.go", src.as_bytes())], &index(&[("GET", "/x")]));
    assert_eq!(edges(&doc), [bound("op:GET /x", "H")]);
    let site = &rows(&doc)[0].sites[0];
    assert_eq!((site.file.as_str(), site.line), ("u.go", 5));
    assert_eq!(spanned(src.as_bytes(), site.span), "// @Router /x [get]");
    assert_eq!(record(&doc, "/x").raw, "// @Router /x [get]");
    assert_eq!(rows(&doc)[0].handler.as_ref().unwrap().site.line, 6);
}

#[test]
fn a18_adjacent_directives_add_no_status_media_or_field_claim() {
    let doc = one("a18", &[("GET", "/x")]);
    assert_eq!(edges(&doc), [bound("op:GET /x", "H")]);
    let row = &rows(&doc)[0];
    assert!(row.success_statuses.is_empty() && row.message_roots.is_empty());
    assert!(row.fields.is_empty() && row.observed_fields.is_empty());
}

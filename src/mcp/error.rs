use super::output::{self, McpToolResult, SCHEMA_VERSION};
use crate::navigation::types::{QueryError, SymbolRef};

/// Max `AmbiguousSymbol` candidates retained in an error result. Their COUNT is unbounded; capping it
/// keeps the rendered JSON small while leaving the list valid (round-6 MAJOR — the prior whole-blob byte
/// clamp truncated this list mid-JSON, destroying the candidates the model needs to disambiguate).
const MAX_ERROR_CANDIDATES: usize = 20;

/// Clamp every repo-derived string inside a candidate `SymbolRef` before it is rendered into an error
/// result. Candidate names/paths come from repo source text with no length cap and never pass through
/// `shape_result`/`payload_budget`, so without this a handful of pathologically-long identifiers could
/// push a valid-JSON `isError` payload past the size cap (round-7 MAJOR — completes the round-6 count cap
/// by bounding per-candidate bytes, closing the error-path string-bounding class).
fn bound_symbol_ref(symbol: SymbolRef) -> SymbolRef {
    match symbol {
        SymbolRef::Function {
            file,
            name,
            start_line,
            end_line,
            start_byte,
            end_byte,
            ordinal,
        } => SymbolRef::Function {
            file: output::clamp_user_text(&file),
            name: output::clamp_user_text(&name),
            start_line,
            end_line,
            start_byte,
            end_byte,
            ordinal,
        },
        SymbolRef::Statement {
            file,
            line,
            kind,
            start_byte,
            end_byte,
            ordinal,
        } => SymbolRef::Statement {
            file: output::clamp_user_text(&file),
            line,
            kind: output::clamp_user_text(&kind),
            start_byte,
            end_byte,
            ordinal,
        },
        SymbolRef::Variable {
            file,
            function,
            line,
            path,
            access,
            start_byte,
            end_byte,
            ordinal,
        } => SymbolRef::Variable {
            file: output::clamp_user_text(&file),
            function: output::clamp_user_text(&function),
            line,
            path: output::clamp_user_text(&path),
            access: output::clamp_user_text(&access),
            start_byte,
            end_byte,
            ordinal,
        },
    }
}

#[derive(Debug)]
pub enum ToolError {
    BadArguments(String),
    Query(crate::navigation::types::QueryError),
}

impl ToolError {
    pub fn into_result(self) -> McpToolResult {
        match self {
            ToolError::BadArguments(message) => bad_arguments_result(message),
            ToolError::Query(error) => query_error_result(error),
        }
    }
}

/// Bound a `QueryError` BEFORE it is rendered, so rendering yields naturally-bounded VALID JSON
/// (round-6 MAJOR — clamping the whole serialized blob byte-truncated the `AmbiguousSymbol`
/// candidate list mid-JSON). User-controlled leaves (`seed`/`file`/`edge`) are clamped; the
/// `candidates` list is capped by COUNT and every retained candidate's strings are clamped
/// (round-7 MAJOR), so no path emits an unbounded string into an error result.
fn bound_query_error(error: QueryError) -> QueryError {
    match error {
        QueryError::AmbiguousSymbol { mut candidates } => {
            candidates.truncate(MAX_ERROR_CANDIDATES);
            let candidates = candidates.into_iter().map(bound_symbol_ref).collect();
            QueryError::AmbiguousSymbol { candidates }
        }
        QueryError::SymbolNotFound { seed } => QueryError::SymbolNotFound {
            seed: output::clamp_user_text(&seed),
        },
        QueryError::LocationOutOfRange { file, line } => QueryError::LocationOutOfRange {
            file: output::clamp_user_text(&file),
            line,
        },
        QueryError::UnsupportedFile { file } => QueryError::UnsupportedFile {
            file: output::clamp_user_text(&file),
        },
        QueryError::UnknownEdge { edge } => QueryError::UnknownEdge {
            edge: output::clamp_user_text(&edge),
        },
    }
}

pub fn query_error_result(error: QueryError) -> McpToolResult {
    // Full count of ambiguous matches before the cap, for the "showing N of M" signal (round-7 MINOR —
    // a silent candidate cap leaves an agent unable to tell the list is partial).
    let ambiguous_total = match &error {
        QueryError::AmbiguousSymbol { candidates } => Some(candidates.len()),
        _ => None,
    };
    // Bound the TYPED error before rendering so the rendered JSON is naturally bounded AND valid (a
    // whole-blob byte clamp would truncate an AmbiguousSymbol candidate list mid-JSON — round-6 MAJOR).
    // Candidate count is capped and every candidate string is clamped (round-7 MAJOR). The actionable
    // sentence is a fixed, trusted string and stays full.
    let error = bound_query_error(error);
    let mut content_text = crate::output::navigation::render_err(&error, "json").0;
    content_text.push('\n');
    content_text.push_str(actionable_sentence(&error));
    if let Some(total) = ambiguous_total {
        if total > MAX_ERROR_CANDIDATES {
            content_text
                .push_str(&format!(" Showing {MAX_ERROR_CANDIDATES} of {total} matches; narrow the seed (e.g. add `file`)."));
        }
    }
    McpToolResult {
        content_text,
        structured: None,
        is_error: true,
        meta: error_meta(),
    }
}

fn bad_arguments_result(message: String) -> McpToolResult {
    let message = output::clamp_user_text(&message);
    McpToolResult {
        content_text: format!(
            "bad arguments: {message}\nFix the tool arguments and retry with the documented schema."
        ),
        structured: None,
        is_error: true,
        meta: error_meta(),
    }
}

fn actionable_sentence(error: &QueryError) -> &'static str {
    match error {
        QueryError::AmbiguousSymbol { .. } => {
            "Specify a file-qualified seed, for example add the `file` field to the symbol seed."
        }
        QueryError::SymbolNotFound { .. } => {
            "Check the seed name or file and retry with a symbol that exists in the indexed repo."
        }
        QueryError::LocationOutOfRange { .. } => {
            "Choose a line that exists in the indexed file and retry."
        }
        QueryError::UnsupportedFile { .. } => {
            "Use a supported source file from the indexed repository."
        }
        QueryError::UnknownEdge { .. } => {
            "Use one of the documented edge names: Call, Return, DataFlow, Contains, ControlFlow, or FieldOf."
        }
    }
}

fn error_meta() -> serde_json::Map<String, serde_json::Value> {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "prism/schema_version".into(),
        serde_json::Value::String(SCHEMA_VERSION.into()),
    );
    meta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::types::SymbolRef;

    fn candidate(n: usize) -> SymbolRef {
        SymbolRef::Function {
            file: "a.rs".into(),
            name: format!("f{n}"),
            start_line: n,
            end_line: n,
            start_byte: 0,
            end_byte: 0,
            ordinal: 0,
        }
    }

    /// Round-6 MAJOR: clamping the whole serialized error blob byte-truncated an `AmbiguousSymbol`
    /// candidate list mid-JSON (lossy/invalid). Bounding the typed error first must cap the candidate
    /// COUNT and leave the rendered JSON valid so the model can disambiguate.
    #[test]
    fn ambiguous_candidates_capped_and_json_valid() {
        let error = QueryError::AmbiguousSymbol {
            candidates: (0..50).map(candidate).collect(),
        };
        let bounded = bound_query_error(error);
        match &bounded {
            QueryError::AmbiguousSymbol { candidates } => {
                assert_eq!(candidates.len(), MAX_ERROR_CANDIDATES);
            }
            other => panic!("expected AmbiguousSymbol, got {other:?}"),
        }

        let result = query_error_result(QueryError::AmbiguousSymbol {
            candidates: (0..50).map(candidate).collect(),
        });
        assert!(result.is_error);
        // content is `<json>\n<actionable sentence>`; the JSON portion is everything up to the LAST
        // newline that separates it from the trailing sentence. render_err emits pretty JSON (multi
        // line), so split on the final newline and parse the leading object.
        let (json_part, _sentence) = result
            .content_text
            .rsplit_once('\n')
            .expect("content must contain a newline separating json and the actionable sentence");
        let parsed: serde_json::Value =
            serde_json::from_str(json_part).expect("error JSON portion must parse as valid JSON");
        let candidates = parsed["error"]["AmbiguousSymbol"]["candidates"]
            .as_array()
            .expect("candidates array present");
        assert_eq!(
            candidates.len(),
            MAX_ERROR_CANDIDATES,
            "candidates must be capped, not byte-truncated"
        );
        // content_text starts with a parseable JSON object.
        assert!(result.content_text.starts_with('{'));
    }

    /// Round-7 MAJOR: candidate `SymbolRef` strings (e.g. a long function `name` from repo source) are
    /// repo-derived and unbounded; they must be clamped so a few pathological identifiers can't push a
    /// valid-JSON `isError` payload past the size cap. The rendered JSON must stay valid and each
    /// candidate name bounded.
    #[test]
    fn ambiguous_candidate_strings_clamped() {
        let huge = SymbolRef::Function {
            file: "a.rs".into(),
            name: "n".repeat(5000),
            start_line: 1,
            end_line: 1,
            start_byte: 0,
            end_byte: 0,
            ordinal: 0,
        };
        let bounded = bound_query_error(QueryError::AmbiguousSymbol {
            candidates: vec![huge.clone()],
        });
        match &bounded {
            QueryError::AmbiguousSymbol { candidates } => match &candidates[0] {
                SymbolRef::Function { name, .. } => assert!(
                    name.len() <= output::MAX_ECHO_BYTES + '…'.len_utf8(),
                    "candidate name must be clamped, got {} bytes",
                    name.len()
                ),
                other => panic!("expected Function, got {other:?}"),
            },
            other => panic!("expected AmbiguousSymbol, got {other:?}"),
        }

        let result = query_error_result(QueryError::AmbiguousSymbol {
            candidates: vec![huge],
        });
        let (json_part, _) = result.content_text.rsplit_once('\n').unwrap();
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        let name = parsed["error"]["AmbiguousSymbol"]["candidates"][0]["Function"]["name"]
            .as_str()
            .expect("candidate name present");
        assert!(name.len() <= output::MAX_ECHO_BYTES + '…'.len_utf8());
    }

    /// Round-7 MINOR: a silent candidate cap leaves an agent unable to tell the list is partial; the
    /// content must carry a "showing N of M" signal when the cap fires.
    #[test]
    fn ambiguous_cap_signals_total() {
        let result = query_error_result(QueryError::AmbiguousSymbol {
            candidates: (0..50).map(candidate).collect(),
        });
        assert!(
            result
                .content_text
                .contains(&format!("Showing {MAX_ERROR_CANDIDATES} of 50")),
            "expected an 'N of M' signal, got: {}",
            result.content_text
        );
    }

    /// A huge user-controlled `seed` is clamped (user leaf), and the rendered error stays valid JSON.
    #[test]
    fn symbol_not_found_seed_clamped_and_json_valid() {
        let bounded = bound_query_error(QueryError::SymbolNotFound {
            seed: "s".repeat(5000),
        });
        match &bounded {
            QueryError::SymbolNotFound { seed } => {
                assert!(
                    seed.len() <= super::output::MAX_ECHO_BYTES + '…'.len_utf8(),
                    "seed must be clamped, got {} bytes",
                    seed.len()
                );
            }
            other => panic!("expected SymbolNotFound, got {other:?}"),
        }

        let result = query_error_result(QueryError::SymbolNotFound {
            seed: "s".repeat(5000),
        });
        let (json_part, _) = result.content_text.rsplit_once('\n').unwrap();
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        let seed = parsed["error"]["SymbolNotFound"]["seed"].as_str().unwrap();
        assert!(seed.len() <= super::output::MAX_ECHO_BYTES + '…'.len_utf8());
    }
}

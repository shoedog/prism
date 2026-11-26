//! Runtime positional offsets for Step 5b argument binding.

use crate::ast::ParsedFile;
use crate::call_graph::CallSite;
use std::ops::Range;

pub(super) type Argument = (String, Range<usize>);

pub(super) struct Offsets {
    /// Exact slot for non-spread arguments with a known runtime offset.
    pub slot: Vec<Option<usize>>,
    /// Lower bound on each argument's offset, counting unknown spreads as empty.
    pub least: Vec<usize>,
}

pub(super) fn for_call(caller: &ParsedFile, site: &CallSite) -> (Vec<Argument>, Offsets) {
    let name = site.effective_source_callee_name();
    let args = if caller.language.is_js_family() {
        caller.call_argument_texts_and_spans_of_call(site.start_byte, site.end_byte, name)
    } else {
        caller.call_argument_texts_and_spans_at(site.start_byte, name)
    };
    let offsets = offsets(caller, &args);
    (args, offsets)
}

pub(super) fn offsets(caller: &ParsedFile, args: &[Argument]) -> Offsets {
    if !caller.language.is_js_family() {
        return Offsets {
            slot: (0..args.len()).map(Some).collect(),
            least: (0..args.len()).collect(),
        };
    }

    let mut result = Offsets {
        slot: Vec::with_capacity(args.len()),
        least: Vec::with_capacity(args.len()),
    };
    let mut least = 0;
    let mut exact = true;
    for (text, span) in args {
        result.least.push(least);
        if text.starts_with("...") {
            result.slot.push(None);
            match array_spread_length(caller, span) {
                Some(length) => least += length,
                None => exact = false,
            }
        } else {
            result.slot.push(exact.then_some(least));
            least += 1;
        }
    }
    result
}

fn array_spread_length(caller: &ParsedFile, span: &Range<usize>) -> Option<usize> {
    dense_array_length(caller.spread_argument_operand(span)?)
}

pub(super) fn dense_array_length(array: tree_sitter::Node<'_>) -> Option<usize> {
    if array.kind() != "array" {
        return None;
    }

    let mut cursor = array.walk();
    let mut length = 0;
    let mut expect_element = true;
    for child in array
        .children(&mut cursor)
        .filter(|child| !child.is_extra())
    {
        match child.kind() {
            "[" | "]" => {}
            // A comma following an element can be a separator or a trailing comma.
            "," if !expect_element => expect_element = true,
            // A comma where an element is expected is a hole. A spread in an
            // element's own expression (e.g. a nested array) does not change this array's length.
            "," | "spread_element" => return None,
            _ => {
                length += 1;
                expect_element = false;
            }
        }
    }
    Some(length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    fn assert_offsets(source: &str, language: Language, slot: &[Option<usize>], least: &[usize]) {
        let caller = ParsedFile::parse("fixture", source, language).unwrap();
        assert_eq!(caller.parse_error_count, 0, "{source}");
        let args = caller.call_argument_texts_and_spans_of_call(0, source.len(), "f");
        let actual = offsets(&caller, &args);
        assert_eq!(actual.slot, slot, "slot for {source}");
        assert_eq!(actual.least, least, "least for {source}");
    }

    #[test]
    fn dense_spreads_count_elements() {
        assert_offsets(
            "f(prefix, ...[0, /* ignored */ 1,], ...[], ...[[...items]], tail)",
            Language::JavaScript,
            &[Some(0), None, None, None, Some(4)],
            &[0, 1, 3, 3, 4],
        );
    }

    #[test]
    fn unknown_spreads_keep_only_a_lower_bound() {
        for operand in ["[, 1]", "[...items]", "items"] {
            assert_offsets(
                &format!("f(prefix, ...{operand}, middle, ...[0, 1], tail)"),
                Language::JavaScript,
                &[Some(0), None, None, None, None],
                &[0, 1, 1, 2, 4],
            );
        }
    }

    #[test]
    fn typescript_wrappers_preserve_spread_length() {
        assert_offsets(
            "f(...((([0] as G)!) satisfies G), x)",
            Language::TypeScript,
            &[None, Some(1)],
            &[0, 1],
        );
        assert_offsets(
            "f(...(<G>[0]), x)",
            Language::TypeScript,
            &[None, Some(1)],
            &[0, 1],
        );
    }

    #[test]
    fn wrapped_instantiation_preserves_spread_length() {
        assert_offsets(
            "f(...((([0] as G)<number>)!), x)",
            Language::Tsx,
            &[None, Some(1)],
            &[0, 1],
        );
    }

    #[test]
    fn other_languages_keep_positional_slots() {
        let source = "f(a, *rest, b)";
        let caller = ParsedFile::parse("fixture", source, Language::Python).unwrap();
        let args = caller.call_argument_texts_and_spans_at(0, "f");
        assert_eq!(args.len(), 3, "{args:?}");
        let actual = offsets(&caller, &args);
        assert_eq!(actual.slot, [Some(0), Some(1), Some(2)]);
        assert_eq!(actual.least, [0, 1, 2]);
    }
}

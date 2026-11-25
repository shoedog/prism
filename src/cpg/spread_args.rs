//! Runtime positional offsets for Step 5b argument binding.

use crate::ast::ParsedFile;
use crate::call_graph::CallSite;
use std::ops::Range;

type Argument = (String, Range<usize>);

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
    let array = caller.spread_argument_operand(span)?;
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

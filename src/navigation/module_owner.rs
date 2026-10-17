//! Public navigation represents module execution as a statement, never a function.
use super::types::{Location, SymbolRef};
use super::IndexedIncomingCall;
use crate::call_graph::FunctionId;

pub(super) fn function_symbol(
    file: &str,
    name: &str,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
) -> SymbolRef {
    if crate::anon_owner::is_module_owner_name(name) {
        SymbolRef::Statement {
            file: file.into(),
            line: start_line,
            kind: "module-level".into(),
            start_byte,
            end_byte,
            ordinal: 0,
        }
    } else {
        SymbolRef::Function {
            file: file.into(),
            name: name.into(),
            start_line,
            end_line,
            start_byte,
            end_byte,
            ordinal: 0,
        }
    }
}

pub(super) fn caller_evidence(
    caller: &FunctionId,
    edge: &IndexedIncomingCall,
    bytes: (usize, usize),
) -> (SymbolRef, Location) {
    let (start_line, end_line, start_byte, end_byte) =
        if crate::anon_owner::is_module_owner_name(&caller.name) {
            (
                edge.call_site_line,
                edge.call_site_line,
                edge.start_byte,
                edge.end_byte,
            )
        } else {
            (caller.start_line, caller.end_line, bytes.0, bytes.1)
        };
    (
        function_symbol(
            &caller.file,
            &caller.name,
            start_line,
            end_line,
            start_byte,
            end_byte,
        ),
        Location {
            file: caller.file.clone(),
            start_line,
            end_line,
            start_byte,
            end_byte,
        },
    )
}

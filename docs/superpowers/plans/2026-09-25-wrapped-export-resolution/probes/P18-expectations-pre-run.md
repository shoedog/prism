# r2 control expectations (written BEFORE running r2 binary; 2026-09-25)
Base binary (60755871) expectations: every C26..C46 app-side site drops UnknownName (no wrapped facts on base);
C31/C32 JSX-free sites drop UnknownName; C32 `new` is not a call site at all (queries.rs has no new_expression).
r2 proto mode (a) [default]:
 C26, C27, C38, C44: Exact import_member -> lib:Island (C44: lib.jsx Island)
 C46 (write to an unrelated member `customThing`): REFUSED react_object_unaccounted (member write refused conservatively; recall cost accepted)
 C28, C33, C34, C35, C36, C37, C40, C41, C42, C43, C45: drop; reason react_object_unaccounted
 C29: drop; reason import_parse_recovery
 C30: drop; reason comparator_line_collision
 C31: App/App2 sites drop with WrappedExportNonJsx
 C32: App2 JSX Exact; no row for `new`
 C39: drop (star-barrel span conflict), js_export_barrel_conflicts >= 1
 C01-C25: identical to P6a v2 summary (C02,C11,C14,C15,C17 Exact; rest as before)
r2 proto mode (b): member forms (C15,C26,C27,C38,C44) drop with reason member_form_disabled; named forms as mode (a).
# addendum (written before running C47-C52)
 C47 React.version++ (w1), C48 for (React.version of …) (w3 for_in), C49 { React } (K6), C50 export { React } (K5),
 C51 escaped spelling React (K3), C52 `with` in .jsx (K1): all drop, reason react_object_unaccounted (mode a)

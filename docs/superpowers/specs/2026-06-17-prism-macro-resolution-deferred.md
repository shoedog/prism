# Prism Macro Resolution Deferred

## Status

Deferred. Macro resolution is out of scope for the Phase-1 F3 win and merits its
own analysis increment.

## Why Deferred

Prism currently drops macro invocations: `call_function_name` returns `None` for
`macro_invocation`, so there is no `m!()` call site and therefore no wrong-edge
risk from macro calls today.

The F3 win, covering free-function and `use` call narrowing plus module
dependencies, does not need macro resolution.

## Seams Already In Place

The future macro-resolution increment is additive rather than a refactor because
these seams already exist:

1. `CallSite.kind: Call | MacroInvocation` provides the call-site discriminator.
2. `NS_MACRO` is part of `RustPolicy`'s three-namespace model.
3. The populator's `MacroWildcard` representation can poison ranges for macro
   name introduction.
4. `graph_callable_edge` routes by `CallSite.kind`; a `MacroInvocation` site
   already resolves in `NS_MACRO`.

## Future Increment

The future increment is three localized changes:

1. Flip `call_function_name`'s `macro_invocation` branch back on by extracting
   the macro name into a `CallSite` tagged `MacroInvocation`. This reintroduces
   macro call sites, so it must land with the next two changes and be Tier-A
   gated because it changes navigation and call-graph output.
2. Upgrade the populator's macro handling from wildcard poison to resolvable
   `NS_MACRO` definition bindings, including `macro_rules!`, `#[macro_export]`,
   and textual `vis_extents` order.
3. Leave the consumer unchanged: the seam routes `MacroInvocation` to
   `NS_MACRO`.

## Recall-Safety Invariant

A `MacroInvocation` site resolves only in `NS_MACRO`. If it is unresolvable or
unexpanded, resolution returns `Poisoned` or otherwise falls through. It never
emits a Value `fn`-of-the-same-name edge.

## Open Analysis Questions

- Macro hygiene and textual scoping precision.
- `macro_rules!` shadowing order.
- `#[macro_use]` and `#[macro_export]` cross-crate behavior.
- Proc and attribute macros, which likely stay poison.
- Whether macro call sites belong in navigation callers/callees as an output and
  UX decision.

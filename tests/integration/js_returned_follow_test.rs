//! Slice V: a value another in-repo function returned is followed. Each row builds a small repo
//! and pins the full record set at the site (origin, target file/name/span, kind, grade). The
//! returned key is spelled differently from the function it holds, with a same-named decoy
//! elsewhere, whenever the old by-name search could have picked the decoy.
use super::js_alias_follow_fallback_test::records_of;
use super::js_alias_follow_test::graph_n;
use prism::call_graph::CallGraph;

/// The records of `caller_file` whose callee text is `callee`.
pub(super) fn rec(cg: &CallGraph, caller_file: &str, callee: &str) -> Vec<String> {
    let prefix = format!(" {callee} ");
    records_of(cg, caller_file)
        .into_iter()
        .filter(|r| r.contains(&prefix))
        .collect()
}

const DECOY: &str = "export function save() {\n  return 2;\n}\n\
    export function Panel() {\n  return null;\n}\nexport function run() {\n  return 3;\n}\n";

const HOOKS: &str = "export function useEditor() {\n  function doSave() {\n    return 1;\n  }\n  \
    return { save: doSave };\n}\n";

const CALLER: &str = "import { useEditor } from './hooks';\nexport function run() {\n  \
    const { save } = useEditor();\n  return save();\n}\n";

#[test]
fn a_renamed_key_binds_the_returned_function_not_the_same_named_decoy() {
    for ext in ["js", "ts"] {
        let (hooks, caller, decoy) = (
            format!("hooks.{ext}"),
            format!("caller.{ext}"),
            format!("decoy.{ext}"),
        );
        let cg = graph_n(&[(&hooks, HOOKS), (&caller, CALLER), (&decoy, DECOY)]);
        assert_eq!(
            rec(&cg, &caller, "save"),
            [format!(
                "L4 save Source: {hooks}:doSave@2-4 local_def exact"
            )],
            ".{ext}"
        );
    }
}

#[test]
fn a_key_spelled_like_its_function_binds_through_the_route_not_the_name_search() {
    let hooks = "export function useEditor() {\n  function save() {\n    return 1;\n  }\n  \
        return { save };\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", hooks),
        ("caller.ts", CALLER),
        ("decoy.ts", DECOY),
    ]);
    // The base's name search found one of two `save` functions by name only (free_multi,
    // name_only); the route proves the returned one.
    assert_eq!(
        rec(&cg, "caller.ts", "save"),
        ["L4 save Source: hooks.ts:save@2-4 local_def exact"]
    );
}

#[test]
fn a_default_import_hook_and_a_hook_through_a_barrel_bind_the_same_function() {
    let default_hooks =
        "export default function useEditor() {\n  function doSave() {\n    return 1;\n  }\n  \
        return { save: doSave };\n}\n";
    let default_caller = "import useEditor from './hooks';\nexport function run() {\n  \
        const { save } = useEditor();\n  return save();\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", default_hooks),
        ("caller.ts", default_caller),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "save"),
        ["L4 save Source: hooks.ts:doSave@2-4 local_def exact"]
    );

    let barrel = "export { useEditor } from './hooks';\n";
    let via_barrel = "import { useEditor } from './index';\nexport function run() {\n  \
        const { save } = useEditor();\n  return save();\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", HOOKS),
        ("index.ts", barrel),
        ("caller.ts", via_barrel),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "save"),
        ["L4 save Source: hooks.ts:doSave@2-4 local_def exact"]
    );
}

#[test]
fn a_hook_in_the_same_file_binds_past_a_same_named_module_function() {
    let src =
        "function save() {\n  return 0;\n}\nfunction useEditor() {\n  function doSave() {\n    \
        return 1;\n  }\n  return { save: doSave };\n}\nexport function run() {\n  \
        const { save } = useEditor();\n  return save();\n}\n";
    for ext in ["js", "ts"] {
        let file = format!("app.{ext}");
        let cg = graph_n(&[(&file, src)]);
        assert_eq!(
            rec(&cg, &file, "save"),
            [format!(
                "L12 save Source: {file}:doSave@5-7 local_def exact"
            )],
            ".{ext}"
        );
    }
}

#[test]
fn the_destructure_and_the_call_in_one_function_are_followed() {
    // The base's lexical guard drops a call whose binding is in the same function; the route
    // runs before it.
    let src = "export function useEditor() {\n  function doSave() {\n    return 1;\n  }\n  \
        return { save: doSave };\n}\nexport function run() {\n  const { save } = useEditor();\n  \
        save();\n  return 0;\n}\n";
    let cg = graph_n(&[("app.ts", src)]);
    assert_eq!(
        rec(&cg, "app.ts", "save"),
        ["L9 save Source: app.ts:doSave@2-4 local_def exact"]
    );
}

#[test]
fn whole_value_array_slot_and_renamed_binding_forms_are_followed() {
    let hooks = "export function useTools() {\n  function open() {\n    return 1;\n  }\n  \
        function close() {\n    return 2;\n  }\n  return [open, close] as const;\n}\n\
        export function useHandler() {\n  function handle() {\n    return 3;\n  }\n  \
        return handle;\n}\n";
    let caller = "import { useTools, useHandler } from './hooks';\nexport function run() {\n  \
        const [first, second] = useTools();\n  const h = useHandler();\n  \
        const [, closer] = useTools();\n  return [first(), second(), h(), closer()];\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", hooks),
        ("caller.ts", caller),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "first"),
        ["L6 first Source: hooks.ts:open@2-4 local_def exact"]
    );
    assert_eq!(
        rec(&cg, "caller.ts", "second"),
        ["L6 second Source: hooks.ts:close@5-7 local_def exact"]
    );
    assert_eq!(
        rec(&cg, "caller.ts", "h"),
        ["L6 h Source: hooks.ts:handle@11-13 local_def exact"]
    );
    assert_eq!(
        rec(&cg, "caller.ts", "closer"),
        ["L6 closer Source: hooks.ts:close@5-7 local_def exact"]
    );
}

#[test]
fn a_value_that_is_an_import_in_the_hooks_file_resolves_as_seen_from_that_file() {
    // `./util` exists only relative to lib/hooks.ts; the caller sits one directory up.
    let util = "export function work() {\n  return 1;\n}\n";
    let hooks = "import { work as imp } from './util';\nexport function useThing() {\n  \
        return { run: imp };\n}\n";
    let caller = "import { useThing } from './lib/hooks';\nexport function main() {\n  \
        const { run } = useThing();\n  return run();\n}\n";
    let cg = graph_n(&[
        ("lib/util.ts", util),
        ("lib/hooks.ts", hooks),
        ("caller.ts", caller),
        ("decoy.ts", "export function run() {\n  return 9;\n}\n"),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "run"),
        ["L4 run Source: lib/util.ts:work@1-3 import_member exact"]
    );
}

#[test]
fn a_namespace_member_value_and_an_alias_value_resolve_through_the_alias_routes() {
    let util =
        "export function work() {\n  return 1;\n}\nexport function other() {\n  return 2;\n}\n";
    let hooks =
        "import * as U from './util';\nconst viaAlias = U.other;\nexport function useThing() {\n  \
        return { run: U.work, again: viaAlias };\n}\n";
    let caller = "import { useThing } from './hooks';\nexport function main() {\n  \
        const { run, again } = useThing();\n  return [run(), again()];\n}\n";
    let cg = graph_n(&[
        ("util.ts", util),
        ("hooks.ts", hooks),
        ("caller.ts", caller),
    ]);
    // `U.work` is not an identifier value: no fact for `run`, so the base outcome stands; the
    // alias value `viaAlias` is followed to the namespace member.
    assert_eq!(
        rec(&cg, "caller.ts", "again"),
        ["L4 again Source: util.ts:other@4-6 import_member exact"]
    );
}

#[test]
fn a_hook_reached_through_an_alias_chain_is_followed() {
    let caller =
        "import { useEditor } from './hooks';\nconst H = useEditor;\nexport function run() {\n  \
        const { save } = H();\n  return save();\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", HOOKS),
        ("caller.ts", caller),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "save"),
        ["L5 save Source: hooks.ts:doSave@2-4 local_def exact"]
    );
}

#[test]
fn a_jsx_element_of_a_returned_component_is_followed() {
    let hooks =
        "export function useParts() {\n  function Panel(props: any) {\n    return null;\n  }\n  \
        return { Panel };\n}\n";
    let caller = "import { useParts } from './hooks';\nexport function App() {\n  \
        const { Panel } = useParts();\n  return <Panel />;\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", hooks),
        ("caller.tsx", caller),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        rec(&cg, "caller.tsx", "Panel"),
        ["L4 Panel Source: hooks.ts:Panel@2-4 local_def exact"]
    );
}

#[test]
fn a_wrapped_returned_component_binds_from_jsx_only() {
    let hooks = "import { memo } from 'react';\nconst Panel = memo(() => null);\n\
        export function useParts() {\n  return { Panel };\n}\n";
    let caller = "import { useParts } from './hooks';\nexport function App() {\n  \
        const { Panel } = useParts();\n  Panel();\n  return <Panel />;\n}\n";
    let cg = graph_n(&[("hooks.tsx", hooks), ("caller.tsx", caller)]);
    let got = rec(&cg, "caller.tsx", "Panel");
    assert!(
        got.contains(&"L5 Panel Source: hooks.tsx:Panel@2-2 local_def exact".to_string()),
        "{got:?}"
    );
    assert!(
        !got.iter()
            .any(|r| r.starts_with("L4 ") && r.contains("exact")),
        "a non-JSX call of a wrapped component must not bind: {got:?}"
    );
}

#[test]
fn a_use_callback_value_binds_the_inline_function() {
    let hooks = "import { useCallback } from 'react';\nexport function useThing() {\n  \
        const doIt = useCallback((x: number) => x, []);\n  return { run: doIt };\n}\n";
    let caller = "import { useThing } from './hooks';\nexport function main() {\n  \
        const { run } = useThing();\n  return run(1);\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", hooks),
        ("caller.ts", caller),
        ("decoy.ts", "export function run() {\n  return 9;\n}\n"),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "run"),
        ["L4 run Source: hooks.ts:doIt@3-3 local_def exact"]
    );
}

#[test]
fn an_object_method_and_an_inline_arrow_property_bind_their_own_function() {
    let hooks = "export function useThing() {\n  return {\n    viaMethod() {\n      return 1;\n    },\n    \
        viaArrow: () => 2,\n  };\n}\n";
    let caller = "import { useThing } from './hooks';\nexport function main() {\n  \
        const { viaMethod, viaArrow: go } = useThing();\n  return [viaMethod(), go()];\n}\n";
    let cg = graph_n(&[("hooks.ts", hooks), ("caller.ts", caller)]);
    assert_eq!(
        rec(&cg, "caller.ts", "viaMethod"),
        ["L4 viaMethod Source: hooks.ts:viaMethod@3-5 local_def exact"]
    );
    assert_eq!(
        rec(&cg, "caller.ts", "go"),
        ["L4 go Source: hooks.ts:viaArrow@6-6 local_def exact"]
    );
}

#[test]
fn the_decoy_with_the_keys_name_is_not_bound_when_the_fact_is_present() {
    // `save` is returned under the key `run` only; the destructure of `decoy` names a key the
    // hook does not return. No fact for it: the site is whatever the base gives.
    let hooks = "export function useEditor() {\n  function doSave() {\n    return 1;\n  }\n  \
        return { run: doSave };\n}\n";
    let caller = "import { useEditor } from './hooks';\nexport function main() {\n  \
        const { run, save } = useEditor();\n  return [run(), save()];\n}\n";
    let cg = graph_n(&[
        ("hooks.ts", hooks),
        ("caller.ts", caller),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        rec(&cg, "caller.ts", "run"),
        ["L4 run Source: hooks.ts:doSave@2-4 local_def exact"]
    );
    assert!(
        !rec(&cg, "caller.ts", "save")
            .iter()
            .any(|r| r.contains("doSave")),
        "the key `save` is not returned"
    );
}

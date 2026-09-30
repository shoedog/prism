//! S1b-3 unit row (SPEC §7 C-40, C172, carry-forward 8): B1 containment against the delimited
//! child, not the whole sealer, for a body/class-body error.
use super::JsBindingCache;
use crate::ast::js_binding::{JsBinding, JsTerminal};
use crate::ast::js_binding_helper_tests::{node_at, parse};

#[test]
fn c40_c172_declaration_export_site_is_outside_its_own_sealed_body() {
    // SPEC §3.8 (6), carry-forward 8: the declaration-export site (the declaration node, which
    // contains its own body) is outside a sealed body: both the declaration route and a
    // re-export elsewhere verify. Mutant C-M32 (containment tested against the whole sealer
    // instead of the delimited body) would wrongly refuse both.
    let src = "export function f() {\n  let x = ;\n}\nexport { f as g };\n";
    let want = JsBinding::Callable(JsTerminal {
        local: "f".to_string(),
        start_line: 1,
        end_line: 3,
        wrapped: false,
    });
    for path in ["a.js", "a.tsx"] {
        let p = parse(path, src);
        assert!(
            p.tree.root_node().has_error(),
            "{path}: fixture should recover"
        );
        for at in ["function f", "export { f as g }"] {
            let mut cache = JsBindingCache::default();
            let got = p.js_ts_module_binding("f", node_at(&p, at), &mut cache);
            assert_eq!(got, want, "{path} at {at:?}");
        }
    }
}

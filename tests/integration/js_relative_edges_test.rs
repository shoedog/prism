//! I1 round-1 fold, item 4: edge-level consequences of resolving more
//! relative specifiers, pinned so the contract is honest. No product
//! behavior is asserted beyond what the language says.
use super::js_relative_extension_test::{graph, rows};

fn barrel_rows(a: &str, b: &str, barrel: &str, extra: &[(&str, &str)]) -> Vec<String> {
    let mut sources = vec![
        ("a.ts", a),
        ("b.ts", b),
        ("barrel.ts", barrel),
        (
            "app.ts",
            "import { f } from './barrel';\nexport function g() { f(); }\n",
        ),
    ];
    sources.extend_from_slice(extra);
    rows(&graph(&sources), "g", "f")
}

const A: &str = "export function f() { return 1; }\n";
const B: &str = "export function f() { return 2; }\n";

#[test]
fn second_star_through_js_suffix_makes_the_name_ambiguous() {
    // main: `./b.js` is unresolved, so the barrel exposes only a.ts:f and the
    // call is `a.ts:f@1-1 Exact/import_member`. The branch resolves the second
    // star to b.ts; two stars exporting different `f` make the name ambiguous,
    // so the call is dropped (as it is for `export * from './b'`).
    let twin = barrel_rows(A, B, "export * from './a';\nexport * from './b';\n", &[]);
    assert_eq!(twin, ["drop UnknownName"]);
    let js = barrel_rows(A, B, "export * from './a';\nexport * from './b.js';\n", &[]);
    assert_eq!(js, twin);
}

#[test]
fn second_star_through_redundant_segments_makes_the_name_ambiguous() {
    // main: `./z/../b` is unresolved (a.ts:f Exact). Branch: ambiguous, dropped.
    let barrel = "export * from './a';\nexport * from './z/../b';\n";
    assert_eq!(barrel_rows(A, B, barrel, &[]), ["drop UnknownName"]);
}

#[test]
fn second_star_through_bare_dot_dot_makes_the_name_ambiguous() {
    // main: `..` is unresolved, so sub/index.ts exposes only its own star.
    // Branch: `..` reaches pkg/index.ts, whose `f` collides with sub/c.ts's.
    let sources = [
        ("pkg/index.ts", "export * from './a';\n"),
        ("pkg/a.ts", A),
        (
            "pkg/sub/index.ts",
            "export * from '..';\nexport * from './c';\n",
        ),
        ("pkg/sub/c.ts", B),
        (
            "pkg/sub/app.ts",
            "import { f } from './index';\nexport function g() { f(); }\n",
        ),
    ];
    assert_eq!(rows(&graph(&sources), "g", "f"), ["drop UnknownName"]);
}

#[test]
fn second_star_forwarding_a_type_only_import_drops_like_the_canonical_spelling() {
    // The correct answer is `a.ts:f` Exact/import_member: `b.ts` forwards a
    // type-only `f`, which is not a value export, so there is no real
    // conflict. The star-conflict check does not see that today, so the
    // canonical `export * from './b'` drops on main and on this branch; the
    // `.js` spelling now inherits that existing limit (main resolved only
    // `./a`, giving `a.ts:f` Exact). Recorded as a follow-up, not fixed here.
    let b = "import type { f } from './types';\nexport { f };\n";
    let types = [("types.ts", "export type f = number;\n")];
    let canonical = barrel_rows(A, b, "export * from './a';\nexport * from './b';\n", &types);
    assert_eq!(canonical, ["drop UnknownName"]);
    let js = barrel_rows(
        A,
        b,
        "export * from './a';\nexport * from './b.js';\n",
        &types,
    );
    assert_eq!(js, canonical);
}

#[test]
fn stars_reaching_the_same_declaration_stay_exact() {
    // Control: b.ts re-exports a.ts's declaration, so both stars agree.
    let b = "export { f } from './a';\n";
    let barrel = "export * from './a';\nexport * from './b.js';\n";
    assert_eq!(
        barrel_rows(A, b, barrel, &[]),
        ["a.ts:f@1-1 Exact/import_member"]
    );
}

#[test]
fn second_star_lacking_the_name_stays_exact() {
    // Control: the newly resolved second module does not export `f`.
    let b = "export function other() { return 2; }\n";
    let barrel = "export * from './a';\nexport * from './b.js';\n";
    assert_eq!(
        barrel_rows(A, b, barrel, &[]),
        ["a.ts:f@1-1 Exact/import_member"]
    );
}

fn props_rows(spec: &str) -> Vec<String> {
    let app = format!(
        "import type {{ Props }} from '{spec}';\nexport function run({{ client }}: Props) {{ client.m(); }}\n"
    );
    let sources = [
        (
            "client.ts",
            "export class Base { m() {} }\nexport class Client extends Base {}\n",
        ),
        (
            "props.ts",
            "import type { Client } from './client';\nexport type Props = { client: Client };\n",
        ),
        ("app.ts", app.as_str()),
    ];
    rows(&graph(&sources), "run", "m")
}

#[test]
fn imported_props_method_inherited_from_a_base_class_matches_the_canonical_spelling() {
    // The receiver route's existing limit: a props type reaches its class but
    // the method lives on a base class, which this route does not follow, so
    // it drops `ExternalReceiver`. main's `./props.js` (unresolved there) gave
    // a NameOnly `r6_single_owner` edge to the base method; the canonical
    // `./props` spelling drops on main too. The `.js` spelling now equals its
    // canonical twin.
    let canonical = props_rows("./props");
    assert_eq!(props_rows("./props.js"), canonical);
    assert_eq!(canonical, ["drop ExternalReceiver"]);
}

fn hop_rows(outer: &str, inner: &str) -> Vec<String> {
    let app = format!("import * as N from '{outer}';\nexport function g() {{ N.f(); }}\n");
    let x = format!("export {{ f }} from '{inner}';\n");
    let sources = [
        ("app.ts", app.as_str()),
        ("x.ts", x.as_str()),
        ("impl/x.ts", "export function f() { return 1; }\n"),
        ("other/x.ts", "export function f() { return 2; }\n"),
    ];
    rows(&graph(&sources), "g", "f")
}

#[test]
fn namespace_proof_through_a_newly_resolved_named_reexport_hop_prunes_the_stem_fallback() {
    // main: the inner `./impl/x.js` hop is unresolved, so there is no namespace
    // proof and the legacy stem fallback returns BOTH
    // `impl/x.ts:f` and `other/x.ts:f` as Exact/import_qualified. The branch
    // resolves the hop, proves the namespace member, and keeps `impl/x.ts:f`
    // alone. The canonical inner hop and the `.js` outer spelling agree.
    let expected = ["impl/x.ts:f@1-1 Exact/import_qualified"];
    assert_eq!(hop_rows("./x", "./impl/x.js"), expected);
    assert_eq!(hop_rows("./x", "./impl/x"), expected);
    assert_eq!(hop_rows("./x.js", "./impl/x"), expected);
}

fn corner_rows(files: &[&str], spec: &str) -> Vec<String> {
    let app = format!("import {{ f }} from '{spec}';\nexport function g() {{ f(); }}\n");
    let mut sources: Vec<(&str, &str)> = files
        .iter()
        .map(|p| (*p, "export function f() { return 1; }\n"))
        .collect();
    sources.push(("app.ts", app.as_str()));
    rows(&graph(&sources), "g", "f")
}

// The tier-policy corners, end to end. Deliberate difference: an existing
// (canonical) candidate list keeps its legacy order; a newly resolved
// (redundant) spelling follows TypeScript's order for a spelled `.js`.
#[test]
fn corner_dotted_ts_beside_plain_ts_follows_typescript_only_when_newly_resolved() {
    let files = ["a/x.js.ts", "a/x.ts"];
    assert_eq!(
        corner_rows(&files, "./a//x.js"),
        ["a/x.ts:f@1-1 Exact/import_member"]
    );
    assert_eq!(
        corner_rows(&files, "./a/x.js"),
        ["a/x.js.ts:f@1-1 Exact/import_member"]
    );
}

#[test]
fn corner_directory_named_js_beside_plain_ts_follows_typescript_only_when_newly_resolved() {
    let files = ["a/x.ts", "a/x.js/index.ts"];
    assert_eq!(
        corner_rows(&files, "./a//x.js"),
        ["a/x.ts:f@1-1 Exact/import_member"]
    );
    assert_eq!(
        corner_rows(&files, "./a/x.js"),
        ["a/x.js/index.ts:f@1-1 Exact/import_member"]
    );
}

#[test]
fn corner_literal_js_beside_dotted_ts_follows_typescript_only_when_newly_resolved() {
    let files = ["a/x.js", "a/x.js.ts"];
    assert_eq!(
        corner_rows(&files, "./a//x.js"),
        ["a/x.js:f@1-1 Exact/import_member"]
    );
    assert_eq!(
        corner_rows(&files, "./a/x.js"),
        ["a/x.js.ts:f@1-1 Exact/import_member"]
    );
}

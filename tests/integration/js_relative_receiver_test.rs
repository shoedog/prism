//! I1 round-1 fold, item 5: receiver consumers (typed parameter and
//! constructor-assigned local through an imported class) agree with the
//! extensionless twin when the class is imported with a `.js` suffix, and the
//! cardinality-one guard still refuses when two candidates exist.
use super::js_relative_extension_test::twin;

const CLIENT: &str = "export class Client { m() {} }\n";
const DECOY: &str = "export class Client { m() {} }\n";

#[test]
fn typed_parameter_class_imported_with_js_suffix_matches_the_twin() {
    let sources = [
        ("pkg/client.ts", CLIENT),
        ("other/client.ts", DECOY),
        (
            "pkg/app.ts",
            "import { Client } from '{S}';\nexport function run(x: Client) { x.m(); }\n",
        ),
    ];
    let (js, plain) = twin(&sources, "./client.js", "./client", "run", "m");
    assert_eq!(plain, ["pkg/client.ts:m@1-1 Exact/typed_param"]);
    assert_eq!(js, plain);
}

#[test]
fn constructor_local_class_imported_with_js_suffix_matches_the_twin() {
    let sources = [
        ("pkg/client.ts", CLIENT),
        ("other/client.ts", DECOY),
        (
            "pkg/app.ts",
            "import { Client } from '{S}';\nexport function run() { const x = new Client(); x.m(); }\n",
        ),
    ];
    let (js, plain) = twin(&sources, "./client.js", "./client", "run", "m");
    assert_eq!(plain, ["pkg/client.ts:m@1-1 Exact/constructor_local"]);
    assert_eq!(js, plain);
}

#[test]
fn two_candidates_refuse_for_the_class_consumer_like_the_twin() {
    // `./c.js` with both c.ts and c.tsx has two candidates, as `./c` does:
    // the cardinality-one class guard refuses in both spellings, so the call
    // falls to the NameOnly single-owner heuristic on both files, never Exact.
    for body in [
        "export function run(x: C) { x.m(); }",
        "export function run() { const x = new C(); x.m(); }",
    ] {
        let app = format!("import {{ C }} from '{{S}}';\n{body}\n");
        let sources = [
            ("pkg/c.ts", "export class C { m() {} }\n"),
            ("pkg/c.tsx", "export class C { m() {} }\n"),
            ("pkg/app.ts", app.as_str()),
        ];
        let (js, plain) = twin(&sources, "./c.js", "./c", "run", "m");
        let expected = [
            "pkg/c.ts:m@1-1 NameOnly/r6_single_owner",
            "pkg/c.tsx:m@1-1 NameOnly/r6_single_owner",
        ];
        assert_eq!(plain, expected, "{body}");
        assert_eq!(js, expected, "{body}");
    }
}

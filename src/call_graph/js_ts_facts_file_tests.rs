//! Full observations through the frozen facts loop and frozen pre-index export helpers.
use super::*;

#[test]
fn nested_file_facts_and_graph_bytes_match_pre_index_oracle() {
    let mut files = generated();
    for (i, ext) in ["js", "ts", "tsx"].into_iter().enumerate() {
        let mut body = "const {shared: leaf, ...tail} = value; return {run: target};".to_string();
        for depth in (0..(8 + i * 2)).rev() {
            body = format!("const local{depth} = shared; consume(function ({{shared, ...rest}}, [first]) {{\nconsume((shared) => {{\n{body}\n}});\n}}); return {{run: target}};");
        }
        let source = format!("import {{remote as shared, untouched}} from './origin';\nexport function target(x) {{ return x; }}\nexport function outer({{shared, ...rest}}, [first]) {{\n{body}\n}}\nconsume(function (shared) {{ consume(() => shared()); }});\nclass C {{ method({{shared}}, [x]) {{ consume(() => shared(x)); }} *gen(shared) {{ yield shared; }} }}\nconsume(function* ({{shared}}) {{ yield shared; }});\nconsume(() => untouched());\n");
        let path = format!("nested{i}.{ext}");
        let p = parsed(&path, &source);
        assert!(!p.tree.root_node().has_error(), "{source}");
        files.insert(path, p);
    }
    for (path, source) in [
        ("origin.ts", "export function remote(x) { return x; } export function untouched() {}"),
        ("write.ts", "import {remote, untouched} from './origin'; remote = other; function inner(untouched) { untouched++; } export function use() { return {run: untouched}; }"),
        ("error.ts", "import {remote} from './origin'; export function okay() { consume(() => remote()); } const = ;"),
        ("types.ts", "import type {T} from './origin'; interface Collision {} function Collision() {} namespace N.Inner {} export class C { method() { consume(() => 1); } }"),
    ] {
        files.insert(path.into(), parsed(path, source));
    }
    let old_files = reparse(&files);
    let expected = ParsedFile::with_legacy_export_facts(|| sequential(old_files.iter()));
    let cold = reparse(&files);
    assert_eq!(extract(cold.iter()), expected);
    assert_eq!(extract(cold.iter()), expected, "warm export facts");
    compare_graphs(&files);
}

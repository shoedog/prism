use super::*;
use prism::slice::{RuleRef, SlicingAlgorithm};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};

const REGISTRY_FILE: &str = "crates/prism-review/src/output/rule_registry.rs";
const SARIF_FILE: &str = "crates/prism-review/src/output/sarif.rs";
const SINK_HIT_FILE: &str = "crates/prism-review/src/algorithms/taint_sinks/mod.rs";

#[test]
fn registry_rows_are_sorted_and_canonical() {
    let rows = rules();
    assert!(rows.windows(2).all(|pair| pair[0].id < pair[1].id));
    assert_eq!(rows.len(), 64);
    for row in rows {
        assert_eq!(row.name, row.id);
        assert_eq!(row.algorithms, ["taint"]);
        assert!(row
            .algorithms
            .iter()
            .all(|a| SlicingAlgorithm::from_str(a).is_some()));
        if let Some(cwe) = row.cwe {
            let digits = cwe.strip_prefix("CWE-").expect(row.id);
            assert!(!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        }
        assert!(
            row.short.ends_with('.') && row.help.ends_with('.'),
            "{}",
            row.id
        );
        for sentence in [row.short, row.help] {
            assert_eq!(sentence, sentence.trim());
            assert!(
                (row.id == "legacy" || !sentence.contains(row.id)) && !sentence.contains("CWE-")
            );
        }
        assert!(std::ptr::eq(lookup(row.id).unwrap(), row));
        let reference = row.rule_ref();
        assert_eq!(reference.id, row.id);
        assert_eq!(reference.cwe.as_deref(), row.cwe);
        assert_eq!(reference.severity, None);
        assert!(check_emitted("taint", &reference).is_ok());
    }
    assert_eq!(lookup(JS_EVAL.id), Some(&JS_EVAL));
    assert_eq!(lookup(LEGACY.id), Some(&LEGACY));
    assert_eq!(
        LEGACY.short,
        "A legacy sink name matched on a tainted line."
    );
    assert_eq!(LEGACY.help, "Inspect the matched name and trace the input to the operation. This detector does not identify a particular operand or weakness.");
    assert_eq!(LEGACY.cwe, None);
    assert!(lookup("JS-eval").is_none());
}

#[test]
fn emitted_reference_must_match_registry_and_algorithm() {
    let mut reference: RuleRef = JS_EVAL.rule_ref();
    assert_eq!(reference.cwe.as_deref(), Some("CWE-95"));
    reference.severity = Some("suggestion".into());
    assert!(check_emitted("taint", &reference).is_ok());
    assert!(check_emitted("primitive", &reference).is_err());
    assert!(check_emitted("TAINT", &reference).is_err());
    reference.cwe = Some("CWE-89".into());
    assert!(check_emitted("taint", &reference).is_err());
    reference.cwe = None;
    assert!(check_emitted("taint", &reference).is_err());
    reference.id = "nonexistent".into();
    assert!(check_emitted("taint", &reference).is_err());

    let mut neutral = TAINT_REDIRECT_NAME.rule_ref();
    assert_eq!(neutral.cwe, None);
    assert!(check_emitted("taint", &neutral).is_ok());
    neutral.cwe = Some("CWE-601".into());
    assert!(check_emitted("taint", &neutral).is_err());
}

#[test]
fn scanner_allows_declarations_and_test_only_construction() {
    let source = r#"
        pub struct RuleRef { id: String }
        use prism::slice::{RuleRef as Ref};
        fn output(r: Ref) -> prism::slice::RuleRef {
            // RuleRef { id: compute() }
            let _ = "RuleRef::new(compute()) and SinkHit { }";
            let _ = JS_EVAL.rule_ref();
            let Ref { id } = r;
            r
        }
        #[cfg(all(test, feature = "extra"))]
        // A comment between the attribute and the item.
        mod fixtures { fn f() { SinkHit { id: compute() }; } }
        #[cfg(test)]
        mod external_fixtures;
    "#;
    let scan = scan_source("control.rs".as_ref(), source);
    assert!(scan.violations.is_empty(), "{:?}", scan.violations);
    assert!(scan.modules.is_empty());
}

#[test]
fn scanner_rejects_qualified_alias_and_associated_construction_in_sarif() {
    let source = r#"
        use prism::slice::RuleRef as ImportedRule;
        fn f() {
            let _ = ::prism::slice::r#RuleRef { id: compute() };
            let _ = ImportedRule { id: compute() };
            let _ = prism::slice::RuleRef::new(compute());
            let _ = (ImportedRule::new)(compute());
            {
                use crate::algorithms::taint_sinks::{SinkHit as Hit};
                let _ = Hit { id: compute() };
                let _ = crate::algorithms::taint_sinks::SinkHit { id: compute() };
            }
        }
        #[cfg(any(test, feature = "extra"))]
        fn feature_or_test() { RuleRef { id: compute() }; }
    "#;
    let scan = scan_source(SARIF_FILE.as_ref(), source);
    assert_eq!(scan.violations.len(), 7, "{:?}", scan.violations);
}

#[test]
fn scanner_exempts_only_the_maker_and_sink_hit_definition_file() {
    let source = r#"
        impl RuleInfo {
            fn rule_ref(&'static self) -> RuleRef { RuleRef { id: self.id.into() } }
            fn other(&self) -> RuleRef { RuleRef { id: self.id.into() } }
        }
        impl RuleRef {
            fn indirect() -> Self { Self { id: compute() } }
        }
        fn sink() -> SinkHit { SinkHit { id: compute() } }
    "#;
    for file in [REGISTRY_FILE, SINK_HIT_FILE] {
        let scan = scan_source(file.as_ref(), source);
        assert_eq!(scan.violations.len(), 3, "{:?}", scan.violations);
    }
}

#[test]
fn production_construction_uses_registry_maker() {
    let violations = production_violations();
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[derive(Default)]
struct Scan {
    violations: Vec<String>,
    modules: Vec<(PathBuf, PathBuf)>,
}

fn production_violations() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pending: Vec<(PathBuf, PathBuf)> = [
        "src/lib.rs",
        "crates/prism-review/src/lib.rs",
        "crates/prism-review/src/main.rs",
    ]
    .map(|file| {
        (
            PathBuf::from(file),
            Path::new(file).parent().unwrap().into(),
        )
    })
    .into();
    let mut seen = HashSet::new();
    let mut violations = Vec::new();
    while let Some((path, module_dir)) = pending.pop() {
        let file = if path.extension().is_some() {
            path
        } else if root.join(path.with_extension("rs")).is_file() {
            path.with_extension("rs")
        } else {
            path.join("mod.rs")
        };
        if !seen.insert(file.clone()) {
            continue;
        }
        let source = fs::read_to_string(root.join(&file))
            .unwrap_or_else(|err| panic!("{}: {err}", file.display()));
        let scan = scan_module(&file, &module_dir, &source);
        pending.extend(scan.modules);
        violations.extend(scan.violations);
    }
    assert!(seen.contains(Path::new(SARIF_FILE)));
    violations
}

fn scan_source(file: &Path, source: &str) -> Scan {
    scan_module(file, file.parent().unwrap(), source)
}

fn scan_module(file: &Path, module_dir: &Path, source: &str) -> Scan {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let mut scan = Scan::default();
    Scanner {
        file,
        source,
        result: &mut scan,
    }
    .visit(
        tree.root_node(),
        module_dir,
        file.parent().unwrap(),
        &HashMap::new(),
        false,
    );
    scan
}

fn named(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    (0..node.named_child_count()).filter_map(move |index| node.named_child(index))
}

fn spelling<'s>(node: Node<'_>, source: &'s str) -> &'s str {
    &source[node.byte_range()]
}

fn path_leaf<'s>(path: Node<'_>, source: &'s str) -> &'s str {
    let leaf = path.child_by_field_name("name").unwrap_or(path);
    spelling(leaf, source).trim_start_matches("r#")
}

fn cfg_args(group: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = group.walk();
    group
        .children(&mut cursor)
        .skip(1)
        .take(group.child_count() - 2)
        .filter(|n| !n.is_extra())
        .collect()
}

// Unknown feature and target predicates can still enable production code.
fn cfg_without_test(tokens: &[Node<'_>], source: &str) -> Option<bool> {
    match tokens {
        [name] if spelling(*name, source) == "test" => Some(false),
        [op, group] if group.kind() == "token_tree" => {
            let args = cfg_args(*group);
            let values: Vec<_> = args
                .split(|n| n.kind() == ",")
                .filter(|p| !p.is_empty())
                .map(|p| cfg_without_test(p, source))
                .collect();
            match spelling(*op, source) {
                "not" => values.first().copied().flatten().map(|value| !value),
                "all" if values.contains(&Some(false)) => Some(false),
                "all" if values.iter().all(|&v| v == Some(true)) => Some(true),
                "any" if values.contains(&Some(true)) => Some(true),
                "any" if values.iter().all(|&v| v == Some(false)) => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

fn test_only(attribute: Node<'_>, source: &str) -> bool {
    match spelling(attribute.named_child(0).unwrap(), source) {
        "test" => true,
        "cfg" => attribute
            .child_by_field_name("arguments")
            .is_some_and(|group| cfg_without_test(&cfg_args(group), source) == Some(false)),
        _ => false,
    }
}

fn import_aliases<'s>(node: Node<'_>, source: &'s str, aliases: &mut HashMap<&'s str, &'s str>) {
    match node.kind() {
        "use_as_clause" => {
            let alias = node.child_by_field_name("alias").unwrap();
            let path = node.child_by_field_name("path").unwrap();
            aliases.insert(path_leaf(alias, source), path_leaf(path, source));
        }
        "use_declaration" | "scoped_use_list" | "use_list" => {
            for child in named(node) {
                import_aliases(child, source, aliases);
            }
        }
        _ => {}
    }
}

struct Scanner<'s, 'r> {
    file: &'s Path,
    source: &'s str,
    result: &'r mut Scan,
}

impl<'s> Scanner<'s, '_> {
    fn visit(
        &mut self,
        node: Node<'_>,
        module_dir: &Path,
        path_dir: &Path,
        aliases: &HashMap<&'s str, &'s str>,
        in_maker: bool,
    ) {
        let source = self.source;
        let in_maker = in_maker || (self.file == Path::new(REGISTRY_FILE) && maker(node, source));
        self.check_expression(node, aliases, in_maker);
        let mut attributes = Vec::new();
        let mut children = Vec::new();
        let mut local = None;
        if node.kind() == "impl_item" {
            let ty = node.child_by_field_name("type").unwrap();
            local
                .get_or_insert_with(|| aliases.clone())
                .insert("Self", path_leaf(ty, source));
        }
        for child in named(node).filter(|n| !n.is_extra()) {
            match child.kind() {
                "inner_attribute_item" if test_only(child.named_child(0).unwrap(), source) => {
                    return
                }
                "attribute_item" => attributes.push(child.named_child(0).unwrap()),
                _ => {
                    if !attributes.iter().any(|&a| test_only(a, source)) {
                        if child.kind() == "use_declaration" {
                            import_aliases(
                                child,
                                source,
                                local.get_or_insert_with(|| aliases.clone()),
                            );
                        } else {
                            let path = attributes.iter().find_map(|a| {
                                (spelling(a.named_child(0).unwrap(), source) == "path").then(|| {
                                    spelling(a.child_by_field_name("value").unwrap(), source)
                                        .trim_matches('"')
                                })
                            });
                            children.push((child, path));
                        }
                    }
                    attributes.clear();
                }
            }
        }
        let aliases = local.as_ref().unwrap_or(aliases);
        for (child, path) in children {
            if child.kind() != "mod_item" {
                self.visit(child, module_dir, path_dir, aliases, in_maker);
                continue;
            }
            let name = spelling(child.child_by_field_name("name").unwrap(), source);
            if let Some(body) = child.child_by_field_name("body") {
                let dir = path.map_or_else(|| module_dir.join(name), |path| path_dir.join(path));
                self.visit(body, &dir, &dir, &HashMap::new(), false);
            } else if let Some(path) = path {
                let file = path_dir.join(path);
                let dir = file.parent().unwrap().into();
                self.result.modules.push((file, dir));
            } else {
                let dir = module_dir.join(name);
                self.result.modules.push((dir.clone(), dir));
            }
        }
    }

    fn check_expression(&mut self, node: Node<'_>, aliases: &HashMap<&str, &str>, in_maker: bool) {
        let (owner, literal) = match node.kind() {
            "struct_expression" => (node.child_by_field_name("name"), true),
            "call_expression" => {
                let mut callee = node.child_by_field_name("function").unwrap();
                loop {
                    callee = match callee.kind() {
                        "generic_function" => callee.child_by_field_name("function").unwrap(),
                        "parenthesized_expression" => callee.named_child(0).unwrap(),
                        _ => break,
                    };
                }
                let owner = (callee.kind() == "scoped_identifier")
                    .then(|| callee.child_by_field_name("path"))
                    .flatten();
                (owner, false)
            }
            _ => return,
        };
        let Some(mut owner) = owner else { return };
        if owner.kind() == "bracketed_type" {
            owner = owner.named_child(0).unwrap();
        }
        let mut name = path_leaf(owner, self.source);
        for _ in 0..aliases.len() {
            let Some(&target) = aliases.get(name) else {
                break;
            };
            name = target;
        }
        if (name == "RuleRef" && !in_maker)
            || (literal && name == "SinkHit" && self.file != Path::new(SINK_HIT_FILE))
        {
            self.result.violations.push(format!(
                "{}:{}: construction of {name}",
                self.file.display(),
                node.start_position().row + 1,
            ));
        }
    }
}

fn maker(node: Node<'_>, source: &str) -> bool {
    node.kind() == "function_item"
        && node
            .child_by_field_name("name")
            .is_some_and(|n| spelling(n, source) == "rule_ref")
        && node.parent().and_then(|n| n.parent()).is_some_and(|n| {
            n.kind() == "impl_item"
                && n.child_by_field_name("trait").is_none()
                && n.child_by_field_name("type")
                    .is_some_and(|ty| spelling(ty, source) == "RuleInfo")
        })
}

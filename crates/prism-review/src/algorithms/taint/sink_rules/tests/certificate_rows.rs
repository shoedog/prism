use super::*;
use crate::algorithms::taint::sink_rules::certificate::{certify, Table};
use std::collections::BTreeMap;

const TSV: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/taint_sink_rules/certificate_rows.tsv"
));

struct Row<'a> {
    id: &'a str,
    language: Language,
    source: &'a str,
    operation: &'a str,
    target: &'a str,
    operand: &'a str,
    reached: Option<&'a str>,
    role: Role<'a>,
    accept: bool,
    reason: &'a str,
    kind: &'a str,
}

impl<'a> Row<'a> {
    fn read(line: &'a str) -> Result<Self, String> {
        let fields: Vec<_> = line.split('\t').collect();
        let [id, language, source, operation, target, operand, role, expect, reason, kind, note, optional @ ..] =
            fields.as_slice()
        else {
            return Err(format!(
                "expected at least 11 fields, found {}",
                fields.len()
            ));
        };
        let reached = match optional {
            [] => None,
            [selector] if !selector.is_empty() => Some(*selector),
            _ => return Err("expected at most one nonempty reached selector".into()),
        };
        let language = match *language {
            "python" => Language::Python,
            "javascript" => Language::JavaScript,
            "typescript" => Language::TypeScript,
            "tsx" => Language::Tsx,
            "go" => Language::Go,
            "java" => Language::Java,
            "c" => Language::C,
            "cpp" => Language::Cpp,
            "rust" => Language::Rust,
            "lua" => Language::Lua,
            "bash" => Language::Bash,
            "hcl" => Language::Terraform,
            _ => return Err(format!("unknown language {language}")),
        };
        let role = match *role {
            "receiver" => Role::ReceiverValue,
            "assign" => Role::AssignmentValue,
            "attr" => Role::AttributeValue,
            ordinal if ordinal.starts_with("arg:") => Role::Argument(
                ordinal[4..]
                    .parse()
                    .map_err(|_| format!("invalid role {role}"))?,
            ),
            name if name.starts_with("kw:") && name.len() > 3 => Role::KeywordArgument(&name[3..]),
            _ => return Err(format!("invalid role {role}")),
        };
        match (*kind, *expect, reason.is_empty()) {
            ("accept_cell" | "adapter_obligation", "accept", true)
            | ("near_negative" | "pass_wrong" | "over_refusal", "refuse", false)
            | ("control_owner", "refuse", true)
                if !note.is_empty() => {}
            // Known limit under the DESIGN input contract (valid source, default build): the
            // row records today's outcome (accept) for a shape that is not a valid program.
            ("known_limit", "accept", true) if note.starts_with("KNOWN LIMIT") => {}
            _ => {
                return Err(format!(
                    "inconsistent kind, expectation, reason or note for {id}"
                ))
            }
        }
        if reached.is_some() && *expect == "refuse" && *reason != "operand_comment" {
            return Err(format!(
                "invalid refusal reason for reached selector in {id}"
            ));
        }
        Ok(Self {
            id,
            language,
            source,
            operation,
            target,
            operand,
            reached,
            role,
            accept: matches!(*expect, "accept"),
            reason,
            kind,
        })
    }

    fn run(&self, assertion: Assertion) -> Result<(), String> {
        let source = unescape(self.source);
        let parsed = ParsedFile::parse(self.id, &source, self.language)
            .map_err(|err| format!("parse failed: {err}"))?;
        let root = parsed.tree.root_node();
        if root.has_error() {
            return Err(format!("parse error: {}", root.to_sexp()));
        }
        if self.reason == "parse_recovery" {
            // A foreign operation fails the parse-identity check even when both trees are clean.
            let copy = ParsedFile::parse(self.id, &source, self.language)
                .map_err(|err| format!("second parse failed: {err}"))?;
            let copy_root = copy.tree.root_node();
            if copy_root.has_error() {
                return Err(format!("second parse error: {}", copy_root.to_sexp()));
            }
            return self.check(&parsed, copy_root, assertion);
        }
        self.check(&parsed, root, assertion)
    }

    fn check(
        &self,
        parsed: &ParsedFile,
        root: Node<'_>,
        assertion: Assertion,
    ) -> Result<(), String> {
        let operation = fixture_node(parsed, root, self.operation)?;
        let target = fixture_node(parsed, operation, self.target)?;
        // Owner controls can nominate a same-spelling node from the enclosing callable.
        let operand_scope = if self.kind == "control_owner" {
            root
        } else {
            operation
        };
        let operand = fixture_node(parsed, operand_scope, self.operand)?;
        if self.kind == "control_owner" {
            let expected_owner = owner(parsed, operation);
            if owner(parsed, target) == expected_owner && owner(parsed, operand) == expected_owner {
                return Err("owner control has no cross-owner node".into());
            }
        }

        match assertion {
            Assertion::Module => {
                if self.kind == "control_owner" {
                    return Ok(());
                }
                // An explicit reached selector tests the receipt after a valid binding.
                let expected = if self.accept || self.reached.is_some() {
                    Ok(())
                } else {
                    Err((reason_table(self.reason)?, self.reason))
                };
                let actual = certify(parsed, operation, target, operand, self.role)
                    .map_err(|(table, reason)| (table, reason.as_str()));
                if actual != expected {
                    return Err(format!(
                        "certify expected {expected:?}, got {actual:?} ({} / {} / {})",
                        operation.kind(),
                        target.kind(),
                        operand.kind()
                    ));
                }
            }
            Assertion::EndToEnd => {
                let reached = self
                    .reached
                    .map(|selector| fixture_node(parsed, operand, selector))
                    .transpose()?
                    .unwrap_or(operand);
                let line = reached.start_position().row + 1;
                let bound = Bound::from_binder(operation, target, operand, self.role, FLAT);
                // The synthetic path nominates the exact read. Literal fixtures may have no
                // corresponding variable definition.
                let definition = fixture_node(parsed, root, "x")
                    .ok()
                    .filter(|node| node.end_byte() <= operation.start_byte())
                    .unwrap_or(operand);
                let path = path_to(parsed, definition, reached);
                let witness = receipt(parsed, line, bound, reached, FLAT, destination(&path));
                let mut refusals = Refusals::default();
                let rules = super::super::collect_rules(
                    parsed,
                    line,
                    &admitted(parsed, &[line]),
                    [witness],
                    &mut refusals,
                    select_eval,
                );
                // The module assertions still check every table in the TSV.
                let in_scope = matches!(
                    self.language,
                    Language::JavaScript
                        | Language::TypeScript
                        | Language::Tsx
                        | Language::Python
                        | Language::Go
                );
                let accepts = in_scope && self.accept;
                if rules.len() != usize::from(accepts) {
                    return Err(format!(
                        "expected {}, earned {} receipt(s)",
                        if accepts { "accept" } else { "refuse" },
                        rules.len()
                    ));
                }
                // Cross-owner and foreign-tree claims stop before the tables.
                let callable = owner(parsed, operation);
                let same_owner =
                    owner(parsed, target) == callable && owner(parsed, operand) == callable;
                let expected = (in_scope
                    && same_owner
                    && !self.reason.is_empty()
                    && self.reason != "parse_recovery")
                    .then_some((self.reason, 1));
                let actual = refusals.iter().collect::<Vec<_>>();
                if actual.as_slice() != expected.as_slice() {
                    return Err(format!(
                        "expected refusal count {expected:?}, got {actual:?}"
                    ));
                }
            }
        }
        Ok(())
    }
}

fn reason_table(reason: &str) -> Result<Table, String> {
    Ok(match reason {
        "op_kind_unlisted"
        | "op_context_non_runtime"
        | "op_special_form"
        | "op_arguments_unlisted"
        | "op_type_arguments"
        | "attribute_spread"
        | "parse_recovery" => Table::Operation,
        "callee_kind_unlisted"
        | "callee_target_unlisted"
        | "callee_type_unlisted"
        | "subscript_key_not_plain_literal"
        | "subscript_c_family"
        | "member_pointer_operator" => Table::Callee,
        "operand_kind_unlisted"
        | "operand_not_direct_child"
        | "operand_spread"
        | "operand_label"
        | "operand_comment"
        | "operand_attribute"
        | "operand_macro_block"
        | "role_not_bound"
        | "slot_mismatch"
        | "parallel_assignment"
        | "attribute_overwritten" => Table::Operand,
        _ => return Err(format!("unknown reason {reason}")),
    })
}

fn unescape(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => decoded.push('\n'),
            Some('t') => decoded.push('\t'),
            Some('\\') => decoded.push('\\'),
            other => {
                decoded.push('\\');
                decoded.extend(other);
            }
        }
    }
    decoded
}

// Ignore statement and list containers that can share the selected node's exact range.
// Other same-range nodes are selectable in preorder, for example Bash command_name/word.
fn fixture_node<'a>(
    parsed: &ParsedFile,
    scope: Node<'a>,
    selector: &str,
) -> Result<Node<'a>, String> {
    let (text, index) = selector
        .rsplit_once('#')
        .and_then(|(text, index)| Some((text, index.parse::<usize>().ok()?)))
        .unwrap_or((selector, 1));
    let text = unescape(text);
    let mut cursor = scope.walk();
    let mut matches = 0;
    loop {
        let candidate = cursor.node();
        if candidate.is_named()
            && !matches!(
                candidate.kind(),
                "block"
                    | "body"
                    | "expression_statement"
                    | "statement_list"
                    | "function_arguments"
                    | "expression_list"
                    | "variable_list"
            )
            && !(scope == parsed.tree.root_node() && candidate.kind() == "expression")
            && parsed.node_text(&candidate) == text
        {
            matches += 1;
            if matches == index {
                return Ok(candidate);
            }
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return Err(format!(
                    "missing named node {selector:?} in {}",
                    scope.to_sexp()
                ));
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Assertion {
    Module,
    EndToEnd,
}

fn check_rows(assertion: Assertion) {
    let mut lines = TSV.lines();
    assert_eq!(
        lines.next(),
        Some("id\tlanguage\tsource\toperation\ttarget\toperand\trole\texpect\treason\tkind\tnote\treached")
    );
    let mut seen = BTreeSet::new();
    let mut counts = BTreeMap::new();
    let mut failures = Vec::new();
    for line in lines {
        let id = line.split('\t').next().unwrap();
        if !seen.insert(id) {
            failures.push(format!("{id} [fixture] duplicate ID"));
            continue;
        }
        match Row::read(line) {
            Ok(row) => {
                let (total, failed) = counts.entry(row.kind).or_insert((0, 0));
                *total += 1;
                if let Err(error) = row.run(assertion) {
                    *failed += 1;
                    failures.push(format!("{} [{}] {}", row.id, row.kind, error));
                }
            }
            Err(error) => failures.push(format!("{id} [fixture] {error}")),
        }
    }
    assert!(!seen.is_empty(), "empty certificate fixture");
    for (kind, (total, failed)) in counts {
        println!("{kind}: {total} rows, {failed} failures");
    }
    for failure in &failures {
        println!("{failure}");
    }
    assert!(
        failures.is_empty(),
        "{} certificate rows failed",
        failures.len()
    );
}

#[test]
fn certificate_rows_module() {
    check_rows(Assertion::Module);
}

#[test]
fn certificate_rows_end_to_end() {
    check_rows(Assertion::EndToEnd);
}

#[test]
fn receipt_counts_current_tree_parse_recovery() {
    let parsed = ParsedFile::parse("a.js", "eval(x;", Language::JavaScript).unwrap();
    let bound = site(&parsed, "eval(x", "eval", "x", Role::Argument(0));
    assert!(bound.operation().has_error());
    let reached = bound.operand();
    let path = path_to(&parsed, reached, reached);
    let witness = receipt(&parsed, 1, bound, reached, FLAT, destination(&path));
    let mut refusals = Refusals::default();
    assert!(super::super::collect_rules(
        &parsed,
        1,
        &admitted(&parsed, &[1]),
        [witness],
        &mut refusals,
        select_eval,
    )
    .is_empty());
    assert_eq!(refusals.iter().collect::<Vec<_>>(), [("parse_recovery", 1)]);
}

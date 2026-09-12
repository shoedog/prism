use super::super::super::binding_table::{
    capture_rows, census, census_artifact_digest, grammar_digest, pinned_census_digest,
    pinned_digest, predicate_matches, rows, select_binding_row, select_capture_row, Ruling,
};
use super::super::parsed;
use crate::ast::ParsedFile;
use crate::cpg::FlowConfidence;
use crate::languages::Language;
use std::collections::BTreeSet;
use std::sync::OnceLock;
use tree_sitter::Node;

#[derive(Clone, Copy)]
pub(super) struct Case {
    pub id: &'static str,
    pub language: Language,
    pub kind: &'static str,
    pub variant: Option<&'static str>,
    pub src: &'static str,
    pub expect: &'static [(&'static str, &'static str, FlowConfidence)],
    pub expect_counter: Option<(&'static str, i64)>,
}

pub(super) fn nodes_of_kind<'t>(parsed: &'t ParsedFile, kind: &str) -> Vec<Node<'t>> {
    fn walk<'t>(node: Node<'t>, kind: &str, out: &mut Vec<Node<'t>>) {
        if node.kind() == kind {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(child, kind, out);
        }
    }
    let mut nodes = Vec::new();
    walk(parsed.tree.root_node(), kind, &mut nodes);
    nodes
}

pub(super) fn check_case(case: &Case) {
    let parsed = parsed(case.src, case.language);
    let nodes = nodes_of_kind(&parsed, case.kind);
    assert!(
        !nodes.is_empty(),
        "{}: snippet has no {}",
        case.id,
        case.kind
    );
    if case.variant == Some("capture") {
        let selected = select_capture_row(&parsed, nodes[0])
            .unwrap_or_else(|| panic!("{}: no capture row selected", case.id));
        assert_eq!(
            selected.regression, case.id,
            "{}: selected capture row",
            case.id
        );
    } else {
        let selected = select_binding_row(&parsed, nodes[0])
            .unwrap_or_else(|| panic!("{}: no binding row selected", case.id));
        assert_eq!(
            selected.regression, case.id,
            "{}: selected binding row",
            case.id
        );
        if let Ruling::Uncertain {
            reason: "not yet curated",
            ..
        } = selected.ruling
        {
            assert_eq!(case.expect, &[], "{}: provisional expectations", case.id);
            assert_eq!(
                case.expect_counter,
                Some(("dfg_label_nameonly_ownership_uncertain", 0)),
                "{}: provisional counter control",
                case.id
            );
        }
    }
}

pub(super) fn source(language: Language) -> &'static str {
    match language {
        Language::Python => super::python::SOURCE,
        Language::JavaScript => super::javascript::SOURCE,
        Language::TypeScript | Language::Tsx => super::typescript::SOURCE,
        Language::Go => {
            r#"package sample
import (alias "fmt")
const constant = 1
var global int
type Box[T any] struct { Field T }
type Worker interface { Work(value int) error }
var callbackType func(int) int
func declared[T any](first int, rest ...int) int {
    local := source()
    var other int = local
    local, other = other, local
    callback := func(value int) int { return value }
    if local > 0 { local = 1 }
    for index := 0; index < len(rest); index++ { local = index }
    for key, value := range rest { local = key + value }
    switch value := any(local).(type) { case int: local = value; default: }
    switch local { case 1: local = 2; default: local = 3 }
    select { case channel <- local: default: }
    return callback(other)
}
func (box Box[T]) Method(value int) int { return value }
"#
        }
        Language::Java => {
            r#"package sample;
import java.util.*;
module sample.module { requires java.base; }
@interface Marker { String value() default "x"; }
interface Shape<T> { T method(T value); }
enum Choice { FIRST; int field; }
record Pair(int left, int right) { Pair { this(left, right); } }
class Child<T> extends Base {
  static final int CONSTANT = 1;
  int field;
  Child(int value) { this.field = value; }
  void method(Child<T> this, int first, String... rest) {
    int local = source();
    local = first;
    for (int i = 0; i < 1; i++) { local = i; }
    for (String item : rest) { local = item.length(); }
    try (var resource = open()) { local = 1; } catch (RuntimeException | Error error) { local = 2; }
    Object fn = (String value) -> value.length();
    Object inferred = (left, right) -> left;
    if (receiver instanceof Pair(var left, var right)) { local = left; }
    if (receiver instanceof String text) { local = text.length(); }
    switch (local) { case 1: local = 2; default: local = 3; }
    Class<?> literal = String.class;
    /* census block comment */
  }
}
"#
        }
        Language::C => {
            r#"#define CALL(x) (x)
typedef int Alias;
typedef int FunctionType(int);
struct Record { int field; int values[2]; };
[[deprecated]];
static int global;
int declared(int first, int (*callback)(int), int rest[], ...) {
  int (*abstract_callback)(int) = callback;
  int (*array_pointer)[2] = &((int[2]){0, 1});
  int local = source(), *pointer = &local;
  int array[2] = {0};
  local = first;
  for (int i = 0; i < 2; i++) { array[i] = local; }
  switch (local) { case 1: break; default: break; }
  __attribute__((unused)) int attributed = 0;
  int ranged[] = {[0 ... 2] = 1};
  __try { local = 1; } __except(1) { local = 2; }
  return callback(*pointer);
}
"#
        }
        Language::Cpp => {
            r#"#define CALL(x) (x)
template <typename T, template<class> class Container, typename... Rest>
requires requires(T value) { value + value; }
class Child : public Base {
  T field;
  using Base::method;
  friend class Friend;
  void removed() = delete;
  explicit operator bool() const noexcept { return true; }
};
using Alias = int;
static_assert(true);
template <typename T> T declared(T first, T optional = T{}, T... rest) noexcept {
  static T staticValue{};
  T local = source(), *pointer = &local;
  T array[2] = {};
  local = first;
  for (int i = 0; i < 2; i++) { array[i] = local; }
  for (auto &item : array) { local = item; }
  try { throw local; } catch (const T &error) { local = error; }
  auto [left, right] = pair;
  auto stored = [copy = local, &local, this](T value) mutable { return value + copy; };
  auto defaultCaptured = [=](T value) { return value + local; };
  auto allocated = new T[3];
  consume(rest...);
  T (*functionPointer)(T) = nullptr;
  T (&arrayReference)[2] = array;
  T ranged[] = {[0 ... 2] = T{}};
  __try { local = T{}; } __except(1) { local = T{}; }
  switch (local) { case 1: break; default: break; }
  return stored(*pointer);
}
"#
        }
        Language::Rust => {
            r#"/* census block comment */
extern crate core as core_alias;
use crate::module::{self as renamed, Item};
type Alias<T> = Option<T>;
const CONSTANT: i32 = 1;
static mut STATIC_VALUE: i32 = 0;
struct Record<T> { field: T, ordered: i32 }
enum Choice<T> { One(T), Two { value: T } }
union Union { integer: i32, float: f32 }
trait Trait<T> { type Item; fn method<'a>(&'a self, value: T) -> T; }
type HigherRanked = for<'a> fn(&'a str) -> &'a str;
type PlainFunction = fn(i32) -> i32;
mod module { pub struct Item; }
fn declared<'a, T: Clone, const N: usize>(mut first: T, rest: &[T]) -> T {
    let mut local = first.clone();
    local = first.clone();
    local += first.clone();
    let (ref tuple, mut second, ..) = (local.clone(), local.clone(), local.clone());
    let Record { field: captured, ordered, .. } = record;
    let [head, middle @ .., tail] = array;
    let referenced = &head;
    let bounded: impl Iterator<Item = T> = iterator;
    let ranged = 0..10;
    if let Some(value) | None = option && let Choice::One(inner) = choice { local = value; }
    while let Some(value) = iterator.next() { local = value; }
    for value in rest { local = value.clone(); }
    match choice { Choice::One(value) => local = value, Choice::Two { value } => local = value, 0..=10 => local = first.clone() }
    let closure = |parameter: T| async move { parameter };
    let generated = gen { yield local.clone(); };
    let constant = const { 1 };
    let attempted = try { local.clone() };
    unsafe { STATIC_VALUE = N as i32; }
    local
}
async unsafe extern "C" fn modified(value: i32, ...) -> i32 { value }
fn invoke<T>() { generic::<T>(); let Some::<T>(value) = option; }
macro_rules! tokens { ($name:ident, $($tree:tt)*) => { let $name = stringify!($($tree)*); }; }
"#
        }
        Language::Lua => {
            r#"local global = source()
local function declared(first, ...)
  local one, two = first, global
  one = two
  for index = 1, 3 do one = index end
  for key, value in pairs({}) do one = value end
  local callback = function(parameter) return parameter end
  callback(one)
  return one
end
function table.method(value) return value end
for implicit in iterator do global = implicit end
"#
        }
        Language::Terraform => {
            r#"variable "items" { default = [] }
locals {
  tuple = [for item in var.items : item if item != null]
  object = {for key, value in var.items : key => value}
  called = join(",", var.items)
}
resource "example" "value" {
  dynamic "entry" { for_each = var.items content { value = entry.value } }
}
output "rendered" { value = "%{ for item in var.items ~}${item}%{ endfor ~}" }
"#
        }
        Language::Bash => {
            r#"declare -a values=(one two)
name=value
A=one B=two command
function declared() {
  local inner=$name
  for item in "${values[@]}"; do inner=$item; done
  for ((index=0; index<2; index++)); do inner=$index; done
  case "$inner" in one|two) inner=${special:-x} ;; esac
  [[ "$inner" == +(one|two) ]]
  inner=$?
  cat <<EOF
$inner
EOF
}
"#
        }
    }
}

fn source_for_kind(language: Language, kind: &str) -> &'static str {
    match language {
        Language::JavaScript => super::javascript::source_for_kind(kind),
        Language::TypeScript => super::typescript::source_for_kind(kind),
        Language::Tsx => super::tsx::source_for_kind(kind),
        Language::Java => super::java::source_for_kind(kind),
        Language::C => super::c::source_for_kind(kind),
        Language::Cpp => super::cpp::source_for_kind(kind),
        Language::Rust => super::rust::source_for_kind(kind),
        Language::Lua => super::lua::source_for_kind(kind),
        Language::Bash => super::bash::source_for_kind(kind),
        _ => source(language),
    }
}

fn curated_cases(language: Language) -> &'static [(&'static str, &'static str)] {
    match language {
        Language::Python => super::python::CURATED,
        Language::JavaScript | Language::TypeScript | Language::Tsx => super::javascript::CURATED,
        Language::Go => super::go::CURATED,
        Language::Java => super::java::CURATED,
        Language::C => super::c::CURATED,
        Language::Cpp => super::cpp::CURATED,
        Language::Rust => super::rust::CURATED,
        Language::Lua => super::lua::CURATED,
        Language::Terraform | Language::Bash => &[],
    }
}

fn language_short(language: Language) -> &'static str {
    match language {
        Language::Python => "py",
        Language::JavaScript => "js",
        Language::TypeScript => "ts",
        Language::Tsx => "tsx",
        Language::Go => "go",
        Language::Java => "java",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::Rust => "rs",
        Language::Lua => "lua",
        Language::Terraform => "tf",
        Language::Bash => "bash",
    }
}

fn leaked_id(prefix: &str, language: Language, kind: &str) -> &'static str {
    Box::leak(format!("{prefix}-{}-{kind}", language_short(language)).into_boxed_str())
}

pub(super) fn all_cases() -> Vec<&'static Case> {
    static CASES: OnceLock<Vec<Case>> = OnceLock::new();
    CASES
        .get_or_init(|| {
            let mut cases = Vec::new();
            let mut overrides = BTreeSet::new();
            for case in super::python::CASES
                .iter()
                .chain(super::javascript::CASES)
                .chain(super::typescript::CASES)
                .chain(super::tsx::CASES)
                .chain(super::go::CASES)
                .chain(super::java::CASES)
                .chain(super::c::CASES)
                .chain(super::cpp::CASES)
                .chain(super::rust::CASES)
                .chain(super::lua::CASES)
                .chain(super::terraform::CASES)
                .chain(super::bash::CASES)
            {
                overrides.insert((case.language, case.kind, case.variant));
                cases.push(*case);
            }
            for language in Language::all() {
                let curated: std::collections::BTreeMap<_, _> =
                    curated_cases(language).iter().copied().collect();
                let mut binding_kinds: BTreeSet<_> = curated.keys().copied().collect();
                binding_kinds.extend(
                    census(language)
                        .kinds
                        .iter()
                        .filter(|kind| kind.candidate || !kind.heuristic_flags.is_empty())
                        .map(|kind| kind.kind.as_str()),
                );
                for kind in binding_kinds {
                    if overrides.contains(&(language, kind, Some("binding"))) {
                        continue;
                    }
                    let (id, provisional) = curated
                        .get(kind)
                        .copied()
                        .map(|id| (id, false))
                        .unwrap_or_else(|| (leaked_id("e0b", language, kind), true));
                    cases.push(Case {
                        id,
                        language,
                        kind: Box::leak(kind.to_string().into_boxed_str()),
                        variant: Some("binding"),
                        src: source_for_kind(language, kind),
                        expect: &[],
                        expect_counter: provisional
                            .then_some(("dfg_label_nameonly_ownership_uncertain", 0)),
                    });
                }
                for kind in language.callable_boundary_node_types() {
                    if overrides.contains(&(language, kind, Some("capture"))) {
                        continue;
                    }
                    cases.push(Case {
                        id: leaked_id("e0a-x-capture", language, kind),
                        language,
                        kind,
                        variant: Some("capture"),
                        src: source_for_kind(language, kind),
                        expect: &[],
                        expect_counter: None,
                    });
                }
            }
            cases
        })
        .iter()
        .collect()
}

#[test]
fn binding_table_digest_matches_grammar() {
    for language in Language::all() {
        let digest = pinned_digest(language);
        assert_eq!(
            grammar_digest(language),
            digest,
            "{language:?}: live grammar"
        );
        assert_eq!(census(language).digest, digest, "{language:?}: census pin");
        assert_eq!(
            census_artifact_digest(language),
            pinned_census_digest(language),
            "{language:?}: census file changed — re-run and re-pin"
        );
    }
}

#[test]
fn binding_table_is_total_over_candidates() {
    for language in Language::all() {
        let census = census(language);
        assert!(!census.kinds.is_empty(), "{language:?}: empty census");
        let expected: BTreeSet<_> = census
            .kinds
            .iter()
            .filter(|kind| kind.candidate || !kind.heuristic_flags.is_empty())
            .map(|kind| kind.kind.as_str())
            .collect();
        let actual: BTreeSet<_> = rows(language)
            .iter()
            .filter(|row| expected.contains(row.kind))
            .map(|row| row.kind)
            .collect();
        assert_eq!(actual, expected, "{language:?}: candidate kind rows");
    }
}

#[test]
fn binding_table_rows_have_regressions() {
    let cases = all_cases();
    let mut expected: Vec<_> = cases
        .iter()
        .map(|case| (case.language, case.id, case.kind))
        .collect();
    let mut actual = Vec::new();
    for language in Language::all() {
        for (kind, regression) in rows(language)
            .iter()
            .map(|row| (row.kind, row.regression))
            .chain(
                capture_rows(language)
                    .iter()
                    .map(|row| (row.kind, row.regression)),
            )
        {
            actual.push((language, regression, kind));
        }
    }
    expected.sort_unstable();
    expected.dedup();
    actual.sort_unstable();
    actual.dedup();
    assert_eq!(actual, expected, "case ids and kinds");
    for case in cases {
        check_case(case);
    }
}

#[test]
fn every_case_selects_exactly_one_row_per_occurrence() {
    for case in all_cases() {
        let parsed = parsed(case.src, case.language);
        let nodes = nodes_of_kind(&parsed, case.kind);
        assert!(
            !nodes.is_empty(),
            "{}: snippet has no {} node",
            case.id,
            case.kind
        );
        for node in nodes {
            let (predicate_hits, residuals, selected) = if case.variant == Some("capture") {
                let table = capture_rows(case.language);
                (
                    table
                        .iter()
                        .filter(|row| {
                            row.kind == case.kind
                                && row.variant.as_ref().is_some_and(|predicate| {
                                    predicate_matches(&parsed, node, predicate)
                                })
                        })
                        .count(),
                    table
                        .iter()
                        .filter(|row| row.kind == case.kind && row.variant.is_none())
                        .count(),
                    select_capture_row(&parsed, node).map(|row| row.regression),
                )
            } else {
                let table = rows(case.language);
                (
                    table
                        .iter()
                        .filter(|row| {
                            row.kind == case.kind
                                && row.variant.as_ref().is_some_and(|predicate| {
                                    predicate_matches(&parsed, node, predicate)
                                })
                        })
                        .count(),
                    table
                        .iter()
                        .filter(|row| row.kind == case.kind && row.variant.is_none())
                        .count(),
                    select_binding_row(&parsed, node).map(|row| row.regression),
                )
            };
            let selection_count = if predicate_hits == 0 {
                residuals
            } else {
                predicate_hits
            };
            assert_eq!(selection_count, 1, "{}: {predicate_hits} predicate rows and {residuals} residual rows matched one occurrence", case.id);
            assert_eq!(selected, Some(case.id), "{}: selected row id", case.id);
        }
    }
}

#[test]
fn uncertain_rows_carry_reasons() {
    for language in Language::all() {
        for row in rows(language) {
            if let Ruling::Uncertain { reason, revisit } = row.ruling {
                assert!(!reason.is_empty(), "{language:?} {} reason", row.kind);
                assert!(!revisit.is_empty(), "{language:?} {} revisit", row.kind);
            }
        }
    }
}

#[test]
fn capture_table_covers_every_boundary_kind() {
    for language in Language::all() {
        let actual: Vec<_> = capture_rows(language).iter().map(|row| row.kind).collect();
        let expected = language.callable_boundary_node_types();
        assert_eq!(actual, expected, "{language:?}");
    }
}

#[test]
fn no_provisional_row_whose_revisit_task_closed() {
    let closed: &[&str] = &[];
    for language in Language::all() {
        for row in rows(language) {
            if let Ruling::Uncertain {
                reason: "not yet curated",
                revisit,
            } = row.ruling
            {
                assert!(
                    !closed.contains(&revisit),
                    "{language:?} {} still provisional after {revisit}",
                    row.kind
                );
            }
        }
    }
}

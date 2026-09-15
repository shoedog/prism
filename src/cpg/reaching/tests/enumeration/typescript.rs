use super::super::super::binding_table::Predicate;
use crate::languages::Language;

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "e0a-x-for_in_statement-predicate",
        language: Language::TypeScript,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: Some(Predicate::FieldTextIs {
            field: "kind",
            any_of: &["let", "const"],
        }),
        src: "function f(items: number[]) { for (const item of items) { sink(item); } }",
        expect: &[],
        expect_counter: None,
    },
    super::case::Case {
        id: "e0a-x-for_in_statement-residual",
        language: Language::TypeScript,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: None,
        src: "function f(items: number[]) { for (item of items) { sink(item); } }",
        expect: &[],
        expect_counter: None,
    },
];

pub(super) const SOURCE: &str = r#"import defaultName, * as ns from "pkg" with { type: "json" };
import {item as alias} from "pkg";
import required = require("pkg");
declare const ambient: number;
declare function signature(value: number): void;
type Alias<T extends object = {}> = { [K in keyof T]?: T[K] };
type Fn = (required: number, optional?: string) => void;
type Ctor = new (value: number) => Base;
interface Shape<T> { (x: T): T; new (x: T): Shape<T>; method?(x: T): void; }
abstract class Abstract<T> extends Base { abstract method(x: T): void; }
enum Choice { First = 1, Second }
let outer: number = source();
var functionScoped: number = outer;
function declared<T>(a: T, optional?: T, ...rest: T[]): T {
  let [first, ...tail] = rest;
  const {key: renamed = first, shorthand, ...others} = ns;
  outer += 1;
  for (let i = 0; i < 1; i++) { outer = i; }
  for (const item of rest) { outer = 1; }
  try { throw outer; } catch ({message}) { outer = message.length; }
  switch (outer) { case 1: break; default: break; }
  with (ns) { outer = value; }
  return /pattern/.test(String(outer)) ? a : a;
}
function* generated() { yield outer; }
const generatorExpression = function* () { yield outer; };
const expression = function named() { return outer; };
const arrow = <T,>(x: T = outer as T) => x;
({assigned = outer, ...assignedRest} = ns);
import Qualified = ns.Member;
class Child extends Base { static { this.ready = true; } method(p: number) { return p; } }
"#;

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "class" => "const C = class Named { method(): void {} };",
        "import" => "async function f() { return import('pkg'); }",
        _ => SOURCE,
    }
}

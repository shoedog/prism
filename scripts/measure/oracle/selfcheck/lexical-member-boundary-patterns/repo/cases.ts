export {};
class A { /*@field*/m = 1 }
const a = new A();
const b = new A();
let out = 0;
const /*@patternPlain*/m = 1;
({ /*@direct*/m: out } = a);
({ a: { /*@nested*/m: out } } = { a });
([{ /*@array*/m: out }] = [a]);
a./*@write*/m = 2;
const { k: { /*@nestedBinding*/m: bound } } = { k: a };
({ p: { /*@defaultKey*/m: out } = a } = { p: b });
function bindingShorthand() {
  const { /*@bindingShorthand*/m } = a;
  return /*@bindingValue*/m;
}
function assignmentShorthand() {
  let m = 0;
  ({ /*@assignmentShorthand*/m } = a);
  return /*@assignmentValue*/m;
}
const /*@plainDirect*/direct = a;
const { m: /*@plainBound*/boundValue } = /*@plainDirectUse*/direct;
const /*@plainNested*/nested = { a: { m: 1 } };
({ a: { m: out } } = /*@plainNestedUse*/nested);
const /*@plainArray*/array = [a];
([{ m: out }] = /*@plainArrayUse*/array);
const /*@plainDefault*/fallback = a;
({ p: { m: out } = /*@plainDefaultUse*/fallback } = { p: b });
const [/*@arrayBinding*/element, { /*@objectBinding*/other }] = [1, { other: 2 }] as const;
const value = { /*@objectShorthand*/other };
const /*@computed*/key = 'm';
const { [/*@computedBinding*/key]: /*@computedValue*/computed } = a;
({ [/*@computedAssignment*/key]: out } = a);
const values = [/*@arrayValue*/element, /*@boundValueUse*/boundValue,
  /*@computedValueUse*/computed, /*@plainPatternUse*/m];

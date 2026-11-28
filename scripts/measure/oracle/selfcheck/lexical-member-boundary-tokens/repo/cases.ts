export {};
namespace Name { export interface Q {} }
let /*@localQ*/Q = 1;
type Qualified = Name./*@qualified*/Q;
const /*@qRead*/qRead = /*@plainQ*/Q;
const /*@localKey*/key = 'm';
interface Shape {
  /*@interfaceMember*/field: number;
  [/*@interfaceMethodKey*/key](): void;
}
type Literal = { [/*@literalMethodKey*/key](): void };
const live = { [/*@objectMethodKey*/key]() {} };
class Concrete { /*@classMember*/m = 1; }
let /*@localMember*/m = /*@nonIdentifier*/1;
let /*@bindingTarget*/target = 0;
const pattern = { /*@objectKey*/m: 2 };
const { /*@bindingKey*/m: /*@bindingValue*/v } = pattern;
({ /*@assignmentKey*/m: target } = pattern);
let /*@plainField*/field = 1;
const fieldRead = /*@fieldRead*/field;
const bare = /*@memberRead*/m;
const binding = /*@bindingRead*/v;
const { [/*@computedKey*/key]: member } = pattern;
const base = /*@plainPath*/m;
enum Order { /*@enumValue*/first = 1, second = /*@enumRead*/first }

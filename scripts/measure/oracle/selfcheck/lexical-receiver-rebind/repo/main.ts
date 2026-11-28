class A { m = 0; }
function sameInstance() {
  let a = new A();
  a./*@sameWrite*/m = 1;
  return a./*@sameRead*/m;
}
function differentInstance() {
  let a = new A();
  a./*@differentWrite*/m = 1;
  a = new A();
  return a./*@differentRead*/m;
}
function patternRebind() {
  let a = new A();
  a./*@patternWrite*/m = 1;
  ({a} = { a: new A() });
  return a./*@patternRead*/m;
}
console.log(sameInstance(), differentInstance(), patternRebind());

function identity(formal: number) { return formal; }
function caller(actual: number) {
  return identity(actual);
}
function repeat(x: number) {
  return x + x;
}
function overwrite(x: number) {
  x = 0;
  return x;
}
function g(o: {x: number}) {
  const u = o;
  o.x = 0;
  u.x++;
  return sink(o.x, o.x);
}
function outer() {
  function inner() { return 2; }
  return inner();
}
function sameA(){return 1;} function sameB(){return 2;}
function useBoth(){return sameA()+sameB();}

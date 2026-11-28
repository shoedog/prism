export {};
declare function use(v: unknown): void;
export function steps(a: number) {
  var x = a;
  x = x + 1;
  use(x);
  use(x);
  var q = 0;
  q = 3;
  use(q);
}
export function shadow(a: number) {
  var y = a;
  var f = function (y: number) { return y; };
  var y2 = a;
  var g = function (y2: number) { return y2; }, w = y2;
  return [f, g, w, y];
}

namespace NS1 {
  export function f1(a1: number) { return a1; }
  export function run1(x1: number) { return f1(x1); }
}
// @ts-ignore
NS1.f1 = function (b1: number) { return b1; };

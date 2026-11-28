declare function seal(target: any, context: any): any;
@seal class D { m = (d1: number) => d1; static s(e1: number) { return e1; } }
class E { m = (f1: number) => f1; static s(g1: number) { return g1; } }
class R { static s(r1: number) { return r1; } constructor() { return {} as any; } }
function run(a1: number, a2: number, b1: number, b2: number, c1: number) {
  return [new D().m(a1), D.s(a2), new E().m(b1), E.s(b2), R.s(c1)];
}

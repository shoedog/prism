class C { m(p1: number) { return p1; } n(q1: number) { return q1; } }
class F { f = (f1: number) => f1; constructor(r: any) { [this.f] = [r]; } }
class G { g = (g1: number) => g1; constructor(r: any) { r; } }
class S {
  static s1(a1: number) { return a1; }
  static s2(b1: number) { return b1; }
  static s3(c1: number) { return c1; }
  static s4(d1: number) { return d1; }
}
declare const fns: ((x: number) => number)[];
declare const dflt: (x: number) => number;
[C.prototype.m] = [(rep: number) => rep];
({ key: S.s2 } = { key: dflt });
for (S.s3 of fns) { }
[S.s4 = dflt] = [];
function run(u1: number, u2: number, u3: number, u4: number, u5: number, u6: number, u7: number, u8: number) {
  return [new C().m(u1), new C().n(u2), new F(0).f(u3), new G(0).g(u4), S.s1(u5), S.s2(u6), S.s3(u7), S.s4(u8)];
}

class B { m(x: number) { return x; } }
class C extends B { m(y: number) { return y; } }
function run(v: number, w: number, z: number, q: number) {
  const o = new C() as B;
  const p = new C();
  return [(new C() as B).m(v), o.m(w), p.m(z), (new C() as C).m(q)];
}

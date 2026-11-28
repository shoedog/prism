class C { m(p1: number) { return p1; } }
function run(a1: number, a2: number, a3: number, a4: number, a5: number, a6: number) {
  const o1 = new C();
  const o2 = new C();
  const o3 = new C();
  const o4 = new C();
  const o5: C = new C();
  const o6 = new C();
  return [o1.m(a1), (o2).m(a2), o3!.m(a3), (o4 satisfies C).m(a4), o5.m(a5), (o6 as C).m(a6)];
}

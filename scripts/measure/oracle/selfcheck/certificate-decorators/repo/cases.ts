declare function replace(...args: any[]): any;
class C {
  @replace static sm(old1: number) { return old1; }
  static pm(plain1: number) { return plain1; }
  @replace im(old2: number) { return old2; }
  pim(plain2: number) { return plain2; }
  @replace f = (old3: number) => old3;
  pf = (plain3: number) => plain3;
  @replace #pv(old4: number) { return old4; }
  #pu(plain4: number) { return plain4; }
  priv(a4: number, b4: number) { return [this.#pv(a4), this.#pu(b4)]; }
}
class D extends C { sup(a5: number, b5: number) { return [super.im(a5), super.pim(b5)]; } }
function run(a1: number, b1: number, a2: number, b2: number, a3: number, b3: number) {
  return [C.sm(a1), C.pm(b1), new C().im(a2), new C().pim(b2), new C().f(a3), new C().pf(b3)];
}

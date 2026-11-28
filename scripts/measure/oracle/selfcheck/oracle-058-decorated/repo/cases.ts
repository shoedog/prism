class C {
  @decorate
  static m(
    v: number) { return v; }
}
function run(a: number) { return C.m(a); }
